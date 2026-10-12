CREATE TABLE lain_conversations (
    id TEXT PRIMARY KEY NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);
CREATE TABLE lain_actor_turns (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT UNIQUE NOT NULL,
    conversation_id TEXT NOT NULL REFERENCES lain_conversations(id),
    request_id TEXT NOT NULL,
    snapshot TEXT NOT NULL CHECK(length(CAST(snapshot AS BLOB)) <= 262144),
    UNIQUE(conversation_id, request_id)
);
CREATE INDEX lain_actor_turns_conversation ON lain_actor_turns(conversation_id, sequence);
PRAGMA user_version = 5;
