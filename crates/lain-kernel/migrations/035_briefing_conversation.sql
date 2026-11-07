-- The durable run owns coordination; foreground disclosure is independently scoped.
CREATE TABLE briefing_runs (
    briefing_id TEXT PRIMARY KEY REFERENCES briefing_manifests(id),
    snapshot TEXT NOT NULL
);
CREATE TABLE briefing_disclosures (
    actor_turn_id TEXT PRIMARY KEY REFERENCES lain_actor_turns(id),
    snapshot TEXT NOT NULL
);
PRAGMA user_version = 35;
