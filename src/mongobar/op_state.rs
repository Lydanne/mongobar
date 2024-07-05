use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub(crate) struct OpState {
    pub stress_index: i64,
    pub stress_start_ts: i64,
    pub stress_end_ts: i64,

    pub record_start_ts: i64,
    pub record_end_ts: i64,
}
