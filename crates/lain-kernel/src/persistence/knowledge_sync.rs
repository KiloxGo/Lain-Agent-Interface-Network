//! Per-source finite GET grants and durable attempt reservations, never model work.
use super::{Store, archive_reads, archives, channels::decode, knowledge};
use crate::backend::ControlError;
use anyhow::{Result, ensure};
use lain_wire::{ArchiveSourceId, RequestId, archives::*};
use rusqlite::{Connection, OptionalExtension};

fn grant(db: &Connection, id: ArchiveSourceId) -> Result<KnowledgeSyncGrant> {
    db.query_row(
        "SELECT snapshot FROM knowledge_sync WHERE source_id=?1 ORDER BY revision DESC LIMIT 1",
        [id.to_string()],
        |r| r.get::<_, String>(0),
    )
    .optional()?
    .map(decode)
    .transpose()
    .map(|v| {
        v.unwrap_or(KnowledgeSyncGrant {
            source_id: id,
            revision: 0,
            scope: None,
            reader_revision: None,
            used_requests: 0,
            next_at_ms: 0,
            last_read: None,
            pending: false,
            stopped: None,
            changed_at_ms: 0,
        })
    })
}

fn save(db: &Connection, g: &KnowledgeSyncGrant) -> Result<()> {
    db.execute("INSERT INTO knowledge_sync VALUES (?1,?2,?3) ON CONFLICT(source_id,revision) DO UPDATE SET snapshot=excluded.snapshot",(g.source_id.to_string(),i64::try_from(g.revision)?,serde_json::to_string(g)?))?;
    Ok(())
}
pub(super) fn read_allowed(db: &Connection, j: &ArchiveReadJob, now: u64) -> Result<bool> {
    let Some(revision) = j.sync_revision else {
        return Ok(true);
    };
    let g = grant(db, j.source_id)?;
    Ok(g.revision == revision
        && g.stopped.is_none()
        && g.pending
        && g.last_read == Some(j.request.request_id)
        && g.reader_revision == j.reader_revision
        && g.scope.is_some_and(|s| {
            s.expires_at_ms > now && s.archive_revision == j.request.expected_revision
        }))
}
impl Store {
    pub fn knowledge_sync_grants(&self, id: ArchiveSourceId) -> Result<Vec<KnowledgeSyncGrant>> {
        self.archive_source(id)?;
        let mut q=self.connection.prepare("SELECT snapshot FROM knowledge_sync WHERE source_id=?1 ORDER BY revision DESC LIMIT 100")?;
        q.query_map([id.to_string()], |r| r.get::<_, String>(0))?
            .map(|r| decode(r?))
            .collect()
    }
    pub fn knowledge_sync_grant(&self, id: ArchiveSourceId) -> Result<KnowledgeSyncGrant> {
        self.archive_source(id)?;
        grant(&self.connection, id)
    }
    pub fn knowledge_sync_set(
        &mut self,
        id: ArchiveSourceId,
        q: SetKnowledgeSync,
        reader: Option<String>,
        now: u64,
    ) -> Result<KnowledgeSyncGrant> {
        let tx = self.connection.transaction()?;
        crate::knowledge::source_key(&archives::source(&tx, id)?.key)?;
        let old = grant(&tx, id)?;
        ensure!(old.revision == q.expected_revision, ControlError::Conflict);
        if let Some(s) = &q.scope {
            let a = archives::grant(&tx, id)?;
            let active = archives::active(&a, now)?;
            ensure!(s.archive_revision == a.revision, ControlError::Conflict);
            ensure!(
                reader.as_ref().is_some_and(|r| r.len() == 64)
                    && s.expires_at_ms > now
                    && s.expires_at_ms <= active.expires_at_ms
                    && s.expires_at_ms <= now.saturating_add(31 * 86400000)
                    && (1..=10000).contains(&s.max_requests)
                    && (5000..=86400000).contains(&s.interval_ms)
                    && (1..=32).contains(&s.limit),
                ControlError::InvalidInput
            );
            if let Some(state) =
                knowledge::state(&tx, id)?.filter(|s| s.local_grant_revision == a.revision)
            {
                ensure!(
                    Some(&state.reader_revision) == reader.as_ref() && state.remote_source_enabled,
                    ControlError::Conflict
                );
            }
        }
        if q.scope.is_some() {
            ensure!(
                tx.query_row("SELECT count(*) FROM knowledge_sync", [], |r| r
                    .get::<_, i64>(0))?
                    < 10000,
                ControlError::Conflict
            );
        }
        let g = KnowledgeSyncGrant {
            source_id: id,
            revision: old.revision + 1,
            scope: q.scope,
            reader_revision: reader,
            used_requests: 0,
            next_at_ms: now,
            last_read: None,
            pending: false,
            stopped: None,
            changed_at_ms: now,
        };
        save(&tx, &g)?;
        // A replacement cannot extend the old cache's authority or finish old reads.
        tx.execute("UPDATE knowledge_sources SET snapshot=json_set(snapshot,'$.fresh_until_ms',0) WHERE source_id=?1",[id.to_string()])?;
        tx.commit()?;
        Ok(g)
    }
    /// Reconcile terminal receipts and reserve at most one due GET in the same transaction.
    pub fn knowledge_sync_plan(
        &mut self,
        reader: Option<String>,
        now: u64,
    ) -> Result<(Option<ArchiveReadJob>, Option<u64>)> {
        let tx = self.connection.transaction()?;
        let rows = {
            let mut q=tx.prepare("SELECT snapshot FROM knowledge_sync k WHERE revision=(SELECT max(revision) FROM knowledge_sync WHERE source_id=k.source_id) ORDER BY json_extract(snapshot,'$.next_at_ms'),source_id")?;
            q.query_map([], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        let busy:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM archive_reads WHERE json_extract(snapshot,'$.state') IN ('prepared','reading'))",[],|r|r.get(0))?;
        let mut due = None;
        let mut deadline = None;
        for encoded in rows {
            let mut g: KnowledgeSyncGrant = decode(encoded)?;
            let Some(s) = g.scope.clone() else {
                continue;
            };
            if g.stopped.is_some() {
                if g.pending
                    && !archive_reads::job(&tx, g.last_read.ok_or(ControlError::Conflict)?)?
                        .state
                        .unfinished()
                {
                    g.pending = false;
                    save(&tx, &g)?;
                }
                continue;
            }
            let a = archives::grant(&tx, g.source_id)?;
            let reason = if s.expires_at_ms <= now {
                Some(KnowledgeSyncStop::Expired)
            } else if a.revision != s.archive_revision || archives::active(&a, now).is_err() {
                Some(KnowledgeSyncStop::ScopeChanged)
            } else if g.reader_revision != reader || reader.is_none() {
                Some(KnowledgeSyncStop::ReaderChanged)
            } else {
                None
            };
            if let Some(reason) = reason {
                g.stopped = Some(reason);
                save(&tx, &g)?;
                continue;
            }
            if g.pending {
                let j = archive_reads::job(&tx, g.last_read.ok_or(ControlError::Conflict)?)?;
                if j.state.unfinished() {
                    deadline =
                        Some(deadline.map_or(s.expires_at_ms, |d: u64| d.min(s.expires_at_ms)));
                    continue;
                }
                g.pending = false;
                if j.state != ArchiveReadState::Completed {
                    g.stopped = Some(KnowledgeSyncStop::ReadFailed);
                } else if knowledge::state(&tx, g.source_id)?
                    .is_none_or(|v| !v.remote_source_enabled)
                {
                    g.stopped = Some(KnowledgeSyncStop::SourceUnavailable);
                } else {
                    g.next_at_ms = j
                        .finished_at_ms
                        .unwrap_or(now)
                        .saturating_add(s.interval_ms);
                }
                save(&tx, &g)?;
                if g.stopped.is_some() {
                    continue;
                }
            }
            if g.used_requests >= s.max_requests {
                g.stopped = Some(KnowledgeSyncStop::Exhausted);
                save(&tx, &g)?;
                continue;
            }
            if g.next_at_ms > now {
                let next = g.next_at_ms.min(s.expires_at_ms);
                deadline = Some(deadline.map_or(next, |d: u64| d.min(next)));
                continue;
            }
            if busy || due.is_some() {
                deadline = Some(deadline.map_or(s.expires_at_ms, |d: u64| d.min(s.expires_at_ms)));
                continue;
            }
            if tx.query_row("SELECT count(*) FROM archive_reads", [], |r| {
                r.get::<_, i64>(0)
            })? >= 10000
            {
                g.stopped = Some(KnowledgeSyncStop::Exhausted);
                save(&tx, &g)?;
                continue;
            }
            let previous = knowledge::state(&tx, g.source_id)?
                .filter(|v| v.local_grant_revision == s.archive_revision);
            if previous.as_ref().is_some_and(|v| {
                Some(&v.reader_revision) != reader.as_ref() || !v.remote_source_enabled
            }) {
                g.stopped = Some(KnowledgeSyncStop::SourceUnavailable);
                save(&tx, &g)?;
                continue;
            }
            let q = ReadArchive {
                request_id: RequestId::new(),
                expected_revision: s.archive_revision,
                after: previous.map(|v| v.last_read),
                limit: s.limit,
            };
            let (mut j, fresh) = knowledge::admit(
                &tx,
                g.source_id,
                q,
                reader.clone().ok_or(ControlError::Unavailable)?,
            )?;
            ensure!(fresh, ControlError::Conflict);
            j.sync_revision = Some(g.revision);
            archive_reads::save(&tx, &j)?;
            g.used_requests += 1;
            g.last_read = Some(j.request.request_id);
            g.pending = true;
            save(&tx, &g)?;
            deadline = Some(deadline.map_or(s.expires_at_ms, |d: u64| d.min(s.expires_at_ms)));
            due = Some(j);
        }
        tx.commit()?;
        Ok((due, deadline))
    }
    pub fn knowledge_read_allowed(&self, j: &ArchiveReadJob, now: u64) -> Result<bool> {
        read_allowed(&self.connection, j, now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::now_ms;
    fn setup() -> (tempfile::TempDir, Store, ArchiveSourceId, String) {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Store::open(&dir.path().join("state.db")).unwrap();
        let now = now_ms().unwrap();
        let source = db
            .archive_register(
                ArchiveSourceKey::QqPrivate {
                    observer_account: "1001".into(),
                    peer_id: "2002".into(),
                },
                now,
            )
            .unwrap()
            .source
            .id;
        db.archive_set_grant(
            source,
            SetArchiveGrant {
                expected_revision: 0,
                scope: Some(ArchiveScope {
                    expires_at_ms: now + 120000,
                    retention_ms: 60000,
                    max_records: 100,
                }),
            },
            now,
        )
        .unwrap();
        (dir, db, source, "a".repeat(64))
    }
    fn scope(now: u64) -> SetKnowledgeSync {
        SetKnowledgeSync {
            expected_revision: 0,
            scope: Some(KnowledgeSyncScope {
                archive_revision: 1,
                expires_at_ms: now + 60000,
                max_requests: 2,
                interval_ms: 5000,
                limit: 16,
            }),
        }
    }
    fn finish(db: &mut Store, j: &ArchiveReadJob) {
        db.archive_read_claim(j.request.request_id).unwrap();
        let page=serde_json::from_value(serde_json::json!({"schema_version":1,"store_id":"c22d2694-7541-453f-b3de-ed7d50d2930c","source":{"id":"qq:1001:private:2002","account":"1001","kind":"private","peer":"2002","revision":1,"enabled":true},"from_after":0,"through":0,"next_after":0,"has_more":false,"items":[]})).unwrap();
        db.knowledge_finish(j.request.request_id, page).unwrap();
    }
    #[test]
    fn finite_attempts_survive_restart_and_use_only_committed_cursor() {
        let (dir, mut db, id, reader) = setup();
        let now = now_ms().unwrap();
        assert!(
            db.knowledge_sync_plan(Some(reader.clone()), now)
                .unwrap()
                .0
                .is_none()
        );
        db.knowledge_sync_set(id, scope(now), Some(reader.clone()), now)
            .unwrap();
        let first = db
            .knowledge_sync_plan(Some(reader.clone()), now)
            .unwrap()
            .0
            .unwrap();
        assert_eq!(first.sync_revision, Some(1));
        assert!(
            db.knowledge_sync_plan(Some(reader.clone()), now)
                .unwrap()
                .0
                .is_none()
        );
        finish(&mut db, &first);
        let (_, deadline) = db.knowledge_sync_plan(Some(reader.clone()), now).unwrap();
        let at = deadline.unwrap();
        assert!(at >= now + 5000);
        drop(db);
        let mut db = Store::open(&dir.path().join("state.db")).unwrap();
        assert_eq!(db.knowledge_sync_grant(id).unwrap().used_requests, 1);
        let second = db
            .knowledge_sync_plan(Some(reader.clone()), at)
            .unwrap()
            .0
            .unwrap();
        assert_eq!(second.request.after, Some(first.request.request_id));
        finish(&mut db, &second);
        assert!(
            db.knowledge_sync_plan(Some(reader), at)
                .unwrap()
                .0
                .is_none()
        );
        let g = db.knowledge_sync_grant(id).unwrap();
        assert_eq!(g.used_requests, 2);
        assert_eq!(g.stopped, Some(KnowledgeSyncStop::Exhausted));
    }
    #[test]
    fn reservation_and_job_rollback_together_and_unknown_never_replays() {
        let (_dir, mut db, id, reader) = setup();
        let now = now_ms().unwrap();
        db.knowledge_sync_set(id, scope(now), Some(reader.clone()), now)
            .unwrap();
        db.connection.execute_batch("CREATE TRIGGER reject_reservation BEFORE UPDATE ON knowledge_sync BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
        assert!(db.knowledge_sync_plan(Some(reader.clone()), now).is_err());
        assert_eq!(db.knowledge_sync_grant(id).unwrap().used_requests, 0);
        assert_eq!(
            db.connection
                .query_row("SELECT count(*) FROM archive_reads", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        db.connection
            .execute_batch("DROP TRIGGER reject_reservation")
            .unwrap();
        let j = db
            .knowledge_sync_plan(Some(reader.clone()), now)
            .unwrap()
            .0
            .unwrap();
        db.archive_read_recover().unwrap();
        assert!(
            db.knowledge_sync_plan(Some(reader), now)
                .unwrap()
                .0
                .is_none()
        );
        assert_eq!(
            db.archive_read_job(j.request.request_id).unwrap().state,
            ArchiveReadState::Unknown
        );
        let g = db.knowledge_sync_grant(id).unwrap();
        assert_eq!(g.used_requests, 1);
        assert_eq!(g.stopped, Some(KnowledgeSyncStop::ReadFailed));
    }
    #[test]
    fn revoke_expiry_source_or_reader_change_block_prepared_and_late_completion() {
        for case in 0..4 {
            let (_dir, mut db, id, reader) = setup();
            let now = now_ms().unwrap();
            let selected = scope(now);
            let expires = selected.scope.as_ref().unwrap().expires_at_ms;
            db.knowledge_sync_set(id, selected, Some(reader.clone()), now)
                .unwrap();
            let j = db
                .knowledge_sync_plan(Some(reader.clone()), now)
                .unwrap()
                .0
                .unwrap();
            db.archive_read_claim(j.request.request_id).unwrap();
            match case {
                0 => {
                    db.knowledge_sync_set(
                        id,
                        SetKnowledgeSync {
                            expected_revision: 1,
                            scope: None,
                        },
                        None,
                        now,
                    )
                    .unwrap();
                }
                1 => {
                    db.knowledge_sync_plan(Some(reader.clone()), expires)
                        .unwrap();
                }
                2 => {
                    db.archive_set_grant(
                        id,
                        SetArchiveGrant {
                            expected_revision: 1,
                            scope: None,
                        },
                        now,
                    )
                    .unwrap();
                    db.knowledge_sync_plan(Some(reader.clone()), now).unwrap();
                }
                _ => {
                    db.knowledge_sync_plan(Some("b".repeat(64)), now).unwrap();
                }
            }
            assert!(!db.knowledge_read_allowed(&j, now).unwrap());
            let page=serde_json::from_value(serde_json::json!({"schema_version":1,"store_id":"c22d2694-7541-453f-b3de-ed7d50d2930c","source":{"id":"qq:1001:private:2002","account":"1001","kind":"private","peer":"2002","revision":1,"enabled":true},"from_after":0,"through":0,"next_after":0,"has_more":false,"items":[]})).unwrap();
            assert!(db.knowledge_finish(j.request.request_id, page).is_err());
            assert!(db.knowledge_state(id).unwrap().is_none());
            db.archive_read_fail(
                j.request.request_id,
                ArchiveReadState::Cancelled,
                ArchiveReadFailure::ScopeChanged,
                None,
            )
            .unwrap();
            db.knowledge_sync_plan(Some(reader), now).unwrap();
            assert!(!db.knowledge_sync_grant(id).unwrap().pending);
        }
    }
    #[test]
    fn schema20_upgrade_preserves_existing_receipts_and_enables_no_sync() {
        let (dir, mut db, id, reader) = setup();
        let q = ReadArchive {
            request_id: RequestId::new(),
            expected_revision: 1,
            after: None,
            limit: 4,
        };
        let j = db.knowledge_admit(id, q, reader.clone()).unwrap().0;
        finish(&mut db, &j);
        db.connection
            .execute(
                "UPDATE archive_reads SET snapshot=json_remove(snapshot,'$.sync_revision')",
                [],
            )
            .unwrap();
        let receipt: String = db
            .connection
            .query_row("SELECT snapshot FROM archive_reads", [], |r| r.get(0))
            .unwrap();
        let state = serde_json::to_string(&db.knowledge_state(id).unwrap()).unwrap();
        db.connection
            .execute_batch("DROP TABLE device_reports; DROP TABLE devices; DROP TABLE archive_memory_suppressions; DROP TABLE archive_memory_grants; DROP TABLE knowledge_sync; PRAGMA user_version=20;")
            .unwrap();
        drop(db);
        let mut db = Store::open(&dir.path().join("state.db")).unwrap();
        assert_eq!(
            db.connection
                .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            23
        );
        assert_eq!(
            db.connection
                .query_row("SELECT snapshot FROM archive_reads", [], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            receipt
        );
        assert_eq!(
            serde_json::to_string(&db.knowledge_state(id).unwrap()).unwrap(),
            state
        );
        assert!(
            db.archive_read_job(j.request.request_id)
                .unwrap()
                .sync_revision
                .is_none()
        );
        assert!(db.knowledge_sync_grant(id).unwrap().scope.is_none());
        assert!(
            db.knowledge_sync_plan(Some(reader), crate::domain::now_ms().unwrap())
                .unwrap()
                .0
                .is_none()
        );
    }
    #[test]
    fn full_decision_ledger_still_allows_revocation_and_preserves_spent_history() {
        let (_dir, mut db, id, reader) = setup();
        let now = now_ms().unwrap();
        let g = db
            .knowledge_sync_set(id, scope(now), Some(reader.clone()), now)
            .unwrap();
        db.connection.execute("WITH RECURSIVE n(v) AS (VALUES(2) UNION ALL SELECT v+1 FROM n WHERE v<10000) INSERT INTO knowledge_sync SELECT ?1,v,json_set(?2,'$.revision',v) FROM n",(id.to_string(),serde_json::to_string(&g).unwrap())).unwrap();
        let mut q = scope(now);
        q.expected_revision = 10000;
        assert!(db.knowledge_sync_set(id, q, Some(reader), now).is_err());
        let revoked = db
            .knowledge_sync_set(
                id,
                SetKnowledgeSync {
                    expected_revision: 10000,
                    scope: None,
                },
                None,
                now,
            )
            .unwrap();
        assert_eq!(revoked.revision, 10001);
        assert!(revoked.scope.is_none());
        assert_eq!(db.knowledge_sync_grants(id).unwrap().len(), 100);
    }
    #[test]
    fn invalid_scope_and_stale_decisions_never_create_authority() {
        let (_dir, mut db, id, reader) = setup();
        let now = now_ms().unwrap();
        for case in 0..6 {
            let mut q = scope(now);
            let s = q.scope.as_mut().unwrap();
            match case {
                0 => s.interval_ms = 0,
                1 => s.max_requests = 0,
                2 => s.limit = 33,
                3 => s.expires_at_ms = now,
                4 => s.archive_revision = 2,
                _ => s.expires_at_ms = now + 86400000,
            }
            assert!(
                db.knowledge_sync_set(id, q, Some(reader.clone()), now)
                    .is_err()
            );
        }
        assert_eq!(db.knowledge_sync_grant(id).unwrap().revision, 0);
        db.knowledge_sync_set(id, scope(now), Some(reader.clone()), now)
            .unwrap();
        assert!(
            db.knowledge_sync_set(id, scope(now), Some(reader), now)
                .is_err()
        );
    }
}
