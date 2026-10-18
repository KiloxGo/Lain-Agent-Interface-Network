//! Durable dispatch identity and factual result delivery. Worker state remains
//! authoritative in the Supervisor; notices are immutable observations of it.
use crate::{
    actor::domain::{ActionId, ActorTurnId, ConversationId},
    domain::{RuntimeProfileId, TurnId, TurnState, WorkSessionId, WorkerFailure},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use lain_wire::NoticeId;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Origin {
    pub actor_turn_id: ActorTurnId,
    pub action_id: ActionId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionState {
    Prepared,
    Submitting,
    Accepted,
    Rejected,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delegation {
    pub conversation_id: ConversationId,
    pub actor_turn_id: ActorTurnId,
    pub action_id: ActionId,
    pub work_session_id: WorkSessionId,
    pub worker_turn_id: Option<TurnId>,
    pub workspace: PathBuf,
    pub profile_id: RuntimeProfileId,
    /// Exact confirmed worker-bound instruction; never compiled or rewritten.
    pub instruction: String,
    pub submission: SubmissionState,
    pub receipt: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NoticeOutcome {
    Worker { state: TurnState },
    SubmissionUnknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandFailure {
    pub command: String,
    pub exit_code: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResultEvidence {
    pub final_assistant_message: Option<String>,
    pub failed_commands: Vec<CommandFailure>,
    pub failed_command_count: u64,
    pub changed_files: Vec<String>,
    pub errors: Vec<WorkerFailure>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationNotice {
    pub id: NoticeId,
    pub cursor: u64,
    pub conversation_id: ConversationId,
    pub actor_turn_id: ActorTurnId,
    pub action_id: ActionId,
    pub work_session_id: WorkSessionId,
    pub worker_turn_id: Option<TurnId>,
    pub timestamp_ms: u64,
    /// Committed normalized event that established this observation, if any.
    pub source_history_order: Option<u64>,
    pub outcome: NoticeOutcome,
    pub evidence: ResultEvidence,
    /// Owner display acknowledgement, not proof of reading or task acceptance.
    pub acknowledged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoticePage {
    pub notices: Vec<ConversationNotice>,
    pub next_cursor: u64,
    pub has_more: bool,
}

pub(crate) fn bounded(text: &str, max: usize, truncated: &mut bool) -> String {
    // Defense in depth for legacy history. Runtime-specific known secrets have
    // already been removed before operational history commits.
    let safe = crate::redaction::Redactor::new(vec![])
        .for_runtime(None)
        .text(text);
    let safe: String = safe
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .collect();
    let mut end = safe.len().min(max);
    while !safe.is_char_boundary(end) {
        end -= 1;
    }
    *truncated |= end != safe.len();
    safe[..end].to_owned()
}
