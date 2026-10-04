CREATE TABLE worker_state (
    session_id TEXT PRIMARY KEY NOT NULL REFERENCES work_sessions(id),
    snapshot TEXT NOT NULL CHECK (json_valid(snapshot))
) STRICT;
PRAGMA user_version = 2;
