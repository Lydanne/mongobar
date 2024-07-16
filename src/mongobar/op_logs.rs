use std::fs::{self, File, OpenOptions};

use std::path::PathBuf;

use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::sync::atomic::AtomicUsize;
use std::sync::RwLock;

use ratatui::buffer;

use super::op_row::{self, OpRow};

static BUFF_SIZE: usize = 3;

#[derive(Debug)]
pub enum OpReadMode {
    StreamLine,
    FullLine,
}

#[derive(Debug)]
pub struct OpLogs {
    pub buffers: [RwLock<Vec<op_row::OpRow>>; 2],
    pub full_buffer: Vec<op_row::OpRow>,
    pub offset: AtomicUsize,
    pub index: AtomicUsize,
    pub op_file: PathBuf,
    pub length: usize,
    pub mode: OpReadMode,
}

impl OpLogs {
    pub fn new(op_file: PathBuf, mode: OpReadMode) -> Self {
        Self {
            op_file: op_file.clone(),
            length: count_lines(op_file.to_str().unwrap()),
            // buffer: RwLock::new(Vec::new()),
            // next_buffer: RwLock::new(Vec::new()),
            buffers: [RwLock::new(Vec::new()), RwLock::new(Vec::new())],
            full_buffer: Vec::new(),
            offset: AtomicUsize::new(0),
            index: AtomicUsize::new(0),
            mode,
        }
    }

    pub fn init(mut self) -> Self {
        match self.mode {
            OpReadMode::StreamLine => {
                // self.load_stream_line(0);
                // self.load_stream_line(1);
            }
            OpReadMode::FullLine => {
                self.load_full_line();
            }
        }
        self
    }

    pub fn load_stream_line(&self, active: usize) -> usize {
        let offset = self.offset.load(std::sync::atomic::Ordering::SeqCst);
        let buffer = read_file_part(self.op_file.to_str().unwrap(), offset, BUFF_SIZE);
        let len = buffer.len();
        // let load_index = offset / BUFF_SIZE % 2;
        *self.buffers.get(active).unwrap().write().unwrap() = buffer;
        self.offset
            .store(offset + BUFF_SIZE, std::sync::atomic::Ordering::SeqCst);
        len
    }

    pub fn load_full_line(&mut self) {
        let buffer = read_file_part(self.op_file.to_str().unwrap(), 0, self.length);
        self.full_buffer = buffer;
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

    pub fn limit(&self, start: usize, length: usize) -> Vec<op_row::OpRow> {
        return read_file_part(self.op_file.to_str().unwrap(), start, length);
    }

    pub fn read(&self, row_index: usize) -> Option<op_row::OpRow> {
        match self.mode {
            OpReadMode::StreamLine => {
                let index = self.index.load(std::sync::atomic::Ordering::SeqCst);
                self.index
                    .store(index + 1, std::sync::atomic::Ordering::SeqCst);

                if index > self.length {
                    return None;
                }
                let buffer_index = index % BUFF_SIZE;

                let active = index / BUFF_SIZE % 2;

                if buffer_index == 0 {
                    self.load_stream_line(active);
                }

                let buffer = self.buffers.get(active).unwrap().read().unwrap();
                let row = buffer.get(buffer_index);
                // println!(
                //     "index: {:?}",
                //     (
                //         index,
                //         buffer_index,
                //         active,
                //         self.offset.load(std::sync::atomic::Ordering::SeqCst)
                //     )
                // );
                return row.cloned();
            }
            OpReadMode::FullLine => {
                if row_index < self.length {
                    return Some(self.full_buffer[row_index].clone());
                } else {
                    return None;
                }
            }
        }
    }
}

fn count_lines(file_path: &str) -> usize {
    let file = File::open(file_path).expect("Failed to open file");
    let mut reader = BufReader::new(file);
    let mut line_count = 0;

    let mut buffer = String::new();
    while reader.read_line(&mut buffer).unwrap_or(0) > 0 {
        line_count += 1;
        buffer.clear();
    }

    line_count
}

fn read_file_part(file_path: &str, start: usize, length: usize) -> Vec<OpRow> {
    let file = File::open(file_path).unwrap();
    let reader = BufReader::new(file.try_clone().unwrap());
    let mut buffer = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        if i >= start && i < start + length {
            let row: op_row::OpRow = serde_json::from_str(&line.unwrap()).unwrap();
            buffer.push(row);
        }
    }

    return buffer;
}
