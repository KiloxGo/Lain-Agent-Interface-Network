CREATE TABLE device_executions (
    command_id TEXT PRIMARY KEY REFERENCES device_commands(id),
    device_id TEXT NOT NULL REFERENCES devices(id),
    execution_id TEXT NOT NULL,
    session_id TEXT,
    turn_id TEXT,
    UNIQUE(device_id, execution_id),
    UNIQUE(device_id, session_id),
    UNIQUE(device_id, turn_id)
);
CREATE TABLE device_approval_decisions (
 command_id TEXT NOT NULL REFERENCES device_commands(id),
 approval_id TEXT NOT NULL,
 request_id TEXT NOT NULL,
 snapshot TEXT NOT NULL,
 PRIMARY KEY(command_id,approval_id),
 UNIQUE(command_id,request_id)
);
PRAGMA user_version=26;
