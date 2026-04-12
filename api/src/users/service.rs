use entity::users;
use sea_orm::{
    prelude::Uuid, ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, Set, SqlErr,
};
use uuid::Uuid as NativeUuid;

use super::{CreateUserError, UserRecord};

pub(crate) async fn create_user(
    database: &DatabaseConnection,
    email: String,
    name: String,
    password_hash: String,
) -> Result<UserRecord, CreateUserError> {
    users::ActiveModel {
        id: Set(NativeUuid::new_v4()),
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

pub(crate) async fn get_user_by_id(
    database: &DatabaseConnection,
    user_id: Uuid,
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
