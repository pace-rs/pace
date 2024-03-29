//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.15

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "activities_tags")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub guid: String,
    pub tag_guid: String,
    pub activity_guid: String,
}

#[derive(DeriveIden)]
pub enum ActivitiesTagsEnum {
    Table,
    Guid,
    TagGuid,
    ActivityGuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::activities::Entity",
        from = "Column::ActivityGuid",
        to = "super::activities::Column::Guid",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Activities,
    #[sea_orm(
        belongs_to = "super::tags::Entity",
        from = "Column::TagGuid",
        to = "super::tags::Column::Guid",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Tags,
}

impl Related<super::activities::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Activities.def()
    }
}

impl Related<super::tags::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tags.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}