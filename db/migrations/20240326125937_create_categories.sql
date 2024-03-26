-- migrate:up
CREATE TABLE categories (
    guid TEXT PRIMARY KEY,
    category TEXT NOT NULL,
    description TEXT NULL
);

-- migrate:down
DROP TABLE categories;
