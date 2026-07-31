use entity::enums::{ChannelType, PollType};
use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};
use serde::{Deserialize, Serialize};

use crate::poll_actions::types::{CreatePollActionRequest, PollActionResponse};
use crate::votes::types::VoteResponse;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PollPath {
    pub(crate) server_id: Uuid,
    pub(crate) channel_id: Uuid,
    pub(crate) poll_id: Uuid,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PollImagePath {
    pub(crate) server_id: Uuid,
    pub(crate) channel_id: Uuid,
    pub(crate) poll_id: Uuid,
    pub(crate) image_id: Uuid,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreatePollRequest {
    pub(crate) body: Option<String>,
    #[serde(default = "default_poll_type")]
    pub(crate) poll_type: PollType,
    pub(crate) action: Option<CreatePollActionRequest>,
    pub(crate) options: Option<Vec<String>>,
    pub(crate) multiple_choice: Option<bool>,
    pub(crate) closing_at: Option<DateTimeWithTimeZone>,
    #[serde(default)]
    pub(crate) image_count: usize,
}

fn default_poll_type() -> PollType {
    PollType::Proposal
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListActiveDecisionsQuery {
    pub(crate) before: Option<String>,
    pub(crate) limit: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PollResponse {
    pub(crate) id: String,
    pub(crate) body: Option<String>,
    pub(crate) poll_type: PollType,
    pub(crate) stage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) action: Option<PollActionResponse>,
    pub(crate) config: PollConfigResponse,
    pub(crate) options: Vec<PollOptionResponse>,
    pub(crate) images: Vec<PollImageResponse>,
    pub(crate) user: PollUserResponse,
    pub(crate) votes: Vec<VoteResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) my_vote: Option<VoteResponse>,
    pub(crate) agreement_vote_count: usize,
    pub(crate) member_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) source_call_id: Option<String>,
    pub(crate) created_at: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct PollPayload {
    pub(crate) poll: PollResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CallDecisionResponse {
    pub(crate) active_item: Option<PollResponse>,
    pub(crate) recent_result: Option<PollResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ActiveDecisionResponse {
    pub(crate) id: String,
    pub(crate) poll_type: PollType,
    pub(crate) body: Option<String>,
    pub(crate) closing_at: Option<String>,
    pub(crate) response_count: usize,
    pub(crate) member_count: usize,
    pub(crate) has_responded: bool,
    pub(crate) created_at: String,
    pub(crate) channel_id: String,
    pub(crate) channel_name: String,
    pub(crate) channel_type: ChannelType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) forum_post_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ActiveDecisionsResponse {
    pub(crate) decisions: Vec<ActiveDecisionResponse>,
    pub(crate) next_cursor: Option<String>,
    pub(crate) has_more: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PollConfigResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) decision_making_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) agreement_threshold: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) quorum_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) quorum_threshold: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) disagreements_limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) abstains_limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) closing_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) multiple_choice: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PollOptionResponse {
    pub(crate) id: String,
    pub(crate) text: String,
    pub(crate) vote_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PollImageResponse {
    pub(crate) id: String,
    pub(crate) is_placeholder: bool,
    pub(crate) created_at: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct PollImagePayload {
    pub(crate) image: PollImageResponse,
}

#[derive(Debug, Serialize)]
pub(crate) struct DeletePollResponse {
    pub(crate) affected: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PollUserResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) display_name: Option<String>,
    pub(crate) profile_picture: Option<crate::users::UserImageRef>,
}

#[derive(Debug, Clone)]
pub(crate) struct StoredPollImage {
    pub(crate) content_type: Option<String>,
    pub(crate) bytes: Vec<u8>,
}
