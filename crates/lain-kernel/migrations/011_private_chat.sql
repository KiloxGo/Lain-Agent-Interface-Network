CREATE INDEX channel_incoming_batch ON channel_incoming(channel_id,json_extract(snapshot,'$.batch_id'),sequence);
PRAGMA user_version=11;
