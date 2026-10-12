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
pub trait ActorModel: Send + Sync {
    fn respond(&self, request: Request)
    -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>>;
}
pub struct ResponsesModel {
    config: ModelConfig,
    http: reqwest::Client,
}
impl ResponsesModel {
    pub fn new(config: ModelConfig) -> Result<Self> {
        config.validate()?;
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .context("construct Actor HTTP client")?;
        Ok(Self { config, http })
    }
}
impl ActorModel for ResponsesModel {
    fn respond(
        &self,
        request: Request,
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
                .map_err(|_| {
                    anyhow::anyhow!("Actor provider connection failed; inference may be uncertain")
                })?;
            ensure!(
                response.status().is_success(),
                "Actor provider rejected request (HTTP {}); check model, credential, endpoint and quota",
                response.status().as_u16()
            );
            // Consume genuine provider streaming, but publish only complete redacted records.
            // Raw fragments could split a credential across frames and are never stored/exposed.
            let mut parser = ResponseStream::default();
            while let Some(chunk) = response.chunk().await.map_err(|_| {
                anyhow::anyhow!("Actor provider stream disconnected; inference uncertain")
            })? {
                if let Some(reply) = parser.push(&chunk)? {
                    return Ok(reply);
                }
            }
            anyhow::bail!("Actor provider stream ended without completion; inference uncertain")
        })
    }
}
#[derive(Default)]
struct ResponseStream {
    pending: Vec<u8>,
    total: usize,
}
impl ResponseStream {
    fn push(&mut self, bytes: &[u8]) -> Result<Option<Reply>> {
        self.total += bytes.len();
        ensure!(
            self.total <= 2 * 1024 * 1024,
            "Actor response exceeded bounds"
        );
        self.pending.extend_from_slice(bytes);
        ensure!(
            self.pending.len() <= 256 * 1024,
            "Actor stream frame exceeded bounds"
        );
        while let Some(end) = self.pending.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=end).collect();
            let line = std::str::from_utf8(&line)
                .context("invalid Actor stream encoding")?
                .trim_end_matches(['\r', '\n']);
            let Some(data) = line.strip_prefix("data:") else {
                continue;
            };
            let data = data.trim();
            if data == "[DONE]" {
                continue;
            }
            let event: Value = serde_json::from_str(data)
                .map_err(|_| anyhow::anyhow!("invalid Actor stream frame"))?;
            match event["type"].as_str() {
                Some("response.completed") => return parse_response(&event["response"]).map(Some),
                Some("response.failed" | "response.incomplete" | "error") => {
                    anyhow::bail!("Actor provider did not complete the response")
                }
                _ => {}
            }
        }
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
                            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b)),
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
        text.len() <= 16384 && calls.len() <= 1 && (!text.trim().is_empty() || !calls.is_empty()),
        "Actor reply empty or exceeds bounds"
    );
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
        assert!(ResponseStream::default().push(b"data: broken\n").is_err());
    }
}
