//! Read projections over existing channel records; no duplicate source bodies.
use super::Store;
use crate::{
    actor::{
        domain::{ActionState, ActorTurn, MessageRole, TurnState},
        tools::Tool,
    },
    backend::ControlError,
    channels::domain::{Batch, Binding, Delivery, Incoming},
    domain::now_ms,
    redaction::Redactor,
};
use anyhow::{Result, ensure};
use lain_wire::{ActionId, ActorTurnId, BatchId, ChannelId, IncomingId, source::*};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

fn encode(value: &impl Serialize) -> Result<String> {
    Ok(serde_json::to_string(value)?)
}

#[cfg(test)]
#[path = "sources_tests.rs"]
mod tests;
fn decode<T: DeserializeOwned>(value: String) -> Result<T> {
    serde_json::from_str(&value).map_err(|_| anyhow::anyhow!("invalid source storage record"))
}
fn safe(value: &impl Serialize, redactor: &Redactor) -> bool {
    crate::profiles::check_sensitive(value).is_ok()
        && encode(value).is_ok_and(|s| redactor.for_runtime(None).text(&s) == s)
}
fn excerpt(text: &str, limit: usize) -> SourceExcerpt {
    let mut value = text.to_owned();
    while value.len() > limit || serde_json::to_string(&value).unwrap().len() > limit + 2 {
        value.pop();
    }
    SourceExcerpt {
        truncated: value.len() < text.len(),
        original_bytes: text.len(),
        text: value,
    }
}
fn binding(db: &Connection, id: ChannelId) -> Result<Binding> {
    let b: Binding = decode(
        db.query_row(
            "SELECT snapshot FROM channel_bindings WHERE id=?1",
            [id.to_string()],
            |r| r.get(0),
        )
        .optional()?
        .ok_or(ControlError::NotFound)?,
    )?;
    ensure!(b.id == id, "invalid source binding identity");
    Ok(b)
}
fn grant(db: &Connection, id: ChannelId) -> Result<SourceGrant> {
    binding(db, id)?;
    let value=db.query_row("SELECT snapshot FROM channel_source_grants WHERE channel_id=?1 ORDER BY revision DESC LIMIT 1",[id.to_string()],|r|r.get::<_,String>(0)).optional()?;
    let g = match value {
        Some(s) => decode(s)?,
        None => SourceGrant {
            channel_id: id,
            enabled: false,
            revision: 0,
            window: None,
            through_sequence: 0,
            expires_at_ms: None,
            changed_at_ms: 0,
        },
    };
    ensure!(
        g.channel_id == id
            && g.through_sequence <= i64::MAX as u64
            && (!g.enabled || (g.revision > 0 && g.window.is_some_and(|w| w.valid()))),
        "invalid source grant"
    );
    Ok(g)
}
fn batch(db: &Connection, id: BatchId) -> Result<Batch> {
    let b: Batch = decode(
        db.query_row(
            "SELECT snapshot FROM channel_batches WHERE id=?1",
            [id.to_string()],
            |r| r.get(0),
        )
        .optional()?
        .ok_or(ControlError::NotFound)?,
    )?;
    ensure!(b.id == id, "invalid source batch identity");
    Ok(b)
}
fn active(db: &Connection, t: &ActorTurn) -> Result<SourceGrant> {
    let c = t.channel_source.as_ref().ok_or(ControlError::Conflict)?;
    let g = grant(db, c.channel_id)?;
    let b = batch(db, c.batch_id)?;
    let route = binding(db, c.channel_id)?;
    ensure!(
        t.state == TurnState::Running
            && g.active(now_ms()?)
            && c.grant_revision == g.revision
            && now_ms()? < c.expires_at_ms
            && b.id.0 == t.request_id
            && b.channel_id == c.channel_id
            && b.conversation_id == t.conversation_id
            && route.conversation_id == t.conversation_id
            && route.reply_enabled
            && route.grant_revision == b.grant_revision,
        ControlError::Conflict
    );
    Ok(g)
}
fn evidence(g: &SourceGrant, selection: &SourceSelection, records: &[SourceRecord]) -> Value {
    json!({"context_role":"untrusted_source_evidence","coverage":SourceCoverage::PartialNoBackfill,
        "time_semantics":SourceTimeSemantics::ReceivedUtcUnixMs,"channel_id":g.channel_id,
        "grant_revision":g.revision,"window":selection.window,"records":records})
}
/// Re-project the exact request selection at commit/send boundaries, including suppression.
pub(super) fn selected_current(db: &Connection, turn: &ActorTurn) -> Result<bool> {
    let Some(chat) = &turn.private_chat else {
        return Ok(true);
    };
    if chat.source_versions.is_empty() {
        return Ok(true);
    }
    let Some(scope) = &turn.channel_source else {
        return Ok(false);
    };
    let g = grant(db, chat.channel_id)?;
    if scope.channel_id != chat.channel_id
        || scope.batch_id != chat.batch_id
        || !g.active(now_ms()?)
        || g.revision != scope.grant_revision
        || now_ms()? >= scope.expires_at_ms
    {
        return Ok(false);
    }
    let redactor = Redactor::new(vec![]).for_runtime(None);
    for version in &chat.source_versions {
        let selection = &version.selection;
        if selection.channel_id != g.channel_id
            || selection.grant_revision != g.revision
            || selection.through_sequence != g.through_sequence
            || !selection
                .window
                .within(g.window.ok_or(ControlError::Conflict)?)
            || selection.incoming_ids.len() > 3
        {
            return Ok(false);
        }
        let mut records = Vec::new();
        for id in &selection.incoming_ids {
            let Some(record) = project(
                db,
                &g,
                selection.window,
                selection.through_sequence,
                *id,
                &redactor,
            )?
            else {
                return Ok(false);
            };
            records.push(record);
        }
        let fingerprint = lain_wire::actor_configuration_revision(
            encode(&evidence(&g, selection, &records))?.as_bytes(),
        );
        if fingerprint != version.fingerprint {
            return Ok(false);
        }
    }
    Ok(true)
}
fn suppressed(db: &Connection, id: ChannelId, input: IncomingId) -> Result<bool> {
    Ok(db
        .query_row(
            "SELECT 1 FROM channel_memory_suppressions WHERE channel_id=?1 AND incoming_id=?2",
            params![id.to_string(), input.to_string()],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}
fn eligible(
    db: &Connection,
    g: &SourceGrant,
    window: SourceWindow,
    through: u64,
    id: IncomingId,
    redactor: &Redactor,
) -> Result<Option<Incoming>> {
    let row = db
        .query_row(
            "SELECT sequence,snapshot FROM channel_incoming WHERE channel_id=?1 AND id=?2",
            params![g.channel_id.to_string(), id.to_string()],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
        )
        .optional()?;
    let Some((sequence, snapshot)) = row else {
        return Ok(None);
    };
    let input: Incoming = decode(snapshot)?;
    ensure!(
        input.id == id && input.channel_id == g.channel_id,
        "invalid source input identity"
    );
    if u64::try_from(sequence)? > through
        || !window.contains(input.received_at_ms)
        || suppressed(db, g.channel_id, id)?
        || !safe(&input, redactor)
    {
        return Ok(None);
    }
    Ok(Some(input))
}
fn project(
    db: &Connection,
    g: &SourceGrant,
    window: SourceWindow,
    through: u64,
    id: IncomingId,
    redactor: &Redactor,
) -> Result<Option<SourceRecord>> {
    let Some(input) = eligible(db, g, window, through, id, redactor)? else {
        return Ok(None);
    };
    let route = binding(db, g.channel_id)?;
    let mut record = SourceRecord {
        incoming_id: id,
        channel_id: g.channel_id,
        conversation_id: route.conversation_id,
        batch_id: input.batch_id,
        received_at_ms: input.received_at_ms,
        input_state: input.state,
        batch_state: None,
        incoming: excerpt(&input.input.text, 1024),
        reply: SourceReply {
            actor_turn_id: None,
            actor_state: None,
            model_request_attempts: None,
            generated: None,
            delivery: None,
            visibility: ReplyVisibility::Missing,
            delivery_id: None,
            delivery_state: None,
            platform_receipt: None,
        },
    };
    let Some(batch_id) = input.batch_id else {
        return Ok(Some(record));
    };
    let b = batch(db, batch_id)?;
    ensure!(
        b.channel_id == g.channel_id && b.conversation_id == route.conversation_id,
        "invalid source batch route"
    );
    record.batch_state = Some(b.state);
    let delivery: Option<Delivery> = db
        .query_row(
            "SELECT snapshot FROM channel_deliveries WHERE batch_id=?1",
            [batch_id.to_string()],
            |r| r.get::<_, String>(0),
        )
        .optional()?
        .map(decode)
        .transpose()?;
    if let Some(d) = &delivery {
        ensure!(
            d.batch_id == batch_id && d.channel_id == g.channel_id,
            "invalid source delivery route"
        );
        record.reply.delivery_id = Some(d.id);
        record.reply.delivery_state = Some(d.state);
        // Receipt identifiers are independently sanitized before disclosure.
        record.reply.platform_receipt = d.receipt.as_ref().filter(|s| safe(s, redactor)).cloned();
    }
    let Some(turn_id) = b.actor_turn_id else {
        return Ok(Some(record));
    };
    let turn: ActorTurn = decode(db.query_row(
        "SELECT snapshot FROM lain_actor_turns WHERE id=?1",
        [turn_id.to_string()],
        |r| r.get(0),
    )?)?;
    turn.validate()?;
    ensure!(
        turn.id == turn_id
            && turn.request_id == batch_id.0
            && turn.conversation_id == route.conversation_id,
        "invalid source Actor route"
    );
    record.reply.actor_turn_id = Some(turn_id);
    record.reply.actor_state = Some(serde_json::from_value(json!(turn.state))?);
    record.reply.model_request_attempts = Some(turn.model_requests);
    if turn.actions.iter().any(|a| a.tool.source_read()) {
        record.reply.visibility = ReplyVisibility::SourceDerived;
        return Ok(Some(record));
    }
    let changed:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM channel_memory_inspections i LEFT JOIN lain_memories m ON m.id=i.memory_id WHERE i.actor_turn_id=?1 AND (m.id IS NULL OR m.status!=?2 OR CAST(json_extract(m.snapshot,'$.revision') AS INTEGER)!=i.revision))",
        params![turn_id.to_string(),encode(&crate::memory::MemoryStatus::Active)?],|r|r.get(0))?;
    if changed {
        record.reply.visibility = ReplyVisibility::MemoryChanged;
        return Ok(Some(record));
    }
    let siblings: Vec<String> = {
        let mut s=db.prepare("SELECT id FROM channel_incoming WHERE channel_id=?1 AND json_extract(snapshot,'$.batch_id')=?2 LIMIT 17")?;
        s.query_map(
            params![g.channel_id.to_string(), batch_id.to_string()],
            |r| r.get(0),
        )?
        .collect::<rusqlite::Result<_>>()?
    };
    ensure!(
        !siblings.is_empty() && siblings.len() <= 16,
        "invalid source batch input count"
    );
    for sibling in siblings {
        let id = sibling
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid source sibling identity"))?;
        if eligible(db, g, window, through, id, redactor)?.is_none() {
            record.reply.visibility = ReplyVisibility::BatchOutsideWindow;
            return Ok(Some(record));
        }
    }
    let generated = turn
        .messages
        .iter()
        .rev()
        .find(|m| m.role == MessageRole::Assistant);
    if generated.is_some_and(|m| !window.contains(m.timestamp_ms))
        || delivery
            .as_ref()
            .is_some_and(|d| !window.contains(d.created_at_ms))
    {
        record.reply.visibility = ReplyVisibility::OutsideWindow;
        return Ok(Some(record));
    }
    if generated.is_some_and(|m| !safe(&m.content, redactor))
        || delivery.as_ref().is_some_and(|d| !safe(&d.text, redactor))
    {
        record.reply.visibility = ReplyVisibility::Sensitive;
        return Ok(Some(record));
    }
    record.reply.generated = generated.map(|m| excerpt(&m.content, 512));
    record.reply.delivery = delivery.as_ref().map(|d| excerpt(&d.text, 512));
    record.reply.visibility = if generated.is_some() || delivery.is_some() {
        ReplyVisibility::Visible
    } else {
        ReplyVisibility::Missing
    };
    Ok(Some(record))
}
fn window(g: &SourceGrant, w: Option<SourceWindow>) -> Result<SourceWindow> {
    let outer = g.window.ok_or(ControlError::Conflict)?;
    let w = w.unwrap_or(outer);
    ensure!(w.within(outer), ControlError::InvalidInput);
    Ok(w)
}
// Receipt-index seek avoids an unbounded sort before the 512-candidate projection budget.
const SOURCE_PAGE_SQL: &str = "SELECT CAST(json_extract(snapshot,'$.received_at_ms') AS INTEGER),sequence,id FROM channel_incoming INDEXED BY channel_source_window WHERE channel_id=?1 AND (CAST(json_extract(snapshot,'$.received_at_ms') AS INTEGER),sequence)>(?2,?3) AND CAST(json_extract(snapshot,'$.received_at_ms') AS INTEGER)<?4 ORDER BY CAST(json_extract(snapshot,'$.received_at_ms') AS INTEGER),sequence LIMIT ?5";

fn query(
    db: &Connection,
    g: &SourceGrant,
    q: &SourceQuery,
    redactor: &Redactor,
) -> Result<(SourcePage, SourceWindow)> {
    ensure!(g.active(now_ms()?), ControlError::Conflict);
    ensure!((1..=10).contains(&q.limit), ControlError::InvalidInput);
    ensure!(
        q.text.as_ref().is_none_or(|s| !s.trim().is_empty()
            && s.len() <= 256
            && !s.chars().any(char::is_control)),
        ControlError::InvalidInput
    );
    ensure!(safe(q, redactor), ControlError::InvalidInput);
    let w = window(g, q.window)?;
    let digest = lain_wire::actor_configuration_revision(
        encode(&(g.channel_id, g.revision, w, &q.text))?.as_bytes(),
    );
    let mut cursor = q.after.clone().unwrap_or(SourceCursor {
        channel_id: g.channel_id,
        grant_revision: g.revision,
        through_sequence: g.through_sequence,
        after_sequence: 0,
        after_received_at_ms: w.from_received_at_ms,
        query_digest: digest.clone(),
    });
    ensure!(
        cursor.channel_id == g.channel_id
            && cursor.grant_revision == g.revision
            && cursor.query_digest == digest
            && cursor.through_sequence == g.through_sequence
            && cursor.after_sequence <= i64::MAX as u64
            && w.contains(cursor.after_received_at_ms),
        ControlError::Conflict
    );
    let mut statement = db.prepare(SOURCE_PAGE_SQL)?;
    let rows = statement
        .query_map(
            params![
                g.channel_id.to_string(),
                cursor.after_received_at_ms as i64,
                cursor.after_sequence as i64,
                w.to_received_at_ms as i64,
                (MAX_SCAN + 1) as i64
            ],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, String>(2)?,
                ))
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let needle = q.text.as_ref().map(|s| s.to_lowercase());
    let mut records = Vec::new();
    let mut examined = 0u16;
    for (received, sequence, id) in rows.iter().take(MAX_SCAN) {
        let id = id
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid source input id"))?;
        // Search full sanitized input, not just the clipped display excerpt.
        let input = eligible(db, g, w, cursor.through_sequence, id, redactor)?;
        let selected = if let Some(input) = input {
            if needle
                .as_ref()
                .is_none_or(|s| input.input.text.to_lowercase().contains(s))
            {
                project(db, g, w, cursor.through_sequence, id, redactor)?
            } else {
                None
            }
        } else {
            None
        };
        if let Some(record) = selected {
            records.push(record);
            if serde_json::to_vec(&records)?.len() > MAX_PAGE_BYTES {
                records.pop();
                break;
            }
        }
        examined += 1;
        cursor.after_sequence = (*sequence).try_into()?;
        cursor.after_received_at_ms = (*received).try_into()?;
        if records.len() >= q.limit as usize {
            break;
        }
    }
    let more = rows.len() > examined as usize;
    Ok((
        SourcePage {
            records,
            next_cursor: cursor,
            has_more: more,
            scan_limit_reached: examined as usize == MAX_SCAN && more,
            examined,
            coverage: SourceCoverage::PartialNoBackfill,
            time_semantics: SourceTimeSemantics::ReceivedUtcUnixMs,
        },
        w,
    ))
}
fn audit(db: &Connection, record: &SourceAudit) -> Result<()> {
    db.execute("INSERT OR IGNORE INTO channel_source_reads(channel_id,actor_turn_id,action_id,model_request,operation,snapshot) VALUES(?1,?2,?3,?4,?5,?6)",
        params![record.channel_id.to_string(),record.actor_turn_id.map(|i|i.to_string()),record.action_id.map(|i|i.to_string()),record.model_request,encode(&record.operation)?,encode(record)?])?;
    Ok(())
}
fn lookup(
    db: &Connection,
    g: &SourceGrant,
    tool: &Tool,
    redactor: &Redactor,
) -> Result<(SourceSelection, Value, SourceOperation, Option<String>)> {
    ensure!(g.active(now_ms()?), ControlError::Conflict);
    let (records, w, through, receipt, op, digest) = match tool {
        Tool::SourceSearch(q) => {
            let (page, w) = query(db, g, q, redactor)?;
            let receipt = json!({"incoming_ids":page.records.iter().map(|r|r.incoming_id).collect::<Vec<_>>(),"next_cursor":page.next_cursor,"has_more":page.has_more,"scan_limit_reached":page.scan_limit_reached,"examined":page.examined,"coverage":page.coverage,"time_semantics":page.time_semantics});
            (
                page.records,
                w,
                page.next_cursor.through_sequence,
                receipt,
                SourceOperation::Search,
                Some(page.next_cursor.query_digest),
            )
        }
        Tool::SourceInspect(q) => {
            let w = window(g, q.window)?;
            let record = project(db, g, w, g.through_sequence, q.incoming_id, redactor)?
                .ok_or(ControlError::NotFound)?;
            (
                vec![record],
                w,
                g.through_sequence,
                json!({"incoming_ids":[q.incoming_id],"coverage":SourceCoverage::PartialNoBackfill,"time_semantics":SourceTimeSemantics::ReceivedUtcUnixMs}),
                SourceOperation::Inspect,
                None,
            )
        }
        _ => return Err(ControlError::InvalidInput.into()),
    };
    let selection = SourceSelection {
        channel_id: g.channel_id,
        grant_revision: g.revision,
        window: w,
        through_sequence: through,
        incoming_ids: records.iter().map(|r| r.incoming_id).collect(),
    };
    Ok((selection, receipt, op, digest))
}
impl Store {
    pub fn source_grant(&self, id: ChannelId) -> Result<SourceGrant> {
        grant(&self.connection, id)
    }
    pub fn set_source_grant(
        &mut self,
        id: ChannelId,
        change: SetSourceGrant,
    ) -> Result<SourceGrant> {
        binding(&self.connection, id)?;
        let now = now_ms()?;
        ensure!(
            change
                .expires_at_ms
                .is_none_or(|t| t > now && t <= i64::MAX as u64),
            ControlError::InvalidInput
        );
        ensure!(
            if change.enabled {
                change
                    .window
                    .is_some_and(|w| w.valid() && w.to_received_at_ms <= now)
            } else {
                change.window.is_none() && change.expires_at_ms.is_none()
            },
            ControlError::InvalidInput
        );
        let tx = self.connection.transaction()?;
        let old = grant(&tx, id)?;
        ensure!(
            old.revision == change.expected_revision,
            ControlError::Conflict
        );
        let next = SourceGrant {
            channel_id: id,
            enabled: change.enabled,
            revision: old
                .revision
                .checked_add(1)
                .filter(|n| *n <= i64::MAX as u64)
                .ok_or(ControlError::Conflict)?,
            window: change.window.or(old.window),
            through_sequence: tx
                .query_row(
                    "SELECT COALESCE(MAX(sequence),0) FROM channel_incoming WHERE channel_id=?1",
                    [id.to_string()],
                    |r| r.get::<_, i64>(0),
                )?
                .try_into()?,
            changed_at_ms: now.max(old.changed_at_ms),
            expires_at_ms: change.expires_at_ms,
        };
        tx.execute(
            "INSERT INTO channel_source_grants VALUES(?1,?2,?3)",
            params![id.to_string(), next.revision as i64, encode(&next)?],
        )?;
        tx.commit()?;
        Ok(next)
    }
    pub fn source_grants(&self, id: ChannelId) -> Result<Vec<SourceGrant>> {
        binding(&self.connection, id)?;
        let mut s=self.connection.prepare("SELECT snapshot FROM channel_source_grants WHERE channel_id=?1 ORDER BY revision DESC LIMIT 100")?;
        s.query_map([id.to_string()], |r| r.get::<_, String>(0))?
            .map(|s| decode(s?))
            .collect()
    }
    pub fn source_context(
        &self,
        id: ChannelId,
        batch_id: BatchId,
    ) -> Result<Option<SourceReadContext>> {
        let g = grant(&self.connection, id)?;
        let b = batch(&self.connection, batch_id)?;
        ensure!(b.channel_id == id, ControlError::InvalidInput);
        let expires = b.expires_at_ms.min(g.expires_at_ms.unwrap_or(u64::MAX));
        Ok(
            (g.active(now_ms()?) && now_ms()? < expires).then_some(SourceReadContext {
                channel_id: id,
                batch_id,
                grant_revision: g.revision,
                expires_at_ms: expires,
            }),
        )
    }
    pub fn source_actor_scope(&self, id: ActorTurnId) -> Result<SourceGrant> {
        active(&self.connection, &self.actor_turn(id)?)
    }
    pub fn source_search(
        &mut self,
        id: ChannelId,
        q: SourceQuery,
        redactor: &Redactor,
    ) -> Result<SourcePage> {
        let tx = self.connection.transaction()?;
        let g = grant(&tx, id)?;
        let (page, w) = query(&tx, &g, &q, redactor)?;
        audit(
            &tx,
            &SourceAudit {
                channel_id: id,
                actor_turn_id: None,
                action_id: None,
                model_request: None,
                operation: SourceOperation::Search,
                grant_revision: g.revision,
                incoming_ids: page.records.iter().map(|r| r.incoming_id).collect(),
                window: w,
                query_digest: Some(page.next_cursor.query_digest.clone()),
                recorded_at_ms: now_ms()?,
            },
        )?;
        tx.commit()?;
        Ok(page)
    }
    pub fn source_inspect(
        &mut self,
        id: ChannelId,
        q: InspectSource,
        redactor: &Redactor,
    ) -> Result<SourceRecord> {
        let tx = self.connection.transaction()?;
        let g = grant(&tx, id)?;
        ensure!(g.active(now_ms()?), ControlError::Conflict);
        let w = window(&g, q.window)?;
        let record = project(&tx, &g, w, g.through_sequence, q.incoming_id, redactor)?
            .ok_or(ControlError::NotFound)?;
        audit(
            &tx,
            &SourceAudit {
                channel_id: id,
                actor_turn_id: None,
                action_id: None,
                model_request: None,
                operation: SourceOperation::Inspect,
                grant_revision: g.revision,
                incoming_ids: vec![q.incoming_id],
                window: w,
                query_digest: None,
                recorded_at_ms: now_ms()?,
            },
        )?;
        tx.commit()?;
        Ok(record)
    }
    pub fn source_actor_lookup(
        &mut self,
        id: ActorTurnId,
        action: ActionId,
        redactor: &Redactor,
    ) -> Result<(SourceSelection, Value)> {
        let turn = self.actor_turn(id)?;
        let tx = self.connection.transaction()?;
        let g = active(&tx, &turn)?;
        let action = turn
            .actions
            .iter()
            .find(|a| a.id == action && a.state == ActionState::Executing)
            .ok_or(ControlError::Conflict)?;
        action.tool.validate()?;
        let (selection, mut receipt, operation, digest) = lookup(&tx, &g, &action.tool, redactor)?;
        audit(
            &tx,
            &SourceAudit {
                channel_id: g.channel_id,
                actor_turn_id: Some(id),
                action_id: Some(action.id),
                model_request: None,
                operation,
                grant_revision: g.revision,
                incoming_ids: selection.incoming_ids.clone(),
                window: selection.window,
                query_digest: digest,
                recorded_at_ms: now_ms()?,
            },
        )?;
        receipt["context_role"] = json!("untrusted_source_evidence");
        receipt["records"] = json!(
            "Fresh authorized source records are supplied in the next model request, not cached in this receipt."
        );
        tx.commit()?;
        Ok((selection, receipt))
    }
    pub fn source_actor_disclosure(
        &mut self,
        id: ActorTurnId,
        selections: &[SourceSelection],
        model_request: Option<u32>,
        redactor: &Redactor,
    ) -> Result<(Vec<String>, Value)> {
        let mut turn = self.actor_turn(id)?;
        let tx = self.connection.transaction()?;
        let g = active(&tx, &turn)?;
        ensure!(
            !selections.is_empty() && selections.len() <= 4,
            ControlError::Conflict
        );
        let mut fingerprints = Vec::new();
        let mut all_ids = Vec::new();
        let mut last = Value::Null;
        let mut window = selections[0].window;
        for selection in selections {
            ensure!(
                selection.channel_id == g.channel_id
                    && selection.grant_revision == g.revision
                    && selection
                        .window
                        .within(g.window.ok_or(ControlError::Conflict)?)
                    && selection.through_sequence == g.through_sequence
                    && selection.incoming_ids.len() <= 3,
                ControlError::Conflict
            );
            let mut records = Vec::new();
            for incoming_id in &selection.incoming_ids {
                records.push(
                    project(
                        &tx,
                        &g,
                        selection.window,
                        selection.through_sequence,
                        *incoming_id,
                        redactor,
                    )?
                    .ok_or(ControlError::Conflict)?,
                );
                if !all_ids.contains(incoming_id) {
                    all_ids.push(*incoming_id);
                }
            }
            ensure!(
                serde_json::to_vec(&records)?.len() <= MAX_PAGE_BYTES,
                ControlError::Conflict
            );
            last = evidence(&g, selection, &records);
            fingerprints.push(lain_wire::actor_configuration_revision(
                encode(&last)?.as_bytes(),
            ));
            window.from_received_at_ms = window
                .from_received_at_ms
                .min(selection.window.from_received_at_ms);
            window.to_received_at_ms = window
                .to_received_at_ms
                .max(selection.window.to_received_at_ms);
        }
        if let Some(request) = model_request {
            ensure!(
                turn.model_requests == request && request > 0,
                ControlError::Conflict
            );
            audit(
                &tx,
                &SourceAudit {
                    channel_id: g.channel_id,
                    actor_turn_id: Some(id),
                    action_id: None,
                    model_request: Some(request),
                    operation: SourceOperation::PreparedForModelRequest,
                    grant_revision: g.revision,
                    incoming_ids: all_ids,
                    window,
                    query_digest: None,
                    recorded_at_ms: now_ms()?,
                },
            )?;
            if let Some(chat) = &mut turn.private_chat {
                chat.source_versions = selections
                    .iter()
                    .zip(&fingerprints)
                    .map(
                        |(selection, fingerprint)| lain_wire::chat::ChatSourceVersion {
                            selection: selection.clone(),
                            fingerprint: fingerprint.clone(),
                        },
                    )
                    .collect();
                Self::save_actor_in_transaction(&tx, &mut turn, false)?;
            }
        }
        tx.commit()?;
        Ok((fingerprints, last))
    }
    pub fn source_reads(&self, id: ChannelId, after: u64) -> Result<Value> {
        binding(&self.connection, id)?;
        ensure!(after <= i64::MAX as u64, ControlError::InvalidInput);
        let mut s=self.connection.prepare("SELECT sequence,snapshot FROM channel_source_reads WHERE channel_id=?1 AND sequence>?2 ORDER BY sequence LIMIT 51")?;
        let rows = s
            .query_map(params![id.to_string(), after as i64], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let more = rows.len() > 50;
        let mut cursor = after;
        let mut reads = Vec::<SourceAudit>::new();
        for (n, value) in rows.into_iter().take(50) {
            cursor = n.try_into()?;
            reads.push(decode(value)?)
        }
        Ok(json!({"reads":reads,"next_cursor":cursor,"has_more":more}))
    }
}
