use std::fs::File;
use std::io::{BufRead, BufReader};

pub fn count_lines(file_path: &str) -> usize {
    let file = File::open(file_path).unwrap();
    let reader = BufReader::new(file);
    reader.lines().count()
}

use chrono::{DateTime, TimeZone, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use sha3::digest::{ExtendableOutput, Update, XofReader};
use sha3::Shake128;

static DATE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}.\d{3}[+-]\d{4})").unwrap());

static DATE_ITEM_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2}).(\d{3})([+-]\d{4})").unwrap()
});

pub fn format_date(date: &str) -> String {
    if let Some(captures) = DATE_ITEM_RE.captures(date) {
        let year = captures.get(1).unwrap().as_str().parse::<i32>().unwrap();
        let month = captures.get(2).unwrap().as_str().parse::<u32>().unwrap();
        let day = captures.get(3).unwrap().as_str().parse::<u32>().unwrap();
        let hour = captures.get(4).unwrap().as_str().parse::<u32>().unwrap();
        let minute = captures.get(5).unwrap().as_str().parse::<u32>().unwrap();
        let second = captures.get(6).unwrap().as_str().parse::<u32>().unwrap();

        let date_time = Utc.with_ymd_and_hms(year, month, day, hour, minute, second);
        let output_date = date_time.map(|dt| {
            dt.with_timezone(&Utc)
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
        });
        output_date.unwrap()
    } else {
        date.to_string()
    }
}

pub fn match_date_replace(input_date: &str) -> String {
    let res = DATE_RE.replace_all(input_date, |caps: &regex::Captures| {
        let date_str = caps.get(1).unwrap().as_str();
        format_date(date_str)
    });
    res.to_string()
}

pub fn to_sha3(s: &str) -> String {
    let mut hasher = Shake128::default();

    hasher.update(s.as_bytes());

    let mut output = [0u8; 16];
    hasher.finalize_xof().read(&mut output);

    // 输出结果
    hex::encode(output)
}

pub fn get_db_coll(ns: &str) -> (String, String) {
    let parts: Vec<&str> = ns.split('.').collect();
    if parts.len() == 2 {
        (parts[0].to_string(), parts[1].to_string())
    } else {
        ("".to_string(), "".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_date_replace() {
        let input_date = "{\"id\":\"7b17adef8237aff0ffe256e5733a1ff2\",\"op\":\"Find\",\"db\":\"xgj\",\"coll\":\"notifies\",\"cmd\":{\"find\":\"notifies\",\"filter\":{\"create_at\":{\"$gt\":{\"$date\":\"2022-09-01T08:00:00.000+0800\",\"$gt3\":{\"$date\":\"2022-09-01T08:00:00.000+0800\"}},\"cls\":{\"$in\":[{\"$oid\":\"605db9438b75f3247ce73ef7\"},{\"$oid\":\"637738c552ff71997131a5b1\"}]},\"is_del\":false,\"setting.delay_publish\":false,\"$and\":[{\"$or\":[{\"level\":{\"$exists\":false}},{\"level\":0}]},{\"$or\":[{\"sp_members\":{\"$in\":[\"605db9c022fb10108149a7d7\",\"637c6c4f1032c13bd918d27f\"]}},{\"sp_members\":{\"$eq\":[]}}]},{\"$or\":[{\"ex_members\":{\"$nin\":[\"605db9c022fb10108149a7d7\",\"637c6c4f1032c13bd918d27f\"]}}]}]},\"sort\":{\"sort_time\":-1,\"create_at\":-1},\"projection\":{\"type\":1,\"title\":1,\"text_content\":1,\"cls\":1,\"class_name\":1,\"create_at\":1,\"sort_time\":1,\"daka_end_day\":1,\"daka_start_day\":1,\"subject\":1,\"end_day\":1,\"daka_day\":1,\"daka_rest\":1,\"creator_wx_name\":1,\"daka_latest\":1,\"need_feedback\":1,\"feedback_type\":1,\"status\":1,\"feedbacks\":1,\"tiku\":1,\"atype\":1,\"cate\":1,\"relevant\":1,\"datika\":1,\"fellow_read\":1,\"fellow_write\":1,\"groupBy\":1,\"from_album_cate\":1,\"files\":1,\"stats_list\":1,\"cycleDay\":1,\"accepts\":1,\"invest\":1,\"creator_wx_avatar\":1,\"need_check\":1,\"setting\":1,\"score\":1,\"sa\":1,\"fee\":1,\"attach\":1},\"limit\":10,\"lsid\":{\"id\":{\"$binary\":\"fpiHYejVTSSn+CZS74LL0A==\",\"$type\":\"04\"}},\"$clusterTime\":{\"clusterTime\":{\"$timestamp\":{\"t\":1721355565,\"i\":1010}},\"signature\":{\"hash\":{\"$binary\":\"XJKn9IOd9YU64EkOucajn2wVWBc=\",\"$type\":\"00\"},\"keyId\":{\"$numberLong\":\"7349532703881955184\"}}},\"$db\":\"xgj\"},\"ns\":\"xgj.notifies\",\"ts\":1721355566}";
        let output_date = match_date_replace(input_date);
        println!("{}", output_date);
        assert!(output_date.contains("2022-09-01T08:00:00.000Z"));
    }
}
