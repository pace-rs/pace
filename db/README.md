# Database

This directory contains the database schema and migrations for the application.
It also contains the development database used for testing and development.

## Migrations

This directory contains all the migrations for the database. Migrations are used
to manage the database schema and are run in order to update the database to the
latest schema version.

## Creating a Migration

To create a new migration, run the following command:

```console
dbmate new <migration_name>
```

This will create a new migration file in the `migrations` directory with the
name `<timestamp>_<migration_name>.sql`.

## Running Migrations

To run all pending migrations, run the following command:

```console
dbmate up
```

To rollback the last migration, run the following command:

```console
dbmate down
```

To rollback all migrations, run the following command:

```console
dbmate drop
```

For more information on the `dbmate` CLI, run the following command:

```console
dbmate help
```

## Database Connection

The database connection string is read from the `DATABASE_URL` environment
variable. This variable should be set to the connection string for the database.
For example:

```console
export DATABASE_URL="sqlite:./db/db.sqlite3"
```
