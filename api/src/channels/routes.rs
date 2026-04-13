use axum::{
    routing::{delete, get, post, put},
    Router,
};
use sea_orm::DatabaseConnection;

use super::handlers::{
    create_channel, delete_channel, get_channel, get_channels, get_joined_channels, update_channel,
    ChannelsState,
};
use crate::messages;

pub(crate) fn router(database: DatabaseConnection, jwt_secret: String) -> Router {
    let channels_router = Router::new()
        .route("/", get(get_channels))
        .route("/joined", get(get_joined_channels))
        .route("/", post(create_channel))
        .route("/{channelId}", get(get_channel))
        .route("/{channelId}", put(update_channel))
        .route("/{channelId}", delete(delete_channel))
        .with_state(ChannelsState::new(database.clone(), jwt_secret.clone()));

    channels_router.merge(messages::router(database, jwt_secret))
}
