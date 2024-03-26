-- migrate:up
CREATE TABLE IF NOT EXISTS categories (
    guid TEXT PRIMARY KEY,
    category TEXT NOT NULL,
    description TEXT NULL
);

-- migrate:down
DROP TABLE categories;
