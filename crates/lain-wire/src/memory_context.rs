//! Explicit owner-selected context. This is not device attestation or a Worker grant.
use crate::{ConversationId, DeviceId, DeviceWorkspaceId, ProjectId, RequestId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryProjectContext {
    pub project_id: ProjectId,
    pub project_revision: u64,
    pub mapping_revision: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryContextSelection {
    pub device_id: DeviceId,
    pub workspace_id: DeviceWorkspaceId,
    pub device_revision: u64,
    pub project: Option<MemoryProjectContext>,
}
impl MemoryContextSelection {
    pub fn valid(&self) -> bool {
        !self.device_id.0.is_nil()
            && !self.workspace_id.0.is_nil()
            && self.device_revision > 0
            && self.project.as_ref().is_none_or(|p| {
                !p.project_id.0.is_nil() && p.project_revision > 0 && p.mapping_revision > 0
            })
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetMemoryContext {
    pub request_id: RequestId,
    pub expected_revision: u64,
    pub selection: Option<MemoryContextSelection>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationMemoryContext {
    pub conversation_id: ConversationId,
    pub revision: u64,
    pub request_id: RequestId,
    pub selection: Option<MemoryContextSelection>,
    pub changed_at_ms: u64,
}

/// Collection origin is independent of the record's visibility scope. Selection
/// by an owner does not prove that a physical PC produced the information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MemoryOrigin {
    Conversation {
        conversation_id: ConversationId,
    },
    OwnerApi,
    OwnerSelectedActorContext {
        selection: Option<MemoryContextSelection>,
    },
    Channel {
        channel_id: crate::ChannelId,
    },
    SessionSummary {
        summary_id: crate::SummaryId,
    },
}
