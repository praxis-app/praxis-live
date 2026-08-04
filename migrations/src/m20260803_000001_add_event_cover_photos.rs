use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PollActionEventCoverPhotos::Table)
                    .col(
                        ColumnDef::new(PollActionEventCoverPhotos::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(
                            PollActionEventCoverPhotos::PollActionEventId,
                        )
                        .uuid()
                        .not_null(),
                    )
                    .col(
                        ColumnDef::new(PollActionEventCoverPhotos::StorageKey)
                            .string(),
                    )
                    .col(
                        ColumnDef::new(PollActionEventCoverPhotos::ContentType)
                            .string(),
                    )
                    .col(timestamp(PollActionEventCoverPhotos::CreatedAt))
                    .col(timestamp(PollActionEventCoverPhotos::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name(
                                "poll-action-event-cover-photos-event-id-fkey",
                            )
                            .from(
                                PollActionEventCoverPhotos::Table,
                                PollActionEventCoverPhotos::PollActionEventId,
                            )
                            .to(PollActionEvents::Table, PollActionEvents::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .name("poll-action-event-cover-photos-event-id-key")
                            .col(PollActionEventCoverPhotos::PollActionEventId)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(EventCoverPhotos::Table)
                    .col(
                        ColumnDef::new(EventCoverPhotos::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(EventCoverPhotos::EventId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(EventCoverPhotos::StorageKey)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(EventCoverPhotos::ContentType).string())
                    .col(timestamp(EventCoverPhotos::CreatedAt))
                    .col(timestamp(EventCoverPhotos::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("event-cover-photos-event-id-fkey")
                            .from(
                                EventCoverPhotos::Table,
                                EventCoverPhotos::EventId,
                            )
                            .to(Events::Table, Events::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .name("event-cover-photos-event-id-key")
                            .col(EventCoverPhotos::EventId)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(EventCoverPhotos::Table).to_owned())
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(PollActionEventCoverPhotos::Table)
                    .to_owned(),
            )
            .await
    }
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
enum PollActionEventCoverPhotos {
    Table,
    Id,
    PollActionEventId,
    StorageKey,
    ContentType,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum EventCoverPhotos {
    Table,
    Id,
    EventId,
    StorageKey,
    ContentType,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum PollActionEvents {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Events {
    Table,
    Id,
}
