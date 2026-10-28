CREATE TABLE archive_memory_grants (
    channel_id TEXT NOT NULL REFERENCES channel_bindings(id),
    revision INTEGER NOT NULL,
    snapshot TEXT NOT NULL,
    PRIMARY KEY(channel_id, revision)
);
CREATE TABLE archive_memory_suppressions (
    channel_id TEXT NOT NULL REFERENCES channel_bindings(id),
    record_id TEXT NOT NULL REFERENCES archive_records(id),
    PRIMARY KEY(channel_id, record_id)
);
PRAGMA user_version = 22;
