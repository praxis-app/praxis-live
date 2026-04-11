use sea_orm::sea_query::Expr;
use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::prelude::Uuid;

const DEFAULT_SERVER_ID: &str = "11111111-1111-1111-1111-111111111111";
const DEFAULT_CHANNEL_ID: &str = "22222222-2222-2222-2222-222222222222";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Servers::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Servers::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Servers::Slug).string().not_null())
                    .col(ColumnDef::new(Servers::Name).string().not_null())
                    .col(ColumnDef::new(Servers::Description).string())
                    .col(
                        ColumnDef::new(Servers::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Servers::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .index(
                        Index::create()
                            .name("servers-slug-key")
                            .col(Servers::Slug)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ServerMembers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ServerMembers::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ServerMembers::ServerId).uuid().not_null())
                    .col(
                        ColumnDef::new(ServerMembers::UserId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ServerMembers::LastActiveAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(ServerMembers::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ServerMembers::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("server-members-server-id-fkey")
                            .from(ServerMembers::Table, ServerMembers::ServerId)
                            .to(Servers::Table, Servers::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("server-members-user-id-fkey")
                            .from(ServerMembers::Table, ServerMembers::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .name("server-members-user-id-server-id-key")
                            .col(ServerMembers::UserId)
                            .col(ServerMembers::ServerId)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ServerConfigs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ServerConfigs::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ServerConfigs::ServerId).uuid().not_null())
                    .col(
                        ColumnDef::new(ServerConfigs::DecisionMakingModel)
                            .string()
                            .not_null()
                            .default("consensus"),
                    )
                    .col(
                        ColumnDef::new(ServerConfigs::DisagreementsLimit)
                            .integer()
                            .not_null()
                            .default(2),
                    )
                    .col(
                        ColumnDef::new(ServerConfigs::AbstainsLimit)
                            .integer()
                            .not_null()
                            .default(2),
                    )
                    .col(
                        ColumnDef::new(ServerConfigs::AgreementThreshold)
                            .integer()
                            .not_null()
                            .default(51),
                    )
                    .col(
                        ColumnDef::new(ServerConfigs::QuorumEnabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(ServerConfigs::QuorumThreshold)
                            .integer()
                            .not_null()
                            .default(25),
                    )
                    .col(
                        ColumnDef::new(ServerConfigs::VotingTimeLimit)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(ServerConfigs::AnonymousUsersEnabled)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(ServerConfigs::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ServerConfigs::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("server-configs-server-id-fkey")
                            .from(ServerConfigs::Table, ServerConfigs::ServerId)
                            .to(Servers::Table, Servers::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .name("server-configs-server-id-key")
                            .col(ServerConfigs::ServerId)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(InstanceConfigs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(InstanceConfigs::Id)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(InstanceConfigs::DefaultServerId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(InstanceConfigs::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(InstanceConfigs::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("instance-configs-default-server-id-fkey")
                            .from(InstanceConfigs::Table, InstanceConfigs::DefaultServerId)
                            .to(Servers::Table, Servers::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Channels::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Channels::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Channels::ServerId).uuid().not_null())
                    .col(ColumnDef::new(Channels::Name).string().not_null())
                    .col(ColumnDef::new(Channels::Description).string())
                    .col(
                        ColumnDef::new(Channels::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Channels::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("channels-server-id-fkey")
                            .from(Channels::Table, Channels::ServerId)
                            .to(Servers::Table, Servers::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .name("channels-server-id-name-key")
                            .col(Channels::ServerId)
                            .col(Channels::Name)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ChannelMembers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ChannelMembers::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ChannelMembers::ChannelId).uuid().not_null())
                    .col(
                        ColumnDef::new(ChannelMembers::UserId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ChannelMembers::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("channel-members-channel-id-fkey")
                            .from(ChannelMembers::Table, ChannelMembers::ChannelId)
                            .to(Channels::Table, Channels::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("channel-members-user-id-fkey")
                            .from(ChannelMembers::Table, ChannelMembers::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .name("channel-members-user-id-channel-id-key")
                            .col(ChannelMembers::UserId)
                            .col(ChannelMembers::ChannelId)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Messages::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Messages::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Messages::ChannelId).uuid().not_null())
                    .col(ColumnDef::new(Messages::UserId).big_integer().not_null())
                    .col(ColumnDef::new(Messages::Body).text())
                    .col(
                        ColumnDef::new(Messages::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Messages::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("messages-channel-id-fkey")
                            .from(Messages::Table, Messages::ChannelId)
                            .to(Channels::Table, Channels::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("messages-user-id-fkey")
                            .from(Messages::Table, Messages::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MessageImages::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MessageImages::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(MessageImages::MessageId).uuid().not_null())
                    .col(ColumnDef::new(MessageImages::StorageKey).string())
                    .col(ColumnDef::new(MessageImages::ContentType).string())
                    .col(
                        ColumnDef::new(MessageImages::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(MessageImages::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("message-images-message-id-fkey")
                            .from(MessageImages::Table, MessageImages::MessageId)
                            .to(Messages::Table, Messages::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Servers::Table)
                    .columns([
                        Servers::Id,
                        Servers::Slug,
                        Servers::Name,
                        Servers::Description,
                    ])
                    .values_panic([
                        Expr::value(DEFAULT_SERVER_ID.parse::<Uuid>().unwrap()),
                        Expr::value("praxis"),
                        Expr::value("Praxis"),
                        Expr::value(Value::String(None)),
                    ])
                    .to_owned(),
            )
            .await?;

        manager
            .exec_stmt(
                Query::insert()
                    .into_table(ServerConfigs::Table)
                    .columns([ServerConfigs::Id, ServerConfigs::ServerId])
                    .values_panic([
                        Expr::value(
                            "33333333-3333-3333-3333-333333333333"
                                .parse::<Uuid>()
                                .unwrap(),
                        ),
                        Expr::value(DEFAULT_SERVER_ID.parse::<Uuid>().unwrap()),
                    ])
                    .to_owned(),
            )
            .await?;

        manager
            .exec_stmt(
                Query::insert()
                    .into_table(InstanceConfigs::Table)
                    .columns([InstanceConfigs::Id, InstanceConfigs::DefaultServerId])
                    .values_panic([
                        Expr::value(1_i64),
                        Expr::value(DEFAULT_SERVER_ID.parse::<Uuid>().unwrap()),
                    ])
                    .to_owned(),
            )
            .await?;

        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Channels::Table)
                    .columns([
                        Channels::Id,
                        Channels::ServerId,
                        Channels::Name,
                        Channels::Description,
                    ])
                    .values_panic([
                        Expr::value(DEFAULT_CHANNEL_ID.parse::<Uuid>().unwrap()),
                        Expr::value(DEFAULT_SERVER_ID.parse::<Uuid>().unwrap()),
                        Expr::value("general"),
                        Expr::value(Value::String(None)),
                    ])
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MessageImages::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Messages::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(ChannelMembers::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(InstanceConfigs::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(ServerConfigs::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(ServerMembers::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Channels::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Servers::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Servers {
    Table,
    Id,
    Slug,
    Name,
    Description,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ServerMembers {
    Table,
    Id,
    ServerId,
    UserId,
    LastActiveAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ServerConfigs {
    Table,
    Id,
    ServerId,
    DecisionMakingModel,
    DisagreementsLimit,
    AbstainsLimit,
    AgreementThreshold,
    QuorumEnabled,
    QuorumThreshold,
    VotingTimeLimit,
    AnonymousUsersEnabled,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum InstanceConfigs {
    Table,
    Id,
    DefaultServerId,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Channels {
    Table,
    Id,
    ServerId,
    Name,
    Description,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ChannelMembers {
    Table,
    Id,
    ChannelId,
    UserId,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Messages {
    Table,
    Id,
    ChannelId,
    UserId,
    Body,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum MessageImages {
    Table,
    Id,
    MessageId,
    StorageKey,
    ContentType,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
