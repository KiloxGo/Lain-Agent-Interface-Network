use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::ProviderConfig;
use crate::memory;

pub struct LlmClient {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
    system_prompt: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    thinking: ThinkingConfig,
}

#[derive(Serialize)]
struct ThinkingConfig {
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChatMessage,
}

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("provider client initialization failed: {0}")]
    ClientBuild(#[source] reqwest::Error),
    #[error("provider request failed: {0}")]
    Request(#[source] reqwest::Error),
    #[error("provider returned HTTP {status}: {message}")]
    Http { status: StatusCode, message: String },
    #[error("provider response decoding failed: {0}")]
    Decode(#[source] reqwest::Error),
    #[error("provider response did not contain a choice")]
    MissingChoice,
    #[error("profile loading failed: {0}")]
    ProfileLoad(#[source] std::io::Error),
}

impl LlmError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ClientBuild(_) => "provider_client_init_failed",
            Self::Request(_) => "provider_request_failed",
            Self::Http { .. } => "provider_http_error",
            Self::Decode(_) => "provider_decode_failed",
            Self::MissingChoice => "provider_missing_choice",
            Self::ProfileLoad(_) => "profile_load_failed",
        }
    }
}

impl LlmClient {
    pub fn new(provider: &ProviderConfig, system_prompt: String) -> Result<Self, LlmError> {
        let api_key = provider
            .api_key()
            .ok_or_else(|| LlmError::Http {
                status: StatusCode::SERVICE_UNAVAILABLE,
                message: "provider API key is not configured".to_string(),
            })?
            .to_string();
        let client = Client::builder()
            .timeout(provider.request_timeout)
            .build()
            .map_err(LlmError::ClientBuild)?;

        Ok(Self {
            client,
            api_key,
            base_url: provider.base_url.clone(),
            model: provider.model.clone(),
            system_prompt: system_prompt.to_string(),
        })
    }

    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String, LlmError> {
        let profile = memory::load_profile()
            .await
            .map_err(LlmError::ProfileLoad)?
            .unwrap_or_else(String::new);

        let system_content = if profile.is_empty() {
            self.system_prompt.clone()
        } else {
            format!("{}\n\n##User Context\n{}", self.system_prompt, profile)
        };

        let mut all_messages = vec![ChatMessage {
            role: "system".to_string(),
            content: system_content,
        }];
        all_messages.extend(messages);

        let request = ChatRequest {
            model: self.model.clone(),
            messages: all_messages,
            stream: false,
            thinking: ThinkingConfig {
                kind: "disabled".to_string(),
            },
        };
        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(LlmError::Request)?;

        let status = response.status();
        if !status.is_success() {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "provider error body unavailable".to_string());
            return Err(LlmError::Http {
                status,
                message: message.chars().take(800).collect(),
            });
        }

        let response = response
            .json::<ChatResponse>()
            .await
            .map_err(LlmError::Decode)?;

        response
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .ok_or(LlmError::MissingChoice)
    }
}
