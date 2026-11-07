//! Durable control-plane intent and device-reported facts, never a Worker runtime.
use crate::{DeviceCommandId, DeviceId, DeviceWorkspaceId, RequestId, Sandbox};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueueDeviceWork {
    pub request_id: RequestId,
    pub expected_device_revision: u64,
    pub workspace_id: DeviceWorkspaceId,
    /// Device-local Codex profile name, not a provider configuration or credential.
    pub profile_id: String,
    pub sandbox: Sandbox,
    pub task: String,
    pub expires_at_ms: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandState {
    Queued,
    Accepted,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Rejected,
    Unknown,
    Expired,
    Revoked,
}
impl CommandState {
    pub fn terminal(self) -> bool {
        !matches!(self, Self::Queued | Self::Accepted | Self::Running)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandEventKind {
    Accepted,
    Started,
    Progress,
    Succeeded,
    Failed,
    Cancelled,
    Rejected,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportCommandEvent {
    pub request_id: RequestId,
    pub sequence: u64,
    pub kind: CommandEventKind,
    /// Untrusted device observation; no raw credentials or terminal protocol objects.
    pub detail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution: Option<crate::device_execution::ExecutionEvidence>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandEventReceipt {
    pub event: ReportCommandEvent,
    pub received_at_ms: u64,
    pub command_revision: u64,
    pub state: CommandState,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandInterruption {
    Deadline,
    DeviceRevoked,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceCommand {
    pub id: DeviceCommandId,
    pub device_id: DeviceId,
    pub input: QueueDeviceWork,
    pub created_at_ms: u64,
    pub revision: u64,
    pub state: CommandState,
    pub cancel_requested: bool,
    pub interruption: Option<CommandInterruption>,
    pub last_event: Option<CommandEventReceipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution: Option<crate::device_execution::ExecutionEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation: Option<crate::device_work::DeviceContinuation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continued_by: Option<DeviceCommandId>,
}

/// Bounded dashboard projection; does not copy task text, output or approvals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCommandSummary {
    pub id: DeviceCommandId,
    pub revision: u64,
    pub state: CommandState,
    pub cancel_requested: bool,
    pub profile_id: String,
    pub workspace_id: DeviceWorkspaceId,
    pub created_at_ms: u64,
    pub expires_at_ms: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancelDeviceCommand {
    pub expected_revision: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandChange {
    pub sequence: u64,
    pub command_id: DeviceCommandId,
    pub revision: u64,
    pub state: CommandState,
    pub cancel_requested: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandChanges {
    pub changes: Vec<CommandChange>,
    pub next_after: u64,
    pub through: u64,
    pub has_more: bool,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CommandChangesQuery {
    pub after: u64,
    pub through: Option<u64>,
    pub limit: u16,
    pub wait_ms: u32,
}
impl Default for CommandChangesQuery {
    fn default() -> Self {
        Self {
            after: 0,
            through: None,
            limit: 32,
            wait_ms: 0,
        }
    }
}
