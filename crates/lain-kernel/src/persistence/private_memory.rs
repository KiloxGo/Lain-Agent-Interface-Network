use super::{
    Store,
    memory::{insert_memory, key, read_memory},
};
use crate::{
    actor::{
        domain::{ActionState, ActorTurn, TurnState},
        tools::Tool,
    },
    backend::ControlError,
    channels::domain::{Batch, BatchId, ChannelId, Incoming},
    domain::now_ms,
    memory::*,
};
use anyhow::{Result, ensure};
use lain_wire::{ActionId, ActorTurnId};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

fn decode<T: serde::de::DeserializeOwned>(s: String) -> Result<T> {
    serde_json::from_str(&s).map_err(|_| anyhow::anyhow!("invalid private memory record"))
}
pub(super) fn grant(db: &Connection, id: ChannelId) -> Result<MemoryGrant> {
    let value = db.query_row("SELECT snapshot FROM channel_memory_grants WHERE channel_id=?1 ORDER BY revision DESC LIMIT 1",
        [id.to_string()], |r| r.get::<_, String>(0)).optional()?;
    let value: MemoryGrant = match value {
        Some(v) => decode(v)?,
        None => MemoryGrant {
            channel_id: id,
            enabled: false,
            revision: 0,
            expires_at_ms: None,
            changed_at_ms: 0,
            source_after: 0,
        },
    };
    ensure!(
        value.channel_id == id && (!value.enabled || value.revision > 0),
        "invalid memory grant identity"
    );
    Ok(value)
}
fn batch(db: &Connection, id: BatchId) -> Result<Batch> {
    let value: Batch = decode(
        db.query_row(
            "SELECT snapshot FROM channel_batches WHERE id=?1",
            [id.to_string()],
            |r| r.get(0),
        )
        .optional()?
        .ok_or(ControlError::NotFound)?,
    )?;
    ensure!(value.id == id, "private memory batch identity mismatch");
    Ok(value)
}
fn active(db: &Connection, turn: &ActorTurn) -> Result<(ChannelMemoryContext, MemoryGrant)> {
    let context = turn
        .channel_memory
        .clone()
        .ok_or(ControlError::Unavailable)?;
    let permission = grant(db, context.channel_id)?;
    let b = batch(db, context.batch_id)?;
    let binding: crate::channels::domain::Binding = decode(db.query_row(
        "SELECT snapshot FROM channel_bindings WHERE id=?1",
        [context.channel_id.to_string()],
        |r| r.get(0),
    )?)?;
    let now = now_ms()?;
    ensure!(
        turn.state == TurnState::Running
            && permission.active(now)
            && permission.revision == context.grant_revision
            && now < context.expires_at_ms
            && b.id.0 == turn.request_id
            && b.channel_id == context.channel_id
            && b.conversation_id == turn.conversation_id
            && binding.conversation_id == turn.conversation_id
            && binding.reply_enabled
            && binding.grant_revision == b.grant_revision,
        ControlError::Conflict
    );
    Ok((context, permission))
}
fn evidence(
    db: &Connection,
    c: &ChannelMemoryContext,
    g: &MemoryGrant,
    e: &MemoryEvidence,
) -> Result<()> {
    ensure!(
        !e.quote.trim().is_empty() && e.quote.len() <= 1024,
        ControlError::InvalidInput
    );
    let (snapshot, sequence): (String, i64) = db
        .query_row(
            "SELECT snapshot,sequence FROM channel_incoming WHERE id=?1 AND channel_id=?2",
            params![e.incoming_id.to_string(), c.channel_id.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or(ControlError::NotFound)?;
    let input: Incoming = decode(snapshot)?;
    ensure!(
        input.id == e.incoming_id
            && input.channel_id == c.channel_id
            && input.batch_id == Some(c.batch_id)
            && u64::try_from(sequence)? > g.source_after
            && input.input.text.contains(&e.quote),
        ControlError::InvalidInput
    );
    let suppressed = db
        .query_row(
            "SELECT 1 FROM channel_memory_suppressions WHERE channel_id=?1 AND incoming_id=?2",
            params![c.channel_id.to_string(), e.incoming_id.to_string()],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    ensure!(!suppressed, ControlError::Conflict);
    Ok(())
}
pub(super) fn suppress(db: &Connection, record: &MemoryRecord) -> Result<()> {
    if let MemorySource::ChannelSelection {
        channel_id,
        incoming_id,
        ..
    } = record.provenance.source
    {
        db.execute(
            "INSERT OR IGNORE INTO channel_memory_suppressions VALUES(?1,?2)",
            params![channel_id.to_string(), incoming_id.to_string()],
        )?;
    }
    Ok(())
}
impl Store {
    pub fn memory_grant(&self, id: ChannelId) -> Result<MemoryGrant> {
        self.channel_binding(id)?;
        grant(&self.connection, id)
    }
    pub fn set_memory_grant(
        &mut self,
        id: ChannelId,
        change: SetMemoryGrant,
    ) -> Result<MemoryGrant> {
        self.channel_binding(id)?;
        let now = now_ms()?;
        ensure!(
            change
                .expires_at_ms
                .is_none_or(|t| t > now && t <= i64::MAX as u64)
                && (change.enabled || change.expires_at_ms.is_none()),
            ControlError::InvalidInput
        );
        let tx = self.connection.transaction()?;
        let old = grant(&tx, id)?;
        ensure!(
            old.revision == change.expected_revision,
            ControlError::Conflict
        );
        let permission = MemoryGrant {
            channel_id: id,
            enabled: change.enabled,
            revision: old
                .revision
                .checked_add(1)
                .filter(|r| *r <= i64::MAX as u64)
                .ok_or(ControlError::Conflict)?,
            expires_at_ms: change.expires_at_ms,
            changed_at_ms: now.max(old.changed_at_ms),
            source_after: tx
                .query_row(
                    "SELECT COALESCE(MAX(sequence),0) FROM channel_incoming WHERE channel_id=?1",
                    [id.to_string()],
                    |r| r.get::<_, i64>(0),
                )?
                .try_into()?,
        };
        tx.execute(
            "INSERT INTO channel_memory_grants VALUES(?1,?2,?3)",
            params![
                id.to_string(),
                permission.revision as i64,
                key(&permission)?
            ],
        )?;
        tx.commit()?;
        Ok(permission)
    }
    pub fn memory_grant_history(&self, id: ChannelId) -> Result<Vec<MemoryGrant>> {
        self.channel_binding(id)?;
        let mut stmt = self.connection.prepare("SELECT snapshot FROM channel_memory_grants WHERE channel_id=?1 ORDER BY revision DESC LIMIT 100")?;
        stmt.query_map([id.to_string()], |r| r.get::<_, String>(0))?
            .map(|s| decode(s?))
            .collect()
    }
    pub fn channel_memory_context(
        &self,
        id: ChannelId,
        batch_id: BatchId,
    ) -> Result<Option<ChannelMemoryContext>> {
        let g = self.memory_grant(id)?;
        let b = batch(&self.connection, batch_id)?;
        ensure!(b.channel_id == id, ControlError::InvalidInput);
        let expires_at_ms = b.expires_at_ms.min(g.expires_at_ms.unwrap_or(u64::MAX));
        Ok(
            (g.active(now_ms()?) && now_ms()? < expires_at_ms).then_some(ChannelMemoryContext {
                channel_id: id,
                batch_id,
                grant_revision: g.revision,
                expires_at_ms,
            }),
        )
    }
    pub fn private_memory_context(&mut self, turn_id: ActorTurnId, text: &str) -> Result<Value> {
        let mut turn = self.actor_turn(turn_id)?;
        let (c, g) = active(&self.connection, &turn)?;
        let mut sources = Vec::new();
        let mut size = 2; // JSON array brackets and separators count toward the source budget.
        let mut stmt = self.connection.prepare("SELECT snapshot FROM channel_incoming WHERE channel_id=?1 AND state='batched' AND sequence>?2 ORDER BY sequence DESC LIMIT 128")?;
        for row in stmt.query_map(
            params![c.channel_id.to_string(), g.source_after as i64],
            |r| r.get::<_, String>(0),
        )? {
            let input: Incoming = decode(row?)?;
            if input.batch_id != Some(c.batch_id) {
                continue;
            }
            let mut quote = input.input.text;
            while quote.len() > 1024 {
                quote.pop();
            }
            let item = json!({"incoming_id":input.id,"text_excerpt":quote});
            size += serde_json::to_vec(&item)?.len() + usize::from(!sources.is_empty());
            if size > 4096 {
                break;
            }
            sources.push(item);
        }
        drop(stmt);
        let mut candidates = std::collections::BTreeMap::new();
        if let Ok(id) = text.parse::<MemoryId>() {
            let record = read_memory(&self.connection, id)?;
            ensure!(
                record.status == MemoryStatus::Active
                    && record.draft.scope
                        == (MemoryScope::Channel {
                            channel_id: c.channel_id
                        }),
                ControlError::NotFound
            );
            candidates.insert(record.id, record);
        } else {
            for term in recall_terms(text) {
                let page = self.memory_page(&MemoryQuery {
                    scope: MemoryScope::Channel {
                        channel_id: c.channel_id,
                    },
                    kind: None,
                    status: Some(MemoryStatus::Active),
                    text: Some(term),
                    after: 0,
                    limit: 3,
                })?;
                for record in page.records {
                    candidates.insert(record.id, record);
                }
            }
        }
        let mut records: Vec<_> = candidates.into_values().collect();
        let terms = recall_terms(text);
        records.sort_by_key(|r| {
            std::cmp::Reverse((
                terms
                    .iter()
                    .filter(|t| r.draft.content.to_lowercase().contains(t.as_str()))
                    .count(),
                r.updated_at_ms,
            ))
        });
        records.truncate(3);
        while serde_json::to_vec(&records)?.len() > 4096 {
            records.pop();
        }
        for record in &records {
            self.connection.execute("INSERT INTO channel_memory_inspections VALUES(?1,?2,?3) ON CONFLICT(actor_turn_id,memory_id) DO UPDATE SET revision=excluded.revision",
                params![turn_id.to_string(),record.id.to_string(),i64::try_from(record.revision)?])?;
        }
        if let Some(chat) = &mut turn.private_chat {
            chat.memory_versions = records
                .iter()
                .map(|r| lain_wire::chat::ChatMemoryVersion {
                    memory_id: r.id,
                    revision: r.revision,
                })
                .collect();
            self.save_actor_turn(&mut turn, false)?;
        }
        // Operation receipts survive correction/deletion; their metadata contains no memory text.
        let mut statement = self.connection.prepare(
            "SELECT snapshot FROM channel_memory_effects WHERE channel_id=?1 ORDER BY sequence DESC LIMIT 3",
        )?;
        let mut committed_effects = statement
            .query_map([c.channel_id.to_string()], |row| row.get::<_, String>(0))?
            .map(|row| decode::<MemoryEffect>(row?))
            .collect::<Result<Vec<_>>>()?;
        committed_effects.reverse();
        Ok(
            json!({"context_role":"untrusted_memory_evidence", "channel_id":c.channel_id,
            "grant_revision":c.grant_revision, "current_sources":sources, "records":records,
            "committed_effects_role":"kernel_operation_metadata", "committed_effects":committed_effects}),
        )
    }
    pub fn private_memory_effect(
        &mut self,
        turn_id: ActorTurnId,
        action_id: ActionId,
    ) -> Result<MemoryEffect> {
        let turn = self.actor_turn(turn_id)?;
        let tx = self.connection.transaction()?;
        let (c, g) = active(&tx, &turn)?;
        super::private_chat::check_current(&tx, &turn)?;
        let action = turn
            .actions
            .iter()
            .find(|a| a.id == action_id && a.state == ActionState::Executing)
            .ok_or(ControlError::Conflict)?;
        action.tool.validate()?;
        if let Some(old) = tx
            .query_row(
                "SELECT snapshot FROM channel_memory_effects WHERE action_id=?1",
                [action_id.to_string()],
                |r| r.get::<_, String>(0),
            )
            .optional()?
        {
            let old: MemoryEffect = decode(old)?;
            ensure!(
                old.actor_turn_id == turn_id && old.channel_id == c.channel_id,
                ControlError::Conflict
            );
            return Ok(old);
        }
        let (selection, previous, e, operation) = match &action.tool {
            Tool::MemoryRemember(s) => (Some(s), None, &s.evidence, MemoryOperation::Remember),
            Tool::MemoryCorrect(s) => (
                Some(&s.selection),
                Some((s.memory_id, s.expected_revision)),
                &s.selection.evidence,
                MemoryOperation::Correct,
            ),
            Tool::MemoryForget(s) => (
                None,
                Some((s.memory_id, s.expected_revision)),
                &s.evidence,
                MemoryOperation::Forget,
            ),
            _ => return Err(ControlError::InvalidInput.into()),
        };
        evidence(&tx, &c, &g, e)?;
        if previous.is_some() {
            for prior in turn.actions.iter().filter(|a| {
                a.id != action_id
                    && matches!(a.tool, Tool::MemoryCorrect(_) | Tool::MemoryForget(_))
            }) {
                let committed = tx
                    .query_row(
                        "SELECT 1 FROM channel_memory_effects WHERE action_id=?1",
                        [prior.id.to_string()],
                        |_| Ok(()),
                    )
                    .optional()?;
                // Bulk correction/forget must be clarified, not split into tool calls.
                ensure!(committed.is_none(), ControlError::Conflict);
            }
        }
        let old = previous.map(|(id, revision)| -> Result<MemoryRecord> {
            let value = read_memory(&tx,id)?;
            ensure!(value.draft.scope == (MemoryScope::Channel { channel_id:c.channel_id })
                && value.status == MemoryStatus::Active && value.revision == revision,ControlError::Conflict);
            let inspected = tx.query_row("SELECT revision FROM channel_memory_inspections WHERE actor_turn_id=?1 AND memory_id=?2",
                params![turn_id.to_string(),id.to_string()],|r|r.get::<_,i64>(0)).optional()?;
            ensure!(inspected == Some(i64::try_from(revision)?),ControlError::Conflict);
            Ok(value)
        }).transpose()?;
        let new = if let Some(s) = selection {
            Some(insert_memory(
                &tx,
                MemoryDraft {
                    content: s.content.clone(),
                    kind: s.kind,
                    scope: MemoryScope::Channel {
                        channel_id: c.channel_id,
                    },
                },
                MemoryProvenance {
                    authority: MemoryAuthority::Inferred,
                    source: MemorySource::ChannelSelection {
                        channel_id: c.channel_id,
                        conversation_id: turn.conversation_id,
                        batch_id: c.batch_id,
                        incoming_id: e.incoming_id,
                        actor_turn_id: turn_id,
                        action_id,
                        evidence: e.quote.clone(),
                        grant_revision: c.grant_revision,
                        grant_expires_at_ms: g.expires_at_ms,
                    },
                },
            )?)
        } else {
            None
        };
        if let Some(record) = &new {
            // Every autonomously selected record must fit a subsequent bounded recall.
            ensure!(
                serde_json::to_vec(&vec![record])?.len() <= 4096,
                ControlError::InvalidInput
            );
        }
        if let Some(mut old) = old {
            suppress(&tx, &old)?;
            if let Some(new) = &new {
                old.status = MemoryStatus::Superseded;
                old.superseded_by = Some(new.id);
                old.revision = old.revision.checked_add(1).ok_or(ControlError::Conflict)?;
                old.updated_at_ms = now_ms()?.max(old.updated_at_ms);
                tx.execute(
                    "UPDATE lain_memories SET status=?2,snapshot=?3 WHERE id=?1",
                    params![old.id.to_string(), key(&old.status)?, key(&old)?],
                )?;
            } else {
                tx.execute(
                    "DELETE FROM lain_memories WHERE id=?1",
                    [old.id.to_string()],
                )?;
            }
        }
        let effect = MemoryEffect {
            action_id,
            actor_turn_id: turn_id,
            channel_id: c.channel_id,
            grant_revision: c.grant_revision,
            operation,
            memory_id: new
                .as_ref()
                .map(|r| r.id)
                .or(previous.map(|(id, _)| id))
                .ok_or(ControlError::InvalidInput)?,
            previous_memory_id: previous.map(|(id, _)| id),
            recorded_at_ms: now_ms()?,
        };
        tx.execute(
            "INSERT INTO channel_memory_effects(action_id,channel_id,snapshot) VALUES(?1,?2,?3)",
            params![
                action_id.to_string(),
                c.channel_id.to_string(),
                key(&effect)?
            ],
        )?;
        tx.commit()?;
        Ok(effect)
    }
    pub fn memory_effects(&self, id: ChannelId, after: u64) -> Result<Value> {
        self.channel_binding(id)?;
        ensure!(after <= i64::MAX as u64, ControlError::InvalidInput);
        let mut stmt = self.connection.prepare("SELECT sequence,snapshot FROM channel_memory_effects WHERE channel_id=?1 AND sequence>?2 ORDER BY sequence LIMIT 51")?;
        let values = stmt
            .query_map(params![id.to_string(), after as i64], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let more = values.len() > 50;
        let mut cursor = after;
        let mut effects = Vec::<MemoryEffect>::new();
        for (n, s) in values.into_iter().take(50) {
            cursor = n.try_into()?;
            effects.push(decode(s)?);
        }
        Ok(json!({"effects":effects,"next_cursor":cursor,"has_more":more}))
    }
}
fn recall_terms(text: &str) -> Vec<String> {
    let mut terms = Vec::new();
    for word in text.split(|c: char| !c.is_alphanumeric()) {
        let word = word.to_lowercase();
        if (3..=32).contains(&word.len()) && !terms.contains(&word) {
            terms.push(word.clone());
        }
        let chars: Vec<_> = word.chars().collect();
        for pair in chars.windows(2) {
            if pair.iter().all(|c| !c.is_ascii()) {
                let term: String = pair.iter().collect();
                if !terms.contains(&term) {
                    terms.push(term);
                }
            }
        }
    }
    if terms.len() > 8 {
        let tail = terms.split_off(terms.len() - 4);
        terms.truncate(4);
        terms.extend(tail);
    }
    terms
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::domain::{Action, MessageRole};
    use crate::channels::domain::{Binding, BindingSpec, Input};

    fn binding(s: &mut Store, name: &str) -> Binding {
        s.channel_bind(
            BindingSpec {
                name: name.into(),
                adapter: "fixture".into(),
                account: "1001".into(),
                peer: "2002".into(),
            },
            true,
        )
        .unwrap()
    }
    fn enable(s: &mut Store, b: &Binding) -> MemoryGrant {
        let old = s.memory_grant(b.id).unwrap();
        s.set_memory_grant(
            b.id,
            SetMemoryGrant {
                enabled: true,
                expected_revision: old.revision,
                expires_at_ms: None,
            },
        )
        .unwrap()
    }
    fn turn(s: &mut Store, b: &Binding, text: &str) -> (ActorTurn, MemoryEvidence) {
        let input = s
            .channel_receive(
                b.id,
                Input {
                    account: "1001".into(),
                    peer: "2002".into(),
                    external_id: uuid::Uuid::new_v4().to_string(),
                    text: text.into(),
                },
            )
            .unwrap();
        let batch = s
            .channel_next_batch(b.id, now_ms().unwrap() + 1000)
            .unwrap()
            .unwrap();
        let mut turn = ActorTurn {
            reading_activity: None,
            sharing_activity: None,
            id: ActorTurnId::new(),
            conversation_id: b.conversation_id,
            request_id: batch.id.0,
            sequence: 0,
            revision: 0,
            state: TurnState::Running,
            messages: vec![],
            actions: vec![],
            error: None,
            model_requests: 0,
            usage_tokens: 0,
            timing: Default::default(),
            channel_memory: s.channel_memory_context(b.id, batch.id).unwrap(),
            channel_source: None,
            private_chat: None,
        };
        turn.message(MessageRole::User, text.into()).unwrap();
        s.save_actor_turn(&mut turn, true).unwrap();
        s.channel_link(batch.id, &turn).unwrap();
        (
            turn,
            MemoryEvidence {
                incoming_id: input.id,
                quote: text.into(),
            },
        )
    }
    fn action(s: &mut Store, t: &mut ActorTurn, tool: Tool) -> ActionId {
        let id = ActionId::new();
        t.actions.push(Action {
            id,
            call_id: id.to_string(),
            tool,
            state: ActionState::Executing,
            result: None,
        });
        s.save_actor_turn(t, false).unwrap();
        id
    }
    fn finish(s: &mut Store, t: &mut ActorTurn) {
        for a in &mut t.actions {
            a.state = ActionState::Executed;
            a.result = Some(json!({"fixture":true}));
        }
        t.state = TurnState::Completed;
        s.save_actor_turn(t, false).unwrap();
        s.channel_finish(BatchId(t.request_id), t).unwrap();
    }
    fn remember(s: &mut Store, b: &Binding, text: &str) -> MemoryRecord {
        let (mut t, e) = turn(s, b, text);
        let id = action(
            s,
            &mut t,
            Tool::MemoryRemember(SelectMemory {
                content: text.into(),
                kind: MemoryKind::Preference,
                evidence: e,
            }),
        );
        let receipt = s.private_memory_effect(t.id, id).unwrap();
        assert_eq!(
            s.private_memory_effect(t.id, id).unwrap().memory_id,
            receipt.memory_id
        );
        finish(s, &mut t);
        s.memory(receipt.memory_id).unwrap()
    }

    #[test]
    fn grants_default_off_are_revisioned_persistent_and_do_not_backfill_old_sources() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.db");
        let mut s = Store::open(&path).unwrap();
        let b = binding(&mut s, "one");
        assert!(!s.memory_grant(b.id).unwrap().active(now_ms().unwrap()));
        let (mut old, e) = turn(&mut s, &b, "old message");
        assert!(old.channel_memory.is_none());
        let first = enable(&mut s, &b);
        let id = action(
            &mut s,
            &mut old,
            Tool::MemoryRemember(SelectMemory {
                content: "old".into(),
                kind: MemoryKind::Preference,
                evidence: e,
            }),
        );
        assert!(s.private_memory_effect(old.id, id).is_err());
        // A queued pre-grant batch admitted after enabling must still not backfill.
        old.channel_memory = s
            .channel_memory_context(b.id, BatchId(old.request_id))
            .unwrap();
        s.save_actor_turn(&mut old, false).unwrap();
        assert!(old.channel_memory.is_some());
        assert!(
            s.private_memory_context(old.id, "old").unwrap()["current_sources"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(s.private_memory_effect(old.id, id).is_err());
        finish(&mut s, &mut old);
        let record = remember(&mut s, &b, "喜欢柠檬茶");
        assert!(record.owner_confirmed_at_ms.is_none());
        assert_eq!(record.provenance.authority, MemoryAuthority::Inferred);
        let (mut t, e) = turn(&mut s, &b, "new message");
        let a = action(
            &mut s,
            &mut t,
            Tool::MemoryRemember(SelectMemory {
                content: "new".into(),
                kind: MemoryKind::Preference,
                evidence: e,
            }),
        );
        s.set_memory_grant(
            b.id,
            SetMemoryGrant {
                enabled: false,
                expected_revision: first.revision,
                expires_at_ms: None,
            },
        )
        .unwrap();
        assert!(s.private_memory_effect(t.id, a).is_err());
        let next = enable(&mut s, &b);
        assert!(s.private_memory_effect(t.id, a).is_err());
        assert!(
            s.set_memory_grant(
                b.id,
                SetMemoryGrant {
                    enabled: true,
                    expected_revision: first.revision,
                    expires_at_ms: None
                }
            )
            .is_err()
        );
        drop(s);
        let mut s = Store::open(&path).unwrap();
        s.recover_actor().unwrap();
        s.recover_channels().unwrap();
        assert_eq!(s.memory_grant(b.id).unwrap().revision, next.revision);
        assert_eq!(s.memory_grant_history(b.id).unwrap().len(), 3);
        assert_eq!(s.memory(record.id).unwrap(), record);
        assert_eq!(s.actor_turn(t.id).unwrap().state, TurnState::Unknown);
        assert_eq!(
            s.memory_effects(b.id, 0).unwrap()["effects"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn correct_forget_scope_evidence_and_tombstones_change_active_retrieval() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(&dir.path().join("state.db")).unwrap();
        let b = binding(&mut s, "one");
        let other = binding(&mut s, "other");
        enable(&mut s, &b);
        enable(&mut s, &other);
        let old = remember(&mut s, &b, "喜欢柠檬茶");
        let foreign = remember(&mut s, &other, "喜欢咖啡");
        let (mut t, e) = turn(&mut s, &b, "我说错了，现在喜欢白水");
        let selection = SelectMemory {
            content: "喜欢白水".into(),
            kind: MemoryKind::Preference,
            evidence: e.clone(),
        };
        let a = action(
            &mut s,
            &mut t,
            Tool::MemoryCorrect(CorrectMemory {
                memory_id: old.id,
                expected_revision: old.revision,
                selection: selection.clone(),
            }),
        );
        assert!(
            s.private_memory_effect(t.id, a).is_err(),
            "target must have been inspected"
        );
        let context = s.private_memory_context(t.id, "柠檬茶").unwrap();
        assert_eq!(context["records"][0]["id"], json!(old.id));
        let effect = s.private_memory_effect(t.id, a).unwrap();
        s.private_memory_context(t.id, "白水").unwrap();
        let second = action(
            &mut s,
            &mut t,
            Tool::MemoryForget(ForgetMemory {
                memory_id: effect.memory_id,
                expected_revision: 1,
                evidence: e.clone(),
            }),
        );
        assert!(
            s.private_memory_effect(t.id, second).is_err(),
            "only one correction/forget target per turn"
        );
        finish(&mut s, &mut t);
        assert_eq!(s.memory(old.id).unwrap().status, MemoryStatus::Superseded);
        assert_eq!(
            s.memory(old.id).unwrap().superseded_by,
            Some(effect.memory_id)
        );
        let new = s.memory(effect.memory_id).unwrap();
        let (mut t, e) = turn(&mut s, &b, "忘记白水偏好");
        assert!(
            s.private_memory_context(t.id, "柠檬茶").unwrap()["records"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(
            s.private_memory_context(t.id, "咖啡").unwrap()["records"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let a = action(
            &mut s,
            &mut t,
            Tool::MemoryForget(ForgetMemory {
                memory_id: foreign.id,
                expected_revision: 1,
                evidence: e.clone(),
            }),
        );
        assert!(s.private_memory_effect(t.id, a).is_err());
        t.actions.pop();
        let a = action(
            &mut s,
            &mut t,
            Tool::MemoryForget(ForgetMemory {
                memory_id: new.id,
                expected_revision: 2,
                evidence: e.clone(),
            }),
        );
        s.private_memory_context(t.id, "白水").unwrap();
        assert!(s.private_memory_effect(t.id, a).is_err());
        t.actions.pop();
        let a = action(
            &mut s,
            &mut t,
            Tool::MemoryForget(ForgetMemory {
                memory_id: new.id,
                expected_revision: 1,
                evidence: e,
            }),
        );
        s.private_memory_effect(t.id, a).unwrap();
        assert!(s.memory(new.id).is_err());
        let after_forget = s.private_memory_context(t.id, "白水").unwrap();
        let receipts = after_forget["committed_effects"].as_array().unwrap();
        assert_eq!(receipts.len(), 3);
        assert_eq!(receipts[0]["operation"], "remember");
        assert_eq!(receipts[1]["operation"], "correct");
        assert_eq!(receipts[2]["operation"], "forget");
        assert!(receipts.iter().all(|r| r["channel_id"] == json!(b.id)));
        assert!(receipts.iter().all(|r| r["memory_id"] != json!(foreign.id)));
        let metadata = serde_json::to_string(receipts).unwrap();
        assert!(!metadata.contains("白水") && !metadata.contains("柠檬茶"));
        assert!(after_forget["records"].as_array().unwrap().is_empty());
        assert!(
            s.private_memory_context(t.id, "白水").unwrap()["records"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        // Use each original batch, so the tombstone is the only reason for rejection.
        for record in [&old, &new] {
            let MemorySource::ChannelSelection {
                batch_id,
                incoming_id,
                evidence: quote,
                ..
            } = &record.provenance.source
            else {
                unreachable!()
            };
            let context = s.channel_memory_context(b.id, *batch_id).unwrap().unwrap();
            let source = MemoryEvidence {
                incoming_id: *incoming_id,
                quote: quote.clone(),
            };
            assert!(
                evidence(
                    &s.connection,
                    &context,
                    &s.memory_grant(b.id).unwrap(),
                    &source
                )
                .is_err()
            );
            s.connection
                .execute(
                    "DELETE FROM channel_memory_suppressions WHERE incoming_id=?1",
                    [incoming_id.to_string()],
                )
                .unwrap();
            evidence(
                &s.connection,
                &context,
                &s.memory_grant(b.id).unwrap(),
                &source,
            )
            .unwrap();
        }
        assert_eq!(
            s.memory_effects(b.id, 0).unwrap()["effects"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
    }

    #[test]
    fn effect_failures_roll_back_memory_fts_suppression_and_receipts() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(&dir.path().join("state.db")).unwrap();
        let b = binding(&mut s, "one");
        enable(&mut s, &b);
        let old = remember(&mut s, &b, "喜欢柠檬茶");
        let (mut t, e) = turn(&mut s, &b, "改为喜欢白水");
        s.private_memory_context(t.id, "柠檬茶").unwrap();
        let a = action(
            &mut s,
            &mut t,
            Tool::MemoryCorrect(CorrectMemory {
                memory_id: old.id,
                expected_revision: 1,
                selection: SelectMemory {
                    content: "喜欢白水".into(),
                    kind: MemoryKind::Preference,
                    evidence: e,
                },
            }),
        );
        s.connection.execute_batch("CREATE TRIGGER fixture_fail_effect BEFORE INSERT ON channel_memory_effects BEGIN SELECT RAISE(FAIL,'fixture failure'); END;").unwrap();
        assert!(s.private_memory_effect(t.id, a).is_err());
        assert_eq!(s.memory(old.id).unwrap(), old);
        assert_eq!(
            s.connection
                .query_row("SELECT count(*) FROM lain_memories", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            s.connection
                .query_row(
                    "SELECT count(*) FROM channel_memory_suppressions",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        assert!(
            s.private_memory_context(t.id, "白水").unwrap()["records"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        s.connection
            .execute_batch("DROP TRIGGER fixture_fail_effect")
            .unwrap();
        assert!(s.private_memory_effect(t.id, a).is_ok());
    }

    #[test]
    fn recall_is_bounded_relevant_and_does_not_cache_retired_records() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(&dir.path().join("state.db")).unwrap();
        let b = binding(&mut s, "one");
        enable(&mut s, &b);
        let selected: Vec<_> = (0..6)
            .map(|n| remember(&mut s, &b, &format!("preference lemon {n}")))
            .collect();
        remember(&mut s, &b, "unrelated coffee");
        let (t, _) = turn(&mut s, &b, "recall lemon");
        let context = s.private_memory_context(t.id, "lemon").unwrap();
        let records = context["records"].as_array().unwrap();
        assert_eq!(records.len(), 3);
        assert!(serde_json::to_vec(records).unwrap().len() <= 4096);
        assert!(
            records
                .iter()
                .all(|r| r["content"].as_str().unwrap().contains("lemon"))
        );
        let id: MemoryId = serde_json::from_value(records[0]["id"].clone()).unwrap();
        s.change_memory(
            id,
            &ChangeMemory {
                expected_revision: 1,
                status: MemoryStatus::Disabled,
                superseded_by: None,
            },
        )
        .unwrap();
        assert!(
            s.private_memory_context(t.id, "lemon").unwrap()["records"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["id"] != json!(id))
        );
        assert_eq!(selected.len(), 6);
        assert!(
            s.private_memory_context(t.id, "no-match").unwrap()["records"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn correction_relevance_keeps_suffix_and_receipt_ids_are_binding_scoped() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(&dir.path().join("state.db")).unwrap();
        let b = binding(&mut s, "one");
        enable(&mut s, &b);
        let old = remember(&mut s, &b, "用户喝茶喜欢无糖。");
        let text = "更正一下，那条测试偏好改成喝茶微糖，不是无糖";
        let (mut t, e) = turn(&mut s, &b, text);
        let context = s.private_memory_context(t.id, text).unwrap();
        assert!(
            context["records"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["id"] == json!(old.id))
        );
        let exact = s.private_memory_context(t.id, &old.id.to_string()).unwrap();
        assert_eq!(exact["records"][0]["id"], json!(old.id));
        let other = binding(&mut s, "other");
        enable(&mut s, &other);
        let foreign = remember(&mut s, &other, "foreign memory");
        assert!(
            s.private_memory_context(t.id, &foreign.id.to_string())
                .is_err()
        );
        let a = action(
            &mut s,
            &mut t,
            Tool::MemoryCorrect(CorrectMemory {
                memory_id: old.id,
                expected_revision: 1,
                selection: SelectMemory {
                    content: "喝茶喜欢微糖。".into(),
                    kind: MemoryKind::Preference,
                    evidence: e,
                },
            }),
        );
        assert!(s.private_memory_effect(t.id, a).is_ok());
        assert!(s.private_memory_context(t.id, &old.id.to_string()).is_err());
    }

    #[test]
    fn schema_eight_migration_preserves_reviewed_records_and_defaults_grants_off() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.db");
        let mut s = Store::open(&path).unwrap();
        let b = binding(&mut s, "existing");
        let legacy = s
            .add_memory(&AddMemory {
                request_id: lain_wire::RequestId::new(),
                draft: MemoryDraft {
                    content: "old reviewed preference".into(),
                    kind: MemoryKind::Preference,
                    scope: MemoryScope::Global,
                },
            })
            .unwrap();
        let before: String = s
            .connection
            .query_row(
                "SELECT snapshot FROM lain_memories WHERE id=?1",
                [legacy.id.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        assert!(legacy.owner_confirmed_at_ms.is_some());
        s.connection.execute_batch("DROP TABLE knowledge_records; DROP TABLE knowledge_sources; DROP TABLE archive_recall_reads; DROP TABLE archive_recall_grants; DROP TABLE archive_reads; DROP TABLE archive_edits; DROP TABLE archive_records; DROP TABLE archive_grants; DROP TABLE archive_sources; DROP TABLE inference_requests; DROP TABLE inference_decisions; DROP TABLE inference_budgets; DROP TABLE share_reads; DROP TABLE share_model_requests; DROP TABLE share_jobs; DROP TABLE share_deliveries; DROP TABLE share_candidates; DROP TABLE channel_attention; DROP TABLE share_grants; DROP TABLE sharing_runtime; DROP TABLE reading_source_decisions; DROP TABLE experience_edits; DROP TABLE reading_source_suppressions; DROP TABLE experience_reads; DROP TABLE reading_model_requests; DROP TABLE reading_jobs; DROP TABLE reading_experiences; DROP TABLE reading_withdrawals; DROP TABLE reading_results; DROP TABLE reading_activities; DROP TABLE reading_grants; DROP TABLE channel_source_reads; DROP TABLE channel_source_grants; DROP INDEX channel_source_window; DROP INDEX channel_incoming_batch; DROP TABLE channel_memory_inspections; DROP TABLE channel_memory_suppressions; DROP TABLE channel_memory_effects; DROP TABLE channel_memory_grants; PRAGMA user_version=8;").unwrap();
        drop(s);
        let s = Store::open(&path).unwrap();
        assert_eq!(s.memory(legacy.id).unwrap(), legacy);
        assert_eq!(
            s.connection
                .query_row(
                    "SELECT snapshot FROM lain_memories WHERE id=?1",
                    [legacy.id.to_string()],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
            before
        );
        assert_eq!(s.memory_grant(b.id).unwrap().revision, 0);
        assert!(!s.memory_grant(b.id).unwrap().enabled);
        assert!(s.memory_grant_history(b.id).unwrap().is_empty());
        assert!(
            s.memory_effects(b.id, 0).unwrap()["effects"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            s.connection
                .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            crate::persistence::SCHEMA_VERSION
        );
    }

    #[test]
    fn expiry_reply_revocation_invalid_evidence_and_secret_content_fail_closed() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(&dir.path().join("state.db")).unwrap();
        let b = binding(&mut s, "one");
        let g = enable(&mut s, &b);
        let (mut t, e) = turn(&mut s, &b, "你好");
        let a = action(
            &mut s,
            &mut t,
            Tool::MemoryRemember(SelectMemory {
                content: "sk-fixtureSensitiveValue123456".into(),
                kind: MemoryKind::Preference,
                evidence: e.clone(),
            }),
        );
        assert!(s.private_memory_effect(t.id, a).is_err());
        t.actions.pop();
        let mut false_evidence = e.clone();
        false_evidence.quote = "invented quote".into();
        let a = action(
            &mut s,
            &mut t,
            Tool::MemoryRemember(SelectMemory {
                content: "hello".into(),
                kind: MemoryKind::Preference,
                evidence: false_evidence,
            }),
        );
        assert!(s.private_memory_effect(t.id, a).is_err());
        t.actions.pop();
        let a = action(
            &mut s,
            &mut t,
            Tool::MemoryRemember(SelectMemory {
                content: "long ".repeat(800),
                kind: MemoryKind::Preference,
                evidence: e.clone(),
            }),
        );
        assert!(
            s.private_memory_effect(t.id, a).is_err(),
            "a selected record must fit later recall"
        );
        t.actions.pop();
        let a = action(
            &mut s,
            &mut t,
            Tool::MemoryRemember(SelectMemory {
                content: "hello".into(),
                kind: MemoryKind::Preference,
                evidence: e,
            }),
        );
        let mut expired = g.clone();
        expired.expires_at_ms = Some(now_ms().unwrap() - 1);
        s.connection
            .execute(
                "UPDATE channel_memory_grants SET snapshot=?1",
                [key(&expired).unwrap()],
            )
            .unwrap();
        assert!(s.private_memory_effect(t.id, a).is_err());
        s.connection
            .execute(
                "UPDATE channel_memory_grants SET snapshot=?1",
                [key(&g).unwrap()],
            )
            .unwrap();
        s.channel_grant(b.id, false, b.grant_revision).unwrap();
        assert!(s.private_memory_effect(t.id, a).is_err());
        assert_eq!(
            s.connection
                .query_row("SELECT count(*) FROM lain_memories", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
