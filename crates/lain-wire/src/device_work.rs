//! Conversation association for remote work. Never a cloud-local Worker session.
use crate::{
    ActionId, ActorTurnId, ConversationId, DeviceCommandId, DeviceId, DeviceWorkspaceId, RequestId,
    Sandbox,
    device_commands::{DeviceCommand, QueueDeviceWork},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FollowupDeviceWork {
    pub request_id: RequestId,
    pub expected_device_revision: u64,
    pub expected_parent_revision: u64,
    pub text: String,
    pub expires_at_ms: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposeDeviceFollowup {
    pub device_id: DeviceId,
    pub parent_command_id: DeviceCommandId,
    pub expected_device_revision: u64,
    pub expected_parent_revision: u64,
    pub text: String,
    pub expires_at_ms: u64,
}
impl ProposeDeviceFollowup {
    pub fn request(&self, request_id: RequestId) -> FollowupDeviceWork {
        FollowupDeviceWork {
            request_id,
            expected_device_revision: self.expected_device_revision,
            expected_parent_revision: self.expected_parent_revision,
            text: self.text.clone(),
            expires_at_ms: self.expires_at_ms,
        }
    }
}
/// Immutable declared continuation, not evidence that a new turn was submitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceContinuation {
    pub root_command_id: DeviceCommandId,
    pub parent_command_id: DeviceCommandId,
    pub expected_parent_revision: u64,
    pub session_id: crate::WorkSessionId,
    pub parent_turn_id: crate::WorkerTurnId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposeDeviceWork {
    pub device_id: DeviceId,
    pub expected_device_revision: u64,
    pub workspace_id: DeviceWorkspaceId,
    pub profile_id: String,
    pub sandbox: Sandbox,
    pub task: String,
    pub expires_at_ms: u64,
}
impl ProposeDeviceWork {
    pub fn queue(&self, request_id: RequestId) -> QueueDeviceWork {
        QueueDeviceWork {
            request_id,
            expected_device_revision: self.expected_device_revision,
            workspace_id: self.workspace_id,
            profile_id: self.profile_id.clone(),
            sandbox: self.sandbox,
            task: self.task.clone(),
            expires_at_ms: self.expires_at_ms,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceWorkOrigin {
    pub conversation_id: ConversationId,
    pub actor_turn_id: Option<ActorTurnId>,
    pub action_id: Option<ActionId>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceWorkRecord {
    pub origin: DeviceWorkOrigin,
    pub command: DeviceCommand,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceWorkPage {
    pub work: Vec<DeviceWorkRecord>,
    pub truncated: bool,
}
impl DeviceWorkPage {
    pub fn validate(&self, conversation: ConversationId) -> Result<(), &'static str> {
        let mut ids = std::collections::BTreeSet::new();
        if self.work.len() > 32
            || self.work.iter().any(|r| {
                r.origin.conversation_id != conversation
                    || r.origin.actor_turn_id.is_some() != r.origin.action_id.is_some()
                    || r.command.id.0.is_nil()
                    || r.command.device_id.0.is_nil()
                    || r.command.revision == 0
                    || r.command.input.task.len() > 8192
                    || !ids.insert((r.command.device_id, r.command.id))
                    || r.command.execution.as_ref().is_some_and(|e| !e.valid())
                    || r.command
                        .continued_by
                        .is_some_and(|id| id == r.command.id || id.0.is_nil())
                    || r.command.continuation.as_ref().is_some_and(|p| {
                        p.parent_command_id == r.command.id
                            || p.root_command_id == r.command.id
                            || p.parent_command_id.0.is_nil()
                            || p.root_command_id.0.is_nil()
                            || p.session_id.0.is_nil()
                            || p.parent_turn_id.0.is_nil()
                            || p.expected_parent_revision == 0
                            || r.command
                                .execution
                                .as_ref()
                                .is_some_and(|e| e.session_id != Some(p.session_id))
                    })
            })
        {
            return Err("invalid conversation-bound device work snapshot");
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectDeviceWork {
    pub device_id: DeviceId,
    pub command_id: DeviceCommandId,
}
