//! Lain-owned conversation records, independent of provider threads and work.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

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
    };
}
id!(ConversationId);
id!(ActorTurnId);
id!(MessageId);
id!(ActionId);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: ConversationId,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnState {
    Running,
    PendingConfirmation,
    Completed,
    Failed,
    Cancelled,
    Unknown,
}
impl TurnState {
    pub fn active(self) -> bool {
        matches!(self, Self::Running | Self::PendingConfirmation)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub conversation_id: ConversationId,
    pub role: String,
    pub content: String,
    pub timestamp_ms: u64,
    pub status: TurnState,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: ActionId,
    pub call_id: String,
    pub tool: super::tools::Tool,
    pub state: ActionState,
    pub result: Option<Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorTurn {
    pub id: ActorTurnId,
    pub conversation_id: ConversationId,
    pub request_id: Uuid,
    pub sequence: u64,
    pub revision: u64,
    pub state: TurnState,
    pub messages: Vec<Message>,
    pub actions: Vec<Action>,
    pub error: Option<String>,
    pub model_requests: u32,
    pub usage_tokens: u64,
}
impl ActorTurn {
    pub fn message(&mut self, role: &str, content: String) -> anyhow::Result<()> {
        self.messages.push(Message {
            id: MessageId::new(),
            conversation_id: self.conversation_id,
            role: role.into(),
            content,
            timestamp_ms: crate::domain::now_ms()?,
            status: self.state,
        });
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct ConversationPage {
    pub conversation: Conversation,
    pub turns: Vec<ActorTurn>,
    pub next_cursor: u64,
    pub has_more: bool,
}
