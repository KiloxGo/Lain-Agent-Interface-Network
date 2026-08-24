UPDATE memory_items
SET valid_to = COALESCE(
    (SELECT COALESCE(next.valid_from, next.created_at)
     FROM memory_items AS next
     WHERE next.supersedes_id = memory_items.id
     ORDER BY next.version
     LIMIT 1),
    updated_at
)
WHERE status = 'superseded' AND valid_to IS NULL;

DELETE FROM memory_items_fts;

INSERT INTO memory_items_fts(memory_id, user_id, searchable_text)
SELECT id, user_id, subject || ' ' || predicate || ' ' || value
FROM memory_items
WHERE status IN ('active', 'superseded');

INSERT INTO schema_metadata(schema_version, app_version, applied_at)
VALUES (4, '0.1.0', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
