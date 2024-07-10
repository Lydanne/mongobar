use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
};

use bson::{doc, DateTime};

use mongodb::{action::Single, bson::Document, options::ClientOptions, Client, Collection, Cursor};

use serde::{Deserialize, Serialize};
use tokio::time::Instant;

use crate::{indicator::Indicator, ui};

mod op_row;

mod mongobar_config;

mod op_state;

#[derive(Clone, Debug, Default)]
pub(crate) struct Mongobar {
    pub(crate) dir: PathBuf,
    pub(crate) name: String,

    pub(crate) op_workdir: PathBuf,
    pub(crate) op_rows: Vec<op_row::OpRow>,
    pub(crate) op_file_padding: PathBuf,
    pub(crate) op_file_done: PathBuf,

    pub(crate) op_state_file: PathBuf,
    pub(crate) op_state: op_state::OpState,

    pub(crate) config_file: PathBuf,
    pub(crate) config: mongobar_config::MongobarConfig,

    pub(crate) indicator: Indicator,
    pub(crate) signal: Arc<crate::signal::Signal>,
}

impl Mongobar {
    pub fn new(name: &str) -> Self {
        let cur_cwd: PathBuf = std::env::current_dir().unwrap();
        let dir: PathBuf = cur_cwd.join("runtime");
        let workdir: PathBuf = dir.join(name);
        Self {
            name: name.to_string(),
            op_workdir: workdir.clone(),
            op_rows: Vec::new(),
            op_file_padding: workdir.join(PathBuf::from("padding.oplog.json")),
            op_file_done: workdir.join(PathBuf::from("done.oplog.json")),
            config_file: cur_cwd.join(PathBuf::from("mongobar.json")),
            config: mongobar_config::MongobarConfig::default(),
            dir,

            op_state_file: workdir.join(PathBuf::from("state.json")),
            op_state: op_state::OpState::default(),
            indicator: Indicator::new(),

            signal: Arc::new(crate::signal::Signal::new()),
        }
    }

    pub fn cwd(&self) -> PathBuf {
        self.dir.join(&self.name)
    }

    pub fn init(mut self) -> Self {
        let cwd = self.cwd();

        if !cwd.exists() {
            fs::create_dir_all(&cwd).unwrap();
            fs::write(cwd.clone().join(&self.op_file_padding), "").unwrap();
            fs::write(cwd.clone().join(&self.op_file_done), "").unwrap();
        }

        self.load_config();
        self.load_state();
        self.load_op_rows();

        return self;
    }

    pub fn set_indicator(mut self, indicator: Indicator) -> Self {
        self.indicator = indicator;
        self
    }

    pub fn set_signal(mut self, signal: Arc<crate::signal::Signal>) -> Self {
        self.signal = signal;
        self
    }

    pub fn clean(self) -> Self {
        let _ = fs::remove_dir_all(&self.cwd());
        Self::new(&self.name).init()
    }

    pub fn load_config(&mut self) {
        if !self.config_file.exists() {
            self.save_config();
        }
        let content: String = fs::read_to_string(&self.config_file).unwrap();
        self.config = serde_json::from_str(&content).unwrap();
    }

    pub fn save_config(&self) {
        let content = serde_json::to_string(&self.config).unwrap();
        fs::write(&self.config_file, content).unwrap();
    }

    pub fn load_state(&mut self) {
        if !self.op_state_file.exists() {
            self.save_state();
        }
        let content = fs::read_to_string(&self.op_state_file).unwrap();
        self.op_state = serde_json::from_str(&content).unwrap();
    }

    pub fn save_state(&self) {
        let content: String = serde_json::to_string(&self.op_state).unwrap();
        fs::write(&self.op_state_file, content).unwrap();
    }

    pub fn add_row(&mut self, row: op_row::OpRow) {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&self.op_file_padding)
            .unwrap();
        let content = serde_json::to_string(&row.clone()).unwrap();
        writeln!(file, "{}", content).unwrap();

        self.op_rows.push(row);
    }

    pub fn add_row_by_profile(&mut self, doc: &Document) {
        let ns = doc.get_str("ns").unwrap().to_string();
        if ns.contains("system.profile") {
            return;
        }
        // let doc_as_json = serde_json::to_string(&doc).unwrap();
        // println!("{}", doc_as_json);
        let mut row = op_row::OpRow::default();
        let op = doc.get_str("op").unwrap();
        let command = doc.get_document("command").unwrap();
        match op {
            "query" => {
                if let Err(_) = doc.get_str("queryHash") {
                    return;
                }
                row.id = doc.get_str("queryHash").unwrap().to_string();
                row.ns = ns;
                row.ts = doc.get_datetime("ts").unwrap().timestamp_millis() as i64;
                row.op = op_row::Op::Query;
                row.db = command.get_str("$db").unwrap().to_string();
                row.coll = command.get_str("find").unwrap().to_string();
                let mut new_cmd = command.clone();
                new_cmd.remove("lsid");
                new_cmd.remove("$clusterTime");
                new_cmd.remove("$db");
                row.cmd = new_cmd
            }
            _ => {}
        }
        // println!("{:?}", row);
        self.add_row(row);
    }

    pub fn load_op_rows(&mut self) {
        let content = fs::read_to_string(&self.op_file_padding).unwrap();
        let rows: Vec<op_row::OpRow> = content
            .split("\n")
            .filter(|v| !v.is_empty())
            .map(|v| serde_json::from_str(v).unwrap())
            .collect();

        self.op_rows = rows;
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
        if self.op_state.record_end_ts > 0 {
            panic!("[OPRecord] 已经录制过了，不能重复录制，请先调用 clean 清理数据");
        }

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
            let v = cursor.deserialize_current().unwrap();
            self.add_row_by_profile(&v);
            // let doc_as_json = serde_json::to_string(&v)?;
            // println!("{}", doc_as_json);
        }

        self.op_state.record_start_ts = start_time.timestamp_millis() as i64;
        self.op_state.record_end_ts = end_time.timestamp_millis() as i64;
        self.save_state();

        Ok(())
    }

    // 执行录制好的压测文件：
    // 1. 【程序】读取文件
    // 2. 【程序】创建 1000 个线程，并预分配好每个线程的操作
    // 3. 【程序】标记开始时间 毫秒
    // 4. 【程序】放开所有线程
    // 5. 【程序】等待所有线程结束
    // 6. 【程序】标记结束时间 毫秒
    // 7. 【程序】计算分析
    pub async fn op_stress(&self) -> Result<(), anyhow::Error> {
        // let record_start_time = DateTime::from_millis(self.op_state.record_start_ts);
        // let record_end_time = DateTime::from_millis(self.op_state.record_end_ts);

        let loop_count = self.config.loop_count;
        let thread_count = self.config.thread_count;

        let mongo_uri = self.config.uri.clone();

        let mut options = ClientOptions::parse(&mongo_uri).await.unwrap();
        options.max_pool_size = Some(thread_count * 1000);
        options.min_pool_size = Some(thread_count);
        let client = Arc::new(Client::with_options(options).unwrap());
        let db = client.database(&self.config.db);

        let cur_profile = db.run_command(doc! {  "profile": -1 }).await?;

        if let Ok(was) = cur_profile.get_i32("was") {
            if was != 0 {
                db.run_command(doc! { "profile": 0 }).await?;
            }
        }

        // println!(
        //     "OPStress [{}] loop_count: {} thread_count: {}",
        //     chrono::Local::now().timestamp(),
        //     loop_count,
        //     thread_count
        // );

        let gate = Arc::new(tokio::sync::Barrier::new(thread_count as usize));
        let mut handles = vec![];

        let boot_worker = self.indicator.take("boot_worker").unwrap();
        let done_worker = self.indicator.take("done_worker").unwrap();
        let dyn_threads = self.indicator.take("dyn_threads").unwrap();
        let query_count = self.indicator.take("query_count").unwrap();
        // let in_size = Arc::new(AtomicUsize::new(0));
        // let out_size = Arc::new(AtomicUsize::new(0));
        let cost_ms = self.indicator.take("cost_ms").unwrap();
        let progress = self.indicator.take("progress").unwrap();
        let logs = self.indicator.take("logs").unwrap();
        let signal = Arc::clone(&self.signal);

        self.indicator
            .take("thread_count")
            .unwrap()
            .set(thread_count as usize);

        let mut created_thread_count = 0;
        loop {
            let dyn_threads_num = dyn_threads.get();
            let thread_count_total = thread_count as i32 + dyn_threads_num as i32;
            let done_worker_num = done_worker.get();
            if done_worker_num >= thread_count_total as usize {
                break;
            }
            if signal.get() != 0 {
                break;
            }
            if created_thread_count >= thread_count_total {
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                continue;
            }
            // -------------------------------------
            let i = created_thread_count;
            let gate = gate.clone();
            let op_rows = self.op_rows.clone();

            // let out_size = out_size.clone();
            // let in_size = in_size.clone();
            let query_count = query_count.clone();
            let progress = progress.clone();
            let cost_ms = cost_ms.clone();
            let boot_worker = boot_worker.clone();
            let logs = logs.clone();
            let client = Arc::clone(&client);
            let signal = Arc::clone(&signal);
            let done_worker = done_worker.clone();
            let thread_count_num = thread_count;
            handles.push(tokio::spawn(async move {
                // println!("Thread[{}] [{}]\twait", i, chrono::Local::now().timestamp());
                boot_worker.increment();
                if i < thread_count_num as i32 {
                    gate.wait().await;
                }
                // println!(
                //     "Thread[{}] [{}]\tstart",
                //     i,
                //     chrono::Local::now().timestamp()
                // );

                // let client = Client::with_uri_str(mongo_uri).await.unwrap();

                for _c in 0..loop_count {
                    // println!(
                    //     "Thread[{}] [{}]\tloop {}",
                    //     i,
                    //     chrono::Local::now().timestamp(),
                    //     _c,
                    // );
                    if signal.get() != 0 {
                        break;
                    }
                    for row in &op_rows {
                        if signal.get() != 0 {
                            break;
                        }
                        progress.increment();
                        match &row.op {
                            op_row::Op::Query => {
                                let db = client.database(&row.db);
                                // out_size.fetch_add(row.cmd.len(), Ordering::Relaxed);
                                let start = Instant::now();
                                let res = db.run_cursor_command(row.cmd.clone()).await;
                                let end = start.elapsed();
                                cost_ms.add(end.as_millis() as usize);
                                query_count.increment();
                                if let Err(e) = &res {
                                    logs.push(format!(
                                        "OPStress [{}] [{}]\t err {}",
                                        chrono::Local::now().timestamp(),
                                        i,
                                        e
                                    ));
                                }
                                // if let Ok(mut cursor) = res {
                                //     let mut sum = 0;
                                //     while cursor.advance().await.unwrap() {
                                //         sum += cursor.current().as_bytes().len();
                                //     }
                                //     in_size.fetch_add(sum, Ordering::Relaxed);
                                // }
                            }
                            _ => {}
                        }
                    }
                }

                // println!("Thread[{}] [{}]\tend", i, chrono::Local::now().timestamp());

                done_worker.increment();
            }));
            created_thread_count += 1;
            self.indicator.take("progress_total").unwrap().set(
                self.op_rows.len()
                    * loop_count as usize
                    * (thread_count as usize + dyn_threads_num),
            );
        }

        // let stress_start_time: i64 = chrono::Local::now().timestamp();
        // self.op_state.stress_start_ts = stress_start_time;
        // self.save_state();

        for handle in handles {
            handle.await?;
        }

        // let stress_end_time = chrono::Local::now().timestamp();
        // self.op_state.stress_end_ts = stress_end_time;
        // self.save_state();

        // if let Ok(was) = cur_profile.get_i32("was") {
        //     if was != 0 {
        //         db.run_command(doc! { "profile": was }).await?;
        //     }
        // }
        Arc::try_unwrap(client).unwrap().shutdown().await;

        Ok(())
    }

    // 恢复压测前状态
    // 1. 【程序】读取上面标记的时间
    // 2. 【程序】通过时间拉取所有的 oplog.rs
    // 3. 【程序】反向执行所有的操作
    // pub async fn op_resume(&self) -> Result<(), anyhow::Error> {
    //     Ok(())
    // }
}

// fn bytes_to_mb(bytes: usize) -> f64 {
//     bytes as f64 / 1024.0 / 1024.0
// }
