//! Transport-independent conversation delivery records. Not semantic memory.
use crate::actor::domain::{ActorTurnId, ConversationId};
use anyhow::{Result, ensure};
pub use lain_wire::source::{BatchState, DeliveryState, InputState};
pub use lain_wire::{BatchId, ChannelId, DeliveryId, IncomingId};
use serde::{Deserialize, Serialize};

pub const MAX_TEXT: usize = 8192;
pub const MAX_QUEUED: usize = 128;
pub const MAX_BATCH: usize = 16;
pub const MERGE_MS: u64 = 750;
pub const MAX_MERGE_MS: u64 = 3000;
pub const REPLY_TTL_MS: u64 = 10 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionPhase {
    NotStarted,
    Connecting,
    Authenticating,
    Online,
    Reconnecting,
    Stopped,
}

/// Closed failure categories: never retain an upstream error, frame or credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionFailure {
    ConnectionUnavailable,
    AuthenticationFailed,
    AccountMismatch,
    IdentityTimeout,
    ProtocolError,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub phase: ConnectionPhase,
    pub failure: Option<ConnectionFailure>,
    pub changed_at_ms: u64,
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        Self {
            phase: ConnectionPhase::NotStarted,
            failure: None,
            changed_at_ms: 0,
        }
    }
}
impl std::fmt::Display for ConnectionFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ConnectionFailure {}

#[derive(Debug, Clone, Serialize)]
pub struct ChannelStatus {
    pub binding: Binding,
    pub connection: ConnectionStatus,
}

/// Opaque, adapter-scoped routing identities; no platform protocol structs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindingSpec {
    pub name: String,
    pub adapter: String,
    pub account: String,
    pub peer: String,
}
impl BindingSpec {
    pub fn validate(&self) -> Result<()> {
        for value in [&self.name, &self.adapter, &self.account, &self.peer] {
            validate_reference(value)?;
        }
        ensure!(
            self.account != self.peer,
            "channel account must differ from peer"
        );
        Ok(())
    }
}
pub fn validate_reference(value: &str) -> Result<()> {
    ensure!(
        !value.is_empty()
            && value.len() <= 128
            && value
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-_.:/".contains(&c)),
        "invalid channel reference"
    );
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    pub id: ChannelId,
    pub spec: BindingSpec,
    pub conversation_id: ConversationId,
    pub reply_enabled: bool,
    pub grant_revision: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Input {
    pub account: String,
    pub peer: String,
    pub external_id: String,
    pub text: String,
}
impl Input {
    pub fn validate(&self) -> Result<()> {
        for reference in [&self.account, &self.peer, &self.external_id] {
            validate_reference(reference)?;
        }
        ensure!(
            !self.text.trim().is_empty() && self.text.len() <= MAX_TEXT,
            "invalid channel text"
        );
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incoming {
    pub id: IncomingId,
    pub channel_id: ChannelId,
    pub input: Input,
    pub received_at_ms: u64,
    pub grant_revision: u64,
    pub state: InputState,
    pub batch_id: Option<BatchId>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    pub id: BatchId,
    pub channel_id: ChannelId,
    pub conversation_id: ConversationId,
    pub grant_revision: u64,
    pub text: String,
    pub actor_turn_id: Option<ActorTurnId>,
    pub created_at_ms: u64,
    pub expires_at_ms: u64,
    pub state: BatchState,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delivery {
    pub id: DeliveryId,
    pub channel_id: ChannelId,
    pub batch_id: BatchId,
    pub grant_revision: u64,
    pub text: String,
    pub state: DeliveryState,
    pub created_at_ms: u64,
    pub expires_at_ms: u64,
    pub receipt: Option<String>,
    #[serde(default)]
    pub purpose: DeliveryPurpose,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryPurpose {
    #[default]
    Conversation,
    Approval,
    Failure,
}

/// Trailing quiet time can extend a live batch, never an already elapsed window.
pub fn merge_deadline(times: impl IntoIterator<Item = u64>) -> Option<u64> {
    let mut times = times.into_iter();
    let first = times.next()?;
    let cap = first.saturating_add(MAX_MERGE_MS);
    let mut due = first.saturating_add(MERGE_MS);
    let mut previous = first;
    for time in times {
        let time = time.max(previous);
        if time <= due {
            due = time.saturating_add(MERGE_MS).min(cap);
        }
        previous = time;
    }
    Some(due)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelSnapshot {
    pub binding: Binding,
    #[serde(default)]
    pub connection: ConnectionStatus,
    /// Latest 100 records in each category, chronological; not a complete export.
    pub incoming: Vec<Incoming>,
    pub batches: Vec<Batch>,
    pub deliveries: Vec<Delivery>,
}

/// Transport evidence, not provider error prose or a claim that the person read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendOutcome {
    Confirmed(String),
    NotSent,
    Rejected,
    Unknown,
}
