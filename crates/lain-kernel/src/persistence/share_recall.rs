//! Fresh projections of Lain's own outgoing receipts, never a new collection grant.
use super::{
    Store,
    channels::{binding, decode, one},
};
use crate::{
    actor::{domain::*, tools::Tool},
    backend::ControlError,
    domain::now_ms,
    redaction::Redactor,
};
use anyhow::{Result, ensure};
use lain_wire::{ActionId, ChannelId, DeliveryId, reading::ExperienceVersion, sharing::*};
use rusqlite::{Connection, params};
use serde_json::{Value, json};

fn digest(value: &impl serde::Serialize) -> Result<String> {
    Ok(lain_wire::actor_configuration_revision(
        serde_json::to_string(value)?.as_bytes(),
    ))
}
fn delivery(db: &Connection, id: DeliveryId) -> Result<ShareDelivery> {
    one(
        db,
        "SELECT snapshot FROM share_deliveries WHERE id=?1",
        id.to_string(),
    )
}
fn source_version(
    db: &Connection,
    d: &ShareDelivery,
    redactor: &Redactor,
) -> Result<Option<ExperienceVersion>> {
    let c: ShareCandidate = one(
        db,
        "SELECT snapshot FROM share_candidates WHERE id=?1",
        d.candidate_id.to_string(),
    )?;
    ensure!(c.channel_id == d.channel_id, ControlError::Conflict);
    let e = super::experiences::read_experience(db, c.experience_id)?;
    if e.revision != c.experience_revision
        || digest(&e)? != c.experience_fingerprint
        || !super::experiences::visible(db, &e, Some(d.channel_id), now_ms()?)?
        || redactor.text(&serde_json::to_string(&e)?) != serde_json::to_string(&e)?
        || redactor.text(&d.text) != d.text
    {
        return Ok(None);
    }
    Ok(Some(ExperienceVersion {
        experience_id: e.id,
        revision: e.revision,
        grant_revision: e.grant_revision,
        fingerprint: digest(&e)?,
    }))
}
fn project(
    db: &Connection,
    id: ChannelId,
    d: &ShareDelivery,
    redactor: &Redactor,
    content: bool,
) -> Result<(Value, ShareVersion)> {
    ensure!(d.channel_id == id, ControlError::NotFound);
    let experience = if content {
        source_version(db, d, redactor)?
    } else {
        None
    };
    let c: ShareCandidate = one(
        db,
        "SELECT snapshot FROM share_candidates WHERE id=?1",
        d.candidate_id.to_string(),
    )?;
    let mut record = json!({"delivery_id":d.id,"candidate_id":d.candidate_id,"experience_id":c.experience_id,
        "original_experience_revision":c.experience_revision,"state":d.state,"claimed_at_utc_ms":d.created_at_ms,
        "receipt_recorded":d.receipt.is_some(),"user_read_receipt":false,"user_read_status":"unknown","outbound_text":null,"experience":null,
        "content_status":"withheld_source_changed_or_unavailable","observed_at_utc_ms":now_ms()?});
    if !content {
        record["content_status"] = json!("metadata_only_use_sharing_inspect_for_content");
    }
    if let Some(v) = &experience {
        let e = super::experiences::read_experience(db, v.experience_id)?;
        record["outbound_text"] = json!(d.text);
        record["experience"] = super::experiences::project(&e);
        record["content_status"] = json!("current_at_disclosure");
    }
    Ok((
        record,
        ShareVersion {
            delivery_id: d.id,
            fingerprint: digest(d)?,
            experience,
        },
    ))
}
fn chat_channel(store: &Store, id: ActorTurnId) -> Result<ChannelId> {
    let t = store.actor_turn(id)?;
    let c = t.private_chat.as_ref().ok_or(ControlError::Unavailable)?;
    ensure!(t.state == TurnState::Running, ControlError::Conflict);
    store.private_chat_check(&t)?;
    let b = binding(&store.connection, c.channel_id)?;
    ensure!(
        b.reply_enabled
            && b.grant_revision == c.grant_revision
            && b.conversation_id == t.conversation_id
            && now_ms()? < c.expires_at_ms,
        ControlError::Conflict
    );
    Ok(c.channel_id)
}
fn matches_topic(record: &Value, topic: &str) -> bool {
    [
        &record["outbound_text"],
        &record["experience"]["source_url"],
        &record["experience"]["summary"]["text"],
        &record["delivery_id"],
        &record["candidate_id"],
        &record["experience_id"],
        &record["state"],
    ]
    .into_iter()
    .any(|value| value.as_str().is_some_and(|s| s.contains(topic)))
}

impl Store {
    pub fn share_receipts(
        &self,
        id: ChannelId,
        q: ShareQuery,
        redactor: &Redactor,
    ) -> Result<Vec<Value>> {
        binding(&self.connection, id)?;
        ensure!(q.valid(), ControlError::InvalidInput);
        let mut query=self.connection.prepare("SELECT snapshot FROM share_deliveries WHERE channel_id=?1 AND json_extract(snapshot,'$.created_at_ms')>=?2 AND json_extract(snapshot,'$.created_at_ms')<?3 ORDER BY sequence DESC LIMIT 100")?;
        let mut result = vec![];
        for row in query.query_map(
            (
                id.to_string(),
                i64::try_from(q.from_ms)?,
                i64::try_from(q.to_ms)?,
            ),
            |r| r.get::<_, String>(0),
        )? {
            let d: ShareDelivery = decode(row?)?;
            let (record, _) = project(&self.connection, id, &d, redactor, true)?;
            if q.text
                .as_ref()
                .is_some_and(|text| !matches_topic(&record, text.trim()))
            {
                continue;
            }
            let mut next = result.clone();
            next.push(record);
            if serde_json::to_vec(&next)?.len() > 8192 {
                break;
            }
            result = next;
            if result.len() >= usize::from(q.limit) {
                break;
            }
        }
        Ok(result)
    }
    pub fn share_receipt(
        &self,
        id: ChannelId,
        d: DeliveryId,
        redactor: &Redactor,
    ) -> Result<Value> {
        binding(&self.connection, id)?;
        Ok(project(
            &self.connection,
            id,
            &delivery(&self.connection, d)?,
            redactor,
            true,
        )?
        .0)
    }
    pub fn share_read_audit(&self, id: ChannelId) -> Result<Value> {
        binding(&self.connection, id)?;
        let mut q=self.connection.prepare("SELECT actor_turn_id,action_id,model_request,delivery_ids,versions,prepared_at_ms FROM share_reads WHERE channel_id=?1 ORDER BY sequence DESC LIMIT 100")?;
        let records=q.query_map([id.to_string()],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Option<String>>(1)?,r.get::<_,Option<i64>>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,i64>(5)?)))?.map(|row|{
            let(actor,action,request,ids,versions,at)=row?;
            Ok(json!({"actor_turn_id":actor,"action_id":action,"model_request":request,"delivery_ids":serde_json::from_str::<Value>(&ids)?,"versions":serde_json::from_str::<Value>(&versions)?,"prepared_at_ms":at}))
        }).collect::<Result<Vec<_>>>()?;
        Ok(
            json!({"channel_id":id,"coverage":"latest_100_content_free_reads","records":records,"preparation_is_not_proof_of_model_comprehension":true}),
        )
    }
    pub fn share_actor_lookup(
        &mut self,
        id: ActorTurnId,
        action: ActionId,
        redactor: &Redactor,
    ) -> Result<(Vec<DeliveryId>, Value)> {
        let channel = chat_channel(self, id)?;
        let t = self.actor_turn(id)?;
        let a = t
            .actions
            .iter()
            .find(|a| a.id == action && a.state == ActionState::Executing)
            .ok_or(ControlError::Conflict)?;
        let records = match &a.tool {
            Tool::SharingSearch(q) => self.share_receipts(channel, q.clone(), redactor)?,
            Tool::SharingInspect { delivery_id } => {
                vec![self.share_receipt(channel, *delivery_id, redactor)?]
            }
            _ => anyhow::bail!(ControlError::InvalidInput),
        };
        let ids = records
            .iter()
            .map(|r| serde_json::from_value::<DeliveryId>(r["delivery_id"].clone()))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        self.connection.execute("INSERT INTO share_reads(actor_turn_id,action_id,channel_id,delivery_ids,versions,prepared_at_ms) VALUES (?1,?2,?3,?4,'[]',?5)",(id.to_string(),action.to_string(),channel.to_string(),serde_json::to_string(&ids)?,i64::try_from(now_ms()?)?))?;
        Ok((
            ids.clone(),
            json!({"delivery_ids":ids,"records":"Fresh receipt/source projections are supplied in the next model request, not saved in this tool receipt.","coverage":"at_most_100_recent_candidates_three_records_partial","absence_is_not_proof_no_contact_occurred":true}),
        ))
    }
    pub fn share_actor_disclosure(
        &mut self,
        id: ActorTurnId,
        selection: Option<Vec<DeliveryId>>,
        request: u32,
        redactor: &Redactor,
    ) -> Result<(Vec<DeliveryId>, Value)> {
        let channel = chat_channel(self, id)?;
        let mut t = self.actor_turn(id)?;
        ensure!(
            t.model_requests == request && request > 0,
            ControlError::Conflict
        );
        let automatic = selection.is_none();
        let ids = if let Some(ids) = selection {
            ids
        } else {
            let mut q=self.connection.prepare("SELECT id FROM share_deliveries WHERE channel_id=?1 AND state IN ('confirmed','unknown','not_sent','rejected') AND json_extract(snapshot,'$.created_at_ms')>=?2 ORDER BY sequence DESC LIMIT 1")?;
            q.query_map(
                (
                    channel.to_string(),
                    i64::try_from(now_ms()?.saturating_sub(86400000))?,
                ),
                |r| r.get::<_, String>(0),
            )?
            .map(|row| Ok(row?.parse()?))
            .collect::<Result<Vec<DeliveryId>>>()?
        };
        ensure!(ids.len() <= 3, ControlError::InvalidInput);
        let mut records = vec![];
        let mut versions = vec![];
        let mut selected = vec![];
        for id in ids {
            if selected.contains(&id) {
                continue;
            }
            let (record, version) = project(
                &self.connection,
                channel,
                &delivery(&self.connection, id)?,
                redactor,
                !automatic,
            )?;
            let mut next = records.clone();
            next.push(record);
            if serde_json::to_vec(&next)?.len() > 8192 {
                break;
            }
            records = next;
            versions.push(version);
            selected.push(id);
        }
        if !versions.is_empty() {
            t.private_chat
                .as_mut()
                .ok_or(ControlError::Conflict)?
                .share_versions = versions.clone();
            let tx = self.connection.transaction()?;
            tx.execute("INSERT INTO share_reads(actor_turn_id,model_request,channel_id,delivery_ids,versions,prepared_at_ms) VALUES (?1,?2,?3,?4,?5,?6)",params![id.to_string(),request,channel.to_string(),serde_json::to_string(&selected)?,serde_json::to_string(&versions)?,i64::try_from(now_ms()?)?])?;
            Self::save_actor_in_transaction(&tx, &mut t, false)?;
            tx.commit()?;
        }
        Ok((
            selected,
            json!({"now_utc_ms":now_ms()?,"records":records,"automatic_recent_context":automatic,"coverage":"partial_not_a_complete_chat_archive","role":"kernel_transport_metadata_and_untrusted_attributed_outbound_text","confirmed_is_not_user_read":true}),
        ))
    }
}

pub(super) fn selected_current(db: &Connection, t: &ActorTurn) -> Result<bool> {
    let Some(c) = &t.private_chat else {
        return Ok(true);
    };
    for v in &c.share_versions {
        let d = delivery(db, v.delivery_id)?;
        if d.channel_id != c.channel_id || digest(&d)? != v.fingerprint {
            return Ok(false);
        }
        if let Some(expected) = &v.experience {
            let redactor = Redactor::new(vec![]).for_runtime(None);
            let current = source_version(db, &d, &redactor)?;
            if current.as_ref().is_none_or(|e| {
                e.experience_id != expected.experience_id
                    || e.revision != expected.revision
                    || e.grant_revision != expected.grant_revision
                    || e.fingerprint != expected.fingerprint
            }) {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
