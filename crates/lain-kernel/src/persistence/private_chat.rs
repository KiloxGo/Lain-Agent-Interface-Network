//! Bounded private input adoption and conversational decisions. No new permissions.
use super::{
    Store,
    channels::{binding, decode, save_input},
};
use crate::{
    actor::{
        domain::{ActionState, ActorTurn, MessageRole, TurnState},
        tools::Tool,
    },
    backend::ControlError,
    channels::domain::*,
    domain::now_ms,
};
use anyhow::{Result, ensure};
use lain_wire::{ActionId, ActorTurnId, chat::*};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

#[derive(Debug, thiserror::Error)]
#[error("new private input requires reconsideration")]
pub(crate) struct NewPrivateInput;

#[derive(Debug, thiserror::Error)]
#[error("private memory or source evidence changed")]
pub(crate) struct ChangedChatMemory;

#[derive(Debug)]
pub(crate) enum ChatRefresh {
    Current,
    WaitUntil(u64),
    Updated,
    Superseded,
}

fn batch(db: &Connection, id: BatchId) -> Result<Batch> {
    super::channels::one(
        db,
        "SELECT snapshot FROM channel_batches WHERE id=?1",
        id.to_string(),
    )
}
fn inputs(db: &Connection, id: ChannelId, batch: BatchId) -> Result<Vec<(u64, Incoming)>> {
    let mut s=db.prepare("SELECT sequence,snapshot FROM channel_incoming WHERE channel_id=?1 AND json_extract(snapshot,'$.batch_id')=?2 ORDER BY sequence LIMIT 17")?;
    let rows = s
        .query_map(params![id.to_string(), batch.to_string()], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter()
        .map(|(n, v)| Ok((n.try_into()?, decode(v)?)))
        .collect()
}
fn queued(db: &Connection, b: &Binding, now: u64) -> Result<Vec<(u64, Incoming)>> {
    let mut s=db.prepare("SELECT sequence,snapshot FROM channel_incoming WHERE channel_id=?1 AND state='queued' ORDER BY sequence LIMIT 128")?;
    let rows = s
        .query_map([b.id.to_string()], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut result = Vec::new();
    for (n, v) in rows {
        let input: Incoming = decode(v)?;
        ensure!(
            input.channel_id == b.id && input.state == InputState::Queued,
            "invalid private input route"
        );
        if input.grant_revision == b.grant_revision
            && now < input.received_at_ms.saturating_add(REPLY_TTL_MS)
        {
            result.push((n.try_into()?, input));
        }
    }
    Ok(result)
}
fn active(db: &Connection, t: &ActorTurn) -> Result<(PrivateChatContext, Binding)> {
    let c = t.private_chat.clone().ok_or(ControlError::Unavailable)?;
    let b = binding(db, c.channel_id)?;
    let batch = batch(db, c.batch_id)?;
    ensure!(
        t.state == TurnState::Running
            && c.batch_id.0 == t.request_id
            && batch.channel_id == c.channel_id
            && b.conversation_id == t.conversation_id
            && batch.conversation_id == t.conversation_id
            && b.reply_enabled
            && b.grant_revision == c.grant_revision
            && batch.grant_revision == c.grant_revision
            && now_ms()? < c.expires_at_ms
            && c.expires_at_ms == batch.expires_at_ms,
        ControlError::Conflict
    );
    Ok((c, b))
}
pub(super) fn latest_input(db: &Connection, id: ChannelId) -> Result<u64> {
    Ok(db
        .query_row(
            "SELECT COALESCE(MAX(sequence),0) FROM channel_incoming WHERE channel_id=?1",
            [id.to_string()],
            |r| r.get::<_, i64>(0),
        )?
        .try_into()?)
}
pub(super) fn check_current(db: &Connection, t: &ActorTurn) -> Result<()> {
    if let Some(c) = &t.private_chat {
        ensure!(
            latest_input(db, c.channel_id)? <= c.input_through_sequence,
            NewPrivateInput
        );
    }
    ensure!(
        memory_current(db, t)?
            && super::sources::selected_current(db, t)?
            && super::experiences::selected_current(db, t)?,
        ChangedChatMemory
    );
    Ok(())
}
pub(super) fn memory_current(db: &Connection, t: &ActorTurn) -> Result<bool> {
    if !super::channel_media::selected_current(db, t)? {
        return Ok(false);
    }
    if !super::archive_recall::selected_current(db, t)?
        || !super::archive_memory::selected_current(db, t)?
        || !super::experiences::selected_current(db, t)?
        || !super::share_recall::selected_current(db, t)?
    {
        return Ok(false);
    }
    let Some(chat) = &t.private_chat else {
        return Ok(true);
    };
    if chat.memory_versions.is_empty() && chat.archive_memory_versions.is_empty() {
        return Ok(true);
    }
    let Some(scope) = &t.channel_memory else {
        return Ok(false);
    };
    if scope.channel_id != chat.channel_id || scope.batch_id != chat.batch_id {
        return Ok(false);
    }
    let grant = super::private_memory::grant(db, chat.channel_id)?;
    if !grant.active(now_ms()?)
        || grant.revision != scope.grant_revision
        || now_ms()? >= scope.expires_at_ms
    {
        return Ok(false);
    }
    for selected in chat
        .memory_versions
        .iter()
        .chain(&chat.archive_memory_versions)
    {
        let Ok(record) = super::memory::read_memory(db, selected.memory_id) else {
            return Ok(false);
        };
        if record.revision != selected.revision
            || record.status != crate::memory::MemoryStatus::Active
            || record.draft.scope
                != (crate::memory::MemoryScope::Channel {
                    channel_id: chat.channel_id,
                })
        {
            return Ok(false);
        }
    }
    Ok(true)
}
pub(super) fn ordinary_delivery(db: &Connection, d: &Delivery) -> Result<bool> {
    if d.purpose != DeliveryPurpose::Conversation {
        return Ok(false);
    }
    let b = batch(db, d.batch_id)?;
    let Some(id) = b.actor_turn_id else {
        return Ok(false);
    };
    let t: ActorTurn = super::channels::one(
        db,
        "SELECT snapshot FROM lain_actor_turns WHERE id=?1",
        id.to_string(),
    )?;
    Ok(t.state == TurnState::Completed
        && t.private_chat
            .as_ref()
            .is_none_or(|c| c.decision == ChatDecision::Reply))
}
pub(super) fn delivery_current(db: &Connection, d: &Delivery) -> Result<bool> {
    if !ordinary_delivery(db, d)? {
        return Ok(true);
    }
    let through = inputs(db, d.channel_id, d.batch_id)?
        .last()
        .map(|(n, _)| *n)
        .unwrap_or(0);
    let b = batch(db, d.batch_id)?;
    let t: ActorTurn = super::channels::one(
        db,
        "SELECT snapshot FROM lain_actor_turns WHERE id=?1",
        b.actor_turn_id.unwrap().to_string(),
    )?;
    Ok(latest_input(db, d.channel_id)? <= through
        && memory_current(db, &t)?
        && super::sources::selected_current(db, &t)?
        && super::memory_context::publication_current(db, &t)?)
}

impl Store {
    pub fn private_chat_check(&self, t: &ActorTurn) -> Result<()> {
        check_current(&self.connection, t)
    }
    pub fn private_chat_memory_current(&self, id: ActorTurnId) -> Result<bool> {
        let t = self.actor_turn(id)?;
        Ok(memory_current(&self.connection, &t)?
            && super::experiences::selected_current(&self.connection, &t)?)
    }
    pub fn private_chat_context(
        &self,
        id: ChannelId,
        batch_id: BatchId,
    ) -> Result<PrivateChatContext> {
        let b = binding(&self.connection, id)?;
        let batch = batch(&self.connection, batch_id)?;
        ensure!(
            batch.channel_id == id && batch.conversation_id == b.conversation_id,
            ControlError::InvalidInput
        );
        let input = inputs(&self.connection, id, batch_id)?;
        ensure!(
            !input.is_empty() && input.len() <= MAX_BATCH,
            "invalid private batch inputs"
        );
        Ok(PrivateChatContext {
            channel_id: id,
            batch_id,
            grant_revision: b.grant_revision,
            expires_at_ms: batch.expires_at_ms,
            input_through_sequence: input.last().unwrap().0,
            reconsiderations: 0,
            waits: 0,
            decision: ChatDecision::Pending,
            wait_until_ms: None,
            memory_versions: vec![],
            archive_memory_versions: vec![],
            archive_memory_tainted: false,
            archive_memory_grant_revision: None,
            source_versions: vec![],
            experience_versions: vec![],
            experience_grant_revision: None,
            archive_recall_revision: None,
            archive_versions: vec![],
            share_versions: vec![],
            media_versions: vec![],
            viewed_media: vec![],
        })
    }
    pub fn private_chat_refresh(&mut self, id: ActorTurnId, now: u64) -> Result<ChatRefresh> {
        self.channel_media_maintain(now)?;
        let mut t = self.actor_turn(id)?;
        if t.private_chat.is_none() {
            return Ok(ChatRefresh::Current);
        }
        let mut c = t.private_chat.clone().unwrap();
        let b = binding(&self.connection, c.channel_id)?;
        let saved_batch = batch(&self.connection, c.batch_id)?;
        ensure!(
            t.state == TurnState::Running
                && c.batch_id.0 == t.request_id
                && saved_batch.channel_id == b.id
                && saved_batch.conversation_id == t.conversation_id
                && b.conversation_id == t.conversation_id,
            ControlError::Conflict
        );
        if !b.reply_enabled || b.grant_revision != c.grant_revision || now >= c.expires_at_ms {
            // A completed response with revoked scope is known discarded, not an uncertain send.
            c.decision = ChatDecision::Superseded;
            c.wait_until_ms = None;
            t.private_chat = Some(c);
            t.state = TurnState::Completed;
            self.save_actor_turn(&mut t, false)?;
            return Ok(ChatRefresh::Superseded);
        }
        let (mut c, b) = active(&self.connection, &t)?;
        let pending = queued(&self.connection, &b, now)?;
        if let Some(due) =
            super::channel_media::pending(&self.connection, pending.iter().map(|(_, i)| i.clone()))?
        {
            return Ok(ChatRefresh::WaitUntil(due));
        }
        if pending.is_empty() {
            if latest_input(&self.connection, b.id)? > c.input_through_sequence {
                // Expired inputs still invalidate an old draft; do not spin on stale context.
                c.decision = ChatDecision::Superseded;
                c.wait_until_ms = None;
                t.private_chat = Some(c);
                t.state = TurnState::Completed;
                self.save_actor_turn(&mut t, false)?;
                return Ok(ChatRefresh::Superseded);
            }
            return Ok(ChatRefresh::Current);
        }
        let old = inputs(&self.connection, b.id, c.batch_id)?;
        let bytes: usize = old
            .iter()
            .map(|(_, i)| i.input.conversation_text().len())
            .sum::<usize>()
            + old.len().saturating_sub(1);
        if t.model_requests >= 4
            || (t.model_requests > 0 && c.reconsiderations >= MAX_RECONSIDERATIONS)
            || old.len() + pending.len() > MAX_BATCH
            || bytes
                + pending
                    .iter()
                    .map(|(_, i)| i.input.conversation_text().len() + 1)
                    .sum::<usize>()
                > MAX_TEXT
        {
            c.decision = ChatDecision::Superseded;
            c.wait_until_ms = None;
            t.private_chat = Some(c);
            t.state = TurnState::Completed;
            self.save_actor_turn(&mut t, false)?;
            return Ok(ChatRefresh::Superseded);
        }
        let due = merge_deadline(pending.iter().map(|(_, i)| i.received_at_ms)).unwrap();
        if now < due {
            return Ok(ChatRefresh::WaitUntil(due));
        }
        let tx = self.connection.transaction()?;
        for (sequence, mut input) in pending {
            t.message(MessageRole::User, input.input.conversation_text())?;
            input.state = InputState::Batched;
            input.batch_id = Some(c.batch_id);
            save_input(&tx, &input)?;
            c.input_through_sequence = sequence;
        }
        if t.model_requests > 0 {
            c.reconsiderations += 1;
        }
        c.decision = ChatDecision::Pending;
        c.wait_until_ms = None;
        c.memory_versions.clear();
        c.archive_memory_versions.clear();
        c.archive_memory_grant_revision = None;
        c.source_versions.clear();
        c.archive_recall_revision = None;
        c.archive_versions.clear();
        c.experience_versions.clear();
        c.experience_grant_revision = None;
        c.share_versions.clear();
        c.media_versions.clear();
        c.viewed_media.clear();
        t.private_chat = Some(c);
        // Actor and input association share the commit; original request text is untouched.
        Self::save_actor_in_transaction(&tx, &mut t, false)?;
        tx.commit()?;
        Ok(ChatRefresh::Updated)
    }
    pub fn private_chat_has_pending(&self, id: ActorTurnId) -> Result<bool> {
        let t = self.actor_turn(id)?;
        let (_, b) = active(&self.connection, &t)?;
        Ok(!queued(&self.connection, &b, now_ms()?)?.is_empty())
    }
    pub fn private_chat_wait(
        &mut self,
        id: ActorTurnId,
        action: ActionId,
        seconds: u8,
    ) -> Result<u64> {
        let mut t = self.actor_turn(id)?;
        let (mut c, _) = active(&self.connection, &t)?;
        check_current(&self.connection, &t)?;
        ensure!(
            c.waits == 0 && (1..=MAX_WAIT_SECONDS).contains(&seconds),
            ControlError::InvalidInput
        );
        ensure!(
            t.actions.iter().any(|a| a.id == action
                && a.state == ActionState::Executing
                && matches!(&a.tool, Tool::ChatWait(q) if q.seconds == seconds)),
            ControlError::Conflict
        );
        let due = now_ms()?
            .saturating_add(u64::from(seconds) * 1000)
            .min(c.expires_at_ms);
        c.waits = 1;
        c.decision = ChatDecision::Waiting;
        c.wait_until_ms = Some(due);
        t.private_chat = Some(c);
        self.save_actor_turn(&mut t, false)?;
        Ok(due)
    }
    pub fn private_chat_wait_done(&mut self, id: ActorTurnId, action: ActionId) -> Result<Value> {
        let mut t = self.actor_turn(id)?;
        let (mut c, b) = active(&self.connection, &t)?;
        ensure!(c.decision == ChatDecision::Waiting, ControlError::Conflict);
        let pending = queued(&self.connection, &b, now_ms()?)?;
        ensure!(
            !pending.is_empty() || now_ms()? >= c.wait_until_ms.unwrap(),
            ControlError::Conflict
        );
        let value = json!({"decision":"wait_completed","wake":if pending.is_empty(){"deadline"}else{"new_input"},"incoming_ids":pending.iter().map(|(_,i)|i.id).collect::<Vec<_>>()});
        let a = t
            .actions
            .iter_mut()
            .find(|a| a.id == action && a.state == ActionState::Executing)
            .ok_or(ControlError::Conflict)?;
        a.state = ActionState::Executed;
        a.result = Some(value.clone());
        c.decision = ChatDecision::Pending;
        c.wait_until_ms = None;
        t.private_chat = Some(c);
        self.save_actor_turn(&mut t, false)?;
        Ok(value)
    }
    pub fn private_chat_finish(
        &mut self,
        id: ActorTurnId,
        action: ActionId,
        args: FinishChat,
    ) -> Result<Value> {
        let mut t = self.actor_turn(id)?;
        let (mut c, _) = active(&self.connection, &t)?;
        check_current(&self.connection, &t)?;
        let input = inputs(&self.connection, c.channel_id, c.batch_id)?;
        let text = input
            .iter()
            .map(|(_, i)| i.input.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        ensure!(
            args.input_kind == ChatInputKind::Social
                && input
                    .iter()
                    .all(|(_, i)| i.input.segments.iter().all(|s| match s {
                        lain_wire::media::InputSegment::Visual { media_id, .. } =>
                            c.viewed_media.iter().any(|v| v.media_id == *media_id),
                        _ => true,
                    }))
                && !reply_required(&text)
                && t.actions.iter().all(|a| matches!(
                    a.tool,
                    Tool::ChatWait(_) | Tool::ChatFinish(_)
                ) && !(matches!(a.tool, Tool::ChatFinish(_))
                    && a.state == ActionState::Failed)),
            ControlError::InvalidInput
        );
        let value = json!({"decision":"no_reply","input_kind":"social","reason":args.reason,"incoming_ids":input.iter().map(|(_,i)|i.id).collect::<Vec<_>>(),"interpretation":"model_inferred"});
        let a = t
            .actions
            .iter_mut()
            .find(|a| {
                a.id == action
                    && a.state == ActionState::Executing
                    && matches!(&a.tool, Tool::ChatFinish(q) if q.input_kind == args.input_kind && q.reason == args.reason)
            })
            .ok_or(ControlError::Conflict)?;
        a.state = ActionState::Executed;
        a.result = Some(value.clone());
        c.decision = ChatDecision::NoReply;
        c.wait_until_ms = None;
        t.private_chat = Some(c);
        t.state = TurnState::Completed;
        self.save_actor_turn(&mut t, false)?;
        Ok(value)
    }
    pub fn private_chat_history(&self, id: ActorTurnId) -> Result<Vec<ActorTurn>> {
        let turn = self.actor_turn(id)?;
        let mut history = self.actor_recent(turn.conversation_id)?;
        if turn.private_chat.is_none() {
            return Ok(history);
        }
        for past in &mut history {
            if past.id == id {
                continue;
            }
            let is_channel: bool = self.connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM channel_batches WHERE id=?1)",
                [past.request_id.to_string()],
                |r| r.get(0),
            )?;
            if !is_channel {
                continue;
            }
            if !super::channel_media::selected_current(&self.connection, past)? {
                // Tool arguments/results may also describe pixels; removing only speech revives them.
                past.actions.clear();
            }
            let delivery: Option<String> = self
                .connection
                .query_row(
                    "SELECT snapshot FROM channel_deliveries WHERE batch_id=?1",
                    [past.request_id.to_string()],
                    |r| r.get(0),
                )
                .optional()?;
            let delivery: Option<Delivery> = delivery.map(decode).transpose()?;
            let changed:bool=self.connection.query_row("SELECT EXISTS(SELECT 1 FROM channel_memory_inspections i LEFT JOIN lain_memories m ON m.id=i.memory_id WHERE i.actor_turn_id=?1 AND (m.id IS NULL OR m.status!='\"active\"' OR CAST(json_extract(m.snapshot,'$.revision') AS INTEGER)!=i.revision))",[past.id.to_string()],|r|r.get(0))?;
            if changed
                || !memory_current(&self.connection, past)?
                || delivery
                    .as_ref()
                    .is_none_or(|d| d.state != DeliveryState::Confirmed)
            {
                past.messages.retain(|m| m.role != MessageRole::Assistant);
            } else if let Some(d) = delivery {
                let mut found = false;
                for m in past.messages.iter_mut().rev() {
                    if m.role == MessageRole::Assistant {
                        if !found {
                            m.content = d.text.clone();
                            found = true;
                        } else {
                            m.content.clear();
                        }
                    }
                }
                past.messages.retain(|m| !m.content.is_empty());
            }
        }
        Ok(history)
    }
}

/// This guard only refuses silence; it never selects a canned response or grants authority.
fn reply_required(text: &str) -> bool {
    if text.chars().count() > 64 {
        return true;
    }
    let lower = text.to_lowercase();
    [
        "?",
        "？",
        "帮",
        "请",
        "怎么",
        "什么",
        "为什么",
        "吗",
        "呢",
        "多少",
        "谁",
        "能否",
        "可以",
        "记住",
        "记得",
        "忘记",
        "改",
        "取消",
        "执行",
        "启动",
        "检查",
        "审批",
        "确认",
        "故障",
        "报错",
        "失败",
        "工作",
        "任务",
        "把",
        "给我",
        "告诉",
        "お願い",
        "して",
        "ですか",
        "help",
        "please",
        "can ",
        "could ",
        "would ",
        "what",
        "why",
        "how",
        "when",
        "who",
        "check",
        "start",
        "cancel",
        "remember",
        "forget",
    ]
    .iter()
    .any(|v| lower.contains(v))
}

#[cfg(test)]
#[path = "private_chat_tests.rs"]
mod tests;
