//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.15

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "descriptions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub guid: String,
    pub description: String,
}

#[derive(DeriveIden)]
pub enum Descriptions {
    Table,
    Guid,
    Description,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::activities::Entity")]
    Activities,
}

impl Related<super::activities::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Activities.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
