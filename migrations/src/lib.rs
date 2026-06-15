pub use sea_orm_migration::prelude::*;

mod m20260404_110000_create_users;
mod m20260408_000001_create_basic_chat;
mod m20260414_000001_create_roles;
mod m20260416_000001_create_invites;
mod m20260418_000001_add_user_profiles_and_images;
mod m20260419_000001_create_polls;
mod m20260420_000001_add_anonymous_users;
mod m20260505_000001_add_call_conversations;
mod m20260525_000001_preserve_call_decisions;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260404_110000_create_users::Migration),
            Box::new(m20260408_000001_create_basic_chat::Migration),
            Box::new(m20260414_000001_create_roles::Migration),
            Box::new(m20260416_000001_create_invites::Migration),
            Box::new(m20260418_000001_add_user_profiles_and_images::Migration),
            Box::new(m20260419_000001_create_polls::Migration),
            Box::new(m20260420_000001_add_anonymous_users::Migration),
            Box::new(m20260505_000001_add_call_conversations::Migration),
            Box::new(m20260525_000001_preserve_call_decisions::Migration),
        ]
    }
}
