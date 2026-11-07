//! Bounded owner startup. Invalidation is not permission or effect replay.
use crate::*;
use serde::{Deserialize, Serialize};
pub const PROTOCOL: u32 = 1;
pub const MAX_BYTES: usize = 1024 * 1024;
pub const MAX_LEASE_MS: u64 = 300_000;
pub const CURSOR_WINDOW: u64 = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncCursor {
    pub instance_id: Uuid,
    pub boot_id: Uuid,
    pub revision: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapRequest {
    pub protocol: u32,
    pub conversation_id: Option<ConversationId>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resources {
    pub protocol: u32,
    pub configuration_revision: String,
    pub persona_revision: String,
    pub policy_revision: String,
    pub capabilities: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceHeader {
    pub id: DeviceId,
    pub revision: u64,
    pub label: String,
    pub state: devices::DeviceState,
    pub expires_at_ms: u64,
    pub workspace_ids: Vec<DeviceWorkspaceId>,
    pub report_sequence: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkPointer {
    pub device_id: DeviceId,
    pub command_id: DeviceCommandId,
    pub revision: u64,
    pub state: device_commands::CommandState,
    pub expires_at_ms: u64,
    pub pending_approvals: Vec<WorkerApprovalId>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoticePointer {
    pub id: NoticeId,
    pub cursor: u64,
    pub acknowledged: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapSnapshot {
    pub cursor: SyncCursor,
    pub issued_at_ms: u64,
    pub valid_until_ms: u64,
    pub resources: Resources,
    pub conversation_id: Option<ConversationId>,
    pub memory_context: Option<memory_context::ConversationMemoryContext>,
    pub turns: Vec<ActorSnapshot>,
    pub history_truncated: bool,
    pub devices: Vec<DeviceHeader>,
    pub projects: Vec<projects::Project>,
    pub mappings: Vec<projects::ProjectWorkspace>,
    pub work: Vec<WorkPointer>,
    pub work_truncated: bool,
    pub notices: Vec<NoticePointer>,
    pub notice_cursor: u64,
    pub notices_truncated: bool,
}
impl BootstrapSnapshot {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.cursor.instance_id.is_nil()
            || self.cursor.boot_id.is_nil()
            || self.cursor.revision > i64::MAX as u64
            || self.resources.protocol != PROTOCOL
            || self.valid_until_ms <= self.issued_at_ms
            || self.valid_until_ms - self.issued_at_ms > MAX_LEASE_MS
            || self.turns.len() > 20
            || self.devices.len() > 128
            || self.projects.len() > 128
            || self.mappings.len() > 1024
            || self.work.len() > 32
            || self.notices.len() > 100
            || self.resources.capabilities.len() > 16
            || [
                &self.resources.configuration_revision,
                &self.resources.persona_revision,
                &self.resources.policy_revision,
            ]
            .iter()
            .any(|r| r.len() != 64 || !r.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            return Err("invalid bootstrap envelope");
        }
        if self.conversation_id.is_none()
            && (!self.turns.is_empty()
                || self.memory_context.is_some()
                || !self.work.is_empty()
                || !self.notices.is_empty())
        {
            return Err("conversation data without selection");
        }
        for t in &self.turns {
            t.validate()?;
            if Some(t.conversation_id) != self.conversation_id {
                return Err("bootstrap conversation mismatch");
            }
        }
        if self
            .memory_context
            .as_ref()
            .is_some_and(|c| Some(c.conversation_id) != self.conversation_id)
        {
            return Err("bootstrap context mismatch");
        }
        if serde_json::to_vec(self)
            .map_err(|_| "invalid bootstrap encoding")?
            .len()
            > MAX_BYTES
        {
            return Err("bootstrap exceeds byte bound");
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub cursor: SyncCursor,
    pub server_ms: u64,
    pub valid_until_ms: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvalidationReason {
    Changed,
    Expired,
    Resnapshot,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invalidation {
    pub cursor: SyncCursor,
    pub reason: InvalidationReason,
}
