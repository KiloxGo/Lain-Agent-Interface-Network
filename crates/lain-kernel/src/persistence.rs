use std::{
    fs::{File, OpenOptions},
    path::Path,
    time::Duration,
};

use anyhow::{Context, Result, ensure};
use rusqlite::Connection;

use crate::{
    domain::{WorkSession, WorkSessionId, WorkSessionState, now_ms},
    worker::WorkerState,
};

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
        ensure!(
            version <= 2,
            "unsupported database schema version {version}"
        );
        if version == 0 {
            transaction
                .execute_batch(include_str!("../migrations/001_sessions.sql"))
                .context("apply migration 001")?;
        }
        if version < 2 {
            transaction
                .execute_batch(include_str!("../migrations/002_worker_state.sql"))
                .context("apply migration 002")?;
        }
        transaction.commit().context("commit schema migration")?;
        connection.execute_batch(
            "PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL; PRAGMA foreign_keys = ON;",
        )?;
        Ok(Self {
            connection,
            _lock: lock,
        })
    }

    pub fn recover(
        &mut self,
    ) -> Result<(
        Vec<WorkSession>,
        std::collections::BTreeMap<WorkSessionId, WorkerState>,
    )> {
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
        let workers = Self::recover_workers(&transaction)?;
        ensure!(
            workers
                .keys()
                .all(|id| sessions.iter().any(|s| s.id == *id)),
            "orphaned worker state"
        );
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
        Ok((sessions, workers))
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

    pub fn recover_workers(
        tx: &rusqlite::Transaction<'_>,
    ) -> Result<std::collections::BTreeMap<WorkSessionId, WorkerState>> {
        let mut workers = std::collections::BTreeMap::new();
        {
            let mut statement = tx.prepare("SELECT session_id, snapshot FROM worker_state")?;
            let rows = statement
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
            for row in rows {
                let (id, snapshot) = row?;
                let id: WorkSessionId = id.parse().context("invalid worker session identity")?;
                let mut worker: WorkerState = serde_json::from_str(&snapshot)
                    .map_err(|_| anyhow::anyhow!("invalid persisted worker state"))?;
                worker.validate(id)?;
                worker.recover(now_ms()?);
                workers.insert(id, worker);
            }
        }
        for (id, worker) in &workers {
            tx.execute(
                "UPDATE worker_state SET snapshot = ?1 WHERE session_id = ?2",
                (serde_json::to_string(worker)?, id.to_string()),
            )?;
        }
        Ok(workers)
    }

    pub fn save_worker(&mut self, session: &WorkSession, worker: &WorkerState) -> Result<()> {
        // ponytail: whole metadata snapshots cost O(history); use indexed turn/approval
        // rows when measured session history makes these infrequent commits expensive.
        worker.validate(session.id)?;
        let tx = self.connection.transaction()?;
        ensure!(
            tx.execute(
                "UPDATE work_sessions SET snapshot = ?1 WHERE id = ?2",
                (serde_json::to_string(session)?, session.id.to_string())
            )? == 1,
            "session missing from durable storage"
        );
        tx.execute("INSERT INTO worker_state (session_id, snapshot) VALUES (?1, ?2) ON CONFLICT(session_id) DO UPDATE SET snapshot = excluded.snapshot", (session.id.to_string(), serde_json::to_string(worker)?))?;
        tx.commit().context("commit worker state")?;
        Ok(())
    }
}
