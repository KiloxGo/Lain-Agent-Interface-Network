-- Operational identity and invalidation, not a new Actor or semantic-memory copy.
CREATE TABLE IF NOT EXISTS client_sync_state (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    instance_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision >= 0 AND revision < 9223372036854775807)
);
CREATE TABLE IF NOT EXISTS client_sync_deadlines (
    source_table TEXT NOT NULL,
    source_rowid INTEGER NOT NULL,
    deadline_ms INTEGER NOT NULL CHECK (deadline_ms > 0),
    PRIMARY KEY (source_table, source_rowid, deadline_ms)
);
CREATE INDEX IF NOT EXISTS client_sync_next_deadline ON client_sync_deadlines(deadline_ms);
PRAGMA user_version = 32;
