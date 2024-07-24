use std::{
    fs::File,
    io::Write,
    sync::{Arc, Mutex},
};

use serde_json::Value;
use sha3::{
    digest::{ExtendableOutput, Update, XofReader},
    Shake128,
};

use crate::{
    analyze::{each_alilog_csv, watch_progress},
    mongobar::op_row::{Op, OpRow},
    utils::{count_lines, match_date_replace},
};

pub fn convert_alilog_csv(csv_path: &str, filter_db: String) -> Result<(), anyhow::Error> {
    println!("convert_alilog_csv: {}", csv_path);
    let file: File = File::open(csv_path)?;
    let current = watch_progress(count_lines(csv_path));
    let w_file = File::create("oplogs.op")?;
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
    Ok(())
}

fn to_sha3(s: &str) -> String {
    let mut hasher = Shake128::default();

    hasher.update(s.as_bytes());

    let mut output = [0u8; 16];
    hasher.finalize_xof().read(&mut output);

    // 输出结果
    hex::encode(output)
}
