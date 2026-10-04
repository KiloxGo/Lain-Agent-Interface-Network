use serde::Serialize;

use crate::domain::{EventId, TurnId, WorkSessionId, WorkSessionState};

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
}
