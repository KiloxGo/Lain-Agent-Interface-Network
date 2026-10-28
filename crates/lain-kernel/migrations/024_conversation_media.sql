CREATE TABLE channel_media (
    id TEXT PRIMARY KEY,
    incoming_id TEXT NOT NULL REFERENCES channel_incoming(id),
    channel_id TEXT NOT NULL REFERENCES channel_bindings(id),
    state TEXT NOT NULL,
    prepare_until_ms INTEGER NOT NULL,
    expires_at_ms INTEGER NOT NULL,
    snapshot TEXT NOT NULL
);
CREATE INDEX channel_media_pending ON channel_media(channel_id,state,prepare_until_ms);
CREATE INDEX channel_media_expiry ON channel_media(expires_at_ms);
CREATE TABLE channel_media_frames (
    media_id TEXT NOT NULL REFERENCES channel_media(id) ON DELETE CASCADE,
    frame_index INTEGER NOT NULL,
    body BLOB NOT NULL,
    PRIMARY KEY(media_id,frame_index)
);
PRAGMA user_version=24;
