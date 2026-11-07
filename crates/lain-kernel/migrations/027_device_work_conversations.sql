CREATE TABLE device_work_links (
 command_id TEXT PRIMARY KEY REFERENCES device_commands(id),
 device_id TEXT NOT NULL REFERENCES devices(id),
 conversation_id TEXT NOT NULL REFERENCES lain_conversations(id),
 created_at_ms INTEGER NOT NULL,
 snapshot TEXT NOT NULL
);
CREATE INDEX device_work_conversation ON device_work_links(conversation_id,created_at_ms DESC,command_id DESC);
PRAGMA user_version=27;
