use super::{Store, channels::decode};
use crate::backend::ControlError;
use anyhow::{Result, ensure};
use lain_wire::{ArchiveRecordId, ArchiveSourceId, archives::*};
use rusqlite::{Connection, OptionalExtension};
fn sql(n: u64) -> Result<i64> {
    i64::try_from(n).map_err(|_| ControlError::InvalidInput.into())
}

pub(super) fn source(db: &Connection, id: ArchiveSourceId) -> Result<ArchiveSource> {
    decode(
        db.query_row(
            "SELECT snapshot FROM archive_sources WHERE id=?1",
            [id.to_string()],
            |r| r.get::<_, String>(0),
        )
        .optional()?
        .ok_or(ControlError::NotFound)?,
    )
}
pub(super) fn grant(db: &Connection, id: ArchiveSourceId) -> Result<ArchiveGrant> {
    source(db, id)?;
    db.query_row(
        "SELECT snapshot FROM archive_grants WHERE source_id=?1 ORDER BY revision DESC LIMIT 1",
        [id.to_string()],
        |r| r.get::<_, String>(0),
    )
    .optional()?
    .map(decode)
    .transpose()
    .map(|g| {
        g.unwrap_or(ArchiveGrant {
            source_id: id,
            revision: 0,
            scope: None,
            changed_at_ms: 0,
        })
    })
}
pub(super) fn active(g: &ArchiveGrant, now: u64) -> Result<&ArchiveScope> {
    g.scope
        .as_ref()
        .filter(|s| now < s.expires_at_ms)
        .ok_or(ControlError::Conflict.into())
}
pub(super) fn record(db: &Connection, id: ArchiveRecordId) -> Result<ArchiveRecord> {
    decode(
        db.query_row(
            "SELECT snapshot FROM archive_records WHERE id=?1",
            [id.to_string()],
            |r| r.get::<_, String>(0),
        )
        .optional()?
        .ok_or(ControlError::NotFound)?,
    )
}
fn save(db: &Connection, r: &ArchiveRecord) -> Result<()> {
    ensure!(
        db.execute(
            "UPDATE archive_records SET snapshot=?2 WHERE id=?1",
            (r.id.to_string(), serde_json::to_string(r)?)
        )? == 1,
        ControlError::NotFound
    );
    Ok(())
}
fn project(mut r: ArchiveRecord, g: &ArchiveGrant, now: u64) -> ArchiveRecord {
    if r.availability != ArchiveAvailability::Stored {
        r.text = None;
        return r;
    }
    if r.retain_until_ms <= now {
        r.text = None;
        r.availability = ArchiveAvailability::Expired;
    } else if g.scope.is_none() {
        r.text = None;
        r.availability = ArchiveAvailability::Revoked;
    } else if active(g, now).is_err() {
        r.text = None;
        r.availability = ArchiveAvailability::GrantExpired;
    } else if g.revision != r.grant_revision {
        r.text = None;
        r.availability = ArchiveAvailability::PreviousGrant;
    }
    r
}
fn through(db: &Connection, id: ArchiveSourceId) -> Result<u64> {
    Ok(db
        .query_row(
            "SELECT COALESCE(max(sequence),0) FROM archive_records WHERE source_id=?1",
            [id.to_string()],
            |r| r.get::<_, i64>(0),
        )?
        .try_into()?)
}
fn view(db: &Connection, id: ArchiveSourceId) -> Result<ArchiveSourceView> {
    let source = source(db, id)?;
    let grant = grant(db, id)?;
    let(total_records,current_grant_records):(i64,i64)=db.query_row("SELECT count(*),COALESCE(sum(grant_revision=?2),0) FROM archive_records WHERE source_id=?1",(id.to_string(),sql(grant.revision)?),|r|Ok((r.get(0)?,r.get(1)?)))?;
    Ok(ArchiveSourceView {
        source,
        grant,
        total_records: total_records.try_into()?,
        current_grant_records: current_grant_records.try_into()?,
        committed_through: through(db, id)?,
        coverage: "selected_visible_slice_origin_per_record_no_completeness_or_participation_claim"
            .into(),
    })
}
pub(super) fn import(
    db: &Connection,
    id: ArchiveSourceId,
    q: ImportArchive,
    now: u64,
    origin: ArchiveTextOrigin,
) -> Result<ArchiveImportReceipt> {
    let g = grant(db, id)?;
    ensure!(g.revision == q.expected_revision, ControlError::Conflict);
    let scope = active(&g, now)?;
    let mut count = db.query_row(
        "SELECT count(*) FROM archive_records WHERE source_id=?1 AND grant_revision=?2",
        (id.to_string(), sql(g.revision)?),
        |r| r.get::<_, i64>(0),
    )?;
    let mut result = ArchiveImportReceipt {
        records: Vec::new(),
        new_records: 0,
        duplicate_records: 0,
        committed_through: 0,
    };
    for input in q.records {
        let fingerprint = lain_wire::actor_configuration_revision(&serde_json::to_vec(&input)?);
        let old=db.query_row("SELECT id,ingest_fingerprint FROM archive_records WHERE source_id=?1 AND external_id=?2",(id.to_string(),&input.external_id),|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).optional()?;
        if let Some((old_id, old_fingerprint)) = old {
            ensure!(old_fingerprint == fingerprint, ControlError::Conflict);
            result
                .records
                .push(project(record(db, old_id.parse()?)?, &g, now));
            result.duplicate_records += 1;
            continue;
        }
        ensure!(count < i64::from(scope.max_records), ControlError::Conflict);
        let mut r = ArchiveRecord {
            id: ArchiveRecordId::new(),
            sequence: 0,
            source_id: id,
            grant_revision: g.revision,
            revision: 1,
            external_id: input.external_id,
            source_position: input.source_position,
            occurred_at_ms: input.occurred_at_ms,
            received_at_ms: now,
            retain_until_ms: now.saturating_add(scope.retention_ms).min(i64::MAX as u64),
            speaker: input.speaker,
            kind: input.kind,
            text: Some(input.text),
            text_origin: origin,
            availability: ArchiveAvailability::Stored,
        };
        db.execute("INSERT INTO archive_records(id,source_id,external_id,grant_revision,ingest_fingerprint,retain_until_ms,snapshot) VALUES (?1,?2,?3,?4,?5,?6,?7)",(r.id.to_string(),id.to_string(),&r.external_id,sql(g.revision)?,fingerprint,sql(r.retain_until_ms)?,serde_json::to_string(&r)?))?;
        r.sequence = db.last_insert_rowid().try_into()?;
        save(db, &r)?;
        count += 1;
        result.new_records += 1;
        result.records.push(r);
    }
    result.committed_through = through(db, id)?;
    Ok(result)
}
impl Store {
    pub fn archive_register(
        &mut self,
        key: ArchiveSourceKey,
        now: u64,
    ) -> Result<ArchiveSourceView> {
        let tx = self.connection.transaction()?;
        let encoded = serde_json::to_string(&key)?;
        if let Some(id) = tx
            .query_row(
                "SELECT id FROM archive_sources WHERE source_key=?1",
                [&encoded],
                |r| r.get::<_, String>(0),
            )
            .optional()?
        {
            return view(&tx, id.parse()?);
        }
        ensure!(
            tx.query_row("SELECT count(*) FROM archive_sources", [], |r| r
                .get::<_, i64>(0))?
                < 64,
            ControlError::Conflict
        );
        let s = ArchiveSource {
            id: ArchiveSourceId::new(),
            key,
            created_at_ms: now,
        };
        tx.execute(
            "INSERT INTO archive_sources VALUES (?1,?2,?3)",
            (s.id.to_string(), encoded, serde_json::to_string(&s)?),
        )?;
        let result = view(&tx, s.id)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn archive_sources(&self) -> Result<Vec<ArchiveSourceView>> {
        let mut q = self
            .connection
            .prepare("SELECT id FROM archive_sources ORDER BY id LIMIT 64")?;
        q.query_map([], |r| r.get::<_, String>(0))?
            .map(|r| view(&self.connection, r?.parse()?))
            .collect()
    }
    pub fn archive_source(&self, id: ArchiveSourceId) -> Result<ArchiveSourceView> {
        view(&self.connection, id)
    }
    pub fn archive_grants(&self, id: ArchiveSourceId) -> Result<Vec<ArchiveGrant>> {
        source(&self.connection, id)?;
        let mut q=self.connection.prepare("SELECT snapshot FROM archive_grants WHERE source_id=?1 ORDER BY revision DESC LIMIT 100")?;
        q.query_map([id.to_string()], |r| r.get::<_, String>(0))?
            .map(|r| decode(r?))
            .collect()
    }
    pub fn archive_edits(&self, id: ArchiveRecordId) -> Result<Vec<ArchiveEdit>> {
        record(&self.connection, id)?;
        let mut q=self.connection.prepare("SELECT revision,kind,at_ms FROM archive_edits WHERE record_id=?1 ORDER BY revision DESC LIMIT 100")?;
        q.query_map([id.to_string()], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })?
        .map(|r| {
            let (revision, kind, at) = r?;
            Ok(ArchiveEdit {
                record_id: id,
                revision: revision.try_into()?,
                kind,
                at_ms: at.try_into()?,
            })
        })
        .collect()
    }
    pub fn archive_set_grant(
        &mut self,
        id: ArchiveSourceId,
        q: SetArchiveGrant,
        now: u64,
    ) -> Result<ArchiveGrant> {
        let tx = self.connection.transaction()?;
        let old = grant(&tx, id)?;
        ensure!(old.revision == q.expected_revision, ControlError::Conflict);
        let revision = old
            .revision
            .checked_add(1)
            .filter(|n| *n <= i64::MAX as u64)
            .ok_or(ControlError::Conflict)?;
        let g = ArchiveGrant {
            source_id: id,
            revision,
            scope: q.scope,
            changed_at_ms: now,
        };
        tx.execute(
            "INSERT INTO archive_grants VALUES (?1,?2,?3)",
            (id.to_string(), sql(revision)?, serde_json::to_string(&g)?),
        )?;
        tx.commit()?;
        Ok(g)
    }
    pub fn archive_prune(&mut self, now: u64) -> Result<()> {
        self.connection.execute("UPDATE archive_records SET snapshot=json_set(snapshot,'$.text',NULL,'$.availability','expired','$.revision',json_extract(snapshot,'$.revision')+1) WHERE retain_until_ms<=?1 AND json_extract(snapshot,'$.text') IS NOT NULL",[sql(now)?])?;
        Ok(())
    }
    pub fn archive_deadline(&mut self, now: u64) -> Result<Option<u64>> {
        self.archive_prune(now)?;
        let t:Option<i64>=self.connection.query_row("SELECT min(retain_until_ms) FROM archive_records WHERE json_extract(snapshot,'$.text') IS NOT NULL",[],|r|r.get(0))?;
        t.map(|t| Ok(t.try_into()?)).transpose()
    }
    pub fn archive_import(
        &mut self,
        id: ArchiveSourceId,
        q: ImportArchive,
        now: u64,
    ) -> Result<ArchiveImportReceipt> {
        let tx = self.connection.transaction()?;
        let result = import(&tx, id, q, now, ArchiveTextOrigin::OwnerImport)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn archive_record(&self, id: ArchiveRecordId, now: u64) -> Result<ArchiveRecord> {
        let r = record(&self.connection, id)?;
        let g = grant(&self.connection, r.source_id)?;
        Ok(project(r, &g, now))
    }
    pub fn archive_query(
        &self,
        id: ArchiveSourceId,
        q: ArchiveQuery,
        now: u64,
    ) -> Result<ArchivePage> {
        let g = grant(&self.connection, id)?;
        active(&g, now)?;
        ensure!(
            g.revision == q.expected_revision && (1..=20).contains(&q.limit),
            ControlError::Conflict
        );
        let high = through(&self.connection, id)?;
        let (after, through) = if let Some(c) = q.cursor {
            ensure!(
                c.source_id == id
                    && c.grant_revision == g.revision
                    && c.after <= c.through
                    && c.through <= high,
                ControlError::Conflict
            );
            (c.after, c.through)
        } else {
            (0, high)
        };
        let mut s=self.connection.prepare("SELECT snapshot FROM archive_records WHERE source_id=?1 AND grant_revision=?2 AND sequence>?3 AND sequence<=?4 ORDER BY sequence LIMIT ?5")?;
        let records = s
            .query_map(
                (
                    id.to_string(),
                    sql(g.revision)?,
                    sql(after)?,
                    sql(through)?,
                    i64::from(q.limit),
                ),
                |r| r.get::<_, String>(0),
            )?
            .map(|r| Ok(project(decode(r?)?, &g, now)))
            .collect::<Result<Vec<ArchiveRecord>>>()?;
        let last = records.last().map_or(after, |r| r.sequence);
        let more=self.connection.query_row("SELECT EXISTS(SELECT 1 FROM archive_records WHERE source_id=?1 AND grant_revision=?2 AND sequence>?3 AND sequence<=?4)",(id.to_string(),sql(g.revision)?,sql(last)?,sql(through)?),|r|r.get::<_,bool>(0))?;
        Ok(ArchivePage {
            records,
            next: more.then_some(ArchiveCursor {
                source_id: id,
                grant_revision: g.revision,
                after: last,
                through,
            }),
            through,
        })
    }
    pub fn archive_forget(
        &mut self,
        id: ArchiveRecordId,
        revision: u64,
        now: u64,
    ) -> Result<ArchiveRecord> {
        let tx = self.connection.transaction()?;
        let mut r = record(&tx, id)?;
        if r.availability == ArchiveAvailability::Forgotten {
            ensure!(
                revision == r.revision || revision.checked_add(1) == Some(r.revision),
                ControlError::Conflict
            );
            return Ok(project(r.clone(), &grant(&tx, r.source_id)?, now));
        }
        ensure!(r.revision == revision, ControlError::Conflict);
        r.text = None;
        r.availability = ArchiveAvailability::Forgotten;
        r.revision = r.revision.checked_add(1).ok_or(ControlError::Conflict)?;
        save(&tx, &r)?;
        tx.execute(
            "INSERT INTO archive_edits VALUES (?1,?2,'forget',?3)",
            (id.to_string(), sql(r.revision)?, sql(now)?),
        )?;
        tx.commit()?;
        Ok(r)
    }
    pub fn archive_correct(
        &mut self,
        id: ArchiveRecordId,
        q: CorrectArchiveRecord,
        now: u64,
    ) -> Result<ArchiveRecord> {
        let tx = self.connection.transaction()?;
        let mut r = record(&tx, id)?;
        let g = grant(&tx, r.source_id)?;
        active(&g, now)?;
        ensure!(
            g.revision == r.grant_revision
                && r.revision == q.expected_revision
                && r.availability == ArchiveAvailability::Stored
                && now < r.retain_until_ms,
            ControlError::Conflict
        );
        r.text = Some(q.text);
        r.text_origin = ArchiveTextOrigin::OwnerCorrection;
        r.revision = r.revision.checked_add(1).ok_or(ControlError::Conflict)?;
        save(&tx, &r)?;
        tx.execute(
            "INSERT INTO archive_edits VALUES (?1,?2,'correct',?3)",
            (id.to_string(), sql(r.revision)?, sql(now)?),
        )?;
        tx.commit()?;
        Ok(r)
    }
}
