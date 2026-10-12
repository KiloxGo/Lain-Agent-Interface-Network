use crate::profiles::SecretValue;
use regex::Regex;
use std::sync::LazyLock;

#[derive(Clone)]
pub(crate) struct Redactor {
    known: Vec<zeroize::Zeroizing<String>>,
}

impl Redactor {
    pub fn new(known: Vec<String>) -> Self {
        Self {
            known: known.into_iter().map(zeroize::Zeroizing::new).collect(),
        }
    }
    pub fn for_runtime(&self, credential: Option<&SecretValue>) -> Self {
        let mut result = self.clone();
        result.known.extend(crate::profiles::known_secrets());
        if let Some(value) = credential {
            result.known.push(value.0.clone());
        }
        result
    }
    pub fn text(&self, text: &str) -> String {
        static SECRET: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?i)(sk-[a-z0-9_-]{12,}|bearer\s+\S+|(?:api[_-]?key|access[_-]?token|refresh[_-]?token|password|secret)\s*[:=]|-----BEGIN [A-Z ]*PRIVATE KEY-----)").unwrap()
        });
        if self
            .known
            .iter()
            .any(|secret| !secret.is_empty() && text.contains(secret.as_str()))
            || SECRET.is_match(text)
        {
            return "[sensitive output redacted]".to_owned();
        }
        // Completed records only: no secret can slip through split token fragments.
        text.to_owned()
    }
}

#[cfg(test)]
#[test]
fn credential_redaction_checks_complete_records_before_truncating() {
    let redactor = Redactor::new(vec!["known-owner-value".into()]);
    for text in [
        "known-owner-value",
        "Bearer abcdef",
        "api_key=abcdef",
        "-----BEGIN PRIVATE KEY-----",
        "sk-secret-split-across-chunks",
    ] {
        assert_eq!(redactor.text(text), "[sensitive output redacted]");
    }
    assert_eq!(
        redactor.text(&format!("{}known-owner-value", "a".repeat(40_000))),
        "[sensitive output redacted]"
    );
    assert_eq!(redactor.text("safe output"), "safe output");
    assert_eq!(redactor.text(&"a".repeat(40_000)).len(), 40_000);
}
