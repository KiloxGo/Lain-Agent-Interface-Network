//! Small public Actor observation DTOs. No provider objects or Kernel services.
use serde::{Deserialize, Serialize};
use uuid::Uuid;
pub mod archives;
pub mod briefing;
pub mod chat;
pub mod client_sync;
pub mod device_commands;
pub mod device_execution;
pub mod device_work;
pub mod devices;
pub mod inference;
pub mod media;
pub mod memory;
pub mod memory_context;
pub mod notices;
pub mod projects;
pub mod reading;
pub mod sharing;
pub mod source;

/// Startup revision of validated non-secret configuration/persona content.
/// Never use this to compare or hash credential values.
pub fn actor_configuration_revision(non_secret_content: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(non_secret_content))
}

macro_rules! id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);
        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
        impl std::str::FromStr for $name {
            type Err = uuid::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                s.parse().map(Self)
            }
        }
    };
}
id!(ConversationId);
id!(BriefingItemId);
id!(DeviceId);
id!(DeviceWorkspaceId);
id!(ProjectId);
id!(DeviceCommandId);
id!(DeviceExecutionId);
id!(WorkerApprovalId);
id!(ActorTurnId);
id!(MessageId);
id!(ActionId);
id!(PreviewId);
id!(RequestId);
id!(WorkSessionId);
id!(NoticeId);
id!(WorkerTurnId);
id!(MemoryId);
id!(CandidateId);
id!(SummaryId);
id!(ChannelId);
id!(IncomingId);
id!(MediaId);
id!(BatchId);
id!(DeliveryId);
id!(ReadingId);
id!(ExperienceId);
id!(ShareId);
id!(InferenceBudgetId);
id!(ArchiveSourceId);
id!(ArchiveRecordId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorTurnState {
    Running,
    PendingConfirmation,
    Completed,
    Failed,
    Cancelled,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    Tool,
}
/// Persisted bodies are complete. Inference/action uncertainty belongs to the
/// enclosing turn/action; incremental text exists only in preview events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageStatus {
    Completed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionState {
    Proposed,
    Executing,
    Executed,
    Rejected,
    Failed,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sandbox {
    ReadOnly,
    WorkspaceWrite,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "name", content = "arguments", deny_unknown_fields)]
pub enum ToolCall {
    #[serde(rename = "memory.selection.inspect")]
    MemorySelectionInspect(memory::InspectMemorySelection),
    #[serde(rename = "memory.selection.change")]
    MemorySelectionChange(memory::ChangeMemorySelection),
    #[serde(rename = "briefing.remember")]
    BriefingRemember(briefing::RememberBriefing),
    #[serde(rename = "briefing.yesterday")]
    BriefingYesterday {},
    #[serde(rename = "briefing.range")]
    BriefingRange(briefing::BriefingRange),
    #[serde(rename = "briefing.read")]
    BriefingRead(briefing::BriefingRead),
    #[serde(rename = "briefing.inspect")]
    BriefingInspect(briefing::BriefingInspect),
    #[serde(rename = "device.list")]
    DeviceList {},
    #[serde(rename = "device.work.list")]
    DeviceWorkList {},
    #[serde(rename = "device.work.inspect")]
    DeviceWorkInspect(device_work::InspectDeviceWork),
    #[serde(rename = "device.queue")]
    DeviceQueue(device_work::ProposeDeviceWork),
    #[serde(rename = "device.send")]
    DeviceSend(device_work::ProposeDeviceFollowup),
    #[serde(rename = "archive.remember")]
    ArchiveRemember(archives::RememberArchive),
    #[serde(rename = "archive.search")]
    ArchiveSearch(archives::ArchiveRecallQuery),
    #[serde(rename = "archive.inspect")]
    ArchiveInspect { record_id: ArchiveRecordId },
    #[serde(rename = "sharing.search")]
    SharingSearch(sharing::ShareQuery),
    #[serde(rename = "sharing.inspect")]
    SharingInspect { delivery_id: DeliveryId },
    #[serde(rename = "source.search")]
    SourceSearch(source::SourceQuery),
    #[serde(rename = "chat.wait")]
    ChatWait(chat::WaitChat),
    #[serde(rename = "chat.finish")]
    ChatFinish(chat::FinishChat),
    #[serde(rename = "source.inspect")]
    SourceInspect(source::InspectSource),
    #[serde(rename = "experience.search")]
    ExperienceSearch(reading::ExperienceQuery),
    #[serde(rename = "experience.inspect")]
    ExperienceInspect { experience_id: ExperienceId },
    #[serde(rename = "memory.search")]
    MemorySearch(memory::MemoryQuery),
    #[serde(rename = "memory.list")]
    MemoryList(memory::MemoryQuery),
    #[serde(rename = "memory.inspect")]
    MemoryInspect {
        memory_id: MemoryId,
        scope: memory::MemoryScope,
    },
    #[serde(rename = "memory.propose")]
    MemoryPropose(memory::ProposeMemory),
    #[serde(rename = "memory.recall")]
    MemoryRecall(memory::RecallMemory),
    #[serde(rename = "memory.remember")]
    MemoryRemember(memory::SelectMemory),
    #[serde(rename = "memory.correct")]
    MemoryCorrect(memory::CorrectMemory),
    #[serde(rename = "memory.forget")]
    MemoryForget(memory::ForgetMemory),
    #[serde(rename = "work.list")]
    WorkList {},
    #[serde(rename = "work.inspect")]
    WorkInspect { session_id: WorkSessionId },
    #[serde(rename = "work.history")]
    WorkHistory {
        session_id: WorkSessionId,
        after: u64,
    },
    #[serde(rename = "profile.list")]
    ProfileList {},
    #[serde(rename = "profile.inspect")]
    ProfileInspect { profile_id: String },
    #[serde(rename = "work.create")]
    WorkCreate {
        workspace: String,
        profile_id: String,
        sandbox: Sandbox,
        task: String,
    },
    #[serde(rename = "work.send")]
    WorkSend {
        session_id: WorkSessionId,
        text: String,
    },
    #[serde(rename = "work.cancel")]
    WorkCancel { session_id: WorkSessionId },
    #[serde(rename = "work.resume")]
    WorkResume { session_id: WorkSessionId },
}
impl ToolCall {
    pub fn mutating(&self) -> bool {
        matches!(
            self,
            Self::MemorySelectionChange(_)
                | Self::BriefingRemember(_)
                | Self::WorkCreate { .. }
                | Self::DeviceQueue(_)
                | Self::DeviceSend(_)
                | Self::WorkSend { .. }
                | Self::WorkCancel { .. }
                | Self::WorkResume { .. }
                | Self::MemoryPropose(_)
                | Self::MemoryRemember(_)
                | Self::MemoryCorrect(_)
                | Self::MemoryForget(_)
        )
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorMessage {
    pub id: MessageId,
    pub conversation_id: ConversationId,
    pub role: MessageRole,
    pub content: String,
    pub timestamp_ms: u64,
    pub status: MessageStatus,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorAction {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_change_review: Option<memory::MemorySelectionReview>,
    /// Owner-only, source-checked preview; not an execution result or persisted action payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_review: Option<memory::MemoryCandidate>,
    pub id: ActionId,
    pub call_id: String,
    pub tool: ToolCall,
    pub state: ActionState,
    pub result: Option<serde_json::Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorSnapshot {
    /// A restricted network projection, never a replacement for durable audit state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub availability: Option<ActorAvailability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effect_receipts: Vec<memory::MemoryEffect>,
    /// Committed reviewed selections; historical metadata, never source text or authority.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reviewed_memory_receipts: Vec<memory::ReviewedMemoryReceipt>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub memory_change_receipts: Vec<memory::MemorySelectionEffect>,
    pub id: ActorTurnId,
    /// Kernel-initiated reading work; None for user-originated turns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reading_activity: Option<ReadingId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub briefing_activity: Option<briefing::BriefingActivity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sharing_activity: Option<ShareId>,
    pub conversation_id: ConversationId,
    pub request_id: RequestId,
    pub sequence: u64,
    pub revision: u64,
    pub state: ActorTurnState,
    pub messages: Vec<ActorMessage>,
    pub actions: Vec<ActorAction>,
    pub error: Option<String>,
    pub model_requests: u32,
    pub usage_tokens: u64,
    #[serde(default)]
    pub timing: ActorTiming,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_memory: Option<memory::ChannelMemoryContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_source: Option<source::SourceReadContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub private_chat: Option<chat::PrivateChatContext>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorAvailability {
    SourceUnavailable,
}
impl ActorSnapshot {
    pub fn validate(&self) -> Result<(), &'static str> {
        use std::collections::BTreeSet;
        if self.messages.len() > 32 || self.actions.len() > 4 || self.model_requests > 4 {
            return Err("Actor bounds exceeded");
        }
        if self.effect_receipts.len() > 4
            || self
                .effect_receipts
                .iter()
                .any(|e| e.actor_turn_id != self.id || e.grant_revision == 0)
        {
            return Err("invalid effect metadata receipt");
        }
        let mut receipts = BTreeSet::new();
        if self.reviewed_memory_receipts.len() > 4
            || self.reviewed_memory_receipts.iter().any(|r| {
                r.actor_turn_id != self.id
                    || r.conversation_id != self.conversation_id
                    || r.action_id.0.is_nil()
                    || r.memory_id.0.is_nil()
                    || r.candidate_id.0 != r.action_id.0
                    || r.recorded_at_ms == 0
                    || !receipts.insert(r.action_id)
            })
        {
            return Err("invalid reviewed memory receipt");
        }
        let mut changes = BTreeSet::new();
        if self.memory_change_receipts.len() > 4
            || self.memory_change_receipts.iter().any(|r| {
                r.actor_turn_id != self.id
                    || r.conversation_id != self.conversation_id
                    || r.action_id.0.is_nil()
                    || r.memory_id.0.is_nil()
                    || r.previous_revision == 0
                    || !matches!(
                        r.operation,
                        memory::MemoryOperation::Forget | memory::MemoryOperation::Correct
                    )
                    || (r.replacement_memory_id.is_some()
                        != (r.operation == memory::MemoryOperation::Correct))
                    || r.replacement_memory_id
                        .is_some_and(|id| id.0.is_nil() || id == r.memory_id)
                    || r.recorded_at_ms == 0
                    || !changes.insert(r.action_id)
            })
        {
            return Err("invalid memory change receipt");
        }
        if self.availability.is_some() {
            if !self.messages.is_empty()
                || !self.actions.is_empty()
                || self.private_chat.is_some()
                || self.channel_memory.is_some()
                || self.channel_source.is_some()
                || self.error.as_deref() != Some("Source unavailable; previous content withheld.")
            {
                return Err("invalid restricted Actor projection");
            }
            return Ok(());
        }
        if (self.reading_activity.is_some()
            || self.sharing_activity.is_some()
            || self.briefing_activity.is_some())
            && (self.private_chat.is_some()
                || self.channel_memory.is_some()
                || self.channel_source.is_some()
                || !self.actions.is_empty())
        {
            return Err("reading activity cannot acquire chat, memory or Worker actions");
        }
        if usize::from(self.reading_activity.is_some())
            + usize::from(self.sharing_activity.is_some())
            + usize::from(self.briefing_activity.is_some())
            > 1
        {
            return Err("conflicting activity origin");
        }
        if self.channel_memory.as_ref().is_some_and(|c| {
            c.batch_id.0 != self.request_id.0 || c.grant_revision == 0 || c.expires_at_ms == 0
        }) {
            return Err("contradictory private memory context");
        }
        if self.channel_source.as_ref().is_some_and(|c| {
            c.batch_id.0 != self.request_id.0 || c.grant_revision == 0 || c.expires_at_ms == 0
        }) {
            return Err("contradictory private source context");
        }
        if self.private_chat.as_ref().is_some_and(|c| {
            c.batch_id.0 != self.request_id.0
                || c.grant_revision == 0
                || c.expires_at_ms == 0
                || c.input_through_sequence == 0
                || c.reconsiderations > chat::MAX_RECONSIDERATIONS
                || c.waits > 1
                || c.memory_versions.len() > 3
                || c.archive_memory_versions.len() > 12
                || c.archive_memory_versions.iter().any(|m| m.revision == 0)
                || c.archive_memory_grant_revision == Some(0)
                || c.memory_versions.iter().any(|m| m.revision == 0)
                || c.source_versions.len() > 4
                || c.experience_versions.len() > 3
                || c.share_versions.len() > 3
                || c.media_versions.len() > 256
                || c.media_versions.iter().any(|v| v.revision == 0)
                || c.viewed_media.len() > 256
                || c.viewed_media.iter().any(|v| !c.media_versions.contains(v))
                || c.share_versions.iter().any(|s| {
                    s.fingerprint.len() != 64
                        || !s.fingerprint.bytes().all(|b| b.is_ascii_hexdigit())
                        || s.experience.as_ref().is_some_and(|e| {
                            e.revision == 0
                                || e.grant_revision == 0
                                || e.fingerprint.len() != 64
                                || !e.fingerprint.bytes().all(|b| b.is_ascii_hexdigit())
                        })
                })
                || c.experience_grant_revision == Some(0)
                || (!c.experience_versions.is_empty() && c.experience_grant_revision.is_none())
                || c.experience_versions.iter().any(|v| {
                    v.revision == 0
                        || v.grant_revision == 0
                        || v.fingerprint.len() != 64
                        || !v.fingerprint.bytes().all(|b| b.is_ascii_hexdigit())
                })
                || c.source_versions.iter().any(|v| {
                    v.fingerprint.len() != 64
                        || !v.fingerprint.bytes().all(|b| b.is_ascii_hexdigit())
                        || v.selection.channel_id != c.channel_id
                        || v.selection.grant_revision == 0
                        || !v.selection.window.valid()
                        || v.selection.incoming_ids.len() > 3
                })
                || (!c.source_versions.is_empty()
                    && self
                        .channel_source
                        .as_ref()
                        .is_none_or(|s| s.channel_id != c.channel_id || s.batch_id != c.batch_id))
                || ((!c.memory_versions.is_empty()
                    || !c.archive_memory_versions.is_empty()
                    || c.archive_memory_grant_revision.is_some())
                    && self
                        .channel_memory
                        .as_ref()
                        .is_none_or(|m| m.channel_id != c.channel_id || m.batch_id != c.batch_id))
                || (c.decision == chat::ChatDecision::Waiting) != c.wait_until_ms.is_some()
                || (c.decision == chat::ChatDecision::Waiting
                    && self.state != ActorTurnState::Running)
                || (matches!(
                    c.decision,
                    chat::ChatDecision::Reply
                        | chat::ChatDecision::NoReply
                        | chat::ChatDecision::Superseded
                ) && self.state != ActorTurnState::Completed)
        }) {
            return Err("contradictory private chat context");
        }
        let mut ids = BTreeSet::new();
        for m in &self.messages {
            if m.conversation_id != self.conversation_id || !ids.insert(m.id) {
                return Err("contradictory message identity");
            }
        }
        let mut ids = BTreeSet::new();
        let mut calls = BTreeSet::new();
        let mut proposed = 0;
        for a in &self.actions {
            if !ids.insert(a.id) || a.call_id.is_empty() || !calls.insert(&a.call_id) {
                return Err("duplicate action identity");
            }
            if a.state == ActionState::Executed && a.result.is_none() {
                return Err("executed action lacks receipt");
            }
            if a.state == ActionState::Proposed {
                proposed += 1;
                if !a.tool.mutating() || a.result.is_some() {
                    return Err("invalid proposal");
                }
            }
            if let Some(c) = &a.memory_review {
                c.validate()?;
                let ToolCall::BriefingRemember(q) = &a.tool else {
                    return Err("memory preview on another tool");
                };
                let memory::MemorySource::BriefingSelection { evidence: e } = &c.provenance.source
                else {
                    return Err("wrong memory preview source");
                };
                if a.state != ActionState::Proposed
                    || c.status != memory::CandidateStatus::Candidate
                    || c.actor_action != Some(a.id)
                    || e.actor_turn_id != self.id
                    || e.conversation_id != self.conversation_id
                    || e.briefing_id != q.briefing_id
                    || e.reconciliation.as_ref().is_some_and(|r| {
                        !q.include_related || !r.member_numbers.contains(&q.item_number)
                    })
                {
                    return Err("memory preview identity mismatch");
                }
            }
            if let Some(r) = &a.memory_change_review {
                let ToolCall::MemorySelectionChange(q) = &a.tool else {
                    return Err("change preview on another tool");
                };
                if a.state != ActionState::Proposed
                    || r.actor_turn_id != self.id
                    || r.action_id != a.id
                    || &r.request != q
                    || r.target.conversation_id != self.conversation_id
                    || r.target.memory_id != q.memory_id
                    || r.target.revision != q.expected_revision
                    || !r.valid_replacement()
                    || q.message_id.0.is_nil()
                    || q.quote.trim().is_empty()
                    || q.quote.len() > 2048
                {
                    return Err("memory change preview identity mismatch");
                }
            }
            if a.state == ActionState::Executing && self.state != ActorTurnState::Running {
                return Err("executing action in inactive turn");
            }
        }
        if (self.state == ActorTurnState::PendingConfirmation && proposed != 1)
            || (self.state != ActorTurnState::PendingConfirmation && proposed != 0)
        {
            return Err("proposal/turn state mismatch");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewEvent {
    pub turn_id: ActorTurnId,
    pub preview_id: PreviewId,
    pub message_id: MessageId,
    pub sequence: u64,
    #[serde(flatten)]
    pub update: PreviewUpdate,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PreviewUpdate {
    Started { text: String },
    Delta { text: String },
    Finished,
    Discarded { reason: DiscardReason },
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscardReason {
    Interrupted,
    Sensitive,
    Inconsistent,
    Limit,
}

/// Durations relative to the start of one provider request, measured monotonically.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelTiming {
    pub first_byte_ms: Option<u64>,
    pub first_text_ms: Option<u64>,
    pub first_visible_ms: Option<u64>,
    pub completed_ms: Option<u64>,
    pub commit_ms: Option<u64>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActorTiming {
    pub queue_ms: Option<u64>,
    pub requests: Vec<ModelTiming>,
    pub total_ms: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contradictory_durable_actor_records_are_rejected() {
        let mut turn = ActorSnapshot {
            availability: None,
            effect_receipts: vec![],
            reviewed_memory_receipts: vec![],
            memory_change_receipts: vec![],
            reading_activity: None,
            briefing_activity: None,
            sharing_activity: None,
            id: ActorTurnId::new(),
            conversation_id: ConversationId::new(),
            request_id: RequestId::new(),
            sequence: 1,
            revision: 1,
            state: ActorTurnState::Completed,
            messages: vec![],
            actions: vec![],
            error: None,
            model_requests: 1,
            usage_tokens: 0,
            timing: ActorTiming::default(),
            channel_memory: None,
            channel_source: None,
            private_chat: None,
        };
        assert!(turn.validate().is_ok());
        let channel = ChannelId::new();
        turn.private_chat = Some(chat::PrivateChatContext {
            media_versions: vec![],
            viewed_media: vec![],
            channel_id: channel,
            batch_id: BatchId(turn.request_id.0),
            grant_revision: 1,
            expires_at_ms: 1,
            input_through_sequence: 1,
            reconsiderations: 0,
            waits: 0,
            decision: chat::ChatDecision::Reply,
            wait_until_ms: None,
            memory_versions: vec![],
            archive_memory_versions: vec![],
            archive_memory_tainted: false,
            archive_memory_grant_revision: None,
            source_versions: vec![],
            experience_versions: vec![],
            experience_grant_revision: None,
            share_versions: vec![],
            archive_recall_revision: None,
            archive_versions: vec![],
        });
        assert!(turn.validate().is_ok());
        let legacy = serde_json::to_value(&turn.private_chat).unwrap();
        assert!(legacy.get("archive_memory_tainted").is_none());
        assert!(legacy.get("archive_memory_grant_revision").is_none());
        let round_trip: chat::PrivateChatContext = serde_json::from_value(legacy).unwrap();
        assert!(!round_trip.archive_memory_tainted);
        turn.private_chat
            .as_mut()
            .unwrap()
            .archive_memory_grant_revision = Some(0);
        assert!(turn.validate().is_err());
        turn.private_chat
            .as_mut()
            .unwrap()
            .archive_memory_grant_revision = Some(1);
        assert!(
            turn.validate().is_err(),
            "write scope requires actual private memory context"
        );
        turn.private_chat
            .as_mut()
            .unwrap()
            .archive_memory_grant_revision = None;
        turn.private_chat.as_mut().unwrap().waits = 2;
        assert!(turn.validate().is_err());
        turn.private_chat.as_mut().unwrap().waits = 0;
        turn.private_chat
            .as_mut()
            .unwrap()
            .memory_versions
            .push(chat::ChatMemoryVersion {
                memory_id: MemoryId::new(),
                revision: 1,
            });
        assert!(
            turn.validate().is_err(),
            "selected memory requires its actual grant scope"
        );
        turn.channel_memory = Some(memory::ChannelMemoryContext {
            channel_id: channel,
            batch_id: BatchId(turn.request_id.0),
            grant_revision: 1,
            expires_at_ms: 1,
        });
        assert!(turn.validate().is_ok());
        turn.channel_memory.as_mut().unwrap().channel_id = ChannelId::new();
        assert!(
            turn.validate().is_err(),
            "memory scope must not cross a private binding"
        );
        turn.channel_memory = None;
        turn.private_chat.as_mut().unwrap().memory_versions.clear();
        turn.private_chat
            .as_mut()
            .unwrap()
            .source_versions
            .push(chat::ChatSourceVersion {
                selection: source::SourceSelection {
                    channel_id: channel,
                    grant_revision: 1,
                    window: source::SourceWindow {
                        from_received_at_ms: 1,
                        to_received_at_ms: 2,
                    },
                    through_sequence: 1,
                    incoming_ids: vec![],
                },
                fingerprint: "a".repeat(64),
            });
        assert!(
            turn.validate().is_err(),
            "selected sources require the actual read scope"
        );
        turn.channel_source = Some(source::SourceReadContext {
            channel_id: channel,
            batch_id: BatchId(turn.request_id.0),
            grant_revision: 1,
            expires_at_ms: 1,
        });
        assert!(turn.validate().is_ok());
        turn.private_chat.as_mut().unwrap().source_versions[0]
            .selection
            .channel_id = ChannelId::new();
        assert!(
            turn.validate().is_err(),
            "selected sources must not cross a private binding"
        );
        turn.channel_source = None;
        turn.private_chat = None;
        turn.state = ActorTurnState::PendingConfirmation;
        assert!(turn.validate().is_err());
        turn.actions.push(ActorAction {
            memory_review: None,
            memory_change_review: None,
            id: ActionId::new(),
            call_id: "call_1".into(),
            tool: ToolCall::WorkCancel {
                session_id: WorkSessionId::new(),
            },
            state: ActionState::Proposed,
            result: None,
        });
        assert!(turn.validate().is_ok());
        turn.state = ActorTurnState::Completed;
        assert!(turn.validate().is_err());
        turn.actions[0].state = ActionState::Executed;
        assert!(turn.validate().is_err());
        turn.actions[0].result = Some(serde_json::json!({"accepted":true}));
        assert!(turn.validate().is_ok());
        turn.actions.push(turn.actions[0].clone());
        turn.actions[1].id = ActionId::new(); // Still duplicates the provider call identity.
        assert!(turn.validate().is_err());
        turn.actions.pop();
        turn.messages.push(ActorMessage {
            id: MessageId::new(),
            conversation_id: turn.conversation_id,
            role: MessageRole::Assistant,
            content: "complete body".into(),
            timestamp_ms: 0,
            status: MessageStatus::Completed,
        });
        for status in [
            "running",
            "pending_confirmation",
            "unknown",
            "failed",
            "cancelled",
        ] {
            let mut invalid = serde_json::to_value(&turn).unwrap();
            invalid["messages"][0]["status"] = status.into();
            assert!(serde_json::from_value::<ActorSnapshot>(invalid).is_err());
        }
        assert!(turn.validate().is_ok());
        turn.messages.push(turn.messages[0].clone());
        assert!(turn.validate().is_err());
    }

    #[test]
    fn preview_variants_round_trip_with_stable_identity() {
        for update in [
            PreviewUpdate::Started {
                text: "你好\n".into(),
            },
            PreviewUpdate::Delta {
                text: "世界".into(),
            },
            PreviewUpdate::Finished,
            PreviewUpdate::Discarded {
                reason: DiscardReason::Interrupted,
            },
        ] {
            let event = PreviewEvent {
                turn_id: ActorTurnId::new(),
                preview_id: PreviewId::new(),
                message_id: MessageId::new(),
                sequence: 3,
                update,
            };
            let value = serde_json::to_value(&event).unwrap();
            let decoded: PreviewEvent = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(serde_json::to_value(decoded).unwrap(), value);
        }
    }
}
