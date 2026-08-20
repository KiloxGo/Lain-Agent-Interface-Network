use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EventEnvelope {
    pub v: u16,
    pub event_id: String,
    pub instance_id: String,
    pub session_id: Option<String>,
    pub run_id: Option<String>,
    pub seq: Option<i64>,
    #[serde(rename = "type")]
    pub event_type: String,
    pub ts: String,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum PersistedEvent {
    #[serde(rename = "message.accepted")]
    MessageAccepted {
        message_id: String,
        client_message_id: String,
        content: String,
    },
    #[serde(rename = "run.queued")]
    RunQueued { trigger_message_id: String },
    #[serde(rename = "run.started")]
    RunStarted { provider: String, model: String },
    #[serde(rename = "text.completed")]
    TextCompleted { message_id: String, content: String },
    #[serde(rename = "run.completed")]
    RunCompleted {
        message_id: String,
        finish_reason: String,
        usage: Option<Value>,
    },
    #[serde(rename = "run.cancelled")]
    RunCancelled,
    #[serde(rename = "run.failed")]
    RunFailed { code: String },
    #[serde(rename = "identity.activated")]
    IdentityActivated {
        version: String,
        content_hash: String,
    },
    #[serde(rename = "context.compiled")]
    ContextCompiled {
        plan_id: String,
        prompt_hash: String,
        identity_version: String,
        estimated_tokens: u32,
        estimated: bool,
        dropped_items: usize,
    },
    #[serde(rename = "memory.candidate_evaluated")]
    MemoryCandidateEvaluated {
        candidate_id: String,
        source_message_id: String,
        status: String,
        reason: String,
    },
    #[serde(rename = "memory.item_committed")]
    MemoryItemCommitted {
        item_id: String,
        candidate_id: String,
        version: i64,
        supersedes_id: Option<String>,
    },
    #[serde(rename = "memory.item_status_changed")]
    MemoryItemStatusChanged {
        item_id: String,
        status: String,
        pinned: bool,
        never_recall: bool,
    },
}

impl PersistedEvent {
    pub const fn event_type(&self) -> &'static str {
        match self {
            Self::MessageAccepted { .. } => "message.accepted",
            Self::RunQueued { .. } => "run.queued",
            Self::RunStarted { .. } => "run.started",
            Self::TextCompleted { .. } => "text.completed",
            Self::RunCompleted { .. } => "run.completed",
            Self::RunCancelled => "run.cancelled",
            Self::RunFailed { .. } => "run.failed",
            Self::IdentityActivated { .. } => "identity.activated",
            Self::ContextCompiled { .. } => "context.compiled",
            Self::MemoryCandidateEvaluated { .. } => "memory.candidate_evaluated",
            Self::MemoryItemCommitted { .. } => "memory.item_committed",
            Self::MemoryItemStatusChanged { .. } => "memory.item_status_changed",
        }
    }

    pub fn payload(&self) -> serde_json::Result<Value> {
        let value = serde_json::to_value(self)?;
        Ok(value
            .get("data")
            .cloned()
            .unwrap_or_else(|| Value::Object(Default::default())))
    }
}
