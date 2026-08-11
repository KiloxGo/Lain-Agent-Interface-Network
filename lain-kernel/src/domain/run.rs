use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Queued,
    Running,
    Cancelling,
    Completed,
    Failed,
    Cancelled,
}

impl RunState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Cancelling => "cancelling",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Queued, Self::Running | Self::Cancelled | Self::Failed)
                | (
                    Self::Running,
                    Self::Cancelling | Self::Completed | Self::Failed
                )
                | (
                    Self::Cancelling,
                    Self::Cancelled | Self::Completed | Self::Failed
                )
        )
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

impl TryFrom<&str> for RunState {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "cancelling" => Ok(Self::Cancelling),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RunState;

    #[test]
    fn only_declared_run_transitions_are_allowed() {
        assert!(RunState::Queued.can_transition_to(RunState::Running));
        assert!(RunState::Running.can_transition_to(RunState::Completed));
        assert!(RunState::Cancelling.can_transition_to(RunState::Cancelled));
        assert!(!RunState::Queued.can_transition_to(RunState::Completed));
        assert!(!RunState::Completed.can_transition_to(RunState::Running));
    }

    #[test]
    fn terminal_states_are_explicit() {
        assert!(RunState::Completed.is_terminal());
        assert!(RunState::Failed.is_terminal());
        assert!(RunState::Cancelled.is_terminal());
        assert!(!RunState::Running.is_terminal());
    }
}
