use super::Store;
use crate::{actor::domain::*, backend::ControlError, domain::now_ms, redaction::Redactor};
use anyhow::{Result, ensure};
use lain_wire::{ExperienceId, ReadingId, reading::*};
use rusqlite::{Connection, OptionalExtension, Transaction};
use serde_json::{Value, json};

fn decode<T: serde::de::DeserializeOwned>(s: String) -> Result<T> {
    serde_json::from_str(&s).map_err(|_| anyhow::anyhow!("invalid reading experience state"))
}
pub(super) fn grant(db: &Connection) -> Result<ReadingGrant> {
    db.query_row(
        "SELECT snapshot FROM reading_grants ORDER BY revision DESC LIMIT 1",
        [],
        |r| r.get::<_, String>(0),
    )
    .optional()?
    .map(decode)
    .transpose()
    .map(Option::unwrap_or_default)
}
fn activity(db: &Connection, id: ReadingId) -> Result<ReadingActivity> {
    decode(
        db.query_row(
            "SELECT snapshot FROM reading_activities WHERE id=?1",
            [id.to_string()],
            |r| r.get::<_, String>(0),
        )
        .optional()?
        .ok_or(ControlError::NotFound)?,
    )
}
fn prefix(s: &str, max: usize) -> &str {
    let mut n = s.len().min(max);
    while !s.is_char_boundary(n) {
        n -= 1;
    }
    &s[..n]
}
fn source(
    db: &Connection,
    id: ReadingId,
    now: u64,
) -> Result<(ReadingActivity, ReadingGrant, String)> {
    let a = activity(db, id)?;
    let g = grant(db)?;
    ensure!(
        a.state == ReadingState::Completed
            && now < a.retain_until_ms
            && g.active(now)
            && g.revision == a.request.grant_revision
            && g.scope
                .as_ref()
                .and_then(|s| s.processing.as_ref())
                .is_some(),
        ControlError::Conflict
    );
    let withdrawn: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM reading_withdrawals WHERE activity_id=?1)",
        [id.to_string()],
        |r| r.get(0),
    )?;
    ensure!(!withdrawn, ControlError::Conflict);
    let text = db
        .query_row(
            "SELECT text FROM reading_results WHERE activity_id=?1",
            [id.to_string()],
            |r| r.get::<_, String>(0),
        )
        .optional()?
        .ok_or(ControlError::Conflict)?;
    Ok((a, g, text))
}
fn fingerprint(text: &str) -> String {
    lain_wire::actor_configuration_revision(text.as_bytes())
}
pub(super) fn read_experience(db: &Connection, id: ExperienceId) -> Result<ReadingExperience> {
    decode(
        db.query_row(
            "SELECT snapshot FROM reading_experiences WHERE id=?1",
            [id.to_string()],
            |r| r.get::<_, String>(0),
        )
        .optional()?
        .ok_or(ControlError::NotFound)?,
    )
}
fn save_experience(db: &Connection, e: &ReadingExperience) -> Result<()> {
    db.execute(
        "UPDATE reading_experiences SET snapshot=?2,active=?3 WHERE id=?1",
        (
            e.id.to_string(),
            serde_json::to_string(e)?,
            e.state == ExperienceState::Active,
        ),
    )?;
    Ok(())
}
pub(super) fn visible(
    db: &Connection,
    e: &ReadingExperience,
    channel: Option<lain_wire::ChannelId>,
    now: u64,
) -> Result<bool> {
    if e.state != ExperienceState::Active || e.summary.is_none() {
        return Ok(false);
    }
    let Ok((_, g, _)) = source(db, e.reading_id, now) else {
        return Ok(false);
    };
    Ok(g.revision == e.grant_revision
        && channel.is_none_or(|id| {
            g.scope
                .as_ref()
                .and_then(|s| s.processing.as_ref())
                .is_some_and(|p| p.channel_id == id)
        }))
}

/// Shares the Actor transaction: a durable request reservation is never refunded
/// after failure/cancel/unknown, and summary publication cannot outrun completion.
pub(super) fn sync_actor(tx: &Transaction<'_>, t: &ActorTurn) -> Result<()> {
    let Some(id) = t.reading_activity else {
        return Ok(());
    };
    let link: Option<(String,i64,String,i64,Option<i64>)> = tx.query_row("SELECT actor_turn_id,grant_revision,source_fingerprint,input_bytes,cancel_requested_at_ms FROM reading_jobs WHERE activity_id=?1", [id.to_string()], |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional()?;
    let (actor, revision, digest, input_bytes, cancel_requested) =
        link.ok_or(ControlError::Conflict)?;
    ensure!(
        actor == t.id.to_string() && t.actions.is_empty(),
        ControlError::Conflict
    );
    let previous: ActorTurn = super::decode_actor(&tx.query_row(
        "SELECT snapshot FROM lain_actor_turns WHERE id=?1",
        [actor],
        |r| r.get::<_, String>(0),
    )?)?;
    let growing = t.model_requests > previous.model_requests;
    let new_body = t
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
        || new_body
        || (t.state == TurnState::Completed && previous.state != TurnState::Completed)
    {
        ensure!(cancel_requested.is_none(), ControlError::Conflict);
        let (a, g, text) = source(tx, id, now_ms()?)?;
        ensure!(
            i64::try_from(g.revision)? == revision && fingerprint(prefix(&text, 8192)) == digest,
            ControlError::Conflict
        );
        if growing {
            ensure!(
                t.model_requests == previous.model_requests + 1,
                ControlError::Conflict
            );
            let used: i64 = tx.query_row(
                "SELECT count(*) FROM reading_model_requests WHERE grant_revision=?1",
                [revision],
                |r| r.get(0),
            )?;
            let cap = g
                .scope
                .as_ref()
                .and_then(|s| s.processing.as_ref())
                .ok_or(ControlError::Conflict)?
                .max_requests;
            ensure!(used < i64::from(cap), ControlError::Conflict);
            tx.execute(
                "INSERT INTO reading_model_requests VALUES (?1,?2,?3,?4)",
                (
                    t.id.to_string(),
                    t.model_requests,
                    revision,
                    i64::try_from(now_ms()?)?,
                ),
            )?;
        }
        if t.state == TurnState::Completed && previous.state != TurnState::Completed {
            ensure!(t.model_requests > 0, ControlError::Conflict);
            let summary = t
                .messages
                .iter()
                .rev()
                .find(|m| m.role == MessageRole::Assistant)
                .ok_or(ControlError::InvalidInput)?
                .content
                .as_str();
            ensure!(!summary.trim().is_empty(), ControlError::InvalidInput);
            let e = ReadingExperience {
                id: ExperienceId(id.0),
                reading_id: id,
                actor_turn_id: t.id,
                grant_revision: g.revision,
                revision: 1,
                state: ExperienceState::Active,
                acquired_at_ms: a.finished_at_ms.ok_or(ControlError::Conflict)?,
                completed_at_ms: now_ms()?,
                source_url: a.request.url,
                source_bytes: a.acquired_bytes,
                model_input_bytes: input_bytes.try_into()?,
                source_truncated: input_bytes < i64::from(a.acquired_bytes),
                summary: Some(prefix(summary, 4096).to_owned()),
                summary_origin: ExperienceSummaryOrigin::Model,
                summary_original_bytes: summary.len().try_into()?,
                summary_truncated: summary.len() > 4096,
            };
            tx.execute(
                "INSERT INTO reading_experiences VALUES (?1,?2,?3,1,?4)",
                (
                    e.id.to_string(),
                    id.to_string(),
                    i64::try_from(e.completed_at_ms)?,
                    serde_json::to_string(&e)?,
                ),
            )?;
        }
    }
    Ok(())
}

pub(super) fn invalidate(db: &Connection, now: u64) -> Result<()> {
    let g = grant(db)?;
    let rows = {
        let mut q = db.prepare("SELECT snapshot FROM reading_experiences WHERE active=1")?;
        q.query_map([], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?
    };
    for row in rows {
        let mut e: ReadingExperience = decode(row)?;
        if !visible(db, &e, None, now)? {
            let withdrawn: bool = db.query_row(
                "SELECT EXISTS(SELECT 1 FROM reading_withdrawals WHERE activity_id=?1)",
                [e.reading_id.to_string()],
                |r| r.get(0),
            )?;
            e.state = if withdrawn {
                ExperienceState::Withdrawn
            } else if g.revision != e.grant_revision {
                ExperienceState::Revoked
            } else {
                ExperienceState::Expired
            };
            e.revision += 1;
            e.summary = None;
            save_experience(db, &e)?;
        }
    }
    // A summary's retention also covers its duplicate in the processing receipt.
    // Include already invalidated experiences so startup repairs older snapshots.
    let turns = {
        let mut q = db.prepare("SELECT a.snapshot FROM reading_jobs j JOIN lain_actor_turns a ON a.id=j.actor_turn_id WHERE EXISTS(SELECT 1 FROM json_each(a.snapshot,'$.messages') m WHERE json_extract(m.value,'$.role')='assistant' AND json_extract(m.value,'$.content')<>?1)")?;
        q.query_map([REMOVED_SUMMARY], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?
    };
    for row in turns {
        let mut t = super::decode_actor(&row)?;
        if t.reading_activity
            .is_some_and(|id| source(db, id, now).is_err())
        {
            for m in &mut t.messages {
                if m.role == MessageRole::Assistant {
                    m.content = REMOVED_SUMMARY.into();
                }
            }
            t.revision = t.revision.checked_add(1).ok_or(ControlError::Conflict)?;
            db.execute(
                "UPDATE lain_actor_turns SET snapshot=?2 WHERE id=?1",
                (t.id.to_string(), super::encode_actor(&t)?),
            )?;
        }
    }
    Ok(())
}

const REMOVED_SUMMARY: &str = "[reading summary removed: source unavailable]";

impl Store {
    pub fn reading_budget(&self) -> Result<Value> {
        let g = grant(&self.connection)?;
        let prepared: i64 = self.connection.query_row(
            "SELECT count(*) FROM reading_model_requests WHERE grant_revision=?1",
            [i64::try_from(g.revision)?],
            |r| r.get(0),
        )?;
        let tokens:i64=self.connection.query_row("SELECT COALESCE(SUM(json_extract(a.snapshot,'$.usage_tokens')),0) FROM reading_jobs j JOIN lain_actor_turns a ON a.id=j.actor_turn_id WHERE j.grant_revision=?1",[i64::try_from(g.revision)?],|r|r.get(0))?;
        Ok(
            json!({"grant_revision":g.revision,"prepared_model_requests":prepared,"request_limit":g.scope.and_then(|s|s.processing).map(|p|p.max_requests),"reported_usage_tokens_sum":tokens,"token_coverage":"provider_reported_only_unknown_or_failed_requests_may_be_billed","currency_cost":null}),
        )
    }
    pub fn reading_audit(&self) -> Result<Value> {
        let mut q=self.connection.prepare("SELECT actor_turn_id,request_number,grant_revision,prepared_at_ms FROM reading_model_requests ORDER BY prepared_at_ms DESC,actor_turn_id,request_number LIMIT 100")?;
        let models=q.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,i64>(2)?,r.get::<_,i64>(3)?)))?.map(|r| {let(a,n,g,t)=r?;Ok(json!({"actor_turn_id":a,"request_number":n,"grant_revision":g,"prepared_at_ms":t,"reason":"authorized_reading_activity_not_proof_of_provider_receipt"}))}).collect::<Result<Vec<_>>>()?;
        let mut q=self.connection.prepare("SELECT sequence,actor_turn_id,action_id,model_request,grant_revision,experience_ids,recorded_at_ms FROM experience_reads ORDER BY sequence DESC LIMIT 100")?;
        let mut reads = vec![];
        for row in q.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<i64>>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, i64>(6)?,
            ))
        })? {
            let (n, a, action, request, g, ids, t) = row?;
            reads.push(json!({"sequence":n,"actor_turn_id":a,"action_id":action,"model_request":request,"grant_revision":g,"experience_ids":decode::<Vec<ExperienceId>>(ids)?,"recorded_at_ms":t}));
        }
        let mut q=self.connection.prepare("SELECT sequence,url,operation,recorded_at_ms FROM reading_source_decisions ORDER BY sequence DESC LIMIT 100")?;
        let decisions = q
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            })?
            .map(|r| {
                let (n, url, op, t) = r?;
                Ok(json!({"sequence":n,"url":url,"operation":op,"recorded_at_ms":t}))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(
            json!({"coverage":"latest_100_per_category","model_preparations":models,"experience_reads":reads,"source_decisions":decisions}),
        )
    }
    pub fn experience_history(&self, id: ExperienceId) -> Result<Value> {
        let e = read_experience(&self.connection, id)?;
        let mut q=self.connection.prepare("SELECT revision,operation,recorded_at_ms FROM experience_edits WHERE experience_id=?1 ORDER BY revision DESC LIMIT 100")?;
        let edits = q
            .query_map([id.to_string()], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })?
            .map(|r| {
                let (rev, op, t) = r?;
                Ok(json!({"revision":rev,"operation":op,"recorded_at_ms":t}))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(
            json!({"experience_id":id,"original_actor_turn_id":e.actor_turn_id,"coverage":"latest_100_content_free_edits","edits":edits}),
        )
    }
    pub fn experience_actor_scope(&self, id: ActorTurnId) -> Result<Value> {
        let t = self.actor_turn(id)?;
        let c = t.private_chat.as_ref().ok_or(ControlError::Unavailable)?;
        let b = self.channel_binding(c.channel_id)?;
        let g = grant(&self.connection)?;
        let p = g
            .scope
            .as_ref()
            .and_then(|s| s.processing.as_ref())
            .ok_or(ControlError::Unavailable)?;
        let now = now_ms()?;
        ensure!(
            t.state == TurnState::Running
                && b.reply_enabled
                && b.grant_revision == c.grant_revision
                && b.conversation_id == t.conversation_id
                && now < c.expires_at_ms
                && g.active(now)
                && p.channel_id == c.channel_id
                && c.experience_grant_revision.is_none_or(|v| v == g.revision),
            ControlError::Conflict
        );
        let (yesterday, day_start, tomorrow) =
            crate::reading::day_bounds(now, p.utc_offset_minutes);
        Ok(
            json!({"grant_revision":g.revision,"channel_id":c.channel_id,"now_utc_ms":now,
            "utc_offset_minutes":p.utc_offset_minutes,"today_from_utc_ms":day_start.max(0),
            "yesterday_from_utc_ms":yesterday,"tomorrow_from_utc_ms":tomorrow,
            "coverage":"at_most_100_recent_candidates_three_records_partial","processing":"model_summary_not_independently_verified"}),
        )
    }
    pub fn experience_actor_lookup(
        &mut self,
        id: ActorTurnId,
        action: ActionId,
        redactor: &Redactor,
    ) -> Result<(Vec<ExperienceId>, Value)> {
        let scope = self.experience_actor_scope(id)?;
        let mut t = self.actor_turn(id)?;
        self.private_chat_check(&t)?;
        let a = t
            .actions
            .iter()
            .find(|a| a.id == action && a.state == ActionState::Executing)
            .ok_or(ControlError::Conflict)?;
        let channel = t
            .private_chat
            .as_ref()
            .ok_or(ControlError::Unavailable)?
            .channel_id;
        let records = match &a.tool {
            crate::actor::tools::Tool::ExperienceSearch(q) => {
                self.experiences(q.clone(), Some(channel), redactor)?
            }
            crate::actor::tools::Tool::ExperienceInspect { experience_id } => {
                let e = self.experience(*experience_id, redactor)?;
                ensure!(
                    visible(&self.connection, &e, Some(channel), now_ms()?)?,
                    ControlError::NotFound
                );
                vec![e]
            }
            _ => anyhow::bail!(ControlError::InvalidInput),
        };
        let mut ids = vec![];
        let mut projected = vec![];
        for e in records {
            projected.push(project(&e));
            if serde_json::to_vec(&projected)?.len() > 8192 {
                break;
            }
            ids.push(e.id);
        }
        let g = scope["grant_revision"]
            .as_u64()
            .ok_or(ControlError::Conflict)?;
        let (from, to) = match &a.tool {
            crate::actor::tools::Tool::ExperienceSearch(q) => (q.from_ms, q.to_ms),
            _ => (0, i64::MAX as u64),
        };
        let mut operations = vec![];
        {
            let mut q=self.connection.prepare("SELECT snapshot FROM reading_experiences WHERE json_extract(snapshot,'$.grant_revision')=?1 AND completed_at_ms>=?2 AND completed_at_ms<?3 ORDER BY completed_at_ms DESC,id LIMIT 3")?;
            for row in q.query_map(
                (i64::try_from(g)?, i64::try_from(from)?, i64::try_from(to)?),
                |r| r.get::<_, String>(0),
            )? {
                let e: ReadingExperience = decode(row?)?;
                operations.push(json!({"experience_id":e.id,"reading_id":e.reading_id,"actor_turn_id":e.actor_turn_id,"summary_generation_recorded":true,"completed_at_ms":e.completed_at_ms,"projection_state_at_lookup":e.state,"observed_at_ms":now_ms()?}));
            }
        }
        let tx = self.connection.transaction()?;
        t.private_chat
            .as_mut()
            .ok_or(ControlError::Conflict)?
            .experience_grant_revision = Some(g);
        tx.execute("INSERT INTO experience_reads(actor_turn_id,action_id,model_request,grant_revision,experience_ids,recorded_at_ms) VALUES (?1,?2,NULL,?3,?4,?5)",(id.to_string(),action.to_string(),i64::try_from(g)?,serde_json::to_string(&ids)?,i64::try_from(now_ms()?)?))?;
        Self::save_actor_in_transaction(&tx, &mut t, false)?;
        tx.commit()?;
        Ok((
            ids.clone(),
            json!({"experience_ids":ids,"context_role":"untrusted_reading_summary","records":"Fresh valid records are supplied in the next request context; no-match is not proof no activity occurred.","coverage":"partial","recent_operations_role":"kernel_operation_metadata","recent_operations":operations}),
        ))
    }
    pub fn experience_actor_disclosure(
        &mut self,
        id: ActorTurnId,
        ids: &[ExperienceId],
        request: u32,
        redactor: &Redactor,
    ) -> Result<Value> {
        let scope = self.experience_actor_scope(id)?;
        let mut t = self.actor_turn(id)?;
        ensure!(
            t.model_requests == request && request > 0 && ids.len() <= 3,
            ControlError::Conflict
        );
        self.private_chat_check(&t)?;
        let c = t
            .private_chat
            .as_ref()
            .ok_or(ControlError::Unavailable)?
            .channel_id;
        let tx = self.connection.transaction()?;
        let mut records = vec![];
        let mut versions = vec![];
        for id in ids {
            let e = read_experience(&tx, *id)?;
            ensure!(
                visible(&tx, &e, Some(c), now_ms()?)?,
                ControlError::Conflict
            );
            let encoded = serde_json::to_string(&e)?;
            ensure!(
                redactor.text(&encoded) == encoded,
                ControlError::InvalidInput
            );
            records.push(project(&e));
            versions.push(ExperienceVersion {
                experience_id: *id,
                revision: e.revision,
                grant_revision: e.grant_revision,
                fingerprint: fingerprint(&encoded),
            });
        }
        ensure!(
            serde_json::to_vec(&records)?.len() <= 8192,
            ControlError::Conflict
        );
        t.private_chat
            .as_mut()
            .ok_or(ControlError::Unavailable)?
            .experience_grant_revision = scope["grant_revision"].as_u64();
        t.private_chat
            .as_mut()
            .ok_or(ControlError::Unavailable)?
            .experience_versions = versions;
        tx.execute("INSERT INTO experience_reads(actor_turn_id,action_id,model_request,grant_revision,experience_ids,recorded_at_ms) VALUES (?1,NULL,?2,?3,?4,?5)",(id.to_string(),request,scope["grant_revision"].as_i64().ok_or(ControlError::Conflict)?,serde_json::to_string(ids)?,i64::try_from(now_ms()?)?))?;
        Self::save_actor_in_transaction(&tx, &mut t, false)?;
        tx.commit()?;
        Ok(
            json!({"scope":scope,"records":records,"role":"untrusted_model_summaries","verification":"not_independently_verified"}),
        )
    }
    pub fn create_reading_job(
        &mut self,
        id: ReadingId,
        redactor: &Redactor,
    ) -> Result<(ActorTurn, bool)> {
        if let Some(actor) = self
            .connection
            .query_row(
                "SELECT actor_turn_id FROM reading_jobs WHERE activity_id=?1",
                [id.to_string()],
                |r| r.get::<_, String>(0),
            )
            .optional()?
        {
            return Ok((self.actor_turn(actor.parse()?)?, false));
        }
        let now = now_ms()?;
        let (_, g, text) = source(&self.connection, id, now)?;
        ensure!(redactor.text(&text) == text, ControlError::InvalidInput);
        let busy:bool=self.connection.query_row("SELECT EXISTS(SELECT 1 FROM reading_jobs j JOIN lain_actor_turns a ON a.id=j.actor_turn_id WHERE json_extract(a.snapshot,'$.state') IN ('running','pending_confirmation'))",[],|r|r.get(0))?;
        let used: i64 = self.connection.query_row(
            "SELECT count(*) FROM reading_model_requests WHERE grant_revision=?1",
            [i64::try_from(g.revision)?],
            |r| r.get(0),
        )?;
        ensure!(
            !busy
                && used
                    < i64::from(
                        g.scope
                            .as_ref()
                            .and_then(|s| s.processing.as_ref())
                            .ok_or(ControlError::Conflict)?
                            .max_requests
                    ),
            ControlError::Conflict
        );
        let input = prefix(&text, 8192);
        let c = ConversationId::new();
        let mut t = ActorTurn {
            id: ActorTurnId::new(),
            reading_activity: None,
            briefing_activity: None,
            sharing_activity: None,
            conversation_id: c,
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
        t.message(MessageRole::User,format!("Kernel reading activity {id}: summarize the supplied authorized source excerpt. This is a Kernel activity request, not a human statement. Describe only that excerpt; do not claim independent verification or any other action."))?;
        let tx = self.connection.transaction()?;
        tx.execute(
            "INSERT INTO lain_conversations VALUES (?1,?2,?2)",
            (c.to_string(), i64::try_from(now)?),
        )?;
        Self::save_actor_in_transaction(&tx, &mut t, true)?;
        tx.execute(
            "INSERT INTO reading_jobs VALUES (?1,?2,?3,?4,?5,?6,NULL)",
            (
                id.to_string(),
                t.id.to_string(),
                i64::try_from(g.revision)?,
                fingerprint(input),
                i64::try_from(input.len())?,
                i64::try_from(now)?,
            ),
        )?;
        t.reading_activity = Some(id);
        Self::save_actor_in_transaction(&tx, &mut t, false)?;
        tx.commit()?;
        Ok((t, true))
    }
    pub fn reading_actor_input(&self, t: &ActorTurn, redactor: &Redactor) -> Result<Value> {
        let id = t.reading_activity.ok_or(ControlError::InvalidInput)?;
        let (a, g, text) = source(&self.connection, id, now_ms()?)?;
        ensure!(redactor.text(&text) == text, ControlError::InvalidInput);
        let expected: String = self.connection.query_row(
            "SELECT source_fingerprint FROM reading_jobs WHERE activity_id=?1 AND actor_turn_id=?2 AND cancel_requested_at_ms IS NULL",
            (id.to_string(), t.id.to_string()),
            |r| r.get(0),
        )?;
        let excerpt = prefix(&text, 8192);
        ensure!(fingerprint(excerpt) == expected, ControlError::Conflict);
        Ok(
            json!({"role":"untrusted_source_data","reading_id":id,"grant_revision":g.revision,"source_url":a.request.url,"acquired_at_ms":a.finished_at_ms,"source_bytes":text.len(),"model_input_bytes":excerpt.len(),"truncated":excerpt.len()<text.len(),"text":excerpt,"verification":"not_independently_verified"}),
        )
    }
    pub(super) fn reading_actor_source_current(&self, t: &ActorTurn) -> Result<()> {
        let id = t.reading_activity.ok_or(ControlError::InvalidInput)?;
        let (_, _, text) = source(&self.connection, id, now_ms()?)?;
        let expected: String = self.connection.query_row(
            "SELECT source_fingerprint FROM reading_jobs WHERE activity_id=?1 AND actor_turn_id=?2 AND cancel_requested_at_ms IS NULL",
            (id.to_string(), t.id.to_string()), |r| r.get(0),
        )?;
        ensure!(
            fingerprint(prefix(&text, 8192)) == expected,
            ControlError::Conflict
        );
        Ok(())
    }
    pub fn experiences(
        &mut self,
        query: ExperienceQuery,
        channel: Option<lain_wire::ChannelId>,
        redactor: &Redactor,
    ) -> Result<Vec<ReadingExperience>> {
        ensure!(
            query.from_ms < query.to_ms
                && query.to_ms <= i64::MAX as u64
                && query.to_ms - query.from_ms <= 31 * 86400000
                && (1..=3).contains(&query.limit)
                && query.text.as_ref().is_none_or(|s| s.len() <= 256),
            ControlError::InvalidInput
        );
        self.prune_reading(now_ms()?)?;
        let rows = {
            let mut q=self.connection.prepare("SELECT snapshot FROM reading_experiences WHERE completed_at_ms>=?1 AND completed_at_ms<?2 ORDER BY completed_at_ms DESC,id LIMIT 100")?;
            q.query_map(
                (i64::try_from(query.from_ms)?, i64::try_from(query.to_ms)?),
                |r| r.get::<_, String>(0),
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?
        };
        let mut result = vec![];
        for row in rows {
            let e: ReadingExperience = decode(row)?;
            if !visible(&self.connection, &e, channel, now_ms()?)? {
                continue;
            }
            let encoded = serde_json::to_string(&e)?;
            if redactor.text(&encoded) != encoded {
                continue;
            }
            if let Some(text) = &query.text
                && !format!("{}\n{}", e.source_url, e.summary.as_deref().unwrap_or(""))
                    .to_lowercase()
                    .contains(&text.to_lowercase())
            {
                continue;
            }
            result.push(e);
            if result.len() == usize::from(query.limit) {
                break;
            }
        }
        Ok(result)
    }
    pub fn experience(
        &mut self,
        id: ExperienceId,
        redactor: &Redactor,
    ) -> Result<ReadingExperience> {
        self.prune_reading(now_ms()?)?;
        let mut e = read_experience(&self.connection, id)?;
        if redactor.text(&serde_json::to_string(&e)?) != serde_json::to_string(&e)? {
            e.summary = None;
        }
        Ok(e)
    }
    pub fn withdraw_experience(
        &mut self,
        id: ExperienceId,
        revision: u64,
    ) -> Result<ReadingExperience> {
        let mut e = read_experience(&self.connection, id)?;
        ensure!(e.revision == revision, ControlError::Conflict);
        let tx = self.connection.transaction()?;
        tx.execute(
            "INSERT OR IGNORE INTO reading_source_suppressions VALUES (?1,?2)",
            (&e.source_url, i64::try_from(now_ms()?)?),
        )?;
        tx.execute("INSERT INTO reading_source_decisions(url,operation,recorded_at_ms) VALUES (?1,'suppress',?2)",(&e.source_url,i64::try_from(now_ms()?)?))?;
        tx.execute("INSERT OR IGNORE INTO reading_withdrawals SELECT id,?2 FROM reading_activities WHERE json_extract(snapshot,'$.request.url')=?1",(&e.source_url,i64::try_from(now_ms()?)?))?;
        tx.execute("DELETE FROM reading_results WHERE activity_id IN (SELECT id FROM reading_activities WHERE json_extract(snapshot,'$.request.url')=?1)",[&e.source_url])?;
        e.state = ExperienceState::Withdrawn;
        e.summary = None;
        e.revision += 1;
        save_experience(&tx, &e)?;
        tx.execute(
            "INSERT INTO experience_edits VALUES (?1,?2,'withdraw',?3)",
            (
                id.to_string(),
                i64::try_from(e.revision)?,
                i64::try_from(now_ms()?)?,
            ),
        )?;
        invalidate(&tx, now_ms()?)?;
        tx.commit()?;
        Ok(e)
    }
    pub fn correct_experience(
        &mut self,
        id: ExperienceId,
        q: CorrectExperience,
    ) -> Result<ReadingExperience> {
        ensure!(
            !q.summary.trim().is_empty() && q.summary.len() <= 4096,
            ControlError::InvalidInput
        );
        let mut e = read_experience(&self.connection, id)?;
        ensure!(
            e.revision == q.expected_revision && visible(&self.connection, &e, None, now_ms()?)?,
            ControlError::Conflict
        );
        e.revision += 1;
        e.summary_original_bytes = q.summary.len().try_into()?;
        e.summary_truncated = false;
        e.summary = Some(q.summary);
        e.summary_origin = ExperienceSummaryOrigin::OwnerCorrection;
        let tx = self.connection.transaction()?;
        save_experience(&tx, &e)?;
        tx.execute(
            "INSERT INTO experience_edits VALUES (?1,?2,'owner_correction',?3)",
            (
                id.to_string(),
                i64::try_from(e.revision)?,
                i64::try_from(now_ms()?)?,
            ),
        )?;
        tx.commit()?;
        Ok(e)
    }
    pub fn reading_suppressions(&self) -> Result<Value> {
        let mut q=self.connection.prepare("SELECT url,recorded_at_ms FROM reading_source_suppressions ORDER BY recorded_at_ms DESC,url LIMIT 100")?;
        let rows = q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        Ok(
            json!({"coverage":"latest_100","records":rows.map(|r|{let(url,at)=r?;Ok(json!({"url":url,"recorded_at_ms":at}))}).collect::<Result<Vec<_>>>()?}),
        )
    }
    pub fn restore_reading_source(&mut self, url: String) -> Result<()> {
        let tx = self.connection.transaction()?;
        tx.execute(
            "DELETE FROM reading_source_suppressions WHERE url=?1",
            [&url],
        )?;
        tx.execute("INSERT INTO reading_source_decisions(url,operation,recorded_at_ms) VALUES (?1,'restore_for_new_acquisition',?2)",(url,i64::try_from(now_ms()?)?))?;
        tx.commit()?;
        Ok(())
    }
}

pub(super) fn project(e: &ReadingExperience) -> Value {
    let mut value = serde_json::to_value(e).expect("experience serialization");
    value["summary"] = e
        .summary
        .as_ref()
        .map(|s| json!({"text":prefix(s,1024),"projection_truncated":s.len()>1024}))
        .unwrap_or(Value::Null);
    value
}

pub(super) fn selected_current(db: &Connection, t: &ActorTurn) -> Result<bool> {
    let Some(c) = &t.private_chat else {
        return Ok(true);
    };
    if let Some(revision) = c.experience_grant_revision {
        let g = grant(db)?;
        if g.revision != revision
            || !g.active(now_ms()?)
            || !g
                .scope
                .as_ref()
                .and_then(|s| s.processing.as_ref())
                .is_some_and(|p| p.channel_id == c.channel_id)
        {
            return Ok(false);
        }
    }
    for v in &c.experience_versions {
        let Ok(e) = read_experience(db, v.experience_id) else {
            return Ok(false);
        };
        let redactor = Redactor::new(vec![]).for_runtime(None);
        let encoded = serde_json::to_string(&e)?;
        if redactor.text(&encoded) != encoded
            || e.revision != v.revision
            || e.grant_revision != v.grant_revision
            || !visible(db, &e, Some(c.channel_id), now_ms()?)?
            || fingerprint(&serde_json::to_string(&e)?) != v.fingerprint
        {
            return Ok(false);
        }
    }
    Ok(true)
}
