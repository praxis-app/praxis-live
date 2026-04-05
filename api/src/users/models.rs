use entity::users;
use sea_orm::DbErr;
use serde::Serialize;

#[derive(Debug, Clone)]
pub(crate) struct UserRecord {
    pub(crate) id: i64,
    pub(crate) email: String,
    pub(crate) name: String,
    pub(crate) password_hash: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PublicUser {
    id: i64,
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
