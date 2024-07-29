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
