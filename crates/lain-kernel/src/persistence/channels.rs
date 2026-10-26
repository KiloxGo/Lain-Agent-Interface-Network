use super::Store;
use crate::{
    actor::domain::{ActorTurn, ConversationId, MessageRole, TurnState},
    channels::domain::*,
    domain::now_ms,
};
use anyhow::{Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Serialize, de::DeserializeOwned};

pub(super) fn decode<T: DeserializeOwned>(text: String) -> Result<T> {
    serde_json::from_str(&text).map_err(|_| anyhow::anyhow!("invalid persisted channel record"))
}
pub(super) fn one<T: DeserializeOwned>(db: &Connection, sql: &str, key: String) -> Result<T> {
    let text = db
        .query_row(sql, [key], |r| r.get::<_, String>(0))
        .optional()?
        .ok_or(crate::backend::ControlError::NotFound)?;
    decode(text)
}
pub(super) fn rows<T: DeserializeOwned>(db: &Connection, sql: &str, key: String) -> Result<Vec<T>> {
    let mut query = db.prepare(sql)?;
    query
        .query_map([key], |r| r.get::<_, String>(0))?
        .map(|r| decode(r?))
        .collect()
}
fn state(value: &impl Serialize) -> Result<String> {
    Ok(serde_json::to_value(value)?
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("invalid channel state"))?
        .to_owned())
}
pub(super) fn save_input(db: &Connection, value: &Incoming) -> Result<()> {
    db.execute(
        "UPDATE channel_incoming SET state=?2,snapshot=?3 WHERE id=?1",
        (
            value.id.to_string(),
            state(&value.state)?,
            serde_json::to_string(value)?,
        ),
    )?;
    Ok(())
}
pub(super) fn save_batch(db: &Connection, value: &Batch) -> Result<()> {
    db.execute(
        "UPDATE channel_batches SET state=?2,snapshot=?3 WHERE id=?1",
        (
            value.id.to_string(),
            state(&value.state)?,
            serde_json::to_string(value)?,
        ),
    )?;
    Ok(())
}
pub(super) fn save_delivery(db: &Connection, value: &Delivery) -> Result<()> {
    db.execute(
        "UPDATE channel_deliveries SET state=?2,snapshot=?3 WHERE id=?1",
        (
            value.id.to_string(),
            state(&value.state)?,
            serde_json::to_string(value)?,
        ),
    )?;
    Ok(())
}
pub(super) fn binding(db: &Connection, id: ChannelId) -> Result<Binding> {
    one(
        db,
        "SELECT snapshot FROM channel_bindings WHERE id=?1",
        id.to_string(),
    )
}
impl Store {
    pub fn channel_bind(&mut self, spec: BindingSpec, enabled: bool) -> Result<Binding> {
        spec.validate()?;
        let old: Option<String> = self
            .connection
            .query_row(
                "SELECT snapshot FROM channel_bindings WHERE name=?1",
                [&spec.name],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(old) = old {
            let old: Binding = decode(old)?;
            ensure!(
                old.spec == spec,
                "existing channel route is immutable; choose a new binding name"
            );
            // Startup config may revoke, but never silently resurrect a persisted revocation.
            return if !enabled && old.reply_enabled {
                self.channel_grant(old.id, false, old.grant_revision)
            } else {
                Ok(old)
            };
        }
        let now = now_ms()?;
        let value = Binding {
            id: ChannelId::new(),
            spec,
            conversation_id: ConversationId::new(),
            reply_enabled: enabled,
            grant_revision: 1,
        };
        let tx = self.connection.transaction()?;
        tx.execute(
            "INSERT INTO lain_conversations VALUES (?1,?2,?2)",
            (value.conversation_id.to_string(), i64::try_from(now)?),
        )?;
        tx.execute(
            "INSERT INTO channel_bindings VALUES (?1,?2,?3)",
            (
                value.id.to_string(),
                &value.spec.name,
                serde_json::to_string(&value)?,
            ),
        )?;
        tx.commit()?;
        Ok(value)
    }
    pub fn channel_binding(&self, id: ChannelId) -> Result<Binding> {
        binding(&self.connection, id)
    }
    pub fn channel_bindings(&self) -> Result<Vec<Binding>> {
        let mut query = self
            .connection
            .prepare("SELECT snapshot FROM channel_bindings ORDER BY name LIMIT 100")?;
        query
            .query_map([], |r| r.get::<_, String>(0))?
            .map(|r| decode(r?))
            .collect()
    }
    pub fn channel_grant(
        &mut self,
        id: ChannelId,
        enabled: bool,
        expected: u64,
    ) -> Result<Binding> {
        let mut value = binding(&self.connection, id)?;
        ensure!(
            value.grant_revision == expected,
            crate::backend::ControlError::Conflict
        );
        if value.reply_enabled == enabled {
            return Ok(value);
        }
        value.reply_enabled = enabled;
        value.grant_revision = value
            .grant_revision
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("grant revision exhausted"))?;
        let tx = self.connection.transaction()?;
        tx.execute(
            "UPDATE channel_bindings SET snapshot=?2 WHERE id=?1",
            (id.to_string(), serde_json::to_string(&value)?),
        )?;
        // Both revocation and a fresh grant invalidate all work from the older grant.
        for mut input in rows::<Incoming>(
            &tx,
            "SELECT snapshot FROM channel_incoming WHERE channel_id=?1 AND state='queued'",
            id.to_string(),
        )? {
            input.state = InputState::Revoked;
            save_input(&tx, &input)?;
        }
        for mut batch in rows::<Batch>(
            &tx,
            "SELECT snapshot FROM channel_batches WHERE channel_id=?1 AND state IN ('prepared','running')",
            id.to_string(),
        )? {
            batch.state = BatchState::Revoked;
            save_batch(&tx, &batch)?;
        }
        for mut delivery in rows::<Delivery>(
            &tx,
            "SELECT snapshot FROM channel_deliveries WHERE channel_id=?1 AND state='ready'",
            id.to_string(),
        )? {
            delivery.state = DeliveryState::Revoked;
            save_delivery(&tx, &delivery)?;
        }
        // Sending already crossed the commit boundary; revocation cannot unsend it.
        tx.commit()?;
        Ok(value)
    }
    pub fn channel_receive(&mut self, id: ChannelId, input: Input) -> Result<Incoming> {
        input.validate()?;
        let value = binding(&self.connection, id)?;
        ensure!(
            value.reply_enabled
                && value.spec.account == input.account
                && value.spec.peer == input.peer,
            crate::backend::ControlError::InvalidInput
        );
        let old: Option<String> = self
            .connection
            .query_row(
                "SELECT snapshot FROM channel_incoming WHERE channel_id=?1 AND external_id=?2",
                (id.to_string(), &input.external_id),
                |r| r.get(0),
            )
            .optional()?;
        if let Some(old) = old {
            let old: Incoming = decode(old)?;
            ensure!(old.input == input, crate::backend::ControlError::Conflict);
            return Ok(old);
        }
        let queued: i64 = self.connection.query_row(
            "SELECT count(*) FROM channel_incoming WHERE channel_id=?1 AND state='queued'",
            [id.to_string()],
            |r| r.get(0),
        )?;
        ensure!(queued < MAX_QUEUED as i64, "channel input queue full");
        let record = Incoming {
            id: IncomingId::new(),
            channel_id: id,
            input,
            received_at_ms: now_ms()?,
            grant_revision: value.grant_revision,
            state: InputState::Queued,
            batch_id: None,
        };
        let tx = self.connection.transaction()?;
        tx.execute("INSERT INTO channel_incoming(id,channel_id,external_id,state,snapshot) VALUES (?1,?2,?3,'queued',?4)", (record.id.to_string(), id.to_string(), &record.input.external_id, serde_json::to_string(&record)?))?;
        for mut delivery in rows::<Delivery>(
            &tx,
            "SELECT snapshot FROM channel_deliveries WHERE channel_id=?1 AND state='ready' ORDER BY sequence LIMIT 128",
            id.to_string(),
        )? {
            if !super::private_chat::delivery_current(&tx, &delivery)? {
                delivery.state = DeliveryState::Superseded;
                save_delivery(&tx, &delivery)?;
            }
        }
        tx.commit()?;
        Ok(record)
    }
    /// Returns the same prepared/running batch after a crash. Request UUID is durable.
    pub fn channel_next_batch(&mut self, id: ChannelId, now: u64) -> Result<Option<Batch>> {
        self.channel_expire(id, now)?;
        let value = binding(&self.connection, id)?;
        if !value.reply_enabled {
            return Ok(None);
        }
        let active = rows::<Batch>(
            &self.connection,
            "SELECT snapshot FROM channel_batches WHERE channel_id=?1 AND state IN ('prepared','running') ORDER BY sequence LIMIT 1",
            id.to_string(),
        )?;
        if let Some(mut active) = active.into_iter().next() {
            if now >= active.expires_at_ms {
                active.state = BatchState::Expired;
                save_batch(&self.connection, &active)?;
            } else {
                return Ok(Some(active));
            }
        }
        // A proposal awaiting explicit confirmation continues to own its conversation.
        if self
            .actor_recent(value.conversation_id)?
            .iter()
            .any(|t| t.state.active())
        {
            return Ok(None);
        }
        let pending = rows::<Incoming>(
            &self.connection,
            "SELECT snapshot FROM channel_incoming WHERE channel_id=?1 AND state='queued' ORDER BY sequence LIMIT 128",
            id.to_string(),
        )?;
        if merge_deadline(pending.iter().map(|i| i.received_at_ms)).is_some_and(|due| now < due) {
            return Ok(None);
        }
        let tx = self.connection.transaction()?;
        let mut selected = Vec::new();
        let mut text = String::new();
        for mut input in pending {
            if input.grant_revision != value.grant_revision
                || now >= input.received_at_ms.saturating_add(REPLY_TTL_MS)
            {
                input.state = if input.grant_revision != value.grant_revision {
                    InputState::Revoked
                } else {
                    InputState::Expired
                };
                save_input(&tx, &input)?;
                continue;
            }
            if selected.len() >= MAX_BATCH
                || text.len() + input.input.text.len() + usize::from(!text.is_empty()) > MAX_TEXT
            {
                break;
            }
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&input.input.text);
            selected.push(input);
        }
        if selected.is_empty() {
            tx.commit()?;
            return Ok(None);
        }
        let batch = Batch {
            id: BatchId::new(),
            channel_id: id,
            conversation_id: value.conversation_id,
            grant_revision: value.grant_revision,
            text,
            actor_turn_id: None,
            created_at_ms: now,
            expires_at_ms: selected[0].received_at_ms.saturating_add(REPLY_TTL_MS),
            state: BatchState::Prepared,
        };
        tx.execute("INSERT INTO channel_batches(id,channel_id,state,snapshot) VALUES (?1,?2,'prepared',?3)", (batch.id.to_string(), id.to_string(), serde_json::to_string(&batch)?))?;
        for mut input in selected {
            input.state = InputState::Batched;
            input.batch_id = Some(batch.id);
            save_input(&tx, &input)?;
        }
        tx.commit()?;
        Ok(Some(batch))
    }
    pub fn channel_actor_request(&self, batch: &Batch) -> Result<Option<ActorTurn>> {
        self.actor_duplicate(batch.conversation_id, batch.id.0)
    }
    pub fn channel_link(&mut self, id: BatchId, turn: &ActorTurn) -> Result<()> {
        let mut batch: Batch = one(
            &self.connection,
            "SELECT snapshot FROM channel_batches WHERE id=?1",
            id.to_string(),
        )?;
        ensure!(
            batch.conversation_id == turn.conversation_id && batch.id.0 == turn.request_id,
            "channel Actor identity mismatch"
        );
        if matches!(batch.state, BatchState::Prepared | BatchState::Running)
            && (batch.actor_turn_id != Some(turn.id) || batch.state != BatchState::Running)
        {
            batch.actor_turn_id = Some(turn.id);
            batch.state = BatchState::Running;
            save_batch(&self.connection, &batch)?;
        }
        Ok(())
    }
    pub fn channel_finish(&mut self, id: BatchId, turn: &ActorTurn) -> Result<()> {
        ensure!(turn.state != TurnState::Running, "Actor still running");
        let mut batch: Batch = one(
            &self.connection,
            "SELECT snapshot FROM channel_batches WHERE id=?1",
            id.to_string(),
        )?;
        ensure!(
            batch.conversation_id == turn.conversation_id && batch.id.0 == turn.request_id,
            "channel Actor identity mismatch"
        );
        if !matches!(batch.state, BatchState::Prepared | BatchState::Running) {
            return Ok(());
        }
        let now = now_ms()?;
        let value = binding(&self.connection, batch.channel_id)?;
        if !value.reply_enabled || value.grant_revision != batch.grant_revision {
            batch.state = BatchState::Revoked;
            return save_batch(&self.connection, &batch);
        }
        if now >= batch.expires_at_ms {
            batch.state = BatchState::Expired;
            return save_batch(&self.connection, &batch);
        }
        let text = match turn.state {
            TurnState::Completed if turn.private_chat.as_ref().is_some_and(|c|c.decision==lain_wire::chat::ChatDecision::NoReply) => String::new(),
            TurnState::Completed if turn.private_chat.as_ref().is_some_and(|c|c.decision==lain_wire::chat::ChatDecision::Superseded) => "补充消息已经收到，这轮还没能形成可靠回复。记录已保留；如果有必须处理的一项，请单独说明。".into(),
            TurnState::Completed => turn
                .messages
                .iter()
                .rev()
                .find(|m| m.role == MessageRole::Assistant)
                .map(|m| m.content.clone())
                .unwrap_or_default(),
            TurnState::PendingConfirmation => format!(
                "这项操作还没有执行，需要你在本地终端查看并确认。对话：{}；回合：{}。聊天中的“同意”不会自动批准操作。",
                turn.conversation_id, turn.id
            ),
            _ => "刚才的处理没有可靠完成。记录已保留，我不会自动重试可能产生影响的操作。".into(),
        };
        batch.state = BatchState::Finished;
        batch.actor_turn_id = Some(turn.id);
        let tx = self.connection.transaction()?;
        save_batch(&tx, &batch)?;
        if !text.is_empty() {
            let mut delivery = Delivery {
                id: DeliveryId::new(),
                channel_id: batch.channel_id,
                batch_id: batch.id,
                grant_revision: batch.grant_revision,
                text,
                state: DeliveryState::Ready,
                created_at_ms: now,
                expires_at_ms: batch.expires_at_ms,
                receipt: None,
                purpose: match turn.state {
                    TurnState::PendingConfirmation => DeliveryPurpose::Approval,
                    TurnState::Completed
                        if turn.private_chat.as_ref().is_none_or(|c| {
                            !matches!(
                                c.decision,
                                lain_wire::chat::ChatDecision::Notice
                                    | lain_wire::chat::ChatDecision::Superseded
                            )
                        }) =>
                    {
                        DeliveryPurpose::Conversation
                    }
                    _ => DeliveryPurpose::Failure,
                },
            };
            if !super::private_chat::delivery_current(&tx, &delivery)? {
                delivery.state = DeliveryState::Superseded;
            }
            tx.execute("INSERT INTO channel_deliveries(id,channel_id,batch_id,state,snapshot) VALUES (?1,?2,?3,?4,?5)", params![delivery.id.to_string(),delivery.channel_id.to_string(),batch.id.to_string(),state(&delivery.state)?,serde_json::to_string(&delivery)?])?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn channel_claim_delivery(&mut self, id: ChannelId) -> Result<Option<Delivery>> {
        let value = binding(&self.connection, id)?;
        let ready = rows::<Delivery>(
            &self.connection,
            "SELECT snapshot FROM channel_deliveries WHERE channel_id=?1 AND state='ready' ORDER BY sequence LIMIT 100",
            id.to_string(),
        )?;
        let now = now_ms()?;
        for mut delivery in ready {
            delivery.state =
                if !value.reply_enabled || value.grant_revision != delivery.grant_revision {
                    DeliveryState::Revoked
                } else if now >= delivery.expires_at_ms {
                    DeliveryState::Expired
                } else if !super::private_chat::delivery_current(&self.connection, &delivery)? {
                    DeliveryState::Superseded
                } else {
                    DeliveryState::Sending
                };
            save_delivery(&self.connection, &delivery)?;
            if delivery.state == DeliveryState::Sending {
                return Ok(Some(delivery));
            }
        }
        Ok(None)
    }
    pub fn channel_may_send(&self, delivery: &Delivery) -> Result<bool> {
        let saved: Delivery = one(
            &self.connection,
            "SELECT snapshot FROM channel_deliveries WHERE id=?1",
            delivery.id.to_string(),
        )?;
        let b = binding(&self.connection, delivery.channel_id)?;
        Ok(saved.state == DeliveryState::Sending
            && saved.channel_id == delivery.channel_id
            && saved.batch_id == delivery.batch_id
            && saved.text == delivery.text
            && saved.grant_revision == delivery.grant_revision
            && b.reply_enabled
            && b.grant_revision == delivery.grant_revision
            && now_ms()? < saved.expires_at_ms
            && super::private_chat::delivery_current(&self.connection, &saved)?)
    }
    pub fn channel_settle(&mut self, id: DeliveryId, outcome: SendOutcome) -> Result<()> {
        let mut value: Delivery = one(
            &self.connection,
            "SELECT snapshot FROM channel_deliveries WHERE id=?1",
            id.to_string(),
        )?;
        ensure!(
            value.state == DeliveryState::Sending,
            crate::backend::ControlError::Conflict
        );
        value.state = match outcome {
            SendOutcome::Confirmed(receipt) => {
                validate_reference(&receipt)?;
                value.receipt = Some(receipt);
                DeliveryState::Confirmed
            }
            SendOutcome::NotSent => DeliveryState::NotSent,
            SendOutcome::Rejected => DeliveryState::Rejected,
            SendOutcome::Unknown => DeliveryState::Unknown,
        };
        save_delivery(&self.connection, &value)
    }
    pub fn recover_channels(&mut self) -> Result<()> {
        let tx = self.connection.transaction()?;
        let mut query =
            tx.prepare("SELECT snapshot FROM channel_deliveries WHERE state='sending'")?;
        let values: Vec<Delivery> = query
            .query_map([], |r| r.get::<_, String>(0))?
            .map(|r| decode(r?))
            .collect::<Result<_>>()?;
        drop(query);
        for mut value in values {
            value.state = DeliveryState::Unknown;
            save_delivery(&tx, &value)?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn channel_snapshot(&self, id: ChannelId) -> Result<ChannelSnapshot> {
        let mut value = ChannelSnapshot {
            binding: binding(&self.connection, id)?,
            connection: ConnectionStatus::default(),
            incoming: rows(
                &self.connection,
                "SELECT snapshot FROM channel_incoming WHERE channel_id=?1 ORDER BY sequence DESC LIMIT 100",
                id.to_string(),
            )?,
            batches: rows(
                &self.connection,
                "SELECT snapshot FROM channel_batches WHERE channel_id=?1 ORDER BY sequence DESC LIMIT 100",
                id.to_string(),
            )?,
            deliveries: rows(
                &self.connection,
                "SELECT snapshot FROM channel_deliveries WHERE channel_id=?1 ORDER BY sequence DESC LIMIT 100",
                id.to_string(),
            )?,
        };
        value.incoming.reverse();
        value.batches.reverse();
        value.deliveries.reverse();
        Ok(value)
    }

    fn channel_expire(&mut self, id: ChannelId, now: u64) -> Result<()> {
        let tx = self.connection.transaction()?;
        for mut input in rows::<Incoming>(
            &tx,
            "SELECT snapshot FROM channel_incoming WHERE channel_id=?1 AND state='queued'",
            id.to_string(),
        )? {
            if now >= input.received_at_ms.saturating_add(REPLY_TTL_MS) {
                input.state = InputState::Expired;
                save_input(&tx, &input)?;
            }
        }
        for mut delivery in rows::<Delivery>(
            &tx,
            "SELECT snapshot FROM channel_deliveries WHERE channel_id=?1 AND state='ready'",
            id.to_string(),
        )? {
            if now >= delivery.expires_at_ms {
                delivery.state = DeliveryState::Expired;
                save_delivery(&tx, &delivery)?;
            }
        }
        tx.commit()?;
        Ok(())
    }
    /// Query pending work, never the bounded inspection page, to schedule a wake.
    pub fn channel_deadline(&self, id: ChannelId, now: u64) -> Result<Option<u64>> {
        let binding = binding(&self.connection, id)?;
        if !binding.reply_enabled {
            return Ok(None);
        }
        let batches = rows::<Batch>(
            &self.connection,
            "SELECT snapshot FROM channel_batches WHERE channel_id=?1 AND state IN ('prepared','running')",
            id.to_string(),
        )?;
        let busy = !batches.is_empty()
            || self
                .actor_recent(binding.conversation_id)?
                .iter()
                .any(|t| t.state.active());
        let mut deadlines: Vec<u64> = batches.iter().map(|b| b.expires_at_ms).collect();
        let pending = rows::<Incoming>(
            &self.connection,
            "SELECT snapshot FROM channel_incoming WHERE channel_id=?1 AND state='queued' ORDER BY sequence LIMIT 128",
            id.to_string(),
        )?;
        if let Some(input) = pending.first() {
            deadlines.push(if busy {
                input.received_at_ms.saturating_add(REPLY_TTL_MS)
            } else {
                merge_deadline(pending.iter().map(|i| i.received_at_ms))
                    .unwrap()
                    .max(now + 1)
            });
        }
        for delivery in rows::<Delivery>(
            &self.connection,
            "SELECT snapshot FROM channel_deliveries WHERE channel_id=?1 AND state='ready'",
            id.to_string(),
        )? {
            deadlines.push(delivery.expires_at_ms);
        }
        Ok(deadlines.into_iter().min())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_batch_and_outbox_commits_roll_back_all_linked_state() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("state.sqlite3")).unwrap();
        let binding = store
            .channel_bind(
                BindingSpec {
                    name: "fixture".into(),
                    adapter: "fixture".into(),
                    account: "a".into(),
                    peer: "b".into(),
                },
                true,
            )
            .unwrap();
        let incoming = store
            .channel_receive(
                binding.id,
                Input {
                    account: "a".into(),
                    peer: "b".into(),
                    external_id: "1".into(),
                    text: "atomic input".into(),
                },
            )
            .unwrap();
        let now = incoming.received_at_ms + MERGE_MS;
        store.connection.execute_batch("CREATE TRIGGER reject_batch_link BEFORE UPDATE ON channel_incoming WHEN NEW.state='batched' BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
        assert!(store.channel_next_batch(binding.id, now).is_err());
        let snapshot = store.channel_snapshot(binding.id).unwrap();
        assert!(snapshot.batches.is_empty());
        assert_eq!(snapshot.incoming[0].state, InputState::Queued);
        assert!(snapshot.incoming[0].batch_id.is_none());
        store
            .connection
            .execute_batch("DROP TRIGGER reject_batch_link;")
            .unwrap();
        let batch = store.channel_next_batch(binding.id, now).unwrap().unwrap();
        let mut turn = ActorTurn {
            reading_activity: None,
            id: crate::actor::domain::ActorTurnId::new(),
            conversation_id: binding.conversation_id,
            request_id: batch.id.0,
            sequence: 0,
            revision: 0,
            state: TurnState::Completed,
            messages: vec![],
            actions: vec![],
            error: None,
            model_requests: 1,
            usage_tokens: 1,
            timing: Default::default(),
            channel_memory: None,
            channel_source: None,
            private_chat: None,
        };
        turn.message(MessageRole::User, batch.text).unwrap();
        turn.message(MessageRole::Assistant, "atomic reply".into())
            .unwrap();
        store.save_actor_turn(&mut turn, true).unwrap();
        store.connection.execute_batch("CREATE TRIGGER reject_outbox BEFORE INSERT ON channel_deliveries BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
        assert!(store.channel_finish(batch.id, &turn).is_err());
        let snapshot = store.channel_snapshot(binding.id).unwrap();
        assert_eq!(snapshot.batches[0].state, BatchState::Prepared);
        assert!(snapshot.deliveries.is_empty());
        store
            .connection
            .execute_batch("DROP TRIGGER reject_outbox;")
            .unwrap();
        store.channel_finish(batch.id, &turn).unwrap();
        store.channel_finish(batch.id, &turn).unwrap();
        let snapshot = store.channel_snapshot(binding.id).unwrap();
        assert_eq!(snapshot.batches[0].state, BatchState::Finished);
        assert_eq!(snapshot.deliveries.len(), 1);
        assert_eq!(snapshot.deliveries[0].state, DeliveryState::Ready);
    }
}
