use entity::users;
use sea_orm::prelude::Uuid;
use sea_orm::DbErr;
use serde::Serialize;
use std::collections::BTreeMap;

use crate::common::authorization::PermissionRule;

#[derive(Debug, Clone)]
pub(crate) struct UserRecord {
    pub(crate) id: Uuid,
    pub(crate) email: String,
    pub(crate) name: String,
    pub(crate) password_hash: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PublicUser {
    id: Uuid,
    email: String,
    name: String,
}

impl From<UserRecord> for PublicUser {
    fn from(user: UserRecord) -> Self {
        Self {
            id: user.id,
            email: user.email,
            name: user.name,
        }
    }
}

impl From<users::Model> for UserRecord {
    fn from(user: users::Model) -> Self {
        Self {
            id: user.id,
            email: user.email,
            name: user.name,
            password_hash: user.password_hash,
        }
    }
}

pub(crate) enum CreateUserError {
    DuplicateEmail,
    Database(DbErr),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CurrentUserResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) anonymous: bool,
    pub(crate) permissions: CurrentUserPermissions,
    pub(crate) profile_picture: Option<serde_json::Value>,
    pub(crate) current_server: serde_json::Value,
    pub(crate) servers_count: usize,
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
    pub(crate) profile_picture: Option<serde_json::Value>,
    pub(crate) cover_photo: Option<serde_json::Value>,
}
