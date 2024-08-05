use std::io::{BufRead, Write};

pub fn reg_filter_line(target: &str, outfile: &str, filter: &str) -> usize {
    let line_number = 0;
    let re = regex::Regex::new(filter).unwrap();
    let file = std::fs::File::open(target).unwrap();
    let reader = std::io::BufReader::new(file);
    let mut writer = std::fs::File::create(outfile).unwrap();
    for line in reader.lines() {
        let line = line.unwrap();
        if re.is_match(&line) {
            writer.write_all(line.as_bytes()).unwrap();
            writer.write_all(b"\n").unwrap();
        }
    }

    line_number
}
