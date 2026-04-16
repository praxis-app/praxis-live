use sea_orm::prelude::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InviteRequest {
    pub(crate) max_uses: Option<i32>,
    pub(crate) expires_at: Option<DateTimeWithTimeZone>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InviteResponse {
    pub(crate) id: String,
    pub(crate) token: String,
    pub(crate) uses: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) max_uses: Option<i32>,
    pub(crate) user: InviteUserResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) expires_at: Option<String>,
    pub(crate) created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InviteUserResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) display_name: Option<String>,
    pub(crate) profile_picture: Option<serde_json::Value>,
}
