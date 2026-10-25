CREATE TABLE channel_memory_grants (
    channel_id TEXT NOT NULL REFERENCES channel_bindings(id),
    revision INTEGER NOT NULL,
    snapshot TEXT NOT NULL CHECK(length(CAST(snapshot AS BLOB)) <= 4096),
    PRIMARY KEY(channel_id, revision)
);
CREATE TABLE channel_memory_effects (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    action_id TEXT NOT NULL UNIQUE,
    channel_id TEXT NOT NULL REFERENCES channel_bindings(id),
    snapshot TEXT NOT NULL CHECK(length(CAST(snapshot AS BLOB)) <= 4096)
);
CREATE INDEX channel_memory_effect_history ON channel_memory_effects(channel_id,sequence);
CREATE TABLE channel_memory_suppressions (
    channel_id TEXT NOT NULL REFERENCES channel_bindings(id),
    incoming_id TEXT NOT NULL,
    PRIMARY KEY(channel_id, incoming_id)
);
CREATE TABLE channel_memory_inspections (
    actor_turn_id TEXT NOT NULL,
    memory_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    PRIMARY KEY(actor_turn_id, memory_id)
);
PRAGMA user_version = 9;
