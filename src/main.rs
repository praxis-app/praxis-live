mod auth;
mod config;
mod user;

use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::{
    env,
    error::Error,
    io,
    net::SocketAddr,
    path::{Component, Path, PathBuf},
};
use tower_http::trace::TraceLayer;

use crate::config::required_env;

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

    let app = Router::new().nest("/api", api);
    let app = attach_frontend(app);
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

fn attach_frontend(app: Router) -> Router {
    let frontend_dist = frontend_dist_dir();
    let index_path = frontend_dist.join("index.html");

    if frontend_dist.is_dir() && index_path.is_file() {
        tracing::info!("Serving frontend assets from {}", frontend_dist.display());

        app.fallback({
            let frontend_dist = frontend_dist.clone();
            move |uri| frontend_fallback(uri, frontend_dist.clone())
        })
    } else {
        tracing::warn!(
            "Frontend assets were not found at {}; serving API routes only.",
            frontend_dist.display()
        );

        app
    }
}

fn frontend_dist_dir() -> PathBuf {
    env::var_os("FRONTEND_DIST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("view/dist"))
}

async fn frontend_fallback(uri: Uri, frontend_dist: PathBuf) -> Response {
    let request_path = uri.path();

    if request_path == "/api" || request_path.starts_with("/api/") {
        return StatusCode::NOT_FOUND.into_response();
    }

    let Some(candidate_path) = frontend_request_path(&frontend_dist, request_path) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    if candidate_path.is_file() {
        return serve_file(candidate_path, StatusCode::OK).await;
    }

    if Path::new(request_path.trim_start_matches('/'))
        .extension()
        .is_some()
    {
        return StatusCode::NOT_FOUND.into_response();
    }

    serve_file(frontend_dist.join("index.html"), StatusCode::OK).await
}

fn frontend_request_path(frontend_dist: &Path, request_path: &str) -> Option<PathBuf> {
    let mut resolved = frontend_dist.to_path_buf();
    let trimmed_path = request_path.trim_start_matches('/');

    if trimmed_path.is_empty() {
        resolved.push("index.html");
        return Some(resolved);
    }

    for component in Path::new(trimmed_path).components() {
        match component {
            Component::Normal(segment) => resolved.push(segment),
            Component::CurDir => {}
            Component::RootDir | Component::ParentDir | Component::Prefix(_) => return None,
        }
    }

    Some(resolved)
}

async fn serve_file(path: PathBuf, status: StatusCode) -> Response {
    match tokio::fs::read(&path).await {
        Ok(contents) => Response::builder()
            .status(status)
            .header(
                header::CONTENT_TYPE,
                mime_guess::from_path(&path).first_or_octet_stream().as_ref(),
            )
            .body(Body::from(contents))
            .unwrap_or_else(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("failed to build frontend response: {error}"),
                )
                    .into_response()
            }),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to read frontend asset {}: {error}", path.display()),
        )
            .into_response(),
    }
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
