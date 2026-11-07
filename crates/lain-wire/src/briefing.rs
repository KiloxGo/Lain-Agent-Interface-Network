//! Explicit per-conversation disclosure. A day manifest is evidence, not a summary or memory.
use crate::{
    ConversationId, RequestId,
    archives::{ArchiveRecallSource, ArchiveRecallVersion, ArchiveRecord, ArchiveSourceStatus},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BriefingScope {
    pub sources: Vec<ArchiveRecallSource>,
    pub from_ms: u64,
    pub expires_at_ms: u64,
    pub utc_offset_minutes: i16,
    /// Hard selection bound, not a claim that more data does not exist.
    pub max_records: u16,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefingGrant {
    pub conversation_id: ConversationId,
    pub revision: u64,
    pub scope: Option<BriefingScope>,
    pub changed_at_ms: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetBriefingGrant {
    pub request_id: RequestId,
    pub expected_revision: u64,
    pub scope: Option<BriefingScope>,
}
/// Historical operation evidence, not the current authority or permission to replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefingGrantReceipt {
    pub request: SetBriefingGrant,
    pub grant: BriefingGrant,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BriefingDay {
    Yesterday,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartBriefing {
    pub request_id: RequestId,
    pub grant_revision: u64,
    pub day: BriefingDay,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefingSource {
    pub status: ArchiveSourceStatus,
    pub through_sequence: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefingManifest {
    pub conversation_id: ConversationId,
    pub request: StartBriefing,
    pub created_at_ms: u64,
    pub expires_at_ms: u64,
    pub utc_offset_minutes: i16,
    pub day_from_ms: u64,
    pub day_to_ms: u64,
    pub permitted_from_ms: u64,
    pub scope_clipped: bool,
    pub record_limit_reached: bool,
    pub unavailable_records: u64,
    pub receipt_time_fallback_records: u64,
    pub records: Vec<ArchiveRecallVersion>,
    pub sources: Vec<BriefingSource>,
    /// Always false: imported/retained data cannot prove continuous upstream collection.
    pub complete_upstream_history: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefingPage {
    pub briefing_id: RequestId,
    pub conversation_id: ConversationId,
    pub from_index: u16,
    pub next_index: Option<u16>,
    pub total_selected: u16,
    pub text_bytes: u32,
    pub records: Vec<ArchiveRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BriefingActivity {
    pub conversation_id: ConversationId,
    pub briefing_id: RequestId,
    pub from_index: u16,
    #[serde(default, skip_serializing_if = "BriefingStage::is_extract")]
    pub stage: BriefingStage,
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BriefingStage {
    #[default]
    Extract,
    Index,
    Compare {
        item_number: u16,
    },
}
impl BriefingStage {
    pub fn is_extract(&self) -> bool {
        matches!(self, Self::Extract)
    }
    pub fn job_key(self) -> Option<u16> {
        match self {
            Self::Extract => None,
            Self::Index => Some(0),
            Self::Compare { item_number } => Some(item_number),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BriefingCategory {
    Schedule,
    Deadline,
    Resource,
    News,
    Other,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefingCitation {
    pub source: ArchiveRecallVersion,
    pub quote: String,
    pub text_origin: crate::archives::ArchiveTextOrigin,
    pub occurred_at_ms: Option<u64>,
    pub received_at_ms: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefingItem {
    pub id: crate::BriefingItemId,
    pub title: String,
    pub summary: String,
    pub category: BriefingCategory,
    /// Sender-relative time, quoted exactly; UTC normalization is not implied.
    pub time_text: Option<String>,
    pub uncertainty: String,
    pub citations: Vec<BriefingCitation>,
    pub summary_origin: BriefingSummaryOrigin,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BriefingSummaryOrigin {
    Model,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefingBatch {
    pub activity: BriefingActivity,
    pub actor_turn_id: crate::ActorTurnId,
    pub state: crate::ActorTurnState,
    pub next_index: Option<u16>,
    pub source_current: bool,
    pub items: Option<Vec<BriefingItem>>,
    pub important_items_omitted: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BriefingRunPhase {
    Running,
    Reconciling,
    Completed,
    Partial,
    Unknown,
    SourceUnavailable,
}
impl BriefingRunPhase {
    pub fn terminal(self) -> bool {
        !matches!(self, Self::Running | Self::Reconciling)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BriefingRunStop {
    Interrupted,
    ModelFailed,
    BatchLimit,
    Deadline,
    SourceChanged,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefingRun {
    pub conversation_id: ConversationId,
    pub briefing_id: RequestId,
    pub phase: BriefingRunPhase,
    pub stop: Option<BriefingRunStop>,
    pub started_at_ms: u64,
    pub finished_at_ms: Option<u64>,
    pub expires_at_ms: u64,
    pub selected_records: u16,
    pub processed_records: u16,
    pub completed_batches: u16,
    pub next_index: Option<u16>,
    pub important_items_omitted: bool,
    pub scope_partial: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefingRunStatus {
    pub run: BriefingRun,
    pub source_current: bool,
    pub day_from_ms: u64,
    pub day_to_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day_from_local: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day_to_local_exclusive: Option<String>,
    pub utc_offset_minutes: i16,
    pub unavailable_records: u64,
    pub complete_upstream_history: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BriefingRead {
    pub briefing_id: Option<RequestId>,
    pub after: u16,
    pub limit: u16,
    /// One literal phrase in this manifest's generated items and checked quotes.
    /// Missing/null preserves ordinary paged reading; this is not archive search.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BriefingInspect {
    pub briefing_id: Option<RequestId>,
    pub item_number: u16,
    pub citation_index: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RememberBriefing {
    pub briefing_id: RequestId,
    pub item_number: u16,
    /// False preserves exact original-item selection; true selects its checked group.
    #[serde(default)]
    pub include_related: bool,
}
