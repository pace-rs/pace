-- migrate:up
CREATE TABLE activities (
    guid TEXT PRIMARY KEY,
    category TEXT NOT NULL,
    description TEXT NOT NULL,
    begin TEXT NOT NULL,
    end TEXT NULL,
    duration INTEGER NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    tags TEXT NULL,
    parent_guid TEXT NULL,
    FOREIGN KEY (parent_id) REFERENCES activities(guid)
);

-- migrate:down
DROP TABLE activities;
