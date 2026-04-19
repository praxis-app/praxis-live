use sea_orm::prelude::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};

use crate::poll_actions::types::{CreatePollActionRequest, PollActionResponse};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreatePollRequest {
    pub(crate) body: Option<String>,
    pub(crate) poll_type: String,
    pub(crate) action: Option<CreatePollActionRequest>,
    pub(crate) options: Option<Vec<String>>,
    pub(crate) multiple_choice: Option<bool>,
    pub(crate) closing_at: Option<DateTimeWithTimeZone>,
    #[serde(default)]
    pub(crate) image_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VoteRequest {
    pub(crate) vote_type: Option<String>,
    pub(crate) poll_option_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PollResponse {
    pub(crate) id: String,
    pub(crate) body: Option<String>,
    pub(crate) poll_type: String,
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
    pub(crate) created_at: String,
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PollUserResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) display_name: Option<String>,
    pub(crate) profile_picture: Option<crate::users::UserImageRef>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VoteResponse {
    pub(crate) id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) vote_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) poll_option_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateVoteResponse {
    pub(crate) id: String,
    pub(crate) poll_id: String,
    pub(crate) user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) vote_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) poll_option_ids: Option<Vec<String>>,
    pub(crate) is_ratifying_vote: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateVoteResponse {
    pub(crate) is_ratifying_vote: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PollOptionVoterResponse {
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
