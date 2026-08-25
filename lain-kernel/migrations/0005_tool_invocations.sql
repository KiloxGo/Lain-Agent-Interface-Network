CREATE TABLE tool_invocations (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    tool_call_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    risk TEXT NOT NULL CHECK (risk IN ('read_only')),
    arguments_json TEXT NOT NULL CHECK (json_valid(arguments_json)),
    status TEXT NOT NULL CHECK (status IN ('running', 'completed', 'failed')),
    result_text TEXT,
    error_code TEXT,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    UNIQUE(run_id, tool_call_id)
);

CREATE INDEX tool_invocations_run_started ON tool_invocations(run_id, started_at);

INSERT INTO schema_metadata(schema_version, app_version, applied_at)
VALUES (5, '0.1.0', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
