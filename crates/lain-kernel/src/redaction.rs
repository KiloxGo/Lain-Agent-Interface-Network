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
            Regex::new(r"(?i)(lain-device-[a-f0-9]{12,}|sk-[a-z0-9_-]{12,}|bearer\s+\S+|(?:api[_-]?key|access[_-]?token|refresh[_-]?token|password|secret)\s*[:=]|-----BEGIN [A-Z ]*PRIVATE KEY-----)").unwrap()
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

    /// Keep any suffix that could still grow into a configured secret or one
    /// of the complete-record patterns. False positives delay preview only.
    fn ambiguous_suffix(&self, text: &str) -> usize {
        let mut hold = 0;
        for (offset, _) in text.char_indices() {
            let suffix = &text[offset..];
            // Unicode simple case folding of ASCII patterns also matches long
            // s and Kelvin sign in the complete-record regex.
            let lower: String = suffix
                .chars()
                .map(|c| match c {
                    'ſ' => 's',
                    'K' => 'k',
                    _ => c.to_ascii_lowercase(),
                })
                .collect();
            let known = self
                .known
                .iter()
                .any(|s| !s.is_empty() && s.starts_with(suffix));
            let mut pattern = false;
            for prefix in [
                "sk-",
                "lain-device-",
                "bearer",
                "apikey",
                "api_key",
                "api-key",
                "accesstoken",
                "access_token",
                "access-token",
                "refreshtoken",
                "refresh_token",
                "refresh-token",
                "password",
                "secret",
                "-----begin ",
            ] {
                if prefix.starts_with(&lower) {
                    pattern = true;
                    break;
                }
                if let Some(tail) = lower.strip_prefix(prefix) {
                    pattern |= match prefix {
                        "sk-" | "lain-device-" => tail
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b)),
                        "-----begin " => tail
                            .bytes()
                            .all(|b| b.is_ascii_alphabetic() || b" -".contains(&b)),
                        _ => tail.chars().all(char::is_whitespace),
                    };
                }
            }
            if known || pattern {
                hold = text.len() - offset;
                break;
            }
        }
        hold
    }
}

/// Preview-only sanitizer; durable complete records still use Redactor::text.
pub(crate) struct StreamingRedactor {
    redactor: Redactor,
    pending: zeroize::Zeroizing<String>,
    total: usize,
    blocked: bool,
}
impl StreamingRedactor {
    pub fn new(redactor: Redactor) -> Self {
        Self {
            redactor,
            pending: zeroize::Zeroizing::new(String::new()),
            total: 0,
            blocked: false,
        }
    }
    pub fn push(&mut self, text: &str) -> anyhow::Result<String> {
        anyhow::ensure!(!self.blocked, "preview withheld");
        self.total = self.total.saturating_add(text.len());
        if self.total > 16384 {
            self.blocked = true;
            anyhow::bail!("preview limit");
        }
        self.pending.push_str(text);
        if self.redactor.text(&self.pending) != self.pending.as_str() {
            self.blocked = true;
            self.pending.clear();
            anyhow::bail!("sensitive preview withheld");
        }
        let safe = self.pending.len() - self.redactor.ambiguous_suffix(&self.pending);
        Ok(self.pending.drain(..safe).collect())
    }
    /// Only after a complete authoritative response passed existing validation.
    pub fn finish(&mut self) -> String {
        if self.blocked {
            String::new()
        } else {
            std::mem::take(&mut *self.pending)
        }
    }
}

#[cfg(test)]
mod streaming_tests {
    use super::*;
    #[test]
    fn every_secret_boundary_and_character_stream_is_withheld() {
        let secret = "aB你-owner-credential-123";
        for value in [
            secret,
            "sk-abcdefghijklmno",
            "lain-device-abcdef0123456789",
            "Bearer abc",
            "API_KEY = abc",
            "access-token: abc",
            "refresh_token=abc",
            "password: abc",
            "secret = abc",
            "ſecret = abc",
            "API_KEY = abc",
            "ſk-abcdefghijklmno",
            "-----BEGIN RSA PRIVATE KEY-----",
        ] {
            let text = format!("safe prefix {value} trailing");
            for split in text.char_indices().map(|(n, _)| n).chain([text.len()]) {
                let mut stream = StreamingRedactor::new(Redactor::new(vec![secret.into()]));
                let mut visible = stream.push(&text[..split]).unwrap_or_default();
                visible.push_str(&stream.push(&text[split..]).unwrap_or_default());
                assert!(!visible.contains(value), "split {split}");
                assert!(!visible.contains("sk-"));
                assert!(!visible.to_ascii_lowercase().contains("api_key"));
            }
            let mut stream = StreamingRedactor::new(Redactor::new(vec![secret.into()]));
            let mut visible = String::new();
            for c in text.chars() {
                visible.push_str(&stream.push(&c.to_string()).unwrap_or_default());
            }
            assert!(!visible.contains(value));
        }
    }
    #[test]
    fn harmless_ambiguous_suffix_waits_until_validation_and_memory_is_bounded() {
        let mut s = StreamingRedactor::new(Redactor::new(vec!["abcdef".into()]));
        assert_eq!(s.push("你好 a").unwrap(), "你好 ");
        assert_eq!(s.push("b").unwrap(), "");
        assert_eq!(s.push("X\n").unwrap(), "abX\n");
        assert_eq!(s.push("a").unwrap(), "");
        assert_eq!(s.finish(), "a");
        assert!(s.push(&" ".repeat(16385)).is_err());
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
