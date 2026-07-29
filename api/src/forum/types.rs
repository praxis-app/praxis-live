use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};

use crate::messages::types::{MessageResponse, MessageUser};
use crate::polls::types::{CreatePollRequest, PollResponse};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ForumChannelPath {
    pub(crate) server_id: Uuid,
    pub(crate) channel_id: Uuid,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ForumPostPath {
    pub(crate) server_id: Uuid,
    pub(crate) channel_id: Uuid,
    pub(crate) post_id: Uuid,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ForumReplyPath {
    pub(crate) server_id: Uuid,
    pub(crate) channel_id: Uuid,
    pub(crate) post_id: Uuid,
    pub(crate) reply_id: Uuid,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListForumPostsQuery {
    pub(crate) sort: Option<String>,
    pub(crate) status: Option<String>,
    pub(crate) before: Option<String>,
    pub(crate) limit: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CreateForumPostRequest {
    pub(crate) title: String,
    pub(crate) body: String,
    pub(crate) proposal: Option<CreatePollRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MoveProposalToForumRequest {
    pub(crate) destination_channel_id: Uuid,
    pub(crate) title: String,
    pub(crate) body: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct UpdateForumPostRequest {
    pub(crate) title: Option<String>,
    pub(crate) body: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateForumReplyRequest {
    pub(crate) body: String,
    #[serde(default)]
    pub(crate) image_count: usize,
    pub(crate) parent_message_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ForumPostSummaryResponse {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) root_message_id: String,
    pub(crate) poll_id: Option<String>,
    pub(crate) status: String,
    pub(crate) user: MessageUser,
    pub(crate) reply_count: usize,
    pub(crate) latest_activity_at: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ForumPostsResponse {
    pub(crate) posts: Vec<ForumPostSummaryResponse>,
    pub(crate) next_cursor: Option<String>,
    pub(crate) has_more: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ForumPostResponse {
    #[serde(flatten)]
    pub(crate) post: ForumPostSummaryResponse,
    pub(crate) body: String,
    pub(crate) replies: Vec<MessageResponse>,
    pub(crate) proposal: Option<PollResponse>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProposalForumReferenceResponse {
    pub(crate) id: String,
    pub(crate) proposal_id: String,
    pub(crate) source_channel_id: String,
    pub(crate) destination_channel_id: String,
    pub(crate) destination_channel_name: String,
    pub(crate) forum_post_id: String,
    pub(crate) user: MessageUser,
    pub(crate) created_at: String,
    pub(crate) moved_at: String,
}

pub(crate) struct MoveProposalToForumResponse {
    pub(crate) post: ForumPostResponse,
    pub(crate) source_reference: ProposalForumReferenceResponse,
    pub(crate) destination_channel_id: Uuid,
}
