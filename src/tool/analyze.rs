use once_cell::sync::Lazy;

use rayon::prelude::*;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use crate::indicator::Metric;
use crate::utils::count_lines;

#[derive(Debug, Deserialize)]
pub struct Record {
    pub __source__: String,
    pub __time__: u64,
    pub __topic__: String,
    pub audit_type: String,
    pub coll: String,
    pub command: String,
    pub db: String,
    pub docs_examined: Option<u64>,
    pub instanceid: String,
    pub keys_examined: Option<u64>,
    pub latency: u64,
    pub optype: String,
    pub return_num: Option<f64>,
    pub thread_id: String,
    pub time: u64,
    pub user: String,
    pub user_ip: String,
}

#[derive(Debug, Deserialize)]
struct Stat {
    count: u64,
    latency: u64,
    eg: Vec<String>,
}

pub fn analysis_alilog_csv(path: &str) -> Result<(), anyhow::Error> {
    println!("analysis_alilog_csv: {}", path);
    let map = Arc::new(Mutex::new(HashMap::<String, Stat>::new()));
    let file: File = File::open(path)?;

    let total_lines = count_lines(path);

    let path_buf = std::path::PathBuf::from(path);

    let current = watch_progress("Analysis".to_string(), total_lines);

    each_alilog_csv(file, |record| {
        current.add(1);
        // let command: Value = serde_json::from_str(&record.command)?;
        let key = format!(
            "{}:{}#{}",
            record.coll,
            record.optype,
            match_keys(&record.command).join(":")
        );
        let mut map = map.lock().unwrap();
        if let Some(v) = map.get_mut(&key) {
            v.latency += record.latency;
            v.count += 1;
        } else {
            map.insert(
                key,
                Stat {
                    count: 1,
                    latency: record.latency,
                    eg: vec![record.command],
                },
            );
        }
    });

    // 生成 csv
    let map = map.lock().unwrap();
    println!("done {}", map.len());

    let wtr = File::create(format!(
        "ana-{}.csv",
        path_buf.file_stem().unwrap().to_str().unwrap()
    ))?;
    let mut wtr = csv::Writer::from_writer(wtr);
    wtr.write_record(&["key", "count", "latency", "eg"])?;
    for (k, v) in map.iter() {
        wtr.write_record(&[
            k,
            &v.count.to_string(),
            &((v.latency as f64) / (v.count as f64)).to_string(),
            &v.eg.join("\n").replace(",", "，").replace("\"", "'"),
        ])?;
    }

    Ok(())
}

pub fn each_alilog_csv<CB: Fn(Record) + Sync + Send>(file: File, cb: CB) {
    let reader = std::io::BufReader::new(file);
    let mut rdr = csv::Reader::from_reader(reader);
    rdr.records().par_bridge().for_each(|result| {
        if let Ok(record) = result {
            let record: Record = record.deserialize(None).unwrap();
            cb(record);
        }
    });
}

pub fn watch_progress(name: String, total_lines: usize) -> Arc<Metric> {
    let current: Arc<Metric> = Arc::new(Metric::default());

    thread::spawn({
        let current = current.clone();
        let mut last_tick = Instant::now();
        let mut last_current = 0;

        move || loop {
            if last_tick.elapsed().as_secs() >= 1 {
                let current = current.get() as u64;

                if current >= (total_lines - 1) as u64 {
                    break;
                }

                let speed = (current - last_current) / last_tick.elapsed().as_secs();
                last_current = current;
                last_tick = Instant::now();
                println!(
                    "{} Progress[{:.2}%]({}/{}) {}/s",
                    name,
                    (current as f64 / total_lines as f64) * 100.0,
                    current,
                    total_lines,
                    speed as f64
                );
            }
            thread::sleep(std::time::Duration::from_secs(1));
        }
    });

    current
}

static IGNORE_KEYS: Lazy<HashSet<&str>> = Lazy::new(|| {
    HashSet::from([
        "command",
        "ns",
        "cursorId",
        "args",
        "singleBatch",
        "batchSize",
        "lsid",
        "clusterTime",
        "t",
        "i",
        "signature",
        "hash",
        "keyId",
        "replRole",
        "repRole",
        "stateStr",
        "mode",
    ])
});

static REG: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r#""(\w+)":\s*"#).unwrap());

/// regex match keys
pub fn match_keys(json: &String) -> Vec<String> {
    let mut keys = HashSet::new();
    for cap in REG.captures_iter(json) {
        let k = cap[1].to_string();
        if IGNORE_KEYS.contains(k.as_str()) {
            continue;
        }
        keys.insert(k);
    }
    // sort
    let mut keys: Vec<String> = keys.into_iter().collect();
    keys.sort();
    keys
}
