CREATE TABLE memory_candidates (
    id TEXT PRIMARY KEY,
    instance_id TEXT NOT NULL REFERENCES instances(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    source_run_id TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    source_message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    extractor_version TEXT NOT NULL,
    memory_type TEXT NOT NULL CHECK (memory_type IN (
        'preference', 'fact', 'constraint', 'relationship', 'episodic',
        'open_loop', 'shared_symbol'
    )),
    scope TEXT NOT NULL CHECK (scope IN ('user', 'session', 'relationship')),
    subject TEXT NOT NULL,
    predicate TEXT NOT NULL,
    value TEXT NOT NULL,
    canonical_key TEXT NOT NULL,
    evidence_quote TEXT NOT NULL,
    explicitness TEXT NOT NULL CHECK (explicitness IN ('explicit', 'inferred')),
    confidence REAL NOT NULL CHECK (confidence >= 0.0 AND confidence <= 1.0),
    importance REAL NOT NULL CHECK (importance >= 0.0 AND importance <= 1.0),
    sensitivity TEXT NOT NULL CHECK (sensitivity IN ('public', 'personal', 'sensitive', 'secret')),
    valid_from TEXT,
    valid_to TEXT,
    status TEXT NOT NULL CHECK (status IN ('pending', 'committed', 'rejected')),
    decision_reason TEXT NOT NULL,
    draft_json TEXT NOT NULL CHECK (json_valid(draft_json)),
    decided_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(source_message_id, canonical_key, value)
);

CREATE INDEX memory_candidates_review
    ON memory_candidates(user_id, status, created_at DESC);

CREATE TABLE memory_items (
    id TEXT PRIMARY KEY,
    instance_id TEXT NOT NULL REFERENCES instances(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_id TEXT REFERENCES sessions(id) ON DELETE CASCADE,
    source_candidate_id TEXT NOT NULL UNIQUE REFERENCES memory_candidates(id) ON DELETE RESTRICT,
    memory_type TEXT NOT NULL CHECK (memory_type IN (
        'preference', 'fact', 'constraint', 'relationship', 'episodic',
        'open_loop', 'shared_symbol'
    )),
    scope TEXT NOT NULL CHECK (scope IN ('user', 'session', 'relationship')),
    subject TEXT NOT NULL,
    predicate TEXT NOT NULL,
    value TEXT NOT NULL,
    canonical_key TEXT NOT NULL,
    sensitivity TEXT NOT NULL CHECK (sensitivity IN ('public', 'personal', 'sensitive', 'secret')),
    explicitness TEXT NOT NULL CHECK (explicitness IN ('explicit', 'inferred')),
    confidence REAL NOT NULL CHECK (confidence >= 0.0 AND confidence <= 1.0),
    importance REAL NOT NULL CHECK (importance >= 0.0 AND importance <= 1.0),
    status TEXT NOT NULL CHECK (status IN ('active', 'superseded', 'disputed', 'retracted')),
    version INTEGER NOT NULL CHECK (version > 0),
    supersedes_id TEXT REFERENCES memory_items(id) ON DELETE SET NULL,
    valid_from TEXT,
    valid_to TEXT,
    never_recall INTEGER NOT NULL DEFAULT 0 CHECK (never_recall IN (0, 1)),
    pinned INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK ((scope = 'session' AND session_id IS NOT NULL) OR scope != 'session')
);

CREATE UNIQUE INDEX memory_items_one_active_key
    ON memory_items(user_id, scope, ifnull(session_id, ''), canonical_key)
    WHERE status = 'active';
CREATE INDEX memory_items_recall
    ON memory_items(user_id, status, never_recall, valid_to, updated_at DESC);
CREATE INDEX memory_items_history
    ON memory_items(user_id, canonical_key, version DESC);

CREATE TABLE memory_links (
    id TEXT PRIMARY KEY,
    memory_id TEXT NOT NULL REFERENCES memory_items(id) ON DELETE CASCADE,
    link_type TEXT NOT NULL CHECK (link_type IN ('source', 'supersedes', 'supports', 'contradicts')),
    event_id TEXT REFERENCES events(event_id) ON DELETE SET NULL,
    message_id TEXT REFERENCES messages(id) ON DELETE SET NULL,
    related_memory_id TEXT REFERENCES memory_items(id) ON DELETE SET NULL,
    metadata_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(metadata_json)),
    created_at TEXT NOT NULL,
    CHECK (event_id IS NOT NULL OR message_id IS NOT NULL OR related_memory_id IS NOT NULL)
);

CREATE INDEX memory_links_memory ON memory_links(memory_id, link_type);
CREATE INDEX memory_links_event ON memory_links(event_id) WHERE event_id IS NOT NULL;

-- This table is a disposable projection. memory_items remains canonical.
CREATE VIRTUAL TABLE memory_items_fts USING fts5(
    memory_id UNINDEXED,
    user_id UNINDEXED,
    searchable_text,
    tokenize = 'trigram'
);

INSERT INTO schema_metadata(schema_version, app_version, applied_at)
VALUES (3, '0.1.0', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
