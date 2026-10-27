//! Selected visible source archives. Import/disclosure never grants execution.
use crate::{ArchiveRecordId, ArchiveSourceId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ArchiveSourceKey {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct ReadWorkerArchive {
    pub request_id: crate::RequestId,
    pub expected_revision: u64,
    pub after: Option<crate::RequestId>,
    pub limit: u16,
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
    pub source_id: ArchiveSourceId,
    pub state: ArchiveReadState,
    pub created_at_ms: u64,
    pub started_at_ms: Option<u64>,
    pub finished_at_ms: Option<u64>,
    pub adapter_version: Option<String>,
    pub record_ids: Vec<ArchiveRecordId>,
    pub new_records: u16,
    pub duplicate_records: u16,
    pub omitted_items: u16,
    pub waiting_for_source: bool,
    /// A Lain receipt ID, never a caller-supplied provider cursor.
    pub next: Option<crate::RequestId>,
    pub failure: Option<ArchiveReadFailure>,
    pub provider_error_code: Option<i64>,
    pub coverage: String,
}
