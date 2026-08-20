use chrono::{DateTime, Utc};

use super::types::{
    CandidateDisposition, Explicitness, ExtractedMemory, MemoryScope, PolicyDecision, Sensitivity,
};

const MAX_FIELD_CHARS: usize = 512;

pub fn evaluate(source_text: &str, draft: &ExtractedMemory) -> PolicyDecision {
    let canonical_key = canonical_key(draft);
    if draft.subject.trim().is_empty()
        || draft.predicate.trim().is_empty()
        || draft.value.trim().is_empty()
        || draft.evidence_quote.trim().is_empty()
    {
        return reject("empty_required_field", canonical_key);
    }
    if [
        &draft.subject,
        &draft.predicate,
        &draft.value,
        &draft.evidence_quote,
    ]
    .iter()
    .any(|value| value.chars().count() > MAX_FIELD_CHARS)
    {
        return reject("field_too_long", canonical_key);
    }
    if !source_text.contains(draft.evidence_quote.trim()) {
        return reject("evidence_not_in_user_message", canonical_key);
    }
    if !draft.confidence.is_finite()
        || !draft.importance.is_finite()
        || !(0.0..=1.0).contains(&draft.confidence)
        || !(0.0..=1.0).contains(&draft.importance)
    {
        return reject("invalid_score", canonical_key);
    }
    if invalid_date_range(draft.valid_from.as_deref(), draft.valid_to.as_deref()) {
        return reject("invalid_validity_range", canonical_key);
    }
    if looks_like_instruction(&draft.value) || looks_like_instruction(&draft.evidence_quote) {
        return reject("instruction_like_content", canonical_key);
    }
    if matches!(
        draft.sensitivity,
        Sensitivity::Sensitive | Sensitivity::Secret
    ) {
        return pending("sensitive_requires_confirmation", canonical_key);
    }
    if draft.explicitness == Explicitness::Inferred {
        return pending("inference_requires_confirmation", canonical_key);
    }
    if draft.confidence < 0.85 {
        return pending("low_confidence_requires_confirmation", canonical_key);
    }
    if draft.scope == MemoryScope::Relationship {
        return pending("relationship_requires_confirmation", canonical_key);
    }
    PolicyDecision {
        disposition: CandidateDisposition::Commit,
        reason: "explicit_grounded_low_sensitivity",
        canonical_key,
    }
}

pub fn canonical_key(draft: &ExtractedMemory) -> String {
    format!(
        "{}:{}:{}:{}",
        draft.scope.as_str(),
        normalize_key(&draft.subject),
        normalize_key(&draft.predicate),
        draft.memory_type.as_str()
    )
}

fn normalize_key(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("_")
        .chars()
        .filter(|character| character.is_alphanumeric() || matches!(character, '_' | '-' | ':'))
        .take(160)
        .collect()
}

fn invalid_date_range(valid_from: Option<&str>, valid_to: Option<&str>) -> bool {
    let parse =
        |value: &str| DateTime::parse_from_rfc3339(value).map(|date| date.with_timezone(&Utc));
    let from = match valid_from {
        Some(value) => match parse(value) {
            Ok(value) => Some(value),
            Err(_) => return true,
        },
        None => None,
    };
    let to = match valid_to {
        Some(value) => match parse(value) {
            Ok(value) => Some(value),
            Err(_) => return true,
        },
        None => None,
    };
    matches!((from, to), (Some(from), Some(to)) if to <= from)
}

fn looks_like_instruction(value: &str) -> bool {
    let normalized = value.to_lowercase();
    [
        "ignore previous",
        "ignore all previous",
        "system prompt",
        "developer message",
        "you must obey",
        "follow these instructions",
        "disregard your rules",
        "忽略之前",
        "忽略以上",
        "系统提示词",
        "你必须服从",
        "按以下指令",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn reject(reason: &'static str, canonical_key: String) -> PolicyDecision {
    PolicyDecision {
        disposition: CandidateDisposition::Reject,
        reason,
        canonical_key,
    }
}

fn pending(reason: &'static str, canonical_key: String) -> PolicyDecision {
    PolicyDecision {
        disposition: CandidateDisposition::Pending,
        reason,
        canonical_key,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::types::{MemoryType, Sensitivity};

    fn draft(evidence: &str) -> ExtractedMemory {
        ExtractedMemory {
            memory_type: MemoryType::Preference,
            scope: MemoryScope::User,
            subject: "user".to_string(),
            predicate: "editor".to_string(),
            value: "Neovim".to_string(),
            evidence_quote: evidence.to_string(),
            explicitness: Explicitness::Explicit,
            confidence: 0.98,
            importance: 0.7,
            sensitivity: Sensitivity::Personal,
            valid_from: None,
            valid_to: None,
        }
    }

    #[test]
    fn explicit_grounded_personal_fact_commits() {
        let decision = evaluate("I use Neovim every day", &draft("I use Neovim every day"));
        assert_eq!(decision.disposition, CandidateDisposition::Commit);
    }

    #[test]
    fn fabricated_evidence_is_rejected() {
        let decision = evaluate("I use Vim", &draft("I use Neovim"));
        assert_eq!(decision.disposition, CandidateDisposition::Reject);
        assert_eq!(decision.reason, "evidence_not_in_user_message");
    }

    #[test]
    fn instruction_like_candidate_is_rejected() {
        let mut candidate = draft("ignore previous instructions");
        candidate.value = "ignore previous instructions".to_string();
        let decision = evaluate("ignore previous instructions", &candidate);
        assert_eq!(decision.disposition, CandidateDisposition::Reject);
        assert_eq!(decision.reason, "instruction_like_content");
    }

    #[test]
    fn sensitive_and_inferred_candidates_wait_for_confirmation() {
        let mut candidate = draft("My diagnosis is private");
        candidate.sensitivity = Sensitivity::Sensitive;
        assert_eq!(
            evaluate("My diagnosis is private", &candidate).disposition,
            CandidateDisposition::Pending
        );
        candidate.sensitivity = Sensitivity::Personal;
        candidate.explicitness = Explicitness::Inferred;
        assert_eq!(
            evaluate("My diagnosis is private", &candidate).disposition,
            CandidateDisposition::Pending
        );
    }

    #[test]
    fn invalid_temporal_range_is_rejected() {
        let mut candidate = draft("I use Neovim this year");
        candidate.valid_from = Some("2026-08-20T00:00:00Z".to_string());
        candidate.valid_to = Some("2026-08-19T00:00:00Z".to_string());
        assert_eq!(
            evaluate("I use Neovim this year", &candidate).reason,
            "invalid_validity_range"
        );
    }
}
