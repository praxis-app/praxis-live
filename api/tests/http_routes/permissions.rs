//! Authorization coverage for permission-gated endpoints.
//!
//! Each test asserts the permission boundary the endpoint is meant to enforce,
//! not the behavior it currently has. Tests that gate a `manage` permission
//! also assert the positive case, so that a check which rejects everyone is
//! not mistaken for a working one.

use axum::http::StatusCode;
use entity::{channels, server_roles};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::json;
use uuid::Uuid;

use crate::support::{json_body, TestApp};

struct TestUser {
    token: String,
    user_id: String,
}

#[tokio::test]
async fn members_without_instance_permissions_cannot_create_servers() {
    let app = TestApp::new().await;
    let _admin = signup(&app, "admin@example.com", "Admin Example").await;
    let member = signup(&app, "member@example.com", "Member Example").await;

    let response = app
        .post_json_with_bearer(
            "/api/servers",
            &server_payload("Member server", "member-server", false),
            &member.token,
        )
        .await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn server_admins_cannot_reassign_the_instance_default_server() {
    let app = TestApp::new().await;
    let admin = signup(&app, "admin@example.com", "Admin Example").await;
    let member = signup(&app, "member@example.com", "Member Example").await;
    let original_default_id = default_server_id(&app).await;
    let other_server_id = create_server(&app, &admin, "Other", "other").await;
    grant_server_admin(&app, &admin, &other_server_id, &member).await;

    let response = app
        .put_json_with_bearer(
            &format!("/api/servers/{other_server_id}"),
            &server_payload("Other", "other", true),
            &member.token,
        )
        .await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(default_server_id(&app).await, original_default_id);
}

#[tokio::test]
async fn server_admins_cannot_delete_a_server() {
    let app = TestApp::new().await;
    let admin = signup(&app, "admin@example.com", "Admin Example").await;
    let member = signup(&app, "member@example.com", "Member Example").await;
    let other_server_id = create_server(&app, &admin, "Other", "other").await;
    grant_server_admin(&app, &admin, &other_server_id, &member).await;

    let response = app
        .delete_with_bearer(
            &format!("/api/servers/{other_server_id}"),
            &member.token,
        )
        .await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn logged_out_users_cannot_read_non_default_server_channels() {
    let app = TestApp::new().await;
    let admin = signup(&app, "admin@example.com", "Admin Example").await;
    let server_id = create_server(&app, &admin, "Private", "private").await;
    let channel_id = general_channel_id(&app, &server_id).await;

    let list_response =
        app.get(&format!("/api/servers/{server_id}/channels")).await;
    assert_eq!(list_response.status(), StatusCode::FORBIDDEN);

    let detail_response = app
        .get(&format!("/api/servers/{server_id}/channels/{channel_id}"))
        .await;
    assert_eq!(detail_response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn proposals_cannot_target_a_role_in_another_server() {
    let app = TestApp::new().await;
    let admin = signup(&app, "admin@example.com", "Admin Example").await;
    let member = signup(&app, "member@example.com", "Member Example").await;
    let default_server_id = default_server_id(&app).await;
    let channel_id = general_channel_id(&app, &default_server_id).await;
    let other_server_id = create_server(&app, &admin, "Other", "other").await;
    let foreign_role_id = admin_role_id(&app, &other_server_id).await;

    let response = app
        .post_json_with_bearer(
            &format!(
                "/api/servers/{default_server_id}/channels/{channel_id}/polls"
            ),
            &json!({
                "body": "Grant myself the other server's admin role",
                "pollType": "proposal",
                "action": {
                    "actionType": "change-role",
                    "serverRole": {
                        "serverRoleToUpdateId": foreign_role_id,
                        "members": [
                            { "userId": member.user_id, "changeType": "add" }
                        ],
                    }
                }
            }),
            &member.token,
        )
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn only_channel_managers_can_create_channels() {
    let app = TestApp::new().await;
    let admin = signup(&app, "admin@example.com", "Admin Example").await;
    let member = signup(&app, "member@example.com", "Member Example").await;
    let server_id = default_server_id(&app).await;
    let uri = format!("/api/servers/{server_id}/channels");

    let member_response = app
        .post_json_with_bearer(
            &uri,
            &json!({ "name": "member-channel" }),
            &member.token,
        )
        .await;
    assert_eq!(member_response.status(), StatusCode::FORBIDDEN);

    let admin_response = app
        .post_json_with_bearer(
            &uri,
            &json!({ "name": "admin-channel" }),
            &admin.token,
        )
        .await;
    assert_eq!(admin_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn only_invite_managers_can_read_server_invites() {
    let app = TestApp::new().await;
    let admin = signup(&app, "admin@example.com", "Admin Example").await;
    let member = signup(&app, "member@example.com", "Member Example").await;
    let server_id = default_server_id(&app).await;
    let uri = format!("/api/servers/{server_id}/invites");

    let member_response = app.get_with_bearer(&uri, &member.token).await;
    assert_eq!(member_response.status(), StatusCode::FORBIDDEN);

    let admin_response = app.get_with_bearer(&uri, &admin.token).await;
    assert_eq!(admin_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn only_role_managers_can_update_server_role_permissions() {
    let app = TestApp::new().await;
    let admin = signup(&app, "admin@example.com", "Admin Example").await;
    let member = signup(&app, "member@example.com", "Member Example").await;
    let server_id = default_server_id(&app).await;

    // Target a purpose-built role so that rewriting its permissions cannot
    // strip the admin's own standing partway through the test.
    let create_response = app
        .post_json_with_bearer(
            &format!("/api/servers/{server_id}/roles"),
            &json!({ "name": "Moderators", "color": "#336699" }),
            &admin.token,
        )
        .await;
    assert_eq!(create_response.status(), StatusCode::OK);
    let role_id = json_body(create_response).await["serverRole"]["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let uri = format!("/api/servers/{server_id}/roles/{role_id}/permissions");
    let payload = json!({
        "permissions": [{ "subject": "Channel", "action": ["manage"] }],
    });

    let member_response = app
        .put_json_with_bearer(&uri, &payload, &member.token)
        .await;
    assert_eq!(member_response.status(), StatusCode::FORBIDDEN);

    let admin_response =
        app.put_json_with_bearer(&uri, &payload, &admin.token).await;
    assert_eq!(admin_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn only_instance_role_managers_can_create_instance_roles() {
    let app = TestApp::new().await;
    let admin = signup(&app, "admin@example.com", "Admin Example").await;
    let member = signup(&app, "member@example.com", "Member Example").await;

    let member_response = app
        .post_json_with_bearer(
            "/api/instance/roles",
            &json!({ "name": "Member role", "color": "#336699" }),
            &member.token,
        )
        .await;
    assert_eq!(member_response.status(), StatusCode::FORBIDDEN);

    let admin_response = app
        .post_json_with_bearer(
            "/api/instance/roles",
            &json!({ "name": "Admin role", "color": "#336699" }),
            &admin.token,
        )
        .await;
    assert_eq!(admin_response.status(), StatusCode::OK);
}

fn server_payload(
    name: &str,
    slug: &str,
    is_default_server: bool,
) -> serde_json::Value {
    json!({
        "name": name,
        "slug": slug,
        "description": null,
        "isDefaultServer": is_default_server,
    })
}

async fn signup(app: &TestApp, email: &str, name: &str) -> TestUser {
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

    let body = json_body(response).await;
    TestUser {
        token: body["access_token"].as_str().unwrap().to_owned(),
        user_id: body["user"]["id"].as_str().unwrap().to_owned(),
    }
}

async fn default_server_id(app: &TestApp) -> String {
    let response = app.get("/api/servers/default").await;
    assert_eq!(response.status(), StatusCode::OK);

    json_body(response).await["server"]["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

async fn create_server(
    app: &TestApp,
    owner: &TestUser,
    name: &str,
    slug: &str,
) -> String {
    let response = app
        .post_json_with_bearer(
            "/api/servers",
            &server_payload(name, slug, false),
            &owner.token,
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    json_body(response).await["server"]["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

// Grants the server's admin role, which carries `ServerConfig: manage` but no
// instance-level permissions.
async fn grant_server_admin(
    app: &TestApp,
    granter: &TestUser,
    server_id: &str,
    user: &TestUser,
) {
    let members_response = app
        .post_json_with_bearer(
            &format!("/api/servers/{server_id}/members"),
            &json!({ "userIds": [user.user_id] }),
            &granter.token,
        )
        .await;
    assert_eq!(members_response.status(), StatusCode::OK);

    let role_id = admin_role_id(app, server_id).await;
    let role_members_response = app
        .post_json_with_bearer(
            &format!("/api/servers/{server_id}/roles/{role_id}/members"),
            &json!({ "userIds": [user.user_id] }),
            &granter.token,
        )
        .await;
    assert_eq!(role_members_response.status(), StatusCode::OK);
}

// Reads role and channel ids straight from the database so that setup does not
// depend on the read endpoints these tests are asserting against.
async fn admin_role_id(app: &TestApp, server_id: &str) -> String {
    let server_id = Uuid::parse_str(server_id).unwrap();
    let role = server_roles::Entity::find()
        .filter(server_roles::Column::ServerId.eq(server_id))
        .filter(server_roles::Column::Name.eq("admin"))
        .one(app.database())
        .await
        .unwrap()
        .expect("expected an admin role for the server");

    role.id.to_string()
}

async fn general_channel_id(app: &TestApp, server_id: &str) -> String {
    let server_id = Uuid::parse_str(server_id).unwrap();
    let channel = channels::Entity::find()
        .filter(channels::Column::ServerId.eq(server_id))
        .order_by_asc(channels::Column::SortOrder)
        .one(app.database())
        .await
        .unwrap()
        .expect("expected a channel in the server");

    channel.id.to_string()
}
