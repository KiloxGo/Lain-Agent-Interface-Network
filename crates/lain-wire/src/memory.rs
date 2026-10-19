//! Lain-owned semantic memory. These records are evidence, never permissions.
use crate::{ActionId, ActorTurnId, CandidateId, ConversationId, MemoryId, RequestId, SummaryId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    Episodic,
    ProjectKnowledge,
    Preference,
    Reflection,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MemoryScope {
    Global,
    Workspace { path: String },
    Internal,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryStatus {
    Active,
    Disabled,
    Superseded,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MemorySource {
    /// Exact text supplied through the authenticated owner memory API.
    ExplicitOwnerStatement { request_id: RequestId },
    /// Model interpretation, even when accepted by the owner.
    LainGenerated {
        conversation_id: ConversationId,
        actor_turn_id: ActorTurnId,
        action_id: ActionId,
    },
    /// Owner-selected claim supported by a bounded factual summary, not verified truth.
    SessionSummary { summary_id: SummaryId },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryAuthority {
    Explicit,
    Inferred,
    UnverifiedSelection,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryProvenance {
    pub source: MemorySource,
    pub authority: MemoryAuthority,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryDraft {
    pub content: String,
    pub kind: MemoryKind,
    pub scope: MemoryScope,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub id: MemoryId,
    #[serde(flatten)]
    pub draft: MemoryDraft,
    pub provenance: MemoryProvenance,
    pub status: MemoryStatus,
    pub superseded_by: Option<MemoryId>,
    pub recorded_at_ms: u64,
    pub owner_confirmed_at_ms: u64,
    pub updated_at_ms: u64,
    pub revision: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateStatus {
    Candidate,
    Accepted,
    Rejected,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryCandidate {
    pub id: CandidateId,
    pub draft: MemoryDraft,
    pub provenance: MemoryProvenance,
    pub status: CandidateStatus,
    pub memory_id: Option<MemoryId>,
    pub created_at_ms: u64,
    pub reviewed_at_ms: Option<u64>,
    /// Actor candidates are reviewed only through the original Actor action.
    pub actor_action: Option<ActionId>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddMemory {
    pub request_id: RequestId,
    pub draft: MemoryDraft,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposeMemory {
    pub draft: MemoryDraft,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryQuery {
    pub scope: MemoryScope,
    pub kind: Option<MemoryKind>,
    pub status: Option<MemoryStatus>,
    pub text: Option<String>,
    pub after: u64,
    pub limit: u16,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPage {
    pub records: Vec<MemoryRecord>,
    pub next_cursor: u64,
    pub has_more: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeMemory {
    pub expected_revision: u64,
    pub status: MemoryStatus,
    pub superseded_by: Option<MemoryId>,
}

impl MemoryProvenance {
    pub fn validate(&self) -> Result<(), &'static str> {
        if matches!(
            (&self.source, self.authority),
            (
                MemorySource::ExplicitOwnerStatement { .. },
                MemoryAuthority::Explicit
            ) | (
                MemorySource::LainGenerated { .. },
                MemoryAuthority::Inferred
            ) | (
                MemorySource::SessionSummary { .. },
                MemoryAuthority::UnverifiedSelection
            )
        ) {
            Ok(())
        } else {
            Err("contradictory memory authority")
        }
    }
}
impl MemoryRecord {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.provenance.validate()?;
        if self.revision == 0
            || self.updated_at_ms < self.recorded_at_ms
            || self.owner_confirmed_at_ms < self.recorded_at_ms
            || (self.status == MemoryStatus::Superseded) != self.superseded_by.is_some()
            || self.superseded_by == Some(self.id)
        {
            return Err("contradictory memory lifecycle");
        }
        Ok(())
    }
}

impl MemoryCandidate {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.provenance.validate()?;
        let valid_review = match self.status {
            CandidateStatus::Candidate => self.memory_id.is_none() && self.reviewed_at_ms.is_none(),
            CandidateStatus::Rejected => self.memory_id.is_none() && self.reviewed_at_ms.is_some(),
            CandidateStatus::Accepted => self.memory_id.is_some() && self.reviewed_at_ms.is_some(),
        };
        let valid_action = match self.provenance.source {
            MemorySource::LainGenerated { action_id, .. } => self.actor_action == Some(action_id),
            _ => self.actor_action.is_none(),
        };
        if !valid_review
            || !valid_action
            || self.reviewed_at_ms.is_some_and(|t| t < self.created_at_ms)
        {
            return Err("contradictory memory candidate lifecycle");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn candidate_review_cannot_disagree_with_origin_or_receipt() {
        let action = ActionId::new();
        let mut candidate = MemoryCandidate {
            id: CandidateId(action.0),
            draft: MemoryDraft {
                content: "selected experience".into(),
                kind: MemoryKind::Episodic,
                scope: MemoryScope::Global,
            },
            provenance: MemoryProvenance {
                source: MemorySource::LainGenerated {
                    conversation_id: ConversationId::new(),
                    actor_turn_id: ActorTurnId::new(),
                    action_id: action,
                },
                authority: MemoryAuthority::Inferred,
            },
            status: CandidateStatus::Candidate,
            memory_id: None,
            created_at_ms: 1,
            reviewed_at_ms: None,
            actor_action: Some(action),
        };
        candidate.validate().unwrap();
        candidate.provenance.authority = MemoryAuthority::Explicit;
        assert!(candidate.validate().is_err());
        candidate.provenance.authority = MemoryAuthority::Inferred;
        candidate.actor_action = None;
        assert!(candidate.validate().is_err());
        candidate.actor_action = Some(action);
        candidate.status = CandidateStatus::Accepted;
        assert!(candidate.validate().is_err());
        candidate.memory_id = Some(MemoryId::new());
        candidate.reviewed_at_ms = Some(2);
        candidate.validate().unwrap();
        candidate.status = CandidateStatus::Rejected;
        assert!(candidate.validate().is_err());
    }
}
