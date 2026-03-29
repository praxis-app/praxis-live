use serde::Serialize;
use sqlx::{FromRow, PgPool};

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

pub(crate) enum CreateUserError {
    DuplicateEmail,
    Database(sqlx::Error),
}

pub(crate) async fn ensure_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(USERS_TABLE_SQL).execute(pool).await?;
    Ok(())
}

pub(crate) async fn create_user(
    pool: &PgPool,
    email: String,
    name: String,
    password_hash: String,
) -> Result<UserRecord, CreateUserError> {
    sqlx::query_as::<_, UserRecord>(
        r#"
        INSERT INTO users (email, name, password_hash)
        VALUES ($1, $2, $3)
        RETURNING id, email, name, password_hash
        "#,
    )
    .bind(email)
    .bind(name)
    .bind(password_hash)
    .fetch_one(pool)
    .await
    .map_err(map_create_user_error)
}

pub(crate) async fn find_user_by_id(
    pool: &PgPool,
    user_id: i64,
) -> Result<Option<UserRecord>, sqlx::Error> {
    sqlx::query_as::<_, UserRecord>(
        r#"
        SELECT id, email, name, password_hash
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub(crate) async fn authenticate(
    pool: &PgPool,
    email: String,
    password: String,
) -> Result<Option<UserRecord>, sqlx::Error> {
    let user = sqlx::query_as::<_, UserRecord>(
        r#"
        SELECT id, email, name, password_hash
        FROM users
        WHERE email = $1
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;

    let Some(user) = user else {
        return Ok(None);
    };

    Ok(password_auth::verify_password(password, &user.password_hash)
        .ok()
        .map(|()| user))
}

fn map_create_user_error(error: sqlx::Error) -> CreateUserError {
    if is_unique_violation(&error) {
        return CreateUserError::DuplicateEmail;
    }

    CreateUserError::Database(error)
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(|database_error| database_error.code())
        .is_some_and(|code| code == UNIQUE_VIOLATION_CODE)
}
