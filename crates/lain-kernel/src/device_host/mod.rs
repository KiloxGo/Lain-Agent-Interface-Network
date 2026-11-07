//! Native device execution; owns no Actor, channel, or cloud administrator API.
mod cloud;
mod diagnostics;
mod journal;
#[cfg(test)]
mod tests;
mod view;

use crate::{
    backend::Supervisor,
    domain,
    domain::{ApprovalDecision, ApprovalStatus, CancellationState, TurnState},
    kernel::Kernel,
    redaction::Redactor,
};
use anyhow::{Context, Result, ensure};
use journal::Journal;
use lain_wire::{
    DeviceCommandId, DeviceExecutionId, DeviceId, RequestId, device_commands::*,
    device_execution::*, devices::*,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::time::Instant;

#[derive(Debug, PartialEq, Eq)]
enum RecoveryPlan {
    Launch,
    ResumeThenSubmit,
    ResumeObserve,
    Observe,
    Unknown,
}
fn recovery_plan(
    launched: bool,
    submitted: bool,
    bound: bool,
    mapped: bool,
    terminal: bool,
    verified_idle_parent: bool,
) -> RecoveryPlan {
    if terminal {
        RecoveryPlan::Observe
    } else if !launched && !submitted && (!bound || verified_idle_parent) {
        RecoveryPlan::Launch
    } else if launched && bound && !submitted {
        RecoveryPlan::ResumeThenSubmit
    } else if launched && submitted && bound && mapped {
        RecoveryPlan::ResumeObserve
    } else {
        RecoveryPlan::Unknown
    }
}
fn safe_text(redactor: &Redactor, text: &str, max: usize, tail: bool) -> (String, bool) {
    let redacted = redactor.text(text);
    let mut clean: String = redacted
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
        .collect();
    let changed = clean != text || clean.len() > max;
    if clean.len() > max {
        if tail {
            let mut start = clean.len() - max;
            while !clean.is_char_boundary(start) {
                start += 1;
            }
            clean = clean[start..].into();
        } else {
            let mut end = max;
            while !clean.is_char_boundary(end) {
                end -= 1;
            }
            clean.truncate(end);
        }
    }
    (clean, changed)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Credential {
    version: u8,
    endpoint: String,
    device_id: DeviceId,
    claim_request_id: RequestId,
    token: String,
}
impl Drop for Credential {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.token.zeroize();
    }
}

fn settings(directory: &Path) -> Result<(Credential, RegisterDevice, DeviceHostPolicy)> {
    ensure!(directory.is_absolute(), "device directory must be absolute");
    lain_local::outside_checkout(directory)?;
    lain_local::private(directory, true, false)?;
    let credential: Credential = journal::read(&directory.join("device-credential.json"))?;
    let registration: RegisterDevice = journal::read(&directory.join("registration.json"))?;
    let policy: DeviceHostPolicy = journal::read(&directory.join("host-policy.json"))?;
    let platform = match std::env::consts::OS {
        "windows" => DevicePlatform::Windows,
        "macos" => DevicePlatform::Macos,
        "linux" => DevicePlatform::Linux,
        _ => anyhow::bail!("unsupported native device"),
    };
    ensure!(
        credential.version == 1
            && registration.valid()
            && credential.device_id == registration.id
            && registration.platform == platform
            && device_credential_fingerprint(&credential.token).as_ref()
                == Some(&registration.credential_fingerprint),
        "device identity/manifest mismatch"
    );
    let _claim_identity = credential.claim_request_id;
    ensure!(policy.valid(), "invalid native host policy");
    let executable = PathBuf::from(&policy.codex_path);
    ensure!(
        executable.is_absolute()
            && executable.is_file()
            && executable.canonicalize()?.to_str() == Some(policy.codex_path.as_str()),
        "native executable identity differs"
    );
    Ok((credential, registration, policy))
}

pub use diagnostics::check;

pub async fn serve(directory: PathBuf, shutdown: impl Future<Output = ()>) -> Result<()> {
    let (credential, registration, policy) = settings(&directory)?;
    let executable = PathBuf::from(&policy.codex_path);
    let root = directory.join("execution");
    journal::directory(&root)?;
    let lock = root.join("host.lock");
    if !lock.exists() {
        lain_local::create_private(&lock, b"")?;
    }
    lain_local::private(&lock, false, false)?;
    let guard = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(lock)?;
    guard
        .try_lock()
        .context("another native host owns this device directory")?;
    let cloud = cloud::Cloud::new(&credential.endpoint, &credential.token, registration.id)?;
    let redactor = Redactor::new(vec![credential.token.clone()]).for_runtime(None);
    let kernel = Arc::new(Kernel::open(&root.join("worker.db"))?);
    kernel.mark_ready()?;
    let token = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
    .parse()?;
    let workers =
        Supervisor::new_device(kernel.clone(), executable, token, credential.token.clone()).await;
    let info = workers.info();
    ensure!(
        info.available
            && ["session_resume", "turns", "cancel_turn", "approvals"]
                .iter()
                .all(|capability| info.capabilities.contains(capability)),
        "native Codex is unavailable or unsupported; version gate unchanged"
    );
    let host = Host {
        directory,
        root,
        registration,
        policy,
        cloud,
        kernel: kernel.clone(),
        workers: workers.clone(),
        redactor,
        active: std::sync::Mutex::new(None),
    };
    tokio::pin!(shutdown);
    let result = tokio::select! {r=host.run()=>r,_=&mut shutdown=>Ok(())};
    let cleanup = workers.shutdown().await;
    let active = *host
        .active
        .lock()
        .map_err(|_| anyhow::anyhow!("device activity lock poisoned"))?;
    if let Some(id) = active
        && let Ok(c) = host.current(id).await
        && let Ok(mut j) = Journal::open(
            &host.root,
            &c,
            registration_fingerprint(&host.registration),
            host.policy_revision(),
        )
    {
        let _ = j.flush(&host.cloud).await;
        let turn = host
            .kernel
            .turns(j.intent.session())
            .ok()
            .and_then(|turns| turns.into_iter().find(|t| t.id == j.intent.turn()));
        let _ = host
            .finish(&mut j, &c, view::phase(turn.map(|t| t.state)))
            .await;
    }
    let stopped = kernel.shutdown();
    drop(host);
    drop(workers);
    drop(kernel);
    drop(guard);
    result.and(cleanup).and(stopped)
}
struct Host {
    directory: PathBuf,
    root: PathBuf,
    registration: RegisterDevice,
    policy: DeviceHostPolicy,
    cloud: cloud::Cloud,
    kernel: Arc<Kernel>,
    workers: Arc<Supervisor>,
    redactor: Redactor,
    active: std::sync::Mutex<Option<DeviceCommandId>>,
}
impl Host {
    async fn reject_unowned(
        &self,
        c: &DeviceCommand,
        namespace: &[u8],
        detail: &str,
    ) -> Result<()> {
        ensure!(
            c.state == CommandState::Queued,
            "only unaccepted work may be rejected without an execution journal"
        );
        let event = ReportCommandEvent {
            request_id: RequestId(uuid::Uuid::new_v5(&c.id.0, namespace)),
            sequence: 1,
            kind: CommandEventKind::Rejected,
            detail: detail.into(),
            execution: None,
        };
        journal::write(&self.root.join(format!("rejected-{}.json", c.id)), &event)?;
        let _: CommandEventReceipt = self.cloud.post(&format!("{}/events", c.id), &event).await?;
        Ok(())
    }
    fn check_continuation(&self, j: &Journal) -> Result<()> {
        if let Some(p) = &j.intent.continuation {
            let state = self
                .kernel
                .worker_state(j.intent.session())
                .context("original Worker session missing; no replacement allowed")?;
            ensure!(
                state.provider_session_id.is_some(),
                "original provider binding missing"
            );
            let last = state.turns.last().context("original Worker turn missing")?;
            ensure!(
                last.turn.id.to_string() == p.parent_turn_id.to_string()
                    && matches!(last.turn.state, TurnState::Completed | TurnState::Failed)
                    && !state.approvals.iter().any(|a| matches!(
                        a.status,
                        ApprovalStatus::Pending | ApprovalStatus::Responding
                    )),
                "original Worker is not the known idle continuation target"
            );
        }
        Ok(())
    }
    fn policy_revision(&self) -> String {
        lain_wire::actor_configuration_revision(
            &serde_json::to_vec(&self.policy).expect("typed nonsecret policy"),
        )
    }
    fn check_local(&self, c: &DeviceCommand) -> Result<PathBuf> {
        let registration: RegisterDevice =
            journal::read(&self.directory.join("registration.json"))?;
        let policy: DeviceHostPolicy = journal::read(&self.directory.join("host-policy.json"))?;
        ensure!(
            registration == self.registration && policy == self.policy,
            "native policy or manifest changed"
        );
        ensure!(
            c.device_id == registration.id
                && c.input.profile_id == "codex/local"
                && (c.input.sandbox == lain_wire::Sandbox::ReadOnly || policy.workspace_write),
            "native scope denied"
        );
        let workspace = PathBuf::from(
            &registration
                .workspaces
                .iter()
                .find(|w| w.id == c.input.workspace_id)
                .context("workspace not admitted")?
                .path,
        );
        ensure!(
            workspace.is_dir() && workspace.canonicalize()? == workspace,
            "native workspace identity changed"
        );
        let text = serde_json::to_string(&c.input)?;
        ensure!(
            self.redactor.text(&text) == text,
            "sensitive task rejected before journaling"
        );
        Ok(workspace)
    }
    fn authority(&self, c: &DeviceCommand, submit: bool) -> Result<()> {
        ensure!(
            domain::now_ms()? < self.registration.expires_at_ms
                && domain::now_ms()? < c.input.expires_at_ms
                && !c.cancel_requested
                && c.interruption.is_none(),
            "command authority no longer permits effects"
        );
        ensure!(
            matches!(c.state, CommandState::Accepted | CommandState::Running)
                || (!submit && c.state == CommandState::Unknown),
            "command no longer permits effects"
        );
        self.check_local(c)?;
        Ok(())
    }
    async fn current(&self, id: DeviceCommandId) -> Result<DeviceCommand> {
        let c: DeviceCommand =
            tokio::time::timeout(Duration::from_secs(3), self.cloud.get(&id.to_string()))
                .await
                .map_err(|_| cloud::CloudError::Transport)??;
        ensure!(
            c.id == id && c.device_id == self.registration.id,
            "cloud command identity differs"
        );
        Ok(c)
    }
    async fn current_owned(&self, j: &Journal) -> Result<DeviceCommand> {
        let c = self.current(j.intent.command_id).await?;
        ensure!(
            c.input == j.intent.input && c.continuation == j.intent.continuation,
            "cloud input differs from immutable native intent"
        );
        ensure!(
            j.last
                .as_ref()
                .is_none_or(|e| c.last_event.as_ref().is_some_and(|r| r.event == *e)),
            "cloud event journal differs from this host"
        );
        Ok(c)
    }
    async fn bounded<T>(&self, c: &DeviceCommand, f: impl Future<Output = Result<T>>) -> Result<T> {
        let remaining = c.input.expires_at_ms.saturating_sub(domain::now_ms()?);
        ensure!(remaining > 0, "command deadline reached");
        tokio::time::timeout(Duration::from_millis(remaining.min(10000)), f)
            .await
            .context("native operation uncertain after timeout")?
    }
    async fn run(&self) -> Result<()> {
        let mut cursor = 0;
        let mut through = None;
        let mut handled = BTreeSet::new();
        let mut pending_reports = BTreeSet::new();
        'feed: loop {
            for id in pending_reports.clone() {
                if let Ok(c) = self.current(id).await {
                    let mut j = Journal::open(
                        &self.root,
                        &c,
                        registration_fingerprint(&self.registration),
                        self.policy_revision(),
                    )?;
                    if j.flush(&self.cloud).await.is_ok() {
                        let turn = self
                            .kernel
                            .turns(j.intent.session())?
                            .into_iter()
                            .find(|t| t.id == j.intent.turn());
                        if self
                            .finish(&mut j, &c, view::phase(turn.map(|t| t.state)))
                            .await
                            .is_ok()
                        {
                            pending_reports.remove(&id);
                        }
                    }
                }
            }
            let wait = if through.is_some() {
                0
            } else if pending_reports.is_empty() {
                25000
            } else {
                1000
            };
            let window = through.map_or_else(String::new, |value| format!("&through={value}"));
            let page: CommandChanges = match self
                .cloud
                .get(&format!(
                    "changes?after={cursor}&limit=32&wait_ms={wait}{window}"
                ))
                .await
            {
                Ok(p) => p,
                Err(e)
                    if e.downcast_ref::<cloud::CloudError>()
                        .is_some_and(|e| matches!(e, cloud::CloudError::Transport)) =>
                {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    continue;
                }
                Err(e) => return Err(e),
            };
            for change in &page.changes {
                if handled.contains(&change.command_id) {
                    continue;
                }
                let c = match self.current(change.command_id).await {
                    Ok(c) => c,
                    Err(e)
                        if e.downcast_ref::<cloud::CloudError>()
                            .is_some_and(|e| matches!(e, cloud::CloudError::Transport)) =>
                    {
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        continue 'feed;
                    }
                    Err(e) => return Err(e),
                };
                let owned = self
                    .root
                    .join(c.id.to_string())
                    .join("intent.json")
                    .exists();
                if c.state != CommandState::Queued && !owned {
                    continue;
                }
                if c.state.terminal() && c.state != CommandState::Unknown {
                    handled.insert(c.id);
                    continue;
                }
                let text = serde_json::to_string(&c.input)?;
                if self.redactor.text(&text) != text {
                    ensure!(
                        c.state == CommandState::Queued,
                        "sensitive existing intent requires local inspection"
                    );
                    let event = ReportCommandEvent {
                        request_id: RequestId(uuid::Uuid::new_v5(
                            &c.id.0,
                            b"native-secret-rejection-v1",
                        )),
                        sequence: 1,
                        kind: CommandEventKind::Rejected,
                        detail: "Sensitive task rejected before native journaling".into(),
                        execution: None,
                    };
                    journal::write(&self.root.join(format!("rejected-{}.json", c.id)), &event)?;
                    let _: CommandEventReceipt =
                        self.cloud.post(&format!("{}/events", c.id), &event).await?;
                    handled.insert(c.id);
                    continue;
                }
                if c.continuation.is_some()
                    && !owned
                    && Journal::verify_parent(
                        &self.root,
                        &c,
                        &registration_fingerprint(&self.registration),
                        &self.policy_revision(),
                    )
                    .is_err()
                {
                    self.reject_unowned(&c,b"native-missing-continuation-v1","Original protected parent/session scope unavailable; no replacement Worker created").await?;
                    handled.insert(c.id);
                    continue;
                }
                let mut j = Journal::open(
                    &self.root,
                    &c,
                    registration_fingerprint(&self.registration),
                    self.policy_revision(),
                )?;
                *self
                    .active
                    .lock()
                    .map_err(|_| anyhow::anyhow!("device activity lock poisoned"))? = Some(c.id);
                let result = self.process(&mut j, c.clone()).await;
                // Cleanup belongs to the service, independent of the terminal/client.
                if result.is_err() && j.claimed("launch")? {
                    let _ = self.stop(&j).await;
                }
                self.workers.stop_device_session(j.intent.session()).await?;
                if result.is_err() {
                    // Preserve any unresolved immutable outbox before making a new report.
                    let _ = j.flush(&self.cloud).await;
                    if j.claimed("launch")?
                        && let Ok(latest) = self.current(c.id).await
                    {
                        let turn = self
                            .kernel
                            .turns(j.intent.session())?
                            .into_iter()
                            .find(|t| t.id == j.intent.turn());
                        let _ = self
                            .finish(&mut j, &latest, view::phase(turn.map(|t| t.state)))
                            .await;
                    }
                    if j.claimed("launch")? {
                        pending_reports.insert(c.id);
                    }
                    if let Err(error) = &result {
                        let error = self.redactor.text(&format!("{error:#}"));
                        tracing::warn!(command_id=%c.id,%error,"native work uncertain; inspect protected execution journal");
                    }
                }
                *self
                    .active
                    .lock()
                    .map_err(|_| anyhow::anyhow!("device activity lock poisoned"))? = None;
                handled.insert(c.id);
            }
            cursor = page.next_after;
            through = page.has_more.then_some(page.through);
        }
    }
    async fn process(&self, j: &mut Journal, mut c: DeviceCommand) -> Result<()> {
        j.flush(&self.cloud).await?;
        if j.last.is_none() {
            if self.check_local(&c).is_err() || self.check_continuation(j).is_err() {
                j.event(
                    CommandEventKind::Rejected,
                    None,
                    "Native policy/workspace/profile denies this task",
                )?;
                j.flush(&self.cloud).await?;
                return Ok(());
            }
            let e = view::base(j, ExecutionPhase::Preparing, false);
            j.event(
                CommandEventKind::Accepted,
                Some(e),
                "Persisted native intent; Worker not yet launched",
            )?;
            j.flush(&self.cloud).await?;
        }
        c = self.current_owned(j).await?;
        let id = j.intent.session();
        let tid = j.intent.turn();
        let state = self.kernel.worker_state(id).ok();
        let turn = state
            .as_ref()
            .and_then(|s| s.turns.iter().find(|r| r.turn.id == tid));
        let plan = recovery_plan(
            j.claimed("launch")?,
            j.claimed("submission")?,
            state
                .as_ref()
                .is_some_and(|s| s.provider_session_id.is_some()),
            turn.is_some_and(|r| r.provider_turn_id.is_some()),
            turn.is_some_and(|r| r.turn.state.is_terminal()),
            j.intent.continuation.is_some() && self.check_continuation(j).is_ok(),
        );
        if plan == RecoveryPlan::Observe {
            return self
                .finish(j, &c, view::phase(turn.map(|r| r.turn.state)))
                .await;
        }
        if j.claimed("stop")? || plan == RecoveryPlan::Unknown {
            return self.finish(j, &c, ExecutionPhase::Unknown).await;
        }
        self.authority(
            &c,
            matches!(plan, RecoveryPlan::Launch | RecoveryPlan::ResumeThenSubmit),
        )?;
        match plan {
            RecoveryPlan::Launch => {
                let workspace = self.check_local(&c)?;
                self.check_continuation(j)?;
                let sandbox = match c.input.sandbox {
                    lain_wire::Sandbox::ReadOnly => domain::SandboxMode::ReadOnly,
                    lain_wire::Sandbox::WorkspaceWrite => domain::SandboxMode::WorkspaceWrite,
                };
                self.kernel.prepare_device_session(
                    id,
                    workspace,
                    domain::SessionOptions {
                        model: None,
                        sandbox: Some(sandbox),
                    },
                )?;
                j.claim("launch")?;
                c = self.current_owned(j).await?;
                self.authority(&c, true)?;
                if c.continuation.is_some() {
                    self.bounded(&c, self.workers.resume(id)).await?;
                } else {
                    self.bounded(&c, self.workers.start_device_session(id))
                        .await?;
                }
            }
            RecoveryPlan::ResumeThenSubmit | RecoveryPlan::ResumeObserve => {
                self.bounded(&c, self.workers.resume(id)).await?;
            }
            _ => unreachable!(),
        }
        if matches!(plan, RecoveryPlan::Launch | RecoveryPlan::ResumeThenSubmit) {
            j.claim("submission")?;
            c = self.current_owned(j).await?;
            self.authority(&c, true)?;
            self.bounded(
                &c,
                self.workers.prompt_device(id, tid, c.input.task.clone()),
            )
            .await?;
        }
        let mut events = self.kernel.subscribe();
        let mut online = Instant::now();
        let mut cursor = 0;
        loop {
            c = match self.current_owned(j).await {
                Ok(c) => {
                    online = Instant::now();
                    c
                }
                Err(e) => {
                    if e.downcast_ref::<cloud::CloudError>()
                        .is_some_and(|e| matches!(e, cloud::CloudError::Transport))
                        && online.elapsed()
                            < Duration::from_secs(self.policy.offline_grace_seconds.into())
                    {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                    self.stop(j).await?;
                    return Err(e);
                }
            };
            ensure!(
                c.input == j.intent.input && c.continuation == j.intent.continuation,
                "cloud command input changed"
            );
            let turn = self
                .kernel
                .turns(id)?
                .into_iter()
                .find(|t| t.id == tid)
                .context("native turn missing")?;
            if turn.state.is_terminal() {
                return self.finish(j, &c, view::phase(Some(turn.state))).await;
            }
            if self.authority(&c, false).is_err() {
                self.stop(j).await?;
                let turn = self.kernel.turns(id)?.into_iter().find(|t| t.id == tid);
                return self.finish(j, &c, view::phase(turn.map(|t| t.state))).await;
            }
            if turn.state == TurnState::Unknown {
                return self.finish(j, &c, ExecutionPhase::Unknown).await;
            }
            let evidence = view::observe(j, &self.kernel, &self.redactor)?;
            if j.last
                .as_ref()
                .is_none_or(|e| e.execution.as_ref() != Some(&evidence))
            {
                if j.last.as_ref().is_some_and(|e| e.sequence >= 254) {
                    self.stop(j).await?;
                    return self.finish(j, &c, ExecutionPhase::Unknown).await;
                }
                let kind = if j
                    .last
                    .as_ref()
                    .is_some_and(|e| e.kind == CommandEventKind::Accepted)
                {
                    CommandEventKind::Started
                } else {
                    CommandEventKind::Progress
                };
                j.event(
                    kind,
                    Some(evidence.clone()),
                    "Native Worker observation; not goal acceptance",
                )?;
                j.flush(&self.cloud).await?;
            }
            if let Some(a) = evidence.pending_approvals.first() {
                self.decide(j, &c, &evidence, a).await?;
            }
            let wait = (self.policy.offline_grace_seconds / 2).clamp(1, 10) * 1000;
            let wait_path = format!("changes?after={cursor}&limit=32&wait_ms={wait}");
            tokio::select! {
                page=self.cloud.get::<CommandChanges>(&wait_path)=>{if let Ok(page)=page {cursor=page.next_after;}},
                _=async {loop {match events.recv().await {Ok(e) if e.work_session_id==id && !matches!(e.kind,crate::events::EventKind::OutputProgress{..})=>break,Err(_)=>break,_=>{}}}}=>{},
                _=tokio::time::sleep(Duration::from_millis(c.input.expires_at_ms.saturating_sub(domain::now_ms()?)))=>{},
            }
        }
    }
    async fn decide(
        &self,
        j: &Journal,
        c: &DeviceCommand,
        e: &ExecutionEvidence,
        a: &DeviceApproval,
    ) -> Result<()> {
        let receipts: Vec<DeviceApprovalDecision> =
            self.cloud.get(&format!("{}/approvals", c.id)).await?;
        let Some(receipt) = receipts.into_iter().find(|r| {
            r.approval_id == a.id
                && r.decision.fingerprint == a.fingerprint
                && a.choices.contains(&r.decision.choice)
        }) else {
            return Ok(());
        };
        let fence = j.path.join(format!("decision-{}.json", a.id));
        ensure!(
            !fence.exists(),
            "decision already claimed; uncertain mutation cannot be replayed"
        );
        let fresh = self.current_owned(j).await?;
        self.authority(&fresh, true)?;
        ensure!(
            fresh.execution.as_ref() == Some(e),
            "approval scope no longer current in cloud"
        );
        let observed = view::observe(j, &self.kernel, &self.redactor)?;
        ensure!(observed == *e, "native approval scope no longer current");
        let approval = self
            .kernel
            .approvals(j.intent.session())?
            .into_iter()
            .find(|r| r.id.to_string() == a.id.to_string() && r.status == ApprovalStatus::Pending)
            .context("pending native approval missing")?;
        let choice = match receipt.decision.choice {
            DeviceApprovalChoice::ApproveOnce => ApprovalDecision::ApproveOnce,
            DeviceApprovalChoice::Deny => ApprovalDecision::Deny,
            DeviceApprovalChoice::Cancel => ApprovalDecision::Cancel,
        };
        // Rechecks precede the durable response fence. Publishing this fence is
        // device receipt evidence, never permission to replay an uncertain RPC.
        journal::write(&fence, &receipt)?;
        self.bounded(
            &fresh,
            self.workers.decide(j.intent.session(), approval.id, choice),
        )
        .await?;
        Ok(())
    }
    async fn stop(&self, j: &Journal) -> Result<()> {
        j.claim("stop")?;
        let id = j.intent.session();
        let tid = j.intent.turn();
        let mut events = self.kernel.subscribe();
        if let Some(t) = self.kernel.turns(id)?.into_iter().find(|t| t.id == tid)
            && !t.state.is_terminal()
        {
            if t.cancellation == CancellationState::NotRequested && !j.claimed("cancel")? {
                j.claim("cancel")?;
                let _ = tokio::time::timeout(Duration::from_secs(5), self.workers.cancel(id)).await;
            }
            let _ = tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    if self
                        .kernel
                        .turns(id)?
                        .into_iter()
                        .find(|t| t.id == tid)
                        .is_some_and(|t| t.state.is_terminal())
                    {
                        return Ok::<_, anyhow::Error>(());
                    }
                    events.recv().await?;
                }
            })
            .await;
        }
        self.workers.stop_device_session(id).await
    }
    async fn finish(
        &self,
        j: &mut Journal,
        c: &DeviceCommand,
        phase: ExecutionPhase,
    ) -> Result<()> {
        if c.state.terminal() && c.state != CommandState::Unknown {
            return Ok(());
        }
        let mut e = view::base(j, phase, j.claimed("submission")?);
        if self.kernel.session(j.intent.session())?.is_some() {
            e.approval_reports = view::reports(j, &self.kernel, &self.redactor)?;
            e.result = Some(view::result(j, &self.kernel, &self.redactor)?);
        }
        // A claimed submission may have crashed before creating its local turn;
        // the stable turn ID still denotes an uncertain intent, never completion.
        let kind = match phase {
            ExecutionPhase::Completed => CommandEventKind::Succeeded,
            ExecutionPhase::Failed => CommandEventKind::Failed,
            ExecutionPhase::Cancelled => CommandEventKind::Cancelled,
            _ => CommandEventKind::Unknown,
        };
        if j.last
            .as_ref()
            .is_some_and(|last| last.kind == kind && last.execution.as_ref() == Some(&e))
        {
            return j.flush(&self.cloud).await;
        }
        j.event(
            kind,
            Some(e),
            "Native terminal observation; inspect failures and truncation before goal acceptance",
        )?;
        j.flush(&self.cloud).await
    }
}
