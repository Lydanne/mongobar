use std::io::{stdout, BufRead, Write};

use crate::{mongobar::op_row::OpRow, utils::to_sha3_8};

pub fn reg_filter_line(target: &str, filter: &str) -> usize {
    let mut line_number = 0;
    let re = regex::Regex::new(filter).unwrap();
    let file = std::fs::File::open(target).unwrap();
    let reader = std::io::BufReader::new(file);
    for line in reader.lines() {
        let line = line.unwrap();
        if re.is_match(&line) {
            println!("{}", line);
            line_number += 1;
        }
    }

    line_number
}

pub fn mode_filter_line(target: &str, mode: &str) -> usize {
    let mut line_number = 0;
    let file = std::fs::File::open(target).unwrap();
    let reader = std::io::BufReader::new(file);
    for line in reader.lines() {
        let line = line.unwrap();
        if line.trim().is_empty() {
            continue;
        }
        let row = serde_json::from_str::<OpRow>(&line).unwrap();
        if row.build_key() == mode {
            println!("{}", line);
            line_number += 1;
        }
    }

    line_number
}
