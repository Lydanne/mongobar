use mongodb::bson::Document;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub(crate) struct OpRow {
    pub id: String,
    pub op: Op,
    pub db: String,
    pub coll: String,
    pub cmd: Document,
    pub ns: String,
    pub ts: i64,
    pub st: Status,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub(crate) enum Op {
    #[default]
    None,
    Insert,
    Update,
    Delete,
    Query,
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
