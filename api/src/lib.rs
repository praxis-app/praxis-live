mod auth;
mod health;
mod users;
mod view;

use axum::{routing::get, Router};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use std::{env, error::Error, io, net::SocketAddr};
use tower_http::trace::TraceLayer;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "api=info,praxis_live=info,tower_http=debug".into()),
        )
        .init();

    let database = connect_database_from_env().await?;
    let jwt_secret = required_env("AUTH_TOKEN_SECRET")?;
    let app = build_router(database, jwt_secret).layer(TraceLayer::new_for_http());

    let server_port = env::var("SERVER_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(3100);

    let addr = SocketAddr::from(([0, 0, 0, 0], server_port));
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

pub fn build_router(database: DatabaseConnection, jwt_secret: impl Into<String>) -> Router {
    let api = Router::new()
        .route("/health", get(health::health))
        .merge(auth::router(database, jwt_secret.into()));

    view::attach(Router::new().nest("/api", api))
}

pub async fn connect_database_from_env() -> Result<DatabaseConnection, Box<dyn Error + Send + Sync>>
{
    connect_database(&database_url_from_env()?, migrations_enabled()).await
}

pub async fn connect_database(
    database_url: &str,
    run_migrations: bool,
) -> Result<DatabaseConnection, Box<dyn Error + Send + Sync>> {
    let mut options = ConnectOptions::new(database_url.to_owned());
    options.max_connections(5);

    let database = Database::connect(options).await?;

    if run_migrations {
        tracing::info!("Running database migrations.");
        migrations::Migrator::up(&database, None).await?;
    }

    Ok(database)
}

pub fn database_url_from_env() -> Result<String, Box<dyn Error + Send + Sync>> {
    if let Ok(database_url) = env::var("DATABASE_URL") {
        return Ok(database_url);
    }

    let host = required_env("DB_HOST")?;
    let port = required_env("DB_PORT")?.parse::<u16>().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("DB_PORT must be a valid u16: {error}"),
        )
    })?;
    let username = required_env("DB_USERNAME")?;
    let password = required_env("DB_PASSWORD")?;
    let database = required_env("DB_SCHEMA")?;

    Ok(format!(
        "postgres://{username}:{password}@{host}:{port}/{database}"
    ))
}

pub fn migrations_enabled() -> bool {
    env::var("DB_MIGRATIONS")
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn required_env(name: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    env::var(name).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name} must be set before starting the server."),
        )
        .into()
    })
}
