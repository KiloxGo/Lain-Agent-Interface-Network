use crate::provider::ModelMessage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenEstimate {
    pub tokens: u32,
    pub estimated: bool,
}

pub trait TokenEstimator: Send + Sync {
    fn provider(&self) -> &'static str;
    fn estimate_text(&self, text: &str) -> TokenEstimate;

    fn estimate_messages(&self, messages: &[ModelMessage]) -> TokenEstimate {
        let tokens = messages.iter().fold(0_u32, |total, message| {
            let tool_tokens = message.tool_calls.iter().fold(0_u32, |subtotal, call| {
                subtotal
                    .saturating_add(self.estimate_text(&call.name).tokens)
                    .saturating_add(self.estimate_text(&call.arguments).tokens)
                    .saturating_add(8)
            });
            total
                .saturating_add(8)
                .saturating_add(self.estimate_text(&message.content).tokens)
                .saturating_add(tool_tokens)
        });
        TokenEstimate {
            tokens,
            estimated: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ConservativeTokenEstimator;

impl TokenEstimator for ConservativeTokenEstimator {
    fn provider(&self) -> &'static str {
        "deepseek-conservative-v1"
    }

    fn estimate_text(&self, text: &str) -> TokenEstimate {
        let mut ascii_run = 0_u32;
        let mut tokens = 0_u32;
        for character in text.chars() {
            if character.is_ascii_alphanumeric() {
                ascii_run += 1;
            } else {
                tokens = tokens.saturating_add(ascii_run.div_ceil(3));
                ascii_run = 0;
                if !character.is_whitespace() {
                    tokens = tokens.saturating_add(1);
                }
            }
        }
        tokens = tokens.saturating_add(ascii_run.div_ceil(3));
        TokenEstimate {
            tokens: tokens.max(1),
            estimated: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimator_is_deterministic_and_conservative_for_cjk() {
        let estimator = ConservativeTokenEstimator;
        assert_eq!(estimator.estimate_text("这是测试").tokens, 4);
        assert_eq!(
            estimator.estimate_text("deterministic").tokens,
            estimator.estimate_text("deterministic").tokens
        );
    }
}
