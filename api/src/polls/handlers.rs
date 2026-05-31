use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, Response, StatusCode},
    response::Json,
};
use sea_orm::DatabaseConnection;
use std::{path::PathBuf, sync::Arc};

use super::{
    extractors::{PollDeleteContext, PollImageUploadContext},
    service,
    types::{CallDecisionResponse, CreatePollRequest, PollImagePath},
};
use crate::{
    auth::HasJwtSecret,
    calls::extractors::CallWriteContext,
    channels::{self, extractors::ChannelWriteContext},
    common::{request::multipart_file, ApiError, AppResult},
    pub_sub::PubSubService,
};

#[derive(Clone, Debug)]
pub(crate) struct PollsState {
    pub(crate) database: DatabaseConnection,
    jwt_secret: Arc<str>,
    pub(crate) pub_sub_service: PubSubService,
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

impl channels::extractors::HasDatabase for PollsState {
    fn database(&self) -> &DatabaseConnection {
        &self.database
    }
}

pub(super) async fn create_poll(
    State(state): State<PollsState>,
    context: ChannelWriteContext,
    Json(payload): Json<CreatePollRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let poll = service::create_poll(
        &state.database,
        context.server_id,
        context.channel_id,
        context.user_id,
        payload,
    )
    .await?;

    if let Err(error) = service::broadcast_poll_update(
        &state.database,
        &state.pub_sub_service,
        context.server_id,
        context.channel_id,
        Some(context.user_id),
        poll.id.parse().expect("created poll id is valid"),
    )
    .await
    {
        tracing::warn!("failed to broadcast created poll: {error}");
    }

    Ok(Json(serde_json::json!({ "poll": poll })))
}

pub(super) async fn create_call_poll(
    State(state): State<PollsState>,
    context: CallWriteContext,
    Json(payload): Json<CreatePollRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let poll = service::create_call_poll(
        &state.database,
        context.server_id,
        context.channel_id,
        context.call_id,
        context.user_id,
        payload,
    )
    .await?;

    if let Err(error) = service::broadcast_poll_update(
        &state.database,
        &state.pub_sub_service,
        context.server_id,
        context.channel_id,
        Some(context.user_id),
        poll.id.parse().expect("created poll id is valid"),
    )
    .await
    {
        tracing::warn!("failed to broadcast created in-call poll: {error}");
    }

    Ok(Json(serde_json::json!({ "poll": poll })))
}

pub(super) async fn get_call_decision(
    State(state): State<PollsState>,
    context: CallWriteContext,
) -> AppResult<Json<CallDecisionResponse>> {
    let decision = service::get_call_decision(
        &state.database,
        context.server_id,
        context.channel_id,
        context.call_id,
        context.user_id,
    )
    .await?;

    Ok(Json(decision))
}

pub(super) async fn upload_poll_image(
    State(state): State<PollsState>,
    context: PollImageUploadContext,
    multipart: Multipart,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let file = multipart_file(multipart, "file").await?;

    let image = service::store_poll_image(
        &state.database,
        &state.upload_root,
        &context.poll,
        context.image_id,
        file.as_ref().and_then(|file| file.content_type.clone()),
        file.map(|file| file.bytes).unwrap_or_default(),
    )
    .await?;

    if let Err(error) = service::broadcast_poll_image_upload(
        &state.database,
        &state.pub_sub_service,
        context.server_id,
        context.channel_id,
        context.user_id,
        &context.poll.id.to_string(),
        &context.image_id.to_string(),
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
    let image = service::get_poll_image(
        &state.database,
        &state.upload_root,
        path.server_id,
        path.channel_id,
        path.poll_id,
        path.image_id,
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

pub(super) async fn delete_poll(
    State(state): State<PollsState>,
    context: PollDeleteContext,
) -> AppResult<Json<serde_json::Value>> {
    let result = service::delete_poll(
        &state.database,
        &state.upload_root,
        &context.poll,
    )
    .await?;

    Ok(Json(
        serde_json::json!({ "affected": result.rows_affected }),
    ))
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("poll route failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
