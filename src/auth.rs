use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use std::{
    env,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

type AppResult<T> = Result<T, ApiError>;

const ACCESS_TOKEN_TTL: Duration = Duration::from_secs(60 * 60 * 24 * 90);
const DEFAULT_AUTH_TOKEN_SECRET: &str = "dev-only-change-me";
const MIN_PASSWORD_LENGTH: usize = 8;
const USERS_TABLE_SQL: &str = r#"
    CREATE TABLE IF NOT EXISTS users (
        id BIGSERIAL PRIMARY KEY,
        email TEXT NOT NULL UNIQUE,
        name TEXT NOT NULL,
        password_hash TEXT NOT NULL
    )
"#;
const UNIQUE_VIOLATION_CODE: &str = "23505";

#[derive(Clone, Debug)]
struct AuthState {
    pool: PgPool,
    jwt_secret: Arc<str>,
}

#[derive(Debug, Clone, FromRow)]
struct UserRecord {
    id: i64,
    email: String,
    name: String,
    password_hash: String,
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
struct ApiError {
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
struct PublicUser {
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionResponse {
    user: Option<PublicUser>,
    access_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: u64,
}

pub async fn router(pool: PgPool) -> Result<Router, sqlx::Error> {
    sqlx::query(USERS_TABLE_SQL).execute(&pool).await?;

    let jwt_secret = env::var("AUTH_TOKEN_SECRET").unwrap_or_else(|_| {
        tracing::warn!(
            "AUTH_TOKEN_SECRET is not set; using the built-in development secret. Set AUTH_TOKEN_SECRET for any shared or production environment."
        );
        DEFAULT_AUTH_TOKEN_SECRET.to_owned()
    });

    let auth_state = AuthState {
        pool,
        jwt_secret: Arc::<str>::from(jwt_secret),
    };

    Ok(Router::new()
        .route("/auth/me", get(me))
        .route("/auth/signup", post(signup))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .with_state(auth_state))
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

    let user = sqlx::query_as::<_, UserRecord>(
        r#"
        INSERT INTO users (email, name, password_hash)
        VALUES ($1, $2, $3)
        RETURNING id, email, name, password_hash
        "#,
    )
    .bind(signup.email)
    .bind(signup.name)
    .bind(password_hash)
    .fetch_one(&auth_state.pool)
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
    let user = authenticate(&auth_state.pool, login)
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

    sqlx::query_as::<_, UserRecord>(
        r#"
        SELECT id, email, name, password_hash
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&auth_state.pool)
    .await
    .map_err(internal_error)
}

async fn authenticate(
    pool: &PgPool,
    login: LoginRequest,
) -> Result<Option<UserRecord>, sqlx::Error> {
    let user = sqlx::query_as::<_, UserRecord>(
        r#"
        SELECT id, email, name, password_hash
        FROM users
        WHERE email = $1
        "#,
    )
    .bind(login.email)
    .fetch_optional(pool)
    .await?;

    let Some(user) = user else {
        return Ok(None);
    };

    Ok(
        password_auth::verify_password(login.password, &user.password_hash)
            .ok()
            .map(|()| user),
    )
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

fn map_create_user_error(error: sqlx::Error) -> ApiError {
    if is_unique_violation(&error) {
        return ApiError::new(
            StatusCode::CONFLICT,
            "An account with that email already exists.",
        );
    }

    internal_error(error)
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(|database_error| database_error.code())
        .is_some_and(|code| code == UNIQUE_VIOLATION_CODE)
}

fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

fn looks_like_email(email: &str) -> bool {
    let mut parts = email.split('@');
    let local = parts.next().unwrap_or_default();
    let domain = parts.next().unwrap_or_default();

    !local.is_empty() && domain.contains('.') && parts.next().is_none()
}

fn internal_error(error: impl std::error::Error) -> ApiError {
    tracing::error!("authentication flow failed: {error}");
    ApiError::new(
        StatusCode::INTERNAL_SERVER_ERROR,
        "The server could not complete the authentication request.",
    )
}
