CREATE TABLE knowledge_sync (
    source_id TEXT NOT NULL REFERENCES archive_sources(id),
    revision INTEGER NOT NULL,
    snapshot TEXT NOT NULL,
    PRIMARY KEY(source_id, revision)
);
PRAGMA user_version = 21;
