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
/// How long a caught-up read keeps copied bodies available. A scheduled read is
/// good until its successor is due; the cap bounds how late an upstream
/// withdrawal can arrive, even for a long interval. See ADR 041.
pub(super) fn freshness_lease_ms(db: &Connection, j: &ArchiveReadJob) -> Result<u64> {
    if j.sync_revision.is_none() {
        return Ok(30_000);
    }
    let interval = grant(db, j.source_id)?
        .scope
        .ok_or(ControlError::Conflict)?
        .interval_ms;
    Ok(interval.saturating_add(60_000).min(3_600_000))
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
        ensure!(
            q.expected_reader_revision
                .as_ref()
                .is_none_or(|r| r.len() == 64 && r.bytes().all(|b| b.is_ascii_hexdigit())),
            ControlError::InvalidInput
        );
        let tx = self.connection.transaction()?;
        crate::knowledge::source_key(&archives::source(&tx, id)?.key)?;
        let old = grant(&tx, id)?;
        ensure!(old.revision == q.expected_revision, ControlError::Conflict);
        if let Some(s) = &q.scope {
            ensure!(
                q.expected_reader_revision
                    .as_ref()
                    .is_none_or(|expected| Some(expected) == reader.as_ref()),
                ControlError::Conflict
            );
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
                    // A backlog costs one read per page; an idle source waits.
                    let wait = if j.waiting_for_source {
                        s.interval_ms.min(5000)
                    } else {
                        s.interval_ms
                    };
                    g.next_at_ms = j.finished_at_ms.unwrap_or(now).saturating_add(wait);
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
            let held = knowledge::state(&tx, g.source_id)?.map(|v| v.fresh_until_ms);
            let (mut j, fresh) = knowledge::admit(
                &tx,
                g.source_id,
                q,
                reader.clone().ok_or(ControlError::Unavailable)?,
            )?;
            ensure!(fresh, ControlError::Conflict);
            j.sync_revision = Some(g.revision);
            archive_reads::save(&tx, &j)?;
            // Admission clears the lease for an unscheduled read. A routine scheduled
            // read must not open a gap that fails a briefing in progress; the lease
            // still ends on its own if this read fails and stops the schedule.
            if let Some(held) = held {
                tx.execute(
                    "UPDATE knowledge_sources SET snapshot=json_set(snapshot,'$.fresh_until_ms',?2) WHERE source_id=?1",
                    (g.source_id.to_string(), i64::try_from(held)?),
                )?;
            }
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
    const DAY: u64 = 86_400_000;

    fn daily_setup(max_records: u32) -> (tempfile::TempDir, Store, ArchiveSourceId, String) {
        let (dir, mut db, id, reader) = setup();
        let now = now_ms().unwrap();
        db.archive_set_grant(
            id,
            SetArchiveGrant {
                expected_revision: 1,
                scope: Some(ArchiveScope {
                    expires_at_ms: now + DAY,
                    retention_ms: DAY,
                    max_records,
                }),
            },
            now,
        )
        .unwrap();
        (dir, db, id, reader)
    }
    fn daily_scope(now: u64, interval_ms: u64) -> SetKnowledgeSync {
        SetKnowledgeSync {
            expected_reader_revision: None,
            expected_revision: 0,
            scope: Some(KnowledgeSyncScope {
                archive_revision: 2,
                expires_at_ms: now + DAY / 2,
                max_requests: 100,
                interval_ms,
                limit: 32,
            }),
        }
    }
    fn finish_messages(db: &mut Store, j: &ArchiveReadJob, ids: &[u64], has_more: bool) {
        db.archive_read_claim(j.request.request_id).unwrap();
        finish_page(db, j, ids, has_more).unwrap();
    }
    fn finish_page(
        db: &mut Store,
        j: &ArchiveReadJob,
        ids: &[u64],
        has_more: bool,
    ) -> Result<ArchiveReadJob> {
        let now = now_ms().unwrap();
        let through = ids.last().copied().unwrap_or(0) + u64::from(has_more);
        let items: Vec<_> = ids.iter().map(|id| serde_json::json!({"sequence":id,"message":{"id":id,"source_id":"qq:1001:private:2002","external_id":format!("native-{id}"),"sender":"2002","occurred_at_ms":now-3000,"received_at_ms":now-2000,"expires_at_ms":now+DAY,"state":"active","revision":1,"text":"群消息","images":[]}})).collect();
        let page=serde_json::from_value(serde_json::json!({"schema_version":1,"store_id":"c22d2694-7541-453f-b3de-ed7d50d2930c","source":{"id":"qq:1001:private:2002","account":"1001","kind":"private","peer":"2002","revision":1,"enabled":true},"from_after":0,"through":through,"next_after":ids.last().copied().unwrap_or(0),"has_more":has_more,"items":items})).unwrap();
        db.knowledge_finish(j.request.request_id, page)
    }

    #[test]
    fn a_synced_source_stays_readable_between_scheduled_reads_up_to_a_bound() {
        for (interval_ms, lease_ms) in [(900_000, 960_000), (7_200_000, 3_600_000), (5000, 65_000)]
        {
            let (_dir, mut db, id, reader) = daily_setup(100);
            let now = now_ms().unwrap();
            db.knowledge_sync_set(id, daily_scope(now, interval_ms), Some(reader.clone()), now)
                .unwrap();
            let read = db
                .knowledge_sync_plan(Some(reader), now)
                .unwrap()
                .0
                .unwrap();
            finish_messages(&mut db, &read, &[], false);
            let finished = db
                .archive_read_job(read.request.request_id)
                .unwrap()
                .finished_at_ms
                .unwrap();
            assert_eq!(
                db.knowledge_state(id).unwrap().unwrap().fresh_until_ms,
                finished + lease_ms,
                "interval {interval_ms}"
            );
        }
        let (_dir, mut db, id, reader) = daily_setup(100);
        let manual = db
            .knowledge_admit(
                id,
                ReadArchive {
                    request_id: RequestId::new(),
                    expected_revision: 2,
                    after: None,
                    limit: 16,
                },
                reader,
            )
            .unwrap()
            .0;
        finish_messages(&mut db, &manual, &[], false);
        let finished = db
            .archive_read_job(manual.request.request_id)
            .unwrap()
            .finished_at_ms
            .unwrap();
        assert_eq!(
            db.knowledge_state(id).unwrap().unwrap().fresh_until_ms,
            finished + 30_000,
            "an unscheduled read has no later read to rely on"
        );
    }

    #[test]
    fn a_backlog_is_read_soon_and_keeps_but_never_extends_its_lease() {
        let (_dir, mut db, id, reader) = daily_setup(100);
        let now = now_ms().unwrap();
        db.knowledge_sync_set(id, daily_scope(now, 900_000), Some(reader.clone()), now)
            .unwrap();
        let first = db
            .knowledge_sync_plan(Some(reader.clone()), now)
            .unwrap()
            .0
            .unwrap();
        finish_messages(&mut db, &first, &[1], true);
        assert_eq!(
            db.knowledge_state(id).unwrap().unwrap().fresh_until_ms,
            0,
            "a source never caught up has no lease"
        );
        let (none, soon) = db.knowledge_sync_plan(Some(reader.clone()), now).unwrap();
        assert!(none.is_none());
        let soon = soon.unwrap();
        assert!(soon <= now_ms().unwrap() + 5000, "backlog waits at most 5s");
        let second = db
            .knowledge_sync_plan(Some(reader.clone()), soon)
            .unwrap()
            .0
            .unwrap();
        finish_messages(&mut db, &second, &[2], false);
        let lease = db.knowledge_state(id).unwrap().unwrap().fresh_until_ms;
        assert!(lease > now + 900_000, "caught up");
        let (none, later) = db.knowledge_sync_plan(Some(reader.clone()), soon).unwrap();
        assert!(none.is_none());
        let later = later.unwrap();
        assert!(later > soon + 800_000, "an idle source waits its interval");
        let third = db
            .knowledge_sync_plan(Some(reader), later)
            .unwrap()
            .0
            .unwrap();
        finish_messages(&mut db, &third, &[3], true);
        assert_eq!(
            db.knowledge_state(id).unwrap().unwrap().fresh_until_ms,
            lease,
            "a running briefing is not interrupted, and unread changes extend nothing"
        );
    }

    #[test]
    fn the_record_limit_is_per_grant_and_not_refunded_by_expiry() {
        let (_dir, mut db, id, reader) = daily_setup(2);
        let now = now_ms().unwrap();
        db.knowledge_sync_set(id, daily_scope(now, 5000), Some(reader.clone()), now)
            .unwrap();
        let read = db
            .knowledge_sync_plan(Some(reader.clone()), now)
            .unwrap()
            .0
            .unwrap();
        finish_messages(&mut db, &read, &[1, 2], false);
        let at = now + 10_000;
        let full = db
            .knowledge_sync_plan(Some(reader.clone()), at)
            .unwrap()
            .0
            .unwrap();
        db.archive_read_claim(full.request.request_id).unwrap();
        assert!(
            finish_page(&mut db, &full, &[3], false).is_err(),
            "two records are still retained"
        );
        db.connection
            .execute("UPDATE archive_records SET retain_until_ms=1", [])
            .unwrap();
        assert!(
            finish_page(&mut db, &full, &[3], false).is_err(),
            "expiry does not refund what this grant authorized"
        );
        db.archive_read_fail(
            full.request.request_id,
            ArchiveReadState::Failed,
            ArchiveReadFailure::InvalidContent,
            None,
        )
        .unwrap();
        db.archive_set_grant(
            id,
            SetArchiveGrant {
                expected_revision: 2,
                scope: Some(ArchiveScope {
                    expires_at_ms: now + DAY,
                    retention_ms: DAY,
                    max_records: 2,
                }),
            },
            now,
        )
        .unwrap();
        let mut renewed = daily_scope(now, 5000);
        renewed.expected_revision = 1;
        renewed.scope.as_mut().unwrap().archive_revision = 3;
        db.knowledge_sync_set(id, renewed, Some(reader.clone()), now)
            .unwrap();
        let next = db
            .knowledge_sync_plan(Some(reader), now)
            .unwrap()
            .0
            .unwrap();
        db.archive_read_claim(next.request.request_id).unwrap();
        assert!(
            finish_page(&mut db, &next, &[4, 5], false).is_ok(),
            "a new grant is a new authorization, not limited by old history"
        );
    }

    #[test]
    fn a_scheduled_read_opens_no_gap_but_an_unscheduled_read_still_clears_the_lease() {
        let (_dir, mut db, id, reader) = daily_setup(100);
        let now = now_ms().unwrap();
        db.knowledge_sync_set(id, daily_scope(now, 5000), Some(reader.clone()), now)
            .unwrap();
        let first = db
            .knowledge_sync_plan(Some(reader.clone()), now)
            .unwrap()
            .0
            .unwrap();
        finish_messages(&mut db, &first, &[], false);
        let lease = db.knowledge_state(id).unwrap().unwrap().fresh_until_ms;
        assert!(lease > now);
        let second = db
            .knowledge_sync_plan(Some(reader.clone()), now + 10_000)
            .unwrap()
            .0
            .unwrap();
        assert_eq!(
            db.knowledge_state(id).unwrap().unwrap().fresh_until_ms,
            lease,
            "the source stays readable while the routine read is in flight"
        );
        db.archive_read_claim(second.request.request_id).unwrap();
        db.archive_read_fail(
            second.request.request_id,
            ArchiveReadState::Failed,
            ArchiveReadFailure::Unavailable,
            None,
        )
        .unwrap();
        db.knowledge_sync_plan(Some(reader.clone()), now + 10_000)
            .unwrap();
        assert_eq!(
            db.knowledge_sync_grant(id).unwrap().stopped,
            Some(KnowledgeSyncStop::ReadFailed)
        );
        assert_eq!(
            db.knowledge_state(id).unwrap().unwrap().fresh_until_ms,
            lease,
            "a failed read neither extends nor cuts short what was already granted"
        );
        db.knowledge_admit(
            id,
            ReadArchive {
                request_id: RequestId::new(),
                expected_revision: 2,
                after: Some(second.request.after.unwrap()),
                limit: 16,
            },
            reader,
        )
        .unwrap();
        assert_eq!(db.knowledge_state(id).unwrap().unwrap().fresh_until_ms, 0);
    }

    #[test]
    fn knowledge_sync_reviewed_reader_fence_rejects_changed_configuration_but_allows_stop() {
        let (_dir, mut db, id, reader) = setup();
        let now = now_ms().unwrap();
        let mut q = scope(now);
        q.expected_reader_revision = Some("b".repeat(64));
        assert!(
            db.knowledge_sync_set(id, q.clone(), Some(reader.clone()), now)
                .is_err()
        );
        assert_eq!(db.knowledge_sync_grant(id).unwrap().revision, 0);
        q.expected_reader_revision = Some(reader.clone());
        db.knowledge_sync_set(id, q, Some(reader), now).unwrap();
        db.knowledge_sync_set(
            id,
            SetKnowledgeSync {
                expected_revision: 1,
                expected_reader_revision: Some("b".repeat(64)),
                scope: None,
            },
            None,
            now,
        )
        .unwrap();
        assert!(db.knowledge_sync_grant(id).unwrap().scope.is_none());
    }
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
            expected_reader_revision: None,
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
    fn full_read_receipt_capacity_stops_unused_plan_without_spending_or_discarding_history() {
        let (_dir, mut db, id, reader) = setup();
        let now = now_ms().unwrap();
        let (first, _) = db
            .knowledge_admit(
                id,
                ReadArchive {
                    request_id: "00000000-0000-4000-8000-000000000000".parse().unwrap(),
                    expected_revision: 1,
                    after: None,
                    limit: 16,
                },
                reader.clone(),
            )
            .unwrap();
        finish(&mut db, &first);
        let seed = db.archive_read_job(first.request.request_id).unwrap();
        db.connection.execute(
            "WITH RECURSIVE n(v) AS (VALUES(1) UNION ALL SELECT v+1 FROM n WHERE v<9999) INSERT INTO archive_reads SELECT printf('00000000-0000-4000-8000-%012x',v),?1,json_set(?2,'$.request.request_id',printf('00000000-0000-4000-8000-%012x',v),'$.next',printf('00000000-0000-4000-8000-%012x',v)),printf('00000000-0000-4000-8000-%012x',v) FROM n",
            (id.to_string(),serde_json::to_string(&seed).unwrap()),
        ).unwrap();
        let mut decision = scope(now);
        for revision in 0..2 {
            decision.expected_revision = revision;
            decision.scope.as_mut().unwrap().max_requests = 2 + u32::try_from(revision).unwrap();
            db.knowledge_sync_set(id, decision.clone(), Some(reader.clone()), now)
                .unwrap();
            assert!(
                db.knowledge_sync_plan(Some(reader.clone()), now)
                    .unwrap()
                    .0
                    .is_none()
            );
            let g = db.knowledge_sync_grant(id).unwrap();
            assert_eq!(g.stopped, Some(KnowledgeSyncStop::Exhausted));
            assert_eq!(g.used_requests, 0);
            assert!(!g.pending);
            assert!(g.last_read.is_none());
            assert_eq!(
                db.connection
                    .query_row("SELECT count(*) FROM archive_reads", [], |r| r
                        .get::<_, i64>(0))
                    .unwrap(),
                10000
            );
        }
        db.knowledge_sync_set(
            id,
            SetKnowledgeSync {
                expected_revision: 2,
                expected_reader_revision: None,
                scope: None,
            },
            None,
            now,
        )
        .unwrap();
        assert!(db.knowledge_sync_grant(id).unwrap().scope.is_none());
        assert_eq!(
            serde_json::to_value(db.archive_read_job(first.request.request_id).unwrap()).unwrap(),
            serde_json::to_value(seed).unwrap()
        );
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
                            expected_reader_revision: None,
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
            .execute_batch("DROP TABLE actor_memory_contexts; DROP TABLE memory_context_requests; DROP TABLE conversation_memory_contexts; DROP TABLE project_requests; DROP TABLE project_workspaces; DROP TABLE projects; DROP TABLE device_work_continuations; DROP TABLE device_work_links; DROP TABLE device_approval_decisions; DROP TABLE device_executions; DROP TABLE device_command_changes; DROP TABLE device_command_events; DROP TABLE device_commands; DROP TABLE channel_media_frames; DROP TABLE channel_media; DROP TABLE device_reports; DROP TABLE devices; DROP TABLE archive_memory_suppressions; DROP TABLE archive_memory_grants; DROP TABLE knowledge_sync; DROP TABLE briefing_relations; DROP TABLE conversation_memory_effects; DROP TABLE briefing_memory_suppressions; DROP TABLE briefing_disclosures; DROP TABLE briefing_runs; DROP TABLE briefing_batches; DROP TABLE briefing_manifests; DROP TABLE briefing_grants; PRAGMA user_version=20;")
            .unwrap();
        drop(db);
        let mut db = Store::open(&dir.path().join("state.db")).unwrap();
        assert_eq!(
            db.connection
                .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            crate::persistence::SCHEMA_VERSION
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
                    expected_reader_revision: None,
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
