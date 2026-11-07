ALTER TABLE device_executions RENAME TO prior_device_executions;
CREATE TABLE device_executions (
    command_id TEXT PRIMARY KEY REFERENCES device_commands(id),
    device_id TEXT NOT NULL REFERENCES devices(id),
    execution_id TEXT NOT NULL,
    session_id TEXT,
    turn_id TEXT,
    UNIQUE(device_id, execution_id),
    UNIQUE(device_id, turn_id)
);
INSERT INTO device_executions SELECT * FROM prior_device_executions;
DROP TABLE prior_device_executions;
CREATE INDEX device_execution_sessions ON device_executions(device_id,session_id);
CREATE TABLE device_work_continuations (
    command_id TEXT PRIMARY KEY REFERENCES device_commands(id),
    root_command_id TEXT NOT NULL REFERENCES device_commands(id),
    parent_command_id TEXT NOT NULL UNIQUE REFERENCES device_commands(id),
    snapshot TEXT NOT NULL
);
PRAGMA user_version=28;
