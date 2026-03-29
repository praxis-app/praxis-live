mod auth;

use axum::{routing::get, Json, Router};
use serde::Serialize;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::{env, error::Error, net::SocketAddr};
use tower_http::trace::TraceLayer;

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

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
        .route("/health", get(health))
        .merge(auth::router(database_pool).await?);

    let app = Router::new()
        .nest("/api", api)
        .layer(TraceLayer::new_for_http());

    let server_port = env::var("SERVER_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(3100);

    let addr = SocketAddr::from(([0, 0, 0, 0], server_port));
    tracing::info!("{}", auth::STORAGE_NOTICE);
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn connect_database() -> Result<sqlx::PgPool, Box<dyn Error + Send + Sync>> {
    let options = if let Ok(database_url) = env::var("DATABASE_URL") {
        database_url.parse::<PgConnectOptions>()?
    } else {
        let host = env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_owned());
        let port = env::var("DB_PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(5432);
        let username = env::var("DB_USERNAME").unwrap_or_else(|_| "postgres".to_owned());
        let password = env::var("DB_PASSWORD").unwrap_or_else(|_| "postgres".to_owned());
        let database = env::var("DB_SCHEMA").unwrap_or_else(|_| "postgres".to_owned());

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
