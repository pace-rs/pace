mod create_activities_20240325143710;
mod create_activities_categories_20240326130013;
mod create_activities_tags_20240326125630;
mod create_activity_kind_values_20240327162015;
mod create_activity_kinds_20240326124253;
mod create_activity_status_20240326125819;
mod create_activity_status_values_20240327163645;
mod create_categories_20240326125937;
mod create_schema_migrations;
mod create_tags_20240326125555;

use eyre::OptionExt;
use rusqlite::Connection;
use sea_query::{Cond, Expr, Query, SqliteQueryBuilder};
use std::collections::VecDeque;
use ulid::Ulid;

use pace_core::prelude::PaceStorageResult;

use crate::entities::schema_migrations::SchemaMigrationsIden;

pub trait SQLiteMigration {
    fn version(&self) -> String;
    fn up(&self) -> String;
    fn down(&self) -> String;
}

pub struct SQLiteMigrator<'conn> {
    iterator: Box<dyn Iterator<Item = Box<dyn SQLiteMigration>>>,
    applied: VecDeque<Box<dyn SQLiteMigration>>,
    connection: &'conn Connection,
}

impl<'conn> SQLiteMigrator<'conn> {
    pub fn new(connection: &'conn Connection) -> PaceStorageResult<Self> {
        let mut migrator = Self {
            iterator: Self::load(),
            applied: VecDeque::default(),
            connection,
        };

        migrator.init()?;

        Ok(migrator)
    }

    fn init(&mut self) -> PaceStorageResult<()> {
        let migration = self.iterator.next().ok_or_eyre("No migrations found")?;
        let query = migration.up();

        self.connection.execute(&query, [])?;

        self.applied.push_back(migration);

        Ok(())
    }

    /// Load all migrations from the directory
    ///
    /// # Attention
    ///
    /// These are loaded in order, so make sure to keep the order correct.
    /// The first migration should be the migration table creation.
    fn load() -> Box<dyn Iterator<Item = Box<dyn SQLiteMigration>>> {
        let migrations: Vec<Box<dyn SQLiteMigration>> = vec![
            Box::new(create_schema_migrations::Migration),
            Box::new(create_activities_20240325143710::Migration),
            Box::new(create_activity_kinds_20240326124253::Migration),
            Box::new(create_tags_20240326125555::Migration),
            Box::new(create_activities_tags_20240326125630::Migration),
            Box::new(create_activity_status_20240326125819::Migration),
            Box::new(create_categories_20240326125937::Migration),
            Box::new(create_activities_categories_20240326130013::Migration),
            Box::new(create_activity_kind_values_20240327162015::Migration),
            Box::new(create_activity_status_values_20240327163645::Migration),
        ];

        Box::new(migrations.into_iter())
    }

    /// Push migration version to schema_migrations table
    fn push_migration_version(&self, version: String) -> PaceStorageResult<()> {
        let query = Query::insert()
            .into_table(SchemaMigrationsIden::Table)
            .columns([SchemaMigrationsIden::Guid, SchemaMigrationsIden::Version])
            .values([Ulid::new().to_string().into(), version.into()])?
            .to_owned();

        let query = query.to_string(SqliteQueryBuilder);

        self.connection.execute(&query, [])?;

        Ok(())
    }

    /// Remove migration version from schema_migrations table
    fn remove_migration_version(&self, version: String) -> PaceStorageResult<()> {
        let query = Query::delete()
            .from_table(SchemaMigrationsIden::Table)
            .cond_where(Cond::any().add(Expr::col(SchemaMigrationsIden::Version).eq(version)))
            .to_owned();

        let query = query.to_string(SqliteQueryBuilder);

        self.connection.execute(&query, [])?;

        Ok(())
    }

    /// Migrate to the latest version
    pub fn up(&mut self) -> PaceStorageResult<()> {
        while let Some(migration) = self.iterator.next() {
            let query = migration.up();

            self.connection.execute(&query, [])?;

            self.push_migration_version(migration.version())?;

            self.applied.push_back(migration);
        }

        Ok(())
    }

    /// Rollback the most recent migration
    pub fn down(&mut self) -> PaceStorageResult<String> {
        let migration = self
            .applied
            .pop_back()
            .ok_or_eyre("No migrations to rollback")?;

        let query = migration.down();

        _ = self.connection.execute(&query, [])?;

        self.remove_migration_version(migration.version())?;

        Ok(migration.version())
    }

    /// List applied migrations
    pub fn status(&self) -> PaceStorageResult<Vec<String>> {
        let applied = self.applied.iter().map(|m| m.version()).collect();

        Ok(applied)
    }
}
