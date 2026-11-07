//! Direct Responses conversation adapter. Provider wire objects stay here.
use crate::profiles::{SecretRef, identifier, resolve_secret, validate_endpoint};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{future::Future, pin::Pin, time::Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelConfig {
    pub driver: String,
    pub model: String,
    pub endpoint: String,
    pub secret_ref: SecretRef,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub supports_images: bool,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    #[serde(default = "timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "output")]
    pub max_output_tokens: u32,
    #[serde(default = "budget")]
    pub max_turn_tokens: u64,
}
fn timeout() -> u64 {
    45
}
fn output() -> u32 {
    2048
}
fn budget() -> u64 {
    16384
}
impl ModelConfig {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.driver == "openai-responses" && identifier(&self.model),
            "Actor requires openai-responses and an explicit model"
        );
        validate_endpoint(&self.endpoint)?;
        ensure!(
            (1..=90).contains(&self.timeout_seconds)
                && (128..=8192).contains(&self.max_output_tokens)
                && (1024..=65536).contains(&self.max_turn_tokens),
            "invalid Actor timeout or token budget"
        );
        ensure!(
            self.reasoning_effort.as_deref().is_none_or(|s| matches!(
                s,
                "none" | "minimal" | "low" | "medium" | "high" | "xhigh"
            )),
            "unsupported Actor reasoning setting"
        );
        crate::profiles::check_sensitive(&(self.model.clone(), self.endpoint.clone()))?;
        Ok(())
    }
}
pub struct Request {
    pub persona: String,
    pub input: Vec<Value>,
    pub tools: Vec<Value>,
    pub images: Vec<InputImage>,
}
/// Private pixels never implement Serialize/Debug or enter persisted Actor messages.
pub struct InputImage {
    pub label: String,
    pub jpeg: Vec<u8>,
}
#[derive(Debug)]
pub struct Call {
    pub id: String,
    pub name: String,
    pub arguments: String,
}
pub struct Reply {
    pub text: String,
    pub calls: Vec<Call>,
    pub usage_tokens: u64,
    pub continuation: Vec<Value>,
}
/// A completed response attempted an invocation through ordinary text. The
/// adapter exposes only returned usage, never a parsed call or the raw body.
#[derive(Debug, thiserror::Error)]
#[error("Actor reply used text instead of structured tool calls")]
pub(super) struct UnstructuredToolInvocation {
    pub usage_tokens: u64,
}
/// Numeric accounting context for an unsuccessful terminal provider response.
/// This is not a Reply and must never qualify for invocation-format repair.
#[derive(Debug, thiserror::Error)]
#[error("Actor unsuccessful terminal response reported usage")]
pub(super) struct ReturnedFailureUsage {
    pub usage_tokens: u64,
}
pub trait ActorModel: Send + Sync {
    fn supports_images(&self) -> bool {
        false
    }
    fn respond_stream(
        &self,
        request: Request,
        _preview: super::preview::PreviewSink,
    ) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
        self.respond(request)
    }
    fn respond(&self, request: Request)
    -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>>;
}
pub struct ResponsesModel {
    config: ModelConfig,
    http: reqwest::Client,
}
/// Allowlisted operational causes only; never retain provider bodies, URLs or credentials.
#[derive(Debug, thiserror::Error)]
pub(super) enum ProviderFailure {
    #[error("Actor provider request timed out")]
    Timeout,
    #[error("Actor provider connection failed")]
    Connection,
    #[error("Actor provider rejected request with HTTP {0}")]
    Http(u16),
    #[error("Actor provider response ended before completion")]
    Disconnected,
    #[error("Actor provider output limit reached")]
    OutputLimit,
    #[error("Actor provider response incomplete")]
    Incomplete,
    #[error("Actor provider reply failed protocol validation")]
    InvalidResponse,
    #[error("Actor provider reply format repair exhausted")]
    FormatRepairExhausted,
}
impl ProviderFailure {
    pub(super) fn notice(&self) -> String {
        if matches!(self, Self::FormatRepairExhausted) {
            return "模型回复格式仍不正确。本轮已尝试一次格式修复，未接收其中的工具调用；已有操作以回执为准，没有重放操作。".into();
        }
        let cause = match self {
            Self::Timeout => "模型服务请求超时".into(),
            Self::Connection => "暂时无法连接模型服务".into(),
            Self::Http(code) => format!("模型服务拒绝了请求（HTTP {code}）"),
            Self::Disconnected => "模型回复在完成前断开".into(),
            Self::OutputLimit => "模型回复达到输出长度上限，未使用未完成的工具参数".into(),
            Self::Incomplete => "模型服务没有完整返回本轮回复".into(),
            Self::InvalidResponse => "模型回复未通过协议格式校验".into(),
            Self::FormatRepairExhausted => unreachable!(),
        };
        format!("{cause}。本次失败请求未自动重试；已有操作以回执为准，没有重放操作。")
    }
}
impl ResponsesModel {
    pub fn new(config: ModelConfig) -> Result<Self> {
        config.validate()?;
        let http = reqwest::Client::builder()
            .retry(reqwest::retry::never())
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .context("construct Actor HTTP client")?;
        Ok(Self { config, http })
    }
}
impl ActorModel for ResponsesModel {
    fn supports_images(&self) -> bool {
        self.config.supports_images
    }
    fn respond(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
        self.respond_inner(request, None)
    }
    fn respond_stream(
        &self,
        request: Request,
        preview: super::preview::PreviewSink,
    ) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
        self.respond_inner(request, Some(preview))
    }
}
impl ResponsesModel {
    fn respond_inner(
        &self,
        request: Request,
        preview: Option<super::preview::PreviewSink>,
    ) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
        Box::pin(async move {
            let credential = resolve_secret(&self.config.secret_ref)
                .context("Actor credential missing; set its LAIN_SECRET variable")?;
            let mut body = json!({"model":self.config.model,"instructions":request.persona,"input":request.input,"tools":request.tools,"parallel_tool_calls":false,"store":false,"stream":true,"include":["reasoning.encrypted_content"],"max_output_tokens":self.config.max_output_tokens});
            if let Some(effort) = &self.config.reasoning_effort {
                body["reasoning"] = json!({"effort":effort});
            }
            ensure!(
                serde_json::to_vec(&body)?.len() <= 128 * 1024,
                "Actor request context exceeds bounds"
            );
            attach_images(&mut body, request.images, self.config.supports_images)?;
            let mut response = self
                .http
                .post(format!(
                    "{}/responses",
                    self.config.endpoint.trim_end_matches('/')
                ))
                .bearer_auth(credential.0.as_str())
                .json(&body)
                .send()
                .await
                .map_err(|e| {
                    if e.is_timeout() {
                        ProviderFailure::Timeout
                    } else {
                        ProviderFailure::Connection
                    }
                })?;
            if !response.status().is_success() {
                return Err(ProviderFailure::Http(response.status().as_u16()).into());
            }
            // Consume genuine provider streaming, but publish only complete redacted records.
            // Raw fragments could split a credential across frames and are never stored/exposed.
            let mut parser = ResponseStream::default();
            while let Some(chunk) = response.chunk().await.map_err(|e| {
                if e.is_timeout() {
                    ProviderFailure::Timeout
                } else {
                    ProviderFailure::Disconnected
                }
            })? {
                if let Some(sink) = &preview {
                    sink.first_byte();
                }
                if let Some(reply) =
                    parser
                        .push_with_preview(&chunk, preview.as_ref())
                        .map_err(|e| {
                            if e.is::<ProviderFailure>() || e.is::<UnstructuredToolInvocation>() {
                                e
                            } else {
                                ProviderFailure::InvalidResponse.into()
                            }
                        })?
                {
                    return Ok(reply);
                }
            }
            Err(ProviderFailure::Disconnected.into())
        })
    }
}
fn attach_images(body: &mut Value, images: Vec<InputImage>, supported: bool) -> Result<()> {
    use base64::Engine;
    ensure!(
        images.is_empty() || supported,
        "Actor image input is not configured"
    );
    ensure!(images.len() <= 24, "Actor image frame count exceeds bounds");
    let input = body["input"]
        .as_array_mut()
        .context("Actor input missing")?;
    for image in images {
        ensure!(
            !image.jpeg.is_empty() && image.jpeg.len() <= 384 * 1024 && image.label.len() <= 512,
            "Actor image frame exceeds bounds"
        );
        let data = base64::engine::general_purpose::STANDARD.encode(&image.jpeg);
        input.push(json!({"role":"user","content":[
            {"type":"input_text","text":image.label},
            {"type":"input_image","image_url":format!("data:image/jpeg;base64,{data}"),"detail":"auto"}
        ]}));
    }
    ensure!(
        serde_json::to_vec(body)?.len() <= 16 * 1024 * 1024,
        "Actor visual request exceeds bounds"
    );
    Ok(())
}
#[derive(Default)]
struct ResponseStream {
    pending: Vec<u8>,
    data: String,
    frame_bytes: usize,
    total: usize,
    saw_delta: bool,
    preview_prefix: String,
    preview_decided: bool,
    preview_blocked: bool,
}
// This is a rejected provider envelope, never an alternate tool protocol.
const TEXT_INVOCATION_PREFIX: &str = "<｜｜DSML｜｜ calls>";
impl ResponseStream {
    fn preview_text(
        &mut self,
        text: &str,
        preview: Option<&super::preview::PreviewSink>,
    ) -> Result<()> {
        let Some(sink) = preview else {
            return Ok(());
        };
        if self.preview_blocked {
            return Ok(());
        }
        if self.preview_decided {
            sink.delta(text);
            return Ok(());
        }
        self.preview_prefix.push_str(text);
        ensure!(
            self.preview_prefix.len() <= 16384,
            "Actor preview prefix exceeds bounds"
        );
        let prefix = self.preview_prefix.trim_start();
        if prefix.starts_with(TEXT_INVOCATION_PREFIX) {
            self.preview_blocked = true;
            self.preview_prefix.clear();
        } else if !TEXT_INVOCATION_PREFIX.starts_with(prefix) {
            self.preview_decided = true;
            sink.delta(&std::mem::take(&mut self.preview_prefix));
        }
        Ok(())
    }
    #[cfg(test)]
    fn push(&mut self, bytes: &[u8]) -> Result<Option<Reply>> {
        self.push_with_preview(bytes, None)
    }
    fn push_with_preview(
        &mut self,
        bytes: &[u8],
        preview: Option<&super::preview::PreviewSink>,
    ) -> Result<Option<Reply>> {
        self.total = self.total.saturating_add(bytes.len());
        ensure!(
            self.total <= 2 * 1024 * 1024,
            "Actor response exceeded bounds"
        );
        self.pending.extend_from_slice(bytes);
        while let Some(end) = self.pending.iter().position(|b| *b == b'\n') {
            self.frame_bytes += end + 1;
            ensure!(
                self.frame_bytes <= 256 * 1024,
                "Actor stream frame exceeded bounds"
            );
            let bytes: Vec<u8> = self.pending.drain(..=end).collect();
            let line = std::str::from_utf8(&bytes)
                .context("invalid Actor stream encoding")?
                .trim_end_matches(['\r', '\n']);
            if line.is_empty() {
                self.frame_bytes = 0;
                let data = std::mem::take(&mut self.data);
                if data.is_empty() || data == "[DONE]" {
                    continue;
                }
                let event: Value = serde_json::from_str(&data)
                    .map_err(|_| anyhow::anyhow!("invalid Actor stream frame"))?;
                match event["type"].as_str() {
                    Some("response.output_text.delta") => {
                        let text = event["delta"]
                            .as_str()
                            .context("Actor text delta missing")?;
                        ensure!(text.len() <= 16384, "Actor delta exceeds bounds");
                        self.saw_delta = true;
                        self.preview_text(text, preview)?;
                    }
                    Some("response.output_item.done")
                        if !self.saw_delta && event["item"]["type"] == "message" =>
                    {
                        ensure!(
                            event["item"]["role"] == "assistant",
                            "unexpected preview role"
                        );
                        if let Some(parts) = event["item"]["content"].as_array() {
                            for part in parts {
                                if part["type"] == "output_text" {
                                    let text = part["text"]
                                        .as_str()
                                        .context("completed Actor text missing")?;
                                    self.preview_text(text, preview)?;
                                }
                            }
                        }
                    }
                    Some("response.completed") => {
                        if let Some(sink) = preview {
                            sink.provider_completed();
                        }
                        let reply = parse_response(&event["response"])?;
                        if !self.preview_blocked
                            && !self.preview_prefix.is_empty()
                            && let Some(sink) = preview
                        {
                            sink.delta(&std::mem::take(&mut self.preview_prefix));
                        }
                        return Ok(Some(reply));
                    }
                    Some("response.incomplete") => {
                        let failure = if event["response"]["incomplete_details"]["reason"]
                            == "max_output_tokens"
                        {
                            ProviderFailure::OutputLimit
                        } else {
                            ProviderFailure::Incomplete
                        };
                        let error = anyhow::Error::from(failure);
                        // Persist only a reported integer representable by the
                        // existing ledger; missing/invalid usage stays unknown.
                        return Err(
                            match event["response"]["usage"]["total_tokens"]
                                .as_u64()
                                .filter(|tokens| *tokens <= i64::MAX as u64)
                            {
                                Some(usage_tokens) => {
                                    error.context(ReturnedFailureUsage { usage_tokens })
                                }
                                None => error,
                            },
                        );
                    }
                    Some("response.failed" | "error") => {
                        return Err(ProviderFailure::Incomplete.into());
                    }
                    // Tool arguments, reasoning and provider metadata remain private.
                    _ => {}
                }
            } else if let Some(data) = line.strip_prefix("data:") {
                if !self.data.is_empty() {
                    self.data.push('\n');
                }
                self.data.push_str(data.strip_prefix(' ').unwrap_or(data));
            }
        }
        ensure!(
            self.pending.len() + self.frame_bytes <= 256 * 1024,
            "Actor stream frame exceeded bounds"
        );
        Ok(None)
    }
}
fn parse_response(value: &Value) -> Result<Reply> {
    ensure!(
        value["status"] == "completed",
        "Actor response not completed"
    );
    let output = value["output"].as_array().context("Actor output missing")?;
    ensure!(
        output.len() <= 32 && serde_json::to_vec(output)?.len() <= 128 * 1024,
        "Actor output exceeded bounds"
    );
    let mut text = String::new();
    let mut calls = Vec::new();
    let mut identities = std::collections::BTreeSet::new();
    for item in output {
        match item["type"].as_str() {
            Some("message") => {
                ensure!(item["role"] == "assistant", "unexpected Actor output role");
                for part in item["content"]
                    .as_array()
                    .context("Actor message content missing")?
                {
                    if part["type"] == "output_text" {
                        text.push_str(part["text"].as_str().context("Actor text missing")?);
                    }
                    if part["type"] == "refusal" {
                        text.push_str("The model declined this request.");
                    }
                }
            }
            Some("function_call") => {
                let id = item["call_id"]
                    .as_str()
                    .context("Actor call identity missing")?;
                ensure!(
                    !id.is_empty()
                        && id.len() <= 128
                        && id
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
                        && identities.insert(id.to_owned()),
                    "invalid Actor call identity"
                );
                calls.push(Call {
                    id: id.into(),
                    name: item["name"]
                        .as_str()
                        .context("Actor tool name missing")?
                        .into(),
                    arguments: item["arguments"]
                        .as_str()
                        .context("Actor arguments missing")?
                        .into(),
                });
            }
            Some("reasoning") => {} // encrypted continuation is transient, never durable/public
            _ => anyhow::bail!("unsupported Actor output type"),
        }
    }
    ensure!(
        text.len() <= 16384
            && calls.len() <= super::MAX_TOOLS
            && (!text.trim().is_empty() || !calls.is_empty()),
        "Actor reply empty or exceeds bounds"
    );
    if text.trim_start().starts_with(TEXT_INVOCATION_PREFIX) {
        return Err(UnstructuredToolInvocation {
            usage_tokens: value["usage"]["total_tokens"].as_u64().unwrap_or(0),
        }
        .into());
    }
    Ok(Reply {
        text,
        calls,
        usage_tokens: value["usage"]["total_tokens"].as_u64().unwrap_or(0),
        continuation: output.clone(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn textual_invocation_response(text: &str) -> Value {
        json!({"status":"completed","usage":{"total_tokens":44},"output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":text}]}]})
    }
    #[test]
    fn textual_tool_invocation_is_not_a_completed_answer_or_executable_call() {
        let text = "\n<｜｜DSML｜｜ calls>\n<｜｜DSML｜｜ invoke name=\"memory_forget\">private-fixture-argument</｜｜DSML｜｜ invoke>\n</｜｜DSML｜｜ calls>";
        let error = parse_response(&textual_invocation_response(text))
            .err()
            .expect("textual calls must not complete ordinary chat");
        let failure = error
            .downcast_ref::<UnstructuredToolInvocation>()
            .expect("keep the normalized returned-format cause");
        assert_eq!(failure.usage_tokens, 44);
        assert!(!format!("{error:?}").contains("private-fixture-argument"));
        for quoted in [
            "```xml\n<｜｜DSML｜｜ calls>example</｜｜DSML｜｜ calls>\n```",
            "这是引用：<｜｜DSML｜｜ calls>，不执行。",
            "<",
        ] {
            assert_eq!(
                parse_response(&textual_invocation_response(quoted))
                    .unwrap()
                    .text,
                quoted
            );
        }
    }
    #[test]
    fn textual_tool_invocation_split_previews_never_disclose_the_envelope() {
        use super::super::{
            Observation,
            preview::{PreviewSink, Registry},
        };
        use crate::redaction::Redactor;
        use lain_wire::{ActorTurnId, MessageId, PreviewUpdate};
        let text = "\n<｜｜DSML｜｜ calls><｜｜DSML｜｜ invoke name=\"briefing_inspect\">fixture</｜｜DSML｜｜ invoke></｜｜DSML｜｜ calls>";
        let mut wire = String::new();
        for ch in text.chars() {
            wire.push_str(&format!(
                "data: {}\n\n",
                json!({"type":"response.output_text.delta","delta":ch.to_string()})
            ));
        }
        wire.push_str(&format!(
            "data: {}\n\n",
            json!({"type":"response.completed","response":textual_invocation_response(text)})
        ));
        for size in [1, 3, 17, wire.len()] {
            let (sender, mut events) = tokio::sync::broadcast::channel(1024);
            let sink = PreviewSink::new(
                sender,
                Redactor::new(vec![]),
                ActorTurnId::new(),
                MessageId::new(),
                &Registry::default(),
            );
            let mut stream = ResponseStream::default();
            let mut failure = None;
            for chunk in wire.as_bytes().chunks(size) {
                match stream.push_with_preview(chunk, Some(&sink)) {
                    Ok(None) => {}
                    Ok(Some(_)) => panic!("textual invocation was accepted as prose"),
                    Err(error) => {
                        failure = Some(error);
                        break;
                    }
                }
            }
            assert!(failure.unwrap().is::<UnstructuredToolInvocation>());
            while let Ok(Observation::Preview(event)) = events.try_recv() {
                if let PreviewUpdate::Delta { text } = event.update {
                    assert!(text.trim().is_empty(), "provider envelope escaped preview");
                }
            }
        }
    }
    #[test]
    fn textual_tool_invocation_filter_preserves_item_done_and_literal_example_previews() {
        use super::super::{
            Observation,
            preview::{PreviewSink, Registry},
        };
        use crate::redaction::Redactor;
        use lain_wire::{ActorTurnId, MessageId, PreviewUpdate};
        for text in [
            "<",
            "  <example>",
            "```xml\n<｜｜DSML｜｜ calls>引用示例</｜｜DSML｜｜ calls>\n```",
            "\n<｜｜DSML｜｜ calls>拒绝</｜｜DSML｜｜ calls>",
        ] {
            let response = textual_invocation_response(text);
            let wire = format!(
                "data: {}\n\ndata: {}\n\n",
                json!({"type":"response.output_item.done","item":response["output"][0]}),
                json!({"type":"response.completed","response":response})
            );
            let (sender, mut events) = tokio::sync::broadcast::channel(64);
            let sink = PreviewSink::new(
                sender,
                Redactor::new(vec![]),
                ActorTurnId::new(),
                MessageId::new(),
                &Registry::default(),
            );
            let result = ResponseStream::default().push_with_preview(wire.as_bytes(), Some(&sink));
            let rejected = text.trim_start().starts_with("<｜｜DSML｜｜ calls>");
            if rejected {
                assert!(result.err().unwrap().is::<UnstructuredToolInvocation>());
            } else {
                let reply = result.unwrap().unwrap();
                sink.finish(&reply.text);
            }
            let mut visible = String::new();
            let mut finished = false;
            while let Ok(Observation::Preview(event)) = events.try_recv() {
                match event.update {
                    PreviewUpdate::Delta { text } => visible.push_str(&text),
                    PreviewUpdate::Finished => finished = true,
                    _ => {}
                }
            }
            assert_eq!(visible, if rejected { "" } else { text });
            assert_eq!(finished, !rejected);
        }
    }
    #[test]
    fn provider_parallel_function_calls_remain_a_bounded_complete_batch() {
        let response = json!({"status":"completed","usage":{"total_tokens":123},"output":[
            {"type":"function_call","call_id":"call_devices","name":"device_list","arguments":"{}"},
            {"type":"function_call","call_id":"call_work","name":"device_work_list","arguments":"{}"}
        ]});
        let reply = parse_response(&response)
            .expect("parallel_tool_calls=false is not a provider guarantee");
        assert_eq!(reply.calls.len(), 2);
        assert_eq!(reply.usage_tokens, 123);
        assert_eq!(reply.continuation.len(), 2);
        let mut invalid = response.clone();
        invalid["output"][1]["call_id"] = json!("call_devices");
        assert!(
            parse_response(&invalid).is_err(),
            "duplicate identities must not alias receipts"
        );
    }
    #[test]
    fn incomplete_output_is_classified_without_retaining_provider_error_text() {
        let wire = b"data: {\"type\":\"response.incomplete\",\"response\":{\"incomplete_details\":{\"reason\":\"max_output_tokens\"},\"error\":{\"message\":\"private-provider-body\"}}}\n\n";
        let error = ResponseStream::default().push(wire).err().unwrap();
        let failure = error.downcast_ref::<ProviderFailure>().unwrap();
        assert!(matches!(failure, ProviderFailure::OutputLimit));
        assert!(failure.notice().contains("输出长度上限"));
        assert!(!failure.notice().contains("private-provider-body"));
        assert!(!format!("{error:?}").contains("private-provider-body"));
    }
    struct IncompleteStreamModel {
        gate: tokio::sync::Semaphore,
        usage: Value,
        requests: std::sync::atomic::AtomicUsize,
    }
    impl ActorModel for IncompleteStreamModel {
        fn respond(&self, _: Request) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
            self.requests
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Box::pin(async move {
                self.gate.acquire().await.unwrap().forget();
                let event = json!({"type":"response.incomplete","response":{
                    "incomplete_details":{"reason":"max_output_tokens"},
                    "usage":{"total_tokens":self.usage},
                    "error":{"message":"private-provider-body"},
                    "output":[{"type":"function_call","call_id":"unfinished-call",
                        "name":"memory_forget","arguments":"private-partial-arguments"}]
                }});
                let wire = format!("data: {event}\n\n");
                match ResponseStream::default().push(wire.as_bytes()) {
                    Err(error) => Err(error),
                    Ok(_) => panic!("incomplete response must never become an accepted reply"),
                }
            })
        }
    }
    #[tokio::test]
    async fn incomplete_terminal_usage_is_accounted_without_acceptance_or_replay() {
        use crate::{
            actor::{
                Actor,
                domain::{MessageRole, TurnState},
            },
            kernel::Kernel,
        };
        use lain_wire::inference::InferenceRequestState;
        use std::sync::{Arc, atomic::Ordering};
        // Unknown/malformed usage stays unreported; zero is a known return.
        for (usage, expected) in [
            (json!(11059), Some(11059)),
            (json!(0), Some(0)),
            (Value::Null, None),
            (json!("11059"), None),
            (json!(u64::MAX), None),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("usage.sqlite3");
            let kernel = Arc::new(Kernel::open(&path).unwrap());
            kernel.mark_ready().unwrap();
            let model = Arc::new(IncompleteStreamModel {
                usage,
                gate: tokio::sync::Semaphore::new(1),
                requests: std::sync::atomic::AtomicUsize::new(0),
            });
            let mut actor = Actor::with_model(
                kernel.clone(),
                None,
                "fixture-incomplete-token-12345678901234567890"
                    .parse()
                    .unwrap(),
                model.clone(),
            );
            // Accounting an over-budget terminal failure must not turn it into
            // a completed reply, a format regeneration or a different cause.
            Arc::get_mut(&mut actor).unwrap().max_tokens = 1024;
            let conversation = kernel.create_conversation().unwrap().id;
            let request_id = uuid::Uuid::new_v4();
            let first = actor
                .submit(conversation, request_id, "fixture question".into())
                .await
                .unwrap();
            tokio::time::timeout(Duration::from_secs(8), async {
                while Arc::strong_count(&actor) > 1 {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            })
            .await
            .unwrap();
            let turn = kernel.actor_turn(first.id).unwrap();
            assert_eq!(turn.state, TurnState::Unknown);
            assert!(turn.error.as_deref().unwrap().contains("输出长度上限"));
            assert!(turn.actions.is_empty());
            assert!(
                !turn
                    .messages
                    .iter()
                    .any(|m| m.role == MessageRole::Assistant || m.content.contains("格式修复"))
            );
            assert_eq!(turn.usage_tokens, expected.unwrap_or(0));
            assert_eq!(turn.timing.requests.len(), usize::from(expected.is_some()));
            let ledger = kernel.inference_snapshot().unwrap();
            assert_eq!(ledger.reserved_requests, 1);
            assert_eq!(ledger.reported_usage_tokens_sum, expected.unwrap_or(0));
            let row = ledger
                .recent_requests
                .iter()
                .find(|r| r.actor_turn_id == first.id)
                .unwrap();
            assert_eq!(row.reported_usage_tokens, expected);
            assert_eq!(row.returned_at_ms.is_some(), expected.is_some());
            assert_eq!(
                row.state,
                if expected.is_some() {
                    InferenceRequestState::Returned
                } else {
                    InferenceRequestState::Unknown
                }
            );
            assert!(!serde_json::to_string(&turn).unwrap().contains("private-"));
            if let Some(tokens) = expected {
                assert!(
                    kernel.inference_returned(first.id, 1, tokens).is_err(),
                    "a returned reservation is not charged twice"
                );
            }
            let duplicate = actor
                .submit(conversation, request_id, "fixture question".into())
                .await
                .unwrap();
            assert_eq!(duplicate.id, first.id);
            assert_eq!(model.requests.load(Ordering::SeqCst), 1);
            drop(actor);
            drop(kernel);
            let reopened = Kernel::open(&path).unwrap();
            assert_eq!(
                reopened.actor_turn(first.id).unwrap().state,
                TurnState::Unknown
            );
            assert_eq!(
                reopened
                    .inference_snapshot()
                    .unwrap()
                    .reported_usage_tokens_sum,
                expected.unwrap_or(0)
            );
            assert_eq!(model.requests.load(Ordering::SeqCst), 1);
        }
    }
    #[tokio::test]
    async fn incomplete_briefing_keeps_reported_usage_without_items_even_after_revocation() {
        use crate::briefing_model_tests::{fixture, until};
        use crate::{
            actor::{
                Actor,
                domain::{MessageRole, TurnState},
            },
            kernel::Kernel,
            redaction::Redactor,
        };
        use lain_wire::{RequestId, briefing::SetBriefingGrant};
        use std::sync::{Arc, atomic::Ordering};
        for revoke in [false, true] {
            let (dir, k, c, manifest, _) = fixture();
            let model = Arc::new(IncompleteStreamModel {
                usage: json!(11059),
                requests: std::sync::atomic::AtomicUsize::new(0),
                gate: tokio::sync::Semaphore::new(0),
            });
            let actor = Actor::with_model(
                k.clone(),
                None,
                "fixture-failed-usage-01234567890123456789".parse().unwrap(),
                model.clone(),
            );
            let first = actor
                .process_briefing(c, manifest.request.request_id, 0)
                .await
                .unwrap();
            until(|| model.requests.load(Ordering::SeqCst) > 0).await;
            if revoke {
                k.briefing_set(
                    c,
                    SetBriefingGrant {
                        request_id: RequestId::new(),
                        expected_revision: 1,
                        scope: None,
                    },
                )
                .unwrap();
            }
            model.gate.add_permits(1);
            until(|| Arc::strong_count(&actor) == 1).await;
            let turn = k.actor_turn(first.id).unwrap();
            assert_eq!(turn.state, TurnState::Unknown);
            assert_eq!(turn.usage_tokens, 11059);
            assert_eq!(turn.model_requests, 1);
            assert_eq!(turn.timing.requests.len(), 1);
            assert!(turn.error.as_deref().unwrap().contains("输出长度上限"));
            assert!(turn.actions.is_empty());
            assert!(
                !turn
                    .messages
                    .iter()
                    .any(|m| m.role == MessageRole::Assistant)
            );
            let batch = k
                .briefing_batch(c, manifest.request.request_id, 0, Redactor::new(vec![]))
                .unwrap();
            assert!(batch.items.is_none());
            let ledger = k.inference_snapshot().unwrap();
            assert_eq!(ledger.reserved_requests, 1);
            assert_eq!(ledger.reported_usage_tokens_sum, 11059);
            let row = ledger
                .recent_requests
                .iter()
                .find(|r| r.actor_turn_id == first.id)
                .unwrap();
            assert_eq!(row.reason, lain_wire::inference::InferenceReason::Briefing);
            assert_eq!(
                row.state,
                lain_wire::inference::InferenceRequestState::Returned
            );
            assert!(
                !serde_json::to_string(&turn)
                    .unwrap()
                    .contains("private-partial-arguments")
            );
            assert_eq!(model.requests.load(Ordering::SeqCst), 1);
            drop(actor);
            drop(k);
            let reopened = Arc::new(Kernel::open(&dir.path().join("state.sqlite")).unwrap());
            reopened.mark_ready().unwrap();
            assert_eq!(
                reopened.actor_turn(first.id).unwrap().state,
                TurnState::Unknown
            );
            assert_eq!(
                reopened
                    .inference_snapshot()
                    .unwrap()
                    .reported_usage_tokens_sum,
                11059
            );
            if !revoke {
                let actor = Actor::with_model(
                    reopened,
                    None,
                    "fixture-failed-usage-01234567890123456789".parse().unwrap(),
                    model.clone(),
                );
                let duplicate = actor
                    .process_briefing(c, manifest.request.request_id, 0)
                    .await
                    .unwrap();
                assert_eq!(duplicate.id, first.id);
                assert_eq!(duplicate.state, TurnState::Unknown);
                assert_eq!(model.requests.load(Ordering::SeqCst), 1);
            }
        }
    }
    #[test]
    fn visual_payload_is_provider_local_bounded_and_requires_declared_support() {
        use base64::Engine;
        let make = || InputImage {
            label: "media=fixture frame=0".into(),
            jpeg: vec![0xff, 0xd8, 0xff, 0xd9],
        };
        let mut body = json!({"input":[{"role":"user","content":"original mixed text [picture]"}]});
        assert!(attach_images(&mut body, vec![make()], false).is_err());
        assert_eq!(body["input"].as_array().unwrap().len(), 1);
        attach_images(&mut body, vec![make()], true).unwrap();
        assert_eq!(body["input"][0]["content"], "original mixed text [picture]");
        let data = body["input"][1]["content"][1]["image_url"]
            .as_str()
            .unwrap()
            .strip_prefix("data:image/jpeg;base64,")
            .unwrap();
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(data)
                .unwrap(),
            make().jpeg
        );
        assert!(
            attach_images(
                &mut json!({"input":[]}),
                vec![InputImage {
                    label: "x".into(),
                    jpeg: vec![0; 384 * 1024 + 1]
                }],
                true
            )
            .is_err()
        );
        assert!(
            attach_images(
                &mut json!({"input":[]}),
                (0..25).map(|_| make()).collect(),
                true
            )
            .is_err()
        );
    }
    #[test]
    fn multiline_character_previews_are_private_until_sanitized_and_final_is_authoritative() {
        use crate::actor::{
            Observation,
            preview::{PreviewSink, Registry},
        };
        use crate::redaction::Redactor;
        use lain_wire::{ActorTurnId, MessageId, PreviewUpdate};
        let completed = json!({"type":"response.completed","response":{"status":"completed","output":[
            {"type":"message","role":"assistant","content":[{"type":"output_text","text":"你好\n世界"}]},
            {"type":"function_call","call_id":"call_1","name":"work_list","arguments":"{}"}]}});
        let mut wire = String::new();
        for text in ["你", "好", "\n", "世", "界"] {
            wire.push_str(&format!(
                "data: {{\"type\":\"response.output_text.delta\",\ndata: \"delta\":{}}}\n\n",
                serde_json::to_string(text).unwrap()
            ));
        }
        wire.push_str("data: {\"type\":\"response.function_call_arguments.delta\",\"delta\":\"private-credential\"}\n\n");
        wire.push_str(&format!("data: {completed}\n\n"));
        for chunk_size in [1, 2, 3, 7, 19, wire.len()] {
            let (sender, mut events) = tokio::sync::broadcast::channel(32);
            let sink = PreviewSink::new(
                sender,
                Redactor::new(vec!["private-credential".into()]),
                ActorTurnId::new(),
                MessageId::new(),
                &Registry::default(),
            );
            let mut parser = ResponseStream::default();
            let mut reply = None;
            for chunk in wire.as_bytes().chunks(chunk_size) {
                if let Some(value) = parser.push_with_preview(chunk, Some(&sink)).unwrap() {
                    reply = Some(value);
                }
            }
            let reply = reply.unwrap();
            assert_eq!(reply.calls[0].arguments, "{}");
            sink.finish(&reply.text);
            let mut visible = String::new();
            let mut finished = false;
            while let Ok(Observation::Preview(event)) = events.try_recv() {
                match event.update {
                    PreviewUpdate::Delta { text } => visible.push_str(&text),
                    PreviewUpdate::Finished => finished = true,
                    _ => {}
                }
            }
            assert_eq!(visible, "你好\n世界");
            assert!(finished);
        }
    }
    #[test]
    fn failures_incomplete_empty_and_unterminated_frames_never_produce_a_reply() {
        for kind in ["response.failed", "response.incomplete", "error"] {
            let wire = format!(
                "data: {{\"type\":\"response.output_text.delta\",\"delta\":\"partial\"}}\n\ndata: {{\"type\":\"{kind}\"}}\n\n"
            );
            assert!(ResponseStream::default().push(wire.as_bytes()).is_err());
        }
        assert!(ResponseStream::default().push(b"data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"output\":[]}}\n\n").is_err());
        assert!(
            ResponseStream::default()
                .push(b"data: {\"type\":\"response.completed\"}")
                .unwrap()
                .is_none()
        );
        assert!(
            ResponseStream::default()
                .push(&vec![b'x'; 256 * 1024 + 1])
                .is_err()
        );
    }
    #[test]
    fn streaming_maps_split_utf8_and_calls_without_exposing_raw_deltas() {
        let event = json!({"type":"response.completed","response":{"status":"completed","output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"你来了。"}]},{"type":"function_call","call_id":"call_1","name":"work_list","arguments":"{}"}],"usage":{"total_tokens":12}}});
        let bytes = format!("data: {}\r\n\r\n", event).into_bytes();
        let mut parser = ResponseStream::default();
        let mut reply = None;
        for byte in bytes {
            if let Some(r) = parser.push(&[byte]).unwrap() {
                reply = Some(r);
            }
        }
        let reply = reply.unwrap();
        assert_eq!(reply.text, "你来了。");
        assert_eq!(reply.calls[0].name, "work_list");
        assert_eq!(reply.usage_tokens, 12);
        assert!(parse_response(&json!({"status":"incomplete","output":[]})).is_err());
        assert!(ResponseStream::default().push(b"data: broken\n\n").is_err());
    }
}
