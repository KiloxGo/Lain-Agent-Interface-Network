-- Reader compatibility gate: persisted BriefingRead actions can now contain a
-- bounded literal query. Old deny_unknown_fields readers must not open them.
-- Existing source records, manifests, items and grants are unchanged.
PRAGMA user_version=40;
