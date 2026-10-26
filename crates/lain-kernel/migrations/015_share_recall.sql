CREATE TABLE share_reads (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    actor_turn_id TEXT NOT NULL REFERENCES lain_actor_turns(id),
    action_id TEXT,
    model_request INTEGER,
    channel_id TEXT NOT NULL REFERENCES channel_bindings(id),
    delivery_ids TEXT NOT NULL,
    versions TEXT NOT NULL,
    prepared_at_ms INTEGER NOT NULL
);
CREATE INDEX share_read_actor ON share_reads(actor_turn_id,sequence);
CREATE INDEX share_read_channel ON share_reads(channel_id,sequence DESC);
PRAGMA user_version=15;
