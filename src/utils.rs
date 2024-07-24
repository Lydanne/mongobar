use std::fs::File;
use std::io::{BufRead, BufReader};

pub fn count_lines(file_path: &str) -> usize {
    let file = File::open(file_path).unwrap();
    let reader = BufReader::new(file);
    reader.lines().count()
}
