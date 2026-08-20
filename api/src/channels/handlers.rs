use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use super::{
    service,
    types::{
        ChannelOrderRequest, ChannelPath, ChannelPayload, ChannelRequest,
        ChannelsPayload, ServerPath,
    },
};
use crate::{
    auth::{AuthenticatedUser, HasJwtSecret},
    common::{response::EmptyResponse, AppResult},
};

#[derive(Clone, Debug)]
pub(super) struct ChannelsState {
    database: DatabaseConnection,
    jwt_secret: Arc<str>,
}

impl ChannelsState {
    pub(super) fn new(
        database: DatabaseConnection,
        jwt_secret: String,
    ) -> Self {
        Self {
            database,
            jwt_secret: Arc::<str>::from(jwt_secret),
        }
    }
}

impl HasJwtSecret for ChannelsState {
    fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
}

// TODO: This and `update_channel`/`delete_channel` below only check that the
// caller is logged in, unlike the sibling `update_channel_order`, which
// calls `ensure_can_manage_channels`. Confirm whether any authenticated
// user creating, renaming, or deleting channels in any server is
// intentional, or whether these need the same check.
pub(super) async fn create_channel(
    State(state): State<ChannelsState>,
    Path(path): Path<ServerPath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
    Json(payload): Json<ChannelRequest>,
) -> AppResult<Json<ChannelPayload>> {
    let channel =
        service::create_channel(&state.database, path.server_id, payload)
            .await?;
    Ok(Json(ChannelPayload { channel }))
}

pub(super) async fn update_channel(
    State(state): State<ChannelsState>,
    Path(path): Path<ChannelPath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
    Json(payload): Json<ChannelRequest>,
) -> AppResult<Json<EmptyResponse>> {
    service::update_channel(
        &state.database,
        path.server_id,
        path.channel_id,
        payload,
    )
    .await?;
    Ok(Json(EmptyResponse {}))
}

pub(super) async fn update_channel_order(
    State(state): State<ChannelsState>,
    Path(path): Path<ServerPath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<ChannelOrderRequest>,
) -> AppResult<StatusCode> {
    service::update_channel_order(
        &state.database,
        path.server_id,
        user_id,
        payload,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn delete_channel(
    State(state): State<ChannelsState>,
    Path(path): Path<ChannelPath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<EmptyResponse>> {
    service::delete_channel(&state.database, path.server_id, path.channel_id)
        .await?;
    Ok(Json(EmptyResponse {}))
}

pub(super) async fn get_channels(
    State(state): State<ChannelsState>,
    Path(path): Path<ServerPath>,
) -> AppResult<Json<ChannelsPayload>> {
    let channels =
        service::get_channels(&state.database, path.server_id).await?;
    Ok(Json(ChannelsPayload { channels }))
}

pub(super) async fn get_joined_channels(
    State(state): State<ChannelsState>,
    Path(path): Path<ServerPath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> AppResult<Json<ChannelsPayload>> {
    let channels =
        service::get_joined_channels(&state.database, path.server_id, user_id)
            .await?;
    Ok(Json(ChannelsPayload { channels }))
}

pub(super) async fn get_channel(
    State(state): State<ChannelsState>,
    Path(path): Path<ChannelPath>,
) -> AppResult<Json<ChannelPayload>> {
    let channel = service::get_channel_with_server(
        &state.database,
        path.server_id,
        path.channel_id,
    )
    .await?;
    Ok(Json(ChannelPayload { channel }))
}
