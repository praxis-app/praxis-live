use axum::http::StatusCode;
use serde_json::{json, Value};

use crate::support::{json_body, TestApp};

#[tokio::test]
async fn config_defaults_to_every_notification_enabled() {
    let app = TestApp::new().await;
    let context = Context::new(&app).await;

    let config = context.get_config(&context.alice).await;
    assert_eq!(config["messageNotificationsEnabled"], true);
    assert_eq!(config["replyNotificationsEnabled"], true);
    assert_eq!(config["proposalNotificationsEnabled"], true);
    assert_eq!(config["roleNotificationsEnabled"], true);
}

#[tokio::test]
async fn a_partial_update_leaves_the_other_settings_alone() {
    let app = TestApp::new().await;
    let context = Context::new(&app).await;

    let updated = context
        .update_config(
            &context.alice,
            &json!({ "replyNotificationsEnabled": false }),
        )
        .await;
    assert_eq!(updated["replyNotificationsEnabled"], false);
    assert_eq!(updated["messageNotificationsEnabled"], true);

    // The change is stored, not just echoed back.
    let config = context.get_config(&context.alice).await;
    assert_eq!(config["replyNotificationsEnabled"], false);
    assert_eq!(config["proposalNotificationsEnabled"], true);
    assert_eq!(config["roleNotificationsEnabled"], true);
}

#[tokio::test]
async fn config_requires_authentication() {
    let app = TestApp::new().await;
    let _ = Context::new(&app).await;

    let response = app.get("/api/users/me/configs").await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn unknown_settings_are_rejected() {
    let app = TestApp::new().await;
    let context = Context::new(&app).await;

    let response = app
        .put_json_with_bearer(
            "/api/users/me/configs",
            &json!({ "somethingElseEnabled": false }),
            &context.alice,
        )
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn disabling_message_notifications_only_silences_that_user() {
    let app = TestApp::new().await;
    let context = Context::new(&app).await;

    context
        .update_config(
            &context.bob,
            &json!({ "messageNotificationsEnabled": false }),
        )
        .await;
    context.post_message(&context.alice, "a new message").await;

    assert!(context.list(&context.bob).await.is_empty());
    assert_eq!(context.list(&context.carol).await.len(), 1);
}

#[tokio::test]
async fn re_enabling_message_notifications_restores_delivery() {
    let app = TestApp::new().await;
    let context = Context::new(&app).await;

    context
        .update_config(
            &context.bob,
            &json!({ "messageNotificationsEnabled": false }),
        )
        .await;
    context.post_message(&context.alice, "silenced").await;
    assert!(context.list(&context.bob).await.is_empty());

    context
        .update_config(
            &context.bob,
            &json!({ "messageNotificationsEnabled": true }),
        )
        .await;
    context.post_message(&context.alice, "heard").await;
    assert_eq!(context.list(&context.bob).await.len(), 1);
}

#[tokio::test]
async fn disabling_proposal_notifications_keeps_message_notifications() {
    let app = TestApp::new().await;
    let context = Context::new(&app).await;

    context
        .update_config(
            &context.alice,
            &json!({ "proposalNotificationsEnabled": false }),
        )
        .await;
    let poll_id = context.create_proposal(&context.alice).await;
    context.vote(&context.bob, &poll_id, "agree").await;

    assert!(context
        .of_kind(&context.alice, "proposal_vote")
        .await
        .is_empty());

    context.post_message(&context.bob, "still notified").await;
    assert_eq!(
        context.of_kind(&context.alice, "new_message").await.len(),
        1
    );
}

struct Context<'a> {
    app: &'a TestApp,
    server_id: String,
    channel_id: String,
    alice: String,
    bob: String,
    carol: String,
}

impl<'a> Context<'a> {
    async fn new(app: &'a TestApp) -> Context<'a> {
        let alice = signup(app, "alice@example.com", "Alice").await;
        let bob = signup(app, "bob@example.com", "Bob").await;
        let carol = signup(app, "carol@example.com", "Carol").await;
        let server = json_body(app.get("/api/servers/default").await).await;
        let server_id = server["server"]["id"].as_str().unwrap().to_owned();
        let channels = json_body(
            app.get(&format!("/api/servers/{server_id}/channels")).await,
        )
        .await;
        let channel_id =
            channels["channels"][0]["id"].as_str().unwrap().to_owned();

        Context {
            app,
            server_id,
            channel_id,
            alice,
            bob,
            carol,
        }
    }

    async fn get_config(&self, token: &str) -> Value {
        let response = self
            .app
            .get_with_bearer("/api/users/me/configs", token)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        json_body(response).await["userConfig"].clone()
    }

    async fn update_config(&self, token: &str, request: &Value) -> Value {
        let response = self
            .app
            .put_json_with_bearer("/api/users/me/configs", request, token)
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        json_body(response).await["userConfig"].clone()
    }

    async fn post_message(&self, token: &str, body: &str) {
        let response = self
            .app
            .post_json_with_bearer(
                &format!(
                    "/api/servers/{}/channels/{}/messages",
                    self.server_id, self.channel_id
                ),
                &json!({ "body": body }),
                token,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    async fn create_proposal(&self, token: &str) -> String {
        let response = self
            .app
            .post_json_with_bearer(
                &format!(
                    "/api/servers/{}/channels/{}/polls",
                    self.server_id, self.channel_id
                ),
                &json!({
                    "body": "A proposal worth notifying about",
                    "pollType": "proposal",
                    "action": { "actionType": "general" },
                }),
                token,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        json_body(response).await["poll"]["id"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    async fn vote(&self, token: &str, poll_id: &str, vote_type: &str) {
        let response = self
            .app
            .post_json_with_bearer(
                &format!(
                    "/api/servers/{}/channels/{}/polls/{poll_id}/votes",
                    self.server_id, self.channel_id
                ),
                &json!({ "voteType": vote_type }),
                token,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    async fn list(&self, token: &str) -> Vec<Value> {
        let response = self
            .app
            .get_with_bearer(
                &format!("/api/servers/{}/notifications", self.server_id),
                token,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        json_body(response).await["notifications"]
            .as_array()
            .unwrap()
            .to_owned()
    }

    async fn of_kind(&self, token: &str, kind: &str) -> Vec<Value> {
        self.list(token)
            .await
            .into_iter()
            .filter(|notification| notification["kind"] == kind)
            .collect()
    }
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

    json_body(response).await["access_token"]
        .as_str()
        .unwrap()
        .to_owned()
}
