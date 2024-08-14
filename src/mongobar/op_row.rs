use mongodb::bson::Document;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::utils::to_sha3_8;

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub(crate) struct OpRow {
    pub id: String,
    pub op: Op,
    pub db: String,
    pub coll: String,
    pub cmd: Value,
    pub ns: String,
    pub ts: i64,

    #[serde(skip)]
    pub args: Document,

    #[serde(skip)]
    pub key: String,

    #[serde(skip)]
    pub hash: String,
}

impl OpRow {
    pub fn build_key(&self) -> String {
        let keys = match self.op {
            Op::Find => {
                let mut keys = deep_build_key(&self.cmd.get("filter").unwrap_or(&Value::Null));
                keys.sort();
                keys
            }
            Op::Update => {
                if let Some(updates) = self.cmd.get("updates") {
                    if updates.as_array().is_none() {
                        return "None".to_string();
                    }
                    let mut keys = vec![];
                    let mut ukeys = vec![];
                    for update in updates.as_array().unwrap() {
                        keys.append(&mut deep_build_key(&update.get("q").unwrap()));
                        ukeys.append(&mut deep_build_key(&update.get("u").unwrap()));
                    }
                    keys.sort();
                    ukeys.sort();
                    ukeys.dedup();
                    keys.push(">>".to_string());
                    keys.push(ukeys.join(":"));
                    keys
                } else if let Some(filter) = self.cmd.get("q") {
                    let mut keys = deep_build_key(filter);
                    keys.sort();
                    keys
                } else {
                    let mut keys = deep_build_key(&self.cmd);
                    keys.sort();
                    keys
                }
            }
            _ => {
                let mut keys = deep_build_key(&self.cmd);
                keys.sort();
                keys
            }
        };
        format!("{}:{:?}:{}", self.coll, self.op, to_sha3_8(&keys.join("")))
    }
}

/// 将递归所有 object（包括子 object） 的 key 取出
fn deep_build_key(v: &Value) -> Vec<String> {
    match v {
        Value::Object(o) => {
            let mut keys = vec![];
            for (k, v) in o.iter() {
                keys.push(replace_number(k));
                keys.append(&mut deep_build_key(v));
            }
            keys
        }
        Value::Array(a) => {
            let mut keys = vec![];
            for v in a.iter() {
                keys.append(&mut deep_build_key(v));
            }
            keys
        }
        _ => vec![],
    }
}

static REG_NUMBER: once_cell::sync::Lazy<regex::Regex> =
    once_cell::sync::Lazy::new(|| regex::Regex::new(r"\d+").unwrap());

fn replace_number(s: &str) -> String {
    REG_NUMBER.replace_all(s, "[n]").to_string()
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub(crate) enum Op {
    #[default]
    None,
    Insert,
    Update,
    Delete,
    Find,
    Count,
    Command,
    Aggregate,
    FindAndModify,
    GetMore,
}

impl From<String> for Op {
    fn from(s: String) -> Self {
        match s.as_str() {
            "update" => Op::Update,
            "delete" => Op::Delete,
            "find" => Op::Find,
            "count" => Op::Count,
            "aggregate" => Op::Aggregate,
            "findAndModify" => Op::FindAndModify,
            "getMore" => Op::GetMore,
            "insert" => Op::Insert,
            _ => Op::None,
        }
    }
}
