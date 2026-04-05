mod support;

use axum::http::StatusCode;
use serde_json::json;
use support::{json_body, TestApp};

#[tokio::test]
async fn health_returns_ok_with_timestamp() {
    let app = TestApp::new().await;

    let response = app.get("/api/health").await;

    assert_eq!(response.status(), StatusCode::OK);

    let body = json_body(response).await;
    assert_eq!(body["status"], "ok");
    assert!(body["timestamp"].as_u64().unwrap_or_default() > 0);
}

#[tokio::test]
async fn signup_returns_created_user_and_access_token() {
    let app = TestApp::new().await;

    let response = app
        .post_json(
            "/api/auth/signup",
            &json!({
                "email": "person@example.com",
                "name": "Person Example",
                "password": "correct horse battery staple",
            }),
        )
        .await;

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = json_body(response).await;
    assert_eq!(body["user"]["email"], "person@example.com");
    assert_eq!(body["user"]["name"], "Person Example");
    assert!(body["user"]["id"].as_i64().unwrap_or_default() > 0);
    assert!(body["access_token"].as_str().is_some());
}

#[tokio::test]
async fn signup_rejects_invalid_payloads() {
    let app = TestApp::new().await;

    let response = app
        .post_json(
            "/api/auth/signup",
            &json!({
                "email": "not-an-email",
                "name": "A",
                "password": "short",
            }),
        )
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = json_body(response).await;
    assert_eq!(body["error"], "Name must be at least 2 characters long.");
}

#[tokio::test]
async fn signup_rejects_duplicate_email_after_normalization() {
    let app = TestApp::new().await;

    let first_response = app
        .post_json(
            "/api/auth/signup",
            &json!({
                "email": "person@example.com",
                "name": "Original Person",
                "password": "correct horse battery staple",
            }),
        )
        .await;
    assert_eq!(first_response.status(), StatusCode::CREATED);

    let second_response = app
        .post_json(
            "/api/auth/signup",
            &json!({
                "email": "  PERSON@example.com ",
                "name": "Another Person",
                "password": "correct horse battery staple",
            }),
        )
        .await;

    assert_eq!(second_response.status(), StatusCode::CONFLICT);

    let body = json_body(second_response).await;
    assert_eq!(body["error"], "An account with that email already exists.");
}

#[tokio::test]
async fn login_and_me_return_the_authenticated_user() {
    let app = TestApp::new().await;

    let signup_response = app
        .post_json(
            "/api/auth/signup",
            &json!({
                "email": "person@example.com",
                "name": "Person Example",
                "password": "correct horse battery staple",
            }),
        )
        .await;
    let signup_body = json_body(signup_response).await;
    let signup_user = signup_body["user"].clone();

    let login_response = app
        .post_json(
            "/api/auth/login",
            &json!({
                "email": "PERSON@example.com",
                "password": "correct horse battery staple",
            }),
        )
        .await;

    assert_eq!(login_response.status(), StatusCode::OK);

    let login_body = json_body(login_response).await;
    let access_token = login_body["access_token"]
        .as_str()
        .expect("expected login access token")
        .to_owned();

    assert_eq!(login_body["user"], signup_user);

    let me_response = app.get_with_bearer("/api/auth/me", &access_token).await;

    assert_eq!(me_response.status(), StatusCode::OK);

    let me_body = json_body(me_response).await;
    assert_eq!(me_body["user"], signup_user);
    assert!(me_body["access_token"].is_null());
}

#[tokio::test]
async fn login_rejects_invalid_credentials() {
    let app = TestApp::new().await;

    let signup_response = app
        .post_json(
            "/api/auth/signup",
            &json!({
                "email": "person@example.com",
                "name": "Person Example",
                "password": "correct horse battery staple",
            }),
        )
        .await;
    assert_eq!(signup_response.status(), StatusCode::CREATED);

    let response = app
        .post_json(
            "/api/auth/login",
            &json!({
                "email": "person@example.com",
                "password": "wrong password",
            }),
        )
        .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body = json_body(response).await;
    assert_eq!(body["error"], "Invalid email or password.");
}

#[tokio::test]
async fn logout_returns_an_empty_session_payload() {
    let app = TestApp::new().await;

    let response = app.post_empty("/api/auth/logout").await;

    assert_eq!(response.status(), StatusCode::OK);

    let body = json_body(response).await;
    assert!(body["user"].is_null());
    assert!(body["access_token"].is_null());
}
