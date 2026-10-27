use super::{Store, archives, channels::decode};
use crate::{backend::ControlError, domain::now_ms};
use anyhow::{Result, ensure};
use lain_wire::{ArchiveSourceId, RequestId, archives::*};
use rusqlite::{Connection, OptionalExtension};

pub(super) fn job(db: &Connection, id: RequestId) -> Result<ArchiveReadJob> {
    decode(
        db.query_row(
            "SELECT snapshot FROM archive_reads WHERE id=?1",
            [id.to_string()],
            |r| r.get::<_, String>(0),
        )
        .optional()?
        .ok_or(ControlError::NotFound)?,
    )
}

pub(super) fn save(db: &Connection, j: &ArchiveReadJob) -> Result<()> {
    ensure!(
        db.execute(
            "UPDATE archive_reads SET snapshot=?2 WHERE id=?1",
            (j.request.request_id.to_string(), serde_json::to_string(j)?)
        )? == 1,
        ControlError::NotFound
    );
    Ok(())
}
pub(super) fn admit(
    db: &Connection,
    source: ArchiveSourceId,
    q: ReadArchive,
    reader: ArchiveReaderKind,
    reader_revision: Option<String>,
) -> Result<(ArchiveReadJob, bool)> {
    if let Some(encoded) = db
        .query_row(
            "SELECT snapshot FROM archive_reads WHERE id=?1",
            [q.request_id.to_string()],
            |r| r.get::<_, String>(0),
        )
        .optional()?
    {
        let old: ArchiveReadJob = decode(encoded)?;
        ensure!(
            old.source_id == source
                && old.request == q
                && old.reader == reader
                && old.reader_revision == reader_revision,
            ControlError::Conflict
        );
        return Ok((old, false));
    }
    let g = archives::grant(db, source)?;
    ensure!(g.revision == q.expected_revision, ControlError::Conflict);
    archives::active(&g, now_ms()?)?;
    ensure!(!db.query_row("SELECT EXISTS(SELECT 1 FROM archive_reads WHERE json_extract(snapshot,'$.state') IN ('prepared','reading'))",[],|r|r.get::<_,bool>(0))?,ControlError::Conflict);
    ensure!(
        db.query_row("SELECT count(*) FROM archive_reads", [], |r| r
            .get::<_, i64>(0))?
            < 10000,
        ControlError::Conflict
    );
    if let Some(after) = q.after {
        let previous = job(db, after)?;
        ensure!(
            previous.state == ArchiveReadState::Completed
                && previous.source_id == source
                && previous.request.expected_revision == g.revision
                && previous.reader == reader
                && previous.reader_revision == reader_revision
                && previous.next == Some(after),
            ControlError::Conflict
        );
    }
    let j = ArchiveReadJob {
        reader,
        reader_revision,
        sync_revision: None,
        request: q,
        source_id: source,
        state: ArchiveReadState::Prepared,
        created_at_ms: now_ms()?,
        started_at_ms: None,
        finished_at_ms: None,
        adapter_version: None,
        record_ids: vec![],
        new_records: 0,
        duplicate_records: 0,
        updated_records: 0,
        withdrawn_records: 0,
        omitted_items: 0,
        waiting_for_source: false,
        next: None,
        failure: None,
        provider_error_code: None,
        coverage: "bounded_terminal_turn_visible_items_partial_no_participation_claim".into(),
    };
    db.execute(
        "INSERT INTO archive_reads VALUES (?1,?2,?3,NULL)",
        (
            j.request.request_id.to_string(),
            source.to_string(),
            serde_json::to_string(&j)?,
        ),
    )?;
    if reader == ArchiveReaderKind::Knowledge {
        db.execute("UPDATE knowledge_sources SET snapshot=json_set(snapshot,'$.fresh_until_ms',0) WHERE source_id=?1",[source.to_string()])?;
    }
    Ok((j, true))
}
impl Store {
    pub fn archive_read_job(&self, id: RequestId) -> Result<ArchiveReadJob> {
        job(&self.connection, id)
    }
    pub fn archive_read_admit(
        &mut self,
        source: ArchiveSourceId,
        q: ReadWorkerArchive,
    ) -> Result<(ArchiveReadJob, bool)> {
        self.archive_read_admit_for(source, q, ArchiveReaderKind::Codex, None)
    }
    pub fn archive_read_admit_for(
        &mut self,
        source: ArchiveSourceId,
        q: ReadArchive,
        reader: ArchiveReaderKind,
        reader_revision: Option<String>,
    ) -> Result<(ArchiveReadJob, bool)> {
        let tx = self.connection.transaction()?;
        let result = admit(&tx, source, q, reader, reader_revision)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn archive_read_claim(
        &mut self,
        id: RequestId,
    ) -> Result<(ArchiveReadJob, ArchiveSource, Option<String>)> {
        let tx = self.connection.transaction()?;
        let mut j = job(&tx, id)?;
        ensure!(
            j.state == ArchiveReadState::Prepared,
            ControlError::Conflict
        );
        ensure!(
            super::knowledge_sync::read_allowed(&tx, &j, now_ms()?)?,
            ControlError::Conflict
        );
        let g = archives::grant(&tx, j.source_id)?;
        ensure!(
            g.revision == j.request.expected_revision,
            ControlError::Conflict
        );
        archives::active(&g, now_ms()?)?;
        let source: ArchiveSource = decode(tx.query_row(
            "SELECT snapshot FROM archive_sources WHERE id=?1",
            [j.source_id.to_string()],
            |r| r.get::<_, String>(0),
        )?)?;
        let cursor = if let Some(after) = j.request.after {
            tx.query_row(
                "SELECT next_cursor FROM archive_reads WHERE id=?1",
                [after.to_string()],
                |r| r.get::<_, Option<String>>(0),
            )?
        } else {
            None
        };
        ensure!(
            j.request.after.is_none() || cursor.is_some(),
            ControlError::Conflict
        );
        j.state = ArchiveReadState::Reading;
        j.started_at_ms = Some(now_ms()?);
        save(&tx, &j)?;
        tx.commit()?;
        Ok((j, source, cursor))
    }
    pub fn archive_read_finish(
        &mut self,
        id: RequestId,
        batch: crate::backend::archive_read::Batch,
    ) -> Result<ArchiveReadJob> {
        let tx = self.connection.transaction()?;
        let mut j = job(&tx, id)?;
        ensure!(j.state == ArchiveReadState::Reading, ControlError::Conflict);
        let g = archives::grant(&tx, j.source_id)?;
        ensure!(
            g.revision == j.request.expected_revision,
            ControlError::Conflict
        );
        archives::active(&g, now_ms()?)?;
        if !batch.records.is_empty() {
            let imported = archives::import(
                &tx,
                j.source_id,
                ImportArchive {
                    expected_revision: j.request.expected_revision,
                    records: batch.records,
                },
                now_ms()?,
                ArchiveTextOrigin::AdapterObservation,
            )?;
            j.new_records = imported.new_records;
            j.duplicate_records = imported.duplicate_records;
            j.record_ids = imported.records.into_iter().map(|r| r.id).collect();
        }
        j.state = ArchiveReadState::Completed;
        j.finished_at_ms = Some(now_ms()?);
        j.adapter_version = Some(batch.version);
        j.omitted_items = batch.omitted;
        j.waiting_for_source = batch.waiting;
        j.next = batch.next.as_ref().map(|_| id);
        save(&tx, &j)?;
        tx.execute(
            "UPDATE archive_reads SET next_cursor=?2 WHERE id=?1",
            (id.to_string(), batch.next),
        )?;
        tx.commit()?;
        Ok(j)
    }
    pub fn archive_read_fail(
        &mut self,
        id: RequestId,
        state: ArchiveReadState,
        reason: ArchiveReadFailure,
        code: Option<i64>,
    ) -> Result<ArchiveReadJob> {
        ensure!(
            matches!(
                state,
                ArchiveReadState::Failed | ArchiveReadState::Cancelled | ArchiveReadState::Unknown
            ),
            ControlError::InvalidInput
        );
        let mut j = job(&self.connection, id)?;
        if j.state.unfinished() {
            j.state = state;
            j.failure = Some(reason);
            j.provider_error_code = code;
            j.finished_at_ms = Some(now_ms()?);
            save(&self.connection, &j)?;
        }
        Ok(j)
    }
    pub fn archive_read_recover(&mut self) -> Result<()> {
        self.connection.execute("UPDATE archive_reads SET snapshot=json_set(snapshot,'$.state','unknown','$.failure','interrupted','$.finished_at_ms',?1) WHERE json_extract(snapshot,'$.state') IN ('prepared','reading')",[i64::try_from(now_ms()?)?])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::archive_read::Batch;
    fn setup() -> (tempfile::TempDir, Store, ArchiveSourceId) {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(&dir.path().join("state.db")).unwrap();
        let source = s
            .archive_register(
                ArchiveSourceKey::CodexSession {
                    installation: "codex/local".into(),
                    session_id: "synthetic-session".into(),
                    workspace: dir.path().display().to_string(),
                },
                now_ms().unwrap(),
            )
            .unwrap()
            .source
            .id;
        s.archive_set_grant(
            source,
            SetArchiveGrant {
                expected_revision: 0,
                scope: Some(ArchiveScope {
                    expires_at_ms: now_ms().unwrap() + 60000,
                    retention_ms: 30000,
                    max_records: 100,
                }),
            },
            now_ms().unwrap(),
        )
        .unwrap();
        (dir, s, source)
    }
    fn request() -> ReadWorkerArchive {
        ReadWorkerArchive {
            request_id: RequestId::new(),
            expected_revision: 1,
            after: None,
            limit: 4,
        }
    }
    fn batch() -> Batch {
        Batch {
            records: vec![ImportArchiveRecord {
                external_id: "synthetic-item".into(),
                source_position: None,
                occurred_at_ms: Some(100),
                speaker: "worker".into(),
                kind: VisibleRecordKind::AssistantMessage,
                text: "synthetic visible text".into(),
            }],
            next: Some("opaque-synthetic-cursor".into()),
            omitted: 0,
            waiting: false,
            version: "0.160.1".into(),
        }
    }
    #[test]
    fn read_receipt_and_import_are_atomic_and_idempotent_with_owned_continuation() {
        let (_dir, mut s, source) = setup();
        let req = request();
        let id = req.request_id;
        s.archive_read_admit(source, req.clone()).unwrap();
        s.archive_read_claim(id).unwrap();
        s.connection.execute_batch("CREATE TRIGGER fail_read_receipt BEFORE UPDATE ON archive_reads WHEN json_extract(NEW.snapshot,'$.state')='completed' BEGIN SELECT RAISE(ABORT,'synthetic receipt failure'); END;").unwrap();
        assert!(s.archive_read_finish(id, batch()).is_err());
        assert_eq!(s.archive_source(source).unwrap().total_records, 0);
        assert_eq!(
            s.archive_read_job(id).unwrap().state,
            ArchiveReadState::Reading
        );
        s.connection
            .execute_batch("DROP TRIGGER fail_read_receipt;")
            .unwrap();
        let done = s.archive_read_finish(id, batch()).unwrap();
        assert_eq!(done.new_records, 1);
        assert!(!s.archive_read_admit(source, req).unwrap().1);
        let mut next = request();
        next.after = Some(id);
        s.archive_read_admit(source, next.clone()).unwrap();
        assert_eq!(
            s.archive_read_claim(next.request_id).unwrap().2.as_deref(),
            Some("opaque-synthetic-cursor")
        );
    }
    #[test]
    fn interrupted_reads_recover_unknown_without_replaying_or_importing() {
        for claim in [false, true] {
            let (dir, mut s, source) = setup();
            let req = request();
            let id = req.request_id;
            s.archive_read_admit(source, req.clone()).unwrap();
            if claim {
                s.archive_read_claim(id).unwrap();
            }
            assert!(s.archive_read_admit(source, request()).is_err());
            drop(s);
            let k = crate::kernel::Kernel::open(&dir.path().join("state.db")).unwrap();
            assert_eq!(
                k.archive_read_job(id).unwrap().state,
                ArchiveReadState::Unknown
            );
            k.mark_ready().unwrap();
            assert!(!k.archive_read_admit(source, req).unwrap().1);
            assert_eq!(k.archive_source(source).unwrap().total_records, 0);
        }
    }
    #[test]
    fn revoke_or_cancel_blocks_late_import_and_continuation_cannot_cross_epoch() {
        let (_dir, mut s, source) = setup();
        let q = request();
        let id = q.request_id;
        s.archive_read_admit(source, q).unwrap();
        s.archive_read_claim(id).unwrap();
        s.archive_set_grant(
            source,
            SetArchiveGrant {
                expected_revision: 1,
                scope: None,
            },
            now_ms().unwrap(),
        )
        .unwrap();
        assert!(s.archive_read_finish(id, batch()).is_err());
        assert_eq!(s.archive_source(source).unwrap().total_records, 0);
        s.archive_read_fail(
            id,
            ArchiveReadState::Cancelled,
            ArchiveReadFailure::ScopeChanged,
            None,
        )
        .unwrap();
        assert_eq!(
            s.archive_read_job(id).unwrap().state,
            ArchiveReadState::Cancelled
        );
    }
}
