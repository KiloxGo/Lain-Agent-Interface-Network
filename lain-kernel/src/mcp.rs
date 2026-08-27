use std::{io, process::Stdio, time::Duration};

use serde_json::{Value, json};
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::Mutex,
    time::timeout,
};

use crate::{config::McpConfig, tools::sensitive_env_name};

const MODERN_PROTOCOL_VERSION: &str = "2026-07-28";
const LEGACY_PROTOCOL_VERSION: &str = "2025-11-25";
const LEGACY_PROTOCOL_VERSIONS: [&str; 4] =
    ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];
const MESSAGE_LIMIT: usize = 64 * 1024;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(3);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_SKIPPED_MESSAGES: usize = 32;

#[derive(Debug, Clone, PartialEq)]
pub struct McpConnectionInfo {
    pub namespace: String,
    pub protocol_version: String,
    pub server_name: Option<String>,
    pub server_version: Option<String>,
    pub capabilities: Value,
}

pub struct McpClient {
    info: McpConnectionInfo,
    process: Mutex<Option<McpProcess>>,
}

impl std::fmt::Debug for McpClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("McpClient")
            .field("info", &self.info)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Error)]
pub enum McpError {
    #[error("MCP process failed: {0}")]
    Io(#[from] io::Error),
    #[error("MCP message is invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("MCP process did not expose {0}")]
    MissingPipe(&'static str),
    #[error("MCP message exceeded 64 KiB")]
    MessageTooLarge,
    #[error("MCP server closed stdout")]
    Closed,
    #[error("MCP request timed out")]
    Timeout,
    #[error("MCP server returned JSON-RPC error {code}: {message}")]
    Remote { code: i64, message: String },
    #[error("MCP server returned an invalid response: {0}")]
    InvalidResponse(&'static str),
    #[error("MCP server does not support a compatible protocol version")]
    UnsupportedVersion,
}

struct McpProcess {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl McpClient {
    pub async fn connect(config: &McpConfig) -> Result<Self, McpError> {
        let mut command = Command::new(&config.command);
        command
            .args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        for (name, _) in std::env::vars_os() {
            if sensitive_env_name(&name.to_string_lossy()) {
                command.env_remove(name);
            }
        }
        let mut child = command.spawn()?;
        let stdin = child.stdin.take().ok_or(McpError::MissingPipe("stdin"))?;
        let stdout = child.stdout.take().ok_or(McpError::MissingPipe("stdout"))?;
        let mut process = McpProcess {
            child,
            stdin: Some(stdin),
            stdout: BufReader::new(stdout),
        };

        let info = match negotiate(&mut process, &config.name).await {
            Ok(info) => info,
            Err(error) => {
                stop_process(&mut process).await;
                return Err(error);
            }
        };
        Ok(Self {
            info,
            process: Mutex::new(Some(process)),
        })
    }

    pub fn info(&self) -> &McpConnectionInfo {
        &self.info
    }

    pub async fn close(&self) {
        if let Some(mut process) = self.process.lock().await.take() {
            stop_process(&mut process).await;
        }
    }
}

async fn negotiate(
    process: &mut McpProcess,
    namespace: &str,
) -> Result<McpConnectionInfo, McpError> {
    let discovery = request(
        process,
        1,
        "server/discover",
        json!({"_meta": modern_meta()}),
    )
    .await;
    match discovery {
        Ok(result) => parse_discovery(namespace, result),
        Err(error) if error.allows_legacy_fallback() => initialize_legacy(process, namespace).await,
        Err(error) => Err(error),
    }
}

async fn initialize_legacy(
    process: &mut McpProcess,
    namespace: &str,
) -> Result<McpConnectionInfo, McpError> {
    let result = request(
        process,
        2,
        "initialize",
        json!({
            "protocolVersion": LEGACY_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": client_info(),
        }),
    )
    .await?;
    let protocol_version = result
        .get("protocolVersion")
        .and_then(Value::as_str)
        .filter(|version| LEGACY_PROTOCOL_VERSIONS.contains(version))
        .ok_or(McpError::UnsupportedVersion)?;
    let capabilities = object_field(&result, "capabilities")?.clone();
    let (server_name, server_version) = implementation_fields(result.get("serverInfo"));
    send(
        process,
        &json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
    )
    .await?;
    Ok(McpConnectionInfo {
        namespace: namespace.to_string(),
        protocol_version: protocol_version.to_string(),
        server_name,
        server_version,
        capabilities,
    })
}

fn parse_discovery(namespace: &str, result: Value) -> Result<McpConnectionInfo, McpError> {
    if result.get("resultType").and_then(Value::as_str) != Some("complete") {
        return Err(McpError::InvalidResponse("discovery did not complete"));
    }
    let supported = result
        .get("supportedVersions")
        .and_then(Value::as_array)
        .ok_or(McpError::InvalidResponse("missing supportedVersions"))?;
    if !supported
        .iter()
        .any(|version| version.as_str() == Some(MODERN_PROTOCOL_VERSION))
    {
        return Err(McpError::UnsupportedVersion);
    }
    let capabilities = object_field(&result, "capabilities")?.clone();
    let server_info = result
        .get("_meta")
        .and_then(|meta| meta.get("io.modelcontextprotocol/serverInfo"));
    let (server_name, server_version) = implementation_fields(server_info);
    Ok(McpConnectionInfo {
        namespace: namespace.to_string(),
        protocol_version: MODERN_PROTOCOL_VERSION.to_string(),
        server_name,
        server_version,
        capabilities,
    })
}

async fn request(
    process: &mut McpProcess,
    id: i64,
    method: &str,
    params: Value,
) -> Result<Value, McpError> {
    send(
        process,
        &json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}),
    )
    .await?;
    timeout(HANDSHAKE_TIMEOUT, read_response(process, id))
        .await
        .map_err(|_| McpError::Timeout)?
}

async fn read_response(process: &mut McpProcess, id: i64) -> Result<Value, McpError> {
    for _ in 0..MAX_SKIPPED_MESSAGES {
        let line = read_frame(&mut process.stdout).await?;
        let message: Value = serde_json::from_slice(&line)?;
        if message.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            return Err(McpError::InvalidResponse("invalid jsonrpc version"));
        }
        if message.get("id").and_then(Value::as_i64) != Some(id) {
            continue;
        }
        if let Some(error) = message.get("error") {
            return Err(McpError::Remote {
                code: error.get("code").and_then(Value::as_i64).unwrap_or(-32603),
                message: error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown error")
                    .to_string(),
            });
        }
        return message
            .get("result")
            .cloned()
            .ok_or(McpError::InvalidResponse("missing result"));
    }
    Err(McpError::InvalidResponse("too many unrelated messages"))
}

async fn send(process: &mut McpProcess, message: &Value) -> Result<(), McpError> {
    let mut encoded = serde_json::to_vec(message)?;
    if encoded.len() > MESSAGE_LIMIT {
        return Err(McpError::MessageTooLarge);
    }
    encoded.push(b'\n');
    let stdin = process.stdin.as_mut().ok_or(McpError::Closed)?;
    stdin.write_all(&encoded).await?;
    stdin.flush().await?;
    Ok(())
}

async fn read_frame(reader: &mut BufReader<ChildStdout>) -> Result<Vec<u8>, McpError> {
    let mut frame = Vec::new();
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            return Err(McpError::Closed);
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let take = newline.unwrap_or(available.len());
        if frame.len().saturating_add(take) > MESSAGE_LIMIT {
            return Err(McpError::MessageTooLarge);
        }
        frame.extend_from_slice(&available[..take]);
        reader.consume(take + usize::from(newline.is_some()));
        if newline.is_some() {
            return Ok(frame);
        }
    }
}

async fn stop_process(process: &mut McpProcess) {
    drop(process.stdin.take());
    if timeout(SHUTDOWN_TIMEOUT, process.child.wait())
        .await
        .is_err()
    {
        let _ = process.child.kill().await;
        let _ = process.child.wait().await;
    }
}

fn modern_meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": MODERN_PROTOCOL_VERSION,
        "io.modelcontextprotocol/clientInfo": client_info(),
        "io.modelcontextprotocol/clientCapabilities": {},
    })
}

fn client_info() -> Value {
    json!({"name":"lain-kernel","version":env!("CARGO_PKG_VERSION")})
}

fn object_field<'a>(value: &'a Value, name: &'static str) -> Result<&'a Value, McpError> {
    value
        .get(name)
        .filter(|field| field.is_object())
        .ok_or(McpError::InvalidResponse(name))
}

fn implementation_fields(value: Option<&Value>) -> (Option<String>, Option<String>) {
    let field = |name| {
        value
            .and_then(|implementation| implementation.get(name))
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    (field("name"), field("version"))
}

impl McpError {
    fn allows_legacy_fallback(&self) -> bool {
        match self {
            Self::Timeout => true,
            Self::Remote { code, .. } => *code != -32022,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[tokio::test]
    async fn negotiates_modern_and_legacy_stdio_lifecycles() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../evals/fixtures/mcp-stdio-server.mjs");
        for (mode, expected) in [
            ("modern", MODERN_PROTOCOL_VERSION),
            ("legacy", LEGACY_PROTOCOL_VERSION),
        ] {
            let client = McpClient::connect(&McpConfig {
                name: "fixture".to_string(),
                command: "node".to_string(),
                args: vec![fixture.to_string_lossy().into_owned(), mode.to_string()],
            })
            .await
            .expect("connect fixture");
            assert_eq!(client.info().protocol_version, expected);
            assert_eq!(client.info().server_name.as_deref(), Some("fixture-server"));
            assert!(client.info().capabilities.get("tools").is_some());
            client.close().await;
        }
    }

    #[test]
    fn explicit_version_errors_do_not_fall_back() {
        assert!(
            !McpError::Remote {
                code: -32022,
                message: "unsupported".to_string(),
            }
            .allows_legacy_fallback()
        );
        assert!(
            McpError::Remote {
                code: -32601,
                message: "unknown method".to_string(),
            }
            .allows_legacy_fallback()
        );
    }
}
