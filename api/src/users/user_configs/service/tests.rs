use chrono::Utc;
use entity::{enums::NotificationKind, user_configs};
use sea_orm::prelude::Uuid;

use super::allows_notification_kind;

const ALL_KINDS: [NotificationKind; 7] = [
    NotificationKind::NewMessage,
    NotificationKind::MessageReply,
    NotificationKind::ForumReply,
    NotificationKind::ProposalVote,
    NotificationKind::ProposalRatified,
    NotificationKind::ProposalClosed,
    NotificationKind::ServerRoleGranted,
];

fn config() -> user_configs::Model {
    user_configs::Model {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        message_notifications_enabled: true,
        reply_notifications_enabled: true,
        proposal_notifications_enabled: true,
        role_notifications_enabled: true,
        created_at: Utc::now().fixed_offset(),
        updated_at: Utc::now().fixed_offset(),
    }
}

#[test]
fn every_kind_is_allowed_by_default() {
    let config = config();

    for kind in ALL_KINDS {
        assert!(allows_notification_kind(&config, kind), "{kind:?}");
    }
}

#[test]
fn each_toggle_only_silences_its_own_kinds() {
    let cases = [
        (
            user_configs::Model {
                message_notifications_enabled: false,
                ..config()
            },
            vec![NotificationKind::NewMessage],
        ),
        (
            user_configs::Model {
                reply_notifications_enabled: false,
                ..config()
            },
            vec![NotificationKind::MessageReply, NotificationKind::ForumReply],
        ),
        (
            user_configs::Model {
                proposal_notifications_enabled: false,
                ..config()
            },
            vec![
                NotificationKind::ProposalVote,
                NotificationKind::ProposalRatified,
                NotificationKind::ProposalClosed,
            ],
        ),
        (
            user_configs::Model {
                role_notifications_enabled: false,
                ..config()
            },
            vec![NotificationKind::ServerRoleGranted],
        ),
    ];

    for (config, silenced) in cases {
        for kind in ALL_KINDS {
            assert_eq!(
                allows_notification_kind(&config, kind),
                !silenced.contains(&kind),
                "{kind:?}",
            );
        }
    }
}
