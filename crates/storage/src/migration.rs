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

use rusqlite::Connection;
use sea_query::{Cond, Expr, Iden, Query, SqliteQueryBuilder};
use std::collections::VecDeque;
use ulid::Ulid;

use pace_error::{DatabaseStorageErrorKind, PaceResult};

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
    /// Create a new `SQLiteMigrator`
    ///
    /// # Arguments
    ///
    /// * `connection` - The [`rusqlite::Connection`]
    ///
    /// # Errors
    ///
    /// Returns an error if the initial migration fails
    ///
    /// # Returns
    ///
    /// Returns a new `SQLiteMigrator`
    pub fn new(connection: &'conn Connection) -> PaceResult<Self> {
        let mut migrator = Self {
            iterator: Self::load(),
            applied: VecDeque::default(),
            connection,
        };

        migrator.init()?;

        Ok(migrator)
    }

    fn init(&mut self) -> PaceResult<()> {
        let migration =
            self.iterator
                .next()
                .ok_or(DatabaseStorageErrorKind::NoMigrationsFound {
                    table: SchemaMigrationsIden::Table.to_string(),
                })?;
        let query = migration.up();

        if self.connection.execute(&query, []).map_err(|source| {
            DatabaseStorageErrorKind::MigrationFailed {
                version: migration.version(),
                query: query.to_string(),
                source,
            }
        })? > 1
        {
            return Err(DatabaseStorageErrorKind::MigrationAffectedMultipleRows.into());
        }

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

    /// Push migration version to `schema_migrations` table
    ///
    /// # Errors
    ///
    /// Returns an error if the migration version cannot be pushed
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the migration version is pushed successfully
    fn push_migration_version(&self, version: String) -> PaceResult<()> {
        let query = Query::insert()
            .into_table(SchemaMigrationsIden::Table)
            .columns([SchemaMigrationsIden::Guid, SchemaMigrationsIden::Version])
            .values([Ulid::new().to_string().into(), version.clone().into()])
            .map_err(
                |source| DatabaseStorageErrorKind::BuildingMigrationQueryFailed {
                    version: version.clone(),
                    table: SchemaMigrationsIden::Table.to_string(),
                    query: source.to_string(),
                    source,
                },
            )?
            .to_owned();

        let query = query.to_string(SqliteQueryBuilder);

        if self.connection.execute(&query, []).map_err(|err| {
            DatabaseStorageErrorKind::MigrationFailed {
                version,
                query,
                source: err,
            }
        })? > 1
        {
            return Err(DatabaseStorageErrorKind::MigrationAffectedMultipleRows.into());
        }

        Ok(())
    }

    /// Remove migration version from `schema_migrations` table
    ///
    /// # Errors
    ///
    /// Returns an error if the migration version cannot be removed
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the migration version is removed successfully
    fn remove_migration_version(&self, version: String) -> PaceResult<()> {
        let query = Query::delete()
            .from_table(SchemaMigrationsIden::Table)
            .cond_where(
                Cond::any().add(Expr::col(SchemaMigrationsIden::Version).eq(version.clone())),
            )
            .to_owned();

        let query = query.to_string(SqliteQueryBuilder);

        if self.connection.execute(&query, []).map_err(|source| {
            DatabaseStorageErrorKind::MigrationFailed {
                version,
                query,
                source,
            }
        })? > 1
        {
            return Err(DatabaseStorageErrorKind::MigrationAffectedMultipleRows.into());
        }

        Ok(())
    }

    /// Migrate to the latest version
    ///
    /// # Errors
    ///
    /// Returns an error if the migration fails
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the migration is successful
    pub fn up(&mut self) -> PaceResult<()> {
        while let Some(migration) = self.iterator.next() {
            let query = migration.up();

            if self.connection.execute(&query, []).map_err(|source| {
                DatabaseStorageErrorKind::MigrationFailed {
                    version: migration.version(),
                    query: query.to_string(),
                    source,
                }
            })? > 1
            {
                return Err(DatabaseStorageErrorKind::MigrationAffectedMultipleRows.into());
            }

            self.push_migration_version(migration.version())?;

            self.applied.push_back(migration);
        }

        Ok(())
    }

    /// Rollback the most recent migration
    ///
    /// # Errors
    ///
    /// Returns an error if there are no migrations to rollback
    ///
    /// # Returns
    ///
    /// Returns the version of the migration that was rolled back
    pub fn down(&mut self) -> PaceResult<String> {
        let migration = self
            .applied
            .pop_back()
            .ok_or(DatabaseStorageErrorKind::NoMigrationsToRollback)?;

        let query = migration.down();

        _ = self.connection.execute(&query, []).map_err(|source| {
            DatabaseStorageErrorKind::MigrationFailed {
                version: migration.version(),
                query: query.to_string(),
                source,
            }
        })?;

        self.remove_migration_version(migration.version())?;

        Ok(migration.version())
    }

    /// List applied migrations
    #[must_use]
    pub fn status(&self) -> Vec<String> {
        self.applied.iter().map(|m| m.version()).collect()
    }
}