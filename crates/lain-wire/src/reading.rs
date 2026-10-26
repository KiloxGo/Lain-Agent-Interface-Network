//! Bounded acquisition receipts. Downloaded text is not a claim of comprehension.
use crate::{ActorTurnId, ChannelId, ExperienceId, ReadingId, RequestId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadingScope {
    pub urls: Vec<String>,
    pub expires_at_ms: u64,
    pub retention_ms: u64,
    pub max_requests: u16,
    pub max_bytes: u32,
    /// Explicit model processing and disclosure to one existing private binding.
    #[serde(default)]
    pub processing: Option<ReadingModelScope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadingModelScope {
    pub channel_id: ChannelId,
    pub max_requests: u16,
    /// Fixed display/day-boundary offset; does not claim IANA/DST support.
    pub utc_offset_minutes: i16,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReadingGrant {
    pub revision: u64,
    pub changed_at_ms: u64,
    pub scope: Option<ReadingScope>,
}
impl ReadingGrant {
    pub fn active(&self, now: u64) -> bool {
        self.scope.as_ref().is_some_and(|s| now < s.expires_at_ms)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetReadingGrant {
    pub expected_revision: u64,
    pub scope: Option<ReadingScope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartReading {
    pub request_id: RequestId,
    pub grant_revision: u64,
    pub url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Unknown,
    Revoked,
    Expired,
}
impl ReadingState {
    pub fn unfinished(self) -> bool {
        matches!(self, Self::Pending | Self::Running)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingFailure {
    Network,
    AddressDenied,
    HttpStatus,
    UnsupportedContent,
    TooLarge,
    Sensitive,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingActivity {
    pub id: ReadingId,
    pub request: StartReading,
    pub state: ReadingState,
    pub created_at_ms: u64,
    pub started_at_ms: Option<u64>,
    pub finished_at_ms: Option<u64>,
    pub retain_until_ms: u64,
    pub acquired_bytes: u32,
    pub failure: Option<ReadingFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingResult {
    pub activity: ReadingActivity,
    /// Untrusted source data. None after expiry/revocation or without a result.
    pub text: Option<String>,
    pub processing: ReadingProcessing,
    #[serde(default)]
    pub processing_turn: Option<ActorTurnId>,
    #[serde(default)]
    pub processing_cancel_requested_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingProcessing {
    NotAcquired,
    AcquiredTextOnly,
    ModelSummaryGenerated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperienceState {
    Active,
    Withdrawn,
    Expired,
    Revoked,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperienceSummaryOrigin {
    Model,
    OwnerCorrection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingExperience {
    pub id: ExperienceId,
    pub reading_id: ReadingId,
    pub actor_turn_id: ActorTurnId,
    pub grant_revision: u64,
    pub revision: u64,
    pub state: ExperienceState,
    pub acquired_at_ms: u64,
    pub completed_at_ms: u64,
    pub source_url: String,
    pub source_bytes: u32,
    pub model_input_bytes: u32,
    pub source_truncated: bool,
    /// Model interpretation, not a verified conclusion or semantic memory.
    pub summary: Option<String>,
    pub summary_origin: ExperienceSummaryOrigin,
    pub summary_original_bytes: u32,
    pub summary_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExperienceQuery {
    pub from_ms: u64,
    pub to_ms: u64,
    pub text: Option<String>,
    pub limit: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WithdrawExperience {
    pub expected_revision: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectExperience {
    pub expected_revision: u64,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreReadingSource {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceVersion {
    pub experience_id: ExperienceId,
    pub revision: u64,
    pub grant_revision: u64,
    pub fingerprint: String,
}
