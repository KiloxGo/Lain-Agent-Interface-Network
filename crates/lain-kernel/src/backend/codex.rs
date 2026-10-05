//! All Codex method names, JSON objects, and provider error details stay here.
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::LazyLock,
    time::Duration,
};

use anyhow::{Context, Result, bail, ensure};
use regex::Regex;
use serde_json::{Value, json};

use super::rpc::{self, Rpc};
use crate::{domain::*, events::EventKind};

pub(super) struct Codex {
    rpc: Rpc,
    pub identity: String,
    pending: BTreeMap<ApprovalId, PendingApproval>,
    redactor: Redactor,
}

struct PendingApproval {
    request_id: Value,
    permissions: Option<Value>,
}

pub(super) struct ObservedTurn {
    pub identity: String,
    pub state: TurnState,
    pub failure: Option<WorkerFailure>,
}
pub(super) struct Connected {
    pub worker: Codex,
    pub active: bool,
    pub history: Vec<ObservedTurn>,
}

// Opaque adapter message: supervision can select on reading without cancelling writes.
pub(super) struct Incoming(Value);

pub(super) enum Notice {
    Started(ObservedTurn),
    Finished(ObservedTurn),
    Approval {
        provider_turn_id: String,
        id: ApprovalId,
        kind: ApprovalKind,
        description: String,
        decisions: Vec<ApprovalDecision>,
    },
    ApprovalCleared(ApprovalId),
    Live {
        provider_turn_id: Option<String>,
        kind: EventKind,
    },
}

#[derive(Clone)]
pub(super) struct Redactor {
    known: Vec<String>,
}

impl Redactor {
    pub fn new(known: Vec<String>) -> Self {
        Self { known }
    }
    pub fn text(&self, text: &str) -> String {
        static SECRET: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?i)(sk-[a-z0-9_-]{12,}|bearer\s+\S+|(?:api[_-]?key|access[_-]?token|refresh[_-]?token|password|secret)\s*[:=]|-----BEGIN [A-Z ]*PRIVATE KEY-----)").unwrap()
        });
        if self
            .known
            .iter()
            .any(|secret| !secret.is_empty() && text.contains(secret))
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

pub(super) fn discover(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        ensure!(path.is_file(), "configured Codex executable is not a file");
        #[cfg(windows)]
        ensure!(
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("exe")),
            "configure the native codex.exe, not a shell shim"
        );
        return path.canonicalize().context("resolve Codex executable");
    }
    for directory in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
        #[cfg(windows)]
        let candidates = [directory.join("codex.exe"), directory.join("node_modules/@openai/codex/node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/bin/codex.exe")];
        #[cfg(not(windows))]
        let candidates = [directory.join("codex")];
        if let Some(path) = candidates.into_iter().find(|path| path.is_file()) {
            return path.canonicalize().context("resolve Codex executable");
        }
    }
    bail!("Codex executable not found; configure LAIN_CODEX_PATH")
}

pub(super) async fn probe(executable: &Path) -> Result<String> {
    let output = tokio::time::timeout(
        Duration::from_secs(10),
        rpc::command(executable).arg("--version").output(),
    )
    .await
    .context("Codex version probe timeout")??;
    ensure!(output.status.success(), "Codex version probe failed");
    let version = std::str::from_utf8(&output.stdout)?
        .trim()
        .strip_prefix("codex-cli ")
        .context("unrecognized Codex version response")?;
    ensure!(
        version == "0.160.0",
        "unsupported Codex version; adapter verified against 0.160.0"
    );
    Ok(version.to_owned())
}

fn identity(value: &Value) -> Result<String> {
    let id = value.as_str().context("missing provider identity")?;
    ensure!(
        !id.is_empty() && id.len() <= 256 && !id.chars().any(char::is_control),
        "invalid provider identity"
    );
    Ok(id.to_owned())
}

fn turn(value: &Value) -> Result<ObservedTurn> {
    let state = match value["status"].as_str() {
        Some("inProgress") => TurnState::Running,
        Some("completed") => TurnState::Completed,
        Some("interrupted") => TurnState::Cancelled,
        Some("failed") => TurnState::Failed,
        _ => bail!("unsupported worker turn status"),
    };
    Ok(ObservedTurn {
        identity: identity(&value["id"])?,
        state,
        failure: (!value["error"].is_null()).then(|| WorkerFailure {
            kind: FailureKind::Execution,
            message: "worker reported a turn error".into(),
            code: None,
        }),
    })
}

impl Codex {
    pub async fn connect(
        executable: &Path,
        workspace: &Path,
        options: &SessionOptions,
        previous: Option<&str>,
        redactor: Redactor,
    ) -> Result<Connected> {
        let rpc = Rpc::spawn(executable).await?;
        rpc.request("initialize", json!({"clientInfo":{"name":"lain","title":"Lain","version":env!("CARGO_PKG_VERSION")}})).await?;
        rpc.notify("initialized").await?;
        let mut params = json!({"cwd":workspace,"model":options.model,"approvalPolicy":"on-request","approvalsReviewer":"user","sandbox":match options.sandbox {SandboxMode::ReadOnly=>"read-only",SandboxMode::WorkspaceWrite=>"workspace-write"}});
        let method = if let Some(previous) = previous {
            params["threadId"] = json!(previous);
            "thread/resume"
        } else {
            params["ephemeral"] = json!(false);
            "thread/start"
        };
        let response = rpc.request(method, params).await?;
        // 0.160.0's default history initialization needs a full read before an
        // unused thread can be forked durably. Its unimplemented turns listing can
        // return -32601 after initialization; all other failures remain fatal.
        // Bind only the durable fork, without synthetic user turns/inference.
        let response = if previous.is_none() {
            let seed = identity(&response["thread"]["id"])?;
            if let Err(error) = rpc
                .request("thread/read", json!({"threadId":seed,"includeTurns":true}))
                .await
            {
                ensure!(
                    error
                        .downcast_ref::<rpc::Rejected>()
                        .is_some_and(|e| e.code == -32601),
                    error
                );
            }
            rpc.request(
                "thread/fork",
                json!({"threadId":seed,"cwd":workspace,"ephemeral":false}),
            )
            .await?
        } else {
            response
        };
        let thread = &response["thread"];
        let id = identity(&thread["id"])?;
        ensure!(
            previous.is_none_or(|previous| previous == id),
            "resume changed provider session identity"
        );
        let active = match thread["status"]["type"].as_str() {
            Some("active") => true,
            Some("idle") => false,
            _ => bail!("worker session is not ready"),
        };
        let history = thread["turns"]
            .as_array()
            .context("resume response missing turns")?
            .iter()
            .map(turn)
            .collect::<Result<Vec<_>>>()?;
        Ok(Connected {
            worker: Self {
                rpc,
                identity: id,
                pending: BTreeMap::new(),
                redactor,
            },
            active,
            history,
        })
    }

    pub async fn prompt(&self, prompt: &str) -> Result<ObservedTurn> {
        let result = self
            .rpc
            .request(
                "turn/start",
                json!({"threadId":self.identity,"input":[{"type":"text","text":prompt}]}),
            )
            .await?;
        turn(&result["turn"])
    }
    pub async fn cancel(&self, turn_id: &str) -> Result<()> {
        self.rpc
            .request(
                "turn/interrupt",
                json!({"threadId":self.identity,"turnId":turn_id}),
            )
            .await?;
        Ok(())
    }
    pub async fn decide(&self, id: ApprovalId, decision: ApprovalDecision) -> Result<()> {
        let pending = self
            .pending
            .get(&id)
            .context("approval no longer pending on this connection")?;
        let result = if let Some(permissions) = &pending.permissions {
            json!({"permissions":if decision == ApprovalDecision::ApproveOnce { permissions.clone() } else { json!({}) },"scope":"turn"})
        } else {
            json!({"decision":match decision { ApprovalDecision::ApproveOnce=>"accept",ApprovalDecision::Deny=>"decline",ApprovalDecision::Cancel=>"cancel" }})
        };
        self.rpc.reply(pending.request_id.clone(), result).await
    }
    pub async fn shutdown(&mut self) -> Result<()> {
        self.rpc.shutdown().await
    }

    pub async fn receive(&mut self) -> Result<Incoming> {
        self.rpc.next().await.map(Incoming)
    }

    pub async fn decode(&mut self, Incoming(message): Incoming) -> Result<Option<Notice>> {
        let method = message["method"]
            .as_str()
            .context("missing notification method")?;
        let p = &message["params"];
        if message.get("id").is_some() {
            let native = message["id"].clone();
            ensure!(
                native.is_string() || native.is_i64(),
                "invalid server request identity"
            );
            if p["threadId"].as_str() != Some(&self.identity) {
                self.rpc.unsupported(native).await?;
                return Ok(None);
            }
            let kind = match method {
                "item/commandExecution/requestApproval" => ApprovalKind::Command,
                "item/fileChange/requestApproval" => ApprovalKind::FileChange,
                "item/permissions/requestApproval" => ApprovalKind::Permissions,
                _ => {
                    self.rpc.unsupported(native).await?;
                    return Ok(Some(Notice::Live {
                        provider_turn_id: None,
                        kind: EventKind::ErrorObserved {
                            failure: WorkerFailure {
                                kind: FailureKind::Unsupported,
                                message: "unsupported worker request declined".into(),
                                code: Some(-32601),
                            },
                        },
                    }));
                }
            };
            ensure!(
                !self
                    .pending
                    .values()
                    .any(|pending| pending.request_id == native),
                "duplicate server request identity"
            );
            let id = ApprovalId::new();
            let permissions = (kind == ApprovalKind::Permissions).then(|| p["permissions"].clone());
            let description = match kind {
                ApprovalKind::Command => format!(
                    "Command: {}\nDirectory: {}\nReason: {}\nNetwork: {}",
                    p["command"].as_str().unwrap_or("(not provided)"),
                    p["cwd"].as_str().unwrap_or("(not provided)"),
                    p["reason"].as_str().unwrap_or(""),
                    p.get("networkApprovalContext").unwrap_or(&Value::Null)
                ),
                ApprovalKind::FileChange => format!(
                    "File changes. Grant root: {}. Reason: {}",
                    p["grantRoot"].as_str().unwrap_or("workspace"),
                    p["reason"].as_str().unwrap_or("")
                ),
                ApprovalKind::Permissions => format!(
                    "Requested turn permissions: {}. Reason: {}",
                    p["permissions"],
                    p["reason"].as_str().unwrap_or("")
                ),
            };
            let mut decisions = vec![ApprovalDecision::ApproveOnce, ApprovalDecision::Deny];
            if kind != ApprovalKind::Permissions {
                decisions.push(ApprovalDecision::Cancel);
            }
            if let Some(available) = p["availableDecisions"].as_array() {
                decisions.retain(|decision| {
                    available.contains(&json!(match decision {
                        ApprovalDecision::ApproveOnce => "accept",
                        ApprovalDecision::Deny => "decline",
                        ApprovalDecision::Cancel => "cancel",
                    }))
                });
            }
            ensure!(!decisions.is_empty(), "unsupported approval decisions");
            self.pending.insert(
                id,
                PendingApproval {
                    request_id: native,
                    permissions,
                },
            );
            let mut description = self.redactor.text(&description);
            if description.len() > 16 * 1024 {
                let mut end = 16 * 1024 - 32;
                while !description.is_char_boundary(end) {
                    end -= 1;
                }
                description.truncate(end);
                description.push_str("\n[description truncated]");
            }
            return Ok(Some(Notice::Approval {
                provider_turn_id: identity(&p["turnId"])?,
                id,
                kind,
                description,
                decisions,
            }));
        }
        if p["threadId"].as_str() != Some(&self.identity) {
            return Ok(None);
        }
        let provider_turn_id = p["turnId"].as_str().map(str::to_owned);
        let kind =
            match method {
                "turn/started" => return Ok(Some(Notice::Started(turn(&p["turn"])?))),
                "turn/completed" => return Ok(Some(Notice::Finished(turn(&p["turn"])?))),
                "serverRequest/resolved" => {
                    if let Some(id) = self.pending.iter().find_map(|(id, pending)| {
                        (pending.request_id == p["requestId"]).then_some(*id)
                    }) {
                        self.pending.remove(&id);
                        return Ok(Some(Notice::ApprovalCleared(id)));
                    }
                    return Ok(None);
                }
                "item/agentMessage/delta" | "item/commandExecution/outputDelta" => {
                    EventKind::OutputProgress {
                        category: if method.contains("agentMessage") {
                            "assistant"
                        } else {
                            "command"
                        }
                        .into(),
                        characters: p["delta"].as_str().unwrap_or_default().chars().count(),
                    }
                }
                "item/started" | "item/completed" => {
                    let item = &p["item"];
                    let completed = method == "item/completed";
                    match item["type"].as_str() {
                        Some("agentMessage") if completed => EventKind::AssistantMessageCompleted {
                            text: self
                                .redactor
                                .text(item["text"].as_str().context("missing assistant text")?),
                        },
                        Some("commandExecution") => {
                            let command = self
                                .redactor
                                .text(item["command"].as_str().unwrap_or_default());
                            if completed {
                                EventKind::CommandCompleted {
                                    command,
                                    output: self.redactor.text(
                                        item["aggregatedOutput"].as_str().unwrap_or_default(),
                                    ),
                                    exit_code: item["exitCode"].as_i64(),
                                }
                            } else {
                                EventKind::CommandStarted { command }
                            }
                        }
                        Some("fileChange") if completed && item["status"] == "completed" => {
                            EventKind::FileChanged {
                                paths: item["changes"]
                                    .as_array()
                                    .context("missing file changes")?
                                    .iter()
                                    .filter_map(|change| change["path"].as_str())
                                    .map(|path| self.redactor.text(path))
                                    .collect(),
                            }
                        }
                        Some("mcpToolCall") => EventKind::ToolActivity {
                            name: self
                                .redactor
                                .text(item["tool"].as_str().unwrap_or("external tool")),
                            completed,
                        },
                        _ => return Ok(None),
                    }
                }
                "error" => EventKind::ErrorObserved {
                    failure: WorkerFailure {
                        kind: FailureKind::Execution,
                        message: if p["willRetry"].as_bool() == Some(true) {
                            "worker error; backend will retry"
                        } else {
                            "worker reported an error"
                        }
                        .into(),
                        code: None,
                    },
                },
                "thread/closed" => bail!("worker closed the session"),
                _ => return Ok(None),
            };
        Ok(Some(Notice::Live {
            provider_turn_id,
            kind,
        }))
    }
}

#[cfg(test)]
#[tokio::test]
#[ignore = "manual real Codex protocol probe; no inference"]
async fn probe_unused_thread_persistence() {
    let path = discover(None).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let options = SessionOptions::default();
    let mut created = Codex::connect(&path, dir.path(), &options, None, Redactor::new(vec![]))
        .await
        .unwrap();
    let identity = created.worker.identity.clone();
    assert!(created.history.is_empty());
    created.worker.shutdown().await.unwrap();
    let mut resumed = Codex::connect(
        &path,
        dir.path(),
        &options,
        Some(&identity),
        Redactor::new(vec![]),
    )
    .await
    .unwrap();
    assert_eq!(resumed.worker.identity, identity);
    assert!(resumed.history.is_empty());
    resumed.worker.shutdown().await.unwrap();
}
