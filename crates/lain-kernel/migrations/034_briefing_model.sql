-- One finite Actor turn for a specific manifest page. Uncertain turns are never replayed.
CREATE TABLE briefing_batches (
    briefing_id TEXT NOT NULL REFERENCES briefing_manifests(id),
    from_index INTEGER NOT NULL CHECK (from_index >= 0),
    actor_turn_id TEXT NOT NULL UNIQUE REFERENCES lain_actor_turns(id),
    next_index INTEGER,
    input_fingerprint TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    result_snapshot TEXT,
    PRIMARY KEY (briefing_id,from_index)
);
PRAGMA user_version = 34;
