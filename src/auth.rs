use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use axum_login::{
    tower_sessions::{MemoryStore, SessionManagerLayer},
    AuthManagerLayerBuilder, AuthUser, AuthnBackend,
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

type AuthSession = axum_login::AuthSession<AuthBackend>;
type AppResult<T> = Result<T, ApiError>;

const MIN_PASSWORD_LENGTH: usize = 8;
pub const STORAGE_NOTICE: &str =
    "Auth users are stored in Postgres. Sessions remain in memory until a persistent session store is configured.";
const USERS_TABLE_SQL: &str = r#"
    CREATE TABLE IF NOT EXISTS users (
        id BIGSERIAL PRIMARY KEY,
        email TEXT NOT NULL UNIQUE,
        name TEXT NOT NULL,
        password_hash TEXT NOT NULL
    )
"#;
const UNIQUE_VIOLATION_CODE: &str = "23505";

#[derive(Debug, Clone, FromRow)]
struct UserRecord {
    id: i64,
    email: String,
    name: String,
    password_hash: String,
}

impl AuthUser for UserRecord {
    type Id = i64;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.password_hash.as_bytes()
    }
}

#[derive(Clone, Debug)]
struct AuthBackend {
    pool: PgPool,
}

impl AuthBackend {
    async fn create_user(&self, new_user: NewUser) -> Result<UserRecord, sqlx::Error> {
        let password_hash = password_auth::generate_hash(new_user.password);

        sqlx::query_as::<_, UserRecord>(
            r#"
            INSERT INTO users (email, name, password_hash)
            VALUES ($1, $2, $3)
            RETURNING id, email, name, password_hash
            "#,
        )
        .bind(new_user.email)
        .bind(new_user.name)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await
    }
}

#[derive(Clone, Debug)]
struct Credentials {
    email: String,
    password: String,
}

impl AuthnBackend for AuthBackend {
    type User = UserRecord;
    type Credentials = Credentials;
    type Error = sqlx::Error;

    async fn authenticate(
        &self,
        credentials: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        let user = sqlx::query_as::<_, UserRecord>(
            r#"
            SELECT id, email, name, password_hash
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(credentials.email)
        .fetch_optional(&self.pool)
        .await?;

        let Some(user) = user else {
            return Ok(None);
        };

        match password_auth::verify_password(credentials.password, &user.password_hash) {
            Ok(()) => Ok(Some(user)),
            Err(_) => Ok(None),
        }
    }

    async fn get_user(&self, user_id: &i64) -> Result<Option<Self::User>, Self::Error> {
        sqlx::query_as::<_, UserRecord>(
            r#"
            SELECT id, email, name, password_hash
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }
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
struct NewUser {
    email: String,
    name: String,
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
struct SessionResponse {
    user: Option<PublicUser>,
}

pub async fn router(pool: PgPool) -> Result<Router, sqlx::Error> {
    sqlx::query(USERS_TABLE_SQL).execute(&pool).await?;

    let backend = AuthBackend { pool };
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);
    let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

    Ok(Router::new()
        .route("/auth/me", get(me))
        .route("/auth/signup", post(signup))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .layer(auth_layer))
}

async fn me(auth_session: AuthSession) -> Json<SessionResponse> {
    Json(SessionResponse {
        user: auth_session.user.map(Into::into),
    })
}

async fn signup(
    mut auth_session: AuthSession,
    Json(payload): Json<SignupRequest>,
) -> AppResult<(StatusCode, Json<SessionResponse>)> {
    let new_user = validate_signup(payload)?;

    let user = auth_session
        .backend
        .create_user(new_user)
        .await
        .map_err(map_create_user_error)?;

    auth_session.login(&user).await.map_err(internal_error)?;

    Ok((
        StatusCode::CREATED,
        Json(SessionResponse {
            user: Some(user.into()),
        }),
    ))
}

async fn login(
    mut auth_session: AuthSession,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<SessionResponse>> {
    let credentials = validate_login(payload)?;

    let user = auth_session
        .authenticate(credentials)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "Invalid email or password."))?;

    auth_session.login(&user).await.map_err(internal_error)?;

    Ok(Json(SessionResponse {
        user: Some(user.into()),
    }))
}

async fn logout(mut auth_session: AuthSession) -> AppResult<Json<SessionResponse>> {
    auth_session.logout().await.map_err(internal_error)?;

    Ok(Json(SessionResponse { user: None }))
}

fn validate_signup(input: SignupRequest) -> AppResult<NewUser> {
    let name = input.name.trim().to_owned();
    let email = normalize_email(&input.email);

    if name.chars().count() < 2 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "Name must be at least 2 characters long.",
        ));
    }

    if !looks_like_email(&email) {
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

    Ok(NewUser {
        email,
        name,
        password: input.password,
    })
}

fn validate_login(input: LoginRequest) -> AppResult<Credentials> {
    let email = normalize_email(&input.email);

    if !looks_like_email(&email) {
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

    Ok(Credentials {
        email,
        password: input.password,
    })
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
