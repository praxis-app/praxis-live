use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};
use serde::{Deserialize, Serialize};

pub(crate) use crate::servers::types::ServerPath;
use crate::users::UserImageRef;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InvitePath {
    pub(crate) server_id: Uuid,
    pub(crate) invite_id: Uuid,
}

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

#[derive(Debug, Serialize)]
pub(crate) struct InvitePayload {
    pub(crate) invite: InviteResponse,
}

#[derive(Debug, Serialize)]
pub(crate) struct InvitesPayload {
    pub(crate) invites: Vec<InviteResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InviteValidityResponse {
    pub(crate) is_valid_invite: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InviteUserResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) display_name: Option<String>,
    pub(crate) profile_picture: Option<UserImageRef>,
}
