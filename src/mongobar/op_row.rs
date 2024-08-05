use mongodb::bson::Document;

use serde::{Deserialize, Serialize};
use serde_json::Value;

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
}

impl OpRow {
    pub fn build_key(&self) -> String {
        let mut keys = match self.op {
            Op::Find => deep_build_key(&self.cmd.get("filter").unwrap_or(&Value::Null)),
            Op::Update => {
                if let Some(updates) = self.cmd.get("updates") {
                    let mut keys = vec![];
                    for update in updates.as_array().unwrap() {
                        keys.append(&mut deep_build_key(&update.get("q").unwrap()));
                    }
                    keys
                } else if let Some(filter) = self.cmd.get("q") {
                    deep_build_key(filter)
                } else {
                    deep_build_key(&self.cmd)
                }
            }
            _ => deep_build_key(&self.cmd),
        };
        keys.sort();
        format!("{}:{:?}:{}", self.coll, self.op, keys.join(":"))
    }
}

/// 将递归所有 object（包括子 object） 的 key 取出
fn deep_build_key(v: &Value) -> Vec<String> {
    match v {
        Value::Object(o) => {
            let mut keys = vec![];
            for (k, v) in o.iter() {
                keys.push(k.clone());
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
