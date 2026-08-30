use axum::http::StatusCode;
use entity::{channel_members, messages, notifications, users};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::support::{json_body, TestApp};

#[tokio::test]
async fn channel_messages_notify_other_members_and_coalesce() {
    let app = TestApp::new().await;
    let context = Context::new(&app).await;

    context.post_message(&context.alice, "first").await;
    let bob_inbox = context.list(&context.bob).await;
    assert_eq!(bob_inbox.len(), 1);
    assert_eq!(bob_inbox[0]["kind"], "new_message");
    assert_eq!(bob_inbox[0]["unreadCount"], 1);
    assert_eq!(bob_inbox[0]["target"]["kind"], "message");
    assert_eq!(bob_inbox[0]["target"]["available"], true);
    assert_eq!(context.list(&context.carol).await.len(), 1);

    // The sender is never notified about their own message.
    assert!(context.list(&context.alice).await.is_empty());

    // A busy channel produces one inbox entry, not one per message.
    context.post_message(&context.alice, "second").await;
    context.post_message(&context.alice, "third").await;
    let bob_inbox = context.list(&context.bob).await;
    assert_eq!(bob_inbox.len(), 1);
    assert_eq!(bob_inbox[0]["unreadCount"], 3);
    assert_eq!(context.unread_count(&context.bob).await, 1);
}

#[tokio::test]
async fn thread_replies_notify_the_root_and_parent_authors() {
    let app = TestApp::new().await;
    let context = Context::new(&app).await;

    let root_id = context.post_message(&context.alice, "root").await;
    let reply_id = context.post_reply(&context.bob, &root_id, None).await;

    let alice_replies = context.of_kind(&context.alice, "message_reply").await;
    assert_eq!(alice_replies.len(), 1);
    assert_eq!(alice_replies[0]["target"]["messageId"], reply_id);
    assert_eq!(alice_replies[0]["target"]["threadRootId"], root_id);
    assert_eq!(alice_replies[0]["target"]["threadRootKind"], "message");
    assert_eq!(alice_replies[0]["actor"]["name"], "Bob");

    // Replying to Bob reaches both the thread root author and Bob.
    context
        .post_reply(&context.carol, &root_id, Some(&reply_id))
        .await;
    assert_eq!(
        context.of_kind(&context.alice, "message_reply").await.len(),
        2
    );
    assert_eq!(
        context.of_kind(&context.bob, "message_reply").await.len(),
        1
    );
    assert!(context
        .of_kind(&context.carol, "message_reply")
        .await
        .is_empty());
}

#[tokio::test]
async fn forum_replies_notify_the_post_author() {
    let app = TestApp::new().await;
    let context = Context::new(&app).await;
    let forum_channel_id = context.create_forum_channel().await;
    let post_id = context.create_forum_post(&forum_channel_id).await;

    let reply = app
        .post_json_with_bearer(
            &format!(
                "/api/servers/{}/channels/{forum_channel_id}/forum/posts/{post_id}/replies",
                context.server_id
            ),
            &json!({ "body": "A forum reply" }),
            &context.bob,
        )
        .await;
    assert_eq!(reply.status(), StatusCode::OK);

    let notifications = context.of_kind(&context.alice, "forum_reply").await;
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0]["target"]["forumPostId"], post_id);
    assert_eq!(notifications[0]["actor"]["name"], "Bob");
    assert!(context
        .of_kind(&context.bob, "forum_reply")
        .await
        .is_empty());
}

#[tokio::test]
async fn proposal_votes_and_outcomes_notify_the_author_and_voters() {
    let app = TestApp::new().await;
    let context = Context::new(&app).await;
    context.set_consensus_config().await;
    let poll_id = context.create_proposal().await;

    context.vote(&context.bob, &poll_id, "agree").await;
    let votes = context.of_kind(&context.alice, "proposal_vote").await;
    assert_eq!(votes.len(), 1);
    assert_eq!(votes[0]["voteType"], "agree");
    assert_eq!(votes[0]["target"]["kind"], "poll");
    assert_eq!(votes[0]["target"]["pollId"], poll_id);

    // The voter is not notified about their own vote.
    assert!(context
        .of_kind(&context.bob, "proposal_vote")
        .await
        .is_empty());

    // Bob's agreement ratifies the proposal, so the outcome reaches the author
    // and every voter exactly once, whichever path finalized it.
    for token in [&context.alice, &context.bob] {
        assert_eq!(
            context.of_kind(token, "proposal_ratified").await.len(),
            1,
            "expected exactly one ratification notification"
        );
    }
    assert!(context
        .of_kind(&context.carol, "proposal_ratified")
        .await
        .is_empty());

    // Reprocessing the same event must not create a second row.
    let existing = notifications::Entity::find()
        .filter(notifications::Column::Kind.eq("proposal_ratified"))
        .filter(
            notifications::Column::RecipientUserId
                .eq(context.user_id("Bob").await),
        )
        .one(app.database())
        .await
        .unwrap()
        .unwrap();
    let duplicate = notifications::ActiveModel {
        id: Set(Uuid::new_v4()),
        recipient_user_id: Set(existing.recipient_user_id),
        actor_user_id: Set(existing.actor_user_id),
        server_id: Set(existing.server_id),
        channel_id: Set(existing.channel_id),
        kind: Set(existing.kind),
        poll_id: Set(existing.poll_id),
        ..Default::default()
    };
    assert!(notifications::Entity::insert(duplicate)
        .exec(app.database())
        .await
        .is_err());
}

#[tokio::test]
async fn the_inbox_paginates_and_tracks_read_state() {
    let app = TestApp::new().await;
    let context = Context::new(&app).await;

    let root_id = context.post_message(&context.alice, "root").await;
    for index in 0..3 {
        context.post_reply(&context.bob, &root_id, None).await;
        let _ = index;
    }

    let first_page = json_body(
        app.get_with_bearer(
            &format!(
                "/api/servers/{}/notifications?limit=2",
                context.server_id
            ),
            &context.alice,
        )
        .await,
    )
    .await;
    assert_eq!(first_page["notifications"].as_array().unwrap().len(), 2);
    assert_eq!(first_page["hasMore"], true);

    let cursor = first_page["nextCursor"].as_str().unwrap().to_owned();
    let second_page = json_body(
        app.get_with_bearer(
            &format!(
                "/api/servers/{}/notifications?limit=2&before={}",
                context.server_id,
                urlencoding(&cursor)
            ),
            &context.alice,
        )
        .await,
    )
    .await;
    assert_eq!(second_page["notifications"].as_array().unwrap().len(), 1);
    assert_eq!(second_page["hasMore"], false);

    // Opening the inbox does not mark anything read.
    assert_eq!(context.unread_count(&context.alice).await, 3);

    let notification_id = first_page["notifications"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    for _ in 0..2 {
        let response = context
            .set_read(&context.alice, &notification_id, "read")
            .await;
        assert_eq!(response.status(), StatusCode::OK);
    }
    assert_eq!(context.unread_count(&context.alice).await, 2);

    for _ in 0..2 {
        let response = context
            .set_read(&context.alice, &notification_id, "unread")
            .await;
        assert_eq!(response.status(), StatusCode::OK);
    }
    assert_eq!(context.unread_count(&context.alice).await, 3);

    for _ in 0..2 {
        let response = app
            .put_json_with_bearer(
                &format!(
                    "/api/servers/{}/notifications/read-all",
                    context.server_id
                ),
                &json!({}),
                &context.alice,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);
    }
    assert_eq!(context.unread_count(&context.alice).await, 0);

    // Read notifications are retained until the user deletes them.
    assert_eq!(context.list(&context.alice).await.len(), 3);

    for _ in 0..2 {
        let response = app
            .delete_with_bearer(
                &format!(
                    "/api/servers/{}/notifications/{notification_id}",
                    context.server_id
                ),
                &context.alice,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);
    }
    assert_eq!(context.list(&context.alice).await.len(), 2);

    for _ in 0..2 {
        let response = app
            .delete_with_bearer(
                &format!("/api/servers/{}/notifications", context.server_id),
                &context.alice,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);
    }
    assert!(context.list(&context.alice).await.is_empty());
}

#[tokio::test]
async fn notifications_are_scoped_to_their_recipient() {
    let app = TestApp::new().await;
    let context = Context::new(&app).await;

    context.post_message(&context.alice, "hello").await;
    let bob_inbox = context.list(&context.bob).await;
    let notification_id = bob_inbox[0]["id"].as_str().unwrap().to_owned();

    // Carol has her own notification for the same message, never Bob's row.
    let carol_inbox = context.list(&context.carol).await;
    assert_ne!(carol_inbox[0]["id"], bob_inbox[0]["id"]);

    let read_response = context
        .set_read(&context.carol, &notification_id, "read")
        .await;
    assert_eq!(read_response.status(), StatusCode::NOT_FOUND);

    let delete_response = app
        .delete_with_bearer(
            &format!(
                "/api/servers/{}/notifications/{notification_id}",
                context.server_id
            ),
            &context.carol,
        )
        .await;
    assert_eq!(delete_response.status(), StatusCode::OK);
    assert_eq!(context.list(&context.bob).await.len(), 1);

    // A non-member of the server cannot reach the inbox at all.
    let other_server = json_body(
        app.post_json_with_bearer(
            "/api/servers",
            &json!({
                "name": "Other server",
                "slug": "other-server",
                "description": "Elsewhere",
            }),
            &context.alice,
        )
        .await,
    )
    .await;
    let other_server_id =
        other_server["server"]["id"].as_str().unwrap().to_owned();
    let forbidden = app
        .get_with_bearer(
            &format!("/api/servers/{other_server_id}/notifications"),
            &context.bob,
        )
        .await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn targets_reflect_what_the_recipient_can_still_read() {
    let app = TestApp::new().await;
    let context = Context::new(&app).await;

    let message_id = context.post_message(&context.alice, "hello").await;
    assert_eq!(context.list(&context.bob).await.len(), 1);

    // Losing access to the channel leaves a safe unavailable target rather
    // than leaking the message.
    channel_members::Entity::delete_many()
        .filter(
            channel_members::Column::ChannelId
                .eq(Uuid::parse_str(&context.channel_id).unwrap()),
        )
        .filter(
            channel_members::Column::UserId.eq(context.user_id("Bob").await),
        )
        .exec(app.database())
        .await
        .unwrap();
    let bob_inbox = context.list(&context.bob).await;
    assert_eq!(bob_inbox[0]["target"]["kind"], "unavailable");
    assert_eq!(bob_inbox[0]["target"]["available"], false);
    assert!(bob_inbox[0]["target"]["messageId"].is_null());

    // A deleted target takes its notifications with it.
    messages::Entity::delete_by_id(Uuid::parse_str(&message_id).unwrap())
        .exec(app.database())
        .await
        .unwrap();
    assert_eq!(
        notifications::Entity::find()
            .count(app.database())
            .await
            .unwrap(),
        0
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

    async fn post_message(&self, token: &str, body: &str) -> String {
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

        json_body(response).await["message"]["id"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    async fn post_reply(
        &self,
        token: &str,
        root_message_id: &str,
        parent_message_id: Option<&str>,
    ) -> String {
        let response = self
            .app
            .post_json_with_bearer(
                &format!(
                    "/api/servers/{}/channels/{}/messages/{root_message_id}/replies",
                    self.server_id, self.channel_id
                ),
                &json!({ "body": "A reply", "parentMessageId": parent_message_id }),
                token,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        json_body(response).await["message"]["id"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    async fn create_forum_channel(&self) -> String {
        let response = self
            .app
            .post_json_with_bearer(
                &format!("/api/servers/{}/channels", self.server_id),
                &json!({ "name": "forum-channel", "channelType": "forum" }),
                &self.alice,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        json_body(response).await["channel"]["id"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    async fn create_forum_post(&self, forum_channel_id: &str) -> String {
        let response = self
            .app
            .post_json_with_bearer(
                &format!(
                    "/api/servers/{}/channels/{forum_channel_id}/forum/posts",
                    self.server_id
                ),
                &json!({ "title": "A post", "body": "Post body" }),
                &self.alice,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        json_body(response).await["post"]["id"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    async fn set_consensus_config(&self) {
        let response = self
            .app
            .put_json_with_bearer(
                &format!("/api/servers/{}/configs", self.server_id),
                &json!({
                    "decisionMakingModel": "consensus",
                    "agreementThreshold": 51,
                    "disagreementsLimit": 2,
                    "abstainsLimit": 2,
                    "quorumEnabled": false,
                    "votingTimeLimit": 0,
                }),
                &self.alice,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    async fn create_proposal(&self) -> String {
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
                &self.alice,
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
        let status = response.status();
        assert_eq!(status, StatusCode::OK, "{:?}", json_body(response).await);
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

    async fn unread_count(&self, token: &str) -> u64 {
        let response = self
            .app
            .get_with_bearer(
                &format!(
                    "/api/servers/{}/notifications/unread-count",
                    self.server_id
                ),
                token,
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);

        json_body(response).await["unreadCount"].as_u64().unwrap()
    }

    async fn set_read(
        &self,
        token: &str,
        notification_id: &str,
        state: &str,
    ) -> axum::http::Response<axum::body::Body> {
        self.app
            .put_json_with_bearer(
                &format!(
                    "/api/servers/{}/notifications/{notification_id}/{state}",
                    self.server_id
                ),
                &json!({}),
                token,
            )
            .await
    }

    async fn user_id(&self, name: &str) -> Uuid {
        users::Entity::find()
            .filter(users::Column::Name.eq(name))
            .one(self.app.database())
            .await
            .unwrap()
            .unwrap()
            .id
    }
}

fn urlencoding(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace(':', "%3A")
        .replace('+', "%2B")
        .replace('|', "%7C")
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
