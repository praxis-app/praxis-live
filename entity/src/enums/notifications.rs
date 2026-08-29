use sea_orm::entity::prelude::*;

use super::macros::impl_enum_string_conversions;

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(
    rs_type = "String",
    db_type = "Enum",
    enum_name = "notifications_kind_enum"
)]
pub enum NotificationKind {
    #[sea_orm(string_value = "new_message")]
    NewMessage,
    #[sea_orm(string_value = "message_reply")]
    MessageReply,
    #[sea_orm(string_value = "forum_reply")]
    ForumReply,
    #[sea_orm(string_value = "proposal_vote")]
    ProposalVote,
    #[sea_orm(string_value = "proposal_ratified")]
    ProposalRatified,
    #[sea_orm(string_value = "proposal_closed")]
    ProposalClosed,
    #[sea_orm(string_value = "server_role_granted")]
    ServerRoleGranted,
}

impl_enum_string_conversions!(NotificationKind {
    NewMessage => "new_message",
    MessageReply => "message_reply",
    ForumReply => "forum_reply",
    ProposalVote => "proposal_vote",
    ProposalRatified => "proposal_ratified",
    ProposalClosed => "proposal_closed",
    ServerRoleGranted => "server_role_granted",
});
