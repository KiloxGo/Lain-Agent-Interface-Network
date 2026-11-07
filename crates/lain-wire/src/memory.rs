//! Lain-owned semantic memory. These records are evidence, never permissions.
pub use crate::memory_context::MemoryOrigin;
use crate::{
    ActionId, ActorTurnId, BatchId, CandidateId, ChannelId, ConversationId, IncomingId, MemoryId,
    RequestId, SummaryId,
};
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
    Conversation {
        conversation_id: ConversationId,
    },
    Global,
    Workspace {
        path: String,
    },
    Internal,
    Channel {
        channel_id: ChannelId,
    },
    Project {
        project_id: crate::ProjectId,
    },
    Device {
        device_id: crate::DeviceId,
    },
    DeviceWorkspace {
        device_id: crate::DeviceId,
        workspace_id: crate::DeviceWorkspaceId,
    },
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
    /// Current user's exact reviewed correction, not verified source content.
    ConversationCorrection {
        evidence: ConversationCorrectionEvidence,
    },
    BriefingSelection {
        evidence: BriefingMemoryEvidence,
    },
    ArchiveSelection {
        evidence: ArchiveMemoryEvidence,
    },
    /// Exact text supplied through the authenticated owner memory API.
    ExplicitOwnerStatement {
        request_id: RequestId,
    },
    /// Model interpretation, even when accepted by the owner.
    LainGenerated {
        conversation_id: ConversationId,
        actor_turn_id: ActorTurnId,
        action_id: ActionId,
    },
    /// Owner-selected claim supported by a bounded factual summary, not verified truth.
    SessionSummary {
        summary_id: SummaryId,
    },
    ChannelSelection {
        channel_id: ChannelId,
        conversation_id: ConversationId,
        batch_id: BatchId,
        incoming_id: IncomingId,
        actor_turn_id: ActorTurnId,
        action_id: ActionId,
        evidence: String,
        grant_revision: u64,
        grant_expires_at_ms: Option<u64>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BriefingMemoryEvidence {
    pub conversation_id: ConversationId,
    pub briefing_id: RequestId,
    pub item_id: crate::BriefingItemId,
    pub extraction_turn_id: ActorTurnId,
    pub batch_from_index: u16,
    pub actor_turn_id: ActorTurnId,
    pub action_id: ActionId,
    pub grant_revision: u64,
    pub expires_at_ms: u64,
    /// All primary extraction inputs, including uncited records. Reconciled groups
    /// additionally bind complete discovery/judgment input versions through the reference below.
    pub records: Vec<crate::archives::ArchiveRecallVersion>,
    /// Full discovery/judgment input versions stay in the immutable briefing manifest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reconciliation: Option<BriefingMemoryReconciliation>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BriefingMemoryReconciliation {
    pub index_turn_id: ActorTurnId,
    pub fingerprint: String,
    pub member_numbers: Vec<u16>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationCorrectionEvidence {
    pub conversation_id: ConversationId,
    pub previous_memory_id: MemoryId,
    pub previous_revision: u64,
    pub actor_turn_id: ActorTurnId,
    pub action_id: ActionId,
    pub message_id: crate::MessageId,
    pub expires_at_ms: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveMemoryEvidence {
    pub model_request: u32,
    pub channel_id: ChannelId,
    pub conversation_id: ConversationId,
    pub batch_id: BatchId,
    pub actor_turn_id: ActorTurnId,
    pub action_id: ActionId,
    pub grant_revision: u64,
    pub memory_grant_revision: u64,
    pub recall_revision: u64,
    pub expires_at_ms: u64,
    pub records: Vec<crate::archives::ArchiveRecallVersion>,
    pub memories: Vec<crate::chat::ChatMemoryVersion>,
    pub incoming_ids: Vec<IncomingId>,
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
    /// Missing for legacy records; never infer an origin from visibility/path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<MemoryOrigin>,
    pub status: MemoryStatus,
    pub superseded_by: Option<MemoryId>,
    pub recorded_at_ms: u64,
    pub owner_confirmed_at_ms: Option<u64>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<MemoryOrigin>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelMemoryContext {
    pub channel_id: ChannelId,
    pub batch_id: BatchId,
    pub grant_revision: u64,
    pub expires_at_ms: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryGrant {
    pub channel_id: ChannelId,
    pub enabled: bool,
    pub revision: u64,
    pub expires_at_ms: Option<u64>,
    pub changed_at_ms: u64,
    /// Inputs committed before this authorization cannot become autonomous sources.
    pub source_after: u64,
}
impl MemoryGrant {
    pub fn active(&self, now: u64) -> bool {
        self.enabled && self.expires_at_ms.is_none_or(|expiry| now < expiry)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetMemoryGrant {
    pub enabled: bool,
    pub expected_revision: u64,
    pub expires_at_ms: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecallMemory {
    pub text: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryEvidence {
    pub incoming_id: IncomingId,
    pub quote: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectMemory {
    pub content: String,
    pub kind: MemoryKind,
    pub evidence: MemoryEvidence,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectMemory {
    pub memory_id: MemoryId,
    pub expected_revision: u64,
    pub selection: SelectMemory,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForgetMemory {
    pub memory_id: MemoryId,
    pub expected_revision: u64,
    pub evidence: MemoryEvidence,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryOperation {
    Remember,
    Correct,
    Forget,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedMemoryReceipt {
    pub action_id: ActionId,
    pub actor_turn_id: ActorTurnId,
    pub conversation_id: ConversationId,
    pub candidate_id: CandidateId,
    pub memory_id: MemoryId,
    pub recorded_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectMemorySelection {
    pub memory_id: MemoryId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeMemorySelection {
    pub memory_id: MemoryId,
    pub expected_revision: u64,
    pub operation: MemoryOperation,
    pub message_id: crate::MessageId,
    pub quote: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemorySelectionTarget {
    pub memory_id: MemoryId,
    pub conversation_id: ConversationId,
    pub revision: u64,
    pub status: MemoryStatus,
    pub recorded_at_ms: u64,
    pub expires_at_ms: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemorySelectionReview {
    pub actor_turn_id: ActorTurnId,
    pub action_id: ActionId,
    pub request: ChangeMemorySelection,
    pub target: MemorySelectionTarget,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement: Option<MemoryCandidate>,
}
impl MemorySelectionReview {
    pub fn valid_replacement(&self) -> bool {
        match (self.request.operation, &self.replacement) {
            (MemoryOperation::Forget, None) => true,
            (MemoryOperation::Correct, Some(c)) => {
                let MemorySource::ConversationCorrection { evidence: e } = &c.provenance.source
                else {
                    return false;
                };
                c.validate().is_ok()
                    && c.status == CandidateStatus::Candidate
                    && self.target.status == MemoryStatus::Active
                    && c.id.0 == self.action_id.0
                    && c.actor_action == Some(self.action_id)
                    && c.draft.content == self.request.quote
                    && e.actor_turn_id == self.actor_turn_id
                    && e.action_id == self.action_id
                    && e.conversation_id == self.target.conversation_id
                    && e.previous_memory_id == self.target.memory_id
                    && e.previous_revision == self.target.revision
                    && e.message_id == self.request.message_id
                    && e.expires_at_ms == self.target.expires_at_ms
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemorySelectionEffect {
    pub actor_turn_id: ActorTurnId,
    pub action_id: ActionId,
    pub conversation_id: ConversationId,
    pub memory_id: MemoryId,
    pub previous_revision: u64,
    pub operation: MemoryOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement_memory_id: Option<MemoryId>,
    pub recorded_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEffect {
    pub action_id: ActionId,
    pub actor_turn_id: ActorTurnId,
    pub channel_id: ChannelId,
    pub grant_revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_grant_revision: Option<u64>,
    pub operation: MemoryOperation,
    pub memory_id: MemoryId,
    pub previous_memory_id: Option<MemoryId>,
    pub recorded_at_ms: u64,
}

impl MemoryProvenance {
    pub fn validate(&self) -> Result<(), &'static str> {
        if matches!(
            (&self.source, self.authority),
            (
                MemorySource::ExplicitOwnerStatement { .. },
                MemoryAuthority::Explicit
            ) | (
                MemorySource::ConversationCorrection { .. },
                MemoryAuthority::Explicit
            ) | (
                MemorySource::LainGenerated { .. },
                MemoryAuthority::Inferred
            ) | (
                MemorySource::SessionSummary { .. },
                MemoryAuthority::UnverifiedSelection
            ) | (
                MemorySource::BriefingSelection { .. },
                MemoryAuthority::UnverifiedSelection
            ) | (
                MemorySource::ArchiveSelection { .. },
                MemoryAuthority::Inferred
            ) | (
                MemorySource::ChannelSelection { .. },
                MemoryAuthority::Inferred
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
        validate_origin(&self.origin, &self.provenance.source)?;
        if self.revision == 0
            || self.updated_at_ms < self.recorded_at_ms
            || self
                .owner_confirmed_at_ms
                .is_some_and(|t| t < self.recorded_at_ms)
            || (self.status == MemoryStatus::Superseded) != self.superseded_by.is_some()
            || self.superseded_by == Some(self.id)
        {
            return Err("contradictory memory lifecycle");
        }
        if let MemorySource::ConversationCorrection { evidence: e } = &self.provenance.source {
            validate_correction(&self.draft, e, self.recorded_at_ms)?;
            if self.owner_confirmed_at_ms.is_none() || e.previous_memory_id == self.id {
                return Err("correction requires actual review and a distinct predecessor");
            }
        } else if let MemorySource::BriefingSelection { evidence: e } = &self.provenance.source {
            validate_briefing(&self.draft, e, self.recorded_at_ms)?;
            if self.owner_confirmed_at_ms.is_none() {
                return Err("briefing selection requires actual review");
            }
        } else if let MemorySource::ArchiveSelection { evidence: e } = &self.provenance.source {
            if self.owner_confirmed_at_ms.is_some()
                || self.draft.scope
                    != (MemoryScope::Channel {
                        channel_id: e.channel_id,
                    })
                || e.grant_revision == 0
                || e.model_request == 0
                || e.memory_grant_revision == 0
                || e.recall_revision == 0
                || e.expires_at_ms <= self.recorded_at_ms
                || !(1..=16).contains(&e.records.len())
                || e.memories.len() > 16
                || !(1..=16).contains(&e.incoming_ids.len())
                || e.records
                    .iter()
                    .any(|r| r.revision == 0 || r.source_grant_revision == 0)
                || e.memories
                    .iter()
                    .any(|m| m.revision == 0 || m.memory_id == self.id)
            {
                return Err("invalid archive memory authority or lineage");
            }
        } else if let MemorySource::ChannelSelection {
            channel_id,
            evidence,
            grant_revision,
            grant_expires_at_ms,
            ..
        } = &self.provenance.source
        {
            if self.owner_confirmed_at_ms.is_some()
                || self.draft.scope
                    != (MemoryScope::Channel {
                        channel_id: *channel_id,
                    })
                || evidence.trim().is_empty()
                || evidence.len() > 1024
                || *grant_revision == 0
                || grant_expires_at_ms.is_some_and(|t| t <= self.recorded_at_ms)
            {
                return Err("invalid autonomous memory authority or scope");
            }
        } else if self.owner_confirmed_at_ms.is_none() {
            return Err("reviewed memory requires its actual confirmation timestamp");
        }
        Ok(())
    }
}

impl MemoryCandidate {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.provenance.validate()?;
        validate_origin(&self.origin, &self.provenance.source)?;
        let valid_review = match self.status {
            CandidateStatus::Candidate => self.memory_id.is_none() && self.reviewed_at_ms.is_none(),
            CandidateStatus::Rejected => self.memory_id.is_none() && self.reviewed_at_ms.is_some(),
            CandidateStatus::Accepted => self.memory_id.is_some() && self.reviewed_at_ms.is_some(),
        };
        let valid_action = match self.provenance.source {
            MemorySource::ConversationCorrection { ref evidence } => {
                validate_correction(&self.draft, evidence, self.created_at_ms)?;
                self.actor_action == Some(evidence.action_id) && self.id.0 == evidence.action_id.0
            }
            MemorySource::LainGenerated { action_id, .. } => self.actor_action == Some(action_id),
            MemorySource::BriefingSelection { ref evidence } => {
                validate_briefing(&self.draft, evidence, self.created_at_ms)?;
                self.actor_action == Some(evidence.action_id) && self.id.0 == evidence.action_id.0
            }
            MemorySource::ChannelSelection { .. } | MemorySource::ArchiveSelection { .. } => false,
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

fn validate_correction(
    draft: &MemoryDraft,
    e: &ConversationCorrectionEvidence,
    at: u64,
) -> Result<(), &'static str> {
    if draft.scope
        != (MemoryScope::Conversation {
            conversation_id: e.conversation_id,
        })
        || draft.kind != MemoryKind::Episodic
        || e.conversation_id.0.is_nil()
        || e.previous_memory_id.0.is_nil()
        || e.previous_revision == 0
        || e.actor_turn_id.0.is_nil()
        || e.action_id.0.is_nil()
        || e.message_id.0.is_nil()
        || e.expires_at_ms <= at
    {
        return Err("invalid reviewed conversation correction lineage");
    }
    Ok(())
}

fn validate_briefing(
    draft: &MemoryDraft,
    e: &BriefingMemoryEvidence,
    at: u64,
) -> Result<(), &'static str> {
    if let Some(r) = &e.reconciliation
        && (r.index_turn_id.0.is_nil()
            || r.fingerprint.len() != 64
            || !r.fingerprint.bytes().all(|b| b.is_ascii_hexdigit())
            || !(2..=256).contains(&r.member_numbers.len())
            || r.member_numbers
                .iter()
                .enumerate()
                .any(|(i, n)| *n == 0 || *n > 256 || (i > 0 && r.member_numbers[i - 1] >= *n)))
    {
        return Err("invalid reconciled briefing reference");
    }
    if draft.scope
        != (MemoryScope::Conversation {
            conversation_id: e.conversation_id,
        })
        || draft.kind != MemoryKind::Episodic
        || e.conversation_id.0.is_nil()
        || e.briefing_id.0.is_nil()
        || e.item_id.0.is_nil()
        || e.extraction_turn_id.0.is_nil()
        || e.actor_turn_id.0.is_nil()
        || e.action_id.0.is_nil()
        || e.grant_revision == 0
        || e.expires_at_ms <= at
        || e.batch_from_index >= 4096
        || !(1..=4).contains(&e.records.len())
        || e.records.iter().enumerate().any(|(i, v)| {
            v.record_id.0.is_nil()
                || v.source_id.0.is_nil()
                || v.revision == 0
                || v.source_grant_revision == 0
                || e.records[..i].iter().any(|p| p.record_id == v.record_id)
        })
    {
        return Err("invalid reviewed briefing scope or source lineage");
    }
    Ok(())
}

fn validate_origin(
    origin: &Option<MemoryOrigin>,
    source: &MemorySource,
) -> Result<(), &'static str> {
    let valid = match (origin, source) {
        (None, MemorySource::ConversationCorrection { .. }) => false,
        (
            Some(MemoryOrigin::Conversation { conversation_id }),
            MemorySource::ConversationCorrection { evidence },
        ) => *conversation_id == evidence.conversation_id,
        (None, MemorySource::BriefingSelection { .. }) => false,
        (
            Some(MemoryOrigin::Conversation { conversation_id }),
            MemorySource::BriefingSelection { evidence },
        ) => *conversation_id == evidence.conversation_id,
        (None, _) => true,
        (Some(MemoryOrigin::OwnerApi), MemorySource::ExplicitOwnerStatement { .. }) => true,
        (Some(MemoryOrigin::Channel { channel_id }), MemorySource::LainGenerated { .. }) => {
            !channel_id.0.is_nil()
        }
        (
            Some(MemoryOrigin::OwnerSelectedActorContext { selection }),
            MemorySource::LainGenerated { .. },
        ) => selection.as_ref().is_none_or(|s| s.valid()),
        (
            Some(MemoryOrigin::Channel { channel_id }),
            MemorySource::ChannelSelection {
                channel_id: actual, ..
            },
        ) => channel_id == actual,
        (
            Some(MemoryOrigin::Channel { channel_id }),
            MemorySource::ArchiveSelection { evidence },
        ) => *channel_id == evidence.channel_id,
        (
            Some(MemoryOrigin::SessionSummary { summary_id }),
            MemorySource::SessionSummary { summary_id: actual },
        ) => summary_id == actual,
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err("memory origin contradicts source")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_briefing_memory_is_reviewed_conversation_local_and_source_bearing() {
        let conversation = ConversationId::new();
        let action = ActionId::new();
        let scope = serde_json::json!({"kind":"conversation","conversation_id":conversation});
        assert!(serde_json::from_value::<MemoryScope>(scope.clone()).is_ok());
        let value = serde_json::json!({"id":CandidateId(action.0),"draft":{"content":"来源说有讨论，未确认参加。","kind":"episodic","scope":scope},"provenance":{"source":{"kind":"briefing_selection","evidence":{"conversation_id":conversation,"briefing_id":RequestId::new(),"item_id":crate::BriefingItemId::new(),"extraction_turn_id":ActorTurnId::new(),"batch_from_index":0,"actor_turn_id":ActorTurnId::new(),"action_id":action,"grant_revision":1,"expires_at_ms":1000,"records":[{"record_id":crate::ArchiveRecordId::new(),"revision":1,"source_id":crate::ArchiveSourceId::new(),"source_grant_revision":1}]}},"authority":"unverified_selection"},"origin":{"kind":"conversation","conversation_id":conversation},"status":"candidate","memory_id":null,"created_at_ms":1,"reviewed_at_ms":null,"actor_action":action});
        let candidate: MemoryCandidate = serde_json::from_value(value).unwrap();
        candidate.validate().unwrap();
        let mut broadened = candidate.clone();
        broadened.draft.scope = MemoryScope::Global;
        assert!(broadened.validate().is_err());
        let mut fake = candidate.clone();
        fake.actor_action = Some(ActionId::new());
        assert!(fake.validate().is_err());
        let mut fake = candidate;
        fake.provenance.authority = MemoryAuthority::Explicit;
        assert!(fake.validate().is_err());
    }
    #[test]
    fn candidate_review_cannot_disagree_with_origin_or_receipt() {
        let action = ActionId::new();
        let mut candidate = MemoryCandidate {
            id: CandidateId(action.0),
            origin: None,
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
