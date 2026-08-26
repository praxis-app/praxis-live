use axum::http::StatusCode;
use serde_json::json;
use std::{collections::HashMap, io::Cursor};

use crate::support::{json_body, MultipartField, TestApp};

#[tokio::test]
async fn create_message_atomically_supports_text_and_images() {
    let app = TestApp::new().await;
    let token = signup_and_get_token(&app).await;
    let default_server_id = default_server_id(&app).await;

    let channels_response = app
        .get(&format!("/api/servers/{default_server_id}/channels"))
        .await;
    let channel_id = json_body(channels_response).await["channels"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let mut fields = HashMap::new();
    let mut png = Cursor::new(Vec::new());
    image::DynamicImage::new_rgba8(1, 1)
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    fields.insert(
        "files".to_owned(),
        MultipartField {
            name: "files".to_owned(),
            filename: Some("pixel.png".to_owned()),
            content_type: Some("image/png".to_owned()),
            bytes: png.into_inner(),
        },
    );
    fields.insert(
        "payload".to_owned(),
        MultipartField {
            name: "payload".to_owned(),
            filename: None,
            content_type: Some("application/json".to_owned()),
            bytes: serde_json::to_vec(&json!({ "body": "hello world" }))
                .unwrap(),
        },
    );

    let create_response = app
        .post_multipart_with_bearer(
            &format!("/api/servers/{default_server_id}/channels/{channel_id}/messages"),
            &token,
            fields,
        )
        .await;
    assert_eq!(create_response.status(), StatusCode::OK);

    let create_body = json_body(create_response).await;
    let message = &create_body["message"];
    let message_id = message["id"].as_str().unwrap().to_owned();
    let image_id = message["images"][0]["id"].as_str().unwrap().to_owned();

    assert_eq!(message["body"], "hello world");
    assert!(message["images"][0]["isPlaceholder"].is_null());
    assert_eq!(message["user"]["name"], "Person Example");

    let feed_response = app
        .get(&format!(
            "/api/servers/{default_server_id}/channels/{channel_id}/feed?limit=20"
        ))
        .await;
    assert_eq!(feed_response.status(), StatusCode::OK);

    let feed_body = json_body(feed_response).await;
    assert_eq!(feed_body["feed"][0]["type"], "message");
    assert_eq!(feed_body["feed"][0]["id"], message_id);
    assert!(feed_body["startCursor"].is_string());
    assert!(feed_body["nextCursor"].is_string());
    assert_eq!(feed_body["hasMore"], false);

    let image_response = app
        .get(&format!(
            "/api/servers/{default_server_id}/channels/{channel_id}/messages/{message_id}/images/{image_id}"
        ))
        .await;
    assert_eq!(image_response.status(), StatusCode::OK);
    assert_eq!(image_response.headers()["content-type"], "image/png");
}

#[tokio::test]
async fn thread_replies_are_paginated_summarized_and_excluded_from_feed() {
    let app = TestApp::new().await;
    let token = signup_and_get_token(&app).await;
    let server_id = default_server_id(&app).await;
    let channel_id = first_channel_id(&app, &server_id, Some(&token)).await;
    let root =
        create_message(&app, &token, &server_id, &channel_id, "Root").await;

    let first_reply_response = app
        .post_json_with_bearer(
            &format!(
                "/api/servers/{server_id}/channels/{channel_id}/messages/{root}/replies"
            ),
            &json!({ "body": "First reply" }),
            &token,
        )
        .await;
    assert_eq!(first_reply_response.status(), StatusCode::OK);
    let first_reply = json_body(first_reply_response).await["message"]["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let second_reply_response = app
        .post_json_with_bearer(
            &format!(
                "/api/servers/{server_id}/channels/{channel_id}/messages/{root}/replies"
            ),
            &json!({
                "body": "Second reply",
                "parentMessageId": first_reply
            }),
            &token,
        )
        .await;
    assert_eq!(second_reply_response.status(), StatusCode::OK);

    let latest_page_response = app
        .get_with_bearer(
            &format!(
                "/api/servers/{server_id}/channels/{channel_id}/messages/{root}/replies?limit=1"
            ),
            &token,
        )
        .await;
    assert_eq!(latest_page_response.status(), StatusCode::OK);
    let latest_page = json_body(latest_page_response).await;
    assert_eq!(latest_page["root"]["id"], root);
    assert_eq!(latest_page["root"]["replyCount"], 2);
    assert!(latest_page["root"]["latestReplyAt"].is_string());
    assert_eq!(latest_page["replies"][0]["body"], "Second reply");
    assert_eq!(latest_page["replies"][0]["threadRootId"], root);
    assert_eq!(latest_page["replies"][0]["parentMessageId"], first_reply);
    assert_eq!(latest_page["replies"][0]["replyCount"], 0);
    assert!(latest_page["replies"][0]["latestReplyAt"].is_null());
    assert_eq!(latest_page["hasMore"], true);

    let cursor = encode_query_value(
        latest_page["nextCursor"].as_str().expect("expected cursor"),
    );
    let older_page_response = app
        .get_with_bearer(
            &format!(
                "/api/servers/{server_id}/channels/{channel_id}/messages/{root}/replies?before={cursor}&limit=1"
            ),
            &token,
        )
        .await;
    assert_eq!(older_page_response.status(), StatusCode::OK);
    let older_page = json_body(older_page_response).await;
    assert_eq!(older_page["replies"][0]["body"], "First reply");
    assert_eq!(older_page["hasMore"], false);

    let feed_response = app
        .get(&format!(
            "/api/servers/{server_id}/channels/{channel_id}/feed?limit=20"
        ))
        .await;
    assert_eq!(feed_response.status(), StatusCode::OK);
    let feed = json_body(feed_response).await;
    assert_eq!(feed["feed"].as_array().unwrap().len(), 1);
    assert_eq!(feed["feed"][0]["id"], root);
    assert_eq!(feed["feed"][0]["replyCount"], 2);
    assert!(feed["feed"][0]["latestReplyAt"].is_string());
}

#[tokio::test]
async fn thread_replies_reject_cross_thread_parents_and_reply_roots() {
    let app = TestApp::new().await;
    let token = signup_and_get_token(&app).await;
    let server_id = default_server_id(&app).await;
    let channel_id = first_channel_id(&app, &server_id, Some(&token)).await;
    let first_root =
        create_message(&app, &token, &server_id, &channel_id, "First root")
            .await;
    let second_root =
        create_message(&app, &token, &server_id, &channel_id, "Second root")
            .await;
    let other_reply_response = app
        .post_json_with_bearer(
            &format!(
                "/api/servers/{server_id}/channels/{channel_id}/messages/{second_root}/replies"
            ),
            &json!({ "body": "Other thread reply" }),
            &token,
        )
        .await;
    assert_eq!(other_reply_response.status(), StatusCode::OK);
    let other_reply = json_body(other_reply_response).await["message"]["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let cross_thread_response = app
        .post_json_with_bearer(
            &format!(
                "/api/servers/{server_id}/channels/{channel_id}/messages/{first_root}/replies"
            ),
            &json!({
                "body": "Invalid parent",
                "parentMessageId": other_reply
            }),
            &token,
        )
        .await;
    assert_eq!(
        cross_thread_response.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let reply_as_root_response = app
        .get_with_bearer(
            &format!(
                "/api/servers/{server_id}/channels/{channel_id}/messages/{other_reply}/replies"
            ),
            &token,
        )
        .await;
    assert_eq!(reply_as_root_response.status(), StatusCode::NOT_FOUND);

    let after_response = app
        .get_with_bearer(
            &format!(
                "/api/servers/{server_id}/channels/{channel_id}/messages/{first_root}/replies?after=invalid"
            ),
            &token,
        )
        .await;
    assert_eq!(after_response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn thread_replies_support_image_only_content() {
    let app = TestApp::new().await;
    let token = signup_and_get_token(&app).await;
    let server_id = default_server_id(&app).await;
    let channel_id = first_channel_id(&app, &server_id, Some(&token)).await;
    let root =
        create_message(&app, &token, &server_id, &channel_id, "Image thread")
            .await;

    let mut png = Cursor::new(Vec::new());
    image::DynamicImage::new_rgba8(1, 1)
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    let mut fields = HashMap::new();
    fields.insert(
        "files".to_owned(),
        MultipartField {
            name: "files".to_owned(),
            filename: Some("reply.png".to_owned()),
            content_type: Some("image/png".to_owned()),
            bytes: png.into_inner(),
        },
    );
    fields.insert(
        "payload".to_owned(),
        MultipartField {
            name: "payload".to_owned(),
            filename: None,
            content_type: Some("application/json".to_owned()),
            bytes: serde_json::to_vec(&json!({})).unwrap(),
        },
    );

    let response = app
        .post_multipart_with_bearer(
            &format!(
                "/api/servers/{server_id}/channels/{channel_id}/messages/{root}/replies"
            ),
            &token,
            fields,
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let reply = json_body(response).await;
    assert!(reply["message"]["body"].is_null());
    assert_eq!(reply["message"]["images"].as_array().unwrap().len(), 1);
    assert_eq!(reply["message"]["threadRootId"], root);
}

#[tokio::test]
async fn logged_out_users_cannot_read_non_default_server_channel_feeds() {
    let app = TestApp::new().await;
    let token = signup_and_get_token(&app).await;

    let server_response = app
        .post_json_with_bearer(
            "/api/servers",
            &json!({
                "name": "Private server",
                "slug": "private-server",
                "description": null,
                "isDefaultServer": false
            }),
            &token,
        )
        .await;
    assert_eq!(server_response.status(), StatusCode::OK);
    let server_id = json_body(server_response).await["server"]["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let channels_response = app
        .get_with_bearer(&format!("/api/servers/{server_id}/channels"), &token)
        .await;
    let channel_id = json_body(channels_response).await["channels"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let feed_response = app
        .get(&format!(
            "/api/servers/{server_id}/channels/{channel_id}/feed"
        ))
        .await;
    assert_eq!(feed_response.status(), StatusCode::FORBIDDEN);
}

async fn default_server_id(app: &TestApp) -> String {
    let response = app.get("/api/servers/default").await;
    let body = json_body(response).await;
    body["server"]["id"].as_str().unwrap().to_owned()
}

async fn first_channel_id(
    app: &TestApp,
    server_id: &str,
    token: Option<&str>,
) -> String {
    let uri = format!("/api/servers/{server_id}/channels");
    let response = match token {
        Some(token) => app.get_with_bearer(&uri, token).await,
        None => app.get(&uri).await,
    };
    json_body(response).await["channels"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

async fn create_message(
    app: &TestApp,
    token: &str,
    server_id: &str,
    channel_id: &str,
    body: &str,
) -> String {
    let response = app
        .post_json_with_bearer(
            &format!("/api/servers/{server_id}/channels/{channel_id}/messages"),
            &json!({ "body": body }),
            token,
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await["message"]["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn encode_query_value(value: &str) -> String {
    value.replace('+', "%2B").replace('|', "%7C")
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
