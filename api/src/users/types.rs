use sea_orm::{prelude::Uuid, DbErr};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::models::UserRecord;
use crate::{common::roles::PermissionRule, servers::types::ServerResponse};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PublicUser {
    id: Uuid,
    email: Option<String>,
    name: String,
    display_name: Option<String>,
    anonymous: bool,
}

impl From<UserRecord> for PublicUser {
    fn from(user: UserRecord) -> Self {
        Self {
            id: user.id,
            email: user.email,
            name: user.name,
            display_name: user.display_name,
            anonymous: user.anonymous,
        }
    }
}

pub(crate) enum CreateUserError {
    DuplicateEmail,
    Database(DbErr),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserImageRef {
    pub(crate) id: String,
    pub(crate) created_at: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct UserImagePayload {
    pub(crate) image: UserImageRef,
}

#[derive(Debug, Clone)]
pub(crate) struct StoredUserImage {
    pub(crate) content_type: Option<String>,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserImagePath {
    pub(crate) user_id: Uuid,
    pub(crate) image_id: Uuid,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CurrentUserResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) display_name: Option<String>,
    pub(crate) anonymous: bool,
    pub(crate) permissions: CurrentUserPermissions,
    pub(crate) profile_picture: Option<UserImageRef>,
    pub(crate) current_server: Option<ServerResponse>,
    pub(crate) servers_count: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct CurrentUserPayload {
    pub(crate) user: CurrentUserResponse,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CurrentUserPermissions {
    pub(crate) instance: Vec<PermissionRule>,
    pub(crate) servers: BTreeMap<String, Vec<PermissionRule>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserProfileResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) display_name: Option<String>,
    pub(crate) bio: Option<String>,
    pub(crate) profile_picture: Option<UserImageRef>,
    pub(crate) cover_photo: Option<UserImageRef>,
}

#[derive(Debug, Serialize)]
pub(crate) struct UserProfilePayload {
    pub(crate) user: UserProfileResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FirstUserResponse {
    pub(crate) is_first_user: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateUserProfileRequest {
    pub(crate) name: Option<String>,
    pub(crate) display_name: Option<String>,
    pub(crate) bio: Option<String>,
}
