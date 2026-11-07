-- Legacy memory bodies and origins are deliberately untouched.
CREATE TABLE conversation_memory_contexts (
    conversation_id TEXT PRIMARY KEY REFERENCES lain_conversations(id),
    snapshot TEXT NOT NULL CHECK (json_valid(snapshot))
);
CREATE TABLE memory_context_requests (
    request_id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES lain_conversations(id),
    payload_hash TEXT NOT NULL CHECK (length(payload_hash) = 64),
    snapshot TEXT NOT NULL CHECK (json_valid(snapshot))
);
CREATE TABLE actor_memory_contexts (
    actor_turn_id TEXT PRIMARY KEY REFERENCES lain_actor_turns(id),
    snapshot TEXT NOT NULL CHECK (json_valid(snapshot))
);
PRAGMA user_version = 30;
