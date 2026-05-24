use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub(super) struct FeedQuery {
    pub(super) offset: Option<u64>,
    pub(super) limit: Option<u64>,
}

#[derive(Debug, Serialize)]
pub(super) struct FeedResponse {
    pub(super) feed: Vec<serde_json::Value>,
}
