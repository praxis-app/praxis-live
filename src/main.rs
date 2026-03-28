use axum::{routing::get, Json, Router};
use serde_json::{json, Value};
use std::{env, net::SocketAddr};

async fn health() -> Json<Value> {
    Json(json!({"status": "ok"}))
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "praxis_live=info,tower_http=debug".into()),
        )
        .init();

    let app = Router::new().route("/health", get(health));

    let server_port = env::var("SERVER_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(3100);

    let addr = SocketAddr::from(([0, 0, 0, 0], server_port));
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
