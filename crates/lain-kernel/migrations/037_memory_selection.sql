CREATE TABLE conversation_memory_effects (
    action_id TEXT PRIMARY KEY NOT NULL,
    actor_turn_id TEXT NOT NULL REFERENCES lain_actor_turns(id),
    conversation_id TEXT NOT NULL REFERENCES lain_conversations(id),
    request_digest TEXT NOT NULL,
    snapshot TEXT NOT NULL
);
PRAGMA user_version=37;
