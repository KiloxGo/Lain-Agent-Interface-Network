//! All Codex method names, JSON objects, and provider error details stay here.
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};

use super::rpc::{self, Rpc};
use crate::profiles::{ApprovalPolicy, ResolvedRuntimeProfile, SecretValue};
pub(super) use crate::redaction::Redactor;
use crate::{domain::*, events::EventKind};

/// Validated adapter-owned translation; secret bytes never appear in argv.
pub(super) struct CodexLaunchSpec {
    config: Vec<String>,
    credential: Option<SecretValue>,
}
impl CodexLaunchSpec {
    pub fn new(profile: &ResolvedRuntimeProfile, credential: Option<SecretValue>) -> Result<Self> {
        profile.validate()?;
        ensure!(
            credential.is_some() == profile.secret_ref.is_some(),
            "profile credential resolution mismatch"
        );
        let mut config = Vec::new();
        let mut set = |key: &str, value: &str| {
            config.push(format!(
                "{key}={}",
                serde_json::to_string(value).expect("string encoding")
            ));
        };
        if let Some(model) = &profile.model {
            set("model", model);
        }
        if let Some(effort) = &profile.reasoning_effort {
            set("model_reasoning_effort", effort);
        }
        if let Some(provider) = &profile.provider {
            set("model_provider", "lain_runtime");
            set("model_providers.lain_runtime.name", provider);
            set(
                "model_providers.lain_runtime.base_url",
                profile
                    .base_url
                    .as_deref()
                    .unwrap_or("https://api.openai.com/v1"),
            );
            set(
                "model_providers.lain_runtime.env_key",
                "LAIN_PROFILE_CREDENTIAL",
            );
            set("model_providers.lain_runtime.wire_api", "responses");
            config.extend([
                "model_providers.lain_runtime.requires_openai_auth=false".into(),
                "model_providers.lain_runtime.supports_websockets=false".into(),
                "shell_environment_policy.exclude=[\"LAIN_PROFILE_CREDENTIAL\"]".into(),
            ]);
        }
        Ok(Self { config, credential })
    }
    pub fn apply(&self, command: &mut tokio::process::Command) {
        for setting in &self.config {
            command.arg("-c").arg(setting);
        }
        if let Some(value) = &self.credential {
            command.env("LAIN_PROFILE_CREDENTIAL", value.0.as_str());
        }
    }
}

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
    pub resolved: ResolvedRuntimeProfile,
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

#[derive(Debug, thiserror::Error)]
#[error("unsupported Codex version; adapter verified against 0.160.0")]
pub(super) struct UnsupportedVersion;
fn verify_version(version: &str) -> Result<()> {
    ensure!(version == "0.160.0", UnsupportedVersion);
    Ok(())
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
    verify_version(version)?;
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

fn normalized_failure(error: &Value, retryable: Option<bool>) -> WorkerFailure {
    let info = &error["codexErrorInfo"];
    let connection = info
        .as_object()
        .filter(|o| o.len() == 1)
        .and_then(|o| o.iter().next())
        .filter(|(key, _)| {
            matches!(
                key.as_str(),
                "httpConnectionFailed"
                    | "responseStreamConnectionFailed"
                    | "responseStreamDisconnected"
                    | "responseTooManyFailedAttempts"
            )
        })
        .and_then(|(_, v)| v.as_object())
        .filter(|v| {
            v.get("httpStatusCode")
                .is_none_or(|s| s.is_null() || s.as_u64().is_some_and(|n| (100..=599).contains(&n)))
        });
    let status = connection
        .and_then(|v| v.get("httpStatusCode"))
        .and_then(Value::as_u64);
    let kind = match (info.as_str(), status) {
        (Some("unauthorized"), _) | (_, Some(401)) => FailureKind::AuthenticationFailed,
        (Some("rateLimitExceeded" | "usageLimitExceeded"), _) | (_, Some(429)) => {
            FailureKind::RateLimited
        }
        (Some("sandboxError" | "tooManyDenials" | "cyberPolicy"), _) | (_, Some(403)) => {
            FailureKind::PermissionDenied
        }
        (Some("serverOverloaded" | "flexUnavailable" | "internalServerError"), _)
        | (_, Some(500..=599)) => FailureKind::ProviderUnavailable,
        (_, _) if connection.is_some() => FailureKind::ProviderUnavailable,
        (
            Some(
                "badRequest"
                | "contextWindowExceeded"
                | "sessionBudgetExceeded"
                | "misalignmentPolicyViolation",
            ),
            _,
        ) => FailureKind::Execution,
        _ => FailureKind::Unknown,
    };
    let description = match kind {
        FailureKind::AuthenticationFailed => {
            "worker authentication failed; check the profile authentication source"
        }
        FailureKind::RateLimited => "worker provider rate or usage limit reached",
        FailureKind::PermissionDenied => "worker operation denied by permissions or policy",
        FailureKind::ProviderUnavailable => {
            "worker provider unavailable or response connection lost"
        }
        FailureKind::Execution => "worker execution failed",
        _ => "worker reported an unclassified error",
    };
    WorkerFailure {
        kind,
        message: format!(
            "{description}{}",
            if retryable == Some(true) {
                "; backend will retry"
            } else {
                ""
            }
        ),
        code: status.map(|n| n as i64),
        retryable,
    }
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
        failure: (!value["error"].is_null() || state == TurnState::Failed)
            .then(|| normalized_failure(&value["error"], Some(false))),
    })
}

impl Codex {
    pub async fn connect(
        executable: &Path,
        workspace: &Path,
        profile: &ResolvedRuntimeProfile,
        launch: CodexLaunchSpec,
        previous: Option<&str>,
        redactor: Redactor,
    ) -> Result<Connected> {
        let rpc = Rpc::spawn(executable, &launch).await?;
        drop(launch);
        rpc.request("initialize", json!({"clientInfo":{"name":"lain","title":"Lain","version":env!("CARGO_PKG_VERSION")}})).await?;
        rpc.notify("initialized").await?;
        let mut params = json!({"cwd":workspace,"model":profile.model,"approvalPolicy":match profile.approval_policy { ApprovalPolicy::OnRequest=>"on-request", ApprovalPolicy::Untrusted=>"untrusted" },"approvalsReviewer":"user","sandbox":match profile.sandbox {SandboxMode::ReadOnly=>"read-only",SandboxMode::WorkspaceWrite=>"workspace-write"}});
        if profile.provider.is_some() {
            params["modelProvider"] = json!("lain_runtime");
        }
        if let Some(effort) = &profile.reasoning_effort {
            params["config"] = json!({"model_reasoning_effort":effort});
        }
        let method = if let Some(previous) = previous {
            params["threadId"] = json!(previous);
            "thread/resume"
        } else {
            params["ephemeral"] = json!(false);
            "thread/start"
        };
        let response = rpc.request(method, params.clone()).await?;
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
            params["threadId"] = json!(seed);
            rpc.request("thread/fork", params).await?
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
        let mut resolved = profile.clone();
        ensure!(
            response["model"]
                .as_str()
                .is_some_and(crate::profiles::identifier),
            "worker omitted effective model"
        );
        ensure!(
            profile
                .model
                .as_deref()
                .is_none_or(|model| response["model"] == model),
            "worker did not apply requested model"
        );
        ensure!(
            profile
                .reasoning_effort
                .as_deref()
                .is_none_or(|effort| response["reasoningEffort"] == effort),
            "worker did not apply requested reasoning effort"
        );
        if resolved.model.is_none() {
            resolved.model = response["model"]
                .as_str()
                .filter(|s| crate::profiles::identifier(s))
                .map(str::to_owned);
        }
        if resolved.reasoning_effort.is_none() {
            resolved.reasoning_effort = response["reasoningEffort"].as_str().map(str::to_owned);
        }
        resolved.validate()?;
        ensure!(
            redactor.text(&serde_json::to_string(&(
                &resolved.model,
                &resolved.reasoning_effort
            ))?) == serde_json::to_string(&(&resolved.model, &resolved.reasoning_effort))?,
            "sensitive effective configuration rejected"
        );
        Ok(Connected {
            worker: Self {
                rpc,
                identity: id,
                pending: BTreeMap::new(),
                redactor,
            },
            active,
            history,
            resolved,
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
                                retryable: Some(false),
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
                    failure: normalized_failure(&p["error"], p["willRetry"].as_bool()),
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
    let options = ResolvedRuntimeProfile::legacy(&SessionOptions::default()).unwrap();
    let mut created = Codex::connect(
        &path,
        dir.path(),
        &options,
        CodexLaunchSpec::new(&options, None).unwrap(),
        None,
        Redactor::new(vec![]),
    )
    .await
    .unwrap();
    let identity = created.worker.identity.clone();
    assert!(created.history.is_empty());
    let options = created.resolved.clone();
    created.worker.shutdown().await.unwrap();
    let mut resumed = Codex::connect(
        &path,
        dir.path(),
        &options,
        CodexLaunchSpec::new(&options, None).unwrap(),
        Some(&identity),
        Redactor::new(vec![]),
    )
    .await
    .unwrap();
    assert_eq!(resumed.worker.identity, identity);
    assert_eq!(resumed.resolved, options);
    assert!(resumed.history.is_empty());
    resumed.worker.shutdown().await.unwrap();
}

#[cfg(test)]
#[test]
fn error_normalization_never_uses_provider_prose() {
    for error in [
        Value::Null,
        json!("raw-key"),
        json!({"message":"raw-key","codexErrorInfo":"futureUnknownKind"}),
        json!({"codexErrorInfo":{"unexpected":{"httpStatusCode":401}}}),
        json!({"codexErrorInfo":{"httpConnectionFailed":"raw-key"}}),
        json!({"codexErrorInfo":{"httpConnectionFailed":{"httpStatusCode":"401"}}}),
        json!({"codexErrorInfo":{"httpConnectionFailed":{"httpStatusCode":999}}}),
        json!({"codexErrorInfo":{"httpConnectionFailed":{},"unexpected":{}}}),
    ] {
        let failure = normalized_failure(&error, None);
        assert_eq!(failure.kind, FailureKind::Unknown);
        assert!(!failure.message.contains("raw-key"));
    }
    let failure = normalized_failure(
        &json!({"codexErrorInfo":{"responseStreamDisconnected":{"httpStatusCode":429}},"message":"do-not-persist"}),
        Some(true),
    );
    assert_eq!(failure.kind, FailureKind::RateLimited);
    assert_eq!(failure.code, Some(429));
    assert_eq!(failure.retryable, Some(true));
    let old: WorkerFailure =
        serde_json::from_value(json!({"kind":"execution","message":"old record","code":null}))
            .unwrap();
    assert_eq!(old.retryable, None);
    let secret = SecretValue(zeroize::Zeroizing::new("fixture-private".into()));
    assert!(!format!("{secret:?}").contains("fixture-private"));
}

#[cfg(test)]
#[tokio::test]
#[ignore = "native Codex profile config verification; no inference"]
async fn probe_profile_launch_configuration() {
    use crate::profiles::{ProfileDefinition, RuntimeProfile};
    let path = discover(None).unwrap();
    assert_eq!(probe(&path).await.unwrap(), "0.160.0");
    let mut processes = Vec::new();
    for (name, model, endpoint) in [
        ("a", "fixture-model-a", "http://127.0.0.1:39001/v1"),
        ("b", "fixture-model-b", "http://127.0.0.1:39002/v1"),
    ] {
        let definition = ProfileDefinition {
            id: format!("codex/{name}").parse().unwrap(),
            model: Some(model.into()),
            provider: Some(format!("fixture-{name}")),
            base_url: Some(endpoint.into()),
            secret_ref: Some(format!("secret://env/fixture-{name}").parse().unwrap()),
            reasoning_effort: Some("high".into()),
            ..ProfileDefinition::local()
        };
        let profile = RuntimeProfile {
            definition,
            revision: 1,
        }
        .resolve(&SessionOptions::default())
        .unwrap();
        let spec = CodexLaunchSpec::new(
            &profile,
            Some(SecretValue(zeroize::Zeroizing::new(format!(
                "dummy-private-{name}"
            )))),
        )
        .unwrap();
        let rpc = Rpc::spawn(&path, &spec).await.unwrap();
        drop(spec);
        rpc.request(
            "initialize",
            json!({"clientInfo":{"name":"lain-config-probe","version":"0.1.0"}}),
        )
        .await
        .unwrap();
        rpc.notify("initialized").await.unwrap();
        let response = rpc
            .request("config/read", json!({"includeLayers":false}))
            .await
            .unwrap();
        assert_eq!(response["config"]["model"], model);
        assert_eq!(response["config"]["model_provider"], "lain_runtime");
        assert_eq!(response["config"]["model_reasoning_effort"], "high");
        assert_eq!(
            response["config"]["model_providers"]["lain_runtime"]["base_url"],
            endpoint
        );
        let provider = &response["config"]["model_providers"]["lain_runtime"];
        assert_eq!(provider["env_key"], "LAIN_PROFILE_CREDENTIAL");
        assert_eq!(provider["wire_api"], "responses");
        assert_eq!(provider["requires_openai_auth"], false);
        assert_eq!(provider["supports_websockets"], false);
        assert_eq!(
            response["config"]["shell_environment_policy"]["exclude"],
            json!(["LAIN_PROFILE_CREDENTIAL"])
        );
        processes.push(rpc);
    }
    for mut rpc in processes {
        rpc.shutdown().await.unwrap();
    }
}

#[test]
fn version_diagnostics_distinguish_unsupported_without_echoing_probe_content() {
    assert!(verify_version("0.160.0").is_ok());
    let error = verify_version("unexpected-private-diagnostic").unwrap_err();
    assert!(error.is::<UnsupportedVersion>());
    assert!(!error.to_string().contains("unexpected-private-diagnostic"));
}
