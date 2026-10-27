CREATE TABLE archive_recall_grants (
    channel_id TEXT NOT NULL REFERENCES channel_bindings(id),
    revision INTEGER NOT NULL,
    snapshot TEXT NOT NULL,
    PRIMARY KEY(channel_id,revision)
);
CREATE TABLE archive_recall_reads (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    channel_id TEXT NOT NULL REFERENCES channel_bindings(id),
    actor_turn_id TEXT NOT NULL REFERENCES lain_actor_turns(id),
    action_id TEXT,
    model_request INTEGER,
    snapshot TEXT NOT NULL
);
CREATE INDEX archive_recall_channel ON archive_recall_reads(channel_id,sequence);
PRAGMA user_version=19;
