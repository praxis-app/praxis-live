use sea_orm::DbBackend;
use sea_orm_migration::prelude::{sea_query::extension::postgres::Type, *};

#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() == DbBackend::Postgres {
            manager
                .create_type(
                    Type::create()
                        .as_enum(Alias::new("poll_closed_reason_enum"))
                        .values([Alias::new("event-start-elapsed")])
                        .to_owned(),
                )
                .await?;
        }

        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("polls"))
                    .add_column(
                        ColumnDef::new(Alias::new("closed_reason"))
                            .enumeration(
                                Alias::new("poll_closed_reason_enum"),
                                [Alias::new("event-start-elapsed")],
                            ),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("polls"))
                    .drop_column(Alias::new("closed_reason"))
                    .to_owned(),
            )
            .await?;

        if manager.get_database_backend() == DbBackend::Postgres {
            manager
                .drop_type(
                    Type::drop()
                        .name(Alias::new("poll_closed_reason_enum"))
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }
}
