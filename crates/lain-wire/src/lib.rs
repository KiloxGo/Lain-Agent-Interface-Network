//! Small public Actor observation DTOs. No provider objects or Kernel services.
use serde::{Deserialize, Serialize};
use uuid::Uuid;
pub mod notices;

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
id!(ActorTurnId);
id!(MessageId);
id!(ActionId);
id!(PreviewId);
id!(RequestId);
id!(WorkSessionId);
id!(NoticeId);
id!(WorkerTurnId);

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
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sandbox {
    ReadOnly,
    WorkspaceWrite,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "name", content = "arguments", deny_unknown_fields)]
pub enum ToolCall {
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
            Self::WorkCreate { .. }
                | Self::WorkSend { .. }
                | Self::WorkCancel { .. }
                | Self::WorkResume { .. }
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
    pub id: ActionId,
    pub call_id: String,
    pub tool: ToolCall,
    pub state: ActionState,
    pub result: Option<serde_json::Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorSnapshot {
    pub id: ActorTurnId,
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
}
impl ActorSnapshot {
    pub fn validate(&self) -> Result<(), &'static str> {
        use std::collections::BTreeSet;
        if self.messages.len() > 32 || self.actions.len() > 4 || self.model_requests > 4 {
            return Err("Actor bounds exceeded");
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
        };
        assert!(turn.validate().is_ok());
        turn.state = ActorTurnState::PendingConfirmation;
        assert!(turn.validate().is_err());
        turn.actions.push(ActorAction {
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
