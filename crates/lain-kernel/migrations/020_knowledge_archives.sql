CREATE TABLE knowledge_sources (
    source_id TEXT PRIMARY KEY REFERENCES archive_sources(id),
    snapshot TEXT NOT NULL
);
CREATE TABLE knowledge_records (
    archive_record_id TEXT UNIQUE REFERENCES archive_records(id),
    source_id TEXT NOT NULL REFERENCES archive_sources(id),
    remote_id INTEGER NOT NULL,
    native_id TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    withdrawn INTEGER NOT NULL,
    PRIMARY KEY(source_id,remote_id)
);
PRAGMA user_version=20;
