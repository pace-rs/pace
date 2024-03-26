-- migrate:up
CREATE TABLE tags (
    guid TEXT PRIMARY KEY,
    tag TEXT NOT NULL
);

-- migrate:down
DROP TABLE tags;