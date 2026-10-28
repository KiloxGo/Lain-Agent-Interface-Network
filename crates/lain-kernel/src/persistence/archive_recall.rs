//! Selected archive evidence, never an import, collection or Worker capability.
use super::{Store, archives, channels::decode};
use crate::{
    actor::{domain::*, tools::Tool},
    backend::ControlError,
    domain::now_ms,
    redaction::Redactor,
};
use anyhow::{Result, ensure};
use lain_wire::{ArchiveRecordId, ChannelId, archives::*};
use rusqlite::{Connection, OptionalExtension};
use serde_json::{Value, json};

// The absent decision is disabled, including on upgraded databases.
pub(super) fn decision(db: &Connection, channel: ChannelId) -> Result<ArchiveRecallGrant> {
    db.query_row("SELECT snapshot FROM archive_recall_grants WHERE channel_id=?1 ORDER BY revision DESC LIMIT 1", [channel.to_string()], |r|r.get::<_,String>(0))
        .optional()?.map(decode).transpose().map(|g|g.unwrap_or(ArchiveRecallGrant{channel_id:channel,revision:0,scope:None,changed_at_ms:0}))
}
fn scope(g: &ArchiveRecallGrant, now: u64) -> Result<&ArchiveRecallScope> {
    g.scope
        .as_ref()
        .filter(|s| now < s.expires_at_ms)
        .ok_or(ControlError::Unavailable.into())
}
fn allowed(
    db: &Connection,
    g: &ArchiveRecallGrant,
    source: lain_wire::ArchiveSourceId,
    now: u64,
) -> Result<()> {
    let s = scope(g, now)?;
    let selected = s
        .sources
        .iter()
        .find(|r| r.source_id == source)
        .ok_or(ControlError::NotFound)?;
    let original = archives::grant(db, source)?;
    ensure!(
        original.revision == selected.grant_revision,
        ControlError::Conflict
    );
    archives::active(&original, now)?;
    Ok(())
}
pub(super) fn visible(
    db: &Connection,
    g: &ArchiveRecallGrant,
    id: ArchiveRecordId,
    now: u64,
) -> Result<ArchiveRecord> {
    let r = archives::record(db, id)?;
    allowed(db, g, r.source_id, now)?;
    ensure!(
        super::knowledge::available(db, r.id, now)?
            && !super::archive_memory::suppressed(db, g.channel_id, r.id)?
            && r.availability == ArchiveAvailability::Stored
            && r.text.is_some()
            && r.retain_until_ms > now
            && archives::grant(db, r.source_id)?.revision == r.grant_revision,
        ControlError::Conflict
    );
    Ok(r)
}
fn version(r: &ArchiveRecord) -> ArchiveRecallVersion {
    ArchiveRecallVersion {
        record_id: r.id,
        revision: r.revision,
        source_id: r.source_id,
        source_grant_revision: r.grant_revision,
    }
}
fn project(db: &Connection, r: &ArchiveRecord, redactor: &Redactor) -> Result<Value> {
    crate::archives::safe(r, redactor)?;
    let text = r.text.as_deref().ok_or(ControlError::Conflict)?;
    let mut end = text.len().min(1024);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let v = json!({"record_id":r.id,"source_id":r.source_id,"source":archives::source(db,r.source_id)?,"revision":r.revision,"source_grant_revision":r.grant_revision,"occurred_at_ms":r.occurred_at_ms,"received_at_ms":r.received_at_ms,"speaker":r.speaker,"kind":r.kind,"text_origin":r.text_origin,"text":&text[..end],"original_bytes":text.len(),"projection_truncated":end<text.len(),"evidence":"archived_report_not_independently_verified_no_participation_claim"});
    crate::archives::safe(&v, redactor)?;
    Ok(v)
}
fn active(db: &Connection, t: &ActorTurn) -> Result<ArchiveRecallGrant> {
    let c = t.private_chat.as_ref().ok_or(ControlError::Unavailable)?;
    let b = super::channels::binding(db, c.channel_id)?;
    ensure!(
        t.state == TurnState::Running
            && b.reply_enabled
            && b.grant_revision == c.grant_revision
            && b.conversation_id == t.conversation_id
            && now_ms()? < c.expires_at_ms,
        ControlError::Conflict
    );
    let g = decision(db, c.channel_id)?;
    scope(&g, now_ms()?)?;
    ensure!(
        c.archive_recall_revision.is_none_or(|r| r == g.revision),
        ControlError::Conflict
    );
    Ok(g)
}
pub(super) fn selected_current(db: &Connection, t: &ActorTurn) -> Result<bool> {
    let Some(c) = &t.private_chat else {
        return Ok(true);
    };
    let Some(revision) = c.archive_recall_revision else {
        return Ok(c.archive_versions.is_empty());
    };
    let g = decision(db, c.channel_id)?;
    if g.revision != revision || scope(&g, now_ms()?).is_err() {
        return Ok(false);
    }
    for source in &scope(&g, now_ms()?)?.sources {
        if allowed(db, &g, source.source_id, now_ms()?).is_err() {
            return Ok(false);
        }
    }
    for v in &c.archive_versions {
        let Ok(r) = visible(db, &g, v.record_id, now_ms()?) else {
            return Ok(false);
        };
        if r.revision != v.revision
            || r.source_id != v.source_id
            || r.grant_revision != v.source_grant_revision
            || crate::archives::safe(&r, &Redactor::new(vec![]).for_runtime(None)).is_err()
        {
            return Ok(false);
        }
    }
    Ok(true)
}
fn audit(
    db: &Connection,
    t: &ActorTurn,
    action: Option<ActionId>,
    request: Option<u32>,
) -> Result<()> {
    let c = t.private_chat.as_ref().ok_or(ControlError::Unavailable)?;
    let metadata = json!({"grant_revision":c.archive_recall_revision,"versions":c.archive_versions,"recorded_at_ms":now_ms()?,"operation":if action.is_some(){"lookup"}else{"model_disclosure_preparation"},"coverage":"partial_not_provider_receipt_or_comprehension"});
    db.execute("INSERT INTO archive_recall_reads(channel_id,actor_turn_id,action_id,model_request,snapshot) VALUES (?1,?2,?3,?4,?5)",(c.channel_id.to_string(),t.id.to_string(),action.map(|id|id.to_string()),request,serde_json::to_string(&metadata)?))?;
    Ok(())
}
impl Store {
    pub fn archive_recall_grant(&self, channel: ChannelId) -> Result<ArchiveRecallGrant> {
        self.channel_binding(channel)?;
        decision(&self.connection, channel)
    }
    pub fn archive_recall_set(
        &mut self,
        channel: ChannelId,
        q: SetArchiveRecall,
    ) -> Result<ArchiveRecallGrant> {
        self.channel_binding(channel)?;
        let now = now_ms()?;
        if let Some(s) = &q.scope {
            ensure!(
                s.expires_at_ms > now
                    && s.expires_at_ms - now <= 31 * 86400000
                    && (-840..=840).contains(&s.utc_offset_minutes)
                    && (1..=8).contains(&s.sources.len()),
                ControlError::InvalidInput
            );
            let mut ids = std::collections::HashSet::new();
            for r in &s.sources {
                ensure!(ids.insert(r.source_id.0), ControlError::InvalidInput);
                let original = archives::grant(&self.connection, r.source_id)?;
                ensure!(
                    r.grant_revision == original.revision,
                    ControlError::Conflict
                );
                archives::active(&original, now)?;
            }
        }
        let tx = self.connection.transaction()?;
        let old = decision(&tx, channel)?;
        ensure!(q.expected_revision == old.revision, ControlError::Conflict);
        let g = ArchiveRecallGrant {
            channel_id: channel,
            revision: old
                .revision
                .checked_add(1)
                .filter(|n| *n <= i64::MAX as u64)
                .ok_or(ControlError::Conflict)?,
            scope: q.scope,
            changed_at_ms: now,
        };
        tx.execute(
            "INSERT INTO archive_recall_grants VALUES (?1,?2,?3)",
            (
                channel.to_string(),
                i64::try_from(g.revision)?,
                serde_json::to_string(&g)?,
            ),
        )?;
        tx.commit()?;
        Ok(g)
    }
    pub fn archive_recall_audit(&self, channel: ChannelId, after: u64) -> Result<Value> {
        self.channel_binding(channel)?;
        let mut q=self.connection.prepare("SELECT sequence,actor_turn_id,action_id,model_request,snapshot FROM archive_recall_reads WHERE channel_id=?1 AND sequence>?2 ORDER BY sequence LIMIT 101")?;
        let mut rows=q.query_map((channel.to_string(),i64::try_from(after)?),|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,Option<u32>>(3)?,r.get::<_,String>(4)?)))?.map(|r|{let(n,t,a,m,s)=r?;Ok(json!({"sequence":n,"actor_turn_id":t,"action_id":a,"model_request":m,"metadata":decode::<Value>(s)?}))}).collect::<Result<Vec<_>>>()?;
        let more = rows.len() > 100;
        rows.truncate(100);
        Ok(
            json!({"records":rows,"has_more":more,"next_after":rows.last().map(|v|v["sequence"].clone())}),
        )
    }
    pub fn archive_actor_scope(&self, id: ActorTurnId) -> Result<Value> {
        let t = self.actor_turn(id)?;
        let g = active(&self.connection, &t)?;
        let s = scope(&g, now_ms()?)?;
        let (yesterday, today, tomorrow) =
            crate::reading::day_bounds(now_ms()?, s.utc_offset_minutes);
        let mut sources = vec![];
        for selected in &s.sources {
            if allowed(&self.connection, &g, selected.source_id, now_ms()?).is_ok() {
                // Listing capability exposes Lain IDs only; source details follow a lookup.
                let upstream=self.knowledge_state(selected.source_id)?.map(|s|json!({"last_read":s.last_read,"fresh_until_ms":s.fresh_until_ms,"fresh":s.remote_source_enabled && s.fresh_until_ms>now_ms().unwrap_or(u64::MAX),"remote_source_enabled":s.remote_source_enabled,"coverage":"bounded_snapshot_no_automatic_source_fetch"}));
                sources.push(json!({"source_id":selected.source_id,"grant_revision":selected.grant_revision,"upstream_snapshot":upstream}));
            }
        }
        Ok(
            json!({"grant_revision":g.revision,"sources":sources,"now_utc_ms":now_ms()?,"utc_offset_minutes":s.utc_offset_minutes,"yesterday_from_utc_ms":yesterday,"today_from_utc_ms":today,"tomorrow_from_utc_ms":tomorrow,"time_filter":"source_reported_occurrence_else_kernel_receipt","coverage":"latest_100_candidates_per_lookup_at_most_three_selected_records_per_turn_partial"}),
        )
    }
    pub fn archive_actor_lookup(
        &mut self,
        id: ActorTurnId,
        action: ActionId,
        redactor: &Redactor,
    ) -> Result<Value> {
        let mut t = self.actor_turn(id)?;
        self.private_chat_check(&t)?;
        let g = active(&self.connection, &t)?;
        let a = t
            .actions
            .iter()
            .find(|a| a.id == action && a.state == ActionState::Executing)
            .ok_or(ControlError::Conflict)?;
        let mut records = vec![];
        match &a.tool {
            Tool::ArchiveInspect { record_id } => {
                records.push(visible(&self.connection, &g, *record_id, now_ms()?)?)
            }
            Tool::ArchiveSearch(q) => {
                ensure!(q.valid(), ControlError::InvalidInput);
                let ids = scope(&g, now_ms()?)?
                    .sources
                    .iter()
                    .map(|s| s.source_id)
                    .collect::<Vec<_>>();
                if let Some(source) = q.source_id {
                    allowed(&self.connection, &g, source, now_ms()?)?;
                }
                let mut statement=self.connection.prepare("SELECT id FROM archive_records WHERE source_id IN (SELECT value FROM json_each(?1)) AND (?2 IS NULL OR source_id=?2) AND COALESCE(json_extract(snapshot,'$.occurred_at_ms'),json_extract(snapshot,'$.received_at_ms'))>=?3 AND COALESCE(json_extract(snapshot,'$.occurred_at_ms'),json_extract(snapshot,'$.received_at_ms'))<?4 ORDER BY sequence DESC LIMIT 100")?;
                let candidates = statement
                    .query_map(
                        (
                            serde_json::to_string(&ids)?,
                            q.source_id.map(|s| s.to_string()),
                            i64::try_from(q.from_ms)?,
                            i64::try_from(q.to_ms)?,
                        ),
                        |r| r.get::<_, String>(0),
                    )?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                for id in candidates {
                    let Ok(r) = visible(&self.connection, &g, id.parse()?, now_ms()?) else {
                        continue;
                    };
                    if q.text.as_ref().is_none_or(|text| {
                        r.text
                            .as_ref()
                            .is_some_and(|body| body.to_lowercase().contains(&text.to_lowercase()))
                    }) {
                        records.push(r);
                    }
                    if records.len() == usize::from(q.limit) {
                        break;
                    }
                }
            }
            _ => anyhow::bail!(ControlError::InvalidInput),
        }
        let c = t.private_chat.as_mut().ok_or(ControlError::Unavailable)?;
        c.archive_recall_revision = Some(g.revision);
        let ids = records.iter().map(|r| r.id).collect::<Vec<_>>();
        for r in &records {
            project(&self.connection, r, redactor)?;
            if !c.archive_versions.iter().any(|v| v.record_id == r.id) {
                c.archive_versions.push(version(r));
            }
        }
        ensure!(c.archive_versions.len() <= 3, ControlError::InvalidInput);
        let tx = self.connection.transaction()?;
        audit(&tx, &t, Some(action), None)?;
        Self::save_actor_in_transaction(&tx, &mut t, false)?;
        tx.commit()?;
        Ok(
            json!({"record_ids":ids,"context_role":"untrusted_archive_report","records":"Fresh source evidence follows in the next model request. A missing match does not prove no event occurred.","coverage":"partial","authority":"no_memory_write_collection_contact_or_worker_permission"}),
        )
    }
    pub fn archive_actor_disclosure(
        &mut self,
        id: ActorTurnId,
        request: u32,
        redactor: &Redactor,
    ) -> Result<Value> {
        let mut t = self.actor_turn(id)?;
        ensure!(
            t.model_requests == request && request > 0,
            ControlError::Conflict
        );
        self.private_chat_check(&t)?;
        let g = active(&self.connection, &t)?;
        let c = t.private_chat.as_ref().ok_or(ControlError::Unavailable)?;
        ensure!(
            c.archive_recall_revision == Some(g.revision) && c.archive_versions.len() <= 3,
            ControlError::Conflict
        );
        let tx = self.connection.transaction()?;
        let records = c
            .archive_versions
            .iter()
            .map(|v| {
                let r = visible(&tx, &g, v.record_id, now_ms()?)?;
                project(&tx, &r, redactor)
            })
            .collect::<Result<Vec<_>>>()?;
        ensure!(
            serde_json::to_vec(&records)?.len() <= 8192,
            ControlError::InvalidInput
        );
        audit(&tx, &t, None, Some(request))?;
        Self::save_actor_in_transaction(&tx, &mut t, false)?;
        tx.commit()?;
        Ok(
            json!({"records":records,"role":"untrusted_archived_reports","verification":"not_independently_verified","participation":"archive_observation_only","coverage":"selected_partial_excerpts"}),
        )
    }
}
