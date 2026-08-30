use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct VotePath {
    pub(super) server_id: Uuid,
    pub(super) channel_id: Uuid,
    pub(super) poll_id: Uuid,
    pub(super) vote_id: Uuid,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PollOptionPath {
    pub(super) server_id: Uuid,
    pub(super) channel_id: Uuid,
    pub(super) poll_id: Uuid,
    pub(super) poll_option_id: Uuid,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct VoteRequest {
    pub(super) vote_type: Option<String>,
    pub(super) poll_option_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VoteResponse {
    pub(super) id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) vote_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) poll_option_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CreateVoteResponse {
    pub(super) id: String,
    pub(super) poll_id: String,
    pub(super) user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) vote_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) poll_option_ids: Option<Vec<String>>,
    pub(super) is_ratifying_vote: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) closed_reason: Option<String>,
}

/// A vote mutation plus the notification rows it persisted, which the handler
/// publishes once the vote transaction has committed.
#[derive(Debug)]
pub(super) struct CreatedVote {
    pub(super) vote: CreateVoteResponse,
    pub(super) notifications: Vec<entity::notifications::Model>,
}

#[derive(Debug)]
pub(super) struct UpdatedVote {
    pub(super) vote: UpdateVoteResponse,
    pub(super) notifications: Vec<entity::notifications::Model>,
}

#[derive(Debug, Serialize)]
pub(super) struct VotePayload {
    pub(super) vote: CreateVoteResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UpdateVoteResponse {
    pub(super) is_ratifying_vote: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) closed_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PollOptionVoterResponse {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) display_name: Option<String>,
    pub(super) profile_picture: Option<crate::users::UserImageRef>,
}

#[derive(Debug, Serialize)]
pub(crate) struct VotersPayload {
    pub(super) voters: Vec<PollOptionVoterResponse>,
}
