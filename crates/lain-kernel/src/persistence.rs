use std::{
    fs::{File, OpenOptions},
    path::Path,
    time::Duration,
};

use anyhow::{Context, Result, bail, ensure};
use rusqlite::Connection;

use crate::domain::{WorkSession, WorkSessionState};

pub(crate) struct Store {
    connection: Connection,
    // Held until after SQLite closes. Never unlink a live lock file.
    _lock: File,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        ensure!(
            path != Path::new(":memory:") && path.file_name().is_some(),
            "database must be a durable file"
        );
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).context("create database directory")?;
        }
        // Canonicalize the file so relative paths and symlinks share an ownership lock.
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .context("create database file")?;
        let path = path.canonicalize().context("resolve database path")?;
        let mut lock_path = path.as_os_str().to_owned();
        lock_path.push(".lock");
        let lock = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(lock_path)
            .context("open database ownership lock")?;
        lock.try_lock()
            .context("database already owned by another Kernel, or locking unavailable")?;
        let mut connection = Connection::open(path).context("open SQLite")?;
        connection.busy_timeout(Duration::from_secs(5))?;
        let transaction = connection.transaction().context("begin schema migration")?;
        let version: u32 = transaction.pragma_query_value(None, "user_version", |r| r.get(0))?;
        match version {
            0 => transaction
                .execute_batch(include_str!("../migrations/001_sessions.sql"))
                .context("apply migration 001")?,
            1 => {}
            _ => bail!("unsupported database schema version {version}"),
        }
        transaction.commit().context("commit schema migration")?;
        connection.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL;")?;
        Ok(Self {
            connection,
            _lock: lock,
        })
    }

    pub fn recover(&mut self) -> Result<Vec<WorkSession>> {
        let transaction = self
            .connection
            .transaction()
            .context("begin session recovery")?;
        let mut sessions = Vec::new();
        {
            let mut statement =
                transaction.prepare("SELECT id, snapshot FROM work_sessions ORDER BY id")?;
            let rows = statement.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (id, snapshot) = row?;
                // Avoid including raw persisted content in errors/logs.
                let session: WorkSession = serde_json::from_str(&snapshot).map_err(|error| {
                    anyhow::anyhow!(
                        "invalid persisted session encoding at line {}, column {} ({:?})",
                        error.line(),
                        error.column(),
                        error.classify()
                    )
                })?;
                ensure!(
                    session.id.to_string() == id,
                    "persisted session identity mismatch"
                );
                session.validate().context("validate persisted session")?;
                sessions.push(session);
            }
        }
        for session in &mut sessions {
            if session.state.needs_reconciliation() {
                session.transition(WorkSessionState::Unknown)?;
                transaction.execute(
                    "UPDATE work_sessions SET snapshot = ?1 WHERE id = ?2",
                    (serde_json::to_string(session)?, session.id.to_string()),
                )?;
            }
        }
        transaction.commit().context("commit session recovery")?;
        Ok(sessions)
    }

    pub fn insert(&self, session: &WorkSession) -> Result<()> {
        self.connection
            .execute(
                "INSERT INTO work_sessions (id, snapshot) VALUES (?1, ?2)",
                (session.id.to_string(), serde_json::to_string(session)?),
            )
            .context("insert session")?;
        Ok(())
    }

    pub fn update(&self, session: &WorkSession) -> Result<()> {
        let changed = self
            .connection
            .execute(
                "UPDATE work_sessions SET snapshot = ?1 WHERE id = ?2",
                (serde_json::to_string(session)?, session.id.to_string()),
            )
            .context("persist session transition")?;
        ensure!(changed == 1, "session missing from durable storage");
        Ok(())
    }
}
