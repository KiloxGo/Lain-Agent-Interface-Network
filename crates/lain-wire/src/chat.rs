//! Private conversation decisions, distinct from actual outbound delivery truth.
use crate::{BatchId, ChannelId};
use serde::{Deserialize, Serialize};

pub const MAX_RECONSIDERATIONS: u8 = 2;
pub const MAX_WAIT_SECONDS: u8 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatDecision {
    #[default]
    Pending,
    Reply,
    Waiting,
    NoReply,
    Superseded,
    Notice,
    Interrupted,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateChatContext {
    pub channel_id: ChannelId,
    pub batch_id: BatchId,
    pub grant_revision: u64,
    pub expires_at_ms: u64,
    pub input_through_sequence: u64,
    pub reconsiderations: u8,
    pub waits: u8,
    pub decision: ChatDecision,
    pub wait_until_ms: Option<u64>,
    /// IDs/revisions supplied in the latest model request; never memory bodies.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub memory_versions: Vec<ChatMemoryVersion>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_versions: Vec<ChatSourceVersion>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub experience_versions: Vec<crate::reading::ExperienceVersion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experience_grant_revision: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub share_versions: Vec<crate::sharing::ShareVersion>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMemoryVersion {
    pub memory_id: crate::MemoryId,
    pub revision: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSourceVersion {
    pub selection: crate::source::SourceSelection,
    pub fingerprint: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatInputKind {
    Social,
    Question,
    Request,
    Work,
    Approval,
    Fault,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SilenceReason {
    ConversationClosed,
    BriefAcknowledgement,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinishChat {
    pub input_kind: ChatInputKind,
    pub reason: SilenceReason,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaitChat {
    pub seconds: u8,
}
