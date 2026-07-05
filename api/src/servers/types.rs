use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};
use serde::{Deserialize, Serialize};

use crate::users::UserImageRef;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerPath {
    pub(crate) server_id: Uuid,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerRequest {
    pub(crate) name: String,
    pub(crate) slug: String,
    pub(crate) description: Option<String>,
    pub(crate) is_default_server: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerMembersRequest {
    pub(crate) user_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JoinServerRequest {
    pub(crate) invite_token: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) slug: String,
    pub(crate) description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) is_default_server: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) general_channel_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) member_count: Option<u64>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) display_name: Option<String>,
    pub(crate) profile_picture: Option<UserImageRef>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerConfigRequest {
    pub(crate) anonymous_users_enabled: Option<bool>,
    pub(crate) decision_making_model: Option<String>,
    pub(crate) disagreements_limit: Option<i32>,
    pub(crate) abstains_limit: Option<i32>,
    pub(crate) agreement_threshold: Option<i32>,
    pub(crate) quorum_enabled: Option<bool>,
    pub(crate) quorum_threshold: Option<i32>,
    pub(crate) voting_time_limit: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerConfigResponse {
    pub(crate) anonymous_users_enabled: bool,
    pub(crate) decision_making_model: String,
    pub(crate) disagreements_limit: i32,
    pub(crate) abstains_limit: i32,
    pub(crate) agreement_threshold: i32,
    pub(crate) quorum_enabled: bool,
    pub(crate) quorum_threshold: i32,
    pub(crate) voting_time_limit: i32,
    pub(crate) updated_at: String,
}

pub(crate) fn serialize_timestamp(value: DateTimeWithTimeZone) -> String {
    value.to_rfc3339()
}
