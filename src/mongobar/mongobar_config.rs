use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub(crate) struct MongobarConfig {
    pub uri: String,
    pub db: String,
}
