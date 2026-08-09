use axum::http::StatusCode;
use entity::{poll_actions, poll_images, polls, server_roles};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde_json::{json, Value};
use uuid::Uuid;

use std::{collections::HashMap, io::Cursor};

use crate::support::{json_body, MultipartField, TestApp};

#[tokio::test]
async fn poll_images_are_stored_during_creation_and_can_be_read() {
    let app = TestApp::new().await;
    let token = signup(&app, "poll-image@example.com", "Poll Image").await;
    let default_server = json_body(app.get("/api/servers/default").await).await;
    let server_id = default_server["server"]["id"].as_str().unwrap();
    let channels =
        json_body(app.get(&format!("/api/servers/{server_id}/channels")).await)
            .await;
    let channel_id = channels["channels"][0]["id"].as_str().unwrap();

    let mut png = Cursor::new(Vec::new());
    image::DynamicImage::new_rgba8(1, 1)
        .write_to(&mut png, image::ImageFormat::Png)
        .expect("test PNG should encode");
    let png_bytes = png.into_inner();
    let mut fields = HashMap::new();
    fields.insert(
        "payload".to_owned(),
        MultipartField {
            name: "payload".to_owned(),
            filename: None,
            content_type: None,
            bytes: serde_json::to_vec(&json!({
                "body": "Which option?",
                "pollType": "poll",
                "options": ["One", "Two"],
            }))
            .unwrap(),
        },
    );
    fields.insert(
        "image".to_owned(),
        MultipartField {
            name: "files".to_owned(),
            filename: Some("pixel.png".to_owned()),
            content_type: Some("text/html".to_owned()),
            bytes: png_bytes.clone(),
        },
    );
    fields.insert(
        "second-image".to_owned(),
        MultipartField {
            name: "files".to_owned(),
            filename: Some("second-pixel.png".to_owned()),
            content_type: Some("image/png".to_owned()),
            bytes: png_bytes.clone(),
        },
    );

    let response = app
        .post_multipart_with_bearer(
            &format!("/api/servers/{server_id}/channels/{channel_id}/polls"),
            &token,
            fields,
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    let poll_id = body["poll"]["id"].as_str().unwrap();
    let image_id = body["poll"]["images"][0]["id"].as_str().unwrap();
    assert_eq!(body["poll"]["images"].as_array().unwrap().len(), 2);

    assert_eq!(
        poll_images::Entity::find()
            .filter(
                poll_images::Column::PollId
                    .eq(Uuid::parse_str(poll_id).unwrap()),
            )
            .count(app.database())
            .await
            .unwrap(),
        2,
    );

    let image_response = app
        .get(&format!(
            "/api/servers/{server_id}/channels/{channel_id}/polls/{poll_id}/images/{image_id}"
        ))
        .await;
    assert_eq!(image_response.status(), StatusCode::OK);
    assert_eq!(image_response.headers()["content-type"], "image/png");
    assert_eq!(
        image_response.headers()["x-content-type-options"],
        "nosniff"
    );

    let poll_count = polls::Entity::find().count(app.database()).await.unwrap();
    let mut invalid_fields = HashMap::new();
    invalid_fields.insert(
        "payload".to_owned(),
        MultipartField {
            name: "payload".to_owned(),
            filename: None,
            content_type: None,
            bytes: serde_json::to_vec(&json!({
                "body": "This should roll back",
                "pollType": "poll",
                "options": ["One", "Two"],
            }))
            .unwrap(),
        },
    );
    invalid_fields.insert(
        "valid-image".to_owned(),
        MultipartField {
            name: "files".to_owned(),
            filename: Some("pixel.png".to_owned()),
            content_type: Some("image/png".to_owned()),
            bytes: png_bytes,
        },
    );
    invalid_fields.insert(
        "invalid-image".to_owned(),
        MultipartField {
            name: "files".to_owned(),
            filename: Some("not-an-image.png".to_owned()),
            content_type: Some("image/png".to_owned()),
            bytes: b"not an image".to_vec(),
        },
    );

    let invalid_response = app
        .post_multipart_with_bearer(
            &format!("/api/servers/{server_id}/channels/{channel_id}/polls"),
            &token,
            invalid_fields,
        )
        .await;
    assert_eq!(invalid_response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        polls::Entity::find().count(app.database()).await.unwrap(),
        poll_count,
    );
}

#[tokio::test]
async fn concurrent_votes_ratify_and_execute_a_proposal_action_once() {
    let app = TestApp::new().await;
    let proposer = signup(&app, "proposer@example.com", "Proposer").await;
    let voter_a = signup(&app, "voter-a@example.com", "Voter A").await;
    let voter_b = signup(&app, "voter-b@example.com", "Voter B").await;
    let default_server = json_body(app.get("/api/servers/default").await).await;
    let server_id = default_server["server"]["id"].as_str().unwrap();

    let channels =
        json_body(app.get(&format!("/api/servers/{server_id}/channels")).await)
            .await;
    let channel_id = channels["channels"][0]["id"].as_str().unwrap();

    let config_response = app
        .put_json_with_bearer(
            &format!("/api/servers/{server_id}/configs"),
            &json!({
                "decisionMakingModel": "consensus",
                "agreementThreshold": 51,
                "disagreementsLimit": 2,
                "abstainsLimit": 2,
                "quorumEnabled": false,
                "votingTimeLimit": 0,
            }),
            &proposer,
        )
        .await;
    assert_eq!(config_response.status(), StatusCode::OK);

    let role_name = "Concurrent consensus role";
    let create_response = app
        .post_json_with_bearer(
            &format!("/api/servers/{server_id}/channels/{channel_id}/polls"),
            &json!({
                "body": "Create one role under concurrent ratification",
                "pollType": "proposal",
                "action": {
                    "actionType": "create-role",
                    "serverRole": {
                        "name": role_name,
                        "color": "#336699",
                        "members": [],
                        "permissions": [],
                    }
                }
            }),
            &proposer,
        )
        .await;
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body = json_body(create_response).await;
    let poll_id = create_body["poll"]["id"].as_str().unwrap();
    let vote_uri = format!(
        "/api/servers/{server_id}/channels/{channel_id}/polls/{poll_id}/votes"
    );

    let disagree_response = app
        .post_json_with_bearer(
            &vote_uri,
            &json!({ "voteType": "disagree" }),
            &proposer,
        )
        .await;
    assert_eq!(disagree_response.status(), StatusCode::OK);
    assert!(
        !json_body(disagree_response).await["vote"]["isRatifyingVote"]
            .as_bool()
            .unwrap()
    );

    let agree = json!({ "voteType": "agree" });
    let (response_a, response_b) = tokio::join!(
        app.post_json_with_bearer(&vote_uri, &agree, &voter_a),
        app.post_json_with_bearer(&vote_uri, &agree, &voter_b),
    );
    assert_eq!(response_a.status(), StatusCode::OK);
    assert_eq!(response_b.status(), StatusCode::OK);

    let body_a = json_body(response_a).await;
    let body_b = json_body(response_b).await;
    let ratifying_responses = [&body_a, &body_b]
        .into_iter()
        .filter(|body| body["vote"]["isRatifyingVote"] == true)
        .count();
    assert_eq!(ratifying_responses, 1);

    let poll_id = Uuid::parse_str(poll_id).unwrap();
    let poll = polls::Entity::find_by_id(poll_id)
        .one(app.database())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(poll.stage.as_str(), "ratified");

    let action = poll_actions::Entity::find()
        .filter(poll_actions::Column::PollId.eq(poll_id))
        .one(app.database())
        .await
        .unwrap()
        .unwrap();
    assert!(action.executed_at.is_some());

    let server_id = Uuid::parse_str(server_id).unwrap();
    let created_role_count = server_roles::Entity::find()
        .filter(server_roles::Column::ServerId.eq(server_id))
        .filter(server_roles::Column::Name.eq(role_name))
        .count(app.database())
        .await
        .unwrap();
    assert_eq!(created_role_count, 1);
}

async fn signup(app: &TestApp, email: &str, name: &str) -> String {
    let response = app
        .post_json(
            "/api/auth/signup",
            &json!({
                "email": email,
                "name": name,
                "password": "correct horse battery staple",
            }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let body: Value = json_body(response).await;
    body["access_token"].as_str().unwrap().to_owned()
}
