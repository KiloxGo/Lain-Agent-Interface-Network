use std::{
    collections::{BTreeMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    time::Duration,
};

use async_stream::stream;
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::{Client, Response, StatusCode, header::RETRY_AFTER};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::config::ProviderConfig;

use super::{
    FinishReason, ModelError, ModelErrorKind, ModelEvent, ModelEventStream, ModelProvider,
    ModelRequest, ProviderCapabilities, ResponseFormat, Role, ThinkingMode, TokenUsage, ToolChoice,
    sse::SseDecoder,
};

const MAX_ATTEMPTS: usize = 2;
const MAX_ERROR_BODY_CHARS: usize = 800;

#[derive(Debug)]
pub struct DeepSeekProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl DeepSeekProvider {
    pub fn new(config: &ProviderConfig) -> Result<Self, ModelError> {
        let api_key = config.api_key().ok_or_else(|| {
            ModelError::new(
                ModelErrorKind::Configuration,
                "provider API key is not configured",
            )
        })?;
        let client = Client::builder()
            .timeout(config.request_timeout)
            .build()
            .map_err(|error| {
                ModelError::new(
                    ModelErrorKind::Configuration,
                    format!("provider HTTP client initialization failed: {error}"),
                )
            })?;
        Ok(Self {
            client,
            api_key: api_key.to_string(),
            base_url: config.base_url.clone(),
        })
    }

    pub fn capabilities_for_base_url(base_url: &str) -> ProviderCapabilities {
        ProviderCapabilities {
            context_window: 1_000_000,
            max_output_tokens: 384_000,
            supports_streaming: true,
            supports_json_output: true,
            supports_tools: true,
            supports_strict_tools: base_url.trim_end_matches('/').ends_with("/beta"),
            supports_parallel_tools: true,
            supports_thinking: true,
            requires_reasoning_replay_for_tools: true,
            returns_usage: true,
            returns_cache_usage: true,
            supports_idempotency: false,
            supported_roles: vec![Role::System, Role::User, Role::Assistant, Role::Tool],
        }
    }

    async fn send_with_retry(
        &self,
        request: &ModelRequest,
        payload: &DeepSeekRequest,
        cancellation: &CancellationToken,
    ) -> Result<Response, ModelError> {
        for attempt in 0..MAX_ATTEMPTS {
            let send = self
                .client
                .post(format!("{}/chat/completions", self.base_url))
                .bearer_auth(&self.api_key)
                .header("x-client-request-id", &request.request_id)
                .json(payload)
                .send();

            let response = tokio::select! {
                () = cancellation.cancelled() => {
                    return Err(ModelError::new(ModelErrorKind::Cancelled, "provider request cancelled"));
                }
                result = tokio::time::timeout(request.timeout, send) => {
                    match result {
                        Err(_) => Err(ModelError::new(ModelErrorKind::Timeout, "provider request timed out")),
                        Ok(Err(error)) if error.is_timeout() => {
                            Err(ModelError::new(ModelErrorKind::Timeout, "provider request timed out"))
                        }
                        Ok(Err(error)) => Err(ModelError::new(
                            ModelErrorKind::Transport,
                            format!("provider transport failed: {error}"),
                        )),
                        Ok(Ok(response)) => Ok(response),
                    }
                }
            };

            let error = match response {
                Ok(response) if response.status().is_success() => return Ok(response),
                Ok(response) => error_from_response(response).await,
                Err(error) => error,
            };
            if attempt + 1 == MAX_ATTEMPTS || !error.is_retryable() {
                return Err(error);
            }

            let delay = retry_delay(&request.request_id, attempt, error.retry_after_ms);
            tracing::warn!(
                request_id = request.request_id,
                attempt = attempt + 1,
                code = error.code(),
                delay_ms = delay.as_millis(),
                "retrying provider request before any output was emitted"
            );
            tokio::select! {
                () = cancellation.cancelled() => {
                    return Err(ModelError::new(ModelErrorKind::Cancelled, "provider request cancelled"));
                }
                () = tokio::time::sleep(delay) => {}
            }
        }

        Err(ModelError::new(
            ModelErrorKind::Server,
            "provider retry loop exhausted",
        ))
    }
}

#[async_trait]
impl ModelProvider for DeepSeekProvider {
    fn name(&self) -> &'static str {
        "deepseek"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        Self::capabilities_for_base_url(&self.base_url)
    }

    async fn stream(
        &self,
        request: ModelRequest,
        cancellation: CancellationToken,
    ) -> Result<ModelEventStream, ModelError> {
        if request.model.trim().is_empty() {
            return Err(ModelError::new(
                ModelErrorKind::Configuration,
                "model route selected an empty model name",
            ));
        }
        if request.tools.iter().any(|tool| tool.strict)
            && !self.capabilities().supports_strict_tools
        {
            return Err(ModelError::new(
                ModelErrorKind::UnsupportedCapability,
                "strict tools require a DeepSeek beta base URL",
            ));
        }

        let payload = DeepSeekRequest::from_model_request(&request);
        let response = self
            .send_with_retry(&request, &payload, &cancellation)
            .await?;
        let provider_request_id = response
            .headers()
            .get("x-request-id")
            .or_else(|| response.headers().get("x-ds-request-id"))
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let mut bytes = response.bytes_stream();
        let request_id = request.request_id.clone();
        let model = request.model.clone();
        let timeout = request.timeout;

        Ok(Box::pin(stream! {
            yield Ok(ModelEvent::ResponseStarted {
                request_id,
                provider_request_id: provider_request_id.clone(),
                model,
            });

            let mut decoder = SseDecoder::default();
            let mut tool_calls = BTreeMap::<usize, ToolCallState>::new();
            let mut saw_content = false;
            let mut saw_finish = false;
            let mut saw_done = false;
            let mut stream_failed = false;
            let deadline = tokio::time::sleep(timeout);
            tokio::pin!(deadline);

            'response: loop {
                let next = tokio::select! {
                    () = cancellation.cancelled() => {
                        let mut error = ModelError::new(ModelErrorKind::Cancelled, "provider stream cancelled");
                        error.partial = saw_content;
                        stream_failed = true;
                        yield Err(error);
                        break 'response;
                    }
                    () = &mut deadline => {
                        let mut error = ModelError::new(ModelErrorKind::Timeout, "provider stream timed out");
                        error.partial = saw_content;
                        stream_failed = true;
                        yield Err(error);
                        break 'response;
                    }
                    chunk = bytes.next() => chunk,
                };

                let Some(chunk) = next else {
                    if !saw_finish && !saw_done && !decoder.has_pending_data() {
                        let kind = if saw_content {
                            ModelErrorKind::IncompleteStream
                        } else {
                            ModelErrorKind::EmptyResponse
                        };
                        let mut error = ModelError::new(kind, "provider stream ended before completion");
                        error.partial = saw_content;
                        error.provider_request_id = provider_request_id.clone();
                        stream_failed = true;
                        yield Err(error);
                    }
                    break;
                };

                let chunk = match chunk {
                    Ok(chunk) => chunk,
                    Err(error) => {
                        let mut mapped = ModelError::new(
                            if error.is_timeout() { ModelErrorKind::Timeout } else { ModelErrorKind::Transport },
                            format!("provider stream transport failed: {error}"),
                        );
                        mapped.partial = saw_content;
                        mapped.provider_request_id = provider_request_id.clone();
                        stream_failed = true;
                        yield Err(mapped);
                        break;
                    }
                };

                let data_events = match decoder.push(&chunk) {
                    Ok(events) => events,
                    Err(mut error) => {
                        error.partial = saw_content;
                        error.provider_request_id = provider_request_id.clone();
                        stream_failed = true;
                        yield Err(error);
                        break;
                    }
                };

                for data in data_events {
                    if data == "[DONE]" {
                        saw_done = true;
                        if !saw_finish && saw_content {
                            saw_finish = true;
                            yield Ok(ModelEvent::Finish { reason: FinishReason::Unknown });
                        }
                        continue;
                    }

                    let chunk: DeepSeekStreamChunk = match serde_json::from_str(&data) {
                        Ok(chunk) => chunk,
                        Err(error) => {
                            let mut mapped = ModelError::new(
                                ModelErrorKind::Decode,
                                format!("provider SSE JSON decoding failed: {error}"),
                            );
                            mapped.partial = saw_content;
                            mapped.provider_request_id = provider_request_id.clone();
                            stream_failed = true;
                            yield Err(mapped);
                            break 'response;
                        }
                    };

                    for choice in chunk.choices {
                        if let Some(text) = choice.delta.content
                            && !text.is_empty()
                        {
                            saw_content = true;
                            yield Ok(ModelEvent::TextDelta { text });
                        }
                        if let Some(text) = choice.delta.reasoning_content
                            && !text.is_empty()
                        {
                            saw_content = true;
                            yield Ok(ModelEvent::ReasoningDelta { text });
                        }

                        for tool_call in choice.delta.tool_calls {
                            let state = tool_calls.entry(tool_call.index).or_default();
                            if let Some(id) = tool_call.id {
                                state.id = id;
                            }
                            if let Some(function) = tool_call.function {
                                if let Some(name) = function.name {
                                    state.name.push_str(&name);
                                }
                                if !state.started && (!state.id.is_empty() || !state.name.is_empty()) {
                                    state.started = true;
                                    yield Ok(ModelEvent::ToolCallStarted {
                                        index: tool_call.index,
                                        id: state.id.clone(),
                                        name: state.name.clone(),
                                    });
                                }
                                if let Some(arguments) = function.arguments
                                    && !arguments.is_empty()
                                {
                                    saw_content = true;
                                    yield Ok(ModelEvent::ToolCallArgumentsDelta {
                                        index: tool_call.index,
                                        arguments,
                                    });
                                }
                            }
                        }

                        if let Some(reason) = choice.finish_reason {
                            let reason = map_finish_reason(&reason);
                            if reason == FinishReason::ToolCalls {
                                for (index, state) in &mut tool_calls {
                                    if !state.completed {
                                        state.completed = true;
                                        yield Ok(ModelEvent::ToolCallCompleted { index: *index });
                                    }
                                }
                            }
                            saw_finish = true;
                            yield Ok(ModelEvent::Finish { reason });
                        }
                    }

                    if let Some(usage) = chunk.usage {
                        yield Ok(ModelEvent::Usage { usage: usage.into() });
                    }
                }
            }

            if !stream_failed && decoder.has_pending_data() && !saw_finish && !saw_done {
                let mut error = ModelError::new(
                    ModelErrorKind::IncompleteStream,
                    "provider stream ended inside an SSE event",
                );
                error.partial = saw_content;
                error.provider_request_id = provider_request_id;
                yield Err(error);
            } else if !stream_failed && !saw_content && saw_done && !saw_finish {
                let mut error = ModelError::new(
                    ModelErrorKind::EmptyResponse,
                    "provider returned no choices or content",
                );
                error.provider_request_id = provider_request_id;
                yield Err(error);
            }
        }))
    }
}

#[derive(Debug, Serialize)]
struct DeepSeekRequest {
    model: String,
    messages: Vec<DeepSeekMessage>,
    stream: bool,
    stream_options: StreamOptions,
    thinking: ThinkingConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ApiResponseFormat>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ApiTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

impl DeepSeekRequest {
    fn from_model_request(request: &ModelRequest) -> Self {
        Self {
            model: request.model.clone(),
            messages: request
                .messages
                .iter()
                .map(|message| DeepSeekMessage {
                    role: match message.role {
                        Role::System => "system",
                        Role::User => "user",
                        Role::Assistant => "assistant",
                        Role::Tool => "tool",
                    },
                    content: message.content.clone(),
                })
                .collect(),
            stream: true,
            stream_options: StreamOptions {
                include_usage: true,
            },
            thinking: ThinkingConfig {
                kind: match request.thinking_mode {
                    ThinkingMode::Disabled => "disabled",
                    ThinkingMode::Enabled => "enabled",
                },
            },
            response_format: (request.response_format == ResponseFormat::JsonObject).then_some(
                ApiResponseFormat {
                    kind: "json_object",
                },
            ),
            tools: request
                .tools
                .iter()
                .map(|tool| ApiTool {
                    kind: "function",
                    function: ApiFunction {
                        name: tool.name.clone(),
                        description: tool.description.clone(),
                        parameters: tool.parameters.clone(),
                        strict: tool.strict,
                    },
                })
                .collect(),
            tool_choice: (!request.tools.is_empty()).then_some(match request.tool_choice {
                ToolChoice::Auto => "auto",
                ToolChoice::None => "none",
                ToolChoice::Required => "required",
            }),
            temperature: request.temperature,
            top_p: request.top_p,
            max_tokens: request.max_output_tokens,
        }
    }
}

#[derive(Debug, Serialize)]
struct DeepSeekMessage {
    role: &'static str,
    content: String,
}

#[derive(Debug, Serialize)]
struct StreamOptions {
    include_usage: bool,
}

#[derive(Debug, Serialize)]
struct ThinkingConfig {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Debug, Serialize)]
struct ApiResponseFormat {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Debug, Serialize)]
struct ApiTool {
    #[serde(rename = "type")]
    kind: &'static str,
    function: ApiFunction,
}

#[derive(Debug, Serialize)]
struct ApiFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
    strict: bool,
}

#[derive(Debug, Deserialize)]
struct DeepSeekStreamChunk {
    #[serde(default)]
    choices: Vec<StreamChoice>,
    usage: Option<ApiUsage>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    #[serde(default)]
    delta: StreamDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct StreamDelta {
    content: Option<String>,
    reasoning_content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ToolCallDelta>,
}

#[derive(Debug, Deserialize)]
struct ToolCallDelta {
    index: usize,
    id: Option<String>,
    function: Option<FunctionDelta>,
}

#[derive(Debug, Deserialize)]
struct FunctionDelta {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Debug, Default)]
struct ToolCallState {
    id: String,
    name: String,
    started: bool,
    completed: bool,
}

#[derive(Debug, Deserialize)]
struct ApiUsage {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
    #[serde(default)]
    prompt_cache_hit_tokens: u64,
    #[serde(default)]
    prompt_cache_miss_tokens: u64,
}

impl From<ApiUsage> for TokenUsage {
    fn from(usage: ApiUsage) -> Self {
        Self {
            input_tokens: usage.prompt_tokens,
            output_tokens: usage.completion_tokens,
            cached_input_tokens: usage.prompt_cache_hit_tokens,
            cache_miss_input_tokens: usage.prompt_cache_miss_tokens,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ErrorEnvelope {
    error: Option<ErrorDetail>,
}

#[derive(Debug, Deserialize)]
struct ErrorDetail {
    message: Option<String>,
}

async fn error_from_response(response: Response) -> ModelError {
    let status = response.status();
    let provider_request_id = response
        .headers()
        .get("x-request-id")
        .or_else(|| response.headers().get("x-ds-request-id"))
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let retry_after_ms = response
        .headers()
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(|seconds| seconds.saturating_mul(1000));
    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "provider error body unavailable".to_string());
    let message = serde_json::from_str::<ErrorEnvelope>(&body)
        .ok()
        .and_then(|envelope| envelope.error)
        .and_then(|detail| detail.message)
        .unwrap_or(body)
        .chars()
        .take(MAX_ERROR_BODY_CHARS)
        .collect::<String>();
    let kind = match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ModelErrorKind::Authentication,
        StatusCode::PAYMENT_REQUIRED => ModelErrorKind::InsufficientBalance,
        StatusCode::UNPROCESSABLE_ENTITY | StatusCode::BAD_REQUEST => {
            ModelErrorKind::InvalidRequest
        }
        StatusCode::TOO_MANY_REQUESTS => ModelErrorKind::RateLimited,
        StatusCode::SERVICE_UNAVAILABLE => ModelErrorKind::Overloaded,
        status if status.is_server_error() => ModelErrorKind::Server,
        _ => ModelErrorKind::InvalidRequest,
    };
    let mut error = ModelError::new(kind, format!("provider returned HTTP {status}: {message}"));
    error.retry_after_ms = retry_after_ms;
    error.provider_request_id = provider_request_id;
    error
}

fn retry_delay(request_id: &str, attempt: usize, retry_after_ms: Option<u64>) -> Duration {
    if let Some(delay) = retry_after_ms {
        return Duration::from_millis(delay.min(10_000));
    }
    let mut hasher = DefaultHasher::new();
    request_id.hash(&mut hasher);
    attempt.hash(&mut hasher);
    let jitter = hasher.finish() % 100;
    Duration::from_millis(150 * (1_u64 << attempt) + jitter)
}

fn map_finish_reason(reason: &str) -> FinishReason {
    match reason {
        "stop" => FinishReason::Stop,
        "length" => FinishReason::Length,
        "content_filter" => FinishReason::ContentFilter,
        "tool_calls" => FinishReason::ToolCalls,
        "insufficient_resource" => FinishReason::InsufficientResource,
        "cancelled" => FinishReason::Cancelled,
        _ => FinishReason::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{BTreeMap, VecDeque},
        convert::Infallible,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use axum::{
        Json, Router,
        body::{Body, Bytes},
        extract::State,
        http::header::HeaderName,
        response::Response as AxumResponse,
        routing::post,
    };
    use futures_util::StreamExt;
    use serde_json::{Value, json};
    use tokio::{sync::Mutex, task::JoinHandle};

    use super::*;
    use crate::provider::{ModelMessage, ResponseFormat, ToolDefinition};

    #[derive(Clone)]
    struct MockResponse {
        status: StatusCode,
        headers: Vec<(&'static str, &'static str)>,
        chunks: Vec<(Duration, Vec<u8>)>,
    }

    impl MockResponse {
        fn sse(payload: impl Into<Vec<u8>>) -> Self {
            Self {
                status: StatusCode::OK,
                headers: vec![("content-type", "text/event-stream")],
                chunks: vec![(Duration::ZERO, payload.into())],
            }
        }

        fn status(status: StatusCode, body: &str) -> Self {
            Self {
                status,
                headers: vec![("content-type", "application/json")],
                chunks: vec![(Duration::ZERO, body.as_bytes().to_vec())],
            }
        }
    }

    #[derive(Clone)]
    struct MockState {
        responses: Arc<Mutex<VecDeque<MockResponse>>>,
        requests: Arc<Mutex<Vec<Value>>>,
        request_count: Arc<AtomicUsize>,
    }

    struct MockServer {
        base_url: String,
        state: MockState,
        task: JoinHandle<()>,
    }

    impl Drop for MockServer {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    async fn mock_handler(
        State(state): State<MockState>,
        Json(request): Json<Value>,
    ) -> AxumResponse {
        state.request_count.fetch_add(1, Ordering::SeqCst);
        state.requests.lock().await.push(request);
        let response = state.responses.lock().await.pop_front().unwrap_or_else(|| {
            MockResponse::status(StatusCode::INTERNAL_SERVER_ERROR, "missing mock response")
        });
        let chunks = response.chunks;
        let body = Body::from_stream(async_stream::stream! {
            for (delay, chunk) in chunks {
                if !delay.is_zero() {
                    tokio::time::sleep(delay).await;
                }
                yield Ok::<Bytes, Infallible>(Bytes::from(chunk));
            }
        });
        let mut builder = AxumResponse::builder().status(response.status);
        for (name, value) in response.headers {
            builder = builder.header(HeaderName::from_static(name), value);
        }
        builder.body(body).expect("valid mock response")
    }

    async fn start_server(responses: Vec<MockResponse>) -> MockServer {
        let state = MockState {
            responses: Arc::new(Mutex::new(responses.into_iter().collect())),
            requests: Arc::new(Mutex::new(Vec::new())),
            request_count: Arc::new(AtomicUsize::new(0)),
        };
        let app = Router::new()
            .route("/chat/completions", post(mock_handler))
            .with_state(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("mock listener");
        let address = listener.local_addr().expect("mock address");
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("mock server");
        });
        MockServer {
            base_url: format!("http://{address}"),
            state,
            task,
        }
    }

    fn request(timeout: Duration) -> ModelRequest {
        ModelRequest {
            request_id: "contract-request".to_string(),
            model: "deepseek-v4-flash".to_string(),
            messages: vec![ModelMessage {
                role: Role::User,
                content: "synthetic request".to_string(),
            }],
            tools: Vec::new(),
            tool_choice: ToolChoice::None,
            temperature: None,
            top_p: None,
            max_output_tokens: Some(128),
            response_format: ResponseFormat::Text,
            thinking_mode: ThinkingMode::Disabled,
            timeout,
            metadata: BTreeMap::new(),
        }
    }

    fn provider(server: &MockServer) -> DeepSeekProvider {
        DeepSeekProvider::new(&ProviderConfig::for_test_endpoint(
            server.base_url.clone(),
            Duration::from_secs(5),
        ))
        .expect("provider")
    }

    #[test]
    fn capabilities_match_stable_and_beta_endpoint_boundaries() {
        let stable = DeepSeekProvider::capabilities_for_base_url("https://api.deepseek.com");
        assert_eq!(stable.context_window, 1_000_000);
        assert_eq!(stable.max_output_tokens, 384_000);
        assert!(!stable.supports_strict_tools);
        assert!(
            DeepSeekProvider::capabilities_for_base_url("https://api.deepseek.com/beta")
                .supports_strict_tools
        );
    }

    async fn collect(
        provider: &DeepSeekProvider,
        request: ModelRequest,
    ) -> Vec<Result<ModelEvent, ModelError>> {
        provider
            .stream(request, CancellationToken::new())
            .await
            .expect("stream starts")
            .collect()
            .await
    }

    #[tokio::test]
    async fn streams_unicode_finish_usage_and_provider_request_id_across_byte_chunks() {
        let payload = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"你\"},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"好\"},\"finish_reason\":\"stop\"}]}\n\n",
            "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":12,\"completion_tokens\":2,\"prompt_cache_hit_tokens\":8,\"prompt_cache_miss_tokens\":4}}\n\n",
            "data: [DONE]\n\n"
        );
        let response = MockResponse {
            status: StatusCode::OK,
            headers: vec![
                ("content-type", "text/event-stream"),
                ("x-request-id", "provider-123"),
            ],
            chunks: payload
                .as_bytes()
                .iter()
                .map(|byte| (Duration::ZERO, vec![*byte]))
                .collect(),
        };
        let server = start_server(vec![response]).await;
        let events = collect(&provider(&server), request(Duration::from_secs(2))).await;
        let events = events
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .expect("valid stream");

        assert!(matches!(
            &events[0],
            ModelEvent::ResponseStarted { provider_request_id: Some(id), .. } if id == "provider-123"
        ));
        let text = events
            .iter()
            .filter_map(|event| match event {
                ModelEvent::TextDelta { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<String>();
        assert_eq!(text, "你好");
        assert!(events.contains(&ModelEvent::Finish {
            reason: FinishReason::Stop
        }));
        assert!(events.contains(&ModelEvent::Usage {
            usage: TokenUsage {
                input_tokens: 12,
                output_tokens: 2,
                cached_input_tokens: 8,
                cache_miss_input_tokens: 4,
            }
        }));
        assert_eq!(server.state.request_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn emits_tool_call_argument_deltas_without_executing_tools() {
        let payload = concat!(
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call-1\",\"function\":{\"name\":\"lookup\",\"arguments\":\"{\\\"q\\\":\"}}]},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"\\\"x\\\"}\"}}]},\"finish_reason\":\"tool_calls\"}]}\n\n",
            "data: [DONE]\n\n"
        );
        let server = start_server(vec![MockResponse::sse(payload)]).await;
        let mut model_request = request(Duration::from_secs(2));
        model_request.tools.push(ToolDefinition {
            name: "lookup".to_string(),
            description: "synthetic".to_string(),
            parameters: json!({"type":"object"}),
            strict: false,
        });
        model_request.tool_choice = ToolChoice::Auto;
        let events = collect(&provider(&server), model_request)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .expect("tool stream");

        assert!(events.iter().any(|event| matches!(
            event,
            ModelEvent::ToolCallStarted { id, name, .. } if id == "call-1" && name == "lookup"
        )));
        assert_eq!(
            events
                .iter()
                .filter_map(|event| match event {
                    ModelEvent::ToolCallArgumentsDelta { arguments, .. } =>
                        Some(arguments.as_str()),
                    _ => None,
                })
                .collect::<String>(),
            "{\"q\":\"x\"}"
        );
        assert!(events.contains(&ModelEvent::ToolCallCompleted { index: 0 }));
    }

    #[tokio::test]
    async fn rejects_strict_tools_outside_the_beta_endpoint_without_an_http_request() {
        let server = start_server(Vec::new()).await;
        let mut model_request = request(Duration::from_secs(2));
        model_request.tools.push(ToolDefinition {
            name: "lookup".to_string(),
            description: "synthetic".to_string(),
            parameters: json!({"type":"object"}),
            strict: true,
        });
        let error = match provider(&server)
            .stream(model_request, CancellationToken::new())
            .await
        {
            Ok(_) => panic!("strict tools must be rejected"),
            Err(error) => error,
        };
        assert_eq!(error.kind, ModelErrorKind::UnsupportedCapability);
        assert_eq!(server.state.request_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn authentication_balance_and_invalid_request_errors_do_not_retry() {
        for (status, expected) in [
            (StatusCode::UNAUTHORIZED, ModelErrorKind::Authentication),
            (
                StatusCode::PAYMENT_REQUIRED,
                ModelErrorKind::InsufficientBalance,
            ),
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                ModelErrorKind::InvalidRequest,
            ),
        ] {
            let server = start_server(vec![MockResponse::status(
                status,
                "{\"error\":{\"message\":\"synthetic\"}}",
            )])
            .await;
            let error = match provider(&server)
                .stream(request(Duration::from_secs(2)), CancellationToken::new())
                .await
            {
                Ok(_) => panic!("request must fail"),
                Err(error) => error,
            };
            assert_eq!(error.kind, expected);
            assert!(!error.is_retryable());
            assert_eq!(server.state.request_count.load(Ordering::SeqCst), 1);
        }
    }

    #[tokio::test]
    async fn retries_rate_limits_and_overload_only_before_streaming() {
        for status in [
            StatusCode::TOO_MANY_REQUESTS,
            StatusCode::INTERNAL_SERVER_ERROR,
            StatusCode::SERVICE_UNAVAILABLE,
        ] {
            let mut retry =
                MockResponse::status(status, "{\"error\":{\"message\":\"retry later\"}}");
            retry.headers.push(("retry-after", "0"));
            let server = start_server(vec![
                retry,
                MockResponse::sse(
                    "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
                ),
            ])
            .await;
            let events = collect(&provider(&server), request(Duration::from_secs(2))).await;
            assert!(events.into_iter().all(|event| event.is_ok()));
            assert_eq!(server.state.request_count.load(Ordering::SeqCst), 2);
        }
    }

    #[tokio::test]
    async fn maps_empty_malformed_and_partial_streams_without_retrying_partial_output() {
        let fixtures = [
            (
                "data: {\"choices\":[]}\n\ndata: [DONE]\n\n",
                ModelErrorKind::EmptyResponse,
                false,
            ),
            ("data: {not-json}\n\n", ModelErrorKind::Decode, false),
            (
                "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"},\"finish_reason\":null}]}\n\n",
                ModelErrorKind::IncompleteStream,
                true,
            ),
        ];
        for (payload, expected_kind, partial) in fixtures {
            let server = start_server(vec![MockResponse::sse(payload)]).await;
            let events = collect(&provider(&server), request(Duration::from_secs(2))).await;
            let error = events
                .into_iter()
                .find_map(Result::err)
                .expect("terminal stream error");
            assert_eq!(error.kind, expected_kind);
            assert_eq!(error.partial, partial);
            assert!(!error.is_retryable());
            assert_eq!(server.state.request_count.load(Ordering::SeqCst), 1);
        }
    }

    #[tokio::test]
    async fn cancellation_after_a_delta_marks_the_stream_partial() {
        let server = start_server(vec![MockResponse {
            status: StatusCode::OK,
            headers: vec![("content-type", "text/event-stream")],
            chunks: vec![
                (
                    Duration::ZERO,
                    b"data: {\"choices\":[{\"delta\":{\"content\":\"partial\"},\"finish_reason\":null}]}\n\n".to_vec(),
                ),
                (Duration::from_secs(5), b"data: [DONE]\n\n".to_vec()),
            ],
        }])
        .await;
        let cancellation = CancellationToken::new();
        let mut events = provider(&server)
            .stream(request(Duration::from_secs(10)), cancellation.clone())
            .await
            .expect("stream starts");
        assert!(matches!(
            events.next().await,
            Some(Ok(ModelEvent::ResponseStarted { .. }))
        ));
        assert!(matches!(
            events.next().await,
            Some(Ok(ModelEvent::TextDelta { .. }))
        ));
        cancellation.cancel();
        let error = events
            .next()
            .await
            .expect("cancel event")
            .expect_err("cancel error");
        assert_eq!(error.kind, ModelErrorKind::Cancelled);
        assert!(error.partial);
    }

    #[tokio::test]
    async fn stream_timeout_is_stable_and_does_not_claim_partial_output() {
        let server = start_server(vec![MockResponse {
            status: StatusCode::OK,
            headers: vec![("content-type", "text/event-stream")],
            chunks: vec![
                (Duration::ZERO, b": keepalive\n\n".to_vec()),
                (Duration::from_secs(3), b"data: [DONE]\n\n".to_vec()),
            ],
        }])
        .await;
        let events = collect(&provider(&server), request(Duration::from_secs(1))).await;
        let errors = events
            .into_iter()
            .filter_map(Result::err)
            .collect::<Vec<_>>();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, ModelErrorKind::Timeout);
        assert!(!errors[0].partial);
        assert_eq!(server.state.request_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn json_mode_rejects_empty_content_and_serializes_response_format() {
        let server = start_server(vec![MockResponse::sse(
            "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
        )])
        .await;
        let error = provider(&server)
            .complete_json(request(Duration::from_secs(2)), CancellationToken::new())
            .await
            .expect_err("empty JSON must fail");
        assert_eq!(error.kind, ModelErrorKind::EmptyResponse);
        let requests = server.state.requests.lock().await;
        assert_eq!(requests[0]["response_format"]["type"], "json_object");
        assert_eq!(requests[0]["stream_options"]["include_usage"], true);
    }
}
