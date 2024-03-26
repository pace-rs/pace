CREATE TABLE IF NOT EXISTS "schema_migrations" (version varchar(128) primary key);
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
CREATE TABLE activity_kinds (
    guid TEXT PRIMARY KEY,
    kind TEXT NOT NULL
);
CREATE TABLE tags (
    guid TEXT PRIMARY KEY,
    tag TEXT NOT NULL
);
CREATE TABLE activities_tags (
    guid TEXT PRIMARY KEY,
    tag_guid TEXT NOT NULL,
    activity_guid TEXT NOT NULL,
    FOREIGN KEY (tag_guid) REFERENCES tags(guid),
    FOREIGN KEY (activity_guid) REFERENCES activities(guid)
);
CREATE TABLE activity_status (
    guid TEXT PRIMARY KEY,
    status TEXT NOT NULL
);
CREATE TABLE categories (
    guid TEXT PRIMARY KEY,
    category TEXT NOT NULL,
    description TEXT NULL
);
CREATE TABLE activities_categories (
    guid TEXT PRIMARY KEY,
    category_guid TEXT NOT NULL,
    activity_guid TEXT NOT NULL,
    FOREIGN KEY (category_guid) REFERENCES categories(guid),
    FOREIGN KEY (activity_guid) REFERENCES activities(guid)
);
-- Dbmate schema migrations
INSERT INTO "schema_migrations" (version) VALUES
  ('20240325143710'),
  ('20240326124253'),
  ('20240326125555'),
  ('20240326125630'),
  ('20240326125819'),
  ('20240326125937'),
  ('20240326130013');
