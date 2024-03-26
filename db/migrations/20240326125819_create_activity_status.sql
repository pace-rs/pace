-- migrate:up
CREATE TABLE IF NOT EXISTS activity_status (
    guid TEXT PRIMARY KEY,
    status TEXT NOT NULL
);

-- migrate:down
DROP TABLE activity_status;
