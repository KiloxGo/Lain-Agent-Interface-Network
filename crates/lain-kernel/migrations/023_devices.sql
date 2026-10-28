CREATE TABLE devices (
    id TEXT PRIMARY KEY,
    request_id TEXT UNIQUE NOT NULL,
    fingerprint TEXT UNIQUE NOT NULL,
    snapshot TEXT NOT NULL
);
CREATE TABLE device_reports (
    device_id TEXT NOT NULL REFERENCES devices(id),
    sequence INTEGER NOT NULL,
    request_id TEXT NOT NULL,
    snapshot TEXT NOT NULL,
    PRIMARY KEY(device_id,sequence),
    UNIQUE(device_id,request_id)
);
PRAGMA user_version=23;
