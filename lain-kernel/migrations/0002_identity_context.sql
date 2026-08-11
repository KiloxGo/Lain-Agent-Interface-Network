CREATE TABLE identity_versions (
    version TEXT PRIMARY KEY,
    manifest_json TEXT NOT NULL CHECK (json_valid(manifest_json)),
    content_hash TEXT NOT NULL UNIQUE,
    activated_at TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE state_snapshots (
    id TEXT PRIMARY KEY,
    instance_id TEXT NOT NULL REFERENCES instances(id) ON DELETE CASCADE,
    run_id TEXT REFERENCES runs(id) ON DELETE SET NULL,
    state_type TEXT NOT NULL CHECK (state_type IN ('self', 'relationship')),
    state_json TEXT NOT NULL CHECK (json_valid(state_json)),
    version INTEGER NOT NULL CHECK (version > 0),
    created_at TEXT NOT NULL
);

CREATE INDEX state_snapshots_latest
    ON state_snapshots(instance_id, state_type, version DESC, created_at DESC);

ALTER TABLE context_plans ADD COLUMN stable_prefix_hash TEXT NOT NULL DEFAULT '';
ALTER TABLE context_plans ADD COLUMN estimated INTEGER NOT NULL DEFAULT 1 CHECK (estimated IN (0, 1));
