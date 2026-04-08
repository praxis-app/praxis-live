use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use sea_orm::prelude::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub(crate) struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    pub(crate) fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                error: self.message,
            }),
        )
            .into_response()
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

pub(crate) type AppResult<T> = Result<T, ApiError>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateMessageRequest {
    pub(crate) body: Option<String>,
    pub(crate) image_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ChannelServer {
    pub(crate) id: String,
    pub(crate) slug: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ChannelResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) server: ChannelServer,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImageResponse {
    pub(crate) id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) is_placeholder: Option<bool>,
    pub(crate) created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MessageUser {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) profile_picture: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MessageResponse {
    pub(crate) id: String,
    pub(crate) body: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) images: Vec<ImageResponse>,
    pub(crate) user: Option<MessageUser>,
    pub(crate) user_id: Option<String>,
    pub(crate) bot_id: Option<String>,
    pub(crate) bot: Option<serde_json::Value>,
    pub(crate) command_status: Option<String>,
    pub(crate) created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FeedMessageResponse {
    #[serde(rename = "type")]
    pub(crate) kind: &'static str,
    #[serde(flatten)]
    pub(crate) message: MessageResponse,
}

#[derive(Debug, Clone)]
pub(crate) struct StoredImage {
    pub(crate) storage_key: Option<String>,
    pub(crate) content_type: Option<String>,
}

pub(crate) fn serialize_timestamp(value: DateTimeWithTimeZone) -> String {
    value.to_rfc3339()
}
