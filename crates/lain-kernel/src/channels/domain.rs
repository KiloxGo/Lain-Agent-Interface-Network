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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub segments: Vec<lain_wire::media::InputSegment>,
}
impl Input {
    pub fn validate(&self) -> Result<()> {
        for reference in [&self.account, &self.peer, &self.external_id] {
            validate_reference(reference)?;
        }
        ensure!(
            (!self.text.trim().is_empty() || !self.segments.is_empty())
                && self.text.len() <= MAX_TEXT
                && self.segments.len() <= 64,
            "invalid channel text"
        );
        if !self.segments.is_empty() {
            use lain_wire::media::InputSegment;
            let mut original = String::new();
            let mut media = std::collections::BTreeSet::new();
            for segment in &self.segments {
                match segment {
                    InputSegment::Text { text } => original.push_str(text),
                    InputSegment::Visual { media_id, .. } => {
                        ensure!(
                            media.insert(*media_id) && media.len() <= 8,
                            "invalid visual selection"
                        );
                    }
                    InputSegment::Expression { code, name } => {
                        ensure!(
                            !code.is_empty()
                                && code.len() <= 64
                                && !code.chars().any(char::is_control)
                                && name.as_ref().is_none_or(|n| !n.is_empty()
                                    && n.len() <= 128
                                    && !n.chars().any(char::is_control)),
                            "invalid expression metadata"
                        );
                    }
                    InputSegment::QuotedMessage { reference } => validate_reference(reference)?,
                }
            }
            ensure!(
                original == self.text && self.conversation_text().len() <= MAX_TEXT,
                "contradictory ordered conversation input"
            );
        }
        Ok(())
    }
    /// Adapter markers are rendered for conversation, never promoted to user quotations.
    pub fn conversation_text(&self) -> String {
        use lain_wire::media::InputSegment;
        if self.segments.is_empty() {
            return self.text.clone();
        }
        let mut text = String::new();
        let mut image = 0;
        for segment in &self.segments {
            match segment {
                InputSegment::Text { text: part } => text.push_str(part),
                InputSegment::Visual { visual_kind, .. } => {
                    image += 1;
                    text.push_str(&format!(
                        "[{} {}]",
                        if *visual_kind == lain_wire::media::VisualKind::Sticker {
                            "表情包"
                        } else {
                            "图片"
                        },
                        image
                    ));
                }
                InputSegment::Expression { code, name } => {
                    text.push_str(&format!("[QQ表情：{}]", name.as_deref().unwrap_or(code)))
                }
                InputSegment::QuotedMessage { .. } => text.push_str("[引用消息，未读取引用正文]"),
            }
        }
        text
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
    /// Latest 128 media metadata records; never pixel bodies or transport locators.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub media: Vec<lain_wire::media::ConversationMedia>,
}

/// Transport evidence, not provider error prose or a claim that the person read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendOutcome {
    Confirmed(String),
    NotSent,
    Rejected,
    Unknown,
}
