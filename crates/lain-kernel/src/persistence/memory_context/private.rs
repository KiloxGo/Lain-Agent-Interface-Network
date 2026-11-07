//! Metadata-only lineage for private evidence carried into later model input.
use super::*;
use lain_wire::{ChannelId, chat::PrivateChatContext, source::SourceReadContext};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Lineage {
    pub tainted: bool,
    pub inherited: Vec<Evidence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disclosed: Vec<Evidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    original: Option<RawBatch>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBatch {
    channel_id: ChannelId,
    batch_id: uuid::Uuid,
    through: i64,
    records: Vec<(String, String)>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Evidence {
    turn_id: ActorTurnId,
    request_id: uuid::Uuid,
    channel_id: ChannelId,
    model_request: u32,
    channel_memory: Option<ChannelMemoryContext>,
    channel_source: Option<SourceReadContext>,
    chat: Option<PrivateChatContext>,
    original: RawBatch,
}
impl Lineage {
    pub(super) fn new(db: &Connection, turn: &ActorTurn) -> Result<Self> {
        Ok(Self {
            tainted: influenced(turn),
            inherited: vec![],
            disclosed: vec![],
            original: RawBatch::capture(db, turn)?,
        })
    }
    pub(super) fn tracked(&self, channel: Option<ChannelId>) -> bool {
        channel == self.original.as_ref().map(|r| r.channel_id)
    }
    pub(super) fn adopt(&mut self, db: &Connection, turn: &ActorTurn) -> Result<bool> {
        if let Some(raw) = &self.original
            && turn
                .private_chat
                .as_ref()
                .is_some_and(|c| c.input_through_sequence > raw.through as u64)
        {
            let fresh = RawBatch::capture(db, turn)?.ok_or(ChangedMemoryContext)?;
            ensure!(
                fresh.batch_id == raw.batch_id
                    && fresh.channel_id == raw.channel_id
                    && raw.records.iter().all(|r| fresh.records.contains(r)),
                ChangedMemoryContext
            );
            if fresh.through > raw.through {
                self.original = Some(fresh);
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub(super) fn current(&self, db: &Connection) -> Result<()> {
        ensure!(
            self.inherited.len() <= 32 && self.disclosed.len() <= 32,
            "private lineage exceeds its bound"
        );
        if let Some(raw) = &self.original {
            raw.current(db)?;
        }
        for e in self.inherited.iter().chain(&self.disclosed) {
            let raw: String = db.query_row(
                "SELECT snapshot FROM lain_actor_turns WHERE id=?1",
                [e.turn_id.to_string()],
                |r| r.get(0),
            )?;
            let mut t = super::super::decode_actor(&raw)?;
            ensure!(
                t.request_id == e.request_id && route(db, &t)? == Some(e.channel_id),
                ChangedMemoryContext
            );
            t.model_requests = e.model_request;
            t.channel_memory = e.channel_memory.clone();
            t.channel_source = e.channel_source.clone();
            t.private_chat = e.chat.clone();
            e.original.current(db)?;
            ensure!(snapshot_current(db, &t)?, ChangedMemoryContext);
        }
        Ok(())
    }
    pub(super) fn observe(&mut self, turn: &ActorTurn) -> Result<bool> {
        if !influenced(turn) || turn.private_chat.is_none() {
            return Ok(false);
        }
        let raw = self.original.as_ref().ok_or(ChangedMemoryContext)?;
        let before = self.disclosed.len();
        merge(
            &mut self.disclosed,
            Evidence {
                turn_id: turn.id,
                request_id: turn.request_id,
                channel_id: raw.channel_id,
                model_request: turn.model_requests,
                channel_memory: turn.channel_memory.clone(),
                channel_source: turn.channel_source.clone(),
                chat: turn.private_chat.clone(),
                original: raw.clone(),
            },
        )?;
        ensure!(
            self.disclosed.len() <= 32 && serde_json::to_vec(self)?.len() <= 98304,
            ControlError::InvalidInput
        );
        Ok(before != self.disclosed.len())
    }
    pub(super) fn inherit(
        &mut self,
        old: &Lineage,
        turn: &ActorTurn,
        channel: Option<ChannelId>,
    ) -> Result<bool> {
        let mut selected = self.inherited.clone();
        if !old.tracked(channel) {
            return Ok(false);
        }
        for e in old.inherited.iter().chain(&old.disclosed) {
            merge(&mut selected, e.clone())?;
        }
        if let Some(channel_id) = channel {
            merge(
                &mut selected,
                Evidence {
                    turn_id: turn.id,
                    request_id: turn.request_id,
                    channel_id,
                    model_request: turn.model_requests,
                    channel_memory: turn.channel_memory.clone(),
                    channel_source: turn.channel_source.clone(),
                    chat: turn.private_chat.clone(),
                    original: old.original.clone().ok_or(ChangedMemoryContext)?,
                },
            )?;
        }
        if selected.len() > 32 || serde_json::to_vec(&(&selected, &self.original))?.len() > 98304 {
            return Ok(false);
        }
        self.inherited = selected;
        self.tainted |= old.tainted || influenced(turn);
        Ok(true)
    }
}
impl RawBatch {
    fn capture(db: &Connection, t: &ActorTurn) -> Result<Option<Self>> {
        let Some(channel_id) = route(db, t)? else {
            return Ok(None);
        };
        let mut query = db.prepare("SELECT id,state,snapshot,sequence FROM channel_incoming WHERE channel_id=?1 AND json_extract(snapshot,'$.batch_id')=?2 ORDER BY sequence LIMIT 513")?;
        let rows = query
            .query_map((channel_id.to_string(), t.request_id.to_string()), |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        ensure!(!rows.is_empty() && rows.len() <= 512, ChangedMemoryContext);
        let through = rows.last().ok_or(ChangedMemoryContext)?.3;
        let records = rows
            .into_iter()
            .map(|(id, state, snapshot, _)| {
                Ok((
                    id,
                    lain_wire::actor_configuration_revision(&serde_json::to_vec(&(
                        state, snapshot,
                    ))?),
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Some(Self {
            channel_id,
            batch_id: t.request_id,
            through,
            records,
        }))
    }
    fn current(&self, db: &Connection) -> Result<()> {
        ensure!(
            !self.records.is_empty() && self.records.len() <= 512,
            ChangedMemoryContext
        );
        for (id, hash) in &self.records {
            let row: Option<(String,String,bool)> = db.query_row("SELECT i.state,i.snapshot,s.incoming_id IS NOT NULL FROM channel_incoming i LEFT JOIN channel_memory_suppressions s ON s.channel_id=i.channel_id AND s.incoming_id=i.id WHERE i.id=?1 AND i.channel_id=?2", (id,self.channel_id.to_string()), |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
            let (state, snapshot, suppressed) = row.ok_or(ChangedMemoryContext)?;
            ensure!(
                !suppressed
                    && !matches!(state.as_str(), "revoked" | "expired")
                    && *hash
                        == lain_wire::actor_configuration_revision(&serde_json::to_vec(&(
                            state, snapshot
                        ))?),
                ChangedMemoryContext
            );
        }
        Ok(())
    }
}
fn merge(out: &mut Vec<Evidence>, e: Evidence) -> Result<()> {
    let encoded = serde_json::to_vec(&e)?;
    if !out
        .iter()
        .any(|old| serde_json::to_vec(old).ok().as_ref() == Some(&encoded))
    {
        out.push(e);
    }
    Ok(())
}
pub(super) fn influenced(t: &ActorTurn) -> bool {
    t.private_chat.as_ref().is_some_and(|c| {
        !c.memory_versions.is_empty()
            || !c.archive_memory_versions.is_empty()
            || c.archive_memory_tainted
            || !c.source_versions.is_empty()
            || !c.experience_versions.is_empty()
            || !c.archive_versions.is_empty()
            || !c.media_versions.is_empty()
            || !c.viewed_media.is_empty()
            || c.share_versions.iter().any(|v| v.experience.is_some())
    }) || t
        .actions
        .iter()
        .any(|a| a.state == ActionState::Executed && a.tool.private_memory())
}
pub(super) fn route(db: &Connection, t: &ActorTurn) -> Result<Option<ChannelId>> {
    let route = t
        .private_chat
        .as_ref()
        .map(|c| c.channel_id)
        .or_else(|| t.channel_memory.as_ref().map(|c| c.channel_id))
        .or_else(|| t.channel_source.as_ref().map(|c| c.channel_id));
    let indexed: Option<String> = db
        .query_row(
            "SELECT channel_id FROM channel_batches WHERE id=?1",
            [t.request_id.to_string()],
            |r| r.get(0),
        )
        .optional()?;
    let indexed = indexed.map(|id| id.parse()).transpose()?;
    ensure!(
        route.is_none() || indexed.is_none() || route == indexed,
        "private route identity mismatch"
    );
    Ok(route.or(indexed))
}
pub(super) fn raw_current(db: &Connection, t: &ActorTurn) -> Result<bool> {
    let Some(channel) = route(db, t)? else {
        return Ok(true);
    };
    let (total, blocked): (i64, i64) = db.query_row("SELECT count(*),COALESCE(sum(CASE WHEN i.state IN ('revoked','expired') OR s.incoming_id IS NOT NULL THEN 1 ELSE 0 END),0) FROM channel_incoming i LEFT JOIN channel_memory_suppressions s ON s.channel_id=i.channel_id AND s.incoming_id=i.id WHERE i.channel_id=?1 AND json_extract(i.snapshot,'$.batch_id')=?2", (channel.to_string(), t.request_id.to_string()), |r| Ok((r.get(0)?,r.get(1)?)))?;
    Ok(total > 0 && blocked == 0)
}
pub(super) fn snapshot_current(db: &Connection, t: &ActorTurn) -> Result<bool> {
    Ok(raw_current(db, t)?
        && super::super::private_chat::memory_current(db, t)?
        && super::super::sources::selected_current(db, t)?
        && super::super::experiences::selected_current(db, t)?)
}
/// This only records an already committed effect's exact metadata receipt. It
/// neither replays the effect nor permits provider text or private read results.
pub(super) fn committed_receipt(db: &Connection, t: &ActorTurn, a: &Action) -> Result<bool> {
    if !(a.state == ActionState::Executed && a.tool.private_memory() && a.tool.mutating()) {
        return Ok(false);
    }
    let Some(value) = &a.result else {
        return Ok(false);
    };
    let Ok(effect) = serde_json::from_value::<MemoryEffect>(value.clone()) else {
        return Ok(false);
    };
    if effect.action_id != a.id || effect.actor_turn_id != t.id {
        return Ok(false);
    }
    let raw: Option<String> = db
        .query_row(
            "SELECT snapshot FROM channel_memory_effects WHERE action_id=?1",
            [a.id.to_string()],
            |r| r.get(0),
        )
        .optional()?;
    Ok(raw
        .map(|raw| serde_json::from_str::<serde_json::Value>(&raw))
        .transpose()?
        .as_ref()
        == Some(value))
}
