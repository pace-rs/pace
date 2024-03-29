//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.15

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "tags")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub guid: String,
    pub tag: String,
}

#[derive(DeriveIden)]
pub enum TagsEnum {
    Table,
    Guid,
    Tag,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::activities_tags::Entity")]
    ActivitiesTags,
}

impl Related<super::activities_tags::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ActivitiesTags.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
