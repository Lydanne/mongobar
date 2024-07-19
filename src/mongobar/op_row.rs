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

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub(crate) enum Status {
    #[default]
    None,
    Pending,
    Success(StatusSuccess),
    Failed,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub(crate) struct StatusSuccess {
    pub rts: i64,
    pub rms: i64,
}
