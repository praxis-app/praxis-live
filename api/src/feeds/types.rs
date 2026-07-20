use serde::{Deserialize, Serialize};

use crate::{calls, forum, messages, polls};

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
    Message(FeedMessageResponse),
    Poll(FeedPollResponse),
    ProposalForumReference(FeedProposalForumReferenceResponse),
    Call(calls::types::CallArtifactResponse),
}

#[derive(Debug, Serialize)]
pub(super) struct FeedProposalForumReferenceResponse {
    #[serde(rename = "type")]
    kind: &'static str,
    #[serde(flatten)]
    pub(super) reference: forum::types::ProposalForumReferenceResponse,
}

impl FeedProposalForumReferenceResponse {
    pub(super) fn new(
        reference: forum::types::ProposalForumReferenceResponse,
    ) -> Self {
        Self {
            kind: "proposalMoved",
            reference,
        }
    }
}

#[derive(Debug, Serialize)]
pub(super) struct FeedMessageResponse {
    #[serde(rename = "type")]
    kind: &'static str,
    #[serde(flatten)]
    pub(super) message: messages::types::MessageResponse,
}

impl FeedMessageResponse {
    pub(super) fn new(message: messages::types::MessageResponse) -> Self {
        Self {
            kind: "message",
            message,
        }
    }
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
