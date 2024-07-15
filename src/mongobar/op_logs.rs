use std::fs::{self, OpenOptions};

use std::path::PathBuf;

use std::io::Write;

use super::op_row;

#[derive(Clone, Debug, Default)]
pub struct OpLogs {
    pub logs: Vec<op_row::OpRow>,
    pub op_file: PathBuf,
    pub length: usize,
}

impl OpLogs {
    pub fn new(op_file: PathBuf) -> Self {
        let logs = OpLogs::load_op_rows(op_file.clone());
        Self {
            op_file,
            length: logs.len(),
            logs,
        }
    }
    pub fn load_op_rows(op_file: PathBuf) -> Vec<op_row::OpRow> {
        let content = fs::read_to_string(op_file).unwrap();
        let rows: Vec<op_row::OpRow> = content
            .split("\n")
            .filter(|v| !v.is_empty())
            // .filter(|v| {
            //     if let Some(filter) = &self.op_filter {
            //         return filter.is_match(v);
            //     }
            //     return true;
            // })
            .map(|v| serde_json::from_str(v).unwrap())
            .collect();
        rows
    }
    pub fn push(&self, row: op_row::OpRow) {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&self.op_file)
            .unwrap();
        let content = serde_json::to_string(&row.clone()).unwrap();
        writeln!(file, "{}", content).unwrap();
    }

    pub fn len(&self) -> usize {
        return self.length;
    }

    pub fn limit(&self, start: usize, length: usize) -> &[op_row::OpRow] {
        let c_len = self.logs.len();
        let start = if start >= c_len { c_len - 1 } else { start };
        let end = if start + length > c_len {
            c_len - 1
        } else {
            start + length
        };
        return &self.logs[start..end];
    }

    pub fn iter(&self, thread_index: usize, row_index: usize) -> Option<&op_row::OpRow> {
        let item = self.logs.get(row_index);
        item
    }
}
