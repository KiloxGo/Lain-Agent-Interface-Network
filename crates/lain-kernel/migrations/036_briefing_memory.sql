-- Version gate for conversation-local reviewed memory; grants remain unchanged.
CREATE TABLE briefing_memory_suppressions (
    conversation_id TEXT NOT NULL REFERENCES lain_conversations(id),
    record_id TEXT NOT NULL,
    PRIMARY KEY (conversation_id, record_id)
);
PRAGMA user_version = 36;
