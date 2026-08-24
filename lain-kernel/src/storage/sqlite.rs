use std::{path::Path, str::FromStr, time::Duration};

use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use serde_json::json;
use sqlx::{
    Row, Sqlite, SqlitePool, Transaction,
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    context::{ContextPlan, RelationshipState, SelfState},
    domain::{
        event::{EventEnvelope, PROTOCOL_VERSION, PersistedEvent},
        run::RunState,
    },
    identity::IdentityKernel,
    memory::{
        policy::evaluate,
        types::{
            CandidateDisposition, DurableJob, EXTRACTOR_VERSION, ExtractedMemory, ExtractionSource,
            MemoryItemView, PendingMemoryView, RecordedCandidate,
        },
    },
};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database operation failed: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("database migration failed: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("event serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("session not found: {0}")]
    SessionNotFound(String),
    #[error("run not found: {0}")]
    RunNotFound(String),
    #[error("invalid run transition from {from} to {to}")]
    InvalidRunTransition { from: String, to: &'static str },
    #[error("stored run state is invalid: {0}")]
    InvalidStoredRunState(String),
    #[error("identity version {version} already exists with a different content hash")]
    IdentityVersionConflict { version: String },
    #[error("memory candidate not found: {0}")]
    MemoryCandidateNotFound(String),
    #[error("memory candidate {id} cannot transition from {status}")]
    InvalidMemoryCandidateStatus { id: String, status: String },
    #[error("memory item not found: {0}")]
    MemoryItemNotFound(String),
    #[error("invalid memory item status: {0}")]
    InvalidMemoryItemStatus(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct StorageScope {
    pub instance_id: String,
    pub user_id: String,
    pub session_id: String,
}

#[derive(Clone)]
pub struct Storage {
    pool: SqlitePool,
    scope: StorageScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct MessageRecord {
    pub id: String,
    pub session_id: String,
    pub run_id: Option<String>,
    pub role: String,
    pub content: String,
    pub status: String,
    pub created_at: String,
    pub client_message_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct RunRecord {
    pub id: String,
    pub session_id: String,
    pub trigger_message_id: String,
    pub state: String,
    pub provider: String,
    pub model: String,
    pub identity_version: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AcceptedTurn {
    pub deduplicated: bool,
    pub message: MessageRecord,
    pub run: RunRecord,
    pub events: Vec<EventEnvelope>,
}

#[derive(Debug, Clone, Default)]
pub struct CompletionUsage {
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cached_input_tokens: Option<i64>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct SessionSnapshot {
    pub session_id: String,
    pub last_seq: i64,
    pub messages: Vec<MessageRecord>,
    pub active_run: Option<RunRecord>,
}

impl Storage {
    pub async fn initialize(data_dir: &Path) -> Result<Self, StorageError> {
        let state_dir = data_dir.join("state");
        tokio::fs::create_dir_all(&state_dir)
            .await
            .map_err(sqlx::Error::Io)?;
        let database_path = state_dir.join("lain.db");
        let options = SqliteConnectOptions::from_str(&format!(
            "sqlite://{}",
            database_path.to_string_lossy().replace('\\', "/")
        ))?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Full)
        .busy_timeout(Duration::from_secs(5))
        .pragma("temp_store", "MEMORY");
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(options)
            .await?;
        MIGRATOR.run(&pool).await?;
        let scope = ensure_default_scope(&pool).await?;
        let storage = Self { pool, scope };
        storage.recover_interrupted_runs().await?;
        Ok(storage)
    }

    #[cfg(test)]
    pub async fn in_memory() -> Result<Self, StorageError> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")?
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Memory)
            .synchronous(SqliteSynchronous::Full)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        MIGRATOR.run(&pool).await?;
        let scope = ensure_default_scope(&pool).await?;
        Ok(Self { pool, scope })
    }

    pub fn scope(&self) -> &StorageScope {
        &self.scope
    }

    pub async fn journal_mode(&self) -> Result<String, StorageError> {
        Ok(sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(&self.pool)
            .await?)
    }

    pub async fn foreign_keys_enabled(&self) -> Result<bool, StorageError> {
        let value: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(&self.pool)
            .await?;
        Ok(value == 1)
    }

    pub async fn accept_user_input(
        &self,
        session_id: &str,
        client_message_id: &str,
        content: &str,
        provider: &str,
        model: &str,
        identity_version: &str,
    ) -> Result<AcceptedTurn, StorageError> {
        self.require_session(session_id).await?;
        let now = now();
        let message_id = new_id();
        let run_id = new_id();
        let mut tx = self.pool.begin().await?;

        let inserted = sqlx::query(
            "INSERT INTO messages(id, session_id, role, content, status, created_at, client_message_id, metadata_json) \
             VALUES (?, ?, 'user', ?, 'complete', ?, ?, '{}') \
             ON CONFLICT(session_id, client_message_id) WHERE client_message_id IS NOT NULL DO NOTHING",
        )
        .bind(&message_id)
        .bind(session_id)
        .bind(content)
        .bind(&now)
        .bind(client_message_id)
        .execute(&mut *tx)
        .await?;

        if inserted.rows_affected() == 0 {
            let message =
                fetch_message_by_client_id(&mut tx, session_id, client_message_id).await?;
            let run = fetch_run_for_trigger(&mut tx, &message.id).await?;
            let events = fetch_events_for_run(&mut tx, &run.id).await?;
            tx.commit().await?;
            return Ok(AcceptedTurn {
                deduplicated: true,
                message,
                run,
                events,
            });
        }

        sqlx::query(
            "INSERT INTO runs(id, session_id, trigger_message_id, state, mode, provider, model, identity_version, created_at) \
             VALUES (?, ?, ?, 'queued', 'chat', ?, ?, ?, ?)",
        )
        .bind(&run_id)
        .bind(session_id)
        .bind(&message_id)
        .bind(provider)
        .bind(model)
        .bind(identity_version)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE messages SET run_id = ? WHERE id = ?")
            .bind(&run_id)
            .bind(&message_id)
            .execute(&mut *tx)
            .await?;

        let message_event = append_event(
            &mut tx,
            &self.scope.instance_id,
            session_id,
            Some(&run_id),
            &PersistedEvent::MessageAccepted {
                message_id: message_id.clone(),
                client_message_id: client_message_id.to_string(),
                content: content.to_string(),
            },
        )
        .await?;
        let run_event = append_event(
            &mut tx,
            &self.scope.instance_id,
            session_id,
            Some(&run_id),
            &PersistedEvent::RunQueued {
                trigger_message_id: message_id.clone(),
            },
        )
        .await?;
        tx.commit().await?;

        Ok(AcceptedTurn {
            deduplicated: false,
            message: MessageRecord {
                id: message_id.clone(),
                session_id: session_id.to_string(),
                run_id: Some(run_id.clone()),
                role: "user".to_string(),
                content: content.to_string(),
                status: "complete".to_string(),
                created_at: now.clone(),
                client_message_id: Some(client_message_id.to_string()),
            },
            run: RunRecord {
                id: run_id,
                session_id: session_id.to_string(),
                trigger_message_id: message_id,
                state: RunState::Queued.as_str().to_string(),
                provider: provider.to_string(),
                model: model.to_string(),
                identity_version: identity_version.to_string(),
                started_at: None,
                completed_at: None,
                error_code: None,
            },
            events: vec![message_event, run_event],
        })
    }

    pub async fn start_run(&self, run_id: &str) -> Result<EventEnvelope, StorageError> {
        let mut tx = self.pool.begin().await?;
        let run = fetch_run(&mut tx, run_id).await?;
        transition_run(&mut tx, &run, RunState::Running, None).await?;
        let event = append_event(
            &mut tx,
            &self.scope.instance_id,
            &run.session_id,
            Some(run_id),
            &PersistedEvent::RunStarted {
                provider: run.provider,
                model: run.model,
            },
        )
        .await?;
        tx.commit().await?;
        Ok(event)
    }

    pub async fn complete_reply(
        &self,
        run_id: &str,
        content: &str,
        finish_reason: &str,
        usage: CompletionUsage,
    ) -> Result<Vec<EventEnvelope>, StorageError> {
        let mut tx = self.pool.begin().await?;
        let run = fetch_run(&mut tx, run_id).await?;
        let current = parse_run_state(&run.state)?;
        if !current.can_transition_to(RunState::Completed) {
            return Err(StorageError::InvalidRunTransition {
                from: run.state,
                to: RunState::Completed.as_str(),
            });
        }

        let message_id = new_id();
        let timestamp = now();
        sqlx::query(
            "INSERT INTO messages(id, session_id, run_id, role, content, status, created_at, metadata_json) \
             VALUES (?, ?, ?, 'assistant', ?, 'complete', ?, '{}')",
        )
        .bind(&message_id)
        .bind(&run.session_id)
        .bind(run_id)
        .bind(content)
        .bind(&timestamp)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE runs SET state = 'completed', completed_at = ?, usage_input_tokens = ?, \
             usage_output_tokens = ?, usage_cached_tokens = ? WHERE id = ?",
        )
        .bind(&timestamp)
        .bind(usage.input_tokens)
        .bind(usage.output_tokens)
        .bind(usage.cached_input_tokens)
        .bind(run_id)
        .execute(&mut *tx)
        .await?;

        let text_event = append_event(
            &mut tx,
            &self.scope.instance_id,
            &run.session_id,
            Some(run_id),
            &PersistedEvent::TextCompleted {
                message_id: message_id.clone(),
                content: content.to_string(),
            },
        )
        .await?;
        let usage_json = json!({
            "input_tokens": usage.input_tokens,
            "output_tokens": usage.output_tokens,
            "cached_input_tokens": usage.cached_input_tokens,
        });
        let completed_event = append_event(
            &mut tx,
            &self.scope.instance_id,
            &run.session_id,
            Some(run_id),
            &PersistedEvent::RunCompleted {
                message_id: message_id.clone(),
                finish_reason: finish_reason.to_string(),
                usage: Some(usage_json),
            },
        )
        .await?;
        let job_payload = serde_json::to_string(&json!({
            "run_id": run_id,
            "session_id": run.session_id,
            "assistant_message_id": message_id,
        }))?;
        sqlx::query(
            "INSERT INTO durable_jobs(id, job_type, payload_json, status, priority, attempts, max_attempts, available_at, created_at, updated_at) \
             VALUES (?, 'memory_extract', ?, 'queued', 0, 0, 3, ?, ?, ?)",
        )
        .bind(new_id())
        .bind(job_payload)
        .bind(&timestamp)
        .bind(&timestamp)
        .bind(&timestamp)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(vec![text_event, completed_event])
    }

    pub async fn cancel_run(&self, run_id: &str) -> Result<EventEnvelope, StorageError> {
        self.finish_run_with_event(run_id, RunState::Cancelled, None)
            .await
    }

    pub async fn fail_run(&self, run_id: &str, code: &str) -> Result<EventEnvelope, StorageError> {
        self.finish_run_with_event(run_id, RunState::Failed, Some(code))
            .await
    }

    async fn finish_run_with_event(
        &self,
        run_id: &str,
        next: RunState,
        error_code: Option<&str>,
    ) -> Result<EventEnvelope, StorageError> {
        let mut tx = self.pool.begin().await?;
        let run = fetch_run(&mut tx, run_id).await?;
        transition_run(&mut tx, &run, next, error_code).await?;
        let event_data = if next == RunState::Cancelled {
            PersistedEvent::RunCancelled
        } else {
            PersistedEvent::RunFailed {
                code: error_code.unwrap_or("unknown").to_string(),
            }
        };
        let event = append_event(
            &mut tx,
            &self.scope.instance_id,
            &run.session_id,
            Some(run_id),
            &event_data,
        )
        .await?;
        tx.commit().await?;
        Ok(event)
    }

    pub async fn events_after(
        &self,
        session_id: &str,
        after_seq: i64,
        limit: i64,
    ) -> Result<Vec<EventEnvelope>, StorageError> {
        self.require_session(session_id).await?;
        let rows = sqlx::query(
            "SELECT event_id, instance_id, session_id, run_id, seq, event_type, schema_version, payload_json, created_at \
             FROM events WHERE session_id = ? AND seq > ? ORDER BY seq ASC LIMIT ?",
        )
        .bind(session_id)
        .bind(after_seq.max(0))
        .bind(limit.max(1))
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(event_from_row).collect()
    }

    pub async fn snapshot(&self, session_id: &str) -> Result<SessionSnapshot, StorageError> {
        let last_seq: Option<i64> = sqlx::query_scalar(
            "SELECT last_seq FROM sessions WHERE id = ? AND status != 'deleted'",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(last_seq) = last_seq else {
            return Err(StorageError::SessionNotFound(session_id.to_string()));
        };
        let rows = sqlx::query(
            "SELECT id, session_id, run_id, role, content, status, created_at, client_message_id \
             FROM messages WHERE session_id = ? AND status != 'deleted' ORDER BY created_at ASC, id ASC",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;
        let messages = rows.into_iter().map(message_from_row).collect();
        let active_run = sqlx::query(
            "SELECT id, session_id, trigger_message_id, state, provider, model, identity_version, started_at, completed_at, error_code \
             FROM runs WHERE session_id = ? AND state IN ('queued', 'running', 'cancelling') ORDER BY created_at DESC LIMIT 1",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?
        .map(run_from_row);
        Ok(SessionSnapshot {
            session_id: session_id.to_string(),
            last_seq,
            messages,
            active_run,
        })
    }

    pub async fn recent_messages(
        &self,
        session_id: &str,
        limit: i64,
    ) -> Result<Vec<MessageRecord>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, session_id, run_id, role, content, status, created_at, client_message_id FROM (\
                 SELECT id, session_id, run_id, role, content, status, created_at, client_message_id \
                 FROM messages WHERE session_id = ? AND status = 'complete' AND role IN ('user', 'assistant', 'tool') \
                 ORDER BY created_at DESC, id DESC LIMIT ?\
             ) ORDER BY created_at ASC, id ASC",
        )
        .bind(session_id)
        .bind(limit.max(1))
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(message_from_row).collect())
    }

    pub async fn claim_durable_job(
        &self,
        job_type: &str,
        worker_id: &str,
        lease_for: Duration,
    ) -> Result<Option<DurableJob>, StorageError> {
        let timestamp = now();
        let lease_expires_at = (Utc::now()
            + chrono::Duration::from_std(lease_for)
                .unwrap_or_else(|_| chrono::Duration::minutes(5)))
        .to_rfc3339_opts(SecondsFormat::Millis, true);
        let row = sqlx::query(
            "UPDATE durable_jobs SET status = 'leased', attempts = attempts + 1, \
             lease_owner = ?, lease_expires_at = ?, updated_at = ? WHERE id = ( \
                 SELECT id FROM durable_jobs WHERE job_type = ? AND ( \
                     (status IN ('queued', 'failed') AND available_at <= ?) OR \
                     (status = 'leased' AND lease_expires_at < ?) \
                 ) ORDER BY priority DESC, available_at ASC, created_at ASC LIMIT 1 \
             ) RETURNING id, job_type, payload_json, attempts, max_attempts",
        )
        .bind(worker_id)
        .bind(lease_expires_at)
        .bind(&timestamp)
        .bind(job_type)
        .bind(&timestamp)
        .bind(&timestamp)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| DurableJob {
            id: row.get("id"),
            job_type: row.get("job_type"),
            payload_json: row.get("payload_json"),
            attempts: row.get("attempts"),
            max_attempts: row.get("max_attempts"),
        }))
    }

    pub async fn complete_durable_job(
        &self,
        job_id: &str,
        worker_id: &str,
    ) -> Result<bool, StorageError> {
        let updated = sqlx::query(
            "UPDATE durable_jobs SET status = 'completed', lease_owner = NULL, \
             lease_expires_at = NULL, last_error_code = NULL, updated_at = ? \
             WHERE id = ? AND status = 'leased' AND lease_owner = ?",
        )
        .bind(now())
        .bind(job_id)
        .bind(worker_id)
        .execute(&self.pool)
        .await?;
        Ok(updated.rows_affected() == 1)
    }

    pub async fn fail_durable_job(
        &self,
        job: &DurableJob,
        worker_id: &str,
        error_code: &str,
        retry_after: Duration,
    ) -> Result<&'static str, StorageError> {
        let terminal = job.attempts >= job.max_attempts;
        let status = if terminal { "dead_letter" } else { "failed" };
        let available_at = (Utc::now()
            + chrono::Duration::from_std(retry_after)
                .unwrap_or_else(|_| chrono::Duration::minutes(1)))
        .to_rfc3339_opts(SecondsFormat::Millis, true);
        let updated = sqlx::query(
            "UPDATE durable_jobs SET status = ?, available_at = ?, lease_owner = NULL, \
             lease_expires_at = NULL, last_error_code = ?, updated_at = ? \
             WHERE id = ? AND status = 'leased' AND lease_owner = ?",
        )
        .bind(status)
        .bind(available_at)
        .bind(error_code)
        .bind(now())
        .bind(&job.id)
        .bind(worker_id)
        .execute(&self.pool)
        .await?;
        if updated.rows_affected() != 1 {
            return Ok("lease_lost");
        }
        Ok(status)
    }

    pub async fn load_extraction_source(
        &self,
        run_id: &str,
    ) -> Result<ExtractionSource, StorageError> {
        let row = sqlx::query(
            "SELECT r.id AS run_id, r.session_id, m.id AS user_message_id, m.content AS user_text, \
             m.created_at AS observed_at, \
             (SELECT e.event_id FROM events e WHERE e.session_id = r.session_id \
              AND e.event_type = 'message.accepted' \
              AND json_extract(e.payload_json, '$.message_id') = m.id LIMIT 1) AS source_event_id \
             FROM runs r JOIN messages m ON m.id = r.trigger_message_id WHERE r.id = ?",
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| StorageError::RunNotFound(run_id.to_string()))?;
        Ok(ExtractionSource {
            run_id: row.get("run_id"),
            session_id: row.get("session_id"),
            user_message_id: row.get("user_message_id"),
            user_text: row.get("user_text"),
            observed_at: row.get("observed_at"),
            source_event_id: row.get("source_event_id"),
        })
    }

    pub async fn record_memory_candidate(
        &self,
        source: &ExtractionSource,
        draft: &ExtractedMemory,
    ) -> Result<RecordedCandidate, StorageError> {
        let decision = evaluate(&source.user_text, draft);
        let candidate_id = new_id();
        let timestamp = now();
        let draft_json = serde_json::to_string(draft)?;
        let mut tx = self.pool.begin().await?;
        let inserted = sqlx::query(
            "INSERT INTO memory_candidates( \
                 id, instance_id, user_id, session_id, source_run_id, source_message_id, \
                 extractor_version, memory_type, scope, subject, predicate, value, canonical_key, \
                 evidence_quote, explicitness, confidence, importance, sensitivity, valid_from, \
                 valid_to, status, decision_reason, draft_json, decided_at, created_at \
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(source_message_id, canonical_key, value) DO NOTHING",
        )
        .bind(&candidate_id)
        .bind(&self.scope.instance_id)
        .bind(&self.scope.user_id)
        .bind(&source.session_id)
        .bind(&source.run_id)
        .bind(&source.user_message_id)
        .bind(EXTRACTOR_VERSION)
        .bind(draft.memory_type.as_str())
        .bind(draft.scope.as_str())
        .bind(draft.subject.trim())
        .bind(draft.predicate.trim())
        .bind(draft.value.trim())
        .bind(&decision.canonical_key)
        .bind(draft.evidence_quote.trim())
        .bind(draft.explicitness.as_str())
        .bind(f64::from(draft.confidence))
        .bind(f64::from(draft.importance))
        .bind(draft.sensitivity.as_str())
        .bind(draft.valid_from.as_deref())
        .bind(draft.valid_to.as_deref())
        .bind(decision.disposition.as_str())
        .bind(decision.reason)
        .bind(draft_json)
        .bind(&timestamp)
        .bind(&timestamp)
        .execute(&mut *tx)
        .await?;

        if inserted.rows_affected() == 0 {
            let row = sqlx::query(
                "SELECT c.id, c.status, i.id AS item_id, i.version, i.supersedes_id \
                 FROM memory_candidates c LEFT JOIN memory_items i ON i.source_candidate_id = c.id \
                 WHERE c.source_message_id = ? AND c.canonical_key = ? AND c.value = ?",
            )
            .bind(&source.user_message_id)
            .bind(&decision.canonical_key)
            .bind(draft.value.trim())
            .fetch_one(&mut *tx)
            .await?;
            tx.commit().await?;
            return Ok(RecordedCandidate {
                candidate_id: row.get("id"),
                status: row.get("status"),
                item_id: row.get("item_id"),
                item_version: row.get("version"),
                supersedes_id: row.get("supersedes_id"),
                inserted: false,
            });
        }

        append_event(
            &mut tx,
            &self.scope.instance_id,
            &source.session_id,
            Some(&source.run_id),
            &PersistedEvent::MemoryCandidateEvaluated {
                candidate_id: candidate_id.clone(),
                source_message_id: source.user_message_id.clone(),
                status: decision.disposition.as_str().to_string(),
                reason: decision.reason.to_string(),
            },
        )
        .await?;

        let mut item_id = None;
        let mut item_version = None;
        let mut supersedes_id = None;
        if decision.disposition == CandidateDisposition::Commit {
            let (next_item_id, version, previous_id) = commit_memory_item(
                &mut tx,
                &self.scope,
                source,
                &candidate_id,
                draft,
                &decision.canonical_key,
                &timestamp,
            )
            .await?;
            item_id = Some(next_item_id);
            item_version = Some(version);
            supersedes_id = previous_id;
        }
        tx.commit().await?;
        Ok(RecordedCandidate {
            candidate_id,
            status: decision.disposition.as_str().to_string(),
            item_id,
            item_version,
            supersedes_id,
            inserted: true,
        })
    }

    pub async fn list_memory_items(
        &self,
        include_history: bool,
    ) -> Result<Vec<MemoryItemView>, StorageError> {
        let rows = sqlx::query(
            "SELECT m.id, m.memory_type, m.scope, m.subject, m.predicate, m.value, \
                    m.sensitivity, m.status, m.version, c.source_message_id, c.evidence_quote, \
                    m.valid_from, m.valid_to, m.never_recall, m.pinned, m.created_at, m.updated_at \
             FROM memory_items m JOIN memory_candidates c ON c.id = m.source_candidate_id \
             WHERE m.user_id = ? AND (? = 1 OR m.status IN ('active', 'disputed')) \
             ORDER BY m.pinned DESC, m.updated_at DESC, m.id DESC LIMIT 500",
        )
        .bind(&self.scope.user_id)
        .bind(i64::from(include_history))
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(memory_item_from_row).collect())
    }

    pub async fn list_pending_memory(&self) -> Result<Vec<PendingMemoryView>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, draft_json, decision_reason, source_message_id, created_at \
             FROM memory_candidates WHERE user_id = ? AND status = 'pending' \
             ORDER BY created_at DESC, id DESC LIMIT 200",
        )
        .bind(&self.scope.user_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(PendingMemoryView {
                    id: row.get("id"),
                    draft: serde_json::from_str(&row.get::<String, _>("draft_json"))?,
                    decision_reason: row.get("decision_reason"),
                    source_message_id: row.get("source_message_id"),
                    created_at: row.get("created_at"),
                })
            })
            .collect()
    }

    pub async fn confirm_memory_candidate(
        &self,
        candidate_id: &str,
    ) -> Result<RecordedCandidate, StorageError> {
        let timestamp = now();
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "SELECT c.status, c.draft_json, c.canonical_key, c.source_run_id, c.session_id, \
                    c.source_message_id, m.content AS user_text, m.created_at AS observed_at, \
                    (SELECT e.event_id FROM events e WHERE e.session_id = c.session_id \
                     AND e.event_type = 'message.accepted' \
                     AND json_extract(e.payload_json, '$.message_id') = c.source_message_id LIMIT 1) AS source_event_id \
             FROM memory_candidates c JOIN messages m ON m.id = c.source_message_id \
             WHERE c.id = ? AND c.user_id = ?",
        )
        .bind(candidate_id)
        .bind(&self.scope.user_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| StorageError::MemoryCandidateNotFound(candidate_id.to_string()))?;
        let status: String = row.get("status");
        if status != "pending" {
            return Err(StorageError::InvalidMemoryCandidateStatus {
                id: candidate_id.to_string(),
                status,
            });
        }
        let draft: ExtractedMemory = serde_json::from_str(&row.get::<String, _>("draft_json"))?;
        let canonical_key: String = row.get("canonical_key");
        let source = ExtractionSource {
            run_id: row.get("source_run_id"),
            session_id: row.get("session_id"),
            user_message_id: row.get("source_message_id"),
            user_text: row.get("user_text"),
            observed_at: row.get("observed_at"),
            source_event_id: row.get("source_event_id"),
        };
        sqlx::query(
            "UPDATE memory_candidates SET status = 'committed', decision_reason = 'user_confirmed', \
             decided_at = ? WHERE id = ?",
        )
        .bind(&timestamp)
        .bind(candidate_id)
        .execute(&mut *tx)
        .await?;
        append_event(
            &mut tx,
            &self.scope.instance_id,
            &source.session_id,
            Some(&source.run_id),
            &PersistedEvent::MemoryCandidateEvaluated {
                candidate_id: candidate_id.to_string(),
                source_message_id: source.user_message_id.clone(),
                status: "committed".to_string(),
                reason: "user_confirmed".to_string(),
            },
        )
        .await?;
        let (item_id, version, supersedes_id) = commit_memory_item(
            &mut tx,
            &self.scope,
            &source,
            candidate_id,
            &draft,
            &canonical_key,
            &timestamp,
        )
        .await?;
        tx.commit().await?;
        Ok(RecordedCandidate {
            candidate_id: candidate_id.to_string(),
            status: "committed".to_string(),
            item_id: Some(item_id),
            item_version: Some(version),
            supersedes_id,
            inserted: true,
        })
    }

    pub async fn reject_memory_candidate(&self, candidate_id: &str) -> Result<(), StorageError> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "SELECT status, source_run_id, session_id, source_message_id FROM memory_candidates \
             WHERE id = ? AND user_id = ?",
        )
        .bind(candidate_id)
        .bind(&self.scope.user_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| StorageError::MemoryCandidateNotFound(candidate_id.to_string()))?;
        let status: String = row.get("status");
        if status != "pending" {
            return Err(StorageError::InvalidMemoryCandidateStatus {
                id: candidate_id.to_string(),
                status,
            });
        }
        sqlx::query(
            "UPDATE memory_candidates SET status = 'rejected', decision_reason = 'user_rejected', \
             decided_at = ? WHERE id = ?",
        )
        .bind(now())
        .bind(candidate_id)
        .execute(&mut *tx)
        .await?;
        append_event(
            &mut tx,
            &self.scope.instance_id,
            &row.get::<String, _>("session_id"),
            Some(&row.get::<String, _>("source_run_id")),
            &PersistedEvent::MemoryCandidateEvaluated {
                candidate_id: candidate_id.to_string(),
                source_message_id: row.get("source_message_id"),
                status: "rejected".to_string(),
                reason: "user_rejected".to_string(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn update_memory_item(
        &self,
        item_id: &str,
        pinned: Option<bool>,
        never_recall: Option<bool>,
        next_status: Option<&str>,
    ) -> Result<MemoryItemView, StorageError> {
        if let Some(status) = next_status
            && !matches!(status, "disputed" | "retracted")
        {
            return Err(StorageError::InvalidMemoryItemStatus(status.to_string()));
        }
        let mut tx = self.pool.begin().await?;
        let source = sqlx::query(
            "SELECT m.status, m.pinned, m.never_recall, c.session_id, c.source_run_id \
             FROM memory_items m JOIN memory_candidates c ON c.id = m.source_candidate_id \
             WHERE m.id = ? AND m.user_id = ?",
        )
        .bind(item_id)
        .bind(&self.scope.user_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| StorageError::MemoryItemNotFound(item_id.to_string()))?;
        let current_status: String = source.get("status");
        let transition_allowed = matches!(
            (current_status.as_str(), next_status),
            (_, None)
                | ("active", Some("disputed" | "retracted"))
                | ("disputed", Some("retracted"))
        );
        if !transition_allowed {
            return Err(StorageError::InvalidMemoryCandidateStatus {
                id: item_id.to_string(),
                status: current_status,
            });
        }
        let next_pinned = pinned.unwrap_or_else(|| source.get::<i64, _>("pinned") != 0);
        let next_never = never_recall.unwrap_or_else(|| source.get::<i64, _>("never_recall") != 0);
        let status = next_status.unwrap_or(&current_status);
        sqlx::query(
            "UPDATE memory_items SET pinned = ?, never_recall = ?, status = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(i64::from(next_pinned))
        .bind(i64::from(next_never))
        .bind(status)
        .bind(now())
        .bind(item_id)
        .execute(&mut *tx)
        .await?;
        if status != "active" {
            sqlx::query("DELETE FROM memory_items_fts WHERE memory_id = ?")
                .bind(item_id)
                .execute(&mut *tx)
                .await?;
        }
        append_event(
            &mut tx,
            &self.scope.instance_id,
            &source.get::<String, _>("session_id"),
            Some(&source.get::<String, _>("source_run_id")),
            &PersistedEvent::MemoryItemStatusChanged {
                item_id: item_id.to_string(),
                status: status.to_string(),
                pinned: next_pinned,
                never_recall: next_never,
            },
        )
        .await?;
        tx.commit().await?;
        self.get_memory_item(item_id).await
    }

    pub async fn get_memory_item(&self, item_id: &str) -> Result<MemoryItemView, StorageError> {
        let row = sqlx::query(
            "SELECT m.id, m.memory_type, m.scope, m.subject, m.predicate, m.value, \
                    m.sensitivity, m.status, m.version, c.source_message_id, c.evidence_quote, \
                    m.valid_from, m.valid_to, m.never_recall, m.pinned, m.created_at, m.updated_at \
             FROM memory_items m JOIN memory_candidates c ON c.id = m.source_candidate_id \
             WHERE m.id = ? AND m.user_id = ?",
        )
        .bind(item_id)
        .bind(&self.scope.user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| StorageError::MemoryItemNotFound(item_id.to_string()))?;
        Ok(memory_item_from_row(row))
    }

    pub async fn retrieve_memory_evidence(
        &self,
        session_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<crate::context::MemoryEvidence>, StorageError> {
        self.require_session(session_id).await?;
        let Some(fts_query) = memory_fts_query(query) else {
            return Ok(Vec::new());
        };
        let current_time = now();
        let as_of = memory_as_of(query);
        let historical = i64::from(as_of.is_some());
        let as_of = as_of.as_deref().unwrap_or(&current_time);
        let rows = sqlx::query(
            "SELECT m.id, m.memory_type, m.scope, m.subject, m.predicate, m.value, \
                    m.version, m.importance, m.pinned, m.valid_from, m.valid_to, \
                    bm25(memory_items_fts) AS text_rank, \
                    (SELECT group_concat(DISTINCT event_id) FROM memory_links l \
                     WHERE l.memory_id = m.id AND l.event_id IS NOT NULL) AS source_event_ids \
             FROM memory_items_fts JOIN memory_items m ON m.id = memory_items_fts.memory_id \
             WHERE memory_items_fts MATCH ? AND m.user_id = ? \
               AND m.never_recall = 0 \
               AND (m.scope != 'session' OR m.session_id = ?) \
               AND ((? = 0 AND m.status = 'active' \
                     AND (m.valid_from IS NULL OR m.valid_from <= ?) \
                     AND (m.valid_to IS NULL OR m.valid_to > ?)) \
                    OR (? = 1 AND m.status IN ('active', 'superseded') \
                        AND COALESCE(m.valid_from, m.created_at) <= ? \
                        AND (m.valid_to IS NULL OR m.valid_to > ?))) \
             ORDER BY text_rank ASC, m.pinned DESC, m.importance DESC, m.updated_at DESC \
             LIMIT ?",
        )
        .bind(fts_query)
        .bind(&self.scope.user_id)
        .bind(session_id)
        .bind(historical)
        .bind(&current_time)
        .bind(&current_time)
        .bind(historical)
        .bind(as_of)
        .bind(as_of)
        .bind(i64::try_from(limit.clamp(1, 24)).unwrap_or(24))
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .enumerate()
            .map(|(index, row)| {
                let importance = row.get::<f64, _>("importance") as f32;
                let pinned = row.get::<i64, _>("pinned") != 0;
                let scope = row.get::<String, _>("scope");
                let reciprocal_rank = 1.0 / (60.0 + index as f32);
                let score = reciprocal_rank
                    + importance.clamp(0.0, 1.0) * 0.002
                    + if pinned { 0.01 } else { 0.0 }
                    + if scope == "session" { 0.003 } else { 0.0 };
                let valid_from: Option<String> = row.get("valid_from");
                let valid_to: Option<String> = row.get("valid_to");
                let temporal = match (valid_from, valid_to) {
                    (None, None) => String::new(),
                    (from, to) => format!(
                        " (valid_from={}, valid_to={})",
                        from.as_deref().unwrap_or("unspecified"),
                        to.as_deref().unwrap_or("open")
                    ),
                };
                let event_ids = row
                    .get::<Option<String>, _>("source_event_ids")
                    .unwrap_or_default()
                    .split(',')
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .collect();
                crate::context::MemoryEvidence {
                    id: row.get("id"),
                    version: row.get::<i64, _>("version").to_string(),
                    text: format!(
                        "[type={}, scope={}] {} {} {}{}",
                        row.get::<String, _>("memory_type"),
                        scope,
                        row.get::<String, _>("subject"),
                        row.get::<String, _>("predicate"),
                        row.get::<String, _>("value"),
                        temporal
                    ),
                    source_event_ids: event_ids,
                    score,
                }
            })
            .collect())
    }

    pub async fn retrieve_memory_for_message(
        &self,
        session_id: &str,
        message_id: &str,
        limit: usize,
    ) -> Result<Vec<crate::context::MemoryEvidence>, StorageError> {
        let content: Option<String> = sqlx::query_scalar(
            "SELECT content FROM messages WHERE id = ? AND session_id = ? \
             AND role = 'user' AND status = 'complete'",
        )
        .bind(message_id)
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(content) = content else {
            return Ok(Vec::new());
        };
        self.retrieve_memory_evidence(session_id, &content, limit)
            .await
    }

    pub async fn rebuild_memory_search_index(&self) -> Result<u64, StorageError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM memory_items_fts")
            .execute(&mut *tx)
            .await?;
        let inserted = sqlx::query(
            "INSERT INTO memory_items_fts(memory_id, user_id, searchable_text) \
             SELECT id, user_id, subject || ' ' || predicate || ' ' || value \
             FROM memory_items WHERE status IN ('active', 'superseded')",
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(inserted.rows_affected())
    }

    pub async fn import_completed_exchange(
        &self,
        source_key: &str,
        source_index: usize,
        user_content: &str,
        assistant_content: &str,
    ) -> Result<bool, StorageError> {
        let client_message_id = format!("legacy:{source_key}:{source_index}");
        let session_id = self.scope.session_id.clone();
        let accepted = self
            .accept_user_input(
                &session_id,
                &client_message_id,
                user_content,
                "legacy-import",
                "legacy-markdown",
                "legacy-prompt-v1",
            )
            .await?;
        match parse_run_state(&accepted.run.state)? {
            RunState::Queued => {
                self.start_run(&accepted.run.id).await?;
                self.complete_reply(
                    &accepted.run.id,
                    assistant_content,
                    "imported",
                    CompletionUsage::default(),
                )
                .await?;
                Ok(!accepted.deduplicated)
            }
            RunState::Running | RunState::Cancelling => {
                self.complete_reply(
                    &accepted.run.id,
                    assistant_content,
                    "imported",
                    CompletionUsage::default(),
                )
                .await?;
                Ok(!accepted.deduplicated)
            }
            RunState::Completed => Ok(false),
            RunState::Failed | RunState::Cancelled => Err(StorageError::InvalidRunTransition {
                from: accepted.run.state,
                to: RunState::Completed.as_str(),
            }),
        }
    }

    pub async fn activate_identity(
        &self,
        identity: &IdentityKernel,
    ) -> Result<Option<EventEnvelope>, StorageError> {
        let mut tx = self.pool.begin().await?;
        let timestamp = now();
        let manifest_json = serde_json::to_string(&identity.manifest_json())?;
        sqlx::query(
            "INSERT INTO identity_versions(version, manifest_json, content_hash, created_at) \
             VALUES (?, ?, ?, ?) ON CONFLICT(version) DO NOTHING",
        )
        .bind(identity.version())
        .bind(manifest_json)
        .bind(identity.content_hash())
        .bind(&timestamp)
        .execute(&mut *tx)
        .await?;
        let stored_hash: String =
            sqlx::query_scalar("SELECT content_hash FROM identity_versions WHERE version = ?")
                .bind(identity.version())
                .fetch_one(&mut *tx)
                .await?;
        if stored_hash != identity.content_hash() {
            return Err(StorageError::IdentityVersionConflict {
                version: identity.version().to_string(),
            });
        }
        let active_version: String =
            sqlx::query_scalar("SELECT identity_version FROM instances WHERE id = ?")
                .bind(&self.scope.instance_id)
                .fetch_one(&mut *tx)
                .await?;
        if active_version == identity.version() {
            tx.commit().await?;
            return Ok(None);
        }
        sqlx::query("UPDATE instances SET identity_version = ?, updated_at = ? WHERE id = ?")
            .bind(identity.version())
            .bind(&timestamp)
            .bind(&self.scope.instance_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE identity_versions SET activated_at = ? WHERE version = ?")
            .bind(&timestamp)
            .bind(identity.version())
            .execute(&mut *tx)
            .await?;
        let event = append_event(
            &mut tx,
            &self.scope.instance_id,
            &self.scope.session_id,
            None,
            &PersistedEvent::IdentityActivated {
                version: identity.version().to_string(),
                content_hash: identity.content_hash().to_string(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(Some(event))
    }

    pub async fn save_context_plan(
        &self,
        plan: &ContextPlan,
    ) -> Result<EventEnvelope, StorageError> {
        let mut tx = self.pool.begin().await?;
        let selected = serde_json::to_string(&plan.selected_items)?;
        let dropped = serde_json::to_string(&plan.dropped_items)?;
        sqlx::query(
            "INSERT INTO context_plans(id, run_id, identity_version, compiler_version, prompt_hash, \
             selected_items_json, dropped_items_json, estimated_tokens, created_at, stable_prefix_hash, estimated) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&plan.id)
        .bind(&plan.run_id)
        .bind(&plan.identity_version)
        .bind(&plan.compiler_version)
        .bind(&plan.prompt_hash)
        .bind(selected)
        .bind(dropped)
        .bind(i64::from(plan.estimated_tokens))
        .bind(now())
        .bind(&plan.stable_prefix_hash)
        .bind(i64::from(plan.estimated))
        .execute(&mut *tx)
        .await?;
        let updated =
            sqlx::query("UPDATE runs SET context_plan_id = ?, identity_version = ? WHERE id = ?")
                .bind(&plan.id)
                .bind(&plan.identity_version)
                .bind(&plan.run_id)
                .execute(&mut *tx)
                .await?;
        if updated.rows_affected() != 1 {
            return Err(StorageError::RunNotFound(plan.run_id.clone()));
        }
        let session_id: String = sqlx::query_scalar("SELECT session_id FROM runs WHERE id = ?")
            .bind(&plan.run_id)
            .fetch_one(&mut *tx)
            .await?;
        let event = append_event(
            &mut tx,
            &self.scope.instance_id,
            &session_id,
            Some(&plan.run_id),
            &PersistedEvent::ContextCompiled {
                plan_id: plan.id.clone(),
                prompt_hash: plan.prompt_hash.clone(),
                identity_version: plan.identity_version.clone(),
                estimated_tokens: plan.estimated_tokens,
                estimated: plan.estimated,
                dropped_items: plan.dropped_items.len(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(event)
    }

    pub async fn load_context_plan(
        &self,
        run_id: &str,
    ) -> Result<Option<ContextPlan>, StorageError> {
        let row = sqlx::query(
            "SELECT id, run_id, identity_version, compiler_version, prompt_hash, stable_prefix_hash, \
             estimated_tokens, estimated, selected_items_json, dropped_items_json \
             FROM context_plans WHERE run_id = ?",
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            Ok(ContextPlan {
                id: row.get("id"),
                run_id: row.get("run_id"),
                identity_version: row.get("identity_version"),
                compiler_version: row.get("compiler_version"),
                prompt_hash: row.get("prompt_hash"),
                stable_prefix_hash: row.get("stable_prefix_hash"),
                estimated_tokens: u32::try_from(row.get::<i64, _>("estimated_tokens"))
                    .unwrap_or(u32::MAX),
                estimated: row.get::<i64, _>("estimated") != 0,
                selected_items: serde_json::from_str(&row.get::<String, _>("selected_items_json"))?,
                dropped_items: serde_json::from_str(&row.get::<String, _>("dropped_items_json"))?,
            })
        })
        .transpose()
    }

    pub async fn latest_self_state(&self) -> Result<Option<SelfState>, StorageError> {
        self.latest_state("self").await
    }

    pub async fn latest_relationship_state(
        &self,
    ) -> Result<Option<RelationshipState>, StorageError> {
        self.latest_state("relationship").await
    }

    async fn latest_state<T: serde::de::DeserializeOwned>(
        &self,
        state_type: &str,
    ) -> Result<Option<T>, StorageError> {
        let state: Option<String> = sqlx::query_scalar(
            "SELECT state_json FROM state_snapshots WHERE instance_id = ? AND state_type = ? \
             ORDER BY version DESC, created_at DESC LIMIT 1",
        )
        .bind(&self.scope.instance_id)
        .bind(state_type)
        .fetch_optional(&self.pool)
        .await?;
        state
            .map(|value| serde_json::from_str(&value).map_err(StorageError::from))
            .transpose()
    }

    #[cfg(test)]
    async fn save_state<T: serde::Serialize>(
        &self,
        state_type: &str,
        state: &T,
    ) -> Result<(), StorageError> {
        let version: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version), 0) + 1 FROM state_snapshots WHERE instance_id = ? AND state_type = ?",
        )
        .bind(&self.scope.instance_id)
        .bind(state_type)
        .fetch_one(&self.pool)
        .await?;
        sqlx::query(
            "INSERT INTO state_snapshots(id, instance_id, state_type, state_json, version, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(new_id())
        .bind(&self.scope.instance_id)
        .bind(state_type)
        .bind(serde_json::to_string(state)?)
        .bind(version)
        .bind(now())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn recover_interrupted_runs(&self) -> Result<(), StorageError> {
        let mut tx = self.pool.begin().await?;
        let rows = sqlx::query(
            "SELECT id, session_id, trigger_message_id, state, provider, model, identity_version, started_at, completed_at, error_code \
             FROM runs WHERE state IN ('queued', 'running', 'cancelling') ORDER BY created_at ASC",
        )
        .fetch_all(&mut *tx)
        .await?;
        for row in rows {
            let run = run_from_row(row);
            transition_run(&mut tx, &run, RunState::Failed, Some("kernel_restarted")).await?;
            append_event(
                &mut tx,
                &self.scope.instance_id,
                &run.session_id,
                Some(&run.id),
                &PersistedEvent::RunFailed {
                    code: "kernel_restarted".to_string(),
                },
            )
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn require_session(&self, session_id: &str) -> Result<(), StorageError> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM sessions WHERE id = ? AND instance_id = ? AND status != 'deleted')",
        )
        .bind(session_id)
        .bind(&self.scope.instance_id)
        .fetch_one(&self.pool)
        .await?;
        if exists {
            Ok(())
        } else {
            Err(StorageError::SessionNotFound(session_id.to_string()))
        }
    }
}

async fn ensure_default_scope(pool: &SqlitePool) -> Result<StorageScope, StorageError> {
    let mut tx = pool.begin().await?;
    let active = sqlx::query("SELECT id FROM instances WHERE status = 'active' LIMIT 1")
        .fetch_optional(&mut *tx)
        .await?;
    let instance_id = if let Some(row) = active {
        row.get("id")
    } else {
        let id = new_id();
        let timestamp = now();
        sqlx::query(
            "INSERT INTO instances(id, display_name, identity_version, status, settings_json, created_at, updated_at) \
             VALUES (?, 'Lain', 'legacy-prompt-v1', 'active', '{}', ?, ?)",
        )
        .bind(&id)
        .bind(&timestamp)
        .bind(&timestamp)
        .execute(&mut *tx)
        .await?;
        id
    };
    let user_id = match sqlx::query(
        "SELECT id FROM users WHERE instance_id = ? ORDER BY created_at LIMIT 1",
    )
    .bind(&instance_id)
    .fetch_optional(&mut *tx)
    .await?
    {
        Some(row) => row.get("id"),
        None => {
            let id = new_id();
            let timestamp = now();
            sqlx::query(
                "INSERT INTO users(id, instance_id, display_name, created_at, updated_at) VALUES (?, ?, 'User', ?, ?)",
            )
            .bind(&id)
            .bind(&instance_id)
            .bind(&timestamp)
            .bind(&timestamp)
            .execute(&mut *tx)
            .await?;
            id
        }
    };
    let session_id = match sqlx::query(
        "SELECT id FROM sessions WHERE instance_id = ? AND user_id = ? AND status = 'active' ORDER BY updated_at DESC LIMIT 1",
    )
    .bind(&instance_id)
    .bind(&user_id)
    .fetch_optional(&mut *tx)
    .await?
    {
        Some(row) => row.get("id"),
        None => {
            let id = new_id();
            let timestamp = now();
            sqlx::query(
                "INSERT INTO sessions(id, instance_id, user_id, title, status, created_at, updated_at) \
                 VALUES (?, ?, ?, 'Main', 'active', ?, ?)",
            )
            .bind(&id)
            .bind(&instance_id)
            .bind(&user_id)
            .bind(&timestamp)
            .bind(&timestamp)
            .execute(&mut *tx)
            .await?;
            id
        }
    };
    tx.commit().await?;
    Ok(StorageScope {
        instance_id,
        user_id,
        session_id,
    })
}

async fn append_event(
    tx: &mut Transaction<'_, Sqlite>,
    instance_id: &str,
    session_id: &str,
    run_id: Option<&str>,
    event: &PersistedEvent,
) -> Result<EventEnvelope, StorageError> {
    let timestamp = now();
    let seq: i64 = sqlx::query_scalar(
        "UPDATE sessions SET last_seq = last_seq + 1, updated_at = ? WHERE id = ? RETURNING last_seq",
    )
    .bind(&timestamp)
    .bind(session_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| StorageError::SessionNotFound(session_id.to_string()))?;
    let event_id = new_id();
    let payload = event.payload()?;
    let payload_json = serde_json::to_string(&payload)?;
    sqlx::query(
        "INSERT INTO events(event_id, instance_id, session_id, run_id, seq, event_type, schema_version, payload_json, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&event_id)
    .bind(instance_id)
    .bind(session_id)
    .bind(run_id)
    .bind(seq)
    .bind(event.event_type())
    .bind(i64::from(PROTOCOL_VERSION))
    .bind(payload_json)
    .bind(&timestamp)
    .execute(&mut **tx)
    .await?;
    Ok(EventEnvelope {
        v: PROTOCOL_VERSION,
        event_id,
        instance_id: instance_id.to_string(),
        session_id: Some(session_id.to_string()),
        run_id: run_id.map(ToString::to_string),
        seq: Some(seq),
        event_type: event.event_type().to_string(),
        ts: timestamp,
        payload,
    })
}

async fn transition_run(
    tx: &mut Transaction<'_, Sqlite>,
    run: &RunRecord,
    next: RunState,
    error_code: Option<&str>,
) -> Result<(), StorageError> {
    let current = parse_run_state(&run.state)?;
    if !current.can_transition_to(next) {
        return Err(StorageError::InvalidRunTransition {
            from: run.state.clone(),
            to: next.as_str(),
        });
    }
    let timestamp = now();
    let started_at = (next == RunState::Running).then_some(timestamp.as_str());
    let completed_at = next.is_terminal().then_some(timestamp.as_str());
    sqlx::query(
        "UPDATE runs SET state = ?, started_at = COALESCE(started_at, ?), completed_at = ?, error_code = ? WHERE id = ?",
    )
    .bind(next.as_str())
    .bind(started_at)
    .bind(completed_at)
    .bind(error_code)
    .bind(&run.id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn parse_run_state(value: &str) -> Result<RunState, StorageError> {
    RunState::try_from(value).map_err(|_| StorageError::InvalidStoredRunState(value.to_string()))
}

async fn fetch_message_by_client_id(
    tx: &mut Transaction<'_, Sqlite>,
    session_id: &str,
    client_message_id: &str,
) -> Result<MessageRecord, StorageError> {
    let row = sqlx::query(
        "SELECT id, session_id, run_id, role, content, status, created_at, client_message_id \
         FROM messages WHERE session_id = ? AND client_message_id = ?",
    )
    .bind(session_id)
    .bind(client_message_id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(message_from_row(row))
}

async fn fetch_run_for_trigger(
    tx: &mut Transaction<'_, Sqlite>,
    message_id: &str,
) -> Result<RunRecord, StorageError> {
    let row = sqlx::query(
        "SELECT id, session_id, trigger_message_id, state, provider, model, identity_version, started_at, completed_at, error_code \
         FROM runs WHERE trigger_message_id = ?",
    )
    .bind(message_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| StorageError::RunNotFound(message_id.to_string()))?;
    Ok(run_from_row(row))
}

async fn fetch_run(
    tx: &mut Transaction<'_, Sqlite>,
    run_id: &str,
) -> Result<RunRecord, StorageError> {
    let row = sqlx::query(
        "SELECT id, session_id, trigger_message_id, state, provider, model, identity_version, started_at, completed_at, error_code \
         FROM runs WHERE id = ?",
    )
    .bind(run_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| StorageError::RunNotFound(run_id.to_string()))?;
    Ok(run_from_row(row))
}

async fn fetch_events_for_run(
    tx: &mut Transaction<'_, Sqlite>,
    run_id: &str,
) -> Result<Vec<EventEnvelope>, StorageError> {
    let rows = sqlx::query(
        "SELECT event_id, instance_id, session_id, run_id, seq, event_type, schema_version, payload_json, created_at \
         FROM events WHERE run_id = ? ORDER BY seq ASC",
    )
    .bind(run_id)
    .fetch_all(&mut **tx)
    .await?;
    rows.into_iter().map(event_from_row).collect()
}

async fn commit_memory_item(
    tx: &mut Transaction<'_, Sqlite>,
    scope: &StorageScope,
    source: &ExtractionSource,
    candidate_id: &str,
    draft: &ExtractedMemory,
    canonical_key: &str,
    timestamp: &str,
) -> Result<(String, i64, Option<String>), StorageError> {
    let scoped_session = if draft.scope.as_str() == "session" {
        Some(source.session_id.as_str())
    } else {
        None
    };
    let previous = sqlx::query(
        "SELECT id, version FROM memory_items WHERE user_id = ? AND scope = ? \
         AND ifnull(session_id, '') = ifnull(?, '') AND canonical_key = ? AND status = 'active'",
    )
    .bind(&scope.user_id)
    .bind(draft.scope.as_str())
    .bind(scoped_session)
    .bind(canonical_key)
    .fetch_optional(&mut **tx)
    .await?;
    let version = previous
        .as_ref()
        .map(|row| row.get::<i64, _>("version") + 1)
        .unwrap_or(1);
    let previous_id = previous.as_ref().map(|row| row.get::<String, _>("id"));
    if let Some(previous_id) = &previous_id {
        let previous_valid_to = draft.valid_from.as_deref().unwrap_or(timestamp);
        sqlx::query(
            "UPDATE memory_items \
             SET status = 'superseded', valid_to = COALESCE(valid_to, ?), updated_at = ? \
             WHERE id = ?",
        )
        .bind(previous_valid_to)
        .bind(timestamp)
        .bind(previous_id)
        .execute(&mut **tx)
        .await?;
    }
    let next_item_id = new_id();
    sqlx::query(
        "INSERT INTO memory_items( \
             id, instance_id, user_id, session_id, source_candidate_id, memory_type, scope, \
             subject, predicate, value, canonical_key, sensitivity, explicitness, confidence, \
             importance, status, version, supersedes_id, valid_from, valid_to, created_at, updated_at \
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'active', ?, ?, ?, ?, ?, ?)",
    )
    .bind(&next_item_id)
    .bind(&scope.instance_id)
    .bind(&scope.user_id)
    .bind(scoped_session)
    .bind(candidate_id)
    .bind(draft.memory_type.as_str())
    .bind(draft.scope.as_str())
    .bind(draft.subject.trim())
    .bind(draft.predicate.trim())
    .bind(draft.value.trim())
    .bind(canonical_key)
    .bind(draft.sensitivity.as_str())
    .bind(draft.explicitness.as_str())
    .bind(f64::from(draft.confidence))
    .bind(f64::from(draft.importance))
    .bind(version)
    .bind(previous_id.as_deref())
    .bind(draft.valid_from.as_deref())
    .bind(draft.valid_to.as_deref())
    .bind(timestamp)
    .bind(timestamp)
    .execute(&mut **tx)
    .await?;
    let searchable = format!(
        "{} {} {}",
        draft.subject.trim(),
        draft.predicate.trim(),
        draft.value.trim()
    );
    sqlx::query(
        "INSERT INTO memory_items_fts(memory_id, user_id, searchable_text) VALUES (?, ?, ?)",
    )
    .bind(&next_item_id)
    .bind(&scope.user_id)
    .bind(searchable)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO memory_links(id, memory_id, link_type, event_id, message_id, metadata_json, created_at) \
         VALUES (?, ?, 'source', ?, ?, '{}', ?)",
    )
    .bind(new_id())
    .bind(&next_item_id)
    .bind(source.source_event_id.as_deref())
    .bind(&source.user_message_id)
    .bind(timestamp)
    .execute(&mut **tx)
    .await?;
    if let Some(previous_id) = &previous_id {
        sqlx::query(
            "INSERT INTO memory_links(id, memory_id, link_type, related_memory_id, metadata_json, created_at) \
             VALUES (?, ?, 'supersedes', ?, '{}', ?)",
        )
        .bind(new_id())
        .bind(&next_item_id)
        .bind(previous_id)
        .bind(timestamp)
        .execute(&mut **tx)
        .await?;
    }
    append_event(
        tx,
        &scope.instance_id,
        &source.session_id,
        Some(&source.run_id),
        &PersistedEvent::MemoryItemCommitted {
            item_id: next_item_id.clone(),
            candidate_id: candidate_id.to_string(),
            version,
            supersedes_id: previous_id.clone(),
        },
    )
    .await?;
    Ok((next_item_id, version, previous_id))
}

fn memory_item_from_row(row: sqlx::sqlite::SqliteRow) -> MemoryItemView {
    MemoryItemView {
        id: row.get("id"),
        memory_type: row.get("memory_type"),
        scope: row.get("scope"),
        subject: row.get("subject"),
        predicate: row.get("predicate"),
        value: row.get("value"),
        sensitivity: row.get("sensitivity"),
        status: row.get("status"),
        version: row.get("version"),
        source_message_id: row.get("source_message_id"),
        evidence_quote: row.get("evidence_quote"),
        valid_from: row.get("valid_from"),
        valid_to: row.get("valid_to"),
        never_recall: row.get::<i64, _>("never_recall") != 0,
        pinned: row.get::<i64, _>("pinned") != 0,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn message_from_row(row: sqlx::sqlite::SqliteRow) -> MessageRecord {
    MessageRecord {
        id: row.get("id"),
        session_id: row.get("session_id"),
        run_id: row.get("run_id"),
        role: row.get("role"),
        content: row.get("content"),
        status: row.get("status"),
        created_at: row.get("created_at"),
        client_message_id: row.get("client_message_id"),
    }
}

fn run_from_row(row: sqlx::sqlite::SqliteRow) -> RunRecord {
    RunRecord {
        id: row.get("id"),
        session_id: row.get("session_id"),
        trigger_message_id: row.get("trigger_message_id"),
        state: row.get("state"),
        provider: row.get("provider"),
        model: row.get("model"),
        identity_version: row.get("identity_version"),
        started_at: row.get("started_at"),
        completed_at: row.get("completed_at"),
        error_code: row.get("error_code"),
    }
}

fn event_from_row(row: sqlx::sqlite::SqliteRow) -> Result<EventEnvelope, StorageError> {
    let version: i64 = row.get("schema_version");
    Ok(EventEnvelope {
        v: u16::try_from(version).unwrap_or(PROTOCOL_VERSION),
        event_id: row.get("event_id"),
        instance_id: row.get("instance_id"),
        session_id: row.get("session_id"),
        run_id: row.get("run_id"),
        seq: row.get("seq"),
        event_type: row.get("event_type"),
        ts: row.get("created_at"),
        payload: serde_json::from_str(row.get::<String, _>("payload_json").as_str())?,
    })
}

fn new_id() -> String {
    Uuid::now_v7().to_string()
}

fn memory_as_of(query: &str) -> Option<String> {
    query
        .split(|character: char| !character.is_ascii_digit())
        .find_map(|segment| {
            (segment.len() == 4)
                .then(|| segment.parse::<u16>().ok())
                .flatten()
                .filter(|year| (1900..=9999).contains(year))
        })
        .map(|year| format!("{year:04}-12-31T23:59:59.999Z"))
}

fn memory_fts_query(query: &str) -> Option<String> {
    const STOPWORDS: [&str; 13] = [
        "about", "could", "does", "have", "please", "remember", "tell", "that", "this", "what",
        "when", "where", "would",
    ];
    let mut segments = Vec::new();
    let mut current = String::new();
    for character in query.chars() {
        if character.is_alphanumeric() || matches!(character, '_' | '-') {
            current.extend(character.to_lowercase());
        } else if !current.is_empty() {
            segments.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        segments.push(current);
    }
    let mut terms = Vec::new();
    for segment in segments {
        let characters = segment.chars().collect::<Vec<_>>();
        if characters.len() < 3 || (segment.is_ascii() && STOPWORDS.contains(&segment.as_str())) {
            continue;
        }
        if characters.len() <= 12 || segment.is_ascii() {
            terms.push(segment);
        } else {
            for window in characters.windows(3) {
                terms.push(window.iter().collect());
            }
        }
    }
    terms.sort();
    terms.dedup();
    terms.truncate(24);
    if terms.is_empty() {
        return None;
    }
    Some(
        terms
            .into_iter()
            .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" OR "),
    )
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migrations_bootstrap_single_scope_and_pragmas() {
        let storage = Storage::in_memory().await.expect("initialize database");
        assert!(!storage.scope().instance_id.is_empty());
        assert!(!storage.scope().session_id.is_empty());
        assert!(storage.foreign_keys_enabled().await.expect("foreign keys"));
        assert_eq!(
            storage.journal_mode().await.expect("journal mode"),
            "memory"
        );
    }

    #[tokio::test]
    async fn user_input_is_atomic_and_idempotent() {
        let storage = Storage::in_memory().await.expect("initialize database");
        let session_id = storage.scope().session_id.clone();
        let first = storage
            .accept_user_input(
                &session_id,
                "client-1",
                "hello",
                "deepseek",
                "synthetic-model",
                "legacy-prompt-v1",
            )
            .await
            .expect("accept first input");
        let retry = storage
            .accept_user_input(
                &session_id,
                "client-1",
                "hello",
                "deepseek",
                "synthetic-model",
                "legacy-prompt-v1",
            )
            .await
            .expect("deduplicate retry");

        assert!(!first.deduplicated);
        assert!(retry.deduplicated);
        assert_eq!(first.message.id, retry.message.id);
        assert_eq!(first.run.id, retry.run.id);
        assert_eq!(first.events.len(), 2);
        assert_eq!(first.events[0].seq, Some(1));
        assert_eq!(first.events[1].seq, Some(2));
        assert_eq!(retry.events, first.events);
    }

    #[tokio::test]
    async fn completion_commits_reply_events_and_job_together() {
        let storage = Storage::in_memory().await.expect("initialize database");
        let session_id = storage.scope().session_id.clone();
        let accepted = storage
            .accept_user_input(
                &session_id,
                "client-2",
                "hello",
                "deepseek",
                "synthetic-model",
                "legacy-prompt-v1",
            )
            .await
            .expect("accept input");
        let started = storage
            .start_run(&accepted.run.id)
            .await
            .expect("start run");
        let completed = storage
            .complete_reply(
                &accepted.run.id,
                "world",
                "stop",
                CompletionUsage {
                    input_tokens: Some(3),
                    output_tokens: Some(1),
                    cached_input_tokens: Some(0),
                },
            )
            .await
            .expect("complete run");

        assert_eq!(started.seq, Some(3));
        assert_eq!(completed[0].seq, Some(4));
        assert_eq!(completed[1].seq, Some(5));
        let snapshot = storage.snapshot(&session_id).await.expect("snapshot");
        assert_eq!(snapshot.last_seq, 5);
        assert_eq!(snapshot.messages.len(), 2);
        assert!(snapshot.active_run.is_none());
        let job_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM durable_jobs")
            .fetch_one(&storage.pool)
            .await
            .expect("count jobs");
        assert_eq!(job_count, 1);
    }

    #[tokio::test]
    async fn durable_job_lease_retries_and_dead_letters() {
        let storage = Storage::in_memory().await.expect("initialize database");
        let session_id = storage.scope().session_id.clone();
        let accepted = storage
            .accept_user_input(
                &session_id,
                "job-lease",
                "remember this",
                "deepseek",
                "synthetic-model",
                "identity-v1",
            )
            .await
            .expect("accept");
        storage.start_run(&accepted.run.id).await.expect("start");
        storage
            .complete_reply(
                &accepted.run.id,
                "acknowledged",
                "stop",
                CompletionUsage::default(),
            )
            .await
            .expect("complete");

        let first = storage
            .claim_durable_job("memory_extract", "worker-a", Duration::from_secs(30))
            .await
            .expect("claim")
            .expect("job");
        assert_eq!(first.attempts, 1);
        assert!(
            storage
                .claim_durable_job("memory_extract", "worker-b", Duration::from_secs(30))
                .await
                .expect("second claim")
                .is_none()
        );
        assert_eq!(
            storage
                .fail_durable_job(&first, "worker-a", "provider_timeout", Duration::ZERO)
                .await
                .expect("fail first"),
            "failed"
        );
        let second = storage
            .claim_durable_job("memory_extract", "worker-b", Duration::from_secs(30))
            .await
            .expect("reclaim")
            .expect("job");
        assert_eq!(second.attempts, 2);
        assert!(
            storage
                .complete_durable_job(&second.id, "worker-b")
                .await
                .expect("complete job")
        );
    }

    #[tokio::test]
    async fn committed_memory_is_idempotent_versioned_and_provenanced() {
        use crate::memory::types::{Explicitness, MemoryScope, MemoryType, Sensitivity};

        let storage = Storage::in_memory().await.expect("initialize database");
        let session_id = storage.scope().session_id.clone();
        let first = storage
            .accept_user_input(
                &session_id,
                "memory-first",
                "I use Vim every day",
                "deepseek",
                "synthetic-model",
                "identity-v1",
            )
            .await
            .expect("accept first");
        let first_source = storage
            .load_extraction_source(&first.run.id)
            .await
            .expect("source");
        let mut draft = ExtractedMemory {
            memory_type: MemoryType::Preference,
            scope: MemoryScope::User,
            subject: "user".to_string(),
            predicate: "editor".to_string(),
            value: "Vim".to_string(),
            evidence_quote: "I use Vim every day".to_string(),
            explicitness: Explicitness::Explicit,
            confidence: 0.99,
            importance: 0.7,
            sensitivity: Sensitivity::Personal,
            valid_from: Some("2025-01-01T00:00:00Z".to_string()),
            valid_to: None,
        };
        let recorded = storage
            .record_memory_candidate(&first_source, &draft)
            .await
            .expect("record first");
        assert_eq!(recorded.status, "committed");
        assert_eq!(recorded.item_version, Some(1));
        assert!(recorded.supersedes_id.is_none());
        let repeated = storage
            .record_memory_candidate(&first_source, &draft)
            .await
            .expect("repeat");
        assert!(!repeated.inserted);
        assert_eq!(repeated.item_id, recorded.item_id);

        let second = storage
            .accept_user_input(
                &session_id,
                "memory-second",
                "Correction: I use Neovim every day",
                "deepseek",
                "synthetic-model",
                "identity-v1",
            )
            .await
            .expect("accept correction");
        let second_source = storage
            .load_extraction_source(&second.run.id)
            .await
            .expect("second source");
        draft.value = "Neovim".to_string();
        draft.evidence_quote = "Correction: I use Neovim every day".to_string();
        draft.valid_from = Some("2026-01-01T00:00:00Z".to_string());
        let corrected = storage
            .record_memory_candidate(&second_source, &draft)
            .await
            .expect("record correction");
        assert_eq!(corrected.item_version, Some(2));
        assert_eq!(corrected.supersedes_id, recorded.item_id);

        let active_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM memory_items WHERE canonical_key = ? AND status = 'active'",
        )
        .bind(crate::memory::policy::canonical_key(&draft))
        .fetch_one(&storage.pool)
        .await
        .expect("active count");
        let source_links: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM memory_links WHERE memory_id = ? AND link_type = 'source'",
        )
        .bind(corrected.item_id.as_deref())
        .fetch_one(&storage.pool)
        .await
        .expect("source links");
        assert_eq!(active_count, 1);
        assert_eq!(source_links, 1);
        let evidence = storage
            .retrieve_memory_evidence(&session_id, "Do I use Neovim?", 12)
            .await
            .expect("retrieve memory");
        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].version, "2");
        assert!(evidence[0].text.contains("Neovim"));
        assert!(!evidence[0].source_event_ids.is_empty());
        let historical = storage
            .retrieve_memory_evidence(&session_id, "Which editor did I use in 2025?", 12)
            .await
            .expect("retrieve historical memory");
        assert_eq!(historical.len(), 1);
        assert_eq!(historical[0].version, "1");
        assert!(historical[0].text.contains("Vim"));
        assert!(historical[0].text.contains("valid_to=2026-01-01"));
        assert!(
            storage
                .retrieve_memory_evidence(&session_id, "favorite beverage matcha", 12)
                .await
                .expect("unrelated retrieval")
                .is_empty()
        );
    }

    #[tokio::test]
    async fn sensitive_memory_stays_pending_and_out_of_canonical_items() {
        use crate::memory::types::{Explicitness, MemoryScope, MemoryType, Sensitivity};

        let storage = Storage::in_memory().await.expect("initialize database");
        let session_id = storage.scope().session_id.clone();
        let accepted = storage
            .accept_user_input(
                &session_id,
                "memory-sensitive",
                "My diagnosis is private",
                "deepseek",
                "synthetic-model",
                "identity-v1",
            )
            .await
            .expect("accept");
        let source = storage
            .load_extraction_source(&accepted.run.id)
            .await
            .expect("source");
        let recorded = storage
            .record_memory_candidate(
                &source,
                &ExtractedMemory {
                    memory_type: MemoryType::Fact,
                    scope: MemoryScope::User,
                    subject: "user".to_string(),
                    predicate: "diagnosis".to_string(),
                    value: "private diagnosis".to_string(),
                    evidence_quote: "My diagnosis is private".to_string(),
                    explicitness: Explicitness::Explicit,
                    confidence: 0.99,
                    importance: 0.9,
                    sensitivity: Sensitivity::Sensitive,
                    valid_from: None,
                    valid_to: None,
                },
            )
            .await
            .expect("record");
        assert_eq!(recorded.status, "pending");
        assert!(recorded.item_id.is_none());
        let item_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memory_items")
            .fetch_one(&storage.pool)
            .await
            .expect("count items");
        assert_eq!(item_count, 0);
        let pending = storage.list_pending_memory().await.expect("pending list");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, recorded.candidate_id);
        let confirmed = storage
            .confirm_memory_candidate(&recorded.candidate_id)
            .await
            .expect("confirm candidate");
        let item_id = confirmed.item_id.expect("committed item");
        let item = storage
            .update_memory_item(&item_id, Some(true), Some(true), None)
            .await
            .expect("update controls");
        assert!(item.pinned);
        assert!(item.never_recall);
        assert!(
            storage
                .retrieve_memory_evidence(&session_id, "private diagnosis", 8)
                .await
                .expect("never recall filter")
                .is_empty()
        );
        let retracted = storage
            .update_memory_item(&item_id, None, None, Some("retracted"))
            .await
            .expect("retract");
        assert_eq!(retracted.status, "retracted");
        assert!(
            storage
                .list_memory_items(false)
                .await
                .expect("active list")
                .is_empty()
        );
    }

    #[tokio::test]
    async fn pending_candidate_can_be_rejected_once() {
        use crate::memory::types::{Explicitness, MemoryScope, MemoryType, Sensitivity};

        let storage = Storage::in_memory().await.expect("initialize database");
        let session_id = storage.scope().session_id.clone();
        let accepted = storage
            .accept_user_input(
                &session_id,
                "memory-reject",
                "I might enjoy jazz",
                "deepseek",
                "synthetic-model",
                "identity-v1",
            )
            .await
            .expect("accept");
        let source = storage
            .load_extraction_source(&accepted.run.id)
            .await
            .expect("source");
        let candidate = storage
            .record_memory_candidate(
                &source,
                &ExtractedMemory {
                    memory_type: MemoryType::Preference,
                    scope: MemoryScope::User,
                    subject: "user".to_string(),
                    predicate: "music".to_string(),
                    value: "jazz".to_string(),
                    evidence_quote: "I might enjoy jazz".to_string(),
                    explicitness: Explicitness::Inferred,
                    confidence: 0.7,
                    importance: 0.4,
                    sensitivity: Sensitivity::Personal,
                    valid_from: None,
                    valid_to: None,
                },
            )
            .await
            .expect("record");
        storage
            .reject_memory_candidate(&candidate.candidate_id)
            .await
            .expect("reject");
        assert!(
            storage
                .list_pending_memory()
                .await
                .expect("pending")
                .is_empty()
        );
        assert!(matches!(
            storage
                .reject_memory_candidate(&candidate.candidate_id)
                .await,
            Err(StorageError::InvalidMemoryCandidateStatus { .. })
        ));
    }

    #[tokio::test]
    async fn memory_retrieval_p95_stays_bounded_on_local_fixture() {
        use std::time::Instant;

        use crate::memory::types::{Explicitness, MemoryScope, MemoryType, Sensitivity};

        let storage = Storage::in_memory().await.expect("initialize database");
        let session_id = storage.scope().session_id.clone();
        for index in 0..250 {
            let marker = format!("toolvalue{index:03}");
            let text = format!("I use {marker}");
            let accepted = storage
                .accept_user_input(
                    &session_id,
                    &format!("memory-latency-{index}"),
                    &text,
                    "deepseek",
                    "synthetic-model",
                    "identity-v1",
                )
                .await
                .expect("accept fixture");
            let source = storage
                .load_extraction_source(&accepted.run.id)
                .await
                .expect("source");
            storage
                .record_memory_candidate(
                    &source,
                    &ExtractedMemory {
                        memory_type: MemoryType::Fact,
                        scope: MemoryScope::User,
                        subject: "user".to_string(),
                        predicate: marker.clone(),
                        value: marker,
                        evidence_quote: text,
                        explicitness: Explicitness::Explicit,
                        confidence: 0.99,
                        importance: 0.5,
                        sensitivity: Sensitivity::Personal,
                        valid_from: None,
                        valid_to: None,
                    },
                )
                .await
                .expect("record fixture");
        }
        let mut latencies = Vec::with_capacity(100);
        for _ in 0..100 {
            let started = Instant::now();
            let evidence = storage
                .retrieve_memory_evidence(&session_id, "toolvalue149", 12)
                .await
                .expect("retrieve");
            latencies.push(started.elapsed());
            assert!(
                evidence
                    .iter()
                    .any(|item| item.text.contains("toolvalue149"))
            );
        }
        latencies.sort_unstable();
        let p95 = latencies[94];
        println!("memory retrieval p95 over 100 queries / 250 items: {p95:?}");
        assert!(
            p95 < Duration::from_millis(50),
            "local FTS retrieval p95 was {p95:?}"
        );
    }

    #[tokio::test]
    async fn illegal_run_transition_does_not_write_an_event() {
        let storage = Storage::in_memory().await.expect("initialize database");
        let session_id = storage.scope().session_id.clone();
        let accepted = storage
            .accept_user_input(
                &session_id,
                "client-3",
                "hello",
                "deepseek",
                "synthetic-model",
                "legacy-prompt-v1",
            )
            .await
            .expect("accept input");
        let error = storage
            .complete_reply(
                &accepted.run.id,
                "world",
                "stop",
                CompletionUsage::default(),
            )
            .await
            .expect_err("queued to completed must fail");
        assert!(matches!(error, StorageError::InvalidRunTransition { .. }));
        assert_eq!(
            storage
                .events_after(&session_id, 0, 10)
                .await
                .expect("events")
                .len(),
            2
        );
    }

    #[tokio::test]
    async fn file_database_uses_wal_and_survives_reopen() {
        let fixture = tempfile::tempdir().expect("temporary directory");
        let first = Storage::initialize(fixture.path())
            .await
            .expect("initialize file database");
        assert_eq!(first.journal_mode().await.expect("journal mode"), "wal");
        let scope = first.scope().clone();
        drop(first);
        let reopened = Storage::initialize(fixture.path())
            .await
            .expect("reopen file database");
        assert_eq!(reopened.scope(), &scope);
        assert!(reopened.foreign_keys_enabled().await.expect("foreign keys"));
    }

    #[tokio::test]
    async fn concurrent_appends_allocate_unique_monotonic_sequences() {
        let fixture = tempfile::tempdir().expect("temporary directory");
        let storage = Storage::initialize(fixture.path())
            .await
            .expect("initialize database");
        let session_id = storage.scope().session_id.clone();
        let first_storage = storage.clone();
        let first_session = session_id.clone();
        let second_storage = storage.clone();
        let second_session = session_id.clone();
        let (first, second) = tokio::join!(
            async move {
                first_storage
                    .accept_user_input(
                        &first_session,
                        "parallel-1",
                        "one",
                        "deepseek",
                        "synthetic-model",
                        "legacy-prompt-v1",
                    )
                    .await
            },
            async move {
                second_storage
                    .accept_user_input(
                        &second_session,
                        "parallel-2",
                        "two",
                        "deepseek",
                        "synthetic-model",
                        "legacy-prompt-v1",
                    )
                    .await
            }
        );
        first.expect("first append");
        second.expect("second append");
        let events = storage
            .events_after(&session_id, 0, 10)
            .await
            .expect("events");
        assert_eq!(
            events
                .iter()
                .filter_map(|event| event.seq)
                .collect::<Vec<_>>(),
            vec![1, 2, 3, 4]
        );
    }

    #[tokio::test]
    async fn legacy_exchange_import_is_restart_safe() {
        let storage = Storage::in_memory().await.expect("initialize database");
        assert!(
            storage
                .import_completed_exchange("2026-08-11.md", 0, "hello", "world")
                .await
                .expect("first import")
        );
        assert!(
            !storage
                .import_completed_exchange("2026-08-11.md", 0, "hello", "world")
                .await
                .expect("repeat import")
        );
        let snapshot = storage
            .snapshot(&storage.scope().session_id)
            .await
            .expect("snapshot");
        assert_eq!(snapshot.messages.len(), 2);
        assert_eq!(snapshot.last_seq, 5);
    }

    #[tokio::test]
    async fn restart_fails_incomplete_runs_instead_of_leaving_them_stuck() {
        let fixture = tempfile::tempdir().expect("temporary directory");
        let storage = Storage::initialize(fixture.path())
            .await
            .expect("initialize database");
        let session_id = storage.scope().session_id.clone();
        storage
            .accept_user_input(
                &session_id,
                "interrupted-1",
                "hello",
                "deepseek",
                "synthetic-model",
                "legacy-prompt-v1",
            )
            .await
            .expect("accept input");
        drop(storage);

        let reopened = Storage::initialize(fixture.path())
            .await
            .expect("reopen database");
        let snapshot = reopened.snapshot(&session_id).await.expect("snapshot");
        assert!(snapshot.active_run.is_none());
        let events = reopened
            .events_after(&session_id, 0, 10)
            .await
            .expect("events");
        assert_eq!(
            events.last().map(|event| event.event_type.as_str()),
            Some("run.failed")
        );
        assert_eq!(events.last().and_then(|event| event.seq), Some(3));
    }

    #[tokio::test]
    async fn identity_activation_and_context_plan_are_versioned_and_replayable() {
        use crate::context::{
            Affect, CompileInput, ContextBudget, ContextCompiler, RelationshipState, SelfState,
        };

        let storage = Storage::in_memory().await.expect("initialize database");
        let identity = std::sync::Arc::new(
            IdentityKernel::load(
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("identity/v1"),
            )
            .await
            .expect("identity"),
        );
        assert!(
            storage
                .activate_identity(&identity)
                .await
                .expect("activate")
                .is_some()
        );
        assert!(
            storage
                .activate_identity(&identity)
                .await
                .expect("activate idempotently")
                .is_none()
        );
        let session_id = storage.scope().session_id.clone();
        let accepted = storage
            .accept_user_input(
                &session_id,
                "context-plan-1",
                "compile this",
                "deepseek",
                "synthetic-model",
                identity.version(),
            )
            .await
            .expect("accept input");
        storage
            .start_run(&accepted.run.id)
            .await
            .expect("start run");
        let now = Utc::now();
        let compiled = ContextCompiler::new(identity)
            .compile(CompileInput {
                run_id: accepted.run.id.clone(),
                mode: "task".to_string(),
                current_message: accepted.message,
                recent_messages: storage
                    .recent_messages(&session_id, 40)
                    .await
                    .expect("recent"),
                self_state: Some(SelfState {
                    affect: Affect::Calm,
                    activation: 0.2,
                    closeness_expression: 0.1,
                    uncertainty: 0.0,
                    active_intents: Vec::new(),
                    expires_at: now + chrono::Duration::minutes(5),
                }),
                relationship_state: RelationshipState::default(),
                memory_evidence: Vec::new(),
                now,
                budget: ContextBudget::for_model(1_000_000, 4_096),
            })
            .expect("compile");
        storage
            .save_context_plan(&compiled.plan)
            .await
            .expect("save plan");
        assert_eq!(
            storage
                .load_context_plan(&accepted.run.id)
                .await
                .expect("load plan"),
            Some(compiled.plan)
        );
    }

    #[tokio::test]
    async fn typed_state_snapshots_round_trip() {
        use crate::context::{Affect, RelationshipState, SelfState};

        let storage = Storage::in_memory().await.expect("initialize database");
        let self_state = SelfState {
            affect: Affect::Warm,
            activation: 0.4,
            closeness_expression: 0.3,
            uncertainty: 0.1,
            active_intents: vec!["intent-1".to_string()],
            expires_at: Utc::now() + chrono::Duration::minutes(5),
        };
        let relationship = RelationshipState {
            familiarity: 0.2,
            ..RelationshipState::default()
        };
        storage
            .save_state("self", &self_state)
            .await
            .expect("save self state");
        storage
            .save_state("relationship", &relationship)
            .await
            .expect("save relationship state");
        assert_eq!(
            storage.latest_self_state().await.expect("load self"),
            Some(self_state)
        );
        assert_eq!(
            storage
                .latest_relationship_state()
                .await
                .expect("load relationship"),
            Some(relationship)
        );
    }
}
