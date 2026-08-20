use serde::{Deserialize, Serialize};

pub const EXTRACTOR_VERSION: &str = "memory-extractor-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    Preference,
    Fact,
    Constraint,
    Relationship,
    Episodic,
    OpenLoop,
    SharedSymbol,
}

impl MemoryType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Preference => "preference",
            Self::Fact => "fact",
            Self::Constraint => "constraint",
            Self::Relationship => "relationship",
            Self::Episodic => "episodic",
            Self::OpenLoop => "open_loop",
            Self::SharedSymbol => "shared_symbol",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryScope {
    User,
    Session,
    Relationship,
}

impl MemoryScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Session => "session",
            Self::Relationship => "relationship",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Explicitness {
    Explicit,
    Inferred,
}

impl Explicitness {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Explicit => "explicit",
            Self::Inferred => "inferred",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sensitivity {
    Public,
    Personal,
    Sensitive,
    Secret,
}

impl Sensitivity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Personal => "personal",
            Self::Sensitive => "sensitive",
            Self::Secret => "secret",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractedMemory {
    pub memory_type: MemoryType,
    pub scope: MemoryScope,
    pub subject: String,
    pub predicate: String,
    pub value: String,
    pub evidence_quote: String,
    pub explicitness: Explicitness,
    pub confidence: f32,
    pub importance: f32,
    pub sensitivity: Sensitivity,
    #[serde(default)]
    pub valid_from: Option<String>,
    #[serde(default)]
    pub valid_to: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractionEnvelope {
    pub candidates: Vec<ExtractedMemory>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateDisposition {
    Commit,
    Pending,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryItemStatus {
    Active,
    Superseded,
    Disputed,
    Retracted,
}

impl MemoryItemStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Superseded => "superseded",
            Self::Disputed => "disputed",
            Self::Retracted => "retracted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryLinkType {
    Source,
    Supersedes,
    Supports,
    Contradicts,
}

impl MemoryLinkType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Supersedes => "supersedes",
            Self::Supports => "supports",
            Self::Contradicts => "contradicts",
        }
    }
}

impl CandidateDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Commit => "committed",
            Self::Pending => "pending",
            Self::Reject => "rejected",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyDecision {
    pub disposition: CandidateDisposition,
    pub reason: &'static str,
    pub canonical_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionSource {
    pub run_id: String,
    pub session_id: String,
    pub user_message_id: String,
    pub user_text: String,
    pub observed_at: String,
    pub source_event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableJob {
    pub id: String,
    pub job_type: String,
    pub payload_json: String,
    pub attempts: i64,
    pub max_attempts: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedCandidate {
    pub candidate_id: String,
    pub status: String,
    pub item_id: Option<String>,
    pub item_version: Option<i64>,
    pub supersedes_id: Option<String>,
    pub inserted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MemoryItemView {
    pub id: String,
    pub memory_type: String,
    pub scope: String,
    pub subject: String,
    pub predicate: String,
    pub value: String,
    pub sensitivity: String,
    pub status: String,
    pub version: i64,
    pub source_message_id: String,
    pub evidence_quote: String,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub never_recall: bool,
    pub pinned: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PendingMemoryView {
    pub id: String,
    pub draft: ExtractedMemory,
    pub decision_reason: String,
    pub source_message_id: String,
    pub created_at: String,
}
