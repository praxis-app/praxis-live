use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, Response, StatusCode},
    response::Json,
};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};

use super::{service, types::CreatePollRequest};
use crate::{
    auth::{AuthenticatedUser, HasJwtSecret},
    channels,
    common::{
        request::{multipart_file, parse_uuid},
        ApiError, AppResult,
    },
    pub_sub::PubSubService,
};

#[derive(Clone, Debug)]
pub(crate) struct PollsState {
    pub(crate) database: DatabaseConnection,
    jwt_secret: Arc<str>,
    pub_sub_service: PubSubService,
    upload_root: Arc<PathBuf>,
}

impl PollsState {
    pub(super) fn new(
        database: DatabaseConnection,
        jwt_secret: String,
        pub_sub_service: PubSubService,
    ) -> Self {
        Self {
            database,
            jwt_secret: Arc::<str>::from(jwt_secret),
            pub_sub_service,
            upload_root: Arc::new(service::upload_root()),
        }
    }
}

impl HasJwtSecret for PollsState {
    fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct ChannelPath {
    #[serde(rename = "serverId")]
    server_id: String,
    #[serde(rename = "channelId")]
    channel_id: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct PollPath {
    #[serde(rename = "serverId")]
    server_id: String,
    #[serde(rename = "channelId")]
    channel_id: String,
    #[serde(rename = "pollId")]
    poll_id: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct PollImagePath {
    #[serde(rename = "serverId")]
    server_id: String,
    #[serde(rename = "channelId")]
    channel_id: String,
    #[serde(rename = "pollId")]
    poll_id: String,
    #[serde(rename = "imageId")]
    image_id: String,
}

pub(super) async fn create_poll(
    State(state): State<PollsState>,
    Path(path): Path<ChannelPath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<CreatePollRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&path.server_id, "serverId")?;
    let channel_id = parse_uuid(&path.channel_id, "channelId")?;
    let poll = service::create_poll(
        &state.database,
        server_id,
        channel_id,
        user_id,
        payload,
    )
    .await?;

    if let Err(error) =
        broadcast_poll(&state, server_id, channel_id, user_id, &poll).await
    {
        tracing::warn!("failed to broadcast created poll: {error}");
    }

    Ok(Json(serde_json::json!({ "poll": poll })))
}

pub(super) async fn upload_poll_image(
    State(state): State<PollsState>,
    Path(path): Path<PollImagePath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    multipart: Multipart,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let server_id = parse_uuid(&path.server_id, "serverId")?;
    let channel_id = parse_uuid(&path.channel_id, "channelId")?;
    let poll_id = parse_uuid(&path.poll_id, "pollId")?;
    let image_id = parse_uuid(&path.image_id, "imageId")?;
    let file = multipart_file(multipart, "file").await?;

    let image = service::store_poll_image(
        &state.database,
        &state.upload_root,
        server_id,
        channel_id,
        poll_id,
        image_id,
        user_id,
        file.as_ref().and_then(|file| file.content_type.clone()),
        file.map(|file| file.bytes).unwrap_or_default(),
    )
    .await?;

    if let Err(error) = broadcast_poll_image_upload(
        &state,
        server_id,
        channel_id,
        user_id,
        &path.poll_id,
        &path.image_id,
    )
    .await
    {
        tracing::warn!("failed to broadcast uploaded poll image: {error}");
    }

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "image": image })),
    ))
}

pub(super) async fn get_poll_image(
    State(state): State<PollsState>,
    Path(path): Path<PollImagePath>,
) -> AppResult<Response<Body>> {
    let server_id = parse_uuid(&path.server_id, "serverId")?;
    let channel_id = parse_uuid(&path.channel_id, "channelId")?;
    let poll_id = parse_uuid(&path.poll_id, "pollId")?;
    let image_id = parse_uuid(&path.image_id, "imageId")?;

    let image = service::get_poll_image(
        &state.database,
        &state.upload_root,
        server_id,
        channel_id,
        poll_id,
        image_id,
    )
    .await?;

    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            image
                .content_type
                .unwrap_or_else(|| "application/octet-stream".to_owned()),
        )
        .body(Body::from(image.bytes))
        .map_err(internal_error)
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("poll route failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}

async fn broadcast_poll(
    state: &PollsState,
    server_id: sea_orm::prelude::Uuid,
    channel_id: sea_orm::prelude::Uuid,
    sender_id: sea_orm::prelude::Uuid,
    poll: &crate::polls::types::PollResponse,
) -> AppResult<()> {
    let body = serde_json::json!({
        "type": "poll",
        "poll": poll,
    });

    broadcast_to_channel_members(state, server_id, channel_id, sender_id, body)
        .await
}

async fn broadcast_poll_image_upload(
    state: &PollsState,
    server_id: sea_orm::prelude::Uuid,
    channel_id: sea_orm::prelude::Uuid,
    sender_id: sea_orm::prelude::Uuid,
    poll_id: &str,
    image_id: &str,
) -> AppResult<()> {
    let body = serde_json::json!({
        "type": "image",
        "isPlaceholder": false,
        "pollId": poll_id,
        "imageId": image_id,
    });

    broadcast_to_channel_members(state, server_id, channel_id, sender_id, body)
        .await
}

async fn broadcast_to_channel_members(
    state: &PollsState,
    server_id: sea_orm::prelude::Uuid,
    channel_id: sea_orm::prelude::Uuid,
    sender_id: sea_orm::prelude::Uuid,
    body: serde_json::Value,
) -> AppResult<()> {
    let members =
        channels::get_channel_member_user_ids(&state.database, channel_id)
            .await?;

    for member_id in members {
        if member_id == sender_id {
            continue;
        }

        let topic = format!("new-poll-{server_id}-{channel_id}-{member_id}");
        state.pub_sub_service.publish(&topic, body.clone()).await?;
    }

    Ok(())
}
