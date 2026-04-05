use axum::http::StatusCode;

use crate::support::{json_body, TestApp};

#[tokio::test]
async fn health_returns_ok_with_timestamp() {
    let app = TestApp::new().await;

    let response = app.get("/api/health").await;

    assert_eq!(response.status(), StatusCode::OK);

    let body = json_body(response).await;
    assert_eq!(body["status"], "ok");
    assert!(body["timestamp"].as_u64().unwrap_or_default() > 0);
}
