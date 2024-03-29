mod m20240325_000001_create_activities;
mod m20240326_000001_create_tags;
mod m20240326_000002_create_categories;
mod m20240326_000003_create_activity_status;
mod m20240326_000004_create_activity_kinds;
mod m20240326_000005_create_activities_tags;
mod m20240326_000006_create_activities_categories;
mod m20240327_000001_create_activity_status_values;
mod m20240327_000002_create_activity_kind_values;
mod m20240329_000001_create_descriptions;

pub use sea_orm_migration::prelude::{async_trait, MigrationTrait, MigratorTrait};

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240325_000001_create_activities::Migration),
            Box::new(m20240326_000001_create_tags::Migration),
            Box::new(m20240326_000002_create_categories::Migration),
            Box::new(m20240326_000003_create_activity_status::Migration),
            Box::new(m20240326_000004_create_activity_kinds::Migration),
            Box::new(m20240326_000005_create_activities_tags::Migration),
            Box::new(m20240326_000006_create_activities_categories::Migration),
            Box::new(m20240327_000001_create_activity_status_values::Migration),
            Box::new(m20240327_000002_create_activity_kind_values::Migration),
            Box::new(m20240329_000001_create_descriptions::Migration),
        ]
    }
}
