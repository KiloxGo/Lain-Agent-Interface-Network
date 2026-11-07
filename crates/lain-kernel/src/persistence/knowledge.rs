use super::{Store, archive_reads, archives, channels::decode};
use crate::{
    backend::ControlError,
    domain::now_ms,
    knowledge::{Page, projection},
};
use anyhow::{Result, ensure};
use lain_wire::{ArchiveRecordId, ArchiveSourceId, RequestId, archives::*};
use rusqlite::{Connection, OptionalExtension};

pub(super) fn state(
    db: &Connection,
    source: ArchiveSourceId,
) -> Result<Option<KnowledgeArchiveState>> {
    db.query_row(
        "SELECT snapshot FROM knowledge_sources WHERE source_id=?1",
        [source.to_string()],
        |r| r.get::<_, String>(0),
    )
    .optional()?
    .map(decode)
    .transpose()
}
pub(super) fn admit(
    db: &Connection,
    source: ArchiveSourceId,
    q: ReadArchive,
    revision: String,
) -> Result<(ArchiveReadJob, bool)> {
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM archive_reads WHERE id=?1)",
        [q.request_id.to_string()],
        |r| r.get(0),
    )?;
    if !exists {
        match state(db, source)? {
            Some(s) if s.local_grant_revision == q.expected_revision => ensure!(
                s.reader_revision == revision && q.after == Some(s.last_read),
                ControlError::Conflict
            ),
            _ => ensure!(q.after.is_none(), ControlError::Conflict),
        }
    }
    archive_reads::admit(db, source, q, ArchiveReaderKind::Knowledge, Some(revision))
}
pub(super) fn available(db: &Connection, id: ArchiveRecordId, now: u64) -> Result<bool> {
    let linked = db
        .query_row(
            "SELECT source_id,withdrawn,native_id FROM knowledge_records WHERE archive_record_id=?1",
            [id.to_string()],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, bool>(1)?,r.get::<_,String>(2)?)),
        )
        .optional()?;
    let Some((source, withdrawn, native)) = linked else {
        return Ok(true);
    };
    let source: ArchiveSourceId = source.parse()?;
    if let ArchiveSourceKey::QqPrivate {
        observer_account,
        peer_id,
    } = archives::source(db, source)?.key
    {
        let suppressed:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM channel_memory_suppressions x JOIN channel_incoming i ON i.id=x.incoming_id AND i.channel_id=x.channel_id JOIN channel_bindings b ON b.id=i.channel_id WHERE json_extract(b.snapshot,'$.spec.account')=?1 AND json_extract(b.snapshot,'$.spec.peer')=?2 AND json_extract(i.snapshot,'$.input.external_id')=?3)",(observer_account,peer_id,native),|r|r.get(0))?;
        if suppressed {
            return Ok(false);
        }
    }
    let Some(s) = state(db, source)? else {
        return Ok(false);
    };
    Ok(!withdrawn
        && s.remote_source_enabled
        && s.fresh_until_ms > now
        && archives::grant(db, s.source_id)?.revision == s.local_grant_revision)
}
fn withdraw(db: &Connection, id: ArchiveRecordId, kind: &str, now: u64) -> Result<bool> {
    let mut r = archives::record(db, id)?;
    if r.availability != ArchiveAvailability::Stored || r.text.is_none() {
        return Ok(false);
    }
    r.revision += 1;
    r.text = None;
    r.availability = ArchiveAvailability::SourceUnavailable;
    archives::save(db, &r)?;
    db.execute(
        "INSERT INTO archive_edits VALUES (?1,?2,?3,?4)",
        (
            id.to_string(),
            i64::try_from(r.revision)?,
            kind,
            i64::try_from(now)?,
        ),
    )?;
    Ok(true)
}
impl Store {
    pub fn knowledge_state(
        &self,
        source: ArchiveSourceId,
    ) -> Result<Option<KnowledgeArchiveState>> {
        self.archive_source(source)?;
        state(&self.connection, source)
    }
    pub fn knowledge_admit(
        &mut self,
        source: ArchiveSourceId,
        q: ReadArchive,
        revision: String,
    ) -> Result<(ArchiveReadJob, bool)> {
        let tx = self.connection.transaction()?;
        let result = admit(&tx, source, q, revision)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn knowledge_finish(&mut self, id: RequestId, page: Page) -> Result<ArchiveReadJob> {
        let now = now_ms()?;
        let tx = self.connection.transaction()?;
        let mut j = archive_reads::job(&tx, id)?;
        ensure!(
            j.state == ArchiveReadState::Reading && j.reader == ArchiveReaderKind::Knowledge,
            ControlError::Conflict
        );
        ensure!(
            super::knowledge_sync::read_allowed(&tx, &j, now)?,
            ControlError::Conflict
        );
        let g = archives::grant(&tx, j.source_id)?;
        ensure!(
            g.revision == j.request.expected_revision,
            ControlError::Conflict
        );
        let scope = archives::active(&g, now)?;
        let old = state(&tx, j.source_id)?.filter(|s| s.local_grant_revision == g.revision);
        if let Some(s) = &old {
            ensure!(
                j.request.after == Some(s.last_read)
                    && j.reader_revision.as_deref() == Some(s.reader_revision.as_str())
                    && page.store_id == s.store_id,
                ControlError::Conflict
            );
        }
        let allowed = page.source.enabled
            && old.as_ref().is_none_or(|s| {
                s.remote_source_revision == page.source.revision && s.remote_source_enabled
            });
        if !allowed {
            let ids = {
                let mut q=tx.prepare("SELECT archive_record_id FROM knowledge_records WHERE source_id=?1 AND archive_record_id IS NOT NULL")?;
                q.query_map([j.source_id.to_string()], |r| r.get::<_, String>(0))?
                    .collect::<std::result::Result<Vec<_>, _>>()?
            };
            for id in ids {
                if withdraw(&tx, id.parse()?, "knowledge_source_changed", now)? {
                    j.withdrawn_records += 1;
                }
            }
            tx.execute(
                "UPDATE knowledge_records SET withdrawn=1 WHERE source_id=?1",
                [j.source_id.to_string()],
            )?;
        }
        for change in &page.items {
            let m = &change.message;
            let fingerprint = lain_wire::actor_configuration_revision(&serde_json::to_vec(m)?);
            let previous=tx.query_row("SELECT archive_record_id,fingerprint,withdrawn,native_id FROM knowledge_records WHERE source_id=?1 AND remote_id=?2",(j.source_id.to_string(),i64::try_from(m.id)?),|r|Ok((r.get::<_,Option<String>>(0)?,r.get::<_,String>(1)?,r.get::<_,bool>(2)?,r.get::<_,String>(3)?))).optional()?;
            let gone = !allowed || m.state != "active" || m.expires_at_ms <= now;
            if let Some((record, old_fingerprint, withdrawn, native)) = previous {
                ensure!(native == m.external_id, ControlError::Conflict);
                if let Some(record) = record {
                    let rid: ArchiveRecordId = record.parse()?;
                    j.record_ids.push(rid);
                    let mut r = archives::record(&tx, rid)?;
                    if withdrawn
                        || r.grant_revision != g.revision
                        || r.availability != ArchiveAvailability::Stored
                        || r.retain_until_ms <= now
                    {
                        j.duplicate_records += 1;
                        continue;
                    }
                    if gone
                        || (old_fingerprint != fingerprint
                            && r.text_origin == ArchiveTextOrigin::OwnerCorrection)
                    {
                        if withdraw(&tx, rid, "knowledge_withdrawal_or_changed_correction", now)? {
                            j.withdrawn_records += 1;
                        }
                        tx.execute("UPDATE knowledge_records SET withdrawn=1 WHERE source_id=?1 AND remote_id=?2",(j.source_id.to_string(),i64::try_from(m.id)?))?;
                    } else if old_fingerprint != fingerprint {
                        r.text = Some(projection(m)?);
                        r.source_position = Some(change.sequence.to_string());
                        r.revision += 1;
                        r.retain_until_ms = r.retain_until_ms.min(m.expires_at_ms);
                        archives::save(&tx, &r)?;
                        tx.execute(
                            "UPDATE archive_records SET retain_until_ms=?2 WHERE id=?1",
                            (record, i64::try_from(r.retain_until_ms)?),
                        )?;
                        tx.execute("INSERT INTO archive_edits VALUES (?1,?2,'knowledge_projection_update',?3)",(rid.to_string(),i64::try_from(r.revision)?,i64::try_from(now)?))?;
                        tx.execute("UPDATE knowledge_records SET fingerprint=?3 WHERE source_id=?1 AND remote_id=?2",(j.source_id.to_string(),i64::try_from(m.id)?,&fingerprint))?;
                        j.updated_records += 1;
                    } else {
                        j.duplicate_records += 1;
                    }
                } else {
                    j.duplicate_records += 1;
                }
                continue;
            }
            // The limit is what this grant authorized. Expiry never refunds it, but
            // history from an earlier grant is not this grant's to spend.
            let count: i64 = tx.query_row(
                "SELECT count(*) FROM knowledge_records k JOIN archive_records a ON a.id=k.archive_record_id WHERE k.source_id=?1 AND a.grant_revision=?2",
                (j.source_id.to_string(), i64::try_from(g.revision)?),
                |r| r.get(0),
            )?;
            ensure!(count < i64::from(scope.max_records), ControlError::Conflict);
            let record = if gone {
                j.withdrawn_records += 1;
                None
            } else {
                let imported = archives::import(
                    &tx,
                    j.source_id,
                    ImportArchive {
                        expected_revision: g.revision,
                        records: vec![ImportArchiveRecord {
                            external_id: format!("knowledge:{}:{}", page.store_id, m.id),
                            source_position: Some(change.sequence.to_string()),
                            occurred_at_ms: Some(m.occurred_at_ms),
                            speaker: m.sender.clone(),
                            kind: VisibleRecordKind::Message,
                            text: projection(m)?,
                        }],
                    },
                    now,
                    ArchiveTextOrigin::KnowledgeProjection,
                )?;
                ensure!(imported.new_records == 1, ControlError::Conflict);
                let mut r = imported
                    .records
                    .into_iter()
                    .next()
                    .ok_or(ControlError::Conflict)?;
                r.retain_until_ms = r.retain_until_ms.min(m.expires_at_ms);
                archives::save(&tx, &r)?;
                tx.execute(
                    "UPDATE archive_records SET retain_until_ms=?2 WHERE id=?1",
                    (r.id.to_string(), i64::try_from(r.retain_until_ms)?),
                )?;
                j.record_ids.push(r.id);
                j.new_records += 1;
                Some(r.id.to_string())
            };
            tx.execute(
                "INSERT INTO knowledge_records VALUES (?1,?2,?3,?4,?5,?6)",
                (
                    record,
                    j.source_id.to_string(),
                    i64::try_from(m.id)?,
                    &m.external_id,
                    fingerprint,
                    gone,
                ),
            )?;
        }
        j.record_ids.sort();
        j.record_ids.dedup();
        j.state = ArchiveReadState::Completed;
        j.finished_at_ms = Some(now);
        j.adapter_version = Some("qq-knowledge/scoped-v1".into());
        j.waiting_for_source = page.has_more || !allowed;
        j.next = Some(id);
        j.coverage =
            "bounded_source_projection_freshness_lease_no_collection_or_participation_claim".into();
        let lease_ms = super::knowledge_sync::freshness_lease_ms(&tx, &j)?;
        let held_until_ms = old.as_ref().map_or(0, |s| s.fresh_until_ms);
        let s = KnowledgeArchiveState {
            coverage: page.coverage,
            source_id: j.source_id,
            local_grant_revision: g.revision,
            reader_revision: j.reader_revision.clone().ok_or(ControlError::Conflict)?,
            store_id: page.store_id,
            remote_source_revision: old
                .map(|s| s.remote_source_revision)
                .unwrap_or(page.source.revision),
            remote_source_enabled: allowed,
            through: page.next_after,
            last_read: id,
            observed_at_ms: now,
            fresh_until_ms: if allowed && !page.has_more {
                now.saturating_add(lease_ms)
                    .max(held_until_ms)
                    .min(scope.expires_at_ms)
            } else if allowed && j.sync_revision.is_some() {
                // The next scheduled read comes soon. Unread changes extend nothing,
                // but they should not interrupt a briefing already in progress.
                held_until_ms
            } else {
                0
            },
        };
        tx.execute("INSERT INTO knowledge_sources VALUES (?1,?2) ON CONFLICT(source_id) DO UPDATE SET snapshot=excluded.snapshot",(s.source_id.to_string(),serde_json::to_string(&s)?))?;
        archive_reads::save(&tx, &j)?;
        tx.execute(
            "UPDATE archive_reads SET next_cursor=?2 WHERE id=?1",
            (id.to_string(), page.next_after.to_string()),
        )?;
        tx.commit()?;
        Ok(j)
    }
}
