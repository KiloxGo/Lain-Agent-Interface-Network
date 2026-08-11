CREATE TABLE schema_metadata (
    schema_version INTEGER PRIMARY KEY,
    app_version TEXT NOT NULL,
    applied_at TEXT NOT NULL
);

CREATE TABLE instances (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    identity_version TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'inactive')),
    settings_json TEXT NOT NULL CHECK (json_valid(settings_json)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX one_active_instance ON instances(status) WHERE status = 'active';

CREATE TABLE users (
    id TEXT PRIMARY KEY,
    instance_id TEXT NOT NULL REFERENCES instances(id) ON DELETE CASCADE,
    display_name TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX users_instance_updated ON users(instance_id, updated_at DESC);

CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    instance_id TEXT NOT NULL REFERENCES instances(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT,
    status TEXT NOT NULL CHECK (status IN ('active', 'archived', 'deleted')),
    last_seq INTEGER NOT NULL DEFAULT 0 CHECK (last_seq >= 0),
    summary TEXT,
    summary_version INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX sessions_instance_updated ON sessions(instance_id, updated_at DESC);
CREATE INDEX sessions_user_updated ON sessions(user_id, updated_at DESC);

CREATE TABLE runs (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    trigger_message_id TEXT NOT NULL REFERENCES messages(id) DEFERRABLE INITIALLY DEFERRED,
    state TEXT NOT NULL CHECK (state IN ('queued', 'running', 'cancelling', 'completed', 'failed', 'cancelled')),
    mode TEXT NOT NULL CHECK (mode IN ('chat', 'task', 'reflection', 'maintenance')),
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    identity_version TEXT NOT NULL,
    context_plan_id TEXT REFERENCES context_plans(id) DEFERRABLE INITIALLY DEFERRED,
    turn_count INTEGER NOT NULL DEFAULT 0,
    deadline_at TEXT,
    started_at TEXT,
    completed_at TEXT,
    error_code TEXT,
    error_detail_redacted TEXT,
    usage_input_tokens INTEGER,
    usage_output_tokens INTEGER,
    usage_cached_tokens INTEGER,
    estimated_cost REAL,
    created_at TEXT NOT NULL
);

CREATE INDEX runs_session_started ON runs(session_id, started_at DESC);
CREATE INDEX runs_state_started ON runs(state, started_at);

CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    run_id TEXT REFERENCES runs(id) ON DELETE SET NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'tool', 'system_projection')),
    content TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('complete', 'incomplete', 'redacted', 'deleted')),
    created_at TEXT NOT NULL,
    client_message_id TEXT,
    provider_message_id TEXT,
    token_count INTEGER,
    metadata_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(metadata_json))
);

CREATE UNIQUE INDEX messages_client_idempotency
    ON messages(session_id, client_message_id)
    WHERE client_message_id IS NOT NULL;
CREATE INDEX messages_session_created ON messages(session_id, created_at);
CREATE INDEX messages_run ON messages(run_id);

CREATE TABLE events (
    event_id TEXT PRIMARY KEY,
    instance_id TEXT NOT NULL REFERENCES instances(id) ON DELETE CASCADE,
    session_id TEXT REFERENCES sessions(id) ON DELETE CASCADE,
    run_id TEXT REFERENCES runs(id) ON DELETE CASCADE,
    seq INTEGER,
    event_type TEXT NOT NULL,
    schema_version INTEGER NOT NULL CHECK (schema_version > 0),
    payload_json TEXT NOT NULL CHECK (json_valid(payload_json)),
    created_at TEXT NOT NULL,
    CHECK ((session_id IS NULL AND seq IS NULL) OR (session_id IS NOT NULL AND seq IS NOT NULL AND seq > 0))
);

CREATE UNIQUE INDEX events_session_seq
    ON events(session_id, seq)
    WHERE session_id IS NOT NULL;
CREATE INDEX events_run ON events(run_id, created_at);

CREATE TABLE context_plans (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL UNIQUE REFERENCES runs(id) ON DELETE CASCADE,
    identity_version TEXT NOT NULL,
    compiler_version TEXT NOT NULL,
    prompt_hash TEXT NOT NULL,
    selected_items_json TEXT NOT NULL CHECK (json_valid(selected_items_json)),
    dropped_items_json TEXT NOT NULL CHECK (json_valid(dropped_items_json)),
    estimated_tokens INTEGER NOT NULL CHECK (estimated_tokens >= 0),
    created_at TEXT NOT NULL
);

CREATE TABLE durable_jobs (
    id TEXT PRIMARY KEY,
    job_type TEXT NOT NULL,
    payload_json TEXT NOT NULL CHECK (json_valid(payload_json)),
    status TEXT NOT NULL CHECK (status IN ('queued', 'leased', 'completed', 'failed', 'dead_letter')),
    priority INTEGER NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    max_attempts INTEGER NOT NULL CHECK (max_attempts > 0),
    available_at TEXT NOT NULL,
    lease_owner TEXT,
    lease_expires_at TEXT,
    last_error_code TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX durable_jobs_claim
    ON durable_jobs(status, available_at, priority DESC);

INSERT INTO schema_metadata(schema_version, app_version, applied_at)
VALUES (1, '0.1.0', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
