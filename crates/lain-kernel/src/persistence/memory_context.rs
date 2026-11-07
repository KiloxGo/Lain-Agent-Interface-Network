//! Trusted context and durable semantic-memory dependencies. No filesystem I/O.
use super::{Store, devices, memory, projects};
use crate::{
    actor::{domain::*, tools::Tool},
    backend::ControlError,
    domain::now_ms,
    memory::*,
};
use anyhow::{Result, ensure};
use lain_wire::{MemoryId, devices::DeviceState, memory_context::*, projects::ProjectState};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
mod private;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Version {
    id: MemoryId,
    revision: u64,
    scope: MemoryScope,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActorContext {
    binding: Option<ConversationMemoryContext>,
    dependencies: Vec<Version>,
    /// None denotes an older context whose private history was not tracked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    private_lineage: Option<private::Lineage>,
    /// Full input replacement may retire speech while preserving immutable audit.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    retired_messages: Vec<MessageId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    retired_actions: Vec<ActionId>,
}
#[derive(Debug, thiserror::Error)]
#[error("semantic memory or selected context changed")]
pub(crate) struct ChangedMemoryContext;

fn decode<T: serde::de::DeserializeOwned>(raw: String) -> Result<T> {
    serde_json::from_str(&raw).map_err(|_| anyhow::anyhow!("invalid memory context storage record"))
}
fn context(db: &Connection, id: ConversationId) -> Result<Option<ConversationMemoryContext>> {
    let raw: Option<String> = db
        .query_row(
            "SELECT snapshot FROM conversation_memory_contexts WHERE conversation_id=?1",
            [id.to_string()],
            |r| r.get(0),
        )
        .optional()?;
    raw.map(|raw| {
        let c: ConversationMemoryContext = decode(raw)?;
        ensure!(
            c.conversation_id == id && c.revision > 0 && !c.request_id.0.is_nil(),
            "invalid memory context identity"
        );
        Ok(c)
    })
    .transpose()
}
fn actor(db: &Connection, id: ActorTurnId) -> Result<Option<ActorContext>> {
    let raw: Option<String> = db
        .query_row(
            "SELECT snapshot FROM actor_memory_contexts WHERE actor_turn_id=?1",
            [id.to_string()],
            |r| r.get(0),
        )
        .optional()?;
    raw.map(decode).transpose()
}
fn save(db: &Connection, id: ActorTurnId, c: &ActorContext) -> Result<()> {
    ensure!(c.dependencies.len() <= 64, ControlError::InvalidInput);
    ensure!(
        c.retired_messages.len() <= 32 && c.retired_actions.len() <= 4,
        ControlError::InvalidInput
    );
    ensure!(
        serde_json::to_vec(c)?.len() <= 131072,
        ControlError::InvalidInput
    );
    ensure!(
        db.execute(
            "UPDATE actor_memory_contexts SET snapshot=?2 WHERE actor_turn_id=?1",
            (id.to_string(), serde_json::to_string(c)?)
        )? == 1,
        ControlError::NotFound
    );
    Ok(())
}
fn selection_current(db: &Connection, c: &MemoryContextSelection) -> Result<()> {
    ensure!(c.valid(), ControlError::InvalidInput);
    let d = devices::device(db, c.device_id)?;
    ensure!(
        d.state == DeviceState::Active
            && now_ms()? < d.registration.expires_at_ms
            && d.revision == c.device_revision
            && d.registration
                .workspaces
                .iter()
                .any(|w| w.id == c.workspace_id),
        ChangedMemoryContext
    );
    if let Some(p) = &c.project {
        let current = projects::project(db, p.project_id)?;
        let m =
            projects::workspace(db, c.device_id, c.workspace_id)?.ok_or(ChangedMemoryContext)?;
        ensure!(
            current.state == ProjectState::Active
                && current.revision == p.project_revision
                && m.project_id == Some(p.project_id)
                && m.revision == p.mapping_revision
                && m.device_revision == c.device_revision
                && m.reviewed_project_revision == Some(p.project_revision),
            ChangedMemoryContext
        );
    }
    Ok(())
}
pub(super) fn scope_exists(db: &Connection, scope: &MemoryScope) -> Result<()> {
    match scope {
        MemoryScope::Conversation { conversation_id } => ensure!(
            db.query_row(
                "SELECT EXISTS(SELECT 1 FROM lain_conversations WHERE id=?1)",
                [conversation_id.to_string()],
                |r| r.get::<_, bool>(0)
            )?,
            ControlError::NotFound
        ),
        MemoryScope::Project { project_id } => ensure!(
            projects::project(db, *project_id)?.state == ProjectState::Active,
            ControlError::InvalidInput
        ),
        MemoryScope::Device { device_id } | MemoryScope::DeviceWorkspace { device_id, .. } => {
            let d = devices::device(db, *device_id)?;
            ensure!(
                d.state == DeviceState::Active && now_ms()? < d.registration.expires_at_ms,
                ControlError::InvalidInput
            );
            if let MemoryScope::DeviceWorkspace { workspace_id, .. } = scope {
                ensure!(
                    d.registration
                        .workspaces
                        .iter()
                        .any(|w| w.id == *workspace_id),
                    ControlError::InvalidInput
                );
            }
        }
        _ => {}
    }
    Ok(())
}
fn permits(c: &ActorContext, conversation: ConversationId, scope: &MemoryScope) -> bool {
    let selection = c.binding.as_ref().and_then(|b| b.selection.as_ref());
    match scope {
        MemoryScope::Conversation { conversation_id } => *conversation_id == conversation,
        MemoryScope::Global | MemoryScope::Internal => true,
        MemoryScope::Workspace { .. } => selection.is_none(), // retained local-only legacy contract
        MemoryScope::Project { project_id } => selection.is_some_and(|s| {
            s.project
                .as_ref()
                .is_some_and(|p| p.project_id == *project_id)
        }),
        MemoryScope::Device { device_id } => selection.is_some_and(|s| s.device_id == *device_id),
        MemoryScope::DeviceWorkspace {
            device_id,
            workspace_id,
        } => {
            selection.is_some_and(|s| s.device_id == *device_id && s.workspace_id == *workspace_id)
        }
        MemoryScope::Channel { .. } => false, // separate source/grant checks own channel access
    }
}
fn current(
    db: &Connection,
    conversation: ConversationId,
    c: &ActorContext,
    live_binding: bool,
) -> Result<()> {
    if live_binding {
        ensure!(c.private_lineage.is_some(), ChangedMemoryContext);
        ensure!(
            context(db, conversation)? == c.binding,
            ChangedMemoryContext
        );
    }
    if let Some(private) = &c.private_lineage {
        private.current(db)?;
    }
    if let Some(s) = c.binding.as_ref().and_then(|b| b.selection.as_ref()) {
        selection_current(db, s)?;
    }
    let mut visited = Vec::new();
    let mut verified = Vec::new();
    for v in &c.dependencies {
        version_current(db, v, &mut visited, &mut verified)?;
    }
    Ok(())
}
pub(super) fn publication_current(db: &Connection, turn: &ActorTurn) -> Result<bool> {
    if super::briefing_model::source_current(db, turn).is_err() {
        return Ok(false);
    }
    let Some(c) = actor(db, turn.id)? else {
        return Ok(false);
    };
    let route = private::route(db, turn)?;
    Ok(c.private_lineage.as_ref().is_some_and(|p| p.tracked(route))
        && current(db, turn.conversation_id, &c, true).is_ok()
        && private::snapshot_current(db, turn)?)
}
fn version_current(
    db: &Connection,
    v: &Version,
    visited: &mut Vec<MemoryId>,
    verified: &mut Vec<Version>,
) -> Result<()> {
    if verified.contains(v) {
        return Ok(());
    }
    ensure!(
        verified.len() < 256 && visited.len() < 64 && !visited.contains(&v.id),
        ChangedMemoryContext
    );
    visited.push(v.id);
    let r = memory::read_memory_snapshot(db, v.id)?;
    ensure!(
        r.revision == v.revision
            && r.draft.scope == v.scope
            && r.status == MemoryStatus::Active
            && super::archive_memory::current(db, &r, now_ms()?)?,
        ChangedMemoryContext
    );
    derived_current_inner(db, &r, visited, verified)?;
    visited.pop();
    ensure!(verified.len() < 256, ChangedMemoryContext);
    verified.push(v.clone());
    Ok(())
}
fn derived_current_inner(
    db: &Connection,
    r: &MemoryRecord,
    visited: &mut Vec<MemoryId>,
    verified: &mut Vec<Version>,
) -> Result<()> {
    if let MemorySource::LainGenerated { actor_turn_id, .. } = r.provenance.source
        && let Some(c) = actor(db, actor_turn_id)?
    {
        if let Some(private) = &c.private_lineage {
            private.current(db)?;
        }
        if let Some(s) = c.binding.as_ref().and_then(|b| b.selection.as_ref()) {
            selection_current(db, s)?;
        }
        for v in &c.dependencies {
            version_current(db, v, visited, verified)?;
        }
    }
    Ok(())
}
pub(super) fn derived_current(db: &Connection, r: &MemoryRecord) -> Result<bool> {
    match derived_current_inner(db, r, &mut vec![r.id], &mut vec![]) {
        Ok(()) => Ok(true),
        Err(e) if stale(&e) => Ok(false),
        Err(e) => Err(e),
    }
}
fn model_record_current(db: &Connection, r: &MemoryRecord) -> Result<bool> {
    if let MemorySource::LainGenerated { actor_turn_id, .. } = r.provenance.source {
        let Some(lineage) = actor(db, actor_turn_id)?.and_then(|c| c.private_lineage) else {
            return Ok(false);
        };
        let raw: String = db.query_row(
            "SELECT snapshot FROM lain_actor_turns WHERE id=?1",
            [actor_turn_id.to_string()],
            |r| r.get(0),
        )?;
        if !lineage.tracked(private::route(db, &super::decode_actor(&raw)?)?) {
            return Ok(false);
        }
    }
    derived_current(db, r)
}
fn stale(e: &anyhow::Error) -> bool {
    e.is::<ChangedMemoryContext>()
        || matches!(
            e.downcast_ref::<ControlError>(),
            Some(ControlError::NotFound | ControlError::Unavailable)
        )
}
pub(super) fn origin(db: &Connection, source: &MemorySource) -> Result<MemoryOrigin> {
    Ok(match source {
        MemorySource::ConversationCorrection { evidence } => MemoryOrigin::Conversation {
            conversation_id: evidence.conversation_id,
        },
        MemorySource::BriefingSelection { evidence } => MemoryOrigin::Conversation {
            conversation_id: evidence.conversation_id,
        },
        MemorySource::ExplicitOwnerStatement { .. } => MemoryOrigin::OwnerApi,
        MemorySource::LainGenerated { actor_turn_id, .. } => {
            let raw: String = db.query_row(
                "SELECT snapshot FROM lain_actor_turns WHERE id=?1",
                [actor_turn_id.to_string()],
                |r| r.get(0),
            )?;
            let turn = super::decode_actor(&raw)?;
            let channel = turn
                .private_chat
                .as_ref()
                .map(|c| c.channel_id)
                .or_else(|| turn.channel_memory.as_ref().map(|c| c.channel_id))
                .or_else(|| turn.channel_source.as_ref().map(|c| c.channel_id));
            if let Some(channel_id) = channel {
                MemoryOrigin::Channel { channel_id }
            } else {
                MemoryOrigin::OwnerSelectedActorContext {
                    selection: actor(db, *actor_turn_id)?
                        .and_then(|c| c.binding.and_then(|b| b.selection)),
                }
            }
        }
        MemorySource::ChannelSelection { channel_id, .. } => MemoryOrigin::Channel {
            channel_id: *channel_id,
        },
        MemorySource::ArchiveSelection { evidence } => MemoryOrigin::Channel {
            channel_id: evidence.channel_id,
        },
        MemorySource::SessionSummary { summary_id } => MemoryOrigin::SessionSummary {
            summary_id: *summary_id,
        },
    })
}
fn briefing_dependencies(db: &Connection, c: &ActorContext) -> Result<bool> {
    for v in &c.dependencies {
        if matches!(
            memory::read_memory_snapshot(db, v.id)?.provenance.source,
            MemorySource::BriefingSelection { .. } | MemorySource::ConversationCorrection { .. }
        ) {
            return Ok(true);
        }
    }
    Ok(false)
}
pub(super) fn candidate_allowed(
    db: &Connection,
    turn: &ActorTurn,
    draft: &MemoryDraft,
) -> Result<()> {
    ensure!(
        !super::briefing_conversation::tainted(db, turn.id)?,
        ControlError::InvalidInput
    );
    let c = actor(db, turn.id)?.ok_or(ChangedMemoryContext)?;
    ensure!(!briefing_dependencies(db, &c)?, ControlError::InvalidInput);
    current(db, turn.conversation_id, &c, true)?;
    ensure!(
        c.private_lineage
            .as_ref()
            .ok_or(ChangedMemoryContext)?
            .tracked(private::route(db, turn)?),
        ChangedMemoryContext
    );
    ensure!(
        !c.private_lineage
            .as_ref()
            .ok_or(ChangedMemoryContext)?
            .tainted,
        ControlError::InvalidInput
    );
    ensure!(
        permits(&c, turn.conversation_id, &draft.scope),
        ControlError::InvalidInput
    );
    // A boolean acceptance of a candidate is not a cross-scope disclosure grant.
    if c.binding
        .as_ref()
        .and_then(|b| b.selection.as_ref())
        .is_some()
    {
        ensure!(
            !matches!(
                draft.scope,
                MemoryScope::Global | MemoryScope::Internal | MemoryScope::Workspace { .. }
            ),
            ControlError::InvalidInput
        );
    }
    for v in &c.dependencies {
        ensure!(
            matches!(v.scope, MemoryScope::Global) || v.scope == draft.scope,
            ControlError::InvalidInput
        );
    }
    Ok(())
}
pub(super) fn sync_actor(
    tx: &Transaction<'_>,
    turn: &ActorTurn,
    create: bool,
    commit: super::ActorCommit,
) -> Result<()> {
    for a in &turn.actions {
        if matches!(a.state, ActionState::Proposed | ActionState::Executing)
            && let Tool::MemorySelectionChange(q) = &a.tool
        {
            super::memory_selection::proposal(tx, turn, a.id, q)?;
        }
    }
    if create {
        let binding = context(tx, turn.conversation_id)?;
        if let Some(s) = binding.as_ref().and_then(|b| b.selection.as_ref()) {
            ensure!(
                turn.private_chat.is_none()
                    && turn.channel_memory.is_none()
                    && turn.channel_source.is_none()
                    && turn.reading_activity.is_none()
                    && turn.briefing_activity.is_none()
                    && turn.sharing_activity.is_none(),
                ControlError::InvalidInput
            );
            selection_current(tx, s)?;
        }
        tx.execute(
            "INSERT INTO actor_memory_contexts VALUES(?1,?2)",
            (
                turn.id.to_string(),
                serde_json::to_string(&ActorContext {
                    binding,
                    dependencies: vec![],
                    private_lineage: Some(private::Lineage::new(tx, turn)?),
                    retired_messages: vec![],
                    retired_actions: vec![],
                })?,
            ),
        )?;
    } else {
        let old = super::decode_actor(&tx.query_row(
            "SELECT snapshot FROM lain_actor_turns WHERE id=?1",
            [turn.id.to_string()],
            |r| r.get::<_, String>(0),
        )?)?;
        let publishes = turn.model_requests > old.model_requests
            || (old.state == TurnState::Running && turn.state == TurnState::Completed)
            || turn.actions.len() > old.actions.len()
            || turn
                .messages
                .iter()
                .filter(|m| m.role == MessageRole::Assistant)
                .count()
                > old
                    .messages
                    .iter()
                    .filter(|m| m.role == MessageRole::Assistant)
                    .count()
            || turn
                .actions
                .iter()
                .zip(&old.actions)
                .map(|(a, b)| {
                    Ok(a.state != b.state
                        && matches!(a.state, ActionState::Executing | ActionState::Executed)
                        && !private::committed_receipt(tx, turn, a)?)
                })
                .collect::<Result<Vec<bool>>>()?
                .into_iter()
                .any(|changed| changed);
        if publishes
            && commit == super::ActorCommit::Normal
            && let Some(c) = actor(tx, turn.id)?
        {
            current(tx, turn.conversation_id, &c, true)?;
        }
        if let Some(mut c) = actor(tx, turn.id)?
            && let Some(private) = &mut c.private_lineage
        {
            let was_tainted = private.tainted;
            let adopted = private.adopt(tx, turn)?;
            let disclosed = private.observe(turn)?;
            private.tainted |= private::influenced(turn);
            if disclosed || adopted || was_tainted != private.tainted {
                save(tx, turn.id, &c)?;
            }
        }
    }
    Ok(())
}

fn pending_briefing_current(db: &Connection, turn: &ActorTurn) -> Result<()> {
    for action in &turn.actions {
        if action.state == ActionState::Proposed
            && let Tool::MemorySelectionChange(q) = &action.tool
        {
            super::memory_selection::proposal(db, turn, action.id, q)?;
        }
        if action.state == ActionState::Proposed && matches!(action.tool, Tool::BriefingRemember(_))
        {
            let candidate: MemoryCandidate = super::channels::decode(db.query_row(
                "SELECT snapshot FROM lain_memory_candidates WHERE id=?1",
                [action.id.to_string()],
                |r| r.get::<_, String>(0),
            )?)?;
            super::briefing_memory::candidate_current(db, &candidate)?;
        }
    }
    Ok(())
}

impl Store {
    /// Source checks and the projection share the Kernel lock. Raw records stay
    /// available to internal audit/recovery, never to ordinary network restores.
    pub fn actor_view(&self, id: ActorTurnId) -> Result<lain_wire::ActorSnapshot> {
        let turn = self.actor_turn(id)?;
        let mut view: lain_wire::ActorSnapshot =
            serde_json::from_value(serde_json::to_value(&turn)?)?;
        for a in &turn.actions {
            if let Some(receipt) = super::memory_selection::effect(&self.connection, &turn, a)? {
                view.memory_change_receipts.push(receipt);
            }
            if let Some(receipt) = super::briefing_memory::receipt(&self.connection, &turn, a)? {
                view.reviewed_memory_receipts.push(receipt);
            }
            if private::committed_receipt(&self.connection, &turn, a)? {
                view.effect_receipts.push(serde_json::from_value(
                    a.result.clone().ok_or(ControlError::Conflict)?,
                )?);
            }
        }
        let available = (|| -> Result<()> {
            pending_briefing_current(&self.connection, &turn)?;
            ensure!(
                publication_current(&self.connection, &turn)?,
                ChangedMemoryContext
            );
            if turn.reading_activity.is_some() {
                self.reading_actor_source_current(&turn)?;
            }
            if turn.sharing_activity.is_some() {
                self.share_actor_source_current(&turn)?;
            }
            Ok(())
        })()
        .is_ok();
        if !available {
            view.availability = Some(lain_wire::ActorAvailability::SourceUnavailable);
            view.messages.clear();
            view.actions.clear();
            view.private_chat = None;
            view.channel_memory = None;
            view.channel_source = None;
            view.error = Some("Source unavailable; previous content withheld.".into());
        } else {
            if let Some(c) = actor(&self.connection, id)? {
                view.messages
                    .retain(|m| !c.retired_messages.contains(&m.id));
                view.actions.retain(|a| !c.retired_actions.contains(&a.id));
            }
            // Private source tool calls are audit data. Expose only separately
            // verified committed effect metadata, without arguments/read results.
            view.actions.retain(|a| {
                !turn.actions.iter().any(|raw| {
                    raw.id == a.id
                        && (raw.tool.private_memory() || raw.tool.source_read())
                        && !(matches!(
                            raw.tool,
                            Tool::BriefingRemember(_) | Tool::MemorySelectionChange(_)
                        ) && matches!(
                            raw.state,
                            ActionState::Proposed | ActionState::Executing
                        ))
                })
            });
            for a in &mut view.actions {
                if a.state == lain_wire::ActionState::Proposed
                    && let lain_wire::ToolCall::MemorySelectionChange(q) = &a.tool
                {
                    let mut review =
                        super::memory_selection::proposal(&self.connection, &turn, a.id, q)?;
                    if q.operation == MemoryOperation::Correct {
                        let c = super::memory::read_candidate(
                            &self.connection,
                            lain_wire::CandidateId(a.id.0),
                        )?;
                        super::memory_selection::candidate_current(&self.connection, &c)?;
                        review.replacement = Some(c);
                    }
                    a.memory_change_review = Some(review);
                }
                if matches!(a.tool, lain_wire::ToolCall::BriefingRemember(_))
                    && a.state == lain_wire::ActionState::Proposed
                {
                    let candidate = self.candidate(lain_wire::CandidateId(a.id.0))?;
                    if super::briefing_memory::candidate_current(&self.connection, &candidate)
                        .is_ok()
                    {
                        a.memory_review = Some(candidate);
                    }
                }
            }
            view.messages.retain(|m| m.role != MessageRole::Tool);
        }
        view.validate().map_err(anyhow::Error::msg)?;
        Ok(view)
    }
    /// Closed operational notice: accepts no provider/user-supplied message text,
    /// preserves all invalidated lineage, and never authorizes another model call.
    pub fn actor_source_notice(
        &mut self,
        id: ActorTurnId,
        usage: Option<(u64, lain_wire::ModelTiming)>,
    ) -> Result<ActorTurn> {
        let mut turn = self.actor_turn(id)?;
        let pending = turn.state == TurnState::PendingConfirmation
            && turn.actions.iter().any(|a| {
                a.state == ActionState::Proposed
                    && matches!(
                        a.tool,
                        Tool::BriefingRemember(_) | Tool::MemorySelectionChange(_)
                    )
            });
        ensure!(
            turn.state == TurnState::Running || pending,
            ControlError::Conflict
        );
        if pending {
            for a in &mut turn.actions {
                if a.state == ActionState::Proposed
                    && matches!(
                        a.tool,
                        Tool::BriefingRemember(_) | Tool::MemorySelectionChange(_)
                    )
                {
                    a.state = ActionState::Rejected;
                }
            }
        }
        if let Some((tokens, timing)) = usage {
            turn.usage_tokens = turn.usage_tokens.saturating_add(tokens);
            turn.timing.requests.push(timing);
        }
        turn.state = TurnState::Completed;
        if let Some(c) = &mut turn.private_chat {
            c.decision = lain_wire::chat::ChatDecision::Notice;
            c.wait_until_ms = None;
        }
        turn.message(MessageRole::Assistant, "图片、记忆或历史资料的权限、来源状态已改变，本轮没有继续引用。请重新发图或在当前范围内提问。".into())?;
        let tx = self.connection.transaction()?;
        Self::save_actor_with_commit(&tx, &mut turn, false, super::ActorCommit::SourceNotice)?;
        tx.commit()?;
        Ok(turn)
    }
    pub fn memory_context(&self, id: ConversationId) -> Result<Option<ConversationMemoryContext>> {
        self.actor_page(id, 0, 1)?;
        context(&self.connection, id)
    }
    pub fn set_memory_context(
        &mut self,
        id: ConversationId,
        input: SetMemoryContext,
    ) -> Result<ConversationMemoryContext> {
        ensure!(
            !input.request_id.0.is_nil() && input.expected_revision < i64::MAX as u64,
            ControlError::InvalidInput
        );
        devices::safe(&input)?;
        self.actor_page(id, 0, 1)?;
        let digest = lain_wire::actor_configuration_revision(&serde_json::to_vec(&input)?);
        let tx = self.connection.transaction()?;
        let receipt: Option<(String,String,String)> = tx.query_row("SELECT conversation_id,payload_hash,snapshot FROM memory_context_requests WHERE request_id=?1", [input.request_id.to_string()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        if let Some((conversation, hash, raw)) = receipt {
            ensure!(
                conversation == id.to_string() && hash == digest,
                ControlError::Conflict
            );
            return decode(raw);
        }
        ensure!(
            context(&tx, id)?.map_or(0, |c| c.revision) == input.expected_revision,
            ControlError::Conflict
        );
        let active: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM lain_actor_turns WHERE conversation_id=?1 AND json_extract(snapshot,'$.state') IN ('running','pending_confirmation'))", [id.to_string()], |r| r.get(0))?;
        ensure!(!active, ControlError::Conflict);
        // A channel-owned conversation cannot gain ambient PC/project authority.
        let channel: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM channel_bindings WHERE json_extract(snapshot,'$.conversation_id')=?1)", [id.to_string()], |r| r.get(0))?;
        ensure!(
            !channel || input.selection.is_none(),
            ControlError::InvalidInput
        );
        if let Some(s) = &input.selection {
            selection_current(&tx, s)?;
        }
        let c = ConversationMemoryContext {
            conversation_id: id,
            revision: input.expected_revision + 1,
            request_id: input.request_id,
            selection: input.selection,
            changed_at_ms: now_ms()?,
        };
        let raw = serde_json::to_string(&c)?;
        tx.execute("INSERT INTO conversation_memory_contexts VALUES(?1,?2) ON CONFLICT(conversation_id) DO UPDATE SET snapshot=excluded.snapshot", (id.to_string(), &raw))?;
        tx.execute(
            "INSERT INTO memory_context_requests VALUES(?1,?2,?3,?4)",
            params![input.request_id.to_string(), id.to_string(), digest, raw],
        )?;
        tx.commit()?;
        Ok(c)
    }
    pub fn check_actor_memory(&self, id: ActorTurnId) -> Result<()> {
        let turn = self.actor_turn(id)?;
        pending_briefing_current(&self.connection, &turn)?;
        super::briefing_model::source_current(&self.connection, &turn)?;
        if let Some(c) = actor(&self.connection, id)? {
            ensure!(
                c.private_lineage
                    .as_ref()
                    .ok_or(ChangedMemoryContext)?
                    .tracked(private::route(&self.connection, &turn)?),
                ChangedMemoryContext
            );
            current(&self.connection, turn.conversation_id, &c, true)?;
        }
        Ok(())
    }
    pub fn actor_memory_requires_hold(&self, id: ActorTurnId) -> Result<bool> {
        if super::briefing_conversation::tainted(&self.connection, id)? {
            return Ok(true);
        }
        Ok(actor(&self.connection, id)?.is_some_and(|c| {
            !c.dependencies.is_empty()
                || c.binding.and_then(|b| b.selection).is_some()
                || c.private_lineage.is_some_and(|p| {
                    p.tainted || !p.inherited.is_empty() || !p.disclosed.is_empty()
                })
        }))
    }
    pub fn actor_memory_proposal_available(&self, id: ActorTurnId) -> Result<bool> {
        if super::briefing_conversation::tainted(&self.connection, id)? {
            return Ok(false);
        }
        if let Some(c) = actor(&self.connection, id)?
            && briefing_dependencies(&self.connection, &c)?
        {
            return Ok(false);
        }
        Ok(actor(&self.connection, id)?
            .and_then(|c| c.private_lineage)
            .is_some_and(|p| !p.tainted))
    }
    pub fn actor_memory_scopes(&self, id: ActorTurnId) -> Result<Vec<MemoryScope>> {
        self.check_actor_memory(id)?;
        let conversation_id = self.actor_turn(id)?.conversation_id;
        let mut scopes = vec![
            MemoryScope::Global,
            MemoryScope::Internal,
            MemoryScope::Conversation { conversation_id },
        ];
        if let Some(s) =
            actor(&self.connection, id)?.and_then(|c| c.binding.and_then(|b| b.selection))
        {
            scopes.push(MemoryScope::Device {
                device_id: s.device_id,
            });
            scopes.push(MemoryScope::DeviceWorkspace {
                device_id: s.device_id,
                workspace_id: s.workspace_id,
            });
            if let Some(p) = s.project {
                scopes.push(MemoryScope::Project {
                    project_id: p.project_id,
                });
            }
        }
        Ok(scopes)
    }
    pub fn actor_memory_page(&mut self, id: ActorTurnId, q: &MemoryQuery) -> Result<MemoryPage> {
        self.check_actor_memory(id)?;
        let mut c = actor(&self.connection, id)?.ok_or(ChangedMemoryContext)?;
        ensure!(
            permits(&c, self.actor_turn(id)?.conversation_id, &q.scope)
                && q.limit <= 10
                && q.status.is_none_or(|s| s == MemoryStatus::Active),
            ControlError::InvalidInput
        );
        let mut page = self.memory_page(q)?;
        let mut visible = Vec::new();
        for r in page.records {
            if model_record_current(&self.connection, &r)? {
                visible.push(r);
            }
        }
        page.records = visible;
        for r in &page.records {
            remember(&mut c, r)?;
        }
        save(&self.connection, id, &c)?;
        Ok(page)
    }
    pub fn actor_memory_inspect(
        &mut self,
        id: ActorTurnId,
        memory_id: MemoryId,
        scope: &MemoryScope,
    ) -> Result<MemoryRecord> {
        self.check_actor_memory(id)?;
        let mut c = actor(&self.connection, id)?.ok_or(ChangedMemoryContext)?;
        ensure!(
            permits(&c, self.actor_turn(id)?.conversation_id, scope),
            ControlError::InvalidInput
        );
        let r = self.memory(memory_id)?;
        ensure!(
            model_record_current(&self.connection, &r)?,
            ControlError::Unavailable
        );
        ensure!(
            r.draft.scope == *scope && r.status == MemoryStatus::Active,
            ControlError::NotFound
        );
        remember(&mut c, &r)?;
        save(&self.connection, id, &c)?;
        Ok(r)
    }
    pub fn memory_context_history(
        &mut self,
        id: ActorTurnId,
        mut history: Vec<ActorTurn>,
    ) -> Result<Vec<ActorTurn>> {
        let this = self.actor_turn(id)?;
        let this_route = private::route(&self.connection, &this)?;
        let mut c = actor(&self.connection, id)?.ok_or(ChangedMemoryContext)?;
        // Only called when replacing the entire model input, never for frontend
        // history reads. Drop inherited leases, retain private influence and all
        // direct semantic-memory versions, then validate the rebuilt selection.
        let input = history
            .iter()
            .find(|t| t.id == id)
            .ok_or(ChangedMemoryContext)?;
        for m in &this.messages {
            if m.role != MessageRole::User
                && !input.messages.iter().any(|kept| kept.id == m.id)
                && !c.retired_messages.contains(&m.id)
            {
                c.retired_messages.push(m.id);
            }
        }
        for a in &this.actions {
            if !input.actions.iter().any(|kept| kept.id == a.id)
                && !c.retired_actions.contains(&a.id)
            {
                c.retired_actions.push(a.id);
            }
        }
        c.private_lineage
            .as_mut()
            .ok_or(ChangedMemoryContext)?
            .inherited
            .clear();
        c.private_lineage
            .as_mut()
            .ok_or(ChangedMemoryContext)?
            .disclosed
            .clear();
        current(&self.connection, this.conversation_id, &c, true)?;
        for past in history.iter_mut().rev() {
            if past.id == id {
                past.messages
                    .retain(|m| !c.retired_messages.contains(&m.id));
                past.actions.retain(|a| !c.retired_actions.contains(&a.id));
                continue;
            }
            if private::route(&self.connection, past)? != this_route
                || !private::snapshot_current(&self.connection, past)?
            {
                past.messages.clear();
                past.actions.clear();
                continue;
            }
            let old = actor(&self.connection, past.id)?;
            let valid = if let Some(old) = &old {
                if old.binding != c.binding {
                    false
                } else {
                    match current(&self.connection, this.conversation_id, old, false) {
                        Ok(()) => true,
                        Err(e) if stale(&e) => false,
                        Err(e) => return Err(e),
                    }
                }
            } else {
                // Legacy replies may have inherited an earlier tool result even
                // without performing a read themselves. Missing lineage cannot
                // establish that any generated content is still disclosable.
                past.messages.retain(|m| m.role == MessageRole::User);
                past.actions.clear();
                past.error = None;
                c.binding.is_none()
            };
            if !valid {
                past.messages.clear();
                past.actions.clear();
                continue;
            }
            if let Some(old) = old {
                past.messages
                    .retain(|m| !old.retired_messages.contains(&m.id));
                past.actions
                    .retain(|a| !old.retired_actions.contains(&a.id));
                if old.private_lineage.is_none() {
                    past.messages.retain(|m| m.role == MessageRole::User);
                    past.actions.clear();
                }
                // Leave room for this turn's fresh bounded retrieval. A historical
                // reply is indivisible evidence, so omit it rather than dropping
                // only some of its source restrictions.
                let additional = old
                    .dependencies
                    .iter()
                    .filter(|v| !c.dependencies.iter().any(|d| d.id == v.id))
                    .count();
                if c.dependencies.len() + additional > 48 {
                    past.messages.clear();
                    past.actions.clear();
                    continue;
                }
                if let Some(prior) = &old.private_lineage
                    && !c
                        .private_lineage
                        .as_mut()
                        .ok_or(ChangedMemoryContext)?
                        .inherit(prior, past, this_route)?
                {
                    past.messages.clear();
                    past.actions.clear();
                    continue;
                }
                for v in old.dependencies {
                    merge(&mut c, v)?;
                }
            }
        }
        save(&self.connection, id, &c)?;
        Ok(history)
    }
}
fn merge(c: &mut ActorContext, v: Version) -> Result<()> {
    if let Some(old) = c.dependencies.iter().find(|d| d.id == v.id) {
        ensure!(*old == v, ChangedMemoryContext);
    } else {
        ensure!(c.dependencies.len() < 64, ControlError::InvalidInput);
        c.dependencies.push(v);
    }
    Ok(())
}
fn remember(c: &mut ActorContext, r: &MemoryRecord) -> Result<()> {
    merge(
        c,
        Version {
            id: r.id,
            revision: r.revision,
            scope: r.draft.scope.clone(),
        },
    )
}
/// A reviewed correction's quoted user body is also the new memory content.
/// Keep the immutable audit, but withdraw its ordinary/model projection when
/// that record is superseded, forgotten or expires. This creates no source grant.
pub(super) fn capture_correction_body(
    db: &Connection,
    id: ActorTurnId,
    r: &MemoryRecord,
) -> Result<()> {
    let MemorySource::ConversationCorrection { evidence: e } = &r.provenance.source else {
        return Err(ControlError::InvalidInput.into());
    };
    ensure!(e.actor_turn_id == id, ControlError::InvalidInput);
    let mut c = actor(db, id)?.ok_or(ChangedMemoryContext)?;
    remember(&mut c, r)?;
    save(db, id, &c)
}
