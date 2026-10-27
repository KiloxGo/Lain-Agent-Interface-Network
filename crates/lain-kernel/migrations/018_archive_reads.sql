CREATE TABLE archive_reads (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES archive_sources(id),
    snapshot TEXT NOT NULL,
    next_cursor TEXT
);
CREATE INDEX archive_reads_source ON archive_reads(source_id);
PRAGMA user_version = 18;
