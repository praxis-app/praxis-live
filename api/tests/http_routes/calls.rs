use axum::http::StatusCode;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde_json::{json, Value};

use crate::support::{json_body, TestApp};

#[tokio::test]
async fn join_channel_call_returns_livekit_connection_config() {
    let app = TestApp::new().await;
    let token = signup_and_get_token(&app).await;
    let default_server_id = default_server_id(&app).await;
    let channel_id = first_channel_id(&app, &default_server_id).await;

    let response = app
        .post_json_with_bearer(
            &format!(
                "/api/servers/{default_server_id}/channels/{channel_id}/calls/join"
            ),
            &json!({}),
            &token,
        )
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;

    assert_eq!(body["livekitUrl"], "ws://livekit.test:7880");
    assert_eq!(body["roomName"], body["call"]["roomName"]);
    assert_eq!(body["call"]["channelId"], channel_id);
    assert_eq!(body["call"]["status"], "starting");

    let token = body["token"].as_str().unwrap();
    let claims = decode::<Value>(
        token,
        &DecodingKey::from_secret("livekit-test-secret".as_bytes()),
        &Validation::default(),
    )
    .expect("expected LiveKit token to decode")
    .claims;

    assert_eq!(claims["iss"], "livekit-test-key");
    assert_eq!(claims["video"]["room"], body["roomName"]);
    assert_eq!(claims["video"]["roomJoin"], true);
    assert_eq!(claims["video"]["canPublishData"], false);
}

async fn default_server_id(app: &TestApp) -> String {
    let response = app.get("/api/servers/default").await;
    let body = json_body(response).await;
    body["server"]["id"].as_str().unwrap().to_owned()
}

async fn first_channel_id(app: &TestApp, server_id: &str) -> String {
    let response = app.get(&format!("/api/servers/{server_id}/channels")).await;
    let body = json_body(response).await;
    body["channels"][0]["id"].as_str().unwrap().to_owned()
}

async fn signup_and_get_token(app: &TestApp) -> String {
    let response = app
        .post_json(
            "/api/auth/signup",
            &json!({
                "email": "caller@example.com",
                "name": "Caller Example",
                "password": "correct horse battery staple",
            }),
        )
        .await;

    let body = json_body(response).await;
    body["access_token"].as_str().unwrap().to_owned()
}
