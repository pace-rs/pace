-- migrate:up
CREATE TABLE IF NOT EXISTS activity_kinds (
    guid TEXT PRIMARY KEY,
    kind TEXT NOT NULL
);

-- migrate:down
DROP TABLE activity_kinds;