-- migrate:up
CREATE TABLE activities (
    id TEXT PRIMARY KEY,
    category TEXT NOT NULL,
    description TEXT NOT NULL,
    begin TEXT NOT NULL,
    end TEXT NULL,
    duration INTEGER NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    parent_id TEXT NULL,
    FOREIGN KEY (parent_id) REFERENCES activities(id)
);

-- migrate:down
DROP TABLE activities;
