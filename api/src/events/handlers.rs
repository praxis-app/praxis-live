use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use super::{
    service,
    types::{
        EventPath, EventPayload, EventsResponse, ListEventsQuery,
        UpsertEventRsvpRequest,
    },
};
use crate::{
    auth::{AuthenticatedUser, HasJwtSecret},
    common::AppResult,
};

#[derive(Clone, Debug)]
pub(super) struct EventsState {
    database: DatabaseConnection,
    jwt_secret: Arc<str>,
}

impl EventsState {
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

impl HasJwtSecret for EventsState {
    fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
}

pub(super) async fn list_events(
    State(state): State<EventsState>,
    Path(path): Path<crate::servers::types::ServerPath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Query(query): Query<ListEventsQuery>,
) -> AppResult<Json<EventsResponse>> {
    service::list_events(&state.database, path.server_id, user_id, query)
        .await
        .map(Json)
}

pub(super) async fn get_event(
    State(state): State<EventsState>,
    Path(path): Path<EventPath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> AppResult<Json<EventPayload>> {
    service::get_event(&state.database, path.server_id, path.event_id, user_id)
        .await
        .map(|event| Json(EventPayload { event }))
}

pub(super) async fn upsert_rsvp(
    State(state): State<EventsState>,
    Path(path): Path<EventPath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<UpsertEventRsvpRequest>,
) -> AppResult<Json<EventPayload>> {
    service::upsert_rsvp(
        &state.database,
        path.server_id,
        path.event_id,
        user_id,
        payload.status,
    )
    .await
    .map(|event| Json(EventPayload { event }))
}

pub(super) async fn clear_rsvp(
    State(state): State<EventsState>,
    Path(path): Path<EventPath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> AppResult<Json<EventPayload>> {
    service::clear_rsvp(&state.database, path.server_id, path.event_id, user_id)
        .await
        .map(|event| Json(EventPayload { event }))
}
