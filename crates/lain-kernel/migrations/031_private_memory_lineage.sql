-- Version gate for metadata-only private influence, raw-source versions and
-- inherited evidence snapshots inside actor_memory_contexts. Older binaries
-- deny unknown context fields and must reject this database before running.
-- Preserve every existing memory, Actor and source body byte-for-byte. Missing
-- legacy lineage stays unknown; no source authority is invented or backfilled.
PRAGMA user_version = 31;
