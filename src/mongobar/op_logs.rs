use std::fs::{self, File, OpenOptions};

use std::path::PathBuf;

use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex, RwLock};

use ratatui::buffer;
use regex::Regex;

use super::op_row::{self, OpRow};

static BUFF_SIZE: usize = 10;

#[derive(Debug, Clone)]
pub enum OpReadMode {
    StreamLine,
    FullLine(Option<String>),
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
    pub lock: Arc<Mutex<()>>,
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
            lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn init(mut self) -> Self {
        match &self.mode {
            OpReadMode::StreamLine => {
                self.load_stream_line(0, 0);
                // self.load_stream_line(1);
            }
            OpReadMode::FullLine(filter) => {
                self.load_full_line(filter.clone());
            }
        }
        self
    }

    pub fn load_stream_line(&self, active: usize, offset: usize) -> usize {
        // let _guard = self.lock.lock().unwrap();
        let mut buffer_write = self.buffers.get(active).unwrap().write().unwrap();
        // if buffer_write.is_err() {
        //     println!("load_stream_line wait: {}", active);
        //     std::thread::sleep(std::time::Duration::from_millis(1));
        //     while self.offset.load(std::sync::atomic::Ordering::SeqCst) == offset {
        //         std::thread::sleep(std::time::Duration::from_millis(1));
        //     }
        //     return 1;
        // }
        // let mut buffer_write = buffer_write.unwrap();
        // println!("load_stream_line active: {}", active);

        self.offset
            .store(offset + BUFF_SIZE, std::sync::atomic::Ordering::SeqCst);

        let buffer: Vec<OpRow> = read_file_part(self.op_file.to_str().unwrap(), offset, BUFF_SIZE)
            .iter()
            .map(|line: &String| serde_json::from_str(&line).unwrap_or_default())
            .collect();
        let len = buffer.len();

        *buffer_write = buffer;

        len
    }

    pub fn load_full_line(&mut self, filter: Option<String>) {
        let buffer: Vec<OpRow> = read_file_part(self.op_file.to_str().unwrap(), 0, self.length)
            .iter()
            .filter(|line| {
                if let Some(filter) = &filter {
                    let filter = Regex::new(filter).unwrap();
                    return filter.is_match(line);
                }
                return true;
            })
            .map(|line| serde_json::from_str(&line).unwrap())
            .collect();
        self.full_buffer = buffer;
        self.length = self.full_buffer.len();
    }

    pub fn push_line(op_file: PathBuf, row: op_row::OpRow) {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(op_file)
            .unwrap();
        let content = serde_json::to_string(&row.clone()).unwrap();
        writeln!(file, "{}", content).unwrap();
    }

    pub fn len(&self) -> usize {
        return self.length;
    }

    pub fn limit(&self, start: usize, length: usize) -> Vec<op_row::OpRow> {
        return read_file_part(self.op_file.to_str().unwrap(), start, length)
            .iter()
            .filter(|line| {
                if let OpReadMode::FullLine(Some(filter)) = &self.mode {
                    let filter = Regex::new(filter).unwrap();
                    return filter.is_match(line);
                }
                return true;
            })
            .map(|line| serde_json::from_str(&line).unwrap())
            .collect();
    }

    pub fn read(&self, thread_index: usize, row_index: usize) -> Option<op_row::OpRow> {
        match self.mode {
            OpReadMode::StreamLine => {
                // let _guard = self.lock.lock().unwrap();

                let index = self.index.load(std::sync::atomic::Ordering::SeqCst);

                if index >= self.length {
                    return None;
                }

                // println!("[{}] index: {:?} {}", thread_index, index, self.length);

                self.index.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                let buffer_index = index % BUFF_SIZE;

                let active = index / BUFF_SIZE % 2;

                let offset: usize = self.offset.load(std::sync::atomic::Ordering::SeqCst);

                if index + BUFF_SIZE / 2 >= offset {
                    // println!(
                    //     "[{}] load_stream_line start: index: {} offset: {}",
                    //     thread_index, index, offset
                    // );

                    self.load_stream_line((active + 1) % 2, offset);

                    // println!(
                    //     "[{}] load_stream_line end: index: {} offset: {}",
                    //     thread_index, index, offset
                    // );
                }

                let buffer = self.buffers.get(active).unwrap().read().unwrap();

                let row: Option<&OpRow> = buffer.get(buffer_index);

                // println!(
                //     "[{}] index: {:?} offset: {:?} active: {:?} {}",
                //     thread_index,
                //     index,
                //     offset,
                //     active,
                //     row.is_none()
                // );

                return row.cloned();
            }
            OpReadMode::FullLine(_) => {
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

fn read_file_part(file_path: &str, start: usize, length: usize) -> Vec<String> {
    let file = File::open(file_path).unwrap();
    let reader = BufReader::new(file.try_clone().unwrap());
    let mut buffer = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        if i >= start && i < start + length {
            buffer.push(line.unwrap());
        }
    }

    return buffer;
}
