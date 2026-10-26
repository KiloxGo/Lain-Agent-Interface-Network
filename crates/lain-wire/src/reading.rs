//! Bounded acquisition receipts. Downloaded text is not a claim of comprehension.
use crate::{ReadingId, RequestId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadingScope {
    pub urls: Vec<String>,
    pub expires_at_ms: u64,
    pub retention_ms: u64,
    pub max_requests: u16,
    pub max_bytes: u32,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingProcessing {
    NotAcquired,
    AcquiredTextOnly,
}
