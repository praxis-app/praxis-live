use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub password_hash: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::user_images::Entity")]
    UserImages,
}

impl Related<super::user_images::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserImages.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
