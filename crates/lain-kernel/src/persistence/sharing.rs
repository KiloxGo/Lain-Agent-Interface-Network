use super::{
    Store,
    channels::{binding, decode, one, rows},
};
use crate::{actor::domain::*, backend::ControlError, domain::now_ms, redaction::Redactor};
use anyhow::{Result, ensure};
use lain_wire::{ChannelId, DeliveryId, ExperienceId, ShareId, sharing::*, source::DeliveryState};
use rusqlite::{Connection, OptionalExtension, Transaction};
use serde_json::{Value, json};

fn state(s: &impl serde::Serialize) -> Result<String> {
    Ok(serde_json::to_value(s)?
        .as_str()
        .ok_or(ControlError::InvalidInput)?
        .into())
}

fn candidate(db: &Connection, id: ShareId) -> Result<ShareCandidate> {
    one(
        db,
        "SELECT snapshot FROM share_candidates WHERE id=?1",
        id.to_string(),
    )
}
fn save(db: &Connection, c: &ShareCandidate) -> Result<()> {
    db.execute(
        "UPDATE share_candidates SET state=?2,snapshot=?3 WHERE id=?1",
        (
            c.id.to_string(),
            state(&c.state)?,
            serde_json::to_string(c)?,
        ),
    )?;
    Ok(())
}
fn save_delivery(db: &Connection, d: &ShareDelivery) -> Result<()> {
    db.execute(
        "UPDATE share_deliveries SET state=?2,snapshot=?3 WHERE id=?1",
        (
            d.id.to_string(),
            state(&d.state)?,
            serde_json::to_string(d)?,
        ),
    )?;
    Ok(())
}
fn grant(db: &Connection, id: ChannelId) -> Result<ShareGrant> {
    let s = db
        .query_row(
            "SELECT snapshot FROM share_grants WHERE channel_id=?1 ORDER BY revision DESC LIMIT 1",
            [id.to_string()],
            |r| r.get::<_, String>(0),
        )
        .optional()?;
    s.map(decode).transpose().map(|g| {
        g.unwrap_or(ShareGrant {
            channel_id: id,
            revision: 0,
            scope: None,
            actor_watermark: 0,
            changed_at_ms: 0,
        })
    })
}
fn attention(db: &Connection, id: ChannelId) -> Result<Attention> {
    let s = db
        .query_row(
            "SELECT snapshot FROM channel_attention WHERE channel_id=?1",
            [id.to_string()],
            |r| r.get::<_, String>(0),
        )
        .optional()?;
    s.map(decode).transpose().map(|a| {
        a.unwrap_or(Attention {
            channel_id: id,
            revision: 0,
            do_not_disturb: true,
            until_ms: None,
            changed_at_ms: 0,
        })
    })
}
fn watermark(db: &Connection) -> Result<u64> {
    Ok(db
        .query_row(
            "SELECT COALESCE(MAX(sequence),0) FROM lain_actor_turns",
            [],
            |r| r.get::<_, i64>(0),
        )?
        .try_into()?)
}
fn input_mark(db: &Connection, id: ChannelId) -> Result<(u64, u64)> {
    let s:Option<(i64,String)>=db.query_row("SELECT sequence,snapshot FROM channel_incoming WHERE channel_id=?1 ORDER BY sequence DESC LIMIT 1",[id.to_string()],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
    s.map(|(n, s)| {
        Ok((
            n.try_into()?,
            decode::<crate::channels::domain::Incoming>(s)?.received_at_ms,
        ))
    })
    .transpose()
    .map(|v| v.unwrap_or((0, 0)))
}
fn digest(e: &lain_wire::reading::ReadingExperience) -> Result<String> {
    Ok(lain_wire::actor_configuration_revision(
        serde_json::to_string(e)?.as_bytes(),
    ))
}
fn current(db: &Connection, c: &ShareCandidate, now: u64) -> Result<bool> {
    let g = grant(db, c.channel_id)?;
    let Some(s) = g.scope.as_ref() else {
        return Ok(false);
    };
    let e = super::experiences::read_experience(db, c.experience_id)?;
    Ok(g.active(now)
        && g.revision == c.grant_revision
        && now < c.expires_at_ms
        && s.urls.contains(&e.source_url)
        && e.revision == c.experience_revision
        && digest(&e)? == c.experience_fingerprint
        && now < e.acquired_at_ms.saturating_add(s.source_max_age_ms)
        && super::experiences::visible(db, &e, Some(c.channel_id), now)?)
}
/// None means an event (new input/config/approval resolution) is needed, not polling.
fn contact_due(
    db: &Connection,
    id: ChannelId,
    now: u64,
    exclude: Option<DeliveryId>,
) -> Result<Option<u64>> {
    let g = grant(db, id)?;
    let Some(s) = g.scope else {
        return Ok(None);
    };
    if now >= s.expires_at_ms || !binding(db, id)?.reply_enabled {
        return Ok(None);
    }
    let a = attention(db, id)?;
    if a.quiet(now) {
        return Ok(a.until_ms.filter(|t| *t > now));
    }
    let b = binding(db, id)?;
    let busy:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM channel_incoming WHERE channel_id=?1 AND state='queued') OR EXISTS(SELECT 1 FROM channel_batches WHERE channel_id=?1 AND state IN ('prepared','running')) OR EXISTS(SELECT 1 FROM lain_actor_turns WHERE conversation_id=?2 AND json_extract(snapshot,'$.state') IN ('running','pending_confirmation'))",(id.to_string(),b.conversation_id.to_string()),|r|r.get(0))?;
    if busy {
        return Ok(None);
    }
    let (input, received) = input_mark(db, id)?;
    let mut due = now.max(received.saturating_add(s.user_quiet_ms));
    let (.., today, tomorrow) = crate::reading::day_bounds(now, s.utc_offset_minutes);
    // Enforcement covers all persisted attempts, independently of the inspection page.
    let (total,day,last,unanswered):(i64,i64,Option<i64>,bool)=db.query_row(
        "SELECT COALESCE(sum(json_extract(snapshot,'$.grant_revision')=?2),0),COALESCE(sum(json_extract(snapshot,'$.created_at_ms')>=?3),0),max(json_extract(snapshot,'$.created_at_ms')),COALESCE(max(state='sending' OR (state IN ('confirmed','unknown') AND json_extract(snapshot,'$.input_watermark')>=?4)),0) FROM share_deliveries WHERE channel_id=?1 AND (?5 IS NULL OR id<>?5)",
        rusqlite::params![id.to_string(),i64::try_from(g.revision)?,i64::try_from(today.max(0))?,i64::try_from(input)?,exclude.map(|id|id.to_string())],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;
    if unanswered || total >= i64::from(s.max_send_attempts) {
        return Ok(None);
    }
    if let Some(last) = last {
        due = due.max(u64::try_from(last)?.saturating_add(s.cooldown_ms));
    }
    if day >= i64::from(s.max_per_day) {
        due = due.max(tomorrow.try_into()?);
    }
    let offset = i128::from(s.utc_offset_minutes) * 60000;
    // At most eight explicit windows; jump to their ends rather than tick polling.
    for _ in 0..=s.quiet_windows.len() * 2 {
        let local = i128::from(due) + offset;
        let minute = local.rem_euclid(86400000) / 60000;
        let mut end = None;
        for w in &s.quiet_windows {
            let from = i128::from(w.from_minute);
            let to = i128::from(w.to_minute);
            if (from < to && minute >= from && minute < to)
                || (from > to && (minute >= from || minute < to))
            {
                let day = local.div_euclid(86400000) * 86400000;
                let t = day
                    + to * 60000
                    + if from > to && minute >= from {
                        86400000
                    } else {
                        0
                    };
                end = Some(end.map_or(t, |old: i128| old.max(t)));
            }
        }
        if let Some(end) = end {
            due = (end - offset).try_into()?;
        } else {
            return Ok(Some(due));
        }
    }
    Ok(None)
}

pub(super) fn sync_actor(tx: &Transaction<'_>, t: &ActorTurn) -> Result<()> {
    let Some(id) = t.sharing_activity else {
        return Ok(());
    };
    let mut c = candidate(tx, id)?;
    ensure!(
        c.actor_turn_id == Some(t.id) && t.actions.is_empty(),
        ControlError::Conflict
    );
    let previous = super::decode_actor(&tx.query_row(
        "SELECT snapshot FROM lain_actor_turns WHERE id=?1",
        [t.id.to_string()],
        |r| r.get::<_, String>(0),
    )?)?;
    let growing = t.model_requests > previous.model_requests;
    let body = t
        .messages
        .iter()
        .filter(|m| m.role == MessageRole::Assistant)
        .count()
        > previous
            .messages
            .iter()
            .filter(|m| m.role == MessageRole::Assistant)
            .count();
    if growing
        || body
        || (t.state == TurnState::Completed && previous.state != TurnState::Completed)
    {
        ensure!(
            c.state == ShareState::Drafting
                && current(tx, &c, now_ms()?)?
                && input_mark(tx, c.channel_id)?.0 == c.input_watermark,
            ControlError::Conflict
        );
        if growing {
            let g = grant(tx, c.channel_id)?;
            let s = g.scope.ok_or(ControlError::Conflict)?;
            let used:i64=tx.query_row("SELECT count(*) FROM share_model_requests WHERE channel_id=?1 AND grant_revision=?2",(c.channel_id.to_string(),i64::try_from(c.grant_revision)?),|r|r.get(0))?;
            ensure!(
                t.model_requests == previous.model_requests + 1
                    && used < i64::from(s.max_model_requests),
                ControlError::Conflict
            );
            tx.execute(
                "INSERT INTO share_model_requests VALUES (?1,?2,?3,?4,?5)",
                (
                    t.id.to_string(),
                    t.model_requests,
                    c.channel_id.to_string(),
                    i64::try_from(c.grant_revision)?,
                    i64::try_from(now_ms()?)?,
                ),
            )?;
        }
        if t.state == TurnState::Completed && previous.state != TurnState::Completed {
            let text = &t
                .messages
                .iter()
                .rev()
                .find(|m| m.role == MessageRole::Assistant)
                .ok_or(ControlError::InvalidInput)?
                .content;
            let d: ShareDecision =
                serde_json::from_str(text).map_err(|_| ControlError::InvalidInput)?;
            ensure!(
                !d.reason.trim().is_empty()
                    && d.reason.len() <= 512
                    && d.text.len() <= 2048
                    && match d.decision {
                        ShareChoice::Share => !d.text.trim().is_empty(),
                        ShareChoice::Skip => d.text.is_empty(),
                    },
                ControlError::InvalidInput
            );
            c.state = if d.decision == ShareChoice::Share {
                ShareState::Ready
            } else {
                ShareState::Skipped
            };
            c.reason = Some(d.reason);
            c.text = Some(d.text);
            save(tx, &c)?;
        }
    }
    Ok(())
}

impl Store {
    pub fn share_grant(&self, id: ChannelId) -> Result<ShareGrant> {
        binding(&self.connection, id)?;
        grant(&self.connection, id)
    }
    pub fn set_share_grant(
        &mut self,
        id: ChannelId,
        q: SetShareGrant,
        now: u64,
    ) -> Result<ShareGrant> {
        binding(&self.connection, id)?;
        let old = grant(&self.connection, id)?;
        ensure!(old.revision == q.expected_revision, ControlError::Conflict);
        let g = ShareGrant {
            channel_id: id,
            revision: old
                .revision
                .checked_add(1)
                .filter(|n| *n <= i64::MAX as u64)
                .ok_or(ControlError::Conflict)?,
            scope: q.scope,
            actor_watermark: watermark(&self.connection)?,
            changed_at_ms: now,
        };
        let tx = self.connection.transaction()?;
        tx.execute(
            "INSERT INTO share_grants VALUES (?1,?2,?3)",
            (
                id.to_string(),
                i64::try_from(g.revision)?,
                serde_json::to_string(&g)?,
            ),
        )?;
        for mut c in rows::<ShareCandidate>(
            &tx,
            "SELECT snapshot FROM share_candidates WHERE channel_id=?1 AND state IN ('prepared','drafting','ready')",
            id.to_string(),
        )? {
            c.state = ShareState::Revoked;
            c.text = None;
            c.reason = Some("grant_replaced".into());
            save(&tx, &c)?;
        }
        tx.commit()?;
        Ok(g)
    }
    pub fn attention(&self, id: ChannelId) -> Result<Attention> {
        binding(&self.connection, id)?;
        attention(&self.connection, id)
    }
    pub fn set_attention(&mut self, id: ChannelId, q: SetAttention, now: u64) -> Result<Attention> {
        binding(&self.connection, id)?;
        let old = attention(&self.connection, id)?;
        ensure!(old.revision == q.expected_revision, ControlError::Conflict);
        let a = Attention {
            channel_id: id,
            revision: old.revision.checked_add(1).ok_or(ControlError::Conflict)?,
            do_not_disturb: q.do_not_disturb,
            until_ms: q.until_ms,
            changed_at_ms: now,
        };
        self.connection.execute("INSERT INTO channel_attention VALUES (?1,?2) ON CONFLICT(channel_id) DO UPDATE SET snapshot=excluded.snapshot",(id.to_string(),serde_json::to_string(&a)?))?;
        Ok(a)
    }
    pub fn share_consider(
        &mut self,
        id: ChannelId,
        experience: ExperienceId,
        now: u64,
    ) -> Result<ShareCandidate> {
        let e = super::experiences::read_experience(&self.connection, experience)?;
        if let Some(s)=self.connection.query_row("SELECT snapshot FROM share_candidates WHERE channel_id=?1 AND experience_id=?2 AND experience_revision=?3",(id.to_string(),experience.to_string(),i64::try_from(e.revision)?),|r|r.get::<_,String>(0)).optional()? {return decode(s);}
        let g = grant(&self.connection, id)?;
        let s = g.scope.as_ref().ok_or(ControlError::Unavailable)?;
        ensure!(
            g.active(now)
                && s.urls.contains(&e.source_url)
                && now < e.acquired_at_ms.saturating_add(s.source_max_age_ms)
                && now < e.completed_at_ms.saturating_add(s.candidate_ttl_ms)
                && super::experiences::visible(&self.connection, &e, Some(id), now)?,
            ControlError::Conflict
        );
        let c = ShareCandidate {
            id: ShareId::new(),
            channel_id: id,
            experience_id: experience,
            experience_revision: e.revision,
            experience_fingerprint: digest(&e)?,
            grant_revision: g.revision,
            state: ShareState::Prepared,
            actor_turn_id: None,
            input_watermark: input_mark(&self.connection, id)?.0,
            created_at_ms: now,
            expires_at_ms: g
                .scope
                .as_ref()
                .unwrap()
                .expires_at_ms
                .min(e.completed_at_ms.saturating_add(s.candidate_ttl_ms))
                .min(e.acquired_at_ms.saturating_add(s.source_max_age_ms)),
            reason: None,
            text: None,
        };
        self.connection.execute("INSERT INTO share_candidates(id,channel_id,experience_id,experience_revision,state,snapshot) VALUES (?1,?2,?3,?4,'prepared',?5)",(c.id.to_string(),id.to_string(),experience.to_string(),i64::try_from(e.revision)?,serde_json::to_string(&c)?))?;
        Ok(c)
    }
    pub fn share_refresh(&mut self, id: ChannelId, now: u64) -> Result<()> {
        let g = grant(&self.connection, id)?;
        let tx = self.connection.transaction()?;
        for mut c in rows::<ShareCandidate>(
            &tx,
            "SELECT snapshot FROM share_candidates WHERE channel_id=?1 AND state IN ('prepared','drafting','ready')",
            id.to_string(),
        )? {
            let terminal = if now >= c.expires_at_ms {
                Some(ShareState::Expired)
            } else if !g.active(now) || g.revision != c.grant_revision {
                Some(ShareState::Revoked)
            } else if !current(&tx, &c, now)? || input_mark(&tx, id)?.0 != c.input_watermark {
                Some(ShareState::Superseded)
            } else if let Some(a) = c.actor_turn_id {
                let t: ActorTurn = super::decode_actor(&tx.query_row(
                    "SELECT snapshot FROM lain_actor_turns WHERE id=?1",
                    [a.to_string()],
                    |r| r.get::<_, String>(0),
                )?)?;
                if matches!(
                    t.state,
                    TurnState::Unknown | TurnState::Failed | TurnState::Cancelled
                ) {
                    Some(ShareState::Unknown)
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(state) = terminal {
                c.state = state;
                c.text = None;
                c.reason = Some("candidate_no_longer_current".into());
                save(&tx, &c)?;
            }
        }
        tx.commit()?;
        if !g.active(now) {
            return Ok(());
        }
        let boot: i64 = self.connection.query_row(
            "SELECT actor_watermark FROM sharing_runtime WHERE singleton=1",
            [],
            |r| r.get(0),
        )?;
        let scope = g.scope.as_ref().ok_or(ControlError::Conflict)?;
        let ids = {
            let mut q=self.connection.prepare("SELECT e.id FROM reading_experiences e JOIN lain_actor_turns a ON a.id=json_extract(e.snapshot,'$.actor_turn_id') WHERE e.active=1 AND a.sequence>?1 AND json_extract(e.snapshot,'$.source_url') IN (SELECT value FROM json_each(?3)) AND e.completed_at_ms>?4 AND json_extract(e.snapshot,'$.acquired_at_ms')>?5 AND NOT EXISTS(SELECT 1 FROM share_candidates c WHERE c.channel_id=?2 AND c.experience_id=e.id AND c.experience_revision=json_extract(e.snapshot,'$.revision')) ORDER BY e.completed_at_ms,e.id LIMIT 100")?;
            q.query_map(
                (
                    boot.max(i64::try_from(g.actor_watermark)?),
                    id.to_string(),
                    serde_json::to_string(&scope.urls)?,
                    i64::try_from(now.saturating_sub(scope.candidate_ttl_ms))?,
                    i64::try_from(now.saturating_sub(scope.source_max_age_ms))?,
                ),
                |r| r.get::<_, String>(0),
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?
        };
        for experience in ids {
            match self.share_consider(id, experience.parse()?, now) {
                Ok(_) => {}
                Err(e)
                    if matches!(
                        e.downcast_ref::<ControlError>(),
                        Some(ControlError::Conflict | ControlError::Unavailable)
                    ) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
    pub fn share_next_job(&mut self, id: ChannelId, now: u64) -> Result<Option<ShareId>> {
        self.share_refresh(id, now)?;
        if !contact_due(&self.connection, id, now, None)?.is_some_and(|due| due <= now) {
            return Ok(None);
        }
        Ok(rows::<ShareCandidate>(&self.connection,"SELECT snapshot FROM share_candidates WHERE channel_id=?1 AND state='prepared' ORDER BY sequence LIMIT 1",id.to_string())?.first().map(|c|c.id))
    }
    pub fn create_share_job(
        &mut self,
        id: ShareId,
        redactor: &Redactor,
    ) -> Result<(ActorTurn, bool)> {
        let mut c = candidate(&self.connection, id)?;
        if let Some(actor) = c.actor_turn_id {
            return Ok((self.actor_turn(actor)?, false));
        }
        let now = now_ms()?;
        ensure!(
            c.state == ShareState::Prepared
                && current(&self.connection, &c, now)?
                && contact_due(&self.connection, c.channel_id, now, None)?
                    .is_some_and(|t| t <= now),
            ControlError::Conflict
        );
        let busy: bool = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM share_candidates WHERE state='drafting')",
            [],
            |r| r.get(0),
        )?;
        ensure!(!busy, ControlError::Conflict);
        let g = grant(&self.connection, c.channel_id)?;
        let used: i64 = self.connection.query_row(
            "SELECT count(*) FROM share_model_requests WHERE channel_id=?1 AND grant_revision=?2",
            (c.channel_id.to_string(), i64::try_from(g.revision)?),
            |r| r.get(0),
        )?;
        ensure!(
            used < i64::from(g.scope.ok_or(ControlError::Unavailable)?.max_model_requests),
            ControlError::Conflict
        );
        let e = super::experiences::read_experience(&self.connection, c.experience_id)?;
        ensure!(
            redactor.text(&serde_json::to_string(&e)?) == serde_json::to_string(&e)?,
            ControlError::InvalidInput
        );
        let mut t = ActorTurn {
            id: ActorTurnId::new(),
            reading_activity: None,
            sharing_activity: Some(id),
            conversation_id: ConversationId::new(),
            request_id: id.0,
            sequence: 0,
            revision: 0,
            state: TurnState::Running,
            messages: vec![],
            actions: vec![],
            error: None,
            model_requests: 0,
            usage_tokens: 0,
            timing: Default::default(),
            channel_memory: None,
            channel_source: None,
            private_chat: None,
        };
        t.message(MessageRole::User,format!("Kernel share-draft activity {id}: consider the supplied reading experience and return a bounded share-or-skip decision. This is not a human message or permission to send."))?;
        let tx = self.connection.transaction()?;
        tx.execute(
            "INSERT INTO lain_conversations VALUES (?1,?2,?2)",
            (t.conversation_id.to_string(), i64::try_from(now)?),
        )?;
        c.state = ShareState::Drafting;
        c.actor_turn_id = Some(t.id);
        save(&tx, &c)?;
        // First persist the Actor before its foreign-key link, within this same commit.
        let activity = t.sharing_activity.take();
        Self::save_actor_in_transaction(&tx, &mut t, true)?;
        tx.execute(
            "INSERT INTO share_jobs VALUES (?1,?2)",
            (id.to_string(), t.id.to_string()),
        )?;
        t.sharing_activity = activity;
        Self::save_actor_in_transaction(&tx, &mut t, false)?;
        tx.commit()?;
        Ok((t, true))
    }
    pub fn share_actor_input(&self, id: ActorTurnId, redactor: &Redactor) -> Result<Value> {
        let t = self.actor_turn(id)?;
        let c = candidate(
            &self.connection,
            t.sharing_activity.ok_or(ControlError::InvalidInput)?,
        )?;
        ensure!(
            c.actor_turn_id == Some(id)
                && c.state == ShareState::Drafting
                && current(&self.connection, &c, now_ms()?)?
                && input_mark(&self.connection, c.channel_id)?.0 == c.input_watermark,
            ControlError::Conflict
        );
        let e = super::experiences::read_experience(&self.connection, c.experience_id)?;
        let encoded = serde_json::to_string(&e)?;
        ensure!(
            redactor.text(&encoded) == encoded,
            ControlError::InvalidInput
        );
        Ok(
            json!({"role":"untrusted_source_linked_experience","candidate_id":c.id,"expires_at_ms":c.expires_at_ms,"experience":super::experiences::project(&e),"independent_verification":false}),
        )
    }
    pub fn share_fail_start(&mut self, id: ShareId) -> Result<()> {
        let mut c = candidate(&self.connection, id)?;
        if c.state == ShareState::Prepared {
            c.state = ShareState::Failed;
            c.reason = Some("draft_not_started".into());
            save(&self.connection, &c)?;
        }
        Ok(())
    }
    pub fn share_claim(&mut self, id: ChannelId, now: u64) -> Result<Option<ShareDelivery>> {
        self.share_refresh(id, now)?;
        if !contact_due(&self.connection, id, now, None)?.is_some_and(|due| due <= now) {
            return Ok(None);
        }
        let Some(mut c)=rows::<ShareCandidate>(&self.connection,"SELECT snapshot FROM share_candidates WHERE channel_id=?1 AND state='ready' ORDER BY sequence LIMIT 1",id.to_string())?.into_iter().next() else{return Ok(None);};
        let a = attention(&self.connection, id)?;
        let d = ShareDelivery {
            id: DeliveryId(c.id.0),
            candidate_id: c.id,
            channel_id: id,
            grant_revision: c.grant_revision,
            attention_revision: a.revision,
            input_watermark: input_mark(&self.connection, id)?.0,
            text: c.text.clone().ok_or(ControlError::Conflict)?,
            state: DeliveryState::Sending,
            created_at_ms: now,
            expires_at_ms: c.expires_at_ms,
            receipt: None,
        };
        let tx = self.connection.transaction()?;
        c.state = ShareState::Sending;
        save(&tx, &c)?;
        tx.execute("INSERT INTO share_deliveries(id,candidate_id,channel_id,state,snapshot) VALUES (?1,?2,?3,'sending',?4)",(d.id.to_string(),c.id.to_string(),id.to_string(),serde_json::to_string(&d)?))?;
        tx.commit()?;
        Ok(Some(d))
    }
    pub fn share_may_send(&self, d: &ShareDelivery, now: u64) -> Result<bool> {
        let saved: ShareDelivery = one(
            &self.connection,
            "SELECT snapshot FROM share_deliveries WHERE id=?1",
            d.id.to_string(),
        )?;
        let c = candidate(&self.connection, d.candidate_id)?;
        Ok(saved == *d
            && saved.state == DeliveryState::Sending
            && c.state == ShareState::Sending
            && current(&self.connection, &c, now)?
            && input_mark(&self.connection, d.channel_id)?.0 == d.input_watermark
            && attention(&self.connection, d.channel_id)?.revision == d.attention_revision
            && contact_due(&self.connection, d.channel_id, now, Some(d.id))?
                .is_some_and(|due| due <= now))
    }
    pub fn share_settle(
        &mut self,
        id: DeliveryId,
        outcome: crate::channels::domain::SendOutcome,
    ) -> Result<()> {
        use crate::channels::domain::SendOutcome;
        let mut d: ShareDelivery = one(
            &self.connection,
            "SELECT snapshot FROM share_deliveries WHERE id=?1",
            id.to_string(),
        )?;
        ensure!(d.state == DeliveryState::Sending, ControlError::Conflict);
        let mut c = candidate(&self.connection, d.candidate_id)?;
        (d.state, c.state) = match outcome {
            SendOutcome::Confirmed(r) => {
                crate::channels::domain::validate_reference(&r)?;
                d.receipt = Some(r);
                (DeliveryState::Confirmed, ShareState::Confirmed)
            }
            SendOutcome::Unknown => (DeliveryState::Unknown, ShareState::Unknown),
            SendOutcome::NotSent => (DeliveryState::NotSent, ShareState::NotSent),
            SendOutcome::Rejected => (DeliveryState::Rejected, ShareState::Rejected),
        };
        let tx = self.connection.transaction()?;
        save_delivery(&tx, &d)?;
        save(&tx, &c)?;
        tx.commit()?;
        Ok(())
    }
    pub fn share_deadline(&self, id: ChannelId, now: u64) -> Result<Option<u64>> {
        let rows = rows::<ShareCandidate>(
            &self.connection,
            "SELECT snapshot FROM share_candidates WHERE channel_id=?1 AND state IN ('prepared','drafting','ready')",
            id.to_string(),
        )?;
        let mut due = rows.iter().map(|c| c.expires_at_ms).min();
        if !rows.is_empty()
            && let Some(contact) = contact_due(&self.connection, id, now, None)?
            && contact > now
        {
            due = Some(due.map_or(contact, |d| d.min(contact)));
        }
        Ok(due)
    }
    pub fn sharing_snapshot(&self, id: ChannelId) -> Result<SharingSnapshot> {
        binding(&self.connection, id)?;
        Ok(SharingSnapshot {
            model_requests_reserved:self.connection.query_row("SELECT count(*) FROM share_model_requests WHERE channel_id=?1 AND grant_revision=?2",(id.to_string(),i64::try_from(grant(&self.connection,id)?.revision)?),|r|r.get::<_,i64>(0))?.try_into()?,
            send_attempts_reserved:self.connection.query_row("SELECT count(*) FROM share_deliveries WHERE channel_id=?1 AND json_extract(snapshot,'$.grant_revision')=?2",(id.to_string(),i64::try_from(grant(&self.connection,id)?.revision)?),|r|r.get::<_,i64>(0))?.try_into()?,
            grant: self.share_grant(id)?,
            attention: self.attention(id)?,
            candidates: rows(
                &self.connection,
                "SELECT snapshot FROM share_candidates WHERE channel_id=?1 ORDER BY sequence DESC LIMIT 100",
                id.to_string(),
            )?,
            deliveries: rows(
                &self.connection,
                "SELECT snapshot FROM share_deliveries WHERE channel_id=?1 ORDER BY sequence DESC LIMIT 100",
                id.to_string(),
            )?,
        })
    }
    pub fn recover_sharing(&mut self) -> Result<()> {
        let tx = self.connection.transaction()?;
        let pending = {
            let mut q=tx.prepare("SELECT snapshot FROM share_candidates WHERE state IN ('prepared','drafting','ready','sending')")?;
            q.query_map([], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        for s in pending {
            let mut c: ShareCandidate = decode(s)?;
            c.state = if c.state == ShareState::Sending {
                ShareState::Unknown
            } else {
                ShareState::Expired
            };
            if c.state == ShareState::Expired {
                c.text = None;
            }
            c.reason = Some("restart_no_catchup".into());
            save(&tx, &c)?;
        }
        let sending = {
            let mut q =
                tx.prepare("SELECT snapshot FROM share_deliveries WHERE state='sending'")?;
            q.query_map([], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        for s in sending {
            let mut d: ShareDelivery = decode(s)?;
            d.state = DeliveryState::Unknown;
            save_delivery(&tx, &d)?;
        }
        tx.execute(
            "UPDATE sharing_runtime SET actor_watermark=?1 WHERE singleton=1",
            [i64::try_from(watermark(&tx)?)?],
        )?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod timing_tests {
    use super::*;
    fn fixture(now: u64) -> (tempfile::TempDir, Store, ChannelId, ShareScope) {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(&dir.path().join("timing.db")).unwrap();
        let b = s
            .channel_bind(
                crate::channels::domain::BindingSpec {
                    name: "timing".into(),
                    adapter: "fixture".into(),
                    account: "1001".into(),
                    peer: "2002".into(),
                },
                true,
            )
            .unwrap();
        let scope = ShareScope {
            urls: vec!["https://example.org/article.txt".into()],
            expires_at_ms: now + 7 * 86400000,
            utc_offset_minutes: 480,
            quiet_windows: vec![],
            cooldown_ms: 1000,
            max_per_day: 2,
            max_send_attempts: 4,
            max_model_requests: 4,
            candidate_ttl_ms: 300000,
            source_max_age_ms: 300000,
            user_quiet_ms: 1000,
        };
        s.set_share_grant(
            b.id,
            SetShareGrant {
                expected_revision: 0,
                scope: Some(scope.clone()),
            },
            now,
        )
        .unwrap();
        s.set_attention(
            b.id,
            SetAttention {
                expected_revision: 0,
                do_not_disturb: false,
                until_ms: None,
            },
            now,
        )
        .unwrap();
        (dir, s, b.id, scope)
    }
    #[test]
    fn explicit_quiet_windows_cross_midnight_overlap_and_fixed_offset() {
        let now = 20 * 86400000 + 23 * 3600000; // Local 07:00 at UTC+8.
        let (_dir, mut s, id, mut scope) = fixture(now);
        scope.quiet_windows = vec![
            QuietWindow {
                from_minute: 22 * 60,
                to_minute: 8 * 60,
            },
            QuietWindow {
                from_minute: 7 * 60,
                to_minute: 9 * 60,
            },
        ];
        s.set_share_grant(
            id,
            SetShareGrant {
                expected_revision: 1,
                scope: Some(scope.clone()),
            },
            now,
        )
        .unwrap();
        assert_eq!(
            contact_due(&s.connection, id, now, None).unwrap(),
            Some(now + 2 * 3600000)
        );
        scope.quiet_windows = vec![
            QuietWindow {
                from_minute: 0,
                to_minute: 720,
            },
            QuietWindow {
                from_minute: 720,
                to_minute: 0,
            },
        ];
        s.set_share_grant(
            id,
            SetShareGrant {
                expected_revision: 2,
                scope: Some(scope),
            },
            now,
        )
        .unwrap();
        assert_eq!(contact_due(&s.connection, id, now, None).unwrap(), None);
    }
    #[test]
    fn dnd_is_explicit_and_wakes_only_at_its_finite_end() {
        let now = 20 * 86400000;
        let (_dir, mut s, id, _) = fixture(now);
        s.set_attention(
            id,
            SetAttention {
                expected_revision: 1,
                do_not_disturb: true,
                until_ms: Some(now + 5000),
            },
            now,
        )
        .unwrap();
        assert_eq!(
            contact_due(&s.connection, id, now, None).unwrap(),
            Some(now + 5000)
        );
        assert_eq!(
            contact_due(&s.connection, id, now + 5000, None).unwrap(),
            Some(now + 5000)
        );
        s.set_attention(
            id,
            SetAttention {
                expected_revision: 2,
                do_not_disturb: true,
                until_ms: None,
            },
            now,
        )
        .unwrap();
        assert!(
            contact_due(&s.connection, id, now + 5000, None)
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn scope_validation_rejects_missing_bounds_and_noncanonical_source() {
        let now = 20 * 86400000;
        let (_dir, _s, _id, scope) = fixture(now);
        assert!(crate::kernel::sharing::validate_scope(Some(&scope), now).is_ok());
        for n in 0..5 {
            let mut s = scope.clone();
            match n {
                0 => s.max_model_requests = 0,
                1 => s.user_quiet_ms = 0,
                2 => s.utc_offset_minutes = 900,
                3 => s.urls[0] = "https://example.org/a?token=no".into(),
                _ => s.quiet_windows.push(QuietWindow {
                    from_minute: 8,
                    to_minute: 8,
                }),
            };
            assert!(crate::kernel::sharing::validate_scope(Some(&s), now).is_err());
        }
    }
}
