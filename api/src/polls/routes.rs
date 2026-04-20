use axum::{
    routing::{get, post, put},
    Router,
};
use sea_orm::DatabaseConnection;

use super::handlers::{
    create_poll, get_poll_image, upload_poll_image, PollsState,
};
use crate::{pub_sub::PubSubService, votes};

pub(crate) fn router(
    database: DatabaseConnection,
    jwt_secret: String,
    pub_sub_service: PubSubService,
) -> Router {
    Router::new()
        .route("/{channelId}/polls", post(create_poll))
        .route(
            "/{channelId}/polls/{pollId}/images/{imageId}",
            get(get_poll_image),
        )
        .route(
            "/{channelId}/polls/{pollId}/images/{imageId}/upload",
            post(upload_poll_image),
        )
        .route(
            "/{channelId}/polls/{pollId}/options/{pollOptionId}/voters",
            get(votes::handlers::get_voters_by_poll_option),
        )
        .route(
            "/{channelId}/polls/{pollId}/votes",
            post(votes::handlers::create_vote),
        )
        .route(
            "/{channelId}/polls/{pollId}/votes/{voteId}",
            put(votes::handlers::update_vote)
                .delete(votes::handlers::delete_vote),
        )
        .with_state(PollsState::new(database, jwt_secret, pub_sub_service))
}
