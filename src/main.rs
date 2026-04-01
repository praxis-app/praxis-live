mod auth;
mod config;
mod health;
mod user;
mod view;

use axum::{routing::get, Router};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::{env, error::Error, io, net::SocketAddr};
use tower_http::trace::TraceLayer;

use crate::config::required_env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "praxis_live=info,tower_http=debug".into()),
        )
        .init();

    let database_pool = connect_database().await?;
    let api = Router::new()
        .route("/health", get(health::health))
        .merge(auth::router(database_pool).await?);

    let app = Router::new().nest("/api", api);
    let app = view::attach(app);
    let app = app.layer(TraceLayer::new_for_http());

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

async fn connect_database() -> Result<sqlx::PgPool, Box<dyn Error + Send + Sync>> {
    let options = if let Ok(database_url) = env::var("DATABASE_URL") {
        database_url.parse::<PgConnectOptions>()?
    } else {
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

        PgConnectOptions::new()
            .host(&host)
            .port(port)
            .username(&username)
            .password(&password)
            .database(&database)
    };

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    Ok(pool)
}
