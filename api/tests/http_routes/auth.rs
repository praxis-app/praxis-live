use axum::http::StatusCode;
use serde_json::json;

use crate::support::{json_body, TestApp};

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
    let user_id = body["user"]["id"].as_str().unwrap();
    assert!(uuid::Uuid::parse_str(user_id).is_ok());
    assert!(body["access_token"].as_str().is_some());
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
async fn login_returns_the_authenticated_user_and_access_token() {
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

    assert_eq!(login_body["user"], signup_user);
    assert!(login_body["access_token"].as_str().is_some());
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
