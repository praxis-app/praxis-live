use axum::http::StatusCode;
use chrono::{Duration, Utc};
use entity::{poll_actions, poll_configs, polls, server_configs, server_roles};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, Set,
};
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

#[tokio::test]
async fn consent_rejects_configurations_that_leave_voting_time_unlimited() {
    let app = TestApp::new().await;
    let admin =
        signup(&app, "consent-admin@example.com", "Consent Admin").await;
    let server_id = default_server_id(&app).await;

    let baseline = set_server_config(
        &app,
        &server_id,
        &admin,
        &json!({ "decisionMakingModel": "consensus", "votingTimeLimit": 0 }),
    )
    .await;
    assert_eq!(baseline.status(), StatusCode::OK);

    let partial = set_server_config(
        &app,
        &server_id,
        &admin,
        &json!({ "decisionMakingModel": "consent" }),
    )
    .await;
    assert_eq!(partial.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let complete = set_server_config(
        &app,
        &server_id,
        &admin,
        &json!({ "decisionMakingModel": "consent", "votingTimeLimit": 60 }),
    )
    .await;
    assert_eq!(complete.status(), StatusCode::OK);

    let unlimited = set_server_config(
        &app,
        &server_id,
        &admin,
        &json!({ "votingTimeLimit": 0 }),
    )
    .await;
    assert_eq!(unlimited.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let stored = server_config(&app, &server_id).await;
    assert_eq!(stored.voting_time_limit, 60);
}

#[tokio::test]
async fn consent_proposals_cannot_be_created_without_a_deadline() {
    let app = TestApp::new().await;
    let proposer =
        signup(&app, "consent-proposer@example.com", "Consent Proposer").await;
    let server_id = default_server_id(&app).await;
    let channel_id = first_channel_id(&app, &server_id).await;

    let response = set_server_config(
        &app,
        &server_id,
        &proposer,
        &json!({ "decisionMakingModel": "consent", "votingTimeLimit": 60 }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    let mut active = server_config(&app, &server_id).await.into_active_model();
    active.voting_time_limit = Set(0);
    active.update(app.database()).await.unwrap();

    let response = create_role_proposal(
        &app,
        &server_id,
        &channel_id,
        &proposer,
        "Unlimited consent role",
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn consent_proposals_keep_their_rules_when_the_server_config_changes() {
    let app = TestApp::new().await;
    let proposer =
        signup(&app, "snapshot-proposer@example.com", "Snapshot Proposer")
            .await;
    let server_id = default_server_id(&app).await;
    let channel_id = first_channel_id(&app, &server_id).await;

    let response = set_server_config(
        &app,
        &server_id,
        &proposer,
        &json!({
            "decisionMakingModel": "consent",
            "votingTimeLimit": 60,
            "disagreementsLimit": 1,
            "abstainsLimit": 1,
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = create_role_proposal(
        &app,
        &server_id,
        &channel_id,
        &proposer,
        "Snapshotted consent role",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let poll_id = poll_id_of(response).await;
    let snapshot = poll_config(&app, poll_id).await;

    let response = set_server_config(
        &app,
        &server_id,
        &proposer,
        &json!({
            "decisionMakingModel": "consensus",
            "votingTimeLimit": 0,
            "disagreementsLimit": 5,
            "abstainsLimit": 5,
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(poll_config(&app, poll_id).await, snapshot);
    assert_eq!(snapshot.decision_making_model.unwrap().as_str(), "consent");
    assert_eq!(snapshot.disagreements_limit, Some(1));
    assert_eq!(snapshot.abstains_limit, Some(1));
    assert!(snapshot.closing_at.is_some());
}

#[tokio::test]
async fn a_blocked_consent_proposal_closes_at_its_deadline() {
    let app = TestApp::new().await;
    let proposer =
        signup(&app, "blocked-proposer@example.com", "Blocked Proposer").await;
    let blocker = signup(&app, "blocker@example.com", "Blocker").await;
    let server_id = default_server_id(&app).await;
    let channel_id = first_channel_id(&app, &server_id).await;
    configure_consent(&app, &server_id, &proposer).await;

    let role_name = "Blocked consent role";
    let response = create_role_proposal(
        &app,
        &server_id,
        &channel_id,
        &proposer,
        role_name,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let poll_id = poll_id_of(response).await;

    cast_vote(&app, &server_id, &channel_id, poll_id, &proposer, "agree").await;
    cast_vote(&app, &server_id, &channel_id, poll_id, &blocker, "block").await;
    assert_eq!(poll(&app, poll_id).await.stage.as_str(), "voting");

    expire_poll_deadline(&app, poll_id).await;
    let poll = wait_for_finalized_poll(&app, poll_id).await;

    assert_eq!(poll.stage.as_str(), "closed");
    assert!(poll_action(&app, poll_id).await.executed_at.is_none());
    assert_eq!(role_count(&app, &server_id, role_name).await, 0);
}

#[tokio::test]
async fn a_consent_proposal_ratifies_at_its_deadline_and_executes_once() {
    let app = TestApp::new().await;
    let proposer = signup(
        &app,
        "consenting-proposer@example.com",
        "Consenting Proposer",
    )
    .await;
    let dissenter = signup(&app, "dissenter@example.com", "Dissenter").await;
    let abstainer = signup(&app, "abstainer@example.com", "Abstainer").await;
    let server_id = default_server_id(&app).await;
    let channel_id = first_channel_id(&app, &server_id).await;
    configure_consent(&app, &server_id, &proposer).await;

    let role_name = "Consented role";
    let response = create_role_proposal(
        &app,
        &server_id,
        &channel_id,
        &proposer,
        role_name,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let poll_id = poll_id_of(response).await;

    // One disagreement and one abstention, each exactly at its limit.
    cast_vote(
        &app,
        &server_id,
        &channel_id,
        poll_id,
        &dissenter,
        "disagree",
    )
    .await;
    cast_vote(
        &app,
        &server_id,
        &channel_id,
        poll_id,
        &abstainer,
        "abstain",
    )
    .await;

    let poll = poll(&app, poll_id).await;
    assert_eq!(poll.stage.as_str(), "voting");
    assert!(poll_action(&app, poll_id).await.executed_at.is_none());

    expire_poll_deadline(&app, poll_id).await;
    let poll = wait_for_finalized_poll(&app, poll_id).await;

    assert_eq!(poll.stage.as_str(), "ratified");
    assert!(poll_action(&app, poll_id).await.executed_at.is_some());
    assert_eq!(role_count(&app, &server_id, role_name).await, 1);
}

/// Quorum is deliberately enabled and unreachable: consent must ignore it.
async fn configure_consent(app: &TestApp, server_id: &str, token: &str) {
    let response = set_server_config(
        app,
        server_id,
        token,
        &json!({
            "decisionMakingModel": "consent",
            "votingTimeLimit": 60,
            "disagreementsLimit": 1,
            "abstainsLimit": 1,
            "quorumEnabled": true,
            "quorumThreshold": 100,
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
}

async fn set_server_config(
    app: &TestApp,
    server_id: &str,
    token: &str,
    payload: &Value,
) -> axum::response::Response {
    app.put_json_with_bearer(
        &format!("/api/servers/{server_id}/configs"),
        payload,
        token,
    )
    .await
}

async fn create_role_proposal(
    app: &TestApp,
    server_id: &str,
    channel_id: &str,
    token: &str,
    role_name: &str,
) -> axum::response::Response {
    app.post_json_with_bearer(
        &format!("/api/servers/{server_id}/channels/{channel_id}/polls"),
        &json!({
            "body": format!("Create the {role_name} role"),
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
        token,
    )
    .await
}

async fn cast_vote(
    app: &TestApp,
    server_id: &str,
    channel_id: &str,
    poll_id: Uuid,
    token: &str,
    vote_type: &str,
) {
    let response = app
        .post_json_with_bearer(
            &format!(
                "/api/servers/{server_id}/channels/{channel_id}/polls/{poll_id}/votes"
            ),
            &json!({ "voteType": vote_type }),
            token,
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        !json_body(response).await["vote"]["isRatifyingVote"]
            .as_bool()
            .unwrap(),
        "consent proposals must never finalize before their deadline"
    );
}

async fn expire_poll_deadline(app: &TestApp, poll_id: Uuid) {
    let mut active = poll_config(app, poll_id).await.into_active_model();
    active.closing_at =
        Set(Some(Utc::now().fixed_offset() - Duration::seconds(1)));
    active.update(app.database()).await.unwrap();
}

async fn wait_for_finalized_poll(app: &TestApp, poll_id: Uuid) -> polls::Model {
    for _ in 0..60 {
        let poll = poll(app, poll_id).await;
        if poll.stage.as_str() != "voting" {
            return poll;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!("expected the proposal synchronizer to finalize the proposal");
}

async fn poll(app: &TestApp, poll_id: Uuid) -> polls::Model {
    polls::Entity::find_by_id(poll_id)
        .one(app.database())
        .await
        .unwrap()
        .unwrap()
}

async fn poll_config(app: &TestApp, poll_id: Uuid) -> poll_configs::Model {
    poll_configs::Entity::find()
        .filter(poll_configs::Column::PollId.eq(poll_id))
        .one(app.database())
        .await
        .unwrap()
        .unwrap()
}

async fn poll_action(app: &TestApp, poll_id: Uuid) -> poll_actions::Model {
    poll_actions::Entity::find()
        .filter(poll_actions::Column::PollId.eq(poll_id))
        .one(app.database())
        .await
        .unwrap()
        .unwrap()
}

async fn server_config(
    app: &TestApp,
    server_id: &str,
) -> server_configs::Model {
    server_configs::Entity::find()
        .filter(
            server_configs::Column::ServerId
                .eq(Uuid::parse_str(server_id).unwrap()),
        )
        .one(app.database())
        .await
        .unwrap()
        .unwrap()
}

async fn role_count(app: &TestApp, server_id: &str, role_name: &str) -> u64 {
    server_roles::Entity::find()
        .filter(
            server_roles::Column::ServerId
                .eq(Uuid::parse_str(server_id).unwrap()),
        )
        .filter(server_roles::Column::Name.eq(role_name))
        .count(app.database())
        .await
        .unwrap()
}

async fn poll_id_of(response: axum::response::Response) -> Uuid {
    let body = json_body(response).await;
    Uuid::parse_str(body["poll"]["id"].as_str().unwrap()).unwrap()
}

async fn default_server_id(app: &TestApp) -> String {
    let body = json_body(app.get("/api/servers/default").await).await;
    body["server"]["id"].as_str().unwrap().to_owned()
}

async fn first_channel_id(app: &TestApp, server_id: &str) -> String {
    let body =
        json_body(app.get(&format!("/api/servers/{server_id}/channels")).await)
            .await;
    body["channels"][0]["id"].as_str().unwrap().to_owned()
}
