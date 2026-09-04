use sea_orm::prelude::Uuid;

use super::{channel_access, PubSubTopic};

#[test]
fn notification_topics_round_trip_without_a_channel() {
    let server_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let topic = PubSubTopic::notification(server_id, user_id);

    assert_eq!(
        topic.to_string(),
        format!("notification:{server_id}:{user_id}")
    );
    assert_eq!(PubSubTopic::parse(&topic.to_string()), Some(topic));
}

#[test]
fn channel_topics_still_round_trip() {
    let server_id = Uuid::new_v4();
    let channel_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let topic = PubSubTopic::new_message(server_id, channel_id, user_id);

    assert_eq!(
        topic.to_string(),
        format!("new-message:{server_id}:{channel_id}:{user_id}")
    );
    assert_eq!(PubSubTopic::parse(&topic.to_string()), Some(topic));
}

#[test]
fn notification_access_does_not_require_a_channel() {
    let server_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let topic = PubSubTopic::notification(server_id, user_id).to_string();

    let access =
        channel_access(&topic, user_id).expect("expected topic access");

    assert_eq!(access.server_id, server_id);
    assert_eq!(access.channel_id, None);
    assert!(access.registered_only);
}

#[test]
fn notification_access_is_denied_for_another_user() {
    let topic =
        PubSubTopic::notification(Uuid::new_v4(), Uuid::new_v4()).to_string();

    assert!(channel_access(&topic, Uuid::new_v4()).is_none());
}

#[test]
fn channel_access_does_not_require_a_registered_account() {
    let server_id = Uuid::new_v4();
    let channel_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let topic =
        PubSubTopic::new_message(server_id, channel_id, user_id).to_string();

    let access =
        channel_access(&topic, user_id).expect("expected topic access");

    assert!(!access.registered_only);
}
