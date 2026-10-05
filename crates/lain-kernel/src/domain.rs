use std::{fmt, path::PathBuf, str::FromStr, time::SystemTime};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

macro_rules! uuid_id {
    ($($name:ident),+ $(,)?) => {$ (
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self { Self(Uuid::new_v4()) }
        }
        impl Default for $name {
            fn default() -> Self { Self::new() }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.0.fmt(f) }
        }
        impl FromStr for $name {
            type Err = uuid::Error;
            fn from_str(value: &str) -> Result<Self, Self::Err> { value.parse().map(Self) }
        }
    )+};
}

uuid_id!(WorkSessionId, TurnId, EventId, ApprovalId);

macro_rules! named_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl TryFrom<String> for $name {
            type Error = DomainError;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                if value.is_empty()
                    || value.len() > 128
                    || !value.split('/').all(|part| {
                        !part.is_empty()
                            && part
                                .bytes()
                                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
                    })
                {
                    return Err(DomainError::InvalidName);
                }
                Ok(Self(value))
            }
        }
        impl FromStr for $name {
            type Err = DomainError;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                value.to_owned().try_into()
            }
        }
        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

named_id!(RuntimeProfileId);
named_id!(BackendKind);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeAttachment {
    #[default]
    Detached,
    Starting,
    Attached,
    Stopping,
    Lost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkSessionState {
    Created,
    Starting,
    Ready,
    Running,
    WaitingForApproval,
    Idle,
    Completed,
    Cancelled,
    Failed,
    Disconnected,
    Unknown,
    Archived,
}

impl WorkSessionState {
    pub fn can_transition_to(self, next: Self) -> bool {
        use WorkSessionState::*;
        match self {
            Created => matches!(next, Starting | Cancelled | Failed),
            Starting => matches!(next, Ready | Cancelled | Failed | Disconnected | Unknown),
            Ready => matches!(
                next,
                Idle | Running | Cancelled | Failed | Disconnected | Unknown
            ),
            Running => matches!(
                next,
                WaitingForApproval | Idle | Completed | Cancelled | Failed | Disconnected | Unknown
            ),
            WaitingForApproval => {
                matches!(
                    next,
                    Running | Idle | Cancelled | Failed | Disconnected | Unknown
                )
            }
            Idle => matches!(
                next,
                Running | Completed | Cancelled | Failed | Disconnected | Unknown
            ),
            Disconnected => matches!(
                next,
                Unknown
                    | Ready
                    | Running
                    | WaitingForApproval
                    | Idle
                    | Completed
                    | Cancelled
                    | Failed
            ),
            Unknown => matches!(
                next,
                Disconnected
                    | Ready
                    | Running
                    | WaitingForApproval
                    | Idle
                    | Completed
                    | Cancelled
                    | Failed
            ),
            Completed | Cancelled | Failed => next == Archived,
            Archived => false,
        }
    }

    pub fn needs_reconciliation(self) -> bool {
        matches!(
            self,
            Self::Starting
                | Self::Ready
                | Self::Running
                | Self::WaitingForApproval
                | Self::Idle
                | Self::Disconnected
        )
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxMode {
    #[default]
    ReadOnly,
    WorkspaceWrite,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionOptions {
    pub model: Option<String>,
    #[serde(default)]
    pub sandbox: Option<SandboxMode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnState {
    Starting,
    Running,
    Completed,
    Cancelled,
    Failed,
    Unknown,
}

impl TurnState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled | Self::Failed)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CancellationState {
    #[default]
    NotRequested,
    Requested,
    Rejected,
    Acknowledged,
    Confirmed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Turn {
    pub id: TurnId,
    pub work_session_id: WorkSessionId,
    pub state: TurnState,
    pub cancellation: CancellationState,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub failure: Option<WorkerFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    Unavailable,
    Protocol,
    Rejected,
    Execution,
    Storage,
    Unsupported,
    AuthenticationFailed,
    RateLimited,
    ProviderUnavailable,
    PermissionDenied,
    SessionNotFound,
    SessionNotResumable,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerFailure {
    pub kind: FailureKind,
    /// A safe category/message, never a raw backend error payload.
    pub message: String,
    pub code: Option<i64>,
    #[serde(default)]
    pub retryable: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalKind {
    Command,
    FileChange,
    Permissions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    ApproveOnce,
    Deny,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Responding,
    Resolved,
    Expired,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Approval {
    pub id: ApprovalId,
    pub work_session_id: WorkSessionId,
    pub turn_id: Option<TurnId>,
    pub kind: ApprovalKind,
    pub description: String,
    pub decisions: Vec<ApprovalDecision>,
    pub status: ApprovalStatus,
    pub decision: Option<ApprovalDecision>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("invalid backend/profile identifier")]
    InvalidName,
    #[error("workspace must be an existing directory with a UTF-8 path")]
    InvalidWorkspace,
    #[error("cannot resolve workspace")]
    Workspace(#[source] std::io::Error),
    #[error("invalid persisted session timestamps or workspace")]
    InvalidRecord,
    #[error("invalid session transition: {from:?} -> {to:?}")]
    InvalidTransition {
        from: WorkSessionState,
        to: WorkSessionState,
    },
    #[error("system clock is outside the supported Unix timestamp range")]
    Clock,
}

/// UTC milliseconds since the Unix epoch. No scheduling/timezone policy is implied.
pub fn now_ms() -> Result<u64, DomainError> {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_| DomainError::Clock)?
        .as_millis()
        .try_into()
        .map_err(|_| DomainError::Clock)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkSession {
    pub(crate) id: WorkSessionId,
    pub(crate) backend: BackendKind,
    pub(crate) runtime_profile_id: RuntimeProfileId,
    pub(crate) workspace: PathBuf,
    pub(crate) state: WorkSessionState,
    #[serde(default)]
    pub(crate) runtime: RuntimeAttachment,
    pub(crate) created_at_ms: u64,
    pub(crate) updated_at_ms: u64,
}

impl WorkSession {
    pub(crate) fn new(
        backend: BackendKind,
        runtime_profile_id: RuntimeProfileId,
        workspace: PathBuf,
    ) -> Result<Self, DomainError> {
        let workspace = workspace.canonicalize().map_err(DomainError::Workspace)?;
        if !workspace.is_dir() || workspace.to_str().is_none() {
            return Err(DomainError::InvalidWorkspace);
        }
        let now = now_ms()?;
        Ok(Self {
            id: WorkSessionId::new(),
            backend,
            runtime_profile_id,
            workspace,
            state: WorkSessionState::Created,
            runtime: RuntimeAttachment::Detached,
            created_at_ms: now,
            updated_at_ms: now,
        })
    }

    pub fn id(&self) -> WorkSessionId {
        self.id
    }
    pub fn state(&self) -> WorkSessionState {
        self.state
    }
    pub fn runtime(&self) -> RuntimeAttachment {
        self.runtime
    }

    pub(crate) fn validate(&self) -> Result<(), DomainError> {
        // A removed workspace must not prevent recovery of historical sessions.
        if !self.workspace.is_absolute()
            || self.workspace.to_str().is_none()
            || self.updated_at_ms < self.created_at_ms
        {
            return Err(DomainError::InvalidRecord);
        }
        Ok(())
    }

    pub(crate) fn transition(&mut self, next: WorkSessionState) -> Result<(), DomainError> {
        if !self.state.can_transition_to(next) {
            return Err(DomainError::InvalidTransition {
                from: self.state,
                to: next,
            });
        }
        let timestamp = now_ms()?.max(self.updated_at_ms);
        self.state = next;
        self.updated_at_ms = timestamp;
        Ok(())
    }
}
