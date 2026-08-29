use serde::{Deserialize, Serialize};

use crate::provider::ModelMessage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextBudget {
    pub max_input_tokens: u32,
    pub max_output_tokens: u32,
    pub identity_tokens: u32,
    pub examples_tokens: u32,
    pub project_instruction_tokens: u32,
    pub self_state_tokens: u32,
    pub relationship_tokens: u32,
    pub memory_tokens: u32,
    pub recent_tokens: u32,
    pub tool_result_reserve: u32,
    pub safety_margin: u32,
}

impl ContextBudget {
    pub fn for_model(context_window: u32, max_output_tokens: u32) -> Self {
        let reserved = max_output_tokens
            .saturating_add(4_096)
            .saturating_add(8_192);
        let available = context_window.saturating_sub(reserved).min(32_768);
        Self {
            max_input_tokens: available,
            max_output_tokens,
            identity_tokens: 8_192,
            examples_tokens: 2_048,
            project_instruction_tokens: 8_192,
            self_state_tokens: 512,
            relationship_tokens: 1_024,
            memory_tokens: 2_000,
            recent_tokens: 16_384,
            tool_result_reserve: 4_096,
            safety_margin: 8_192,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextSourceType {
    HarnessContract,
    Canon,
    Boundaries,
    Voice,
    Behavior,
    Example,
    ProjectInstructions,
    SelfState,
    Relationship,
    MemoryEvidence,
    RecentMessage,
    CurrentMessage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    Trusted,
    UserProvided,
    UntrustedEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DropReason {
    BudgetExceeded,
    Expired,
    NotRelevant,
    IncompletePair,
    DuplicateCurrentMessage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextPlanItem {
    pub item_id: String,
    pub source_type: ContextSourceType,
    pub source_version: String,
    pub trust_level: TrustLevel,
    pub priority: i32,
    pub estimated_tokens: u32,
    pub selected: bool,
    pub drop_reason: Option<DropReason>,
    pub final_position: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextPlan {
    pub id: String,
    pub run_id: String,
    pub identity_version: String,
    pub compiler_version: String,
    pub prompt_hash: String,
    pub stable_prefix_hash: String,
    pub estimated_tokens: u32,
    pub estimated: bool,
    pub selected_items: Vec<ContextPlanItem>,
    pub dropped_items: Vec<ContextPlanItem>,
}

#[derive(Debug, Clone)]
pub struct CompiledContext {
    pub plan: ContextPlan,
    pub messages: Vec<ModelMessage>,
}
