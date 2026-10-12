use std::{
    fs::{File, OpenOptions},
    path::Path,
    time::Duration,
};

use anyhow::{Context, Result, ensure};
use rusqlite::Connection;

use crate::{
    domain::{RuntimeAttachment, WorkSession, WorkSessionId, WorkSessionState, now_ms},
    events::{Event, EventKind, HistoryPage},
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
            version <= 5,
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
        if version < 3 {
            transaction
                .execute_batch(include_str!("../migrations/003_history.sql"))
                .context("apply migration 003")?;
        }
        if version < 4 {
            transaction.execute_batch(include_str!("../migrations/004_profiles.sql"))?;
            let local = crate::profiles::RuntimeProfile {
                definition: crate::profiles::ProfileDefinition::local(),
                revision: 1,
            };
            transaction.execute(
                "INSERT INTO runtime_profiles VALUES (?1, ?2)",
                (
                    local.definition.id.to_string(),
                    serde_json::to_string(&local)?,
                ),
            )?;
            let mut rows = transaction.prepare("SELECT w.session_id, w.snapshot, s.snapshot FROM worker_state w JOIN work_sessions s ON s.id = w.session_id")?;
            let records = rows
                .query_map([], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            drop(rows);
            for (id, state, session) in records {
                let mut worker: WorkerState = serde_json::from_str(&state)
                    .map_err(|_| anyhow::anyhow!("invalid legacy worker state"))?;
                let session: WorkSession = serde_json::from_str(&session)
                    .map_err(|_| anyhow::anyhow!("invalid legacy session"))?;
                if session.runtime_profile_id.to_string() == "codex/local"
                    && worker.resolved_profile.is_none()
                {
                    worker.resolved_profile = Some(
                        crate::profiles::ResolvedRuntimeProfile::legacy(&worker.options)?,
                    );
                    transaction.execute(
                        "UPDATE worker_state SET snapshot = ?1 WHERE session_id = ?2",
                        (serde_json::to_string(&worker)?, id),
                    )?;
                }
            }
        }
        if version < 5 {
            transaction.execute_batch(include_str!("../migrations/005_actor.sql"))?;
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
            let from = session.state;
            let runtime_from = session.runtime;
            let known_idle = matches!(
                session.state,
                WorkSessionState::Idle | WorkSessionState::Ready
            ) && workers.get(&session.id).is_some_and(|w| {
                w.provider_session_id.is_some()
                    && w.turns.iter().all(|r| r.turn.state.is_terminal())
                    && w.approvals.iter().all(|a| {
                        !matches!(
                            a.status,
                            crate::domain::ApprovalStatus::Pending
                                | crate::domain::ApprovalStatus::Responding
                        )
                    })
            });
            let runtime_changed = session.runtime != RuntimeAttachment::Detached;
            session.runtime = RuntimeAttachment::Detached;
            if known_idle && session.state == WorkSessionState::Ready {
                session.transition(WorkSessionState::Idle)?;
            }
            if session.state.needs_reconciliation() && !known_idle {
                session.transition(WorkSessionState::Unknown)?;
            }
            if runtime_changed
                || from != session.state
                || session.state == WorkSessionState::Unknown
            {
                transaction.execute(
                    "UPDATE work_sessions SET snapshot = ?1 WHERE id = ?2",
                    (serde_json::to_string(session)?, session.id.to_string()),
                )?;
            }
            let mut events = Vec::new();
            if from != session.state {
                events.push(Event::new(
                    session.id,
                    None,
                    session.updated_at_ms,
                    EventKind::SessionStateChanged {
                        from,
                        to: session.state,
                    },
                ));
            }
            if runtime_changed {
                events.push(Event::new(
                    session.id,
                    None,
                    now_ms()?,
                    EventKind::RuntimeAttachmentChanged {
                        from: runtime_from,
                        to: session.runtime,
                    },
                ));
            }
            Self::write_events(&transaction, &mut events)?;
        }
        transaction.commit().context("commit session recovery")?;
        Ok((sessions, workers))
    }

    pub fn profiles(&self) -> Result<Vec<crate::profiles::RuntimeProfile>> {
        let mut query = self
            .connection
            .prepare("SELECT snapshot FROM runtime_profiles ORDER BY id")?;
        let rows = query.query_map([], |r| r.get::<_, String>(0))?;
        let mut profiles = Vec::new();
        for row in rows {
            let p: crate::profiles::RuntimeProfile = serde_json::from_str(&row?)
                .map_err(|_| anyhow::anyhow!("invalid persisted profile"))?;
            p.definition.validate()?;
            ensure!(p.revision > 0, "invalid profile revision");
            profiles.push(p);
        }
        Ok(profiles)
    }
    pub fn save_profile(
        &mut self,
        profile: &crate::profiles::RuntimeProfile,
        create: bool,
    ) -> Result<()> {
        let tx = self.connection.transaction()?;
        ensure!(
            tx.execute(
                if create {
                    "INSERT INTO runtime_profiles VALUES (?1, ?2)"
                } else {
                    "UPDATE runtime_profiles SET snapshot = ?2 WHERE id = ?1"
                },
                (
                    profile.definition.id.to_string(),
                    serde_json::to_string(profile)?
                )
            )? == 1,
            "profile not found"
        );
        tx.commit()?;
        Ok(())
    }
    pub fn policy(&self, workspace: &Path) -> Result<Option<crate::profiles::WorkspacePolicy>> {
        use rusqlite::OptionalExtension;
        let snapshot: Option<String> = self
            .connection
            .query_row(
                "SELECT snapshot FROM workspace_profile_policies WHERE workspace = ?1",
                [workspace.to_str().context("non-UTF8 workspace")?],
                |r| r.get(0),
            )
            .optional()?;
        snapshot
            .map(|s| {
                serde_json::from_str(&s)
                    .map_err(|_| anyhow::anyhow!("invalid workspace profile policy"))
            })
            .transpose()
    }
    pub fn save_policy(&mut self, policy: &crate::profiles::WorkspacePolicy) -> Result<()> {
        let tx = self.connection.transaction()?;
        tx.execute("INSERT INTO workspace_profile_policies VALUES (?1, ?2) ON CONFLICT(workspace) DO UPDATE SET snapshot = excluded.snapshot", (policy.workspace.to_str().context("non-UTF8 workspace")?, serde_json::to_string(policy)?))?;
        tx.commit()?;
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
                let now = now_ms()?;
                let mut events: Vec<_> = worker
                    .recover(now)
                    .into_iter()
                    .map(|kind| Event::new(id, None, now, kind))
                    .collect();
                Self::write_events(tx, &mut events)?;
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

    pub fn save(
        &mut self,
        session: &WorkSession,
        worker: Option<&WorkerState>,
        create: bool,
        events: &mut [Event],
    ) -> Result<()> {
        // ponytail: whole metadata snapshots cost O(history); use indexed turn/approval
        // rows when measured session history makes these infrequent commits expensive.
        if let Some(worker) = worker {
            worker.validate(session.id)?;
        }
        let tx = self.connection.transaction()?;
        ensure!(
            tx.execute(
                if create {
                    "INSERT INTO work_sessions (snapshot, id) VALUES (?1, ?2)"
                } else {
                    "UPDATE work_sessions SET snapshot = ?1 WHERE id = ?2"
                },
                (serde_json::to_string(session)?, session.id.to_string())
            )? == 1,
            "session missing from durable storage"
        );
        if let Some(worker) = worker {
            tx.execute("INSERT INTO worker_state (session_id, snapshot) VALUES (?1, ?2) ON CONFLICT(session_id) DO UPDATE SET snapshot = excluded.snapshot", (session.id.to_string(), serde_json::to_string(worker)?))?;
        }
        Self::write_events(&tx, events)?;
        tx.commit().context("commit worker state")?;
        Ok(())
    }

    fn write_events(tx: &rusqlite::Transaction<'_>, events: &mut [Event]) -> Result<()> {
        for event in events {
            ensure!(
                !matches!(event.kind, EventKind::OutputProgress { .. }),
                "ephemeral progress cannot be persisted"
            );
            let kind = serde_json::to_value(&event.kind)?["kind"]
                .as_str()
                .context("missing event kind")?
                .to_owned();
            tx.execute("INSERT INTO operational_history (event_id, session_id, turn_id, timestamp_ms, kind, payload) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", rusqlite::params![event.id.to_string(), event.work_session_id.to_string(), event.turn_id.map(|id| id.to_string()), i64::try_from(event.timestamp_ms)?, kind, serde_json::to_string(event)?])?;
            event.history_order = Some(tx.last_insert_rowid().try_into()?);
        }
        Ok(())
    }

    pub fn append_events(&mut self, events: &mut [Event]) -> Result<()> {
        let tx = self.connection.transaction()?;
        Self::write_events(&tx, events)?;
        tx.commit().context("commit operational history")?;
        Ok(())
    }

    pub fn history_cursor(&self) -> Result<u64> {
        Ok(self
            .connection
            .query_row(
                "SELECT coalesce(max(sequence), 0) FROM operational_history",
                [],
                |row| row.get::<_, i64>(0),
            )?
            .try_into()?)
    }

    pub fn history(
        &self,
        id: WorkSessionId,
        after: u64,
        until: u64,
        limit: usize,
        before: bool,
    ) -> Result<HistoryPage> {
        let mut statement = self.connection.prepare("SELECT sequence, payload FROM operational_history WHERE session_id = ?1 AND sequence > ?2 AND sequence <= ?3 ORDER BY sequence LIMIT ?4")?;
        if before {
            statement = self.connection.prepare("SELECT sequence, payload FROM operational_history WHERE session_id = ?1 AND sequence > ?2 AND sequence <= ?3 ORDER BY sequence DESC LIMIT ?4")?;
        }
        let rows = statement.query_map(
            rusqlite::params![
                id.to_string(),
                i64::try_from(after)?,
                i64::try_from(until)?,
                i64::try_from(limit + 1)?
            ],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )?;
        let mut events = Vec::new();
        for row in rows {
            let (order, payload) = row?;
            let mut event: Event = serde_json::from_str(&payload)
                .map_err(|_| anyhow::anyhow!("invalid operational history encoding"))?;
            event.history_order = Some(order.try_into()?);
            events.push(event);
        }
        let has_more = events.len() > limit;
        events.truncate(limit);
        if before {
            events.reverse();
        }
        let next_cursor = events.last().and_then(|e| e.history_order).unwrap_or(after);
        Ok(HistoryPage {
            events,
            next_cursor,
            has_more,
        })
    }
}

impl Store {
    pub fn create_conversation(&mut self) -> Result<crate::actor::domain::Conversation> {
        use crate::actor::domain::*;
        let now = now_ms()?;
        let conversation = Conversation {
            id: ConversationId::new(),
            created_at_ms: now,
            updated_at_ms: now,
        };
        self.connection.execute(
            "INSERT INTO lain_conversations VALUES (?1, ?2, ?2)",
            (conversation.id.to_string(), i64::try_from(now)?),
        )?;
        Ok(conversation)
    }
    pub fn conversations(&self) -> Result<Vec<crate::actor::domain::Conversation>> {
        let mut query=self.connection.prepare("SELECT id, created_at_ms, updated_at_ms FROM lain_conversations ORDER BY updated_at_ms DESC, id LIMIT 100")?;
        let rows = query.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })?;
        rows.map(|r| {
            let (id, created_at_ms, updated_at_ms) = r?;
            Ok(crate::actor::domain::Conversation {
                id: crate::actor::domain::ConversationId(id.parse()?),
                created_at_ms: created_at_ms.try_into()?,
                updated_at_ms: updated_at_ms.try_into()?,
            })
        })
        .collect()
    }
    pub fn actor_page(
        &self,
        id: crate::actor::domain::ConversationId,
        after: u64,
        limit: usize,
    ) -> Result<crate::actor::domain::ConversationPage> {
        use crate::actor::domain::*;
        use rusqlite::OptionalExtension;
        let meta = self
            .connection
            .query_row(
                "SELECT created_at_ms, updated_at_ms FROM lain_conversations WHERE id = ?1",
                [id.to_string()],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
            )
            .optional()?
            .ok_or(crate::backend::ControlError::NotFound)?;
        let mut query=self.connection.prepare("SELECT snapshot FROM lain_actor_turns WHERE conversation_id = ?1 AND sequence > ?2 ORDER BY sequence LIMIT ?3")?;
        let rows = query.query_map(
            rusqlite::params![
                id.to_string(),
                i64::try_from(after)?,
                i64::try_from(limit + 1)?
            ],
            |r| r.get::<_, String>(0),
        )?;
        let mut turns: Vec<ActorTurn> = rows.map(|r| decode_actor(&r?)).collect::<Result<_>>()?;
        let has_more = turns.len() > limit;
        turns.truncate(limit);
        let next_cursor = turns.last().map(|t| t.sequence).unwrap_or(after);
        Ok(ConversationPage {
            conversation: Conversation {
                id,
                created_at_ms: meta.0.try_into()?,
                updated_at_ms: meta.1.try_into()?,
            },
            turns,
            next_cursor,
            has_more,
        })
    }
    pub fn actor_recent(
        &self,
        id: crate::actor::domain::ConversationId,
    ) -> Result<Vec<crate::actor::domain::ActorTurn>> {
        let mut query=self.connection.prepare("SELECT snapshot FROM lain_actor_turns WHERE conversation_id = ?1 ORDER BY sequence DESC LIMIT 20")?;
        let rows = query.query_map([id.to_string()], |r| r.get::<_, String>(0))?;
        let mut turns: Vec<_> = rows.map(|r| decode_actor(&r?)).collect::<Result<_>>()?;
        turns.reverse();
        Ok(turns)
    }
    pub fn actor_turn(
        &self,
        id: crate::actor::domain::ActorTurnId,
    ) -> Result<crate::actor::domain::ActorTurn> {
        use rusqlite::OptionalExtension;
        let value = self
            .connection
            .query_row(
                "SELECT snapshot FROM lain_actor_turns WHERE id = ?1",
                [id.to_string()],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .ok_or(crate::backend::ControlError::NotFound)?;
        decode_actor(&value)
    }
    pub fn actor_duplicate(
        &self,
        id: crate::actor::domain::ConversationId,
        request: uuid::Uuid,
    ) -> Result<Option<crate::actor::domain::ActorTurn>> {
        use rusqlite::OptionalExtension;
        let value=self.connection.query_row("SELECT snapshot FROM lain_actor_turns WHERE conversation_id = ?1 AND request_id = ?2", (id.to_string(),request.to_string()), |r| r.get::<_,String>(0)).optional()?;
        value.map(|v| decode_actor(&v)).transpose()
    }
    pub fn save_actor_turn(
        &mut self,
        turn: &mut crate::actor::domain::ActorTurn,
        create: bool,
    ) -> Result<()> {
        turn.revision = turn
            .revision
            .checked_add(1)
            .context("Actor revision exhausted")?;
        let tx = self.connection.transaction()?;
        if create {
            tx.execute("INSERT INTO lain_actor_turns(id, conversation_id, request_id, snapshot) VALUES (?1, ?2, ?3, '{}')", (turn.id.to_string(),turn.conversation_id.to_string(),turn.request_id.to_string()))?;
            turn.sequence = tx.last_insert_rowid().try_into()?;
        }
        ensure!(
            tx.execute(
                "UPDATE lain_actor_turns SET snapshot = ?2 WHERE id = ?1",
                (turn.id.to_string(), serde_json::to_string(turn)?)
            )? == 1,
            "Actor turn not found"
        );
        tx.execute(
            "UPDATE lain_conversations SET updated_at_ms = max(updated_at_ms, ?2) WHERE id = ?1",
            (turn.conversation_id.to_string(), i64::try_from(now_ms()?)?),
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn recover_actor(&mut self) -> Result<()> {
        use crate::actor::domain::*;
        let tx = self.connection.transaction()?;
        let mut query = tx.prepare("SELECT id, conversation_id, snapshot FROM lain_actor_turns")?;
        let rows = query.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        let mut turns = Vec::new();
        for row in rows {
            let (id, conversation, value) = row?;
            let mut turn = decode_actor(&value)?;
            ensure!(
                turn.id.to_string() == id && turn.conversation_id.to_string() == conversation,
                "Actor record identity mismatch"
            );
            if turn.state == TurnState::Running {
                turn.revision += 1;
                turn.state = TurnState::Unknown;
                turn.error = Some(
                    "Kernel restarted during inference or execution; no action was replayed".into(),
                );
                for action in &mut turn.actions {
                    if action.state == ActionState::Executing {
                        action.state = ActionState::Unknown;
                    }
                }
                for message in &mut turn.messages {
                    if message.status == TurnState::Running {
                        message.status = TurnState::Unknown;
                    }
                }
                turns.push(turn);
            }
        }
        drop(query);
        for turn in turns {
            tx.execute(
                "UPDATE lain_actor_turns SET snapshot = ?2 WHERE id = ?1",
                (turn.id.to_string(), serde_json::to_string(&turn)?),
            )?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn storage_healthy(&self) -> Result<bool> {
        Ok(self
            .connection
            .query_row("PRAGMA quick_check(1)", [], |r| r.get::<_, String>(0))?
            == "ok")
    }
}
fn decode_actor(value: &str) -> Result<crate::actor::domain::ActorTurn> {
    serde_json::from_str(value).map_err(|_| anyhow::anyhow!("invalid persisted Actor record"))
}
