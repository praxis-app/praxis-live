use axum::http::StatusCode;
use jsonwebtoken::{encode, EncodingKey, Header};
use sea_orm::{prelude::Uuid, DatabaseConnection, TransactionTrait};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::{
    handlers::AuthState,
    types::{Claims, LoginRequest, SignupRequest},
};
use crate::{
    channels,
    common::{ApiError, AppResult},
    servers,
    users::{self, CreateUserError, UserRecord},
};

const ACCESS_TOKEN_TTL: Duration = Duration::from_secs(60 * 60 * 24 * 90);
const MIN_PASSWORD_LENGTH: usize = 8;

pub(super) fn issue_access_token(auth_state: &AuthState, user_id: Uuid) -> AppResult<String> {
    let claims = Claims {
        sub: user_id.to_string(),
        exp: current_unix_timestamp() + ACCESS_TOKEN_TTL.as_secs(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(auth_state.jwt_secret.as_bytes()),
    )
    .map_err(internal_error)
}

pub(super) async fn signup(
    database: &DatabaseConnection,
    payload: SignupRequest,
) -> AppResult<UserRecord> {
    let signup = validate_signup(payload)?;
    let password_hash = password_auth::generate_hash(signup.password);
    let user = users::create_user(database, signup.email, signup.name, password_hash)
        .await
        .map_err(map_create_user_error)?;

    let default_server_id = servers::default_server_id(database)
        .await
        .map_err(internal_error)?;
    let transaction = database.begin().await.map_err(internal_error)?;

    servers::add_member_to_server(&transaction, default_server_id, user.id)
        .await
        .map_err(internal_error)?;
    channels::add_member_to_all_server_channels(&transaction, default_server_id, user.id)
        .await
        .map_err(internal_error)?;

    transaction.commit().await.map_err(internal_error)?;

    Ok(user)
}

pub(super) fn validate_signup(mut input: SignupRequest) -> AppResult<SignupRequest> {
    input.name = input.name.trim().to_owned();
    input.email = normalize_email(&input.email);

    if input.name.chars().count() < 2 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "Name must be at least 2 characters long.",
        ));
    }

    if !looks_like_email(&input.email) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "Enter a valid email address.",
        ));
    }

    if input.password.chars().count() < MIN_PASSWORD_LENGTH {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            format!("Password must be at least {MIN_PASSWORD_LENGTH} characters long."),
        ));
    }

    Ok(input)
}

pub(super) fn validate_login(mut input: LoginRequest) -> AppResult<LoginRequest> {
    input.email = normalize_email(&input.email);

    if !looks_like_email(&input.email) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "Enter a valid email address.",
        ));
    }

    if input.password.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "Password is required.",
        ));
    }

    Ok(input)
}

pub(super) fn map_create_user_error(error: CreateUserError) -> ApiError {
    match error {
        CreateUserError::DuplicateEmail => ApiError::new(
            StatusCode::CONFLICT,
            "An account with that email already exists.",
        ),
        CreateUserError::Database(error) => internal_error(error),
    }
}

pub(crate) fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}

fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn normalize_email(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn looks_like_email(value: &str) -> bool {
    let mut parts = value.split('@');
    let Some(local_part) = parts.next() else {
        return false;
    };
    let Some(domain) = parts.next() else {
        return false;
    };

    !local_part.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && parts.next().is_none()
}
