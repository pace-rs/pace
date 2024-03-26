-- migrate:up
CREATE TABLE activity_status (
    guid TEXT PRIMARY KEY,
    status TEXT NOT NULL
);

-- migrate:down
DROP TABLE activity_status;
