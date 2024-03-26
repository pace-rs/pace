-- migrate:up
CREATE TABLE activities_categories (
    guid TEXT PRIMARY KEY,
    category_guid TEXT NOT NULL,
    activity_guid TEXT NOT NULL,
    FOREIGN KEY (category_guid) REFERENCES categories(guid),
    FOREIGN KEY (activity_guid) REFERENCES activities(guid)
);

-- migrate:down
DROP TABLE activities_categories;
