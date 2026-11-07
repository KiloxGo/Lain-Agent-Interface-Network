-- Tool-free discovery and exact pair judgments, owned by the existing Actor.
CREATE TABLE briefing_relations (
    briefing_id TEXT NOT NULL REFERENCES briefing_manifests(id),
    step INTEGER NOT NULL CHECK(step>=0 AND step<=256),
    actor_turn_id TEXT NOT NULL UNIQUE REFERENCES lain_actor_turns(id),
    input_fingerprint TEXT NOT NULL,
    result_snapshot TEXT,
    PRIMARY KEY(briefing_id,step)
);
PRAGMA user_version=39;
