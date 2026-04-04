mod auth;
mod health;
mod user;
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
                .unwrap_or_else(|_| "praxis_live=info,tower_http=debug".into()),
        )
        .init();

    let database = connect_database().await?;
    let jwt_secret = required_env("AUTH_TOKEN_SECRET")?;
    let app = router(database, jwt_secret).layer(TraceLayer::new_for_http());

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

fn router(database: DatabaseConnection, jwt_secret: String) -> Router {
    let api = Router::new()
        .route("/health", get(health::health))
        .merge(auth::router(database, jwt_secret));

    view::attach(Router::new().nest("/api", api))
}

async fn connect_database() -> Result<DatabaseConnection, Box<dyn Error + Send + Sync>> {
    let mut options = ConnectOptions::new(database_url()?);
    options.max_connections(5);

    let database = Database::connect(options).await?;

    if migrations_enabled() {
        tracing::info!("Running database migrations.");
        migrations::Migrator::up(&database, None).await?;
    } else {
        tracing::info!("DB_MIGRATIONS is not set to true. Skipping migrations.");
    }

    Ok(database)
}

fn database_url() -> Result<String, Box<dyn Error + Send + Sync>> {
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

fn migrations_enabled() -> bool {
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
