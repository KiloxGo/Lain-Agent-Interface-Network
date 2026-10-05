CREATE TABLE operational_history (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE,
    session_id TEXT NOT NULL REFERENCES work_sessions(id),
    turn_id TEXT,
    timestamp_ms INTEGER NOT NULL,
    kind TEXT NOT NULL,
    payload TEXT NOT NULL CHECK(length(CAST(payload AS BLOB)) <= 262144)
);
CREATE INDEX history_session_order ON operational_history(session_id, sequence);
CREATE INDEX history_turn_order ON operational_history(turn_id, sequence);
CREATE INDEX history_timestamp ON operational_history(timestamp_ms, sequence);
CREATE INDEX history_kind_order ON operational_history(kind, sequence);
PRAGMA user_version = 3;
