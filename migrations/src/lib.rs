pub use sea_orm_migration::prelude::*;

mod m20260404_110000_create_users;
mod m20260408_000001_create_basic_chat;
mod m20260414_000001_create_roles;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260404_110000_create_users::Migration),
            Box::new(m20260408_000001_create_basic_chat::Migration),
            Box::new(m20260414_000001_create_roles::Migration),
        ]
    }
}
