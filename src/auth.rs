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
use std::{
    collections::HashMap,
    convert::Infallible,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc,
    },
};
use tokio::sync::RwLock;

type AuthSession = axum_login::AuthSession<AuthBackend>;
type AppResult<T> = Result<T, ApiError>;

const MIN_PASSWORD_LENGTH: usize = 8;
pub const STORAGE_WARNING: &str = "Auth users and sessions are currently stored in memory.";

#[derive(Debug, Clone)]
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

#[derive(Debug, Default)]
struct UserStore {
    users_by_id: HashMap<i64, UserRecord>,
    user_ids_by_email: HashMap<String, i64>,
}

#[derive(Clone, Debug)]
struct AuthBackend {
    store: Arc<RwLock<UserStore>>,
    next_user_id: Arc<AtomicI64>,
}

impl Default for AuthBackend {
    fn default() -> Self {
        Self {
            store: Arc::new(RwLock::new(UserStore::default())),
            next_user_id: Arc::new(AtomicI64::new(1)),
        }
    }
}

impl AuthBackend {
    async fn create_user(&self, new_user: NewUser) -> Result<UserRecord, CreateUserError> {
        let mut store = self.store.write().await;

        if store.user_ids_by_email.contains_key(&new_user.email) {
            return Err(CreateUserError::DuplicateEmail);
        }

        let user = UserRecord {
            id: self.next_user_id.fetch_add(1, Ordering::Relaxed),
            email: new_user.email.clone(),
            name: new_user.name,
            password_hash: password_auth::generate_hash(new_user.password),
        };

        store.user_ids_by_email.insert(new_user.email, user.id);
        store.users_by_id.insert(user.id, user.clone());

        Ok(user)
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
    type Error = Infallible;

    async fn authenticate(
        &self,
        credentials: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        let user = {
            let store = self.store.read().await;
            let user_id = store
                .user_ids_by_email
                .get(&normalize_email(&credentials.email));

            user_id.and_then(|id| store.users_by_id.get(id)).cloned()
        };

        let Some(user) = user else {
            return Ok(None);
        };

        match password_auth::verify_password(credentials.password, &user.password_hash) {
            Ok(()) => Ok(Some(user)),
            Err(_) => Ok(None),
        }
    }

    async fn get_user(&self, user_id: &i64) -> Result<Option<Self::User>, Self::Error> {
        let store = self.store.read().await;
        Ok(store.users_by_id.get(user_id).cloned())
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
enum CreateUserError {
    DuplicateEmail,
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

pub fn router() -> Router {
    let backend = AuthBackend::default();
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);
    let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

    Router::new()
        .route("/auth/me", get(me))
        .route("/auth/signup", post(signup))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .layer(auth_layer)
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
        .map_err(|error| match error {
            CreateUserError::DuplicateEmail => ApiError::new(
                StatusCode::CONFLICT,
                "An account with that email already exists.",
            ),
        })?;

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
