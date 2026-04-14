use serde::{Deserialize, Serialize};

use crate::{common::auth::PermissionRule, servers::types::UserResponse};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoleRequest {
    pub(crate) name: String,
    pub(crate) color: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoleMembersRequest {
    pub(crate) user_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdatePermissionsRequest {
    pub(crate) permissions: Vec<PermissionRule>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerRoleResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) color: String,
    pub(crate) permissions: Vec<PermissionRule>,
    pub(crate) member_count: usize,
    pub(crate) members: Vec<UserResponse>,
}
