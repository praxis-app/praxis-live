use axum::{
    routing::{delete, get, post, put},
    Router,
};
use sea_orm::DatabaseConnection;

use super::handlers::{
    create_channel, delete_channel, get_channel, get_channels,
    get_joined_channels, update_channel, ChannelsState,
};
use crate::{
    calls, calls::LiveKitConfig, feeds, forum, messages, polls,
    pub_sub::PubSubService,
};

pub(crate) fn router(
    database: DatabaseConnection,
    jwt_secret: String,
    pub_sub_service: PubSubService,
    livekit: Option<LiveKitConfig>,
) -> Router {
    let channels_router = Router::new()
        .route("/", get(get_channels))
        .route("/joined", get(get_joined_channels))
        .route("/", post(create_channel))
        .route("/{channelId}", get(get_channel))
        .route("/{channelId}", put(update_channel))
        .route("/{channelId}", delete(delete_channel))
        .with_state(ChannelsState::new(database.clone(), jwt_secret.clone()));

    channels_router
        .nest(
            "/{channelId}/feed",
            feeds::channel_feed_router(database.clone(), jwt_secret.clone()),
        )
        .nest(
            "/{channelId}/forum",
            forum::router(
                database.clone(),
                jwt_secret.clone(),
                pub_sub_service.clone(),
            ),
        )
        .nest(
            "/{channelId}/messages",
            messages::router(
                database.clone(),
                jwt_secret.clone(),
                pub_sub_service.clone(),
            ),
        )
        .nest(
            "/{channelId}/calls",
            calls::router(
                database.clone(),
                jwt_secret.clone(),
                pub_sub_service.clone(),
                livekit,
            ),
        )
        .nest(
            "/{channelId}/polls",
            polls::router(
                database.clone(),
                jwt_secret.clone(),
                pub_sub_service.clone(),
            ),
        )
}
