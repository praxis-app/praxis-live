use sea_orm::prelude::Uuid;
use serde::{Deserialize, Deserializer, Serialize};

use crate::messages::types::{MessageResponse, MessageUser};

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
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateForumPostRequest {
    pub(crate) title: String,
    pub(crate) body: String,
    pub(crate) poll_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateForumPostRequest {
    pub(crate) title: Option<String>,
    pub(crate) body: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    pub(crate) poll_id: Option<Option<Uuid>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateForumReplyRequest {
    pub(crate) body: String,
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ForumPostResponse {
    #[serde(flatten)]
    pub(crate) post: ForumPostSummaryResponse,
    pub(crate) body: String,
    pub(crate) replies: Vec<MessageResponse>,
}

fn deserialize_optional_field<'de, D, T>(
    deserializer: D,
) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}
