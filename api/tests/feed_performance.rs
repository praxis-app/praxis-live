//! Guards the channel feed against N+1 query growth. Shaping a feed page may
//! cost a fixed number of queries, but that cost must not grow with the number
//! of decisions the page happens to hold.

#[path = "support/mod.rs"]
#[allow(dead_code)]
mod support;

use axum::http::StatusCode;
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    OnceLock,
};
use tracing_subscriber::{layer::SubscriberExt, Layer};

use crate::support::{json_body, TestApp};

/// Decisions on the smaller page. The larger page holds every seeded decision.
const SMALL_PAGE: usize = 4;
const SEEDED_DECISIONS: usize = 20;
const LARGE_PAGE: usize = SEEDED_DECISIONS;

/// Queries the larger page may cost beyond the smaller one. Batched shaping
/// adds a fixed number of statements per page, so widening a page by 16
/// decisions should cost roughly nothing extra.
const SCALING_ALLOWANCE: usize = 10;

static EXECUTED_QUERIES: AtomicUsize = AtomicUsize::new(0);
static QUERY_COUNTER: OnceLock<()> = OnceLock::new();

/// Counts every statement sqlx executes. sqlx emits one `sqlx::query` event per
/// statement, so this is an exact count of database round trips rather than a
/// timing measurement.
struct QueryCounter;

impl<S: tracing::Subscriber> Layer<S> for QueryCounter {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _context: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if event.metadata().target() == "sqlx::query" {
            EXECUTED_QUERIES.fetch_add(1, Ordering::SeqCst);
        }
    }
}

fn install_query_counter() {
    QUERY_COUNTER.get_or_init(|| {
        let subscriber = tracing_subscriber::registry().with(QueryCounter);
        tracing::subscriber::set_global_default(subscriber)
            .expect("expected the query counter to install");
    });
}

fn executed_queries() -> usize {
    EXECUTED_QUERIES.load(Ordering::SeqCst)
}

#[tokio::test]
async fn channel_feed_queries_do_not_scale_with_decision_count() {
    install_query_counter();

    let app = TestApp::new().await;
    let author = signup(&app, "feed-perf@example.com", "Feed Perf").await;
    let default_server = json_body(app.get("/api/servers/default").await).await;
    let server_id = default_server["server"]["id"].as_str().unwrap().to_owned();
    let channels =
        json_body(app.get(&format!("/api/servers/{server_id}/channels")).await)
            .await;
    let channel_id = channels["channels"][0]["id"].as_str().unwrap().to_owned();

    seed_decisions(&app, &server_id, &channel_id, &author).await;

    let feed_uri = |limit: usize| {
        format!(
            "/api/servers/{server_id}/channels/{channel_id}/feed?limit={limit}"
        )
    };

    let (small_page_items, small_page_queries) =
        measure_feed(&app, &feed_uri(SMALL_PAGE), &author).await;
    let (large_page_items, large_page_queries) =
        measure_feed(&app, &feed_uri(LARGE_PAGE), &author).await;

    assert_eq!(small_page_items, SMALL_PAGE);
    assert_eq!(large_page_items, LARGE_PAGE);
    assert!(
        large_page_queries <= small_page_queries + SCALING_ALLOWANCE,
        "channel feed queries scale with the number of decisions on a page: \
         {small_page_queries} queries for {SMALL_PAGE} decisions, \
         {large_page_queries} queries for {LARGE_PAGE}. A page should cost a \
         fixed number of queries regardless of how many decisions it holds."
    );
}

/// Returns the number of items the page returned and the queries it cost.
async fn measure_feed(app: &TestApp, uri: &str, token: &str) -> (usize, usize) {
    let before = executed_queries();
    let response = app.get_with_bearer(uri, token).await;
    assert_eq!(response.status(), StatusCode::OK);
    let queries = executed_queries() - before;

    let body = json_body(response).await;
    (body["feed"].as_array().unwrap().len(), queries)
}

/// Seeds a mix of decision shapes so the measurement covers the expensive
/// proposal action paths, not just bare polls.
async fn seed_decisions(
    app: &TestApp,
    server_id: &str,
    channel_id: &str,
    token: &str,
) {
    let polls_uri =
        format!("/api/servers/{server_id}/channels/{channel_id}/polls");

    for index in 0..SEEDED_DECISIONS {
        let payload = match index % 4 {
            0 => json!({
                "body": format!("Feed perf poll {index}"),
                "pollType": "poll",
                "options": ["Yes", "No"],
                "multipleChoice": false,
            }),
            1 => json!({
                "body": format!("Feed perf general proposal {index}"),
                "pollType": "proposal",
                "action": { "actionType": "general" },
            }),
            2 => json!({
                "body": format!("Feed perf settings proposal {index}"),
                "pollType": "proposal",
                "action": {
                    "actionType": "change-settings",
                    "serverConfig": { "anonymousUsersEnabled": true },
                },
            }),
            _ => json!({
                "body": format!("Feed perf role proposal {index}"),
                "pollType": "proposal",
                "action": {
                    "actionType": "create-role",
                    "serverRole": {
                        "name": format!("feed-perf-role-{index}"),
                        "color": "#336699",
                        "members": [],
                        "permissions": [],
                    },
                },
            }),
        };

        let response =
            app.post_json_with_bearer(&polls_uri, &payload, token).await;
        assert_eq!(response.status(), StatusCode::OK);
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

    let body: Value = json_body(response).await;
    body["access_token"].as_str().unwrap().to_owned()
}
