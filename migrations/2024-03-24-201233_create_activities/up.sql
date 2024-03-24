-- Migration script for creating the activities table in SQLite

CREATE TABLE activities (
    id TEXT PRIMARY KEY,
    category TEXT NOT NULL,
    description TEXT NOT NULL,
    begin TEXT NOT NULL,
    end TEXT NOT NULL,
    duration INTEGER NOT NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    parent_id INTEGER,
    FOREIGN KEY (parent_id) REFERENCES activities(id)
);
