-- This adds no inferred projects, mappings, memories or permissions.
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    snapshot TEXT NOT NULL CHECK (json_valid(snapshot))
);
CREATE TABLE project_workspaces (
    device_id TEXT NOT NULL REFERENCES devices(id),
    workspace_id TEXT NOT NULL,
    project_id TEXT REFERENCES projects(id),
    snapshot TEXT NOT NULL CHECK (json_valid(snapshot)),
    PRIMARY KEY (device_id, workspace_id)
);
CREATE TABLE project_requests (
    request_id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('create', 'change', 'workspace')),
    target TEXT NOT NULL,
    payload_hash TEXT NOT NULL CHECK (length(payload_hash) = 64),
    snapshot TEXT NOT NULL CHECK (json_valid(snapshot))
);
PRAGMA user_version = 29;
