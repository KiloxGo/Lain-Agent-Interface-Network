CREATE TABLE reading_grants (
    revision INTEGER PRIMARY KEY,
    snapshot TEXT NOT NULL
);
CREATE TABLE reading_activities (
    id TEXT PRIMARY KEY,
    request_id TEXT UNIQUE NOT NULL,
    grant_revision INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL,
    unfinished INTEGER NOT NULL,
    snapshot TEXT NOT NULL
);
CREATE INDEX reading_by_revision ON reading_activities(grant_revision);
CREATE INDEX reading_by_creation ON reading_activities(created_at_ms DESC, id);
CREATE INDEX reading_unfinished ON reading_activities(unfinished);
CREATE TABLE reading_results (
    activity_id TEXT PRIMARY KEY REFERENCES reading_activities(id),
    text TEXT NOT NULL
);
PRAGMA user_version = 12;
