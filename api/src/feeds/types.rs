use serde::{Deserialize, Serialize};

use crate::{calls, messages, polls};

#[derive(Debug, Deserialize)]
pub(super) struct FeedQuery {
    pub(super) offset: Option<u64>,
    pub(super) limit: Option<u64>,
}

#[derive(Debug, Serialize)]
pub(super) struct FeedResponse {
    pub(super) feed: Vec<FeedItem>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(super) enum FeedItem {
    Message(messages::types::FeedMessageResponse),
    Poll(FeedPollResponse),
    Call(calls::types::CallArtifactResponse),
}

#[derive(Debug, Serialize)]
pub(super) struct FeedPollResponse {
    #[serde(rename = "type")]
    kind: &'static str,
    #[serde(flatten)]
    pub(super) poll: polls::types::PollResponse,
}

impl FeedPollResponse {
    pub(super) fn new(poll: polls::types::PollResponse) -> Self {
        Self { kind: "poll", poll }
    }
}
