//! Selected interpretations retain all disclosed dependencies; no extraction loop.
use super::{Store, archive_recall, archives, channels::decode, memory, private_memory};
use crate::{
    actor::{domain::*, tools::Tool},
    backend::ControlError,
    domain::now_ms,
};
use anyhow::{Result, ensure};
use lain_wire::{ChannelId, archives::*, chat::ChatMemoryVersion, memory::*};
use rusqlite::{Connection, OptionalExtension};
use serde_json::{Value, json};

fn decision(db: &Connection, channel: ChannelId) -> Result<ArchiveMemoryGrant> {
    db.query_row("SELECT snapshot FROM archive_memory_grants WHERE channel_id=?1 ORDER BY revision DESC LIMIT 1",[channel.to_string()],|r|r.get::<_,String>(0))
        .optional()?.map(decode).transpose().map(|g|g.unwrap_or(ArchiveMemoryGrant{channel_id:channel,revision:0,scope:None,used_writes:0,changed_at_ms:0}))
}
fn scope(g: &ArchiveMemoryGrant, now: u64) -> Result<&ArchiveMemoryScope> {
    g.scope
        .as_ref()
        .filter(|s| s.expires_at_ms > now)
        .ok_or(ControlError::Unavailable.into())
}
pub(super) fn selected_current(db: &Connection, t: &ActorTurn) -> Result<bool> {
    let Some(c) = &t.private_chat else {
        return Ok(true);
    };
    let Some(revision) = c.archive_memory_grant_revision else {
        return Ok(true);
    };
    let g = decision(db, c.channel_id)?;
    Ok(g.revision == revision && scope(&g, now_ms()?).is_ok())
}
fn permits(
    db: &Connection,
    g: &ArchiveMemoryGrant,
    v: &ArchiveRecallVersion,
    now: u64,
) -> Result<()> {
    ensure!(
        scope(g, now)?
            .sources
            .iter()
            .any(|s| s.source_id == v.source_id && s.grant_revision == v.source_grant_revision),
        ControlError::Conflict
    );
    let a = archives::grant(db, v.source_id)?;
    ensure!(
        a.revision == v.source_grant_revision,
        ControlError::Conflict
    );
    archives::active(&a, now)?;
    Ok(())
}
pub(super) fn suppressed(
    db: &Connection,
    channel: ChannelId,
    record: lain_wire::ArchiveRecordId,
) -> Result<bool> {
    Ok(db.query_row("SELECT EXISTS(SELECT 1 FROM archive_memory_suppressions WHERE channel_id=?1 AND record_id=?2)",(channel.to_string(),record.to_string()),|r|r.get(0))?)
}
fn input_suppressed(
    db: &Connection,
    channel: ChannelId,
    id: lain_wire::IncomingId,
) -> Result<bool> {
    Ok(db.query_row("SELECT EXISTS(SELECT 1 FROM channel_memory_suppressions WHERE channel_id=?1 AND incoming_id=?2)",(channel.to_string(),id.to_string()),|r|r.get(0))?)
}
pub(super) fn suppress(db: &Connection, r: &MemoryRecord) -> Result<()> {
    if let MemorySource::ArchiveSelection { evidence: e } = &r.provenance.source {
        for v in &e.records {
            db.execute(
                "INSERT OR IGNORE INTO archive_memory_suppressions VALUES(?1,?2)",
                (e.channel_id.to_string(), v.record_id.to_string()),
            )?;
        }
        for id in &e.incoming_ids {
            db.execute(
                "INSERT OR IGNORE INTO channel_memory_suppressions VALUES(?1,?2)",
                (e.channel_id.to_string(), id.to_string()),
            )?;
        }
    }
    Ok(())
}
pub(super) fn current(db: &Connection, r: &MemoryRecord, now: u64) -> Result<bool> {
    let MemorySource::ArchiveSelection { evidence: e } = &r.provenance.source else {
        return Ok(true);
    };
    let g = decision(db, e.channel_id)?;
    if g.revision != e.grant_revision || scope(&g, now).is_err() || e.expires_at_ms <= now {
        return Ok(false);
    }
    let memory_grant = private_memory::grant(db, e.channel_id)?;
    if memory_grant.revision != e.memory_grant_revision || !memory_grant.active(now) {
        return Ok(false);
    }
    let recall = archive_recall::decision(db, e.channel_id)?;
    if recall.revision != e.recall_revision {
        return Ok(false);
    }
    for v in &e.records {
        if permits(db, &g, v, now).is_err() {
            return Ok(false);
        }
        let Ok(a) = archive_recall::visible(db, &recall, v.record_id, now) else {
            return Ok(false);
        };
        if a.source_id != v.source_id
            || a.grant_revision != v.source_grant_revision
            || a.revision != v.revision
        {
            return Ok(false);
        }
    }
    for v in &e.memories {
        // Lineage is flattened at write time. Do not recursively follow a graph.
        let Ok(m) = memory::read_memory_snapshot(db, v.memory_id) else {
            return Ok(false);
        };
        if m.status != MemoryStatus::Active
            || m.revision != v.revision
            || m.draft.scope != r.draft.scope
        {
            return Ok(false);
        }
        match &m.provenance.source {
            MemorySource::ChannelSelection { incoming_id, .. }
                if input_suppressed(db, e.channel_id, *incoming_id)? =>
            {
                return Ok(false);
            }
            MemorySource::ArchiveSelection { evidence: p }
                if p.grant_revision != g.revision || p.expires_at_ms <= now =>
            {
                return Ok(false);
            }
            _ => {}
        }
    }
    for id in &e.incoming_ids {
        if input_suppressed(db, e.channel_id, *id)? {
            return Ok(false);
        }
    }
    Ok(true)
}
impl Store {
    pub fn archive_memory_grant(&self, channel: ChannelId) -> Result<ArchiveMemoryGrant> {
        self.channel_binding(channel)?;
        decision(&self.connection, channel)
    }
    pub fn archive_memory_grants(&self, channel: ChannelId) -> Result<Vec<ArchiveMemoryGrant>> {
        self.channel_binding(channel)?;
        let mut q=self.connection.prepare("SELECT snapshot FROM archive_memory_grants WHERE channel_id=?1 ORDER BY revision DESC LIMIT 100")?;
        q.query_map([channel.to_string()], |r| r.get::<_, String>(0))?
            .map(|r| decode(r?))
            .collect()
    }
    pub fn archive_memory_set(
        &mut self,
        channel: ChannelId,
        q: SetArchiveMemory,
    ) -> Result<ArchiveMemoryGrant> {
        self.channel_binding(channel)?;
        let now = now_ms()?;
        let tx = self.connection.transaction()?;
        let old = decision(&tx, channel)?;
        ensure!(q.expected_revision == old.revision, ControlError::Conflict);
        if let Some(s) = &q.scope {
            ensure!(
                s.expires_at_ms > now
                    && s.expires_at_ms <= now.saturating_add(31 * 86400000)
                    && (1..=31 * 86400000).contains(&s.retention_ms)
                    && (1..=1000).contains(&s.max_writes)
                    && (1..=8).contains(&s.sources.len()),
                ControlError::InvalidInput
            );
            let mut seen = std::collections::BTreeSet::new();
            for v in &s.sources {
                ensure!(seen.insert(v.source_id), ControlError::InvalidInput);
                let a = archives::grant(&tx, v.source_id)?;
                ensure!(
                    a.revision == v.grant_revision
                        && archives::active(&a, now)?.expires_at_ms >= s.expires_at_ms,
                    ControlError::Conflict
                );
            }
        }
        let g = ArchiveMemoryGrant {
            channel_id: channel,
            revision: old
                .revision
                .checked_add(1)
                .filter(|r| *r <= i64::MAX as u64)
                .ok_or(ControlError::Conflict)?,
            scope: q.scope,
            used_writes: 0,
            changed_at_ms: now,
        };
        tx.execute(
            "INSERT INTO archive_memory_grants VALUES (?1,?2,?3)",
            (
                channel.to_string(),
                i64::try_from(g.revision)?,
                serde_json::to_string(&g)?,
            ),
        )?;
        tx.commit()?;
        Ok(g)
    }
    pub fn archive_memory_scope(&mut self, id: ActorTurnId) -> Result<Value> {
        let mut t = self.actor_turn(id)?;
        let (c, _) = private_memory::active(&self.connection, &t)?;
        let g = decision(&self.connection, c.channel_id)?;
        scope(&g, now_ms()?)?;
        let chat = t.private_chat.as_ref().ok_or(ControlError::Unavailable)?;
        ensure!(
            !chat.archive_versions.is_empty()
                && chat
                    .archive_memory_grant_revision
                    .is_none_or(|r| r == g.revision),
            ControlError::Unavailable
        );
        ensure!(
            chat.source_versions.is_empty()
                && chat.viewed_media.is_empty()
                && chat.experience_grant_revision.is_none()
                && !chat.share_versions.iter().any(|s| s.experience.is_some()),
            ControlError::Unavailable
        );
        ensure!(
            g.used_writes < scope(&g, now_ms()?)?.max_writes,
            ControlError::Unavailable
        );
        t.private_chat
            .as_mut()
            .ok_or(ControlError::Conflict)?
            .archive_memory_grant_revision = Some(g.revision);
        self.save_actor_turn(&mut t, false)?;
        Ok(
            json!({"grant_revision":g.revision,"scope":g.scope,"used_writes":g.used_writes,"authority":"inferred_source_interpretation_not_owner_confirmation"}),
        )
    }
    pub fn archive_memory_effect(
        &mut self,
        id: ActorTurnId,
        action: ActionId,
    ) -> Result<MemoryEffect> {
        let t = self.actor_turn(id)?;
        self.private_chat_check(&t)?;
        let tx = self.connection.transaction()?;
        let (c, mg) = private_memory::active(&tx, &t)?;
        let chat = t.private_chat.as_ref().ok_or(ControlError::Unavailable)?;
        ensure!(
            chat.source_versions.is_empty()
                && chat.experience_grant_revision.is_none()
                && !chat.share_versions.iter().any(|s| s.experience.is_some()),
            ControlError::Conflict
        );
        let a = t
            .actions
            .iter()
            .find(|a| a.id == action && a.state == ActionState::Executing)
            .ok_or(ControlError::Conflict)?;
        let Tool::ArchiveRemember(q) = &a.tool else {
            anyhow::bail!(ControlError::InvalidInput)
        };
        a.tool.validate()?;
        if let Some(old) = tx
            .query_row(
                "SELECT snapshot FROM channel_memory_effects WHERE action_id=?1",
                [action.to_string()],
                |r| r.get::<_, String>(0),
            )
            .optional()?
        {
            let effect: MemoryEffect = decode(old)?;
            ensure!(
                effect.actor_turn_id == id && effect.channel_id == c.channel_id,
                ControlError::Conflict
            );
            return Ok(effect);
        }
        ensure!(!tx.query_row("SELECT EXISTS(SELECT 1 FROM channel_memory_effects WHERE json_extract(snapshot,'$.actor_turn_id')=?1 AND json_extract(snapshot,'$.archive_grant_revision') IS NOT NULL)",[id.to_string()],|r|r.get::<_,bool>(0))?,ControlError::Conflict);
        let mut g = decision(&tx, c.channel_id)?;
        ensure!(
            chat.archive_memory_grant_revision == Some(g.revision),
            ControlError::Conflict
        );
        let disclosure:serde_json::Value=decode(tx.query_row("SELECT snapshot FROM archive_recall_reads WHERE actor_turn_id=?1 AND model_request=?2 ORDER BY sequence DESC LIMIT 1",(id.to_string(),t.model_requests),|r|r.get::<_,String>(0)).optional()?.ok_or(ControlError::Conflict)?)?;
        ensure!(
            disclosure["versions"] == serde_json::to_value(&chat.archive_versions)?
                && disclosure["grant_revision"]
                    == serde_json::to_value(chat.archive_recall_revision)?,
            ControlError::Conflict
        );
        let s = scope(&g, now_ms()?)?.clone();
        ensure!(
            g.used_writes < s.max_writes && !chat.archive_versions.is_empty(),
            ControlError::Conflict
        );
        let recall = archive_recall::decision(&tx, c.channel_id)?;
        ensure!(
            chat.archive_recall_revision == Some(recall.revision),
            ControlError::Conflict
        );
        let mut e = ArchiveMemoryEvidence {
            model_request: t.model_requests,
            channel_id: c.channel_id,
            conversation_id: t.conversation_id,
            batch_id: c.batch_id,
            actor_turn_id: id,
            action_id: action,
            grant_revision: g.revision,
            memory_grant_revision: mg.revision,
            recall_revision: recall.revision,
            expires_at_ms: now_ms()?
                .saturating_add(s.retention_ms)
                .min(s.expires_at_ms)
                .min(mg.expires_at_ms.unwrap_or(u64::MAX)),
            records: chat.archive_versions.clone(),
            memories: vec![],
            incoming_ids: vec![],
        };
        let rows = {
            let mut q = tx.prepare(
                "SELECT memory_id,revision FROM channel_memory_inspections WHERE actor_turn_id=?1",
            )?;
            q.query_map([id.to_string()], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
        };
        for (mid, revision) in rows {
            let m = memory::read_memory(&tx, mid.parse()?)?;
            ensure!(
                m.status == MemoryStatus::Active
                    && m.revision == u64::try_from(revision)?
                    && m.draft.scope
                        == (MemoryScope::Channel {
                            channel_id: c.channel_id
                        }),
                ControlError::Conflict
            );
            e.memories.push(ChatMemoryVersion {
                memory_id: m.id,
                revision: m.revision,
            });
            match &m.provenance.source {
                MemorySource::ArchiveSelection { evidence: p } => {
                    e.records.extend(p.records.clone());
                    e.memories.extend(p.memories.clone());
                    e.incoming_ids.extend(&p.incoming_ids);
                    e.expires_at_ms = e.expires_at_ms.min(p.expires_at_ms);
                }
                MemorySource::ChannelSelection { incoming_id, .. } => {
                    e.incoming_ids.push(*incoming_id)
                }
                _ => {}
            }
        }
        let mut q_inputs=tx.prepare("SELECT id FROM channel_incoming WHERE channel_id=?1 AND json_extract(snapshot,'$.batch_id')=?2 AND sequence>?3")?;
        let current_inputs = q_inputs
            .query_map(
                (
                    c.channel_id.to_string(),
                    c.batch_id.to_string(),
                    i64::try_from(mg.source_after)?,
                ),
                |r| r.get::<_, String>(0),
            )?
            .map(|r| Ok(r?.parse::<lain_wire::IncomingId>()?))
            .collect::<Result<Vec<_>>>()?;
        ensure!(!current_inputs.is_empty(), ControlError::Conflict);
        e.incoming_ids.extend(current_inputs);
        drop(q_inputs);
        e.records.sort_by_key(|v| v.record_id);
        e.records.dedup();
        e.memories.sort_by_key(|v| v.memory_id);
        e.memories.dedup();
        e.incoming_ids.sort();
        e.incoming_ids.dedup();
        ensure!(
            e.records.len() <= 16
                && e.memories.len() <= 16
                && (1..=16).contains(&e.incoming_ids.len()),
            ControlError::InvalidInput
        );
        for v in &e.records {
            permits(&tx, &g, v, now_ms()?)?;
            let r = archive_recall::visible(&tx, &recall, v.record_id, now_ms()?)?;
            ensure!(
                r.revision == v.revision
                    && r.source_id == v.source_id
                    && r.grant_revision == v.source_grant_revision,
                ControlError::Conflict
            );
            e.expires_at_ms = e.expires_at_ms.min(r.retain_until_ms);
        }
        e.expires_at_ms = e.expires_at_ms.min(
            recall
                .scope
                .as_ref()
                .ok_or(ControlError::Conflict)?
                .expires_at_ms,
        );
        for incoming in &e.incoming_ids {
            ensure!(
                !input_suppressed(&tx, c.channel_id, *incoming)?,
                ControlError::Conflict
            );
        }
        let m = memory::insert_memory(
            &tx,
            MemoryDraft {
                content: q.content.clone(),
                kind: q.kind,
                scope: MemoryScope::Channel {
                    channel_id: c.channel_id,
                },
            },
            MemoryProvenance {
                authority: MemoryAuthority::Inferred,
                source: MemorySource::ArchiveSelection { evidence: e },
            },
        )?;
        ensure!(
            serde_json::to_vec(&vec![&m])?.len() <= 4096,
            ControlError::InvalidInput
        );
        let effect = MemoryEffect {
            action_id: action,
            actor_turn_id: id,
            channel_id: c.channel_id,
            grant_revision: mg.revision,
            archive_grant_revision: Some(g.revision),
            operation: MemoryOperation::Remember,
            memory_id: m.id,
            previous_memory_id: None,
            recorded_at_ms: now_ms()?,
        };
        g.used_writes += 1;
        tx.execute(
            "UPDATE archive_memory_grants SET snapshot=?3 WHERE channel_id=?1 AND revision=?2",
            (
                g.channel_id.to_string(),
                i64::try_from(g.revision)?,
                serde_json::to_string(&g)?,
            ),
        )?;
        tx.execute(
            "INSERT INTO channel_memory_effects(action_id,channel_id,snapshot) VALUES(?1,?2,?3)",
            (
                action.to_string(),
                c.channel_id.to_string(),
                serde_json::to_string(&effect)?,
            ),
        )?;
        tx.commit()?;
        Ok(effect)
    }
}
