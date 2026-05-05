use axum::{
    routing::{get, post},
    Router,
};
use sea_orm::DatabaseConnection;

use super::handlers::{
    create_call_message, create_message, get_call_feed, get_call_message_image,
    get_channel_feed, get_message_image, upload_call_message_image,
    upload_message_image, ChatState,
};
use crate::pub_sub::PubSubService;

pub(crate) fn router(
    database: DatabaseConnection,
    jwt_secret: String,
    pub_sub_service: PubSubService,
) -> Router {
    Router::new()
        .route("/{channelId}/feed", get(get_channel_feed))
        .route("/{channelId}/calls/{callId}/feed", get(get_call_feed))
        .route("/{channelId}/messages", post(create_message))
        .route("/{channelId}/calls/{callId}/messages", post(create_call_message))
        .route(
            "/{channelId}/messages/{messageId}/images/{imageId}",
            get(get_message_image),
        )
        .route(
            "/{channelId}/calls/{callId}/messages/{messageId}/images/{imageId}",
            get(get_call_message_image),
        )
        .route(
            "/{channelId}/messages/{messageId}/images/{imageId}/upload",
            post(upload_message_image),
        )
        .route(
            "/{channelId}/calls/{callId}/messages/{messageId}/images/{imageId}/upload",
            post(upload_call_message_image),
        )
        .with_state(ChatState::new(database, jwt_secret, pub_sub_service))
}
