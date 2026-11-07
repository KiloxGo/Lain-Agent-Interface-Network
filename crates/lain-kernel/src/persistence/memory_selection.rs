//! Exact user-directed management; unavailable source bodies are never restored here.
use super::{Store, channels::decode, memory};
use crate::{
    actor::{domain::*, tools::Tool},
    backend::ControlError,
    domain::now_ms,
};
use anyhow::{Result, ensure};
use lain_wire::memory::*;
use rusqlite::{Connection, OptionalExtension};

impl Store {
    /// Caller holds the Kernel lock and has just completed the ordinary scoped
    /// current-record read. Only expose a matching User message's clock, never
    /// additional audit text, predecessor bodies or platform-source content.
    pub(crate) fn memory_statement_captures(
        &self,
        records: &[MemoryRecord],
    ) -> Result<Vec<crate::memory::MemoryStatementCapture>> {
        ensure!(records.len() <= 10, ControlError::InvalidInput);
        let mut captures = Vec::new();
        for record in records {
            let MemorySource::ConversationCorrection { evidence } = &record.provenance.source
            else {
                continue;
            };
            let original = match self.actor_turn(evidence.actor_turn_id) {
                Ok(turn) => turn,
                Err(error)
                    if matches!(
                        error.downcast_ref::<ControlError>(),
                        Some(ControlError::NotFound)
                    ) =>
                {
                    continue;
                }
                Err(error) => return Err(error),
            };
            if original.id != evidence.actor_turn_id
                || original.conversation_id != evidence.conversation_id
                || original.reading_activity.is_some()
                || original.briefing_activity.is_some()
                || original.sharing_activity.is_some()
                || record.draft.scope
                    != (MemoryScope::Conversation {
                        conversation_id: evidence.conversation_id,
                    })
            {
                continue;
            }
            let mut matches = original
                .messages
                .iter()
                .filter(|m| m.id == evidence.message_id);
            let Some(message) = matches.next() else {
                continue;
            };
            if matches.next().is_some()
                || message.role != MessageRole::User
                || message.status != MessageStatus::Completed
                || message.conversation_id != evidence.conversation_id
                || !message.content.contains(&record.draft.content)
            {
                continue;
            }
            captures.push(crate::memory::MemoryStatementCapture {
                memory_id: record.id,
                message_id: message.id,
                captured_at_ms: message.timestamp_ms,
            });
        }
        Ok(captures)
    }
}

fn target(
    db: &Connection,
    t: &ActorTurn,
    id: lain_wire::MemoryId,
) -> Result<MemorySelectionTarget> {
    let r = memory::read_memory_snapshot(db, id)?;
    let (conversation_id, expires_at_ms) = match &r.provenance.source {
        MemorySource::BriefingSelection { evidence: e } => (e.conversation_id, e.expires_at_ms),
        MemorySource::ConversationCorrection { evidence: e } => {
            (e.conversation_id, e.expires_at_ms)
        }
        _ => return Err(ControlError::NotFound.into()),
    };
    ensure!(
        r.draft.scope
            == MemoryScope::Conversation {
                conversation_id: t.conversation_id
            }
            && conversation_id == t.conversation_id,
        ControlError::NotFound
    );
    Ok(MemorySelectionTarget {
        memory_id: id,
        conversation_id: t.conversation_id,
        revision: r.revision,
        status: r.status,
        recorded_at_ms: r.recorded_at_ms,
        expires_at_ms,
    })
}
pub(super) fn proposal(
    db: &Connection,
    t: &ActorTurn,
    action: ActionId,
    q: &ChangeMemorySelection,
) -> Result<MemorySelectionReview> {
    ensure!(
        t.reading_activity.is_none()
            && t.briefing_activity.is_none()
            && t.sharing_activity.is_none()
            && matches!(
                q.operation,
                MemoryOperation::Forget | MemoryOperation::Correct
            )
            && !q.quote.trim().is_empty()
            && q.quote.len() <= 2048,
        ControlError::InvalidInput
    );
    crate::profiles::check_sensitive(q)?;
    ensure!(
        t.messages.iter().any(|m| m.id == q.message_id
            && m.role == MessageRole::User
            && m.conversation_id == t.conversation_id
            && m.content.contains(&q.quote)),
        ControlError::InvalidInput
    );
    let own = t
        .actions
        .iter()
        .position(|a| a.id == action)
        .ok_or(ControlError::Conflict)?;
    let current = target(db, t, q.memory_id)?;
    ensure!(
        current.revision == q.expected_revision,
        ControlError::Conflict
    );
    let inspected = t.actions[..own].iter().any(|a| {
        matches!(&a.tool, Tool::MemorySelectionInspect(i) if i.memory_id==q.memory_id)
            && a.state == ActionState::Executed
            && a.result.as_ref().is_some_and(|v| {
                serde_json::from_value::<MemorySelectionTarget>(v.clone())
                    .is_ok_and(|v| v == current)
            })
    });
    ensure!(inspected, ControlError::Conflict);
    let replacement = if q.operation == MemoryOperation::Correct {
        ensure!(
            current.status == MemoryStatus::Active && current.expires_at_ms > now_ms()?,
            ControlError::Unavailable
        );
        Some(replacement(t, action, q, &current)?)
    } else {
        None
    };
    Ok(MemorySelectionReview {
        actor_turn_id: t.id,
        action_id: action,
        request: q.clone(),
        target: current,
        replacement,
    })
}
fn replacement(
    t: &ActorTurn,
    action: ActionId,
    q: &ChangeMemorySelection,
    target: &MemorySelectionTarget,
) -> Result<MemoryCandidate> {
    let c = MemoryCandidate {
        id: lain_wire::CandidateId(action.0),
        draft: MemoryDraft {
            content: q.quote.clone(),
            kind: MemoryKind::Episodic,
            scope: MemoryScope::Conversation {
                conversation_id: t.conversation_id,
            },
        },
        provenance: MemoryProvenance {
            authority: MemoryAuthority::Explicit,
            source: MemorySource::ConversationCorrection {
                evidence: ConversationCorrectionEvidence {
                    conversation_id: t.conversation_id,
                    previous_memory_id: q.memory_id,
                    previous_revision: q.expected_revision,
                    actor_turn_id: t.id,
                    action_id: action,
                    message_id: q.message_id,
                    expires_at_ms: target.expires_at_ms,
                },
            },
        },
        origin: Some(MemoryOrigin::Conversation {
            conversation_id: t.conversation_id,
        }),
        status: CandidateStatus::Candidate,
        memory_id: None,
        created_at_ms: now_ms()?,
        reviewed_at_ms: None,
        actor_action: Some(action),
    };
    crate::memory::validate(&c.draft, &c.provenance)?;
    c.validate().map_err(anyhow::Error::msg)?;
    Ok(c)
}
pub(super) fn candidate_current(db: &Connection, c: &MemoryCandidate) -> Result<()> {
    let MemorySource::ConversationCorrection { evidence: e } = &c.provenance.source else {
        return Ok(());
    };
    let t: ActorTurn = super::decode_actor(&db.query_row(
        "SELECT snapshot FROM lain_actor_turns WHERE id=?1",
        [e.actor_turn_id.to_string()],
        |r| r.get::<_, String>(0),
    )?)?;
    let a = t
        .actions
        .iter()
        .find(|a| a.id == e.action_id)
        .ok_or(ControlError::Conflict)?;
    let Tool::MemorySelectionChange(q) = &a.tool else {
        return Err(ControlError::Conflict.into());
    };
    let expected = proposal(db, &t, a.id, q)?
        .replacement
        .ok_or(ControlError::Conflict)?;
    ensure!(
        expected.draft == c.draft
            && expected.provenance == c.provenance
            && expected.origin == c.origin
            && expected.actor_action == c.actor_action
            && expected.id == c.id,
        ControlError::Conflict
    );
    Ok(())
}
pub(super) fn effect(
    db: &Connection,
    t: &ActorTurn,
    a: &Action,
) -> Result<Option<MemorySelectionEffect>> {
    let Tool::MemorySelectionChange(q) = &a.tool else {
        return Ok(None);
    };
    let row: Option<(String, String)> = db
        .query_row(
            "SELECT request_digest,snapshot FROM conversation_memory_effects WHERE action_id=?1",
            [a.id.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((digest, raw)) = row else {
        return Ok(None);
    };
    let effect: MemorySelectionEffect = decode(raw)?;
    ensure!(
        digest == lain_wire::actor_configuration_revision(&serde_json::to_vec(q)?)
            && effect.action_id == a.id
            && effect.actor_turn_id == t.id
            && effect.conversation_id == t.conversation_id
            && effect.memory_id == q.memory_id
            && effect.previous_revision == q.expected_revision
            && effect.operation == q.operation
            && (effect.replacement_memory_id.is_some()
                == (q.operation == MemoryOperation::Correct)),
        ControlError::Conflict
    );
    Ok(Some(effect))
}
impl Store {
    pub fn selection_available(&self, id: ActorTurnId) -> Result<bool> {
        let t = self.actor_turn(id)?;
        if t.reading_activity.is_some()
            || t.briefing_activity.is_some()
            || t.sharing_activity.is_some()
        {
            return Ok(false);
        }
        let scope = serde_json::to_string(&MemoryScope::Conversation {
            conversation_id: t.conversation_id,
        })?;
        Ok(self.connection.query_row("SELECT EXISTS(SELECT 1 FROM lain_memories WHERE scope=?1 AND json_extract(snapshot,'$.provenance.source.kind') IN ('briefing_selection','conversation_correction'))",[scope],|r|r.get(0))?)
    }
    pub fn selection_inspect(
        &self,
        id: ActorTurnId,
        action: ActionId,
    ) -> Result<MemorySelectionTarget> {
        let t = self.actor_turn(id)?;
        self.private_chat_check(&t)?;
        ensure!(
            t.state == TurnState::Running
                && t.reading_activity.is_none()
                && t.briefing_activity.is_none()
                && t.sharing_activity.is_none(),
            ControlError::Conflict
        );
        let a = t
            .actions
            .iter()
            .find(|a| a.id == action && a.state == ActionState::Executing)
            .ok_or(ControlError::Conflict)?;
        let Tool::MemorySelectionInspect(q) = &a.tool else {
            return Err(ControlError::InvalidInput.into());
        };
        target(&self.connection, &t, q.memory_id)
    }
    pub fn selection_change(
        &mut self,
        id: ActorTurnId,
        action: ActionId,
    ) -> Result<MemorySelectionEffect> {
        let mut t = self.actor_turn(id)?;
        let a = t
            .actions
            .iter()
            .find(|a| a.id == action)
            .ok_or(ControlError::Conflict)?;
        if let Some(e) = effect(&self.connection, &t, a)? {
            return Ok(e);
        }
        self.check_actor_memory(id)?;
        self.private_chat_check(&t)?;
        ensure!(
            t.state == TurnState::Running && a.state == ActionState::Executing,
            ControlError::Conflict
        );
        let Tool::MemorySelectionChange(q) = &a.tool else {
            return Err(ControlError::InvalidInput.into());
        };
        let q = q.clone();
        proposal(&self.connection, &t, action, &q)?;
        let digest = lain_wire::actor_configuration_revision(&serde_json::to_vec(&q)?);
        let mut effect = MemorySelectionEffect {
            actor_turn_id: id,
            action_id: action,
            conversation_id: t.conversation_id,
            memory_id: q.memory_id,
            previous_revision: q.expected_revision,
            operation: q.operation,
            replacement_memory_id: None,
            recorded_at_ms: now_ms()?,
        };
        let tx = self.connection.transaction()?;
        if q.operation == MemoryOperation::Correct {
            // Candidate acceptance and exact predecessor retirement share the effect transaction.
            let candidate = memory::review(&tx, lain_wire::CandidateId(action.0), true)?;
            let new_id = candidate.memory_id.ok_or(ControlError::Conflict)?;
            memory::supersede_selection_tx(&tx, q.memory_id, q.expected_revision, new_id)?;
            super::memory_context::capture_correction_body(
                &tx,
                id,
                &memory::read_memory_snapshot(&tx, new_id)?,
            )?;
            effect.replacement_memory_id = Some(new_id);
        } else {
            memory::delete_memory_tx(&tx, q.memory_id, q.expected_revision)?;
        }
        effect.recorded_at_ms = now_ms()?;
        tx.execute(
            "INSERT INTO conversation_memory_effects VALUES(?1,?2,?3,?4,?5)",
            (
                action.to_string(),
                id.to_string(),
                t.conversation_id.to_string(),
                digest,
                serde_json::to_string(&effect)?,
            ),
        )?;
        let a = t
            .actions
            .iter_mut()
            .find(|a| a.id == action)
            .ok_or(ControlError::Conflict)?;
        a.state = ActionState::Executed;
        a.result = Some(serde_json::to_value(&effect)?);
        t.state = TurnState::Completed;
        if let Some(c) = &mut t.private_chat {
            c.decision = lain_wire::chat::ChatDecision::Notice;
            c.wait_until_ms = None;
        }
        t.message(
            MessageRole::Assistant,
            if q.operation == MemoryOperation::Correct {
                "已按你的原话更正这条事项；仅当前会话有效，原有效期不变。"
            } else {
                "已忘记这条来源事项，不会从原来源自动重新记住。"
            }
            .into(),
        )?;
        // The deletion can invalidate this turn's earlier memory read. Commit only
        // the fixed operational acknowledgement; retain invalidated lineage.
        Self::save_actor_with_commit(&tx, &mut t, false, super::ActorCommit::MemoryEffectNotice)?;
        tx.commit()?;
        Ok(effect)
    }
}
