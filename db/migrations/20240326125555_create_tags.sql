-- migrate:up
CREATE TABLE IF NOT EXISTS tags (
    guid TEXT PRIMARY KEY,
    tag TEXT NOT NULL
);

-- migrate:down
DROP TABLE tags;