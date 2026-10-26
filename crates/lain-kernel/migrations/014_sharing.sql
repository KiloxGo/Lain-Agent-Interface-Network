CREATE TABLE share_grants (
    channel_id TEXT NOT NULL REFERENCES channel_bindings(id),
    revision INTEGER NOT NULL,
    snapshot TEXT NOT NULL,
    PRIMARY KEY(channel_id,revision)
);
CREATE TABLE channel_attention (
    channel_id TEXT PRIMARY KEY REFERENCES channel_bindings(id),
    snapshot TEXT NOT NULL
);
CREATE TABLE share_candidates (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT UNIQUE NOT NULL,
    channel_id TEXT NOT NULL REFERENCES channel_bindings(id),
    experience_id TEXT NOT NULL REFERENCES reading_experiences(id),
    experience_revision INTEGER NOT NULL,
    state TEXT NOT NULL,
    snapshot TEXT NOT NULL,
    UNIQUE(channel_id,experience_id,experience_revision)
);
CREATE INDEX share_pending ON share_candidates(channel_id,state,sequence);
CREATE TABLE share_jobs (
    candidate_id TEXT PRIMARY KEY REFERENCES share_candidates(id),
    actor_turn_id TEXT UNIQUE NOT NULL REFERENCES lain_actor_turns(id)
);
CREATE TABLE share_model_requests (
    actor_turn_id TEXT NOT NULL REFERENCES share_jobs(actor_turn_id),
    request_number INTEGER NOT NULL,
    channel_id TEXT NOT NULL,
    grant_revision INTEGER NOT NULL,
    prepared_at_ms INTEGER NOT NULL,
    PRIMARY KEY(actor_turn_id,request_number)
);
CREATE INDEX share_model_budget ON share_model_requests(channel_id,grant_revision);
CREATE TABLE share_deliveries (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT UNIQUE NOT NULL,
    candidate_id TEXT UNIQUE NOT NULL REFERENCES share_candidates(id),
    channel_id TEXT NOT NULL,
    state TEXT NOT NULL,
    snapshot TEXT NOT NULL
);
CREATE INDEX share_contact_history ON share_deliveries(channel_id,sequence DESC);
CREATE TABLE sharing_runtime (
    singleton INTEGER PRIMARY KEY CHECK(singleton=1),
    actor_watermark INTEGER NOT NULL
);
INSERT INTO sharing_runtime SELECT 1,COALESCE(MAX(sequence),0) FROM lain_actor_turns;
PRAGMA user_version=14;
