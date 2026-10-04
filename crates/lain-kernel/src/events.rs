use serde::Serialize;

use crate::domain::{
    Approval, ApprovalId, CancellationState, EventId, Turn, TurnId, WorkSessionId,
    WorkSessionState, WorkerFailure,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Event {
    pub id: EventId,
    pub timestamp_ms: u64,
    pub work_session_id: WorkSessionId,
    pub turn_id: Option<TurnId>,
    #[serde(flatten)]
    pub kind: EventKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventKind {
    SessionCreated,
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
