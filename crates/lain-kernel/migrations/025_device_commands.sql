CREATE TABLE device_commands (
 id TEXT PRIMARY KEY,
 device_id TEXT NOT NULL REFERENCES devices(id),
 request_id TEXT NOT NULL,
 snapshot TEXT NOT NULL,
 UNIQUE(device_id, request_id)
);
CREATE TABLE device_command_events (
 command_id TEXT NOT NULL REFERENCES device_commands(id),
 sequence INTEGER NOT NULL,
 request_id TEXT NOT NULL,
 snapshot TEXT NOT NULL,
 PRIMARY KEY(command_id, sequence),
 UNIQUE(command_id, request_id)
);
CREATE TABLE device_command_changes (
 device_id TEXT NOT NULL REFERENCES devices(id),
 sequence INTEGER NOT NULL,
 snapshot TEXT NOT NULL,
 PRIMARY KEY(device_id, sequence)
);
PRAGMA user_version=25;
