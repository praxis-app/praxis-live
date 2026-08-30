use entity::enums::{NotificationKind, VoteType};
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};

/// What a notification points at. Exactly one target is stored per row, which
/// the `notifications_one_target_check` constraint enforces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NotificationTarget {
    Message(Uuid),
    Poll(Uuid),
    ServerRole(Uuid),
}

/// One domain transition worth notifying about. `recipient_ids` are candidates:
/// the creation seam drops the actor, anonymous users, and anyone who cannot
/// currently read the target.
#[derive(Clone, Debug)]
pub(crate) struct NewNotification {
    pub(crate) kind: NotificationKind,
    pub(crate) server_id: Uuid,
    pub(crate) channel_id: Option<Uuid>,
    pub(crate) actor_user_id: Option<Uuid>,
    pub(crate) target: NotificationTarget,
    pub(crate) vote_type: Option<VoteType>,
    pub(crate) recipient_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NotificationPath {
    pub(super) server_id: Uuid,
    pub(super) notification_id: Uuid,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ListNotificationsQuery {
    pub(super) before: Option<String>,
    pub(super) limit: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NotificationUserResponse {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) display_name: Option<String>,
    pub(super) profile_picture: Option<crate::users::UserImageRef>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NotificationTargetResponse {
    pub(super) kind: &'static str,
    pub(super) available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) channel_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) channel_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) thread_root_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) thread_root_kind: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) forum_post_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) poll_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) server_role_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) server_role_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NotificationResponse {
    pub(super) id: String,
    pub(super) kind: &'static str,
    pub(super) server_id: String,
    pub(super) channel_id: Option<String>,
    pub(super) actor: Option<NotificationUserResponse>,
    pub(super) vote_type: Option<&'static str>,
    pub(super) unread_count: Option<i32>,
    pub(super) read_at: Option<String>,
    pub(super) created_at: String,
    pub(super) target: NotificationTargetResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NotificationsResponse {
    pub(super) notifications: Vec<NotificationResponse>,
    pub(super) next_cursor: Option<String>,
    pub(super) has_more: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NotificationPayload {
    pub(super) notification: NotificationResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UnreadCountResponse {
    pub(super) unread_count: u64,
}
