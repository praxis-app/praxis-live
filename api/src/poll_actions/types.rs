use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreatePollActionRequest {
    pub(crate) action_type: String,
    pub(crate) server_role: Option<CreatePollActionServerRoleRequest>,
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
    pub(crate) change_type: String,
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
    pub(crate) change_type: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PollActionResponse {
    pub(crate) id: String,
    pub(crate) action_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) server_role: Option<PollActionServerRoleResponse>,
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
    pub(crate) change_type: String,
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
    pub(crate) change_type: String,
}
