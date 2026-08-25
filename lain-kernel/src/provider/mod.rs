use std::{collections::BTreeMap, pin::Pin, sync::Arc, time::Duration};

use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

use crate::config::ProviderConfig;

pub mod deepseek;
pub mod mock;
mod sse;

pub type ModelEventStream =
    Pin<Box<dyn Stream<Item = Result<ModelEvent, ModelError>> + Send + 'static>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelMessage {
    pub role: Role,
    pub content: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ModelToolCall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ModelMessage {
    pub fn text(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }
    }

    pub fn assistant_tool_calls(
        content: impl Into<String>,
        tool_calls: Vec<ModelToolCall>,
    ) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            tool_calls,
            tool_call_id: None,
        }
    }

    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub strict: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    #[default]
    Auto,
    None,
    Required,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    #[default]
    Text,
    JsonObject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingMode {
    #[default]
    Disabled,
    Enabled,
}

#[derive(Debug, Clone)]
pub struct ModelRequest {
    pub request_id: String,
    pub model: String,
    pub messages: Vec<ModelMessage>,
    pub tools: Vec<ToolDefinition>,
    pub tool_choice: ToolChoice,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_output_tokens: Option<u32>,
    pub response_format: ResponseFormat,
    pub thinking_mode: ThinkingMode,
    pub timeout: Duration,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderCapabilities {
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub supports_streaming: bool,
    pub supports_json_output: bool,
    pub supports_tools: bool,
    pub supports_strict_tools: bool,
    pub supports_parallel_tools: bool,
    pub supports_thinking: bool,
    pub requires_reasoning_replay_for_tools: bool,
    pub returns_usage: bool,
    pub returns_cache_usage: bool,
    pub supports_idempotency: bool,
    pub supported_roles: Vec<Role>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ContentFilter,
    ToolCalls,
    InsufficientResource,
    Cancelled,
    Unknown,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_input_tokens: u64,
    pub cache_miss_input_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ModelEvent {
    ResponseStarted {
        request_id: String,
        provider_request_id: Option<String>,
        model: String,
    },
    TextDelta {
        text: String,
    },
    ReasoningDelta {
        text: String,
    },
    ToolCallStarted {
        index: usize,
        id: String,
        name: String,
    },
    ToolCallArgumentsDelta {
        index: usize,
        arguments: String,
    },
    ToolCallCompleted {
        index: usize,
    },
    Usage {
        usage: TokenUsage,
    },
    Finish {
        reason: FinishReason,
    },
    ProviderWarning {
        code: String,
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelErrorKind {
    Configuration,
    Authentication,
    InsufficientBalance,
    InvalidRequest,
    RateLimited,
    Timeout,
    Transport,
    Decode,
    EmptyResponse,
    ContentFiltered,
    Overloaded,
    Server,
    UnsupportedCapability,
    Cancelled,
    IncompleteStream,
}

#[derive(Debug, Clone, Error, Serialize)]
#[error("{kind:?}: {message}")]
pub struct ModelError {
    pub kind: ModelErrorKind,
    pub message: String,
    pub retry_after_ms: Option<u64>,
    pub provider_request_id: Option<String>,
    pub partial: bool,
}

impl ModelError {
    pub fn new(kind: ModelErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            retry_after_ms: None,
            provider_request_id: None,
            partial: false,
        }
    }

    pub fn code(&self) -> &'static str {
        match self.kind {
            ModelErrorKind::Configuration => "provider_configuration",
            ModelErrorKind::Authentication => "provider_authentication",
            ModelErrorKind::InsufficientBalance => "provider_insufficient_balance",
            ModelErrorKind::InvalidRequest => "provider_invalid_request",
            ModelErrorKind::RateLimited => "provider_rate_limited",
            ModelErrorKind::Timeout => "provider_timeout",
            ModelErrorKind::Transport => "provider_transport",
            ModelErrorKind::Decode => "provider_decode",
            ModelErrorKind::EmptyResponse => "provider_empty_response",
            ModelErrorKind::ContentFiltered => "provider_content_filtered",
            ModelErrorKind::Overloaded => "provider_overloaded",
            ModelErrorKind::Server => "provider_server",
            ModelErrorKind::UnsupportedCapability => "provider_unsupported_capability",
            ModelErrorKind::Cancelled => "provider_cancelled",
            ModelErrorKind::IncompleteStream => "provider_incomplete_stream",
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(
            self.kind,
            ModelErrorKind::RateLimited
                | ModelErrorKind::Timeout
                | ModelErrorKind::Transport
                | ModelErrorKind::Overloaded
                | ModelErrorKind::Server
        ) && !self.partial
    }
}

#[async_trait]
pub trait ModelProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn capabilities(&self) -> ProviderCapabilities;

    async fn stream(
        &self,
        request: ModelRequest,
        cancellation: CancellationToken,
    ) -> Result<ModelEventStream, ModelError>;

    async fn complete_json(
        &self,
        mut request: ModelRequest,
        cancellation: CancellationToken,
    ) -> Result<Value, ModelError> {
        request.response_format = ResponseFormat::JsonObject;
        let mut stream = self.stream(request, cancellation).await?;
        let mut content = String::new();
        while let Some(event) = stream.next().await {
            if let ModelEvent::TextDelta { text } = event? {
                content.push_str(&text);
            }
        }
        if content.trim().is_empty() {
            return Err(ModelError::new(
                ModelErrorKind::EmptyResponse,
                "JSON response contained no content",
            ));
        }
        serde_json::from_str(&content).map_err(|error| {
            ModelError::new(
                ModelErrorKind::Decode,
                format!("JSON response decoding failed: {error}"),
            )
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteKind {
    Chat,
    ComplexTask,
    Maintenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RouteDecision {
    pub model: String,
    pub thinking_mode: ThinkingMode,
    pub response_format: ResponseFormat,
    pub reason: &'static str,
}

#[derive(Debug, Clone)]
pub struct ModelRouter {
    flash_model: String,
    pro_model: String,
}

impl ModelRouter {
    pub fn new(flash_model: String, pro_model: String) -> Self {
        Self {
            flash_model,
            pro_model,
        }
    }

    pub fn route(&self, kind: RouteKind) -> RouteDecision {
        match kind {
            RouteKind::Chat => RouteDecision {
                model: self.flash_model.clone(),
                thinking_mode: ThinkingMode::Disabled,
                response_format: ResponseFormat::Text,
                reason: "low-latency conversational path",
            },
            RouteKind::ComplexTask => RouteDecision {
                model: self.pro_model.clone(),
                thinking_mode: ThinkingMode::Enabled,
                response_format: ResponseFormat::Text,
                reason: "explicit complex task",
            },
            RouteKind::Maintenance => RouteDecision {
                model: self.flash_model.clone(),
                thinking_mode: ThinkingMode::Disabled,
                response_format: ResponseFormat::JsonObject,
                reason: "structured maintenance request",
            },
        }
    }
}

pub type SharedProvider = Arc<dyn ModelProvider>;

#[derive(Debug, Clone, Serialize)]
pub struct ProviderDiagnostics {
    pub name: &'static str,
    pub ready: bool,
    pub default_model: String,
    pub pro_model: String,
    pub capabilities: ProviderCapabilities,
}

pub fn provider_diagnostics(config: &ProviderConfig, ready: bool) -> ProviderDiagnostics {
    ProviderDiagnostics {
        name: "deepseek",
        ready,
        default_model: config.model.clone(),
        pro_model: config.pro_model.clone(),
        capabilities: deepseek::DeepSeekProvider::capabilities_for_base_url(&config.base_url),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retryability_is_explicit_and_partial_streams_never_retry() {
        let retryable = ModelError::new(ModelErrorKind::RateLimited, "busy");
        assert!(retryable.is_retryable());

        let mut partial = retryable;
        partial.partial = true;
        assert!(!partial.is_retryable());
        assert!(!ModelError::new(ModelErrorKind::Authentication, "bad key").is_retryable());
    }

    #[test]
    fn router_uses_code_rules_without_an_extra_model_call() {
        let router = ModelRouter::new("flash".to_string(), "pro".to_string());
        assert_eq!(router.route(RouteKind::Chat).model, "flash");
        assert_eq!(
            router.route(RouteKind::ComplexTask).thinking_mode,
            ThinkingMode::Enabled
        );
        assert_eq!(
            router.route(RouteKind::Maintenance).response_format,
            ResponseFormat::JsonObject
        );
    }
}
