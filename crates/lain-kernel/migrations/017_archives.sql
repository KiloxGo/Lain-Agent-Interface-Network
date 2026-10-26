CREATE TABLE archive_sources (
    id TEXT PRIMARY KEY,
    source_key TEXT NOT NULL UNIQUE,
    snapshot TEXT NOT NULL
);
CREATE TABLE archive_grants (
    source_id TEXT NOT NULL REFERENCES archive_sources(id),
    revision INTEGER NOT NULL,
    snapshot TEXT NOT NULL,
    PRIMARY KEY(source_id,revision)
);
CREATE TABLE archive_records (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    source_id TEXT NOT NULL REFERENCES archive_sources(id),
    external_id TEXT NOT NULL,
    grant_revision INTEGER NOT NULL,
    ingest_fingerprint TEXT NOT NULL,
    retain_until_ms INTEGER NOT NULL,
    snapshot TEXT NOT NULL,
    UNIQUE(source_id,external_id)
);
CREATE INDEX archive_source_sequence ON archive_records(source_id,grant_revision,sequence);
CREATE INDEX archive_retention ON archive_records(retain_until_ms) WHERE json_extract(snapshot,'$.text') IS NOT NULL;
CREATE TABLE archive_edits (
    record_id TEXT NOT NULL REFERENCES archive_records(id),
    revision INTEGER NOT NULL,
    kind TEXT NOT NULL,
    at_ms INTEGER NOT NULL,
    PRIMARY KEY(record_id,revision)
);
PRAGMA user_version=17;
