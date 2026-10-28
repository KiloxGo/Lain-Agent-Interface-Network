use super::{Store, channels::decode};
use crate::{actor::domain::*, backend::ControlError, domain::now_ms};
use anyhow::{Result, ensure};
use lain_wire::{InferenceBudgetId, inference::*};
use rusqlite::{Connection, OptionalExtension, Transaction};

fn decision(db: &Connection) -> Result<InferenceDecision> {
    let row = db
        .query_row(
            "SELECT snapshot FROM inference_decisions ORDER BY revision DESC LIMIT 1",
            [],
            |r| r.get::<_, String>(0),
        )
        .optional()?;
    Ok(row.map(decode).transpose()?.unwrap_or(InferenceDecision {
        revision: 0,
        enabled: false,
        limit: None,
        unlimited_private_channel: None,
        changed_at_ms: 0,
    }))
}
fn totals(db: &Connection, id: Option<InferenceBudgetId>) -> Result<(u64, u64)> {
    let(count,tokens):(i64,i64)=db.query_row("SELECT count(*),COALESCE(sum(json_extract(snapshot,'$.reported_usage_tokens')),0) FROM inference_requests WHERE budget_id IS ?1",[id.map(|id|id.to_string())],|r|Ok((r.get(0)?,r.get(1)?)))?;
    Ok((count.try_into()?, tokens.try_into()?))
}
fn reserved(db: &Connection, id: Option<InferenceBudgetId>) -> Result<u64> {
    Ok(db
        .query_row(
            "SELECT count(*) FROM inference_requests WHERE budget_id IS ?1",
            [id.map(|id| id.to_string())],
            |r| r.get::<_, i64>(0),
        )?
        .try_into()?)
}
fn admission(g: &InferenceDecision, used: u64, now: u64) -> InferenceAdmission {
    if g.revision == 0 {
        return InferenceAdmission::ExistingLimitsOnly;
    }
    if !g.enabled {
        return InferenceAdmission::Stopped;
    }
    if g.unlimited_private_channel.is_some() && g.limit.is_none() {
        return InferenceAdmission::PrivateConversationOnly;
    }
    let Some(s) = &g.limit else {
        return InferenceAdmission::Stopped;
    };
    if now >= s.expires_at_ms {
        InferenceAdmission::Expired
    } else if used >= u64::from(s.max_requests) {
        InferenceAdmission::Exhausted
    } else {
        InferenceAdmission::Available
    }
}
fn private_daily_allowed(db: &Connection, g: &InferenceDecision, t: &ActorTurn) -> Result<bool> {
    if !(g.enabled
        && g.limit.is_none()
        && t.reading_activity.is_none()
        && t.sharing_activity.is_none()
        && t.private_chat
            .as_ref()
            .is_some_and(|c| Some(c.channel_id) == g.unlimited_private_channel))
    {
        return Ok(false);
    }
    let c = t
        .private_chat
        .as_ref()
        .ok_or(ControlError::InferenceDenied)?;
    let b = super::channels::binding(db, c.channel_id)?;
    Ok(b.reply_enabled
        && b.grant_revision == c.grant_revision
        && b.conversation_id == t.conversation_id
        && now_ms()? < c.expires_at_ms)
}
fn read(db: &Connection, id: ActorTurnId, n: u32) -> Result<InferenceRequest> {
    let row = db
        .query_row(
            "SELECT snapshot FROM inference_requests WHERE actor_turn_id=?1 AND request_number=?2",
            (id.to_string(), n),
            |r| r.get::<_, String>(0),
        )
        .optional()?
        .ok_or(ControlError::NotFound)?;
    decode(row)
}
fn state(s: InferenceRequestState) -> Result<String> {
    Ok(serde_json::to_value(s)?
        .as_str()
        .ok_or(ControlError::InvalidInput)?
        .into())
}
fn save(db: &Connection, r: &InferenceRequest) -> Result<()> {
    ensure!(db.execute("UPDATE inference_requests SET state=?3,snapshot=?4 WHERE actor_turn_id=?1 AND request_number=?2",(r.actor_turn_id.to_string(),r.request_number,state(r.state)?,serde_json::to_string(r)?))?==1,ControlError::NotFound);
    Ok(())
}
fn pending(db: &Connection) -> Result<u64> {
    Ok(db
        .query_row(
            "SELECT count(*) FROM inference_requests WHERE state IN ('prepared','attempting')",
            [],
            |r| r.get::<_, i64>(0),
        )?
        .try_into()?)
}

pub(super) fn sync_actor(tx: &Transaction<'_>, t: &ActorTurn, create: bool) -> Result<()> {
    if create {
        return Ok(());
    }
    let old = super::decode_actor(&tx.query_row(
        "SELECT snapshot FROM lain_actor_turns WHERE id=?1",
        [t.id.to_string()],
        |r| r.get::<_, String>(0),
    )?)?;
    ensure!(
        t.model_requests >= old.model_requests && t.model_requests <= old.model_requests + 1,
        ControlError::Conflict
    );
    if t.model_requests > old.model_requests {
        ensure!(t.state == TurnState::Running, ControlError::Conflict);
        let g = decision(tx)?;
        let used = reserved(tx, g.limit.as_ref().map(|s| s.id))?;
        let now = now_ms()?;
        ensure!(
            matches!(
                admission(&g, used, now),
                InferenceAdmission::Available | InferenceAdmission::ExistingLimitsOnly
            ) || private_daily_allowed(tx, &g, t)?,
            ControlError::InferenceDenied
        );
        let reason = if t.reading_activity.is_some() {
            InferenceReason::Reading
        } else if t.sharing_activity.is_some() {
            InferenceReason::Sharing
        } else if t.private_chat.is_some() {
            InferenceReason::PrivateConversation
        } else {
            InferenceReason::Conversation
        };
        let r = InferenceRequest {
            actor_turn_id: t.id,
            request_number: t.model_requests,
            budget_id: g.limit.as_ref().map(|s| s.id),
            decision_revision: g.revision,
            reason,
            state: InferenceRequestState::Prepared,
            prepared_at_ms: now,
            attempted_at_ms: None,
            returned_at_ms: None,
            reported_usage_tokens: None,
        };
        tx.execute("INSERT INTO inference_requests(actor_turn_id,request_number,budget_id,state,snapshot) VALUES (?1,?2,?3,'prepared',?4)",(t.id.to_string(),r.request_number,r.budget_id.map(|id|id.to_string()),serde_json::to_string(&r)?))?;
    }
    if t.state != TurnState::Running {
        let rows = {
            let mut q=tx.prepare("SELECT snapshot FROM inference_requests WHERE actor_turn_id=?1 AND state IN ('prepared','attempting')")?;
            q.query_map([t.id.to_string()], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        for row in rows {
            let mut r: InferenceRequest = decode(row)?;
            r.state = if r.state == InferenceRequestState::Prepared {
                InferenceRequestState::NotAttempted
            } else {
                InferenceRequestState::Unknown
            };
            save(tx, &r)?;
        }
    }
    Ok(())
}

impl Store {
    pub fn inference_snapshot(&self) -> Result<InferenceSnapshot> {
        let g = decision(&self.connection)?;
        let (used, tokens) = totals(&self.connection, g.limit.as_ref().map(|s| s.id))?;
        Ok(InferenceSnapshot {
            coverage: "all_lain_actor_adapter_calls_not_worker_or_upstream_billing".into(),
            usage_precision: "adapter_normalized_tokens_zero_may_mean_unreported_fees_unknown"
                .into(),
            admission: admission(&g, used, now_ms()?),
            reserved_requests: used,
            remaining_requests: g
                .limit
                .as_ref()
                .map(|s| u64::from(s.max_requests).saturating_sub(used)),
            reported_usage_tokens_sum: tokens,
            pending_actor_requests: pending(&self.connection)?,
            recent_requests: self.inference_requests(None)?,
            decision: g,
        })
    }
    pub fn inference_requests(
        &self,
        id: Option<InferenceBudgetId>,
    ) -> Result<Vec<InferenceRequest>> {
        let mut q=self.connection.prepare("SELECT snapshot FROM inference_requests WHERE (?1 IS NULL OR budget_id=?1) ORDER BY sequence DESC LIMIT 100")?;
        q.query_map([id.map(|id| id.to_string())], |r| r.get::<_, String>(0))?
            .map(|row| decode(row?))
            .collect()
    }
    pub fn inference_budgets(&self) -> Result<Vec<InferenceBudgetStatus>> {
        let mut q = self
            .connection
            .prepare("SELECT snapshot FROM inference_budgets ORDER BY rowid DESC LIMIT 100")?;
        q.query_map([], |r| r.get::<_, String>(0))?
            .map(|row| {
                let limit: InferenceLimit = decode(row?)?;
                let (reserved_requests, reported_usage_tokens_sum) =
                    totals(&self.connection, Some(limit.id))?;
                Ok(InferenceBudgetStatus {
                    limit,
                    reserved_requests,
                    reported_usage_tokens_sum,
                })
            })
            .collect()
    }
    pub fn set_inference_budget(
        &mut self,
        q: SetInferenceBudget,
        now: u64,
    ) -> Result<InferenceDecision> {
        let old = decision(&self.connection)?;
        ensure!(q.expected_revision == old.revision, ControlError::Conflict);
        ensure!(
            q.limit.is_none() || q.unlimited_private_channel.is_none(),
            ControlError::InvalidInput
        );
        if let Some(channel) = q.unlimited_private_channel {
            self.channel_binding(channel)?;
        }
        if q.limit.is_some() || q.unlimited_private_channel.is_some() {
            ensure!(pending(&self.connection)? == 0, ControlError::Conflict);
        }
        let enabled = q.limit.is_some() || q.unlimited_private_channel.is_some();
        let limit = if enabled { q.limit } else { old.limit };
        let unlimited_private_channel = if enabled {
            q.unlimited_private_channel
        } else {
            old.unlimited_private_channel
        };
        let g = InferenceDecision {
            revision: old
                .revision
                .checked_add(1)
                .filter(|n| *n <= i64::MAX as u64)
                .ok_or(ControlError::Conflict)?,
            enabled,
            limit,
            unlimited_private_channel,
            changed_at_ms: now,
        };
        let tx = self.connection.transaction()?;
        if enabled && let Some(limit) = &g.limit {
            let existing = tx
                .query_row(
                    "SELECT snapshot FROM inference_budgets WHERE id=?1",
                    [limit.id.to_string()],
                    |r| r.get::<_, String>(0),
                )
                .optional()?;
            if let Some(existing) = existing {
                ensure!(
                    decode::<InferenceLimit>(existing)? == *limit,
                    ControlError::Conflict
                );
            } else {
                tx.execute(
                    "INSERT INTO inference_budgets VALUES (?1,?2)",
                    (limit.id.to_string(), serde_json::to_string(limit)?),
                )?;
            }
        }
        tx.execute(
            "INSERT INTO inference_decisions VALUES (?1,?2)",
            (i64::try_from(g.revision)?, serde_json::to_string(&g)?),
        )?;
        tx.commit()?;
        Ok(g)
    }
    pub fn begin_inference(&mut self, id: ActorTurnId, n: u32) -> Result<()> {
        let t = self.actor_turn(id)?;
        ensure!(
            super::archive_recall::selected_current(&self.connection, &t)?,
            super::private_chat::ChangedChatMemory
        );
        ensure!(
            t.state == TurnState::Running && t.model_requests == n,
            ControlError::Conflict
        );
        let mut r = read(&self.connection, id, n)?;
        ensure!(
            r.state == InferenceRequestState::Prepared,
            ControlError::Conflict
        );
        let g = decision(&self.connection)?;
        ensure!(
            g.revision == r.decision_revision,
            ControlError::InferenceDenied
        );
        if g.revision > 0 && !private_daily_allowed(&self.connection, &g, &t)? {
            let s = g
                .limit
                .as_ref()
                .filter(|_| g.enabled)
                .ok_or(ControlError::InferenceDenied)?;
            ensure!(
                Some(s.id) == r.budget_id
                    && now_ms()? < s.expires_at_ms
                    && reserved(&self.connection, Some(s.id))? <= u64::from(s.max_requests),
                ControlError::InferenceDenied
            );
        }
        r.state = InferenceRequestState::Attempting;
        r.attempted_at_ms = Some(now_ms()?);
        save(&self.connection, &r)
    }
    pub fn inference_returned(&mut self, id: ActorTurnId, n: u32, tokens: u64) -> Result<()> {
        ensure!(tokens <= i64::MAX as u64, ControlError::InvalidInput);
        let mut r = read(&self.connection, id, n)?;
        ensure!(
            matches!(
                r.state,
                InferenceRequestState::Attempting | InferenceRequestState::Unknown
            ) && r.attempted_at_ms.is_some(),
            ControlError::Conflict
        );
        r.state = InferenceRequestState::Returned;
        r.returned_at_ms = Some(now_ms()?);
        r.reported_usage_tokens = Some(tokens);
        save(&self.connection, &r)
    }
    pub fn recover_inference(&mut self) -> Result<()> {
        let tx = self.connection.transaction()?;
        let rows = {
            let mut q = tx.prepare(
                "SELECT snapshot FROM inference_requests WHERE state IN ('prepared','attempting')",
            )?;
            q.query_map([], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        for row in rows {
            let mut r: InferenceRequest = decode(row)?;
            r.state = if r.state == InferenceRequestState::Prepared {
                InferenceRequestState::NotAttempted
            } else {
                InferenceRequestState::Unknown
            };
            save(&tx, &r)?;
        }
        tx.commit()?;
        Ok(())
    }
}
