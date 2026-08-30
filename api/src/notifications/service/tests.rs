use sea_orm::prelude::Uuid;

use super::{
    resolve_limit, target_message_id, target_poll_id, target_server_role_id,
    DEFAULT_LIMIT, MAX_LIMIT,
};
use crate::notifications::NotificationTarget;

#[test]
fn missing_limit_falls_back_to_the_default_page_size() {
    assert_eq!(resolve_limit(None), DEFAULT_LIMIT);
}

#[test]
fn limit_is_capped() {
    assert_eq!(resolve_limit(Some(MAX_LIMIT + 100)), MAX_LIMIT);
}

#[test]
fn limit_of_zero_still_returns_a_page() {
    assert_eq!(resolve_limit(Some(0)), 1);
}

#[test]
fn limit_within_range_is_honored() {
    assert_eq!(resolve_limit(Some(10)), 10);
}

#[test]
fn each_target_fills_exactly_one_column() {
    let id = Uuid::new_v4();

    for (target, expected) in [
        (NotificationTarget::Message(id), (Some(id), None, None)),
        (NotificationTarget::Poll(id), (None, Some(id), None)),
        (NotificationTarget::ServerRole(id), (None, None, Some(id))),
    ] {
        assert_eq!(
            (
                target_message_id(target),
                target_poll_id(target),
                target_server_role_id(target),
            ),
            expected,
        );
    }
}
