use axum::{
    routing::{delete, get, post},
    Router,
};
use sea_orm::DatabaseConnection;

use super::handlers::{
    create_call_poll, create_poll, delete_poll, get_poll_image,
    upload_poll_image, PollsState,
};
use crate::{pub_sub::PubSubService, votes};

pub(crate) fn router(
    database: DatabaseConnection,
    jwt_secret: String,
    pub_sub_service: PubSubService,
) -> Router {
    Router::new()
        .route("/", post(create_poll))
        .route("/{pollId}", delete(delete_poll))
        .route("/{pollId}/images/{imageId}", get(get_poll_image))
        .route("/{pollId}/images/{imageId}/upload", post(upload_poll_image))
        .route(
            "/{pollId}/options/{pollOptionId}/voters",
            get(votes::handlers::get_voters_by_poll_option),
        )
        .nest("/{pollId}/votes", votes::routes::router())
        .with_state(PollsState::new(database, jwt_secret, pub_sub_service))
}

pub(crate) fn call_router(
    database: DatabaseConnection,
    jwt_secret: String,
    pub_sub_service: PubSubService,
) -> Router {
    Router::new()
        .route("/", post(create_call_poll))
        .with_state(PollsState::new(database, jwt_secret, pub_sub_service))
}
