use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct MessageImagePath {
    pub(super) server_id: Uuid,
    pub(super) channel_id: Uuid,
    pub(super) message_id: Uuid,
    pub(super) image_id: Uuid,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CallMessageImagePath {
    pub(super) server_id: Uuid,
    pub(super) channel_id: Uuid,
    pub(super) call_id: Uuid,
    pub(super) message_id: Uuid,
    pub(super) image_id: Uuid,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CreateMessageRequest {
    pub(crate) body: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ImageResponse {
    pub(super) id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) is_placeholder: Option<bool>,
    pub(super) created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MessageUser {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) display_name: Option<String>,
    pub(crate) profile_picture: Option<crate::users::UserImageRef>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MessageResponse {
    pub(crate) id: String,
    pub(crate) body: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) images: Vec<ImageResponse>,
    pub(super) user: Option<MessageUser>,
    pub(super) user_id: Option<String>,
    pub(super) bot_id: Option<String>,
    pub(super) bot: Option<serde_json::Value>,
    pub(super) command_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) thread_root_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) parent_message_id: Option<String>,
    pub(crate) created_at: String,
}

#[derive(Debug, Serialize)]
pub(super) struct MessagePayload {
    pub(super) message: MessageResponse,
}

#[derive(Debug, Clone)]
pub(super) struct StoredImage {
    pub(super) bytes: Vec<u8>,
}

pub(crate) fn serialize_timestamp(value: DateTimeWithTimeZone) -> String {
    value.to_rfc3339()
}
