use std::{
    fs::{self},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use bson::{doc, DateTime};

use mongodb::{bson::Document, options::ClientOptions, Client, Collection, Cursor};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::indicator::Indicator;
use futures::TryStreamExt;
use op_logs::{reverse_file, OpLogs, OpReadMode};
use tokio::time::Instant;

mod mongobar_config;

mod op_state;

pub mod op_logs;
pub mod op_row;

#[derive(Debug, Clone)]
pub enum OpRunMode {
    Readonly,
    ReadWrite,
}

#[derive(Clone, Debug)]
pub(crate) struct Mongobar {
    pub(crate) dir: PathBuf,
    pub(crate) name: String,

    pub(crate) op_workdir: PathBuf,
    pub(crate) op_file_oplogs: PathBuf,
    pub(crate) op_file_revert: PathBuf,
    pub(crate) op_file_resume: PathBuf,
    pub(crate) op_file_data: PathBuf,

    pub(crate) op_state_file: PathBuf,
    pub(crate) op_state: op_state::OpState,

    pub(crate) config: mongobar_config::MongobarConfig,

    pub(crate) indicator: Indicator,
    pub(crate) signal: Arc<crate::signal::Signal>,
}

impl Mongobar {
    pub fn new(name: &str) -> Self {
        let cur_cwd: PathBuf = std::env::current_dir().unwrap();
        let dir: PathBuf = cur_cwd.join(".mongobar");
        let workdir: PathBuf = dir.join(name);
        let op_file_oplogs = workdir.join(PathBuf::from("oplogs.op"));
        Self {
            name: name.to_string(),
            op_workdir: workdir.clone(),
            op_file_oplogs: op_file_oplogs,
            op_file_revert: workdir.join(PathBuf::from("revert.op")),
            op_file_resume: workdir.join(PathBuf::from("resume.op")),
            op_file_data: workdir.join(PathBuf::from("data.op")),
            config: mongobar_config::MongobarConfig::new(
                cur_cwd.join(PathBuf::from("mongobar.json")),
            ),
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
            fs::write(cwd.clone().join(&self.op_file_oplogs), "").unwrap();
        }

        self.load_state();
        // self.load_op_rows();

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

    pub fn merge_config_rebuild(mut self, rebuild: Option<bool>) -> Self {
        self.config.rebuild = rebuild;
        self
    }

    pub fn merge_config_uri(mut self, uri: Option<String>) -> Self {
        if let Some(uri) = uri {
            self.config.uri = uri;
        }
        self
    }

    pub fn clean(self) -> Self {
        let _ = fs::remove_dir_all(&self.cwd());
        Self::new(&self.name).init()
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

    pub fn add_row_by_profile(&mut self, doc: &Document) {
        let ns = doc.get_str("ns").unwrap().to_string();
        if ns.contains("system.profile") {
            return;
        }
        // let doc_as_json = serde_json::to_string(&doc).unwrap();
        // println!("{}", doc_as_json);
        let mut row = op_row::OpRow::default();
        let op = doc.get_str("op").unwrap();
        let cmd = doc.get_document("command").unwrap();
        match op {
            "query" => {
                if let Err(_) = doc.get_str("queryHash") {
                    return;
                }
                row.id = doc.get_str("queryHash").unwrap().to_string();
                row.ns = ns;
                row.ts = doc.get_datetime("ts").unwrap().timestamp_millis() as i64;
                row.op = op_row::Op::Find;
                row.db = cmd.get_str("$db").unwrap().to_string();
                row.coll = cmd.get_str("find").unwrap().to_string();

                row.cmd = json!(cmd);
            }
            _ => {}
        }
        // println!("{:?}", row);
        op_logs::OpLogs::push_line(self.op_file_oplogs.clone(), row);
    }

    // pub fn load_op_rows(&mut self) {
    //     let content = fs::read_to_string(&self.op_file_padding).unwrap();
    //     let rows: Vec<op_row::OpRow> = content
    //         .split("\n")
    //         .filter(|v| !v.is_empty())
    //         .filter(|v| {
    //             if let Some(filter) = &self.op_filter {
    //                 return filter.is_match(v);
    //             }
    //             return true;
    //         })
    //         .map(|v| serde_json::from_str(v).unwrap())
    //         .collect();

    //     self.op_logs = rows;
    // }
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

    pub async fn op_exec(
        &self,
        exec_file: PathBuf,
        thread_count: u32,
        loop_count: usize,
        mode: op_logs::OpReadMode,
        op_run_mode: OpRunMode,
    ) -> Result<(), anyhow::Error> {
        // let record_start_time = DateTime::from_millis(self.op_state.record_start_ts);
        // let record_end_time = DateTime::from_millis(self.op_state.record_end_ts);

        let mongo_uri: String = self.config.uri.clone();
        {
            let options = ClientOptions::parse(&mongo_uri).await.unwrap();
            let client = Client::with_options(options).unwrap();
            let db = client.database(&self.config.db);

            let cur_profile = db.run_command(doc! {  "profile": -1 }).await?;

            if let Ok(was) = cur_profile.get_i32("was") {
                if was != 0 {
                    db.run_command(doc! { "profile": 0 }).await?;
                }
            }
            client.shutdown().await;
        }

        // println!(
        //     "OPExec [{}] loop_count: {} thread_count: {}",
        //     chrono::Local::now().timestamp(),
        //     loop_count,
        //     thread_count
        // );

        let gate = Arc::new(tokio::sync::Barrier::new(thread_count as usize));
        let mut handles = vec![];

        let dyn_threads = self.indicator.take("dyn_threads").unwrap();
        let dyn_cc_limit = self.indicator.take("dyn_cc_limit").unwrap();

        let boot_worker = self.indicator.take("boot_worker").unwrap();
        let done_worker = self.indicator.take("done_worker").unwrap();
        let query_count = self.indicator.take("query_count").unwrap();
        let query_qps = self.indicator.take("query_qps").unwrap();
        let querying = self.indicator.take("querying").unwrap();
        // let in_size = Arc::new(AtomicUsize::new(0));
        // let out_size = Arc::new(AtomicUsize::new(0));
        let cost_ms = self.indicator.take("cost_ms").unwrap();
        let progress = self.indicator.take("progress").unwrap();
        let progress_total = self.indicator.take("progress_total").unwrap();
        let logs = self.indicator.take("logs").unwrap();
        let signal = Arc::clone(&self.signal);

        self.indicator
            .take("thread_count")
            .unwrap()
            .set(thread_count as usize);
        let mut client_pool = ClientPool::new(&self.config.uri, thread_count * 100);
        let op_logs = Arc::new(op_logs::OpLogs::new(exec_file, mode.clone()).init());

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
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
            // -------------------------------------
            let thread_index = created_thread_count as usize;
            let gate = gate.clone();
            let op_rows = op_logs.clone();

            // let out_size = out_size.clone();
            // let in_size = in_size.clone();
            let query_count = query_count.clone();
            let progress = progress.clone();
            let progress_total = progress_total.clone();
            let cost_ms = cost_ms.clone();
            let boot_worker = boot_worker.clone();
            let logs = logs.clone();
            let client = client_pool.get().await?;
            let signal = Arc::clone(&signal);
            let done_worker = done_worker.clone();
            let dyn_cc_limit = dyn_cc_limit.clone();
            let query_qps = query_qps.clone();
            let querying = querying.clone();
            let thread_count_num = thread_count;
            let mode = mode.clone();
            let op_run_mode = op_run_mode.clone();

            handles.push(tokio::spawn(async move {
                // println!("Thread[{}] [{}]\twait", i, chrono::Local::now().timestamp());
                boot_worker.increment();
                if thread_index < thread_count_num as usize {
                    if loop_count != 1 {
                        gate.wait().await;
                    };
                }
                // println!(
                //     "Thread[{}] [{}]\tstart",
                //     i,
                //     chrono::Local::now().timestamp()
                // );

                // let client = Client::with_uri_str(mongo_uri).await.unwrap();
                let mut loop_index = 0 as usize;

                loop {
                    if loop_count != 0 {
                        loop_index += 1;
                        if loop_index > loop_count as usize {
                            break;
                        }
                    }
                    if signal.get() != 0 {
                        break;
                    }
                    let dyn_cc_limit_n = dyn_cc_limit.get();
                    if dyn_cc_limit_n > 0 && querying.get() >= dyn_cc_limit_n {
                        let rand = rand::random::<u64>() % 100;
                        tokio::time::sleep(tokio::time::Duration::from_millis(rand)).await;
                        continue;
                    }
                    let mut row_index = 0;
                    while let Some(row) = op_rows.read(thread_index, row_index) {
                        if signal.get() != 0 {
                            break;
                        }
                        // if progress.get() >= progress_total.get() {
                        //     break;
                        // }
                        progress.increment();
                        querying.increment();
                        match &row.op {
                            op_row::Op::Find => {
                                let db = client.database(&row.db);
                                // out_size.fetch_add(row.cmd.len(), Ordering::Relaxed);
                                let mut cmd = row.cmd.clone();
                                // println!("before cmd {:?}", cmd);
                                if let Value::Object(ref mut cmd) = cmd {
                                    cmd.remove("lsid");
                                    cmd.remove("$clusterTime");
                                    cmd.remove("$db");
                                    cmd.remove("cursor");
                                    cmd.remove("cursorId");
                                }
                                // println!("after cmd {:?}", cmd);
                                let cmd: Document = Document::deserialize(cmd).unwrap();
                                let start = Instant::now();
                                let res = db.run_cursor_command(cmd).await;
                                let end = start.elapsed();
                                cost_ms.add(end.as_millis() as usize);
                                query_count.increment();
                                if let Err(e) = &res {
                                    logs.push(format!(
                                        "OPExec [{}] [{}] err {}",
                                        chrono::Local::now().timestamp(),
                                        row.id,
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
                            op_row::Op::Count => {
                                let db = client.database(&row.db);
                                let mut cmd = row.cmd.clone();
                                // println!("before cmd {:?}", cmd);
                                if let Value::Object(ref mut cmd) = cmd {
                                    cmd.remove("lsid");
                                    cmd.remove("$clusterTime");
                                    cmd.remove("$db");
                                    cmd.remove("cursor");
                                    cmd.remove("cursorId");
                                }
                                // println!("after cmd {:?}", cmd);
                                let cmd: Document = Document::deserialize(cmd).unwrap();
                                let start = Instant::now();
                                let res = db.run_command(cmd).await;
                                let end = start.elapsed();
                                cost_ms.add(end.as_millis() as usize);
                                query_count.increment();
                                if let Err(e) = &res {
                                    logs.push(format!(
                                        "OPExec [{}] [{}] err {}",
                                        chrono::Local::now().timestamp(),
                                        row.id,
                                        e
                                    ));
                                }
                            }
                            op_row::Op::Aggregate => {
                                let db = client.database(&row.db);
                                // out_size.fetch_add(row.cmd.len(), Ordering::Relaxed);
                                let get_document: Vec<Document> = row
                                    .cmd
                                    .get("pipeline")
                                    .unwrap()
                                    .as_array()
                                    .unwrap()
                                    .iter()
                                    .map(|v| Document::deserialize(v).unwrap())
                                    .collect();
                                let start = Instant::now();
                                let res = db
                                    .collection::<Document>(&row.coll)
                                    .aggregate(get_document)
                                    .await;
                                let end = start.elapsed();
                                cost_ms.add(end.as_millis() as usize);
                                query_count.increment();
                                if let Err(e) = &res {
                                    logs.push(format!(
                                        "OPExec [{}] [{}] err {}",
                                        chrono::Local::now().timestamp(),
                                        row.id,
                                        e
                                    ));
                                }
                            }

                            op_row::Op::GetMore => {
                                let db = client.database(&row.db);
                                let start = Instant::now();
                                let mut cmd = row.cmd.clone();
                                let originating_command =
                                    cmd.get_mut("originatingCommand").map(|v| {
                                        if let Value::Object(ref mut v) = v {
                                            v.remove("lsid");
                                            v.remove("$clusterTime");
                                            v.remove("$db");
                                        }
                                        Document::deserialize(v.to_owned()).unwrap()
                                    });
                                if let Some(oc) = originating_command {
                                    let res = db.run_cursor_command(oc).await;
                                    if let Err(e) = &res {
                                        logs.push(format!(
                                            "OPExec [{}] [{}] getMore Error {}",
                                            chrono::Local::now().timestamp(),
                                            row.id,
                                            e
                                        ));
                                    }
                                } else {
                                    let _ = db
                                        .collection::<Document>(&row.coll)
                                        .find(doc! {})
                                        .limit(row.cmd.get("batchSize").unwrap().as_i64().unwrap());
                                }
                                let end = start.elapsed();
                                cost_ms.add(end.as_millis() as usize);
                                query_count.increment();
                            }
                            op_row::Op::Update => {
                                if let OpRunMode::ReadWrite = op_run_mode {
                                    let db = client.database(&row.db);
                                    let updates =
                                        row.cmd.get("updates").unwrap().as_array().unwrap();
                                    let start = Instant::now();
                                    for update in updates.iter() {
                                        let update = Document::deserialize(update.clone()).unwrap();
                                        let q = update.get_document("q");
                                        if let Ok(q) = q {
                                            let u = update.get_document("u");
                                            if let Ok(u) = u {
                                                let multi =
                                                    update.get_bool("multi").unwrap_or_default();
                                                let upsert =
                                                    update.get_bool("upsert").unwrap_or_default();
                                                let res = db
                                                    .collection::<Document>(&row.coll)
                                                    .update_many(q.clone(), u.clone())
                                                    .await;
                                                if let Err(e) = &res {
                                                    logs.push(format!(
                                                        "OPExec [{}] [{}] Update Err {}",
                                                        chrono::Local::now().timestamp(),
                                                        row.id,
                                                        e
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                    let end = start.elapsed();
                                    cost_ms.add(end.as_millis() as usize);
                                    query_count.increment();
                                }
                            }
                            op_row::Op::Insert => {
                                if let OpRunMode::ReadWrite = op_run_mode {
                                    let db = client.database(&row.db);
                                    let documents =
                                        row.cmd.get("documents").unwrap().as_array().unwrap();

                                    let start = Instant::now();
                                    for doc in documents.iter() {
                                        let mut doc: Document =
                                            Document::deserialize(doc.clone()).unwrap();
                                        doc.remove("__v");
                                        let res = db.collection(&row.coll).insert_one(doc).await;
                                        if let Err(e) = &res {
                                            logs.push(format!(
                                                "OPExec [{}] [{}] Insert Err {}",
                                                chrono::Local::now().timestamp(),
                                                row.id,
                                                e
                                            ));
                                        }
                                    }
                                    let end = start.elapsed();
                                    cost_ms.add(end.as_millis() as usize);
                                    query_count.increment();
                                }
                            }
                            op_row::Op::Delete => {
                                if let OpRunMode::ReadWrite = op_run_mode {
                                    let db = client.database(&row.db);
                                    let deletes =
                                        row.cmd.get("deletes").unwrap().as_array().unwrap();
                                    let start = Instant::now();
                                    for delete in deletes.iter() {
                                        let delete = Document::deserialize(delete.clone()).unwrap();
                                        let q = delete.get_document("q");
                                        if let Ok(q) = q {
                                            let limit = delete.get_i64("limit").unwrap_or(0);
                                            let res = db
                                                .collection::<Document>(&row.coll)
                                                .delete_many(q.clone())
                                                .await;
                                            if let Err(e) = &res {
                                                logs.push(format!(
                                                    "OPExec [{}] [{}] Delete Err {}",
                                                    chrono::Local::now().timestamp(),
                                                    row.id,
                                                    e
                                                ));
                                            }
                                        }
                                    }
                                    let end = start.elapsed();
                                    cost_ms.add(end.as_millis() as usize);
                                    query_count.increment();
                                }
                            }
                            op_row::Op::FindAndModify => {
                                if let OpRunMode::ReadWrite = op_run_mode {
                                    let db = client.database(&row.db);
                                    let query = row.cmd.get("query").unwrap();
                                    let query = Document::deserialize(query.clone()).unwrap();
                                    let start = Instant::now();
                                    let res = db
                                        .collection::<Document>(&row.coll)
                                        .find_one_and_delete(query.clone())
                                        .await;
                                    if let Err(e) = &res {
                                        logs.push(format!(
                                            "OPExec [{}] [{}] FindAndModify Err {}",
                                            chrono::Local::now().timestamp(),
                                            row.id,
                                            e
                                        ));
                                    }
                                    let end = start.elapsed();
                                    cost_ms.add(end.as_millis() as usize);
                                    query_count.increment();
                                }
                            }
                            op_row::Op::None => (),
                        }

                        querying.decrement();
                        row_index += 1;
                    }
                }

                // println!("Thread[{}] [{}]\tend", i, chrono::Local::now().timestamp());

                done_worker.increment();
            }));
            created_thread_count += 1;
            if loop_count == 0 {
                self.indicator.take("progress_total").unwrap().set(0);
            } else {
                if let op_logs::OpReadMode::FullLine(_) = mode {
                    self.indicator.take("progress_total").unwrap().set(
                        op_logs.len()
                            * loop_count as usize
                            * (thread_count as usize + dyn_threads_num),
                    );
                } else {
                    self.indicator
                        .take("progress_total")
                        .unwrap()
                        .set(op_logs.len());
                }
            }
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

        client_pool.shutdown().await;

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
    pub async fn op_stress(&self, filter: Option<String>) -> Result<(), anyhow::Error> {
        let loop_count = self.config.loop_count;
        self.op_exec(
            self.op_file_oplogs.clone(),
            self.config.thread_count,
            loop_count,
            op_logs::OpReadMode::FullLine(filter),
            OpRunMode::Readonly,
        )
        .await?;
        Ok(())
    }

    // 恢复压测前状态
    // 1. 【程序】读取上面标记的时间
    // 2. 【程序】通过时间拉取所有的 oplog.rs
    // 3. 【程序】反向执行所有的操作
    ///
    /// 恢复逻辑：
    ///   insert => 记录 insert id => 执行删除
    ///   update => 查询 该 update 的数据 => 执行 update 还原
    ///   delete => 查询 该 delete 的数据 => 执行 insert
    pub async fn op_revert(&self) -> Result<(), anyhow::Error> {
        let client = Client::with_uri_str(self.config.uri.clone()).await?;

        let op_logs =
            op_logs::OpLogs::new(self.op_file_oplogs.clone(), OpReadMode::StreamLine).init();

        while let Some(op_row) = op_logs.read(0, 0) {
            match op_row.op {
                op_row::Op::None => (),
                op_row::Op::GetMore => (),
                op_row::Op::Aggregate => (),
                op_row::Op::Find => (),
                op_row::Op::Count => (),
                op_row::Op::Insert => {
                    let cmd = op_row.cmd.clone();
                    let _ids = cmd
                        .get("documents")
                        .map(|v| v.as_array().unwrap())
                        .unwrap()
                        .iter()
                        .map(|v| v.get("_id").unwrap())
                        .collect::<Vec<&Value>>();
                    let re_cmd = json!({
                        "deletes": [
                            {
                                "q": {
                                    "_id": {
                                        "$in": _ids
                                    }
                                },
                                "limit": 0
                            }
                        ],
                    });
                    let re_row = op_row::OpRow {
                        id: op_row.id.clone(),
                        ns: op_row.ns.clone(),
                        ts: op_row.ts,
                        op: op_row::Op::Delete,
                        db: op_row.db.clone(),
                        coll: op_row.coll.clone(),
                        cmd: re_cmd,
                    };
                    OpLogs::push_line(self.op_file_revert.clone(), re_row);
                }
                op_row::Op::Update => {
                    let cmd = op_row.cmd.clone();
                    let qs: Vec<Document> = cmd
                        .get("updates")
                        .unwrap()
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| {
                            let q = v.get("q").unwrap();
                            Document::deserialize(q).unwrap()
                        })
                        .collect();

                    for q in qs {
                        let mut res = client
                            .database(&op_row.db)
                            .collection::<Document>(&op_row.coll)
                            .find(q.clone())
                            .await?;

                        while let Some(doc) = res.try_next().await? {
                            let doc = doc.clone();
                            let re_row = op_row::OpRow {
                                id: op_row.id.clone(),
                                ns: op_row.ns.clone(),
                                ts: op_row.ts,
                                op: op_row::Op::Update,
                                db: op_row.db.clone(),
                                coll: op_row.coll.clone(),
                                cmd: json!({
                                    "updates": [
                                        {
                                            "q": {
                                                "_id": doc.get_object_id("_id").unwrap()
                                            },
                                            "u": {
                                                "$set": doc
                                            },
                                            "multi": q.get_bool("multi").unwrap_or_default(),
                                            "upsert": q.get_bool("upsert").unwrap_or_default()
                                        }
                                    ],
                                }),
                            };

                            OpLogs::push_line(self.op_file_revert.clone(), re_row);
                        }
                    }
                }
                op_row::Op::Delete => {
                    // let qs: Vec<&Value> = op_row
                    //     .cmd
                    //     .get("deletes")
                    //     .map(|v| v.as_array().unwrap())
                    //     .unwrap()
                    //     .iter()
                    //     .map(|v| v.get("q").unwrap())
                    //     .collect();

                    // for q in qs {
                    //     let q = Document::deserialize(q).unwrap();
                    //     let mut res = client
                    //         .database(&op_row.db)
                    //         .collection::<Document>(&op_row.coll)
                    //         .find(q.clone())
                    //         .await?;

                    //     while let Some(doc) = res.try_next().await? {
                    //         let doc = json!(doc);
                    //         let cmd = json!({
                    //             "documents": [doc]
                    //         });
                    //         let re_row = op_row::OpRow {
                    //             id: op_row.id.clone(),
                    //             ns: op_row.ns.clone(),
                    //             ts: op_row.ts,
                    //             op: op_row::Op::Insert,
                    //             db: op_row.db.clone(),
                    //             coll: op_row.coll.clone(),
                    //             cmd,
                    //         };

                    //         OpLogs::push_line(self.op_file_revert.clone(), re_row);
                    //     }
                    // }
                }
                op_row::Op::FindAndModify => {
                    // println!("{:?}", op_row);

                    let remove = op_row
                        .cmd
                        .get("remove")
                        .unwrap()
                        .as_bool()
                        .unwrap_or_default();
                    let query = op_row.cmd.get("query").unwrap();

                    let query = Document::deserialize(query).unwrap();

                    let mut res = client
                        .database(&op_row.db)
                        .collection::<Document>(&op_row.coll)
                        .find(query.clone())
                        .await?;

                    while let Some(doc) = res.try_next().await? {
                        let re_row = if remove {
                            op_row::OpRow {
                                id: op_row.id.clone(),
                                ns: op_row.ns.clone(),
                                ts: op_row.ts,
                                op: op_row::Op::Insert,
                                db: op_row.db.clone(),
                                coll: op_row.coll.clone(),
                                cmd: json!({
                                    "documents": [doc]
                                }),
                            }
                        } else {
                            op_row::OpRow {
                                id: op_row.id.clone(),
                                ns: op_row.ns.clone(),
                                ts: op_row.ts,
                                op: op_row::Op::Update,
                                db: op_row.db.clone(),
                                coll: op_row.coll.clone(),
                                cmd: json!({
                                    "updates": [
                                        {
                                            "q": {
                                                "_id": doc.get("_id")
                                            },
                                            "u": {
                                                "$set": doc
                                            },
                                            "multi": false,
                                            "upsert": false
                                        }
                                    ],
                                }),
                            }
                        };

                        OpLogs::push_line(self.op_file_revert.clone(), re_row);
                    }
                }
            }
        }

        reverse_file(self.op_file_revert.to_str().unwrap()).unwrap();

        Ok(())
    }

    pub async fn op_resume(&self) -> Result<(), anyhow::Error> {
        // self.op_exec(1, OpReadMode::StreamLine, OpRunMode::ReadWrite)
        //     .await?;
        let client: Client = Client::with_uri_str(self.config.uri.clone()).await?;

        let op_logs =
            op_logs::OpLogs::new(self.op_file_oplogs.clone(), OpReadMode::StreamLine).init();

        while let Some(op_row) = op_logs.read(0, 0) {
            match op_row.op {
                op_row::Op::None => (),
                op_row::Op::GetMore => (),
                op_row::Op::Aggregate => (),
                op_row::Op::Find => (),
                op_row::Op::Count => (),
                op_row::Op::Insert => {
                    let cmd = op_row.cmd.clone();
                    let values = cmd
                        .get("documents")
                        .map(|v| v.as_array().unwrap())
                        .unwrap()
                        .iter()
                        .collect::<Vec<&Value>>();

                    for val in values {
                        let _id = val.get("_id").unwrap();
                        let doc = Document::deserialize(val).unwrap();
                        let mut res = client
                            .database(&op_row.db)
                            .collection::<Document>(&op_row.coll)
                            .find(doc! {
                                "_id": doc.get_object_id("_id").unwrap()
                            })
                            .await?;

                        if let Some(doc) = res.try_next().await? {
                            // 如果当前有则更新一下
                            let re_row = op_row::OpRow {
                                id: op_row.id.clone(),
                                ns: op_row.ns.clone(),
                                ts: op_row.ts,
                                op: op_row::Op::Update,
                                db: op_row.db.clone(),
                                coll: op_row.coll.clone(),
                                cmd: json!({
                                    "updates": [
                                        {
                                            "q": {
                                                "_id": _id
                                            },
                                            "u": {
                                                "$set": doc
                                            },
                                            "multi": false,
                                            "upsert": false,
                                        }
                                    ],
                                }),
                            };

                            OpLogs::push_line(self.op_file_resume.clone(), re_row);
                        } else {
                            // 如果查到当前没有就删除掉
                            let re_row = op_row::OpRow {
                                id: op_row.id.clone(),
                                ns: op_row.ns.clone(),
                                ts: op_row.ts,
                                op: op_row::Op::Delete,
                                db: op_row.db.clone(),
                                coll: op_row.coll.clone(),
                                cmd: json!({
                                    "deletes": [
                                        {
                                            "q": {
                                                "_id": _id
                                            },
                                            "limit": 0
                                        }
                                    ],
                                }),
                            };
                            OpLogs::push_line(self.op_file_resume.clone(), re_row);
                        }
                    }
                }
                op_row::Op::Update => {
                    let cmd = op_row.cmd.clone();
                    let qs: Vec<Document> = cmd
                        .get("updates")
                        .unwrap()
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| {
                            let q = v.get("q").unwrap();
                            Document::deserialize(q).unwrap()
                        })
                        .collect();

                    for q in qs {
                        let mut res = client
                            .database(&op_row.db)
                            .collection::<Document>(&op_row.coll)
                            .find(q.clone())
                            .await?;

                        while let Some(doc) = res.try_next().await? {
                            let doc = doc.clone();
                            let re_row = op_row::OpRow {
                                id: op_row.id.clone(),
                                ns: op_row.ns.clone(),
                                ts: op_row.ts,
                                op: op_row::Op::Update,
                                db: op_row.db.clone(),
                                coll: op_row.coll.clone(),
                                cmd: json!({
                                    "updates": [
                                        {
                                            "q": {
                                                "_id": doc.get_object_id("_id").unwrap()
                                            },
                                            "u": {
                                                "$set": doc
                                            },
                                            "multi": q.get_bool("multi").unwrap_or_default(),
                                            "upsert": q.get_bool("upsert").unwrap_or_default()
                                        }
                                    ],
                                }),
                            };

                            OpLogs::push_line(self.op_file_resume.clone(), re_row);
                        }
                    }
                }
                op_row::Op::Delete => {}
                op_row::Op::FindAndModify => {
                    // println!("{:?}", op_row);

                    let remove = op_row
                        .cmd
                        .get("remove")
                        .unwrap()
                        .as_bool()
                        .unwrap_or_default();

                    if !remove {
                        // 更新的情况
                        let query = op_row.cmd.get("query").unwrap();
                        let query = Document::deserialize(query).unwrap();

                        let mut res = client
                            .database(&op_row.db)
                            .collection::<Document>(&op_row.coll)
                            .find(query.clone())
                            .await?;

                        while let Some(doc) = res.try_next().await? {
                            let re_row = op_row::OpRow {
                                id: op_row.id.clone(),
                                ns: op_row.ns.clone(),
                                ts: op_row.ts,
                                op: op_row::Op::Update,
                                db: op_row.db.clone(),
                                coll: op_row.coll.clone(),
                                cmd: json!({
                                    "updates": [
                                        {
                                            "q": {
                                                "_id": doc.get("_id")
                                            },
                                            "u": {
                                                "$set": doc
                                            },
                                            "multi": false,
                                            "upsert": false
                                        }
                                    ],
                                }),
                            };

                            OpLogs::push_line(self.op_file_resume.clone(), re_row);
                        }
                    }
                }
            }
        }

        reverse_file(self.op_file_resume.to_str().unwrap()).unwrap();

        Ok(())
    }

    /// 回放压测文件
    /// 1. 【程序】读取文件
    /// 2. 【程序】通过文件生成 恢复操作（首次操作）
    /// 3. 【程序】执行恢复 op_revert 操作， 这会将这这段时间内地操作还原
    /// 4. 【程序】执行压测 op_stress 操作，这会将这段时间内地操作再次执行（只执行 1 遍）
    pub async fn op_replay(&self) -> Result<(), anyhow::Error> {
        if !self.op_file_revert.exists() || self.config.rebuild.unwrap_or_default() {
            let _ = fs::remove_file(&self.op_file_revert);
            self.op_revert().await?;
            let _ = fs::remove_file(&self.op_file_resume);
            self.op_resume().await?;
        }
        let logs = self.indicator.take("logs").unwrap();
        logs.push(format!("OPReplay op_exec revert.op running...",));
        logs.push(format!("OPReplay op_exec oplogs.op waiting...",));
        logs.push(format!("OPReplay op_exec resume.op waiting...",));
        self.fork(Indicator::new())
            .op_exec(
                self.op_file_revert.clone(),
                1,
                1,
                op_logs::OpReadMode::StreamLine,
                OpRunMode::ReadWrite,
            )
            .await?;
        logs.update(0, format!("OPReplay op_exec revert.op done"));
        logs.update(1, format!("OPReplay op_exec oplogs.op running..."));
        self.op_exec(
            self.op_file_oplogs.clone(),
            self.config.thread_count,
            1,
            op_logs::OpReadMode::StreamLine,
            OpRunMode::ReadWrite,
        )
        .await?;
        logs.update(1, format!("OPReplay op_exec oplogs.op done"));
        logs.update(2, format!("OPReplay op_exec resume.op running..."));
        self.fork(Indicator::new())
            .op_exec(
                self.op_file_resume.clone(),
                1,
                1,
                op_logs::OpReadMode::StreamLine,
                OpRunMode::ReadWrite,
            )
            .await?;
        logs.update(2, format!("OPReplay op_exec resume.op done"));
        Ok(())
    }

    /// 将线上相关的数据拉取到本地文件
    pub async fn op_export(&self) -> Result<(), anyhow::Error> {
        let _ = fs::remove_file(&self.op_file_data);
        let client: Client = Client::with_uri_str(self.config.uri.clone()).await?;

        let op_logs =
            op_logs::OpLogs::new(self.op_file_oplogs.clone(), OpReadMode::StreamLine).init();

        while let Some(op_row) = op_logs.read(0, 0) {
            match op_row.op {
                op_row::Op::None => (),
                op_row::Op::GetMore => (),
                op_row::Op::Aggregate => (),
                op_row::Op::Find => {
                    // let query = op_row.cmd.get("filter");
                    // if let Some(query) = query {
                    //     let query: Document = Document::deserialize(query).unwrap();

                    //     let mut res = client
                    //         .database(&op_row.db)
                    //         .collection::<Document>(&op_row.coll)
                    //         .find(query)
                    //         .await?;

                    //     while let Some(doc) = res.try_next().await? {
                    //         let doc = doc.clone();
                    //         let re_row = op_row::OpRow {
                    //             id: op_row.id.clone(),
                    //             ns: op_row.ns.clone(),
                    //             ts: op_row.ts,
                    //             op: op_row::Op::Insert,
                    //             db: op_row.db.clone(),
                    //             coll: op_row.coll.clone(),
                    //             cmd: json!({
                    //                 "documents": [doc]
                    //             }),
                    //         };

                    //         OpLogs::push_line(self.op_file_data.clone(), re_row);
                    //     }
                    // }
                }
                op_row::Op::Count => {
                    // let query = op_row.cmd.get("query");
                    // if let Some(query) = query {
                    //     let query: Document = Document::deserialize(query).unwrap();

                    //     let mut res = client
                    //         .database(&op_row.db)
                    //         .collection::<Document>(&op_row.coll)
                    //         .find(query)
                    //         .await?;

                    //     while let Some(doc) = res.try_next().await? {
                    //         let doc = doc.clone();
                    //         let re_row = op_row::OpRow {
                    //             id: op_row.id.clone(),
                    //             ns: op_row.ns.clone(),
                    //             ts: op_row.ts,
                    //             op: op_row::Op::Insert,
                    //             db: op_row.db.clone(),
                    //             coll: op_row.coll.clone(),
                    //             cmd: json!({
                    //                 "documents": [doc]
                    //             }),
                    //         };

                    //         OpLogs::push_line(self.op_file_data.clone(), re_row);
                    //     }
                    // }
                }
                op_row::Op::Insert => {}
                op_row::Op::Update => {
                    let cmd = op_row.cmd.clone();
                    let qs: Vec<Document> = cmd
                        .get("updates")
                        .unwrap()
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| {
                            let q = v.get("q").unwrap();
                            Document::deserialize(q).unwrap()
                        })
                        .collect();

                    for q in qs {
                        let mut res = client
                            .database(&op_row.db)
                            .collection::<Document>(&op_row.coll)
                            .find(q.clone())
                            .await?;

                        while let Some(doc) = res.try_next().await? {
                            let doc = doc.clone();
                            let re_row = op_row::OpRow {
                                id: op_row.id.clone(),
                                ns: op_row.ns.clone(),
                                ts: op_row.ts,
                                op: op_row::Op::Insert,
                                db: op_row.db.clone(),
                                coll: op_row.coll.clone(),
                                cmd: json!({
                                    "documents": [doc]
                                }),
                            };

                            OpLogs::push_line(self.op_file_data.clone(), re_row);
                        }
                    }
                }
                op_row::Op::Delete => {}
                op_row::Op::FindAndModify => {
                    // println!("{:?}", op_row);

                    let remove = op_row
                        .cmd
                        .get("remove")
                        .unwrap()
                        .as_bool()
                        .unwrap_or_default();

                    if !remove {
                        // 更新的情况
                        let query = op_row.cmd.get("query").unwrap();
                        let query = Document::deserialize(query).unwrap();

                        let mut res = client
                            .database(&op_row.db)
                            .collection::<Document>(&op_row.coll)
                            .find(query.clone())
                            .await?;

                        while let Some(doc) = res.try_next().await? {
                            let re_row = op_row::OpRow {
                                id: op_row.id.clone(),
                                ns: op_row.ns.clone(),
                                ts: op_row.ts,
                                op: op_row::Op::Insert,
                                db: op_row.db.clone(),
                                coll: op_row.coll.clone(),
                                cmd: json!({
                                    "documents": [doc]
                                }),
                            };

                            OpLogs::push_line(self.op_file_data.clone(), re_row);
                        }
                    }
                }
            }
        }

        reverse_file(self.op_file_data.to_str().unwrap()).unwrap();

        Ok(())
    }

    /// 将本地文件导入到连接的数据库
    pub async fn op_import(&self) -> Result<(), anyhow::Error> {
        self.op_exec(
            self.op_file_data.clone(),
            1,
            1,
            op_logs::OpReadMode::StreamLine,
            OpRunMode::ReadWrite,
        )
        .await?;

        Ok(())
    }

    fn fork(&self, indic: Indicator) -> Self {
        self.clone().set_indicator(indic).init()
    }
}

// fn bytes_to_mb(bytes: usize) -> f64 {
//     bytes as f64 / 1024.0 / 1024.0
// }

struct ClientPool {
    uri: String,
    clients: Vec<Arc<Client>>,
    every_size: u32,
    get_index: usize,
}

impl ClientPool {
    fn new(uri: &str, every_size: u32) -> Self {
        let clients = vec![];

        Self {
            clients,
            every_size,
            uri: uri.to_string(),
            get_index: 0,
        }
    }

    async fn get(&mut self) -> Result<Arc<Client>, anyhow::Error> {
        let len = self.clients.len();
        let total = len * self.every_size as usize;
        if total <= self.get_index {
            let mut options = ClientOptions::parse(&self.uri).await?;
            options.max_pool_size = Some(self.every_size + 1);
            options.min_pool_size = Some(self.every_size / 100 + 1);
            let client = Arc::new(Client::with_options(options).unwrap());
            self.clients.push(client);
        }

        let block_index = self.get_index / self.every_size as usize;
        let client = Arc::clone(&self.clients[block_index]);

        self.get_index = self.get_index + 1;

        Ok(client)
    }

    async fn shutdown(self) {
        for client in self.clients {
            Arc::try_unwrap(client).unwrap().shutdown().await;
        }
    }
}
