CREATE TABLE lain_memories (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL,
    scope TEXT NOT NULL,
    status TEXT NOT NULL,
    content TEXT NOT NULL CHECK(length(CAST(content AS BLOB)) BETWEEN 1 AND 4096),
    snapshot TEXT NOT NULL CHECK(length(CAST(snapshot AS BLOB)) <= 16384)
);
CREATE INDEX lain_memory_filter ON lain_memories(scope, status, kind, sequence);
CREATE VIRTUAL TABLE lain_memory_fts USING fts5(content, content='lain_memories', content_rowid='sequence', tokenize='trigram');
CREATE TRIGGER lain_memory_insert AFTER INSERT ON lain_memories BEGIN
 INSERT INTO lain_memory_fts(rowid,content) VALUES(new.sequence,new.content);
END;
CREATE TRIGGER lain_memory_delete AFTER DELETE ON lain_memories BEGIN
 INSERT INTO lain_memory_fts(lain_memory_fts,rowid,content) VALUES('delete',old.sequence,old.content);
END;
CREATE TABLE lain_memory_requests (
    request_id TEXT PRIMARY KEY,
    payload_hash TEXT NOT NULL,
    memory_id TEXT NOT NULL
);
CREATE TABLE lain_memory_candidates (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    actor_action TEXT UNIQUE,
    snapshot TEXT NOT NULL CHECK(length(CAST(snapshot AS BLOB)) <= 16384)
);
CREATE TABLE lain_session_summaries (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    session_id TEXT NOT NULL,
    turn_id TEXT NOT NULL,
    source_cursor INTEGER NOT NULL,
    snapshot TEXT NOT NULL CHECK(length(CAST(snapshot AS BLOB)) <= 32768),
    UNIQUE(session_id, turn_id, source_cursor)
);
PRAGMA user_version = 7;
