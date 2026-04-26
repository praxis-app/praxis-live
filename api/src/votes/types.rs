use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VotePath {
    pub(crate) server_id: String,
    pub(crate) channel_id: String,
    pub(crate) poll_id: String,
    pub(crate) vote_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PollOptionPath {
    pub(crate) server_id: String,
    pub(crate) channel_id: String,
    pub(crate) poll_id: String,
    pub(crate) poll_option_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VoteRequest {
    pub(crate) vote_type: Option<String>,
    pub(crate) poll_option_ids: Option<Vec<String>>,
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
