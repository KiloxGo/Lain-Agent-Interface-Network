//! Lain-owned conversation records, independent of provider threads and work.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

pub use lain_wire::{ActionId, ActorTurnId, ConversationId, MessageId, MessageRole, MessageStatus};
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
    pub role: MessageRole,
    pub content: String,
    pub timestamp_ms: u64,
    pub status: MessageStatus,
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
    #[serde(default)]
    pub timing: lain_wire::ActorTiming,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_memory: Option<lain_wire::memory::ChannelMemoryContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_source: Option<lain_wire::source::SourceReadContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub private_chat: Option<lain_wire::chat::PrivateChatContext>,
}
impl ActorTurn {
    pub fn validate(&self) -> anyhow::Result<()> {
        let wire: lain_wire::ActorSnapshot = serde_json::from_value(serde_json::to_value(self)?)?;
        wire.validate().map_err(anyhow::Error::msg)
    }
    pub fn message(&mut self, role: MessageRole, content: String) -> anyhow::Result<()> {
        self.messages.push(Message {
            id: MessageId::new(),
            conversation_id: self.conversation_id,
            role,
            content,
            timestamp_ms: crate::domain::now_ms()?,
            status: MessageStatus::Completed,
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
