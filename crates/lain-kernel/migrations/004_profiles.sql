CREATE TABLE runtime_profiles (id TEXT PRIMARY KEY NOT NULL, snapshot TEXT NOT NULL);
CREATE TABLE workspace_profile_policies (workspace TEXT PRIMARY KEY NOT NULL, snapshot TEXT NOT NULL);
PRAGMA user_version = 4;
