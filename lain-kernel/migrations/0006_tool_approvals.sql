DROP INDEX tool_invocations_run_started;
ALTER TABLE tool_invocations RENAME TO tool_invocations_v5;

CREATE TABLE tool_invocations (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    tool_call_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    risk TEXT NOT NULL CHECK (risk IN ('read_only', 'workspace_write')),
    arguments_json TEXT NOT NULL CHECK (json_valid(arguments_json)),
    status TEXT NOT NULL CHECK (status IN ('running', 'completed', 'failed')),
    result_text TEXT,
    error_code TEXT,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    UNIQUE(run_id, tool_call_id)
);

INSERT INTO tool_invocations
SELECT * FROM tool_invocations_v5;
DROP TABLE tool_invocations_v5;
CREATE INDEX tool_invocations_run_started ON tool_invocations(run_id, started_at);

CREATE TABLE tool_approvals (
    id TEXT PRIMARY KEY,
    invocation_id TEXT NOT NULL UNIQUE REFERENCES tool_invocations(id) ON DELETE CASCADE,
    run_id TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    tool_call_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    action_hash TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('requested', 'approved', 'rejected', 'cancelled')),
    requested_at TEXT NOT NULL,
    decided_at TEXT
);

CREATE INDEX tool_approvals_run_requested ON tool_approvals(run_id, requested_at);

INSERT INTO schema_metadata(schema_version, app_version, applied_at)
VALUES (6, '0.1.0', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
