use std::{
    fs::File,
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use bson::doc;
use serde_json::Value;

use crate::tool::analyze::{each_alilog_csv, watch_progress};
use crate::{
    mongobar::op_row::{Op, OpRow},
    utils::{count_lines, match_date_replace, to_sha3},
};

pub fn convert_alilog_csv(csv_path: &str, filter_db: String) -> Result<PathBuf, anyhow::Error> {
    println!("convert_alilog_csv: {}", csv_path);
    let file: File = File::open(csv_path)?;
    let current = watch_progress("Convert".to_string(), count_lines(csv_path));
    let out_path = PathBuf::from(format!("oplogs.{}.op", chrono::Local::now().timestamp()));
    let w_file = File::create(out_path.clone())?;
    let writer = Arc::new(Mutex::new(std::io::BufWriter::new(w_file)));

    each_alilog_csv(file, |record| {
        current.add(1);
        if filter_db.len() > 0 && record.db != filter_db {
            return;
        }
        let cmd: Value = serde_json::from_str(&record.command).unwrap();
        let op_row = OpRow {
            id: to_sha3(record.command.as_str()),
            op: Op::from(record.optype.clone()),
            ns: format!("{}.{}", record.db, record.coll),
            db: record.db,
            coll: record.coll,
            cmd: cmd.get("args").unwrap().to_owned(),
            ts: record.time as i64,
            args: doc! {},
            key: String::new(),
        };
        // println!("{}", serde_json::to_string(&op_row).unwrap());
        // println!("{:?}", record);
        writer
            .lock()
            .unwrap()
            .write_all(
                format!(
                    "{}\n",
                    match_date_replace(&serde_json::to_string(&op_row).unwrap())
                )
                .as_bytes(),
            )
            .unwrap();
    });
    Ok(out_path)
}
