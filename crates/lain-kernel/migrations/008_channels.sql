CREATE TABLE channel_bindings (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    snapshot TEXT NOT NULL CHECK(length(CAST(snapshot AS BLOB)) <= 4096)
);
CREATE TABLE channel_incoming (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    channel_id TEXT NOT NULL REFERENCES channel_bindings(id),
    external_id TEXT NOT NULL,
    state TEXT NOT NULL,
    snapshot TEXT NOT NULL CHECK(length(CAST(snapshot AS BLOB)) <= 65536),
    UNIQUE(channel_id, external_id)
);
CREATE INDEX channel_incoming_queue ON channel_incoming(channel_id, state, sequence);
CREATE TABLE channel_batches (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    channel_id TEXT NOT NULL REFERENCES channel_bindings(id),
    state TEXT NOT NULL,
    snapshot TEXT NOT NULL CHECK(length(CAST(snapshot AS BLOB)) <= 65536)
);
CREATE INDEX channel_batches_active ON channel_batches(channel_id, state, sequence);
CREATE TABLE channel_deliveries (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    channel_id TEXT NOT NULL REFERENCES channel_bindings(id),
    batch_id TEXT NOT NULL UNIQUE REFERENCES channel_batches(id),
    state TEXT NOT NULL,
    snapshot TEXT NOT NULL CHECK(length(CAST(snapshot AS BLOB)) <= 131072)
);
CREATE INDEX channel_deliveries_ready ON channel_deliveries(channel_id, state, sequence);
PRAGMA user_version = 8;
