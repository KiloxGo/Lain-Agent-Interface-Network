//! Bounded historical evidence, separate from semantic memory and permissions to act.
use crate::{
    ActionId, ActorTurnId, ActorTurnState, BatchId, ChannelId, ConversationId, DeliveryId,
    IncomingId,
};
use serde::{Deserialize, Serialize};

pub const MAX_WINDOW_MS: u64 = 31 * 24 * 60 * 60 * 1000;
pub const MAX_PAGE_BYTES: usize = 8192;
pub const MAX_SCAN: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputState {
    Queued,
    Batched,
    Revoked,
    Expired,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchState {
    Prepared,
    Running,
    Finished,
    Revoked,
    Expired,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryState {
    Ready,
    Sending,
    Confirmed,
    NotSent,
    Rejected,
    Unknown,
    Revoked,
    Expired,
    Superseded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceWindow {
    pub from_received_at_ms: u64,
    pub to_received_at_ms: u64,
}
impl SourceWindow {
    pub fn valid(&self) -> bool {
        self.from_received_at_ms < self.to_received_at_ms
            && self.to_received_at_ms <= i64::MAX as u64
            && self.to_received_at_ms - self.from_received_at_ms <= MAX_WINDOW_MS
    }
    pub fn contains(&self, time: u64) -> bool {
        time >= self.from_received_at_ms && time < self.to_received_at_ms
    }
    pub fn within(&self, outer: Self) -> bool {
        self.valid()
            && self.from_received_at_ms >= outer.from_received_at_ms
            && self.to_received_at_ms <= outer.to_received_at_ms
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceGrant {
    pub channel_id: ChannelId,
    pub enabled: bool,
    pub revision: u64,
    pub window: Option<SourceWindow>,
    pub through_sequence: u64,
    pub expires_at_ms: Option<u64>,
    pub changed_at_ms: u64,
}
impl SourceGrant {
    pub fn active(&self, now: u64) -> bool {
        self.enabled
            && self.revision > 0
            && self.window.is_some_and(|w| w.valid())
            && self.expires_at_ms.is_none_or(|t| now < t)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetSourceGrant {
    pub enabled: bool,
    pub expected_revision: u64,
    pub window: Option<SourceWindow>,
    pub expires_at_ms: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceReadContext {
    pub channel_id: ChannelId,
    pub batch_id: BatchId,
    pub grant_revision: u64,
    pub expires_at_ms: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceCursor {
    pub channel_id: ChannelId,
    pub grant_revision: u64,
    pub through_sequence: u64,
    pub after_sequence: u64,
    pub after_received_at_ms: u64,
    pub query_digest: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceQuery {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub window: Option<SourceWindow>,
    #[serde(default)]
    pub after: Option<SourceCursor>,
    pub limit: u16,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectSource {
    pub incoming_id: IncomingId,
    #[serde(default)]
    pub window: Option<SourceWindow>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceExcerpt {
    pub text: String,
    pub original_bytes: usize,
    pub truncated: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplyVisibility {
    Visible,
    Missing,
    OutsideWindow,
    BatchOutsideWindow,
    SourceDerived,
    MemoryChanged,
    Sensitive,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceReply {
    pub actor_turn_id: Option<ActorTurnId>,
    pub actor_state: Option<ActorTurnState>,
    pub model_request_attempts: Option<u32>,
    pub generated: Option<SourceExcerpt>,
    pub delivery: Option<SourceExcerpt>,
    pub visibility: ReplyVisibility,
    pub delivery_id: Option<DeliveryId>,
    pub delivery_state: Option<DeliveryState>,
    pub platform_receipt: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRecord {
    pub incoming_id: IncomingId,
    pub channel_id: ChannelId,
    pub conversation_id: ConversationId,
    pub batch_id: Option<BatchId>,
    pub received_at_ms: u64,
    pub input_state: InputState,
    pub batch_state: Option<BatchState>,
    pub incoming: SourceExcerpt,
    pub reply: SourceReply,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcePage {
    pub records: Vec<SourceRecord>,
    pub next_cursor: SourceCursor,
    pub has_more: bool,
    pub scan_limit_reached: bool,
    pub examined: u16,
    pub coverage: SourceCoverage,
    pub time_semantics: SourceTimeSemantics,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceCoverage {
    PartialNoBackfill,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceTimeSemantics {
    ReceivedUtcUnixMs,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceOperation {
    Search,
    Inspect,
    PreparedForModelRequest,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceAudit {
    pub channel_id: ChannelId,
    pub actor_turn_id: Option<ActorTurnId>,
    pub action_id: Option<ActionId>,
    pub model_request: Option<u32>,
    pub operation: SourceOperation,
    pub grant_revision: u64,
    pub incoming_ids: Vec<IncomingId>,
    pub window: SourceWindow,
    pub query_digest: Option<String>,
    pub recorded_at_ms: u64,
}
/// Kernel-produced selection, not model-supplied permission or cached source text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSelection {
    pub channel_id: ChannelId,
    pub grant_revision: u64,
    pub window: SourceWindow,
    pub through_sequence: u64,
    pub incoming_ids: Vec<IncomingId>,
}
