CREATE TABLE reading_jobs (
    activity_id TEXT PRIMARY KEY REFERENCES reading_activities(id),
    actor_turn_id TEXT UNIQUE NOT NULL REFERENCES lain_actor_turns(id),
    grant_revision INTEGER NOT NULL,
    source_fingerprint TEXT NOT NULL,
    input_bytes INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL,
    cancel_requested_at_ms INTEGER
);
CREATE TABLE reading_model_requests (
    actor_turn_id TEXT NOT NULL REFERENCES reading_jobs(actor_turn_id),
    request_number INTEGER NOT NULL,
    grant_revision INTEGER NOT NULL,
    prepared_at_ms INTEGER NOT NULL,
    PRIMARY KEY(actor_turn_id, request_number)
);
CREATE INDEX reading_model_budget ON reading_model_requests(grant_revision);
CREATE TABLE reading_experiences (
    id TEXT PRIMARY KEY,
    activity_id TEXT UNIQUE NOT NULL REFERENCES reading_activities(id),
    completed_at_ms INTEGER NOT NULL,
    active INTEGER NOT NULL,
    snapshot TEXT NOT NULL
);
CREATE INDEX reading_experience_time ON reading_experiences(completed_at_ms DESC, id);
CREATE INDEX reading_experience_grant_time ON reading_experiences(json_extract(snapshot,'$.grant_revision'),completed_at_ms DESC,id);
CREATE INDEX reading_experience_active ON reading_experiences(active);
CREATE TABLE reading_withdrawals (
    activity_id TEXT PRIMARY KEY REFERENCES reading_activities(id),
    recorded_at_ms INTEGER NOT NULL
);
CREATE TABLE experience_reads (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    actor_turn_id TEXT NOT NULL REFERENCES lain_actor_turns(id),
    action_id TEXT,
    model_request INTEGER,
    grant_revision INTEGER NOT NULL,
    experience_ids TEXT NOT NULL,
    recorded_at_ms INTEGER NOT NULL
);
CREATE TABLE reading_source_suppressions (
    url TEXT PRIMARY KEY,
    recorded_at_ms INTEGER NOT NULL
);
CREATE INDEX reading_source_url ON reading_activities(json_extract(snapshot,'$.request.url'));
CREATE TABLE experience_edits (
    experience_id TEXT NOT NULL REFERENCES reading_experiences(id),
    revision INTEGER NOT NULL,
    operation TEXT NOT NULL,
    recorded_at_ms INTEGER NOT NULL,
    PRIMARY KEY(experience_id,revision)
);
CREATE TABLE reading_source_decisions (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL,
    operation TEXT NOT NULL,
    recorded_at_ms INTEGER NOT NULL
);
PRAGMA user_version=13;
