use sea_orm::sea_query::Expr;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserConfigs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserConfigs::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(UserConfigs::UserId)
                            .uuid()
                            .not_null()
                            .unique_key(),
                    )
                    .col(notifications_enabled(
                        UserConfigs::MessageNotificationsEnabled,
                    ))
                    .col(notifications_enabled(
                        UserConfigs::ReplyNotificationsEnabled,
                    ))
                    .col(notifications_enabled(
                        UserConfigs::ProposalNotificationsEnabled,
                    ))
                    .col(notifications_enabled(
                        UserConfigs::RoleNotificationsEnabled,
                    ))
                    .col(timestamp(UserConfigs::CreatedAt))
                    .col(timestamp(UserConfigs::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("user-configs-user-id-fkey")
                            .from(UserConfigs::Table, UserConfigs::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserConfigs::Table).to_owned())
            .await
    }
}

fn notifications_enabled<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column)
        .boolean()
        .not_null()
        .default(true)
        .take()
}

fn timestamp<T>(column: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(column)
        .timestamp_with_time_zone()
        .not_null()
        .default(Expr::current_timestamp())
        .take()
}

#[derive(DeriveIden)]
enum UserConfigs {
    Table,
    Id,
    UserId,
    MessageNotificationsEnabled,
    ReplyNotificationsEnabled,
    ProposalNotificationsEnabled,
    RoleNotificationsEnabled,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
