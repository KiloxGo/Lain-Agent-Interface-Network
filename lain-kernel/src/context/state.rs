use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StateValidationError {
    #[error("{field} must be between 0 and 1")]
    UnitRange { field: &'static str },
    #[error("self state can contain at most five active intent IDs")]
    TooManyIntents,
    #[error("state ID references cannot be empty")]
    EmptyReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Affect {
    Calm,
    Curious,
    Distant,
    Warm,
    Tense,
    Tired,
}

impl Affect {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Calm => "calm",
            Self::Curious => "curious",
            Self::Distant => "distant",
            Self::Warm => "warm",
            Self::Tense => "tense",
            Self::Tired => "tired",
        }
    }

    pub const fn voice_directive(self) -> &'static str {
        match self {
            Self::Calm => "Be quiet, direct, and steady.",
            Self::Curious => "Be observant and precise; surface the next useful clue.",
            Self::Distant => "Be restrained and factual without becoming cold or evasive.",
            Self::Warm => "Be gently present while keeping the useful answer first.",
            Self::Tense => "Be concise and risk-aware; do not dramatize urgency.",
            Self::Tired => "Use short, clear sentences without claiming human fatigue.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfStateSignal {
    Conversation,
    Task,
    Maintenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelfState {
    pub affect: Affect,
    pub activation: f32,
    pub closeness_expression: f32,
    pub uncertainty: f32,
    pub active_intents: Vec<String>,
    pub expires_at: DateTime<Utc>,
}

impl SelfState {
    pub fn reduce(signal: SelfStateSignal, now: DateTime<Utc>) -> Self {
        let (affect, activation, closeness_expression, uncertainty, active_intents) = match signal {
            SelfStateSignal::Conversation => (Affect::Warm, 0.35, 0.35, 0.05, Vec::new()),
            SelfStateSignal::Task => (
                Affect::Curious,
                0.7,
                0.15,
                0.15,
                vec!["complete_current_task".to_string()],
            ),
            SelfStateSignal::Maintenance => (
                Affect::Calm,
                0.45,
                0.1,
                0.05,
                vec!["maintain_system".to_string()],
            ),
        };
        Self {
            affect,
            activation,
            closeness_expression,
            uncertainty,
            active_intents,
            expires_at: now + Duration::minutes(10),
        }
    }

    pub fn validate(&self) -> Result<(), StateValidationError> {
        validate_unit("activation", self.activation)?;
        validate_unit("closeness_expression", self.closeness_expression)?;
        validate_unit("uncertainty", self.uncertainty)?;
        if self.active_intents.len() > 5 {
            return Err(StateValidationError::TooManyIntents);
        }
        if self
            .active_intents
            .iter()
            .any(|intent| intent.trim().is_empty())
        {
            return Err(StateValidationError::EmptyReference);
        }
        Ok(())
    }

    pub fn is_active_at(&self, now: DateTime<Utc>) -> bool {
        self.expires_at > now
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationshipState {
    pub familiarity: f32,
    pub trust: f32,
    pub warmth: f32,
    pub tension: f32,
    pub reciprocity: f32,
    pub continuity: f32,
    pub shared_symbols: Vec<String>,
    pub open_loops: Vec<String>,
}

impl Default for RelationshipState {
    fn default() -> Self {
        Self {
            familiarity: 0.0,
            trust: 0.0,
            warmth: 0.0,
            tension: 0.0,
            reciprocity: 0.0,
            continuity: 0.0,
            shared_symbols: Vec::new(),
            open_loops: Vec::new(),
        }
    }
}

impl RelationshipState {
    pub fn validate(&self) -> Result<(), StateValidationError> {
        validate_unit("familiarity", self.familiarity)?;
        validate_unit("trust", self.trust)?;
        validate_unit("warmth", self.warmth)?;
        validate_unit("tension", self.tension)?;
        validate_unit("reciprocity", self.reciprocity)?;
        validate_unit("continuity", self.continuity)?;
        if self
            .shared_symbols
            .iter()
            .chain(self.open_loops.iter())
            .any(|reference| reference.trim().is_empty())
        {
            return Err(StateValidationError::EmptyReference);
        }
        Ok(())
    }
}

fn validate_unit(field: &'static str, value: f32) -> Result<(), StateValidationError> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(StateValidationError::UnitRange { field })
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    #[test]
    fn self_state_enforces_range_intent_limit_and_ttl() {
        let now = Utc::now();
        let state = SelfState {
            affect: Affect::Curious,
            activation: 0.5,
            closeness_expression: 0.25,
            uncertainty: 0.1,
            active_intents: vec!["intent-1".to_string()],
            expires_at: now + Duration::minutes(5),
        };
        assert!(state.validate().is_ok());
        assert!(state.is_active_at(now));
        assert!(!state.is_active_at(now + Duration::minutes(6)));
    }

    #[test]
    fn reducer_maps_activity_to_bounded_voice_state() {
        let now = Utc::now();
        let task = SelfState::reduce(SelfStateSignal::Task, now);
        assert_eq!(task.affect, Affect::Curious);
        assert_eq!(task.active_intents, ["complete_current_task"]);
        assert!(task.is_active_at(now + Duration::minutes(9)));
        assert!(!task.is_active_at(now + Duration::minutes(10)));
        assert!(task.validate().is_ok());
        assert!(task.affect.voice_directive().contains("useful clue"));
    }

    #[test]
    fn relationship_state_rejects_out_of_range_values() {
        let state = RelationshipState {
            warmth: 1.5,
            ..RelationshipState::default()
        };
        assert_eq!(
            state.validate(),
            Err(StateValidationError::UnitRange { field: "warmth" })
        );
    }
}
