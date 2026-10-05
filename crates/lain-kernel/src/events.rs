use serde::{Deserialize, Serialize};

use crate::domain::{
    Approval, ApprovalId, CancellationState, EventId, RuntimeAttachment, Turn, TurnId,
    WorkSessionId, WorkSessionState, WorkerFailure,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub history_order: Option<u64>,
    #[serde(default)]
    pub truncated: bool,
    pub timestamp_ms: u64,
    pub work_session_id: WorkSessionId,
    pub turn_id: Option<TurnId>,
    #[serde(flatten)]
    pub kind: EventKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventKind {
    SessionCreated {
        session: crate::domain::WorkSession,
    },
    RuntimeAttachmentChanged {
        from: RuntimeAttachment,
        to: RuntimeAttachment,
    },
    SessionStateChanged {
        from: WorkSessionState,
        to: WorkSessionState,
    },
    TurnStarted {
        turn: Turn,
    },
    TurnCompleted {
        turn: Turn,
    },
    TurnStateChanged {
        turn: Turn,
    },
    CancellationChanged {
        state: CancellationState,
    },
    OutputProgress {
        category: String,
        characters: usize,
    },
    AssistantMessageCompleted {
        text: String,
    },
    CommandStarted {
        command: String,
    },
    CommandCompleted {
        command: String,
        output: String,
        exit_code: Option<i64>,
    },
    FileChanged {
        paths: Vec<String>,
    },
    ToolActivity {
        name: String,
        completed: bool,
    },
    ApprovalRequested {
        approval: Approval,
    },
    ApprovalResolved {
        approval: Approval,
    },
    ApprovalStateChanged {
        approval: Approval,
    },
    ApprovalDecisionSubmitted {
        approval_id: ApprovalId,
    },
    BackendDisconnected {
        failure: WorkerFailure,
    },
    BackendReconnected,
    ErrorObserved {
        failure: WorkerFailure,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryPage {
    pub events: Vec<Event>,
    /// Last returned order, or the requested cursor for an empty page.
    pub next_cursor: u64,
    pub has_more: bool,
}

impl Event {
    pub(crate) fn new(
        work_session_id: WorkSessionId,
        turn_id: Option<TurnId>,
        timestamp_ms: u64,
        mut kind: EventKind,
    ) -> Self {
        let mut truncated = false;
        let mut bound = |text: &mut String, max: usize| {
            if text.len() > max {
                let mut end = max;
                while !text.is_char_boundary(end) {
                    end -= 1;
                }
                text.truncate(end);
                truncated = true;
            }
        };
        match &mut kind {
            EventKind::AssistantMessageCompleted { text } => bound(text, 32 * 1024),
            EventKind::CommandStarted { command } => bound(command, 4 * 1024),
            EventKind::CommandCompleted {
                command, output, ..
            } => {
                bound(command, 4 * 1024);
                bound(output, 16 * 1024);
            }
            EventKind::FileChanged { paths } => {
                for path in paths.iter_mut() {
                    bound(path, 1024);
                }
                if paths.len() > 16 {
                    paths.truncate(16);
                    truncated = true;
                }
            }
            EventKind::ToolActivity { name, .. } => bound(name, 1024),
            EventKind::ApprovalRequested { approval }
            | EventKind::ApprovalResolved { approval }
            | EventKind::ApprovalStateChanged { approval } => {
                bound(&mut approval.description, 16 * 1024)
            }
            EventKind::BackendDisconnected { failure } | EventKind::ErrorObserved { failure } => {
                bound(&mut failure.message, 4096)
            }
            _ => {}
        }
        let turn_id = turn_id.or(match &kind {
            EventKind::TurnStarted { turn }
            | EventKind::TurnCompleted { turn }
            | EventKind::TurnStateChanged { turn } => Some(turn.id),
            EventKind::ApprovalRequested { approval }
            | EventKind::ApprovalResolved { approval }
            | EventKind::ApprovalStateChanged { approval } => approval.turn_id,
            _ => None,
        });
        Self {
            id: EventId::new(),
            history_order: None,
            truncated,
            timestamp_ms,
            work_session_id,
            turn_id,
            kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_payloads_bound_utf8_and_mark_truncation() {
        let event = Event::new(
            WorkSessionId::new(),
            None,
            0,
            EventKind::CommandCompleted {
                command: "中".repeat(4000),
                output: "\0".repeat(100_000),
                exit_code: Some(1),
            },
        );
        assert!(event.truncated);
        let EventKind::CommandCompleted {
            command, output, ..
        } = &event.kind
        else {
            unreachable!()
        };
        assert!(command.len() <= 4096 && output.len() <= 16384);
        assert!(serde_json::to_vec(&event).unwrap().len() < 262144);
        let files = Event::new(
            WorkSessionId::new(),
            None,
            0,
            EventKind::FileChanged {
                paths: vec!["\0".repeat(4096); 100],
            },
        );
        assert!(files.truncated && serde_json::to_vec(&files).unwrap().len() < 262144);
    }
}
