//! Selected visible source archives. Import/disclosure never grants execution.
use crate::{ArchiveRecordId, ArchiveSourceId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ArchiveSourceKey {
    QqPrivate {
        observer_account: String,
        peer_id: String,
    },
    QqGroup {
        observer_account: String,
        group_id: String,
    },
    CodexSession {
        installation: String,
        session_id: String,
        workspace: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveSource {
    pub id: ArchiveSourceId,
    pub key: ArchiveSourceKey,
    pub created_at_ms: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveScope {
    pub expires_at_ms: u64,
    pub retention_ms: u64,
    pub max_records: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveGrant {
    pub source_id: ArchiveSourceId,
    pub revision: u64,
    pub scope: Option<ArchiveScope>,
    pub changed_at_ms: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetArchiveGrant {
    pub expected_revision: u64,
    pub scope: Option<ArchiveScope>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisibleRecordKind {
    Message,
    UserMessage,
    AssistantMessage,
    ExecutionResult,
    ApprovalNotice,
    FileChange,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportArchiveRecord {
    pub external_id: String,
    pub source_position: Option<String>,
    pub occurred_at_ms: Option<u64>,
    pub speaker: String,
    pub kind: VisibleRecordKind,
    pub text: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportArchive {
    pub expected_revision: u64,
    pub records: Vec<ImportArchiveRecord>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveAvailability {
    SourceUnavailable,
    Stored,
    Expired,
    Forgotten,
    Revoked,
    GrantExpired,
    PreviousGrant,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveTextOrigin {
    KnowledgeProjection,
    OwnerImport,
    OwnerCorrection,
    AdapterObservation,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveRecord {
    pub id: ArchiveRecordId,
    pub sequence: u64,
    pub source_id: ArchiveSourceId,
    pub grant_revision: u64,
    pub revision: u64,
    pub external_id: String,
    pub source_position: Option<String>,
    pub occurred_at_ms: Option<u64>,
    pub received_at_ms: u64,
    pub retain_until_ms: u64,
    pub speaker: String,
    pub kind: VisibleRecordKind,
    pub text: Option<String>,
    pub text_origin: ArchiveTextOrigin,
    pub availability: ArchiveAvailability,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveSourceView {
    pub source: ArchiveSource,
    pub grant: ArchiveGrant,
    pub total_records: u64,
    pub current_grant_records: u64,
    pub committed_through: u64,
    /// An imported slice does not prove complete coverage, observation or participation.
    pub coverage: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveImportState {
    Disabled,
    Enabled,
    Expired,
}
/// Owner-facing metadata; authorization, imports and collection liveness are separate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveSourceStatus {
    pub view: ArchiveSourceView,
    pub observed_at_ms: u64,
    pub import_state: ArchiveImportState,
    /// Current grant epoch only. Duplicate replay never advances this timestamp.
    pub latest_received_at_ms: Option<u64>,
    pub latest_reported_occurrence_at_ms: Option<u64>,
    /// Retained storage, not proof of disclosure authority or complete history.
    pub retained_body_count: u64,
    /// None until an actual collector-presence contract supplies evidence.
    pub receiving_now: Option<bool>,
    pub latest_read: Option<ArchiveReadJob>,
    pub upstream_snapshot: Option<KnowledgeArchiveState>,
    pub upstream_snapshot_fresh: bool,
    pub synchronization: KnowledgeSyncGrant,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveImportReceipt {
    pub records: Vec<ArchiveRecord>,
    pub new_records: u16,
    pub duplicate_records: u16,
    pub committed_through: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveCursor {
    pub source_id: ArchiveSourceId,
    pub grant_revision: u64,
    pub after: u64,
    pub through: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveQuery {
    pub expected_revision: u64,
    pub cursor: Option<ArchiveCursor>,
    pub limit: u16,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivePage {
    pub records: Vec<ArchiveRecord>,
    pub next: Option<ArchiveCursor>,
    pub through: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveRecordRevision {
    pub expected_revision: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectArchiveRecord {
    pub expected_revision: u64,
    pub text: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveEdit {
    pub record_id: ArchiveRecordId,
    pub revision: u64,
    pub kind: String,
    pub at_ms: u64,
}

/// One explicit authenticated read; this never enables background collection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadArchive {
    pub request_id: crate::RequestId,
    pub expected_revision: u64,
    pub after: Option<crate::RequestId>,
    pub limit: u16,
}
pub type ReadWorkerArchive = ReadArchive;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveReaderKind {
    #[default]
    Codex,
    Knowledge,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveReadState {
    Prepared,
    Reading,
    Completed,
    Failed,
    Cancelled,
    Unknown,
}
impl ArchiveReadState {
    pub fn unfinished(self) -> bool {
        matches!(self, Self::Prepared | Self::Reading)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveReadFailure {
    Unavailable,
    Unsupported,
    ScopeChanged,
    InvalidContent,
    Interrupted,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveReadJob {
    pub request: ReadWorkerArchive,
    #[serde(default)]
    pub reader: ArchiveReaderKind,
    #[serde(default)]
    pub reader_revision: Option<String>,
    #[serde(default)]
    pub sync_revision: Option<u64>,
    pub source_id: ArchiveSourceId,
    pub state: ArchiveReadState,
    pub created_at_ms: u64,
    pub started_at_ms: Option<u64>,
    pub finished_at_ms: Option<u64>,
    pub adapter_version: Option<String>,
    pub record_ids: Vec<ArchiveRecordId>,
    pub new_records: u16,
    pub duplicate_records: u16,
    #[serde(default)]
    pub updated_records: u16,
    #[serde(default)]
    pub withdrawn_records: u16,
    pub omitted_items: u16,
    pub waiting_for_source: bool,
    /// A Lain receipt ID, never a caller-supplied provider cursor.
    pub next: Option<crate::RequestId>,
    pub failure: Option<ArchiveReadFailure>,
    pub provider_error_code: Option<i64>,
    pub coverage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeArchiveState {
    pub source_id: ArchiveSourceId,
    pub local_grant_revision: u64,
    pub reader_revision: String,
    pub store_id: String,
    pub remote_source_revision: u64,
    pub remote_source_enabled: bool,
    pub through: u64,
    pub last_read: crate::RequestId,
    pub observed_at_ms: u64,
    pub fresh_until_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<ArchiveCoverage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveCoverageKind {
    ConnectedNoBackfill,
    DisconnectedGap,
    MediaUnavailable,
    OutboxDeliveryFailed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveCoverageEvent {
    pub sequence: u64,
    pub kind: ArchiveCoverageKind,
    /// When the upstream service received the report, not the actual gap onset.
    pub recorded_at_ms: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveCoverage {
    pub observed_at_ms: u64,
    pub bounded_history: bool,
    pub source_epochs_untracked: bool,
    pub events: Vec<ArchiveCoverageEvent>,
}
impl ArchiveCoverage {
    pub fn valid(&self) -> bool {
        self.observed_at_ms <= i64::MAX as u64
            && self.bounded_history
            && self.source_epochs_untracked
            && self.events.len() <= 20
            && self.events.iter().all(|e| {
                e.sequence > 0
                    && e.sequence <= i64::MAX as u64
                    && e.recorded_at_ms <= self.observed_at_ms
            })
            && self
                .events
                .windows(2)
                .all(|w| w[0].sequence > w[1].sequence)
    }
}

#[cfg(test)]
mod coverage_tests {
    use super::*;
    fn sample() -> ArchiveCoverage {
        ArchiveCoverage {
            observed_at_ms: 2000,
            bounded_history: true,
            source_epochs_untracked: true,
            events: vec![
                ArchiveCoverageEvent {
                    sequence: 4,
                    kind: ArchiveCoverageKind::DisconnectedGap,
                    recorded_at_ms: 1800,
                },
                ArchiveCoverageEvent {
                    sequence: 1,
                    kind: ArchiveCoverageKind::ConnectedNoBackfill,
                    recorded_at_ms: 1000,
                },
            ],
        }
    }
    #[test]
    fn coverage_is_bounded_historical_metadata_without_future_reports() {
        let c = sample();
        assert!(c.valid());
        let mut future = c.clone();
        future.events[0].recorded_at_ms = future.observed_at_ms + 1;
        assert!(!future.valid());
        let mut unordered = c.clone();
        unordered.events.reverse();
        assert!(!unordered.valid());
        let mut duplicate = c.clone();
        duplicate.events[1].sequence = duplicate.events[0].sequence;
        assert!(!duplicate.valid());
        let mut oversized = c.clone();
        oversized.events = vec![c.events[0].clone(); 21];
        assert!(!oversized.valid());
        let mut claims_complete = c;
        claims_complete.bounded_history = false;
        assert!(!claims_complete.valid());
        assert!(serde_json::from_str::<ArchiveCoverage>(r#"{"observed_at_ms":2000,"bounded_history":true,"source_epochs_untracked":true,"events":[{"sequence":1,"kind":"online","recorded_at_ms":1000}]}"#).is_err());
    }
}

/// Independent, default-off authority for scheduled GETs of one source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeSyncScope {
    pub archive_revision: u64,
    pub expires_at_ms: u64,
    pub max_requests: u32,
    pub interval_ms: u64,
    pub limit: u16,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetKnowledgeSync {
    pub expected_revision: u64,
    /// Optional launch-time configuration fence, independent of source authority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_reader_revision: Option<String>,
    pub scope: Option<KnowledgeSyncScope>,
}
/// Local configuration/credential availability, never proof of a live connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeReaderStatus {
    pub configured: bool,
    pub credentials_available: bool,
    pub revision: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeSyncStop {
    Expired,
    Exhausted,
    ScopeChanged,
    ReaderChanged,
    ReadFailed,
    SourceUnavailable,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSyncGrant {
    pub source_id: ArchiveSourceId,
    pub revision: u64,
    pub scope: Option<KnowledgeSyncScope>,
    pub reader_revision: Option<String>,
    pub used_requests: u32,
    pub next_at_ms: u64,
    pub last_read: Option<crate::RequestId>,
    pub pending: bool,
    pub stopped: Option<KnowledgeSyncStop>,
    pub changed_at_ms: u64,
}

/// Disclosure to one private conversation is independent of archive import/read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveRecallSource {
    pub source_id: ArchiveSourceId,
    pub grant_revision: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveRecallScope {
    pub sources: Vec<ArchiveRecallSource>,
    pub expires_at_ms: u64,
    pub utc_offset_minutes: i16,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveRecallGrant {
    pub channel_id: crate::ChannelId,
    pub revision: u64,
    pub scope: Option<ArchiveRecallScope>,
    pub changed_at_ms: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetArchiveRecall {
    pub expected_revision: u64,
    pub scope: Option<ArchiveRecallScope>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveRecallQuery {
    pub source_id: Option<ArchiveSourceId>,
    pub from_ms: u64,
    pub to_ms: u64,
    pub text: Option<String>,
    pub limit: u16,
}
impl ArchiveRecallQuery {
    pub fn valid(&self) -> bool {
        self.from_ms < self.to_ms
            && self.to_ms <= i64::MAX as u64
            && self.to_ms - self.from_ms <= 31 * 86400000
            && (1..=3).contains(&self.limit)
            && self.text.as_ref().is_none_or(|s| {
                !s.trim().is_empty() && s.len() <= 256 && !s.chars().any(char::is_control)
            })
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveRecallVersion {
    pub record_id: ArchiveRecordId,
    pub revision: u64,
    pub source_id: ArchiveSourceId,
    pub source_grant_revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveMemoryScope {
    pub sources: Vec<ArchiveRecallSource>,
    pub expires_at_ms: u64,
    pub retention_ms: u64,
    pub max_writes: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveMemoryGrant {
    pub channel_id: crate::ChannelId,
    pub revision: u64,
    pub scope: Option<ArchiveMemoryScope>,
    pub used_writes: u32,
    pub changed_at_ms: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetArchiveMemory {
    pub expected_revision: u64,
    pub scope: Option<ArchiveMemoryScope>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RememberArchive {
    pub content: String,
    pub kind: crate::memory::MemoryKind,
}
