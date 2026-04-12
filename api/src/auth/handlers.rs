use axum::{extract::State, http::HeaderMap, response::Json};
use sea_orm::{DatabaseConnection, TransactionTrait};
use std::sync::Arc;

use super::{
    service::{
        bearer_token, internal_error, issue_access_token, map_create_user_error, validate_login,
        validate_signup, verify_access_token,
    },
    types::{ApiError, AppResult, LoginRequest, SessionResponse, SignupRequest},
};
use crate::users::{self, UserRecord};
use crate::{channels, servers};

#[derive(Clone, Debug)]
pub(super) struct AuthState {
    pub(super) database: DatabaseConnection,
    pub(super) jwt_secret: Arc<str>,
}

impl AuthState {
    pub(super) fn new(database: DatabaseConnection, jwt_secret: String) -> Self {
        Self {
            database,
            jwt_secret: Arc::<str>::from(jwt_secret),
        }
    }
}

pub(super) async fn me(
    State(auth_state): State<AuthState>,
    headers: HeaderMap,
) -> AppResult<Json<SessionResponse>> {
    let user = current_user(&auth_state, &headers).await?;

    Ok(Json(SessionResponse {
        user: user.map(Into::into),
        access_token: None,
    }))
}

pub(super) async fn signup(
    State(auth_state): State<AuthState>,
    Json(payload): Json<SignupRequest>,
) -> AppResult<(axum::http::StatusCode, Json<SessionResponse>)> {
    let signup = validate_signup(payload)?;
    let password_hash = password_auth::generate_hash(signup.password);
    let user = users::create_user(
        &auth_state.database,
        signup.email,
        signup.name,
        password_hash,
    )
    .await
    .map_err(map_create_user_error)?;

    let default_server_id = servers::default_server_id(&auth_state.database)
        .await
        .map_err(internal_error)?;
    let transaction = auth_state.database.begin().await.map_err(internal_error)?;

    servers::add_member_to_server(&transaction, default_server_id, user.id)
        .await
        .map_err(internal_error)?;

    channels::add_member_to_all_server_channels(&transaction, default_server_id, user.id)
        .await
        .map_err(internal_error)?;

    transaction.commit().await.map_err(internal_error)?;

    let access_token = issue_access_token(&auth_state, user.id)?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(SessionResponse {
            user: Some(user.into()),
            access_token: Some(access_token),
        }),
    ))
}

pub(super) async fn login(
    State(auth_state): State<AuthState>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<SessionResponse>> {
    let login = validate_login(payload)?;
    let user = users::authenticate(&auth_state.database, login.email, login.password)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(
                axum::http::StatusCode::UNAUTHORIZED,
                "Invalid email or password.",
            )
        })?;

    let access_token = issue_access_token(&auth_state, user.id)?;

    Ok(Json(SessionResponse {
        user: Some(user.into()),
        access_token: Some(access_token),
    }))
}

pub(super) async fn logout() -> Json<SessionResponse> {
    Json(SessionResponse {
        user: None,
        access_token: None,
    })
}

async fn current_user(
    auth_state: &AuthState,
    headers: &HeaderMap,
) -> AppResult<Option<UserRecord>> {
    let Some(token) = bearer_token(headers) else {
        return Ok(None);
    };

    let Some(user_id) = verify_access_token(auth_state, token) else {
        return Ok(None);
    };

    users::get_user_by_id(&auth_state.database, user_id)
        .await
        .map_err(internal_error)
}
