CREATE TABLE work_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    snapshot TEXT NOT NULL CHECK (json_valid(snapshot))
) STRICT;
PRAGMA user_version = 1;

