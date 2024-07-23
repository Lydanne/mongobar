use once_cell::sync::Lazy;
use serde_json::Value;

use csv::Reader;
use rayon::prelude::*;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use crate::indicator::Metric;

#[derive(Debug, Deserialize)]
struct Record {
    __source__: String,
    __time__: u64,
    __topic__: String,
    audit_type: String,
    coll: String,
    command: String,
    db: String,
    docs_examined: Option<u64>,
    instanceid: String,
    keys_examined: Option<u64>,
    latency: u64,
    optype: String,
    return_num: Option<f64>,
    thread_id: String,
    time: u64,
    user: String,
    user_ip: String,
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
    let file = File::open(path)?;
    let reader = std::io::BufReader::new(file);
    // let rdr = Arc::new(Mutex::new(csv::Reader::from_reader(reader)));
    let mut rdr = csv::Reader::from_reader(reader);

    let current = Arc::new(Metric::default());
    let total_lines = 1000000;

    thread::spawn({
        let current = Arc::clone(&current);
        let mut last_tick = Instant::now();
        let mut last_current = 0;

        move || loop {
            if last_tick.elapsed().as_secs() >= 1 {
                let current = current.get() as u64;
                let speed = (current - last_current) / last_tick.elapsed().as_secs();
                last_current = current;
                last_tick = Instant::now();
                println!(
                    "[{:.2}%]({}/{}) {}/s",
                    (current as f64 / total_lines as f64) * 100.0,
                    current,
                    total_lines,
                    speed as f64
                );
            }
            thread::sleep(std::time::Duration::from_secs(1));
        }
    });

    rdr.records().par_bridge().for_each(|result| {
        if let Ok(record) = result {
            current.add(1);
            let record: Record = record.deserialize(None).unwrap();
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
        }
    });

    // let mut childs = vec![];
    // for _ in 1..100 {
    //     let c = thread::spawn({
    //         let map = Arc::clone(&map);
    //         let current = Arc::clone(&current);
    //         let rdr = Arc::clone(&rdr);
    //         move || {
    //             loop {
    //                 let result = {
    //                     let mut rdr = rdr.lock().unwrap();
    //                     rdr.records().next()
    //                 };
    //                 if let Some(result) = result {
    //                     // The iterator yields Result<StringRecord, Error>, so we check the
    //                     // error here.
    //                     let record = result.unwrap();
    //                     current.add(record.len());
    //                     let record: Record = record.deserialize(None).unwrap();
    //                     // let command: Value = serde_json::from_str(&record.command)?;
    //                     let key = format!(
    //                         "{}:{}#{}",
    //                         record.coll,
    //                         record.optype,
    //                         match_keys(&record.command).join(":")
    //                     );
    //                     let mut map = map.lock().unwrap();
    //                     if let Some(v) = map.get_mut(&key) {
    //                         *v += 1;
    //                     } else {
    //                         map.insert(key, 1);
    //                     }
    //                 } else {
    //                     break;
    //                 }
    //             }
    //         }
    //     });
    //     childs.push(c);
    // }

    // for c in childs {
    //     c.join().unwrap();
    // }

    // println!("{:?}", map);

    // 生成 csv
    let map = map.lock().unwrap();
    println!("done {}", map.len());

    let wtr = File::create("result.csv")?;
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
