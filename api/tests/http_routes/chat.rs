use axum::http::StatusCode;
use serde_json::json;
use std::collections::HashMap;

use crate::support::{json_body, MultipartField, TestApp};

const DEFAULT_SERVER_ID: &str = "11111111-1111-1111-1111-111111111111";

#[tokio::test]
async fn create_message_and_upload_image_support_text_and_images() {
    let app = TestApp::new().await;
    let token = signup_and_get_token(&app).await;

    let channels_response = app
        .get(&format!("/api/servers/{DEFAULT_SERVER_ID}/channels"))
        .await;
    let channel_id = json_body(channels_response).await["channels"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let create_response = app
        .post_json_with_bearer(
            &format!("/api/servers/{DEFAULT_SERVER_ID}/channels/{channel_id}/messages"),
            &json!({
                "body": "hello world",
                "imageCount": 1
            }),
            &token,
        )
        .await;
    assert_eq!(create_response.status(), StatusCode::OK);

    let create_body = json_body(create_response).await;
    let message = &create_body["message"];
    let message_id = message["id"].as_str().unwrap().to_owned();
    let image_id = message["images"][0]["id"].as_str().unwrap().to_owned();

    assert_eq!(message["body"], "hello world");
    assert_eq!(message["images"][0]["isPlaceholder"], true);
    assert_eq!(message["user"]["name"], "Person Example");

    let feed_response = app
        .get(&format!(
            "/api/servers/{DEFAULT_SERVER_ID}/channels/{channel_id}/feed?offset=0&limit=20"
        ))
        .await;
    assert_eq!(feed_response.status(), StatusCode::OK);

    let feed_body = json_body(feed_response).await;
    assert_eq!(feed_body["feed"][0]["type"], "message");
    assert_eq!(feed_body["feed"][0]["id"], message_id);

    let mut fields = HashMap::new();
    fields.insert(
        "file".to_owned(),
        MultipartField {
            name: "file".to_owned(),
            filename: Some("pixel.png".to_owned()),
            content_type: Some("image/png".to_owned()),
            bytes: vec![137, 80, 78, 71, 13, 10, 26, 10],
        },
    );

    let upload_response = app
        .post_multipart_with_bearer(
            &format!(
                "/api/servers/{DEFAULT_SERVER_ID}/channels/{channel_id}/messages/{message_id}/images/{image_id}/upload"
            ),
            &token,
            fields,
        )
        .await;
    assert_eq!(upload_response.status(), StatusCode::CREATED);

    let upload_body = json_body(upload_response).await;
    assert_eq!(upload_body["image"]["id"], image_id);
    assert!(upload_body["image"]["isPlaceholder"].is_null());

    let image_response = app
        .get(&format!(
            "/api/servers/{DEFAULT_SERVER_ID}/channels/{channel_id}/messages/{message_id}/images/{image_id}"
        ))
        .await;
    assert_eq!(image_response.status(), StatusCode::OK);
    assert_eq!(image_response.headers()["content-type"], "image/png");
}

async fn signup_and_get_token(app: &TestApp) -> String {
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

    let body = json_body(response).await;
    body["access_token"].as_str().unwrap().to_owned()
}
