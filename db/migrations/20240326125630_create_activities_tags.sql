-- migrate:up
CREATE TABLE activities_tags (
    guid TEXT PRIMARY KEY,
    tag_guid TEXT NOT NULL,
    activity_guid TEXT NOT NULL,
    FOREIGN KEY (tag_guid) REFERENCES tags(guid),
    FOREIGN KEY (activity_guid) REFERENCES activities(guid)
);

-- migrate:down
DROP TABLE activities_tags;
