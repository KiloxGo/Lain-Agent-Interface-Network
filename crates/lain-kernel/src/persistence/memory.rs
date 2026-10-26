use super::Store;
use crate::{
    actor::{
        domain::{ActionState, ActorTurn},
        tools::Tool,
    },
    backend::ControlError,
    domain::{WorkSessionId, now_ms},
    memory::*,
};
use anyhow::{Context, Result, ensure};
use lain_wire::{ActionId, RequestId};
use rusqlite::{OptionalExtension, Transaction, params};

pub(super) fn key<T: serde::Serialize>(value: &T) -> Result<String> {
    Ok(serde_json::to_string(value)?)
}
fn decode<T: serde::de::DeserializeOwned>(value: String) -> Result<T> {
    serde_json::from_str(&value).map_err(|_| anyhow::anyhow!("invalid memory storage record"))
}
fn read_memory_snapshot(db: &rusqlite::Connection, id: MemoryId) -> Result<MemoryRecord> {
    let record: MemoryRecord = decode(
        db.query_row(
            "SELECT snapshot FROM lain_memories WHERE id=?1",
            [id.to_string()],
            |r| r.get(0),
        )
        .optional()?
        .ok_or(ControlError::NotFound)?,
    )?;
    record.validate().map_err(anyhow::Error::msg)?;
    ensure!(record.id == id, "memory identity mismatch");
    Ok(record)
}
pub(super) fn read_memory(db: &rusqlite::Connection, id: MemoryId) -> Result<MemoryRecord> {
    let record = read_memory_snapshot(db, id)?;
    crate::memory::validate(&record.draft, &record.provenance)?;
    Ok(record)
}
fn read_candidate(db: &rusqlite::Connection, id: CandidateId) -> Result<MemoryCandidate> {
    let candidate: MemoryCandidate = decode(
        db.query_row(
            "SELECT snapshot FROM lain_memory_candidates WHERE id=?1",
            [id.to_string()],
            |r| r.get(0),
        )
        .optional()?
        .ok_or(ControlError::NotFound)?,
    )?;
    candidate.validate().map_err(anyhow::Error::msg)?;
    ensure!(candidate.id == id, "memory candidate identity mismatch");
    Ok(candidate)
}
pub(super) fn insert_memory(
    tx: &Transaction<'_>,
    draft: MemoryDraft,
    provenance: MemoryProvenance,
) -> Result<MemoryRecord> {
    crate::memory::validate(&draft, &provenance)?;
    let time = now_ms()?;
    let owner_confirmed_at_ms =
        if matches!(provenance.source, MemorySource::ChannelSelection { .. }) {
            None
        } else {
            Some(time)
        };
    let record = MemoryRecord {
        id: MemoryId::new(),
        draft,
        provenance,
        status: MemoryStatus::Active,
        superseded_by: None,
        recorded_at_ms: time,
        owner_confirmed_at_ms,
        updated_at_ms: time,
        revision: 1,
    };
    record.validate().map_err(anyhow::Error::msg)?;
    ensure!(key(&record)?.len() <= 12000, ControlError::InvalidInput);
    tx.execute("INSERT INTO lain_memories(id,kind,scope,status,content,snapshot) VALUES(?1,?2,?3,?4,?5,?6)",
        params![record.id.to_string(),key(&record.draft.kind)?,key(&record.draft.scope)?,key(&record.status)?,record.draft.content,key(&record)?])?;
    Ok(record)
}
fn write_candidate(tx: &Transaction<'_>, candidate: &MemoryCandidate) -> Result<()> {
    tx.execute(
        "UPDATE lain_memory_candidates SET snapshot=?2 WHERE id=?1",
        (candidate.id.to_string(), key(candidate)?),
    )?;
    Ok(())
}
fn review(tx: &Transaction<'_>, id: CandidateId, accept: bool) -> Result<MemoryCandidate> {
    let mut candidate = read_candidate(tx, id)?;
    let desired = if accept {
        CandidateStatus::Accepted
    } else {
        CandidateStatus::Rejected
    };
    if candidate.status != CandidateStatus::Candidate {
        ensure!(candidate.status == desired, ControlError::Conflict);
        return Ok(candidate);
    }
    if accept {
        candidate.memory_id =
            Some(insert_memory(tx, candidate.draft.clone(), candidate.provenance.clone())?.id);
    }
    candidate.status = desired;
    candidate.reviewed_at_ms = Some(now_ms()?);
    write_candidate(tx, &candidate)?;
    Ok(candidate)
}
impl Store {
    pub fn memory(&self, id: MemoryId) -> Result<MemoryRecord> {
        read_memory(&self.connection, id)
    }
    pub fn add_memory(&mut self, input: &AddMemory) -> Result<MemoryRecord> {
        let provenance = MemoryProvenance {
            source: MemorySource::ExplicitOwnerStatement {
                request_id: input.request_id,
            },
            authority: MemoryAuthority::Explicit,
        };
        crate::memory::validate(&input.draft, &provenance)?;
        let digest = lain_wire::actor_configuration_revision(key(&input.draft)?.as_bytes());
        let tx = self.connection.transaction()?;
        let duplicate: Option<(String, String)> = tx
            .query_row(
                "SELECT payload_hash,memory_id FROM lain_memory_requests WHERE request_id=?1",
                [input.request_id.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((hash, id)) = duplicate {
            ensure!(hash == digest, ControlError::Conflict);
            // Tombstones prevent a retried old create from resurrecting deleted content.
            return read_memory(&tx, id.parse()?);
        }
        let record = insert_memory(&tx, input.draft.clone(), provenance)?;
        tx.execute(
            "INSERT INTO lain_memory_requests VALUES(?1,?2,?3)",
            params![input.request_id.to_string(), digest, record.id.to_string()],
        )?;
        tx.commit()?;
        Ok(record)
    }
    pub fn memory_page(&self, q: &MemoryQuery) -> Result<MemoryPage> {
        ensure!(
            (1..=50).contains(&q.limit) && q.after <= i64::MAX as u64,
            ControlError::InvalidInput
        );
        if let Some(text) = &q.text {
            ensure!(
                !text.trim().is_empty() && text.len() <= 256 && !text.chars().any(char::is_control),
                ControlError::InvalidInput
            );
        }
        let scope = key(&q.scope)?;
        let kind = q.kind.map(|v| key(&v)).transpose()?;
        let status = key(&q.status.unwrap_or(MemoryStatus::Active))?;
        // Literal quoted MATCH prevents query syntax injection. Short Unicode strings
        // use a bounded-page literal scan because trigram indexes require 3 scalars.
        let fts = q
            .text
            .as_ref()
            .filter(|t| t.chars().count() >= 3)
            .map(|t| format!("\"{}\"", t.replace('"', "\"\"")));
        let mut query = self.connection.prepare("SELECT sequence,snapshot FROM lain_memories WHERE scope=?1 AND status=?2 AND (?3 IS NULL OR kind=?3) AND sequence>?4 AND (?5 IS NULL OR instr(lower(content),lower(?5))>0) AND (?6 IS NULL OR sequence IN (SELECT rowid FROM lain_memory_fts WHERE lain_memory_fts MATCH ?6)) ORDER BY sequence LIMIT ?7")?;
        let rows = query.query_map(
            params![
                scope,
                status,
                kind,
                q.after as i64,
                q.text,
                fts,
                i64::from(q.limit) + 1
            ],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
        )?;
        let mut records = Vec::new();
        let mut cursor = q.after;
        let mut more = false;
        let mut bytes = 0;
        for row in rows {
            let (sequence, value) = row?;
            if records.len() == usize::from(q.limit)
                || (!records.is_empty() && bytes + value.len() > 14000)
            {
                more = true;
                break;
            }
            bytes += value.len();
            let record: MemoryRecord = decode(value)?;
            record.validate().map_err(anyhow::Error::msg)?;
            crate::memory::validate(&record.draft, &record.provenance)?;
            ensure!(
                record.draft.scope == q.scope
                    && record.status == q.status.unwrap_or(MemoryStatus::Active)
                    && q.kind.is_none_or(|kind| record.draft.kind == kind),
                "memory index mismatch"
            );
            records.push(record);
            cursor = sequence.try_into()?;
        }
        Ok(MemoryPage {
            records,
            next_cursor: cursor,
            has_more: more,
        })
    }
    pub fn change_memory(&mut self, id: MemoryId, change: &ChangeMemory) -> Result<MemoryRecord> {
        let tx = self.connection.transaction()?;
        let mut record = read_memory(&tx, id)?;
        ensure!(
            record.revision == change.expected_revision,
            ControlError::Conflict
        );
        ensure!(
            (change.status == MemoryStatus::Superseded) == change.superseded_by.is_some(),
            ControlError::InvalidInput
        );
        if let Some(target) = change.superseded_by {
            let target = read_memory(&tx, target)?;
            ensure!(
                target.id != id
                    && target.draft.scope == record.draft.scope
                    && target.status == MemoryStatus::Active,
                ControlError::InvalidInput
            );
        }
        record.status = change.status;
        if change.status != MemoryStatus::Active {
            super::private_memory::suppress(&tx, &record)?;
        }
        record.superseded_by = change.superseded_by;
        record.revision = record
            .revision
            .checked_add(1)
            .context("memory revision exhausted")?;
        record.updated_at_ms = now_ms()?;
        tx.execute(
            "UPDATE lain_memories SET status=?2,snapshot=?3 WHERE id=?1",
            params![id.to_string(), key(&record.status)?, key(&record)?],
        )?;
        tx.commit()?;
        Ok(record)
    }
    pub fn delete_memory(&mut self, id: MemoryId, revision: u64) -> Result<()> {
        let tx = self.connection.transaction()?;
        // Deletion remains possible even if a rotated credential now matches text
        // that was not recognizable as sensitive when originally stored.
        let record = read_memory_snapshot(&tx, id)?;
        ensure!(record.revision == revision, ControlError::Conflict);
        super::private_memory::suppress(&tx, &record)?;
        tx.execute("DELETE FROM lain_memories WHERE id=?1", [id.to_string()])?;
        // Remove the semantic candidate copy too. Original Actor/operational audit
        // remains under its own retention; deletion is not secure disk erasure.
        let mut stmt = tx.prepare("SELECT snapshot FROM lain_memory_candidates")?;
        let candidates = stmt
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(stmt);
        for value in candidates {
            let mut c: MemoryCandidate = decode(value)?;
            if c.memory_id == Some(id) {
                c.draft.content = "[deleted]".into();
                write_candidate(&tx, &c)?;
            }
        }
        tx.commit()?;
        Ok(())
    }
    pub fn candidate(&self, id: CandidateId) -> Result<MemoryCandidate> {
        read_candidate(&self.connection, id)
    }
    pub fn candidates(&self, after: u64, limit: u16) -> Result<serde_json::Value> {
        ensure!(
            (1..=50).contains(&limit) && after <= i64::MAX as u64,
            ControlError::InvalidInput
        );
        let mut stmt=self.connection.prepare("SELECT sequence,snapshot FROM lain_memory_candidates WHERE sequence>?1 ORDER BY sequence LIMIT ?2")?;
        let mut records = Vec::<MemoryCandidate>::new();
        let mut cursor = after;
        let mut more = false;
        for row in stmt.query_map(params![after as i64, i64::from(limit) + 1], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })? {
            let (n, v) = row?;
            if records.len() == limit as usize {
                more = true;
                break;
            }
            let candidate: MemoryCandidate = decode(v)?;
            candidate.validate().map_err(anyhow::Error::msg)?;
            records.push(candidate);
            cursor = n.try_into()?;
        }
        Ok(serde_json::json!({"candidates":records,"next_cursor":cursor,"has_more":more}))
    }
    pub fn summary_candidate(
        &mut self,
        request: RequestId,
        summary: SummaryId,
        draft: MemoryDraft,
    ) -> Result<MemoryCandidate> {
        let summary = self.summary(summary)?;
        ensure!(
            draft.kind == MemoryKind::ProjectKnowledge
                && draft.scope
                    == MemoryScope::Workspace {
                        path: summary
                            .workspace
                            .to_str()
                            .ok_or(ControlError::InvalidInput)?
                            .into()
                    },
            ControlError::InvalidInput
        );
        let provenance = MemoryProvenance {
            source: MemorySource::SessionSummary {
                summary_id: summary.id,
            },
            authority: MemoryAuthority::UnverifiedSelection,
        };
        crate::memory::validate(&draft, &provenance)?;
        let id = CandidateId(request.0);
        match self.candidate(id) {
            Ok(existing) => {
                ensure!(
                    existing.draft == draft && existing.provenance == provenance,
                    ControlError::Conflict
                );
                return Ok(existing);
            }
            Err(error)
                if matches!(
                    error.downcast_ref::<ControlError>(),
                    Some(ControlError::NotFound)
                ) => {}
            Err(error) => return Err(error),
        }

        let candidate = MemoryCandidate {
            id,
            draft,
            provenance,
            status: CandidateStatus::Candidate,
            memory_id: None,
            created_at_ms: now_ms()?,
            reviewed_at_ms: None,
            actor_action: None,
        };
        self.connection.execute(
            "INSERT INTO lain_memory_candidates(id,snapshot) VALUES(?1,?2)",
            (id.to_string(), key(&candidate)?),
        )?;
        Ok(candidate)
    }
    pub fn review_candidate(
        &mut self,
        id: CandidateId,
        accept: bool,
        actor: Option<ActionId>,
    ) -> Result<MemoryCandidate> {
        let tx = self.connection.transaction()?;
        ensure!(
            read_candidate(&tx, id)?.actor_action == actor,
            ControlError::Conflict
        );
        let result = review(&tx, id, accept)?;
        tx.commit()?;
        Ok(result)
    }
    pub(super) fn sync_memory_candidates(tx: &Transaction<'_>, turn: &ActorTurn) -> Result<()> {
        for action in &turn.actions {
            let Tool::MemoryPropose(args) = &action.tool else {
                continue;
            };
            let id = CandidateId(action.id.0);
            if action.state == ActionState::Proposed {
                let mut draft = args.draft.clone();
                draft.scope = crate::memory::canonical_scope(draft.scope)?;
                let provenance = MemoryProvenance {
                    source: MemorySource::LainGenerated {
                        conversation_id: turn.conversation_id,
                        actor_turn_id: turn.id,
                        action_id: action.id,
                    },
                    authority: MemoryAuthority::Inferred,
                };
                crate::memory::validate(&draft, &provenance)?;
                let c = MemoryCandidate {
                    id,
                    draft,
                    provenance,
                    status: CandidateStatus::Candidate,
                    memory_id: None,
                    created_at_ms: now_ms()?,
                    reviewed_at_ms: None,
                    actor_action: Some(action.id),
                };
                tx.execute("INSERT INTO lain_memory_candidates(id,actor_action,snapshot) VALUES(?1,?2,?3) ON CONFLICT(id) DO NOTHING",params![id.to_string(),action.id.to_string(),key(&c)?])?;
            } else if action.state == ActionState::Rejected {
                review(tx, id, false)?;
            }
        }
        Ok(())
    }
    pub fn summary(&self, id: SummaryId) -> Result<SessionSummary> {
        decode(
            self.connection
                .query_row(
                    "SELECT snapshot FROM lain_session_summaries WHERE id=?1",
                    [id.to_string()],
                    |r| r.get(0),
                )
                .optional()?
                .ok_or(ControlError::NotFound)?,
        )
    }
    pub fn summaries(&self, session: WorkSessionId) -> Result<Vec<SessionSummary>> {
        let mut q=self.connection.prepare("SELECT snapshot FROM lain_session_summaries WHERE session_id=?1 ORDER BY sequence DESC LIMIT 50")?;
        q.query_map([session.to_string()], |r| r.get::<_, String>(0))?
            .map(|r| decode(r?))
            .collect()
    }
    pub fn create_summary(
        &mut self,
        session: &crate::domain::WorkSession,
        turn: &crate::domain::Turn,
    ) -> Result<SessionSummary> {
        ensure!(
            turn.work_session_id == session.id
                && (turn.state.is_terminal() || turn.state == crate::domain::TurnState::Unknown),
            ControlError::Conflict
        );
        let tx = self.connection.transaction()?;
        let cursor:i64=tx.query_row("SELECT COALESCE(MAX(sequence),0) FROM operational_history WHERE session_id=?1 AND turn_id=?2",(session.id.to_string(),turn.id.to_string()),|r|r.get(0))?;
        ensure!(cursor > 0, ControlError::Conflict);
        let existing:Option<String>=tx.query_row("SELECT snapshot FROM lain_session_summaries WHERE session_id=?1 AND turn_id=?2 AND source_cursor=?3",params![session.id.to_string(),turn.id.to_string(),cursor],|r|r.get(0)).optional()?;
        if let Some(value) = existing {
            return decode(value);
        }
        let evidence = Self::result_evidence(&tx, session.id, turn.id, cursor.try_into()?)?;
        let summary = SessionSummary {
            id: SummaryId::new(),
            work_session_id: session.id,
            worker_turn_id: turn.id,
            workspace: session.workspace.clone(),
            outcome: turn.state,
            source_history_cursor: cursor.try_into()?,
            source_turn_updated_at_ms: turn.updated_at_ms,
            created_at_ms: now_ms()?,
            evidence,
            uncertainty: "Worker reports are unverified claims. Turn completion does not establish command/test success. Evidence is bounded; unattributed events are excluded.".into(),
        };

        tx.execute("INSERT INTO lain_session_summaries(id,session_id,turn_id,source_cursor,snapshot) VALUES(?1,?2,?3,?4,?5)",params![summary.id.to_string(),session.id.to_string(),turn.id.to_string(),cursor,key(&summary)?])?;
        tx.commit()?;
        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn memory_and_candidate_review_are_atomic_and_legacy_schema_migrates() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("legacy.sqlite3");
        let db = rusqlite::Connection::open(&path).unwrap();
        for migration in [
            include_str!("../../migrations/001_sessions.sql"),
            include_str!("../../migrations/002_worker_state.sql"),
            include_str!("../../migrations/003_history.sql"),
            include_str!("../../migrations/004_profiles.sql"),
            include_str!("../../migrations/005_actor.sql"),
            include_str!("../../migrations/006_delegations.sql"),
        ] {
            db.execute_batch(migration).unwrap();
        }
        let conversation = crate::actor::domain::ConversationId::new();
        db.execute(
            "INSERT INTO lain_conversations VALUES(?1,1,1)",
            [conversation.to_string()],
        )
        .unwrap();
        drop(db);
        let mut store = Store::open(&path).unwrap();
        assert_eq!(store.conversations().unwrap()[0].id, conversation);
        assert_eq!(
            store
                .connection
                .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            15
        );
        let input = AddMemory {
            request_id: RequestId::new(),
            draft: MemoryDraft {
                content: "atomic creation".into(),
                kind: MemoryKind::Preference,
                scope: MemoryScope::Global,
            },
        };
        store.connection.execute_batch("CREATE TRIGGER reject_memory_request BEFORE INSERT ON lain_memory_requests BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
        assert!(store.add_memory(&input).is_err());
        assert_eq!(
            store
                .connection
                .query_row("SELECT count(*) FROM lain_memories", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            store
                .connection
                .query_row(
                    "SELECT count(*) FROM lain_memory_fts WHERE lain_memory_fts MATCH 'atomic'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        store
            .connection
            .execute_batch("DROP TRIGGER reject_memory_request;")
            .unwrap();
        let record = store.add_memory(&input).unwrap();
        let candidate = MemoryCandidate {
            id: CandidateId::new(),
            draft: record.draft.clone(),
            provenance: record.provenance.clone(),
            status: CandidateStatus::Candidate,
            memory_id: None,
            created_at_ms: now_ms().unwrap(),
            reviewed_at_ms: None,
            actor_action: None,
        };
        store
            .connection
            .execute(
                "INSERT INTO lain_memory_candidates(id,snapshot) VALUES(?1,?2)",
                (candidate.id.to_string(), key(&candidate).unwrap()),
            )
            .unwrap();
        store.connection.execute_batch("CREATE TRIGGER reject_review BEFORE UPDATE ON lain_memory_candidates BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
        assert!(store.review_candidate(candidate.id, true, None).is_err());
        assert_eq!(store.candidate(candidate.id).unwrap(), candidate);
        assert_eq!(
            store
                .connection
                .query_row("SELECT count(*) FROM lain_memories", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    #[test]
    fn deletion_is_available_when_existing_content_is_now_sensitive() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("delete.sqlite3")).unwrap();
        let mut record = store
            .add_memory(&AddMemory {
                request_id: RequestId::new(),
                draft: MemoryDraft {
                    content: "previously innocuous selected text".into(),
                    kind: MemoryKind::Preference,
                    scope: MemoryScope::Global,
                },
            })
            .unwrap();
        // Simulate a previously accepted snapshot now matching the sanitizer.
        // Owner deletion must not require first returning that content publicly.
        record.draft.content = "Bearer fixture-sensitive-content".into();
        store
            .connection
            .execute(
                "UPDATE lain_memories SET snapshot=?1 WHERE id=?2",
                (key(&record).unwrap(), record.id.to_string()),
            )
            .unwrap();
        assert!(store.memory(record.id).is_err());
        store.delete_memory(record.id, record.revision).unwrap();
        assert!(matches!(
            store
                .memory(record.id)
                .unwrap_err()
                .downcast_ref::<ControlError>(),
            Some(ControlError::NotFound)
        ));
    }

    #[test]
    fn contradictory_authority_and_scope_index_corruption_fail_closed() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("state.sqlite3")).unwrap();
        let mut record = store
            .add_memory(&AddMemory {
                request_id: Default::default(),
                draft: MemoryDraft {
                    content: "owner preference".into(),
                    kind: MemoryKind::Preference,
                    scope: MemoryScope::Global,
                },
            })
            .unwrap();
        record.provenance.authority = MemoryAuthority::Inferred;
        assert!(record.validate().is_err());
        store
            .connection
            .execute(
                "UPDATE lain_memories SET snapshot=?1",
                [key(&record).unwrap()],
            )
            .unwrap();
        assert!(store.memory(record.id).is_err());
        record.provenance.authority = MemoryAuthority::Explicit;
        record.draft.scope = MemoryScope::Internal;
        store
            .connection
            .execute(
                "UPDATE lain_memories SET snapshot=?1",
                [key(&record).unwrap()],
            )
            .unwrap();
        assert!(
            store
                .memory_page(&MemoryQuery {
                    scope: MemoryScope::Global,
                    kind: None,
                    status: None,
                    text: None,
                    after: 0,
                    limit: 10
                })
                .is_err()
        );
    }
}
