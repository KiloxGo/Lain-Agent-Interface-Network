//! Durable operational metadata. Provider identities are opaque, never API IDs.
use crate::domain::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct WorkerState {
    pub provider_session_id: Option<String>,
    pub options: SessionOptions,
    pub turns: Vec<TurnRecord>,
    pub approvals: Vec<Approval>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TurnRecord {
    pub turn: Turn,
    pub provider_turn_id: Option<String>,
}

impl WorkerState {
    pub fn validate(&self, session: WorkSessionId) -> anyhow::Result<()> {
        use anyhow::ensure;
        let mut turns = std::collections::BTreeSet::new();
        let mut provider_turns = std::collections::BTreeSet::new();
        for record in &self.turns {
            let turn = &record.turn;
            ensure!(
                turn.work_session_id == session
                    && turn.updated_at_ms >= turn.created_at_ms
                    && turns.insert(turn.id),
                "invalid persisted turn"
            );
            if let Some(id) = &record.provider_turn_id {
                ensure!(
                    !id.is_empty() && id.len() <= 256 && provider_turns.insert(id),
                    "invalid provider turn identity"
                );
            }
        }
        let mut approvals = std::collections::BTreeSet::new();
        for approval in &self.approvals {
            ensure!(
                approval.work_session_id == session
                    && approval.updated_at_ms >= approval.created_at_ms
                    && approvals.insert(approval.id),
                "invalid persisted approval"
            );
            ensure!(
                approval.turn_id.is_none_or(|id| turns.contains(&id)),
                "approval references absent turn"
            );
        }
        ensure!(
            self.provider_session_id
                .as_ref()
                .is_none_or(|id| !id.is_empty() && id.len() <= 256),
            "invalid provider session identity"
        );
        Ok(())
    }

    pub fn recover(&mut self, now: u64) -> Vec<crate::events::EventKind> {
        let mut events = Vec::new();
        for record in &mut self.turns {
            if !record.turn.state.is_terminal() && record.turn.state != TurnState::Unknown {
                record.turn.state = TurnState::Unknown;
                record.turn.updated_at_ms = now.max(record.turn.updated_at_ms);
                events.push(crate::events::EventKind::TurnStateChanged {
                    turn: record.turn.clone(),
                });
            }
        }
        for approval in &mut self.approvals {
            match approval.status {
                ApprovalStatus::Pending => approval.status = ApprovalStatus::Expired,
                ApprovalStatus::Responding => approval.status = ApprovalStatus::Unknown,
                _ => continue,
            }
            approval.updated_at_ms = now.max(approval.updated_at_ms);
            events.push(crate::events::EventKind::ApprovalStateChanged {
                approval: approval.clone(),
            });
        }
        events
    }
}
