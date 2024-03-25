CREATE TABLE IF NOT EXISTS "schema_migrations" (version varchar(128) primary key);
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
-- Dbmate schema migrations
INSERT INTO "schema_migrations" (version) VALUES
  ('20240325143710');
