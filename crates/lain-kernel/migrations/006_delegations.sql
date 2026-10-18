CREATE TABLE lain_delegations (
    action_id TEXT PRIMARY KEY NOT NULL,
    conversation_id TEXT NOT NULL REFERENCES lain_conversations(id),
    actor_turn_id TEXT NOT NULL REFERENCES lain_actor_turns(id),
    session_id TEXT NOT NULL REFERENCES work_sessions(id),
    worker_turn_id TEXT UNIQUE,
    snapshot TEXT NOT NULL CHECK(length(CAST(snapshot AS BLOB)) <= 32768)
);
CREATE INDEX delegations_conversation ON lain_delegations(conversation_id, action_id);
CREATE TABLE lain_conversation_notices (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT UNIQUE NOT NULL,
    conversation_id TEXT NOT NULL REFERENCES lain_conversations(id),
    action_id TEXT NOT NULL REFERENCES lain_delegations(action_id),
    outcome_key TEXT NOT NULL,
    acknowledged INTEGER NOT NULL DEFAULT 0 CHECK(acknowledged IN (0, 1)),
    snapshot TEXT NOT NULL CHECK(length(CAST(snapshot AS BLOB)) <= 65536),
    UNIQUE(action_id, outcome_key)
);
CREATE INDEX notices_conversation_order ON lain_conversation_notices(conversation_id, sequence);
PRAGMA user_version = 6;
