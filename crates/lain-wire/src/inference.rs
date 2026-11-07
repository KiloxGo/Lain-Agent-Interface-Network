//! Optional Kernel-wide Lain inference ceiling; never grants source/tool authority.
use crate::{ActorTurnId, InferenceBudgetId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InferenceLimit {
    pub id: InferenceBudgetId,
    pub max_requests: u32,
    pub expires_at_ms: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceDecision {
    pub revision: u64,
    pub enabled: bool,
    pub limit: Option<InferenceLimit>,
    /// Explicitly remove only the aggregate count/expiry ceiling.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unlimited: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unlimited_private_channel: Option<crate::ChannelId>,
    pub changed_at_ms: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetInferenceBudget {
    pub expected_revision: u64,
    pub limit: Option<InferenceLimit>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unlimited: bool,
    #[serde(default)]
    pub unlimited_private_channel: Option<crate::ChannelId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InferenceAdmission {
    PrivateConversationOnly,
    ExistingLimitsOnly,
    Available,
    Stopped,
    Expired,
    Exhausted,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InferenceRequestState {
    Prepared,
    Attempting,
    Returned,
    NotAttempted,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InferenceReason {
    Briefing,
    Conversation,
    PrivateConversation,
    Reading,
    Sharing,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub actor_turn_id: ActorTurnId,
    pub request_number: u32,
    pub budget_id: Option<InferenceBudgetId>,
    pub decision_revision: u64,
    pub reason: InferenceReason,
    pub state: InferenceRequestState,
    pub prepared_at_ms: u64,
    pub attempted_at_ms: Option<u64>,
    pub returned_at_ms: Option<u64>,
    pub reported_usage_tokens: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceBudgetStatus {
    pub limit: InferenceLimit,
    pub reserved_requests: u64,
    pub reported_usage_tokens_sum: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceSnapshot {
    pub coverage: String,
    pub usage_precision: String,
    pub decision: InferenceDecision,
    pub admission: InferenceAdmission,
    pub reserved_requests: u64,
    pub remaining_requests: Option<u64>,
    pub reported_usage_tokens_sum: u64,
    pub pending_actor_requests: u64,
    pub recent_requests: Vec<InferenceRequest>,
}
