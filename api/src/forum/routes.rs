use axum::{
    routing::{delete, get, post},
    Router,
};
use sea_orm::DatabaseConnection;

use super::handlers::{
    close_forum_post, create_forum_post, create_forum_reply,
    delete_forum_reply, get_forum_post, list_forum_posts, update_forum_post,
    ForumState,
};
use crate::pub_sub::PubSubService;

pub(crate) fn router(
    database: DatabaseConnection,
    jwt_secret: String,
    pub_sub_service: PubSubService,
) -> Router {
    Router::new()
        .route("/posts", get(list_forum_posts).post(create_forum_post))
        .route(
            "/posts/{postId}",
            get(get_forum_post).put(update_forum_post),
        )
        .route("/posts/{postId}/close", post(close_forum_post))
        .route("/posts/{postId}/replies", post(create_forum_reply))
        .route(
            "/posts/{postId}/replies/{replyId}",
            delete(delete_forum_reply),
        )
        .with_state(ForumState::new(database, jwt_secret, pub_sub_service))
}
