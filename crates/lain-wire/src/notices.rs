//! Conversation notice DTOs. No provider objects, storage or execution logic.
use crate::{ActionId, ActorTurnId, ConversationId, NoticeId, WorkSessionId, WorkerTurnId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerOutcome {
    Completed,
    Failed,
    Cancelled,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NoticeOutcome {
    Worker { state: WorkerOutcome },
    SubmissionUnknown,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandFailure {
    pub command: String,
    pub exit_code: Option<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeError {
    pub kind: String,
    pub message: String,
    pub code: Option<i64>,
    pub retryable: Option<bool>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResultEvidence {
    pub final_assistant_message: Option<String>,
    pub failed_commands: Vec<CommandFailure>,
    pub failed_command_count: u64,
    pub changed_files: Vec<String>,
    pub errors: Vec<SafeError>,
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
    pub worker_turn_id: Option<WorkerTurnId>,
    pub timestamp_ms: u64,
    pub source_history_order: Option<u64>,
    pub outcome: NoticeOutcome,
    pub evidence: ResultEvidence,
    pub acknowledged: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoticePage {
    pub notices: Vec<ConversationNotice>,
    pub next_cursor: u64,
    pub has_more: bool,
}
impl NoticePage {
    pub fn validate(&self, conversation: ConversationId, after: u64) -> Result<(), &'static str> {
        if self.notices.len() > 100 {
            return Err("notice page exceeds bound");
        }
        let mut cursor = after;
        for notice in &self.notices {
            if notice.conversation_id != conversation
                || notice.cursor <= cursor
                || notice
                    .evidence
                    .final_assistant_message
                    .as_ref()
                    .is_some_and(|s| s.len() > 8192)
                || notice.evidence.failed_commands.len() > 8
                || notice.evidence.changed_files.len() > 16
                || notice.evidence.errors.len() > 4
                || (matches!(notice.outcome, NoticeOutcome::Worker { .. })
                    && (notice.worker_turn_id.is_none() || notice.source_history_order.is_none()))
            {
                return Err("invalid conversation notice");
            }
            cursor = notice.cursor;
        }
        if self.next_cursor != cursor {
            return Err("notice cursor mismatch");
        }
        Ok(())
    }
}
