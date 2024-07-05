use std::fs::{self, OpenOptions};
use std::{io::Write, path::PathBuf};

use bson::{doc, DateTime, RawDocument};
use mongodb::{bson::Document, Client, Collection, Cursor};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
struct RecRow {
    pub id: String,
    pub op: Op,
    pub ns: String,
    pub ts: u64,
    pub st: Status,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
enum Op {
    #[default]
    None,
    Insert,
    Update,
    Delete,
    Query(OpQuery),
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
struct OpQuery {
    pub db: String,
    pub find: String,
    pub filter: Value,
    pub limit: Option<i32>,
    pub sort: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
enum Status {
    #[default]
    None,
    Pending,
    Success(StatusSuccess),
    Failed,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
struct StatusSuccess {
    pub rts: u64,
    pub rms: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
struct MongobarConfig {
    pub uri: String,
    pub db: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
struct Mongobar {
    dir: PathBuf,
    name: String,
    rows: Vec<RecRow>,

    op_file_padding: PathBuf,
    op_file_done: PathBuf,

    config_file: PathBuf,
    config: MongobarConfig,
}

impl Mongobar {
    pub fn new(name: &str) -> Self {
        let cur_cwd: PathBuf = std::env::current_dir().unwrap();
        let dir = cur_cwd.join("runtime");
        let cwd: PathBuf = dir.join(name);
        Self {
            name: name.to_string(),
            rows: Vec::new(),
            op_file_padding: cwd.join(PathBuf::from("padding.oplog.json")),
            op_file_done: cwd.join(PathBuf::from("done.oplog.json")),
            config_file: cur_cwd.join(PathBuf::from("mongobar.json")),
            config: MongobarConfig::default(),
            dir,
        }
    }

    pub fn cwd(&self) -> PathBuf {
        self.dir.join(&self.name)
    }

    pub fn init(mut self) -> Self {
        let cwd = self.cwd();

        if cwd.exists() {
            fs::remove_dir_all(&cwd).unwrap();
        }

        fs::create_dir_all(&cwd).unwrap();
        fs::write(cwd.clone().join(&self.op_file_padding), "").unwrap();
        fs::write(cwd.clone().join(&self.op_file_done), "").unwrap();

        self.load_config();

        return self;
    }

    fn load_config(&mut self) {
        let content = fs::read_to_string(&self.config_file).unwrap();
        self.config = serde_json::from_str(&content).unwrap();
    }

    pub fn add_row(&mut self, row: RecRow) {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&self.op_file_padding)
            .unwrap();
        let content = serde_json::to_string(&row.clone()).unwrap();
        writeln!(file, "{}", content).unwrap();

        self.rows.push(row);
    }

    pub fn add_row_by_profile(&mut self, doc: &RawDocument) {
        let ns = doc.get_str("ns").unwrap().to_string();
        if ns.contains("system.profile") {
            return;
        }
        // let doc_as_json = serde_json::to_string(&doc).unwrap();
        // println!("{}", doc_as_json);
        let mut row = RecRow::default();
        let op = doc.get_str("op").unwrap();
        let command = doc.get_document("command").unwrap();
        match op {
            "query" => {
                row.id = doc.get_str("queryHash").unwrap().to_string();
                row.ns = ns;
                row.ts = doc.get_datetime("ts").unwrap().timestamp_millis() as u64;
                row.op = Op::Query(OpQuery {
                    db: command.get_str("$db").unwrap().to_string(),
                    find: command.get_str("find").unwrap().to_string(),
                    filter: serde_json::to_value(&command.get_document("filter").unwrap()).unwrap(),
                    limit: if let Ok(v) = command.get_i32("limit") {
                        Some(v)
                    } else {
                        None
                    },
                    sort: if let Ok(v) = command.get_document("sort") {
                        Some(serde_json::to_value(&v).unwrap())
                    } else {
                        None
                    },
                });
            }
            _ => {}
        }
        println!("{:?}", row);
        self.add_row(row);
    }

    /// 录制逻辑：
    /// 1. 【程序】标记开始时间 毫秒
    /// 2. 【人工】操作具体业务
    /// 3. 【程序】标记结束时间 毫秒
    /// 4. 【程序】读取 oplog.rs 中的数据，找到对应的操作
    /// 5. 【程序】读取 db.system.profile 中的数据，找到对应的操作
    /// 6. 【程序】处理两个数据，并且按时间排序，最终生成可以执行的逻辑，生成文件
    pub async fn op_record(
        &mut self,
        time_range: (DateTime, DateTime),
    ) -> Result<(), anyhow::Error> {
        let start_time = time_range.0;
        let end_time = time_range.1;
        let client = Client::with_uri_str(&self.config.uri).await?;

        let db = client.database(&self.config.db);

        let c: Collection<Document> = db.collection("system.profile");

        let ns_ne = self.config.db.clone() + ".system.profile";

        let query = doc! {
           "op": "query",
           "ns": { "$ne": ns_ne },
           "ts": { "$gte": start_time, "$lt": end_time }
        };
        // let doc_as_json = serde_json::to_string(&query)?;
        // println!("{}", doc_as_json);
        let mut cursor: Cursor<Document> = c.find(query).await?;

        while cursor.advance().await? {
            let v = cursor.current();
            self.add_row_by_profile(&v);
            // let doc_as_json = serde_json::to_string(&v)?;
            // println!("{}", doc_as_json);
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // 执行录制好的压测文件：
    // 1. 【程序】读取文件
    // 2. 【程序】创建 1000 个线程，并预分配好每个线程的操作
    // 3. 【程序】标记开始时间 毫秒
    // 4. 【程序】放开所有线程
    // 5. 【程序】等待所有线程结束
    // 6. 【程序】标记结束时间 毫秒
    // 7. 【程序】计算分析

    // 恢复压测前状态
    // 1. 【程序】读取上面标记的时间
    // 2. 【程序】通过时间拉取所有的 oplog.rs
    // 3. 【程序】反向执行所有的操作

    Mongobar::new("qxg")
        .init()
        .op_record((
            DateTime::parse_rfc3339_str("2024-07-03T10:54:18.837Z").unwrap(),
            DateTime::parse_rfc3339_str("2024-07-05T10:54:18.838Z").unwrap(),
        ))
        .await?;

    Ok(())
}
