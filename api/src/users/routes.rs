use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::Json,
    routing::get,
    Router,
};
use entity::users;
use jsonwebtoken::{decode, DecodingKey, Validation};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};
use serde::Deserialize;
use std::sync::Arc;

use super::find_user_by_id;
use crate::{
    messages::types::{ApiError, AppResult},
    servers,
};

#[derive(Clone, Debug)]
struct UsersState {
    database: DatabaseConnection,
    jwt_secret: Arc<str>,
}

#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
}

pub(crate) fn router(database: DatabaseConnection, jwt_secret: String) -> Router {
    let state = UsersState {
        database,
        jwt_secret: Arc::<str>::from(jwt_secret),
    };

    Router::new()
        .route("/users/me", get(get_current_user))
        .route("/users/me/servers", get(get_current_user_servers))
        .route("/users/is-first", get(is_first_user))
        .route("/users/{userId}/profile", get(get_user_profile))
        .with_state(state)
}

async fn get_current_user(
    State(state): State<UsersState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = require_user_id(&state, &headers)?;
    let user = find_user_by_id(&state.database, user_id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "Authentication required."))?;
    let servers = servers::service::get_servers_for_user(&state.database, user_id).await?;
    let current_server = servers::service::get_current_server(&state.database, user_id).await?;

    Ok(Json(serde_json::json!({
        "user": {
            "id": user.id.to_string(),
            "name": user.name,
            "anonymous": false,
            "permissions": {
                "instance": ["read:Server", "create:Server", "update:Server", "delete:Server"],
                "servers": {}
            },
            "profilePicture": null,
            "currentServer": current_server,
            "serversCount": servers.len()
        }
    })))
}

async fn get_current_user_servers(
    State(state): State<UsersState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = require_user_id(&state, &headers)?;
    let servers = servers::service::get_servers_for_user(&state.database, user_id).await?;
    Ok(Json(serde_json::json!({ "servers": servers })))
}

async fn is_first_user(State(state): State<UsersState>) -> AppResult<Json<serde_json::Value>> {
    let count = users::Entity::find()
        .count(&state.database)
        .await
        .map_err(internal_error)?;
    Ok(Json(serde_json::json!({ "isFirstUser": count == 0 })))
}

async fn get_user_profile(
    State(state): State<UsersState>,
    Path(user_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = user_id
        .parse::<i64>()
        .map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, "userId must be numeric."))?;
    let user = users::Entity::find()
        .filter(users::Column::Id.eq(user_id))
        .one(&state.database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "User not found."))?;

    Ok(Json(serde_json::json!({
        "user": {
            "id": user.id.to_string(),
            "name": user.name,
            "profilePicture": null,
            "coverPhoto": null
        }
    })))
}

fn require_user_id(state: &UsersState, headers: &HeaderMap) -> AppResult<i64> {
    let token = bearer_token(headers)
        .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "Authentication required."))?;

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .ok()
    .and_then(|claims| claims.claims.sub.parse::<i64>().ok())
    .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "Authentication required."))
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let header_value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = header_value.split_once(' ')?;

    if scheme.eq_ignore_ascii_case("Bearer") && !token.is_empty() {
        Some(token)
    } else {
        None
    }
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("users request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
