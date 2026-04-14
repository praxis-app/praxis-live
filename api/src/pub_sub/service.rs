use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use dashmap::DashMap;
use redis::AsyncCommands;
use sea_orm::prelude::Uuid;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, env, sync::Arc};
use tokio::sync::mpsc;

use crate::{
    auth::{authenticate_token, HasJwtSecret},
    channels,
    common::{ApiError, AppResult},
};

const UUID_PATTERN: &str = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx";

#[derive(Clone, Debug)]
pub(crate) struct PubSubState {
    database: DatabaseConnection,
    jwt_secret: Arc<str>,
    service: PubSubService,
}

impl PubSubState {
    pub(crate) fn new(
        database: DatabaseConnection,
        jwt_secret: String,
        service: PubSubService,
    ) -> Self {
        Self {
            database,
            jwt_secret: Arc::<str>::from(jwt_secret),
            service,
        }
    }
}

impl HasJwtSecret for PubSubState {
    fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PubSubService {
    inner: Arc<PubSubServiceInner>,
}

#[derive(Debug)]
struct PubSubServiceInner {
    store: SubscriptionStore,
    subscribers: DashMap<Uuid, mpsc::UnboundedSender<Message>>,
    socket_channels: DashMap<Uuid, HashSet<String>>,
}

impl PubSubService {
    pub(crate) fn from_env() -> Self {
        Self::new(SubscriptionStore::from_env())
    }

    fn new(store: SubscriptionStore) -> Self {
        Self {
            inner: Arc::new(PubSubServiceInner {
                store,
                subscribers: DashMap::new(),
                socket_channels: DashMap::new(),
            }),
        }
    }

    fn register(
        &self,
        socket_id: Uuid,
        sender: mpsc::UnboundedSender<Message>,
    ) {
        self.inner.subscribers.insert(socket_id, sender);
        self.inner.socket_channels.insert(socket_id, HashSet::new());
    }

    async fn subscribe(&self, socket_id: Uuid, channel: &str) -> AppResult<()> {
        self.inner
            .store
            .add_member(channel_cache_key(channel), socket_id.to_string())
            .await?;

        self.inner
            .socket_channels
            .entry(socket_id)
            .or_default()
            .insert(channel.to_owned());

        Ok(())
    }

    async fn unsubscribe(
        &self,
        socket_id: Uuid,
        channel: &str,
    ) -> AppResult<()> {
        self.inner
            .store
            .remove_member(channel_cache_key(channel), &socket_id.to_string())
            .await?;

        if let Some(mut channels) =
            self.inner.socket_channels.get_mut(&socket_id)
        {
            channels.remove(channel);
        }

        Ok(())
    }

    async fn disconnect(&self, socket_id: Uuid) {
        self.inner.subscribers.remove(&socket_id);

        let Some((_socket_id, channels)) =
            self.inner.socket_channels.remove(&socket_id)
        else {
            return;
        };

        for channel in channels {
            if let Err(error) = self
                .inner
                .store
                .remove_member(
                    channel_cache_key(&channel),
                    &socket_id.to_string(),
                )
                .await
            {
                tracing::warn!(
                    "failed to clean up websocket subscription: {error}"
                );
            }
        }
    }

    pub(crate) async fn publish(
        &self,
        channel: &str,
        body: serde_json::Value,
    ) -> AppResult<()> {
        let message = response_message(channel, Some(body), None)?;
        let subscriber_ids =
            self.inner.store.members(channel_cache_key(channel)).await?;

        for subscriber_id in subscriber_ids {
            let Ok(subscriber_id) = subscriber_id.parse::<Uuid>() else {
                continue;
            };
            let Some(sender) = self.inner.subscribers.get(&subscriber_id)
            else {
                continue;
            };
            let _ = sender.send(message.clone());
        }

        Ok(())
    }
}

pub(crate) async fn websocket_handler(
    State(state): State<PubSubState>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(state, socket))
}

async fn handle_socket(state: PubSubState, mut socket: WebSocket) {
    let socket_id = Uuid::new_v4();
    let (sender, mut outbound) = mpsc::unbounded_channel();
    state.service.register(socket_id, sender);

    loop {
        tokio::select! {
            Some(message) = outbound.recv() => {
                if socket.send(message).await.is_err() {
                    break;
                }
            }
            inbound = socket.recv() => {
                let Some(Ok(message)) = inbound else {
                    break;
                };

                if handle_inbound_message(&state, socket_id, message).await {
                    break;
                }
            }
        }
    }

    state.service.disconnect(socket_id).await;
}

async fn handle_inbound_message(
    state: &PubSubState,
    socket_id: Uuid,
    message: Message,
) -> bool {
    let Message::Text(text) = message else {
        return false;
    };
    let Ok(request) = serde_json::from_str::<PubSubRequest>(&text) else {
        send_socket_error(
            &state.service,
            socket_id,
            "",
            "BAD_REQUEST",
            "Invalid pub-sub request.",
        );
        return false;
    };

    let user_id = match authenticate_token(&request.token, state.jwt_secret()) {
        Ok(user_id) => user_id,
        Err(_) => {
            send_socket_error(
                &state.service,
                socket_id,
                &request.channel,
                "UNAUTHORIZED",
                "Invalid token.",
            );
            return false;
        }
    };

    let access = channel_access(&request.channel, user_id);
    let Some(access) = access else {
        send_socket_error(
            &state.service,
            socket_id,
            &request.channel,
            "FORBIDDEN",
            "Forbidden.",
        );
        return false;
    };

    if channels::get_channel(
        &state.database,
        access.server_id,
        access.channel_id,
    )
    .await
    .is_err()
        || channels::ensure_channel_membership(
            &state.database,
            access.channel_id,
            user_id,
        )
        .await
        .is_err()
    {
        send_socket_error(
            &state.service,
            socket_id,
            &request.channel,
            "FORBIDDEN",
            "Forbidden.",
        );
        return false;
    }

    match request.request {
        PubSubRequestKind::Subscribe => {
            if let Err(error) =
                state.service.subscribe(socket_id, &request.channel).await
            {
                tracing::warn!("failed to subscribe websocket: {error}");
                send_socket_error(
                    &state.service,
                    socket_id,
                    &request.channel,
                    "INTERNAL_SERVER_ERROR",
                    "Internal server error.",
                );
            }
        }
        PubSubRequestKind::Unsubscribe => {
            if let Err(error) =
                state.service.unsubscribe(socket_id, &request.channel).await
            {
                tracing::warn!("failed to unsubscribe websocket: {error}");
            }
        }
        PubSubRequestKind::Publish => {
            if let Some(body) = request.body {
                if let Err(error) =
                    state.service.publish(&request.channel, body).await
                {
                    tracing::warn!(
                        "failed to publish websocket message: {error}"
                    );
                }
            }
        }
    }

    false
}

fn send_socket_error(
    service: &PubSubService,
    socket_id: Uuid,
    channel: &str,
    code: &'static str,
    message: &'static str,
) {
    let Some(sender) = service.inner.subscribers.get(&socket_id) else {
        return;
    };
    let Ok(message) =
        response_message(channel, None, Some(PubSubError { code, message }))
    else {
        return;
    };
    let _ = sender.send(message);
}

fn response_message(
    channel: &str,
    body: Option<serde_json::Value>,
    error: Option<PubSubError>,
) -> AppResult<Message> {
    serde_json::to_string(&PubSubResponse {
        kind: "RESPONSE",
        channel,
        error,
        body,
    })
    .map(|value| Message::Text(value.into()))
    .map_err(internal_error)
}

#[derive(Clone, Debug)]
struct ChannelAccess {
    server_id: Uuid,
    channel_id: Uuid,
}

fn channel_access(channel: &str, user_id: Uuid) -> Option<ChannelAccess> {
    let parts: Vec<&str> = channel.split('-').collect();
    if parts.len() != 17 {
        return None;
    }

    let topic = parts[0..2].join("-");
    if topic != "new-message" && topic != "new-poll" {
        return None;
    }

    let server_id = parse_uuid_parts(&parts[2..7])?;
    let channel_id = parse_uuid_parts(&parts[7..12])?;
    let topic_user_id = parse_uuid_parts(&parts[12..17])?;

    (topic_user_id == user_id).then_some(ChannelAccess {
        server_id,
        channel_id,
    })
}

fn parse_uuid_parts(parts: &[&str]) -> Option<Uuid> {
    if parts.len() != 5 {
        return None;
    }

    let value = parts.join("-");
    if value.len() != UUID_PATTERN.len() {
        return None;
    }

    value.parse::<Uuid>().ok()
}

fn channel_cache_key(channel: &str) -> String {
    format!("channel:{channel}")
}

#[derive(Debug)]
enum SubscriptionStore {
    Redis(redis::Client),
    Memory(Arc<DashMap<String, HashSet<String>>>),
}

impl SubscriptionStore {
    fn from_env() -> Self {
        let Some(redis_url) = redis_url_from_env() else {
            tracing::info!("Redis pub-sub cache is not configured; using in-memory websocket subscriptions.");
            return Self::memory();
        };

        match redis::Client::open(redis_url.clone()) {
            Ok(client) => Self::Redis(client),
            Err(error) => {
                tracing::warn!(
                    "failed to initialize Redis pub-sub cache at {redis_url}: {error}; using in-memory websocket subscriptions"
                );
                Self::memory()
            }
        }
    }

    fn memory() -> Self {
        Self::Memory(Arc::new(DashMap::new()))
    }

    async fn members(&self, key: String) -> AppResult<Vec<String>> {
        match self {
            Self::Redis(client) => {
                let mut connection = client
                    .get_multiplexed_async_connection()
                    .await
                    .map_err(internal_error)?;
                connection.smembers(key).await.map_err(internal_error)
            }
            Self::Memory(channels) => Ok(channels
                .get(&key)
                .map(|members| members.iter().cloned().collect())
                .unwrap_or_default()),
        }
    }

    async fn add_member(&self, key: String, value: String) -> AppResult<()> {
        match self {
            Self::Redis(client) => {
                let mut connection = client
                    .get_multiplexed_async_connection()
                    .await
                    .map_err(internal_error)?;
                let _: usize = connection
                    .sadd(key, value)
                    .await
                    .map_err(internal_error)?;
            }
            Self::Memory(channels) => {
                channels.entry(key).or_default().insert(value);
            }
        }

        Ok(())
    }

    async fn remove_member(&self, key: String, value: &str) -> AppResult<()> {
        match self {
            Self::Redis(client) => {
                let mut connection = client
                    .get_multiplexed_async_connection()
                    .await
                    .map_err(internal_error)?;
                let _: usize = connection
                    .srem(key, value)
                    .await
                    .map_err(internal_error)?;
            }
            Self::Memory(channels) => {
                let remove_key =
                    if let Some(mut members) = channels.get_mut(&key) {
                        members.remove(value);
                        members.is_empty()
                    } else {
                        false
                    };
                if remove_key {
                    channels.remove(&key);
                }
            }
        }

        Ok(())
    }
}

fn redis_url_from_env() -> Option<String> {
    if let Ok(redis_url) = env::var("REDIS_URL") {
        if !redis_url.trim().is_empty() {
            return Some(redis_url);
        }
    }

    let host = env::var("REDIS_HOST").ok()?;
    if host.trim().is_empty() {
        return None;
    }
    let port = env::var("REDIS_PORT").unwrap_or_else(|_| "6379".to_owned());
    let password = env::var("REDIS_PASSWORD").ok();

    Some(match password {
        Some(password) if !password.is_empty() => {
            format!("redis://:{password}@{host}:{port}")
        }
        _ => format!("redis://{host}:{port}"),
    })
}

#[derive(Debug, Deserialize)]
struct PubSubRequest {
    request: PubSubRequestKind,
    channel: String,
    token: String,
    body: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum PubSubRequestKind {
    Publish,
    Subscribe,
    Unsubscribe,
}

#[derive(Debug, Serialize)]
struct PubSubResponse<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    channel: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<PubSubError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct PubSubError {
    code: &'static str,
    message: &'static str,
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("pub-sub request failed: {error}");
    ApiError::new(
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        "Internal server error.",
    )
}
