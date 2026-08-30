use axum::http::StatusCode;
use serde_json::json;
use std::{collections::HashMap, io::Cursor};

use crate::support::{json_body, MultipartField, TestApp};

struct TestUser {
    token: String,
    user_id: String,
}

#[tokio::test]
async fn shared_channel_members_can_read_profile_pictures() {
    let app = TestApp::new().await;
    let admin = signup(&app, "admin@example.com", "admin", None).await;
    let server_id = create_server(&app, &admin).await;
    let invite_token = create_invite(&app, &admin, &server_id).await;
    let owner =
        signup(&app, "owner@example.com", "owner", Some(&invite_token)).await;
    let stranger = signup(&app, "stranger@example.com", "stranger", None).await;
    let image_id = upload_profile_picture(&app, &owner).await;
    let image_uri = format!("/api/users/{}/images/{image_id}", owner.user_id);

    let member_response = app.get_with_bearer(&image_uri, &admin.token).await;
    assert_eq!(member_response.status(), StatusCode::OK);
    assert_eq!(member_response.headers()["content-type"], "image/png");

    let stranger_response =
        app.get_with_bearer(&image_uri, &stranger.token).await;
    assert_eq!(stranger_response.status(), StatusCode::FORBIDDEN);
}

async fn signup(
    app: &TestApp,
    email: &str,
    name: &str,
    invite_token: Option<&str>,
) -> TestUser {
    let response = app
        .post_json(
            "/api/auth/signup",
            &json!({
                "email": email,
                "name": name,
                "password": "correct horse battery staple",
                "inviteToken": invite_token,
            }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = json_body(response).await;
    TestUser {
        token: body["access_token"].as_str().unwrap().to_owned(),
        user_id: body["user"]["id"].as_str().unwrap().to_owned(),
    }
}

async fn create_server(app: &TestApp, admin: &TestUser) -> String {
    let response = app
        .post_json_with_bearer(
            "/api/servers",
            &json!({
                "name": "Private server",
                "slug": "private-server",
                "description": null,
                "isDefaultServer": false,
            }),
            &admin.token,
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    json_body(response).await["server"]["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

async fn create_invite(
    app: &TestApp,
    admin: &TestUser,
    server_id: &str,
) -> String {
    let response = app
        .post_json_with_bearer(
            &format!("/api/servers/{server_id}/invites"),
            &json!({ "maxUses": null, "expiresAt": null }),
            &admin.token,
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    json_body(response).await["invite"]["token"]
        .as_str()
        .unwrap()
        .to_owned()
}

async fn upload_profile_picture(app: &TestApp, owner: &TestUser) -> String {
    let mut png = Cursor::new(Vec::new());
    image::DynamicImage::new_rgba8(1, 1)
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    let mut fields = HashMap::new();
    fields.insert(
        "file".to_owned(),
        MultipartField {
            name: "file".to_owned(),
            filename: Some("profile.png".to_owned()),
            content_type: Some("image/png".to_owned()),
            bytes: png.into_inner(),
        },
    );

    let response = app
        .post_multipart_with_bearer(
            "/api/users/profile-picture",
            &owner.token,
            fields,
        )
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    json_body(response).await["image"]["id"]
        .as_str()
        .unwrap()
        .to_owned()
}
