use entity::users;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set, SqlErr,
};
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

pub(crate) async fn create_user(
    database: &DatabaseConnection,
    email: String,
    name: String,
    password_hash: String,
) -> Result<UserRecord, CreateUserError> {
    users::ActiveModel {
        email: Set(email),
        name: Set(name),
        password_hash: Set(password_hash),
        ..Default::default()
    }
    .insert(database)
    .await
    .map(Into::into)
    .map_err(map_create_user_error)
}

pub(crate) async fn find_user_by_id(
    database: &DatabaseConnection,
    user_id: i64,
) -> Result<Option<UserRecord>, DbErr> {
    users::Entity::find_by_id(user_id)
        .one(database)
        .await
        .map(|user| user.map(Into::into))
}

pub(crate) async fn authenticate(
    database: &DatabaseConnection,
    email: String,
    password: String,
) -> Result<Option<UserRecord>, DbErr> {
    let user = users::Entity::find()
        .filter(users::Column::Email.eq(email))
        .one(database)
        .await?;

    let Some(user) = user.map(UserRecord::from) else {
        return Ok(None);
    };

    Ok(
        password_auth::verify_password(password, &user.password_hash)
            .ok()
            .map(|()| user),
    )
}

fn map_create_user_error(error: DbErr) -> CreateUserError {
    if matches!(error.sql_err(), Some(SqlErr::UniqueConstraintViolation(_))) {
        return CreateUserError::DuplicateEmail;
    }

    CreateUserError::Database(error)
}
