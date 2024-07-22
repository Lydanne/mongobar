use std::{fs, path::PathBuf};

use educe::Educe;
use serde::{Deserialize, Serialize};

#[derive(Educe)]
#[educe(Default)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct MongobarConfig {
    pub uri: String,
    pub db: String,
    #[educe(Default = 10)]
    pub thread_count: u32,
    #[educe(Default = 1000)]
    pub loop_count: usize,

    pub rebuild: Option<bool>,
}

impl MongobarConfig {
    pub fn new(config_file: PathBuf) -> Self {
        if !config_file.exists() {
            let content = serde_json::to_string(&MongobarConfig::default()).unwrap();
            fs::write(&config_file, content).unwrap();
        }
        let content: String = fs::read_to_string(&config_file).unwrap();
        return serde_json::from_str(&content).unwrap();
    }
}
