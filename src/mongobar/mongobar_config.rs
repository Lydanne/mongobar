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

    pub force_build_resume: Option<bool>,
}
