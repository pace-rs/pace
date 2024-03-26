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
    parent_guid TEXT NULL,
    FOREIGN KEY (kind) REFERENCES activity_kinds(guid),
    FOREIGN KEY (status) REFERENCES activity_status(guid),
    FOREIGN KEY (parent_guid) REFERENCES activities(guid)
);

-- migrate:down
DROP TABLE activities;
