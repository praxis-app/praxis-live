use axum::http::StatusCode;
use entity::users;
use sea_orm::{
    prelude::Uuid, ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    PaginatorTrait, QueryFilter, Set, SqlErr,
};
use uuid::Uuid as NativeUuid;

use super::models::{
    CreateUserError, CurrentUserPermissions, CurrentUserResponse, UserProfileResponse, UserRecord,
};
use crate::{
    common::{ApiError, AppResult},
    servers,
};

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

pub(crate) async fn get_current_user(
    database: &DatabaseConnection,
    user_id: Uuid,
) -> AppResult<CurrentUserResponse> {
    let user = get_user_by_id(database, user_id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "Authentication required."))?;
    let servers = servers::service::get_servers_for_user(database, user_id).await?;
    let current_server = servers::service::get_current_server(database, user_id).await?;

    Ok(CurrentUserResponse {
        id: user.id.to_string(),
        name: user.name,
        anonymous: false,

        // TODO: Implement permissions
        permissions: CurrentUserPermissions {
            instance: [
                "read:Server",
                "create:Server",
                "update:Server",
                "delete:Server",
            ],
            servers: serde_json::json!({}),
        },
        profile_picture: None,
        current_server: serde_json::json!(current_server),
        servers_count: servers.len(),
    })
}

pub(crate) async fn is_first_user(database: &DatabaseConnection) -> AppResult<bool> {
    users::Entity::find()
        .count(database)
        .await
        .map(|count| count == 0)
        .map_err(internal_error)
}

pub(crate) async fn get_user_profile(
    database: &DatabaseConnection,
    user_id: Uuid,
) -> AppResult<UserProfileResponse> {
    let user = users::Entity::find_by_id(user_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "User not found."))?;

    Ok(UserProfileResponse {
        id: user.id.to_string(),
        name: user.name,
        profile_picture: None,
        cover_photo: None,
    })
}

fn map_create_user_error(error: DbErr) -> CreateUserError {
    if matches!(error.sql_err(), Some(SqlErr::UniqueConstraintViolation(_))) {
        return CreateUserError::DuplicateEmail;
    }

    CreateUserError::Database(error)
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("users request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
