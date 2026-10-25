CREATE TABLE channel_source_grants (
    channel_id TEXT NOT NULL REFERENCES channel_bindings(id),
    revision INTEGER NOT NULL,
    snapshot TEXT NOT NULL CHECK(length(CAST(snapshot AS BLOB)) <= 4096),
    PRIMARY KEY(channel_id,revision)
);
CREATE TABLE channel_source_reads (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    channel_id TEXT NOT NULL REFERENCES channel_bindings(id),
    actor_turn_id TEXT,
    action_id TEXT,
    model_request INTEGER,
    operation TEXT NOT NULL,
    snapshot TEXT NOT NULL CHECK(length(CAST(snapshot AS BLOB)) <= 4096),
    UNIQUE(action_id,operation),
    UNIQUE(actor_turn_id,model_request,operation)
);
CREATE INDEX channel_source_read_history ON channel_source_reads(channel_id,sequence);
CREATE INDEX channel_source_window ON channel_incoming(channel_id,CAST(json_extract(snapshot,'$.received_at_ms') AS INTEGER),sequence);
PRAGMA user_version=10;
