use entity::enums::{
    PollActionPermissionChangeType, PollActionRoleMemberChangeType,
    PollActionType,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreatePollActionRequest {
    pub(crate) action_type: PollActionType,
    pub(crate) server_role: Option<CreatePollActionServerRoleRequest>,
    pub(crate) server_config:
        Option<crate::servers::types::ServerConfigRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreatePollActionServerRoleRequest {
    pub(crate) name: Option<String>,
    pub(crate) color: Option<String>,
    pub(crate) members: Option<Vec<PollActionServerRoleMemberRequest>>,
    pub(crate) permissions: Option<Vec<CreatePollActionPermissionRequest>>,
    pub(crate) server_role_to_update_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PollActionServerRoleMemberRequest {
    pub(crate) user_id: String,
    pub(crate) change_type: PollActionRoleMemberChangeType,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreatePollActionPermissionRequest {
    pub(crate) subject: String,
    pub(crate) actions: Vec<PollActionPermissionChangeRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PollActionPermissionChangeRequest {
    pub(crate) action: String,
    pub(crate) change_type: PollActionPermissionChangeType,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PollActionResponse {
    pub(crate) id: String,
    pub(crate) action_type: PollActionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) server_role: Option<PollActionServerRoleResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) server_config: Option<PollActionServerConfigResponse>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PollActionServerConfigResponse {
    pub(crate) anonymous_users_enabled: Option<bool>,
    pub(crate) prev_anonymous_users_enabled: Option<bool>,
    pub(crate) decision_making_model: Option<String>,
    pub(crate) prev_decision_making_model: Option<String>,
    pub(crate) disagreements_limit: Option<i32>,
    pub(crate) prev_disagreements_limit: Option<i32>,
    pub(crate) abstains_limit: Option<i32>,
    pub(crate) prev_abstains_limit: Option<i32>,
    pub(crate) agreement_threshold: Option<i32>,
    pub(crate) prev_agreement_threshold: Option<i32>,
    pub(crate) quorum_enabled: Option<bool>,
    pub(crate) prev_quorum_enabled: Option<bool>,
    pub(crate) quorum_threshold: Option<i32>,
    pub(crate) prev_quorum_threshold: Option<i32>,
    pub(crate) voting_time_limit: Option<i32>,
    pub(crate) prev_voting_time_limit: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PollActionServerRoleResponse {
    pub(crate) id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) prev_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) prev_color: Option<String>,
    pub(crate) server_role_id: String,
    pub(crate) members: Vec<PollActionServerRoleMemberResponse>,
    pub(crate) permissions: Vec<PollActionPermissionResponse>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PollActionServerRoleMemberResponse {
    pub(crate) change_type: PollActionRoleMemberChangeType,
    pub(crate) user: PollActionUserResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PollActionUserResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) display_name: Option<String>,
    pub(crate) profile_picture: Option<crate::users::UserImageRef>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PollActionPermissionResponse {
    pub(crate) subject: String,
    pub(crate) action: String,
    pub(crate) change_type: PollActionPermissionChangeType,
}
