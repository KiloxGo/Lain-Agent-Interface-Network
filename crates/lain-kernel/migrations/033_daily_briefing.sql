-- Independent per-conversation read authority. No grants are created on upgrade.
CREATE TABLE briefing_grants (
    conversation_id TEXT NOT NULL REFERENCES lain_conversations(id),
    revision INTEGER NOT NULL CHECK (revision > 0),
    request_id TEXT NOT NULL UNIQUE,
    request TEXT NOT NULL,
    snapshot TEXT NOT NULL,
    PRIMARY KEY (conversation_id, revision)
);
-- No raw text, generated summaries or semantic memories are stored here.
CREATE TABLE briefing_manifests (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES lain_conversations(id),
    request TEXT NOT NULL,
    snapshot TEXT NOT NULL
);
CREATE INDEX briefing_conversation ON briefing_manifests(conversation_id);
PRAGMA user_version = 33;
