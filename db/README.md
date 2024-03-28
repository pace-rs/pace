# Database

This directory contains the database schema and migrations for the application.
It also contains the development database used for testing and development.

## Database Connection

The database connection string is read from the `DATABASE_URL` environment
variable. This variable should be set to the connection string for the database.
For example:

```console
export DATABASE_URL="sqlite:./db/db.sqlite3"
```
