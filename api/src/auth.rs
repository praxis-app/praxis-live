use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::user::{self, CreateUserError, PublicUser, UserRecord};

type AppResult<T> = Result<T, ApiError>;

const ACCESS_TOKEN_TTL: Duration = Duration::from_secs(60 * 60 * 24 * 90);
const MIN_PASSWORD_LENGTH: usize = 8;

#[derive(Clone, Debug)]
struct AuthState {
    database: DatabaseConnection,
    jwt_secret: Arc<str>,
}

#[derive(Debug, Deserialize)]
struct SignupRequest {
    email: String,
    name: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug)]
pub(crate) struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                error: self.message,
            }),
        )
            .into_response()
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionResponse {
    user: Option<PublicUser>,
    #[serde(rename = "access_token")]
    access_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: u64,
}

pub fn router(database: DatabaseConnection, jwt_secret: String) -> Router {
    let auth_state = AuthState {
        database,
        jwt_secret: Arc::<str>::from(jwt_secret),
    };

    Router::new()
        .route("/auth/me", get(me))
        .route("/auth/signup", post(signup))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .with_state(auth_state)
}

async fn me(
    State(auth_state): State<AuthState>,
    headers: HeaderMap,
) -> AppResult<Json<SessionResponse>> {
    let user = current_user(&auth_state, &headers).await?;

    Ok(Json(SessionResponse {
        user: user.map(Into::into),
        access_token: None,
    }))
}

async fn signup(
    State(auth_state): State<AuthState>,
    Json(payload): Json<SignupRequest>,
) -> AppResult<(StatusCode, Json<SessionResponse>)> {
    let signup = validate_signup(payload)?;
    let password_hash = password_auth::generate_hash(signup.password);
    let user = user::create_user(
        &auth_state.database,
        signup.email,
        signup.name,
        password_hash,
    )
    .await
    .map_err(map_create_user_error)?;

    let access_token = issue_access_token(&auth_state, user.id)?;

    Ok((
        StatusCode::CREATED,
        Json(SessionResponse {
            user: Some(user.into()),
            access_token: Some(access_token),
        }),
    ))
}

async fn login(
    State(auth_state): State<AuthState>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<SessionResponse>> {
    let login = validate_login(payload)?;
    let user = user::authenticate(&auth_state.database, login.email, login.password)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "Invalid email or password."))?;

    let access_token = issue_access_token(&auth_state, user.id)?;

    Ok(Json(SessionResponse {
        user: Some(user.into()),
        access_token: Some(access_token),
    }))
}

async fn logout() -> Json<SessionResponse> {
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

    user::find_user_by_id(&auth_state.database, user_id)
        .await
        .map_err(internal_error)
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

fn issue_access_token(auth_state: &AuthState, user_id: i64) -> AppResult<String> {
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

fn verify_access_token(auth_state: &AuthState, token: &str) -> Option<i64> {
    let claims = decode::<Claims>(
        token,
        &DecodingKey::from_secret(auth_state.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .ok()?
    .claims;

    claims.sub.parse().ok()
}

fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn validate_signup(mut input: SignupRequest) -> AppResult<SignupRequest> {
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

fn validate_login(mut input: LoginRequest) -> AppResult<LoginRequest> {
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

fn map_create_user_error(error: CreateUserError) -> ApiError {
    match error {
        CreateUserError::DuplicateEmail => ApiError::new(
            StatusCode::CONFLICT,
            "An account with that email already exists.",
        ),
        CreateUserError::Database(error) => internal_error(error),
    }
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
