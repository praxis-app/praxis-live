use axum::http::StatusCode;
use entity::{poll_actions, polls, server_roles};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::support::{json_body, TestApp};

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
