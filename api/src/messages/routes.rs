use axum::{
    routing::{get, post},
    Router,
};
use sea_orm::DatabaseConnection;

use super::handlers::{
    create_message, get_channel_feed, get_message_image, upload_message_image,
    ChatState,
};

pub(crate) fn router(
    database: DatabaseConnection,
    jwt_secret: String,
) -> Router {
    Router::new()
        .route("/{channelId}/feed", get(get_channel_feed))
        .route("/{channelId}/messages", post(create_message))
        .route(
            "/{channelId}/messages/{messageId}/images/{imageId}",
            get(get_message_image),
        )
        .route(
            "/{channelId}/messages/{messageId}/images/{imageId}/upload",
            post(upload_message_image),
        )
        .with_state(ChatState::new(database, jwt_secret))
}
