//! Worker supervision. Only the private Codex module knows its wire protocol.
mod codex;
mod process;
mod rpc;

use crate::{
    domain::*,
    events::EventKind,
    kernel::{Kernel, KernelState, blocking},
    worker::{TurnRecord, WorkerState},
};
use anyhow::{Context, Result, ensure};
use codex::{Codex, Notice, ObservedTurn, Redactor};
use serde::Serialize;
use std::{collections::BTreeMap, path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::{Mutex, mpsc, oneshot, watch};

#[derive(Debug, thiserror::Error)]
pub enum ControlError {
    #[error("session or approval not found")]
    NotFound,
    #[error("operation conflicts with current worker state")]
    Conflict,
    #[error("worker is unavailable")]
    Unavailable,
    #[error("invalid or sensitive worker input")]
    InvalidInput,
    #[error("workspace policy denies the requested profile")]
    ProfileDenied,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackendInfo {
    pub backend: &'static str,
    pub version: Option<String>,
    pub available: bool,
    pub status: &'static str,
    pub executable: Option<PathBuf>,
    pub supported_version: &'static str,
    pub capabilities: Vec<&'static str>,
}

enum Command {
    Prompt(
        String,
        Option<crate::delegation::Origin>,
        oneshot::Sender<Result<Turn>>,
    ),
    Cancel(oneshot::Sender<Result<Turn>>),
    Decide(
        ApprovalId,
        ApprovalDecision,
        oneshot::Sender<Result<Approval>>,
    ),
    Stop(oneshot::Sender<Result<()>>),
    Detach(oneshot::Sender<Result<()>>),
}

struct Runtime {
    generation: uuid::Uuid,
    sender: mpsc::Sender<Command>,
    task: tokio::task::JoinHandle<()>,
    ready: watch::Receiver<Option<bool>>,
    done: watch::Receiver<bool>,
}

pub struct Supervisor {
    kernel: Arc<Kernel>,
    executable: Option<PathBuf>,
    info: BackendInfo,
    redactor: Redactor,
    runtimes: Mutex<BTreeMap<WorkSessionId, Runtime>>,
    idle_timeout: Duration,
}

impl Supervisor {
    pub async fn new(
        kernel: Arc<Kernel>,
        executable: Option<PathBuf>,
        api_token: crate::config::ApiToken,
        idle_timeout: Duration,
    ) -> Arc<Self> {
        let (executable, version, status) = match codex::discover(executable) {
            Err(_) => (None, None, "unavailable"),
            Ok(path) => match codex::probe(&path).await {
                Ok(version) => (Some(path), Some(version), "available"),
                Err(error) if error.is::<codex::UnsupportedVersion>() => {
                    (Some(path), None, "unsupported_version")
                }
                Err(_) => (Some(path), None, "unavailable"),
            },
        };
        let available = status == "available";
        let info = BackendInfo {
            backend: "codex",
            available,
            status,
            executable: executable.clone(),
            supported_version: "0.160.0",
            version,
            capabilities: if available {
                vec![
                    "session_create",
                    "model_selection",
                    "session_resume",
                    "turns",
                    "cancel_turn",
                    "approvals",
                    "structured_events",
                    "independent_sessions",
                ]
            } else {
                vec![]
            },
        };
        Arc::new(Self {
            kernel,
            executable: executable.filter(|_| available),
            info,
            redactor: Redactor::new(api_token.redaction_values()),
            runtimes: Mutex::new(BTreeMap::new()),
            idle_timeout,
        })
    }

    pub fn info(&self) -> BackendInfo {
        self.info.clone()
    }
    pub async fn profiles(&self) -> Result<Vec<crate::profiles::ProfileView>> {
        Ok(blocking(&self.kernel, Kernel::profiles).await?.into_iter().map(|profile| {
            use crate::profiles::ProfileHealth;
            let secret_status = crate::profiles::secret_status(profile.definition.secret_ref.as_ref());
            let (health, explanation) = if !profile.definition.enabled { (ProfileHealth::Unavailable, "Profile disabled; existing session snapshots remain inspectable") }
                else if secret_status == "missing" { (ProfileHealth::Unavailable, "Set the referenced LAIN_SECRET variable on the daemon") }
                else if !self.info.available { (ProfileHealth::Unavailable, "Install the verified backend executable") }
                else { (ProfileHealth::Valid, "Configuration and local prerequisites valid; provider access is not probed") };
            let is_default = profile.definition.id == crate::profiles::ProfileDefinition::local().id;
            crate::profiles::ProfileView { profile, health, explanation, secret_status, is_default, supported_overrides: vec!["model", "sandbox"] }
        }).collect())
    }
    pub fn safe_configuration(
        &self,
        definition: &crate::profiles::ProfileDefinition,
    ) -> Result<()> {
        let text = serde_json::to_string(&(
            &definition.id,
            &definition.backend,
            &definition.driver,
            &definition.model,
            &definition.provider,
            &definition.base_url,
            &definition.reasoning_effort,
        ))?;
        ensure!(
            self.redactor.for_runtime(None).text(&text) == text,
            ControlError::InvalidInput
        );
        definition.validate()
    }

    pub async fn create(
        self: &Arc<Self>,
        workspace: PathBuf,
        options: SessionOptions,
    ) -> Result<WorkSession> {
        self.create_with_profile(workspace, None, options).await
    }
    pub async fn create_with_profile(
        self: &Arc<Self>,
        workspace: PathBuf,
        profile: Option<RuntimeProfileId>,
        options: SessionOptions,
    ) -> Result<WorkSession> {
        self.create_owned(workspace, profile, options, None).await
    }
    pub(crate) async fn create_with_actor_receipt(
        self: &Arc<Self>,
        workspace: PathBuf,
        profile: RuntimeProfileId,
        options: SessionOptions,
        actor: crate::actor::domain::ActorTurnId,
        action: crate::actor::domain::ActionId,
    ) -> Result<WorkSession> {
        self.create_owned(workspace, Some(profile), options, Some((actor, action)))
            .await
    }
    async fn create_owned(
        self: &Arc<Self>,
        workspace: PathBuf,
        profile: Option<RuntimeProfileId>,
        options: SessionOptions,
        receipt: Option<(
            crate::actor::domain::ActorTurnId,
            crate::actor::domain::ActionId,
        )>,
    ) -> Result<WorkSession> {
        let supervisor = self.clone();
        tokio::spawn(async move {
            supervisor
                .create_inner(workspace, profile, options, receipt)
                .await
        })
        .await
        .context("worker creation task failed")?
    }
    async fn create_inner(
        self: &Arc<Self>,
        workspace: PathBuf,
        profile: Option<RuntimeProfileId>,
        options: SessionOptions,
        receipt: Option<(
            crate::actor::domain::ActorTurnId,
            crate::actor::domain::ActionId,
        )>,
    ) -> Result<WorkSession> {
        ensure!(self.executable.is_some(), ControlError::Unavailable);
        ensure!(
            options
                .model
                .as_ref()
                .is_none_or(|s| self.redactor.for_runtime(None).text(s) == *s),
            ControlError::InvalidInput
        );
        ensure!(
            options.model.as_ref().is_none_or(|model| !model.is_empty()
                && model.len() <= 128
                && model
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-._/".contains(&b))),
            ControlError::InvalidInput
        );
        let kernel = self.kernel.clone();
        let session = tokio::task::spawn_blocking(move || {
            kernel.create_profiled_session(workspace, profile, options, receipt)
        })
        .await??;
        let id = session.id();
        mutate(&self.kernel, id, None, move |session, _| {
            session.transition(WorkSessionState::Starting)?;
            Ok(vec![])
        })
        .await?;
        self.launch(id, false).await?;
        blocking(&self.kernel, move |k| k.session(id))
            .await?
            .context("created session missing")
    }

    pub async fn resume(self: &Arc<Self>, id: WorkSessionId) -> Result<WorkSession> {
        ensure!(
            blocking(&self.kernel, Kernel::state).await? == KernelState::Ready,
            ControlError::Unavailable
        );
        let session = blocking(&self.kernel, move |k| k.session(id))
            .await?
            .ok_or(ControlError::NotFound)?;
        ensure!(
            matches!(
                session.state(),
                WorkSessionState::Unknown
                    | WorkSessionState::Disconnected
                    | WorkSessionState::Idle
                    | WorkSessionState::Ready
            ),
            ControlError::Conflict
        );
        self.launch(id, true).await?;
        blocking(&self.kernel, move |k| k.session(id))
            .await?
            .context("resumed session missing")
    }

    pub async fn recover(self: &Arc<Self>) {
        for session in blocking(&self.kernel, Kernel::list_sessions)
            .await
            .unwrap_or_default()
        {
            let id = session.id();
            if blocking(&self.kernel, move |k| k.worker_state(id))
                .await
                .ok()
                .is_some_and(|w| w.provider_session_id.is_some())
                && session.state() == WorkSessionState::Unknown
            {
                match self.launch(id, true).await {
                    Ok(()) => {
                        let _ = self.detach(id).await;
                    }
                    Err(error) => {
                        tracing::warn!(session_id=%id,error=%error,"worker resume failed; uncertainty retained")
                    }
                }
            }
        }
    }

    async fn launch(self: &Arc<Self>, id: WorkSessionId, resume: bool) -> Result<()> {
        let path = self.executable.clone().ok_or(ControlError::Unavailable)?;
        let checked = blocking(&self.kernel, move |k| k.session(id))
            .await?
            .ok_or(ControlError::NotFound)?;
        blocking(&self.kernel, move |k| {
            k.check_workspace_profile(&checked.workspace, &checked.runtime_profile_id)
        })
        .await?;
        let mut runtimes = loop {
            let runtimes = self.runtimes.lock().await;
            if let Some(runtime) = runtimes.get(&id) {
                if !runtime.sender.is_closed() {
                    let mut ready = runtime.ready.clone();
                    drop(runtimes);
                    let value = ready
                        .wait_for(|ready| ready.is_some())
                        .await
                        .context("worker startup stopped")?;
                    ensure!(*value == Some(true), ControlError::Unavailable);
                    return Ok(());
                }
                let mut done = runtime.done.clone();
                drop(runtimes);
                done.wait_for(|done| *done)
                    .await
                    .context("worker teardown stopped")?;
                let mut runtimes = self.runtimes.lock().await;
                if runtimes.get(&id).is_some_and(|r| r.sender.is_closed()) {
                    runtimes.remove(&id);
                }
                continue;
            }
            break runtimes;
        };
        let session = blocking(&self.kernel, move |k| k.session(id))
            .await?
            .ok_or(ControlError::NotFound)?;
        let state = blocking(&self.kernel, move |k| k.worker_state(id)).await?;
        let profile = state
            .resolved_profile
            .clone()
            .ok_or(ControlError::Unavailable)?;
        let workspace = session.workspace.clone();
        let profile_id = profile.profile_id.clone();
        blocking(&self.kernel, move |k| {
            k.check_workspace_profile(&workspace, &profile_id)
        })
        .await?;
        let credential = profile
            .secret_ref
            .as_ref()
            .map(crate::profiles::resolve_secret)
            .transpose()?;
        let redactor = self.redactor.for_runtime(credential.as_ref());
        let launch = codex::CodexLaunchSpec::new(&profile, credential)?;
        if resume {
            ensure!(state.provider_session_id.is_some(), ControlError::Conflict);
        }
        mutate(&self.kernel, id, None, |session, _| {
            session.runtime = RuntimeAttachment::Starting;
            Ok(vec![])
        })
        .await?;
        let (sender, receiver) = mpsc::channel(16);
        let (started, ready) = oneshot::channel();
        let kernel = self.kernel.clone();
        let owner = Arc::downgrade(self);
        let generation = uuid::Uuid::new_v4();
        let idle_timeout = self.idle_timeout;
        let (ready_tx, ready_rx) = watch::channel(None);
        let (done_tx, done_rx) = watch::channel(false);
        let task = tokio::spawn(async move {
            let connected = Codex::connect(
                &path,
                &session.workspace,
                &profile,
                launch,
                state.provider_session_id.as_deref(),
                redactor,
            )
            .await;
            match connected {
                Ok(mut connected) => {
                    let identity = connected.worker.identity.clone();
                    let active = connected.active;
                    let history = connected.history;
                    let resolved = connected.resolved;
                    let result =
                        mutate(&kernel, id, None, move |session, worker| {
                            worker.provider_session_id = Some(identity);
                            worker.resolved_profile = Some(resolved);
                            session.runtime = RuntimeAttachment::Attached;
                            for observed in history {
                                let state = if observed.state == TurnState::Running && !active {
                                    TurnState::Unknown
                                } else {
                                    observed.state
                                };
                                if let Some(record) = worker.turns.iter_mut().find(|r| {
                                    r.provider_turn_id.as_deref() == Some(&observed.identity)
                                }) {
                                    record.turn.state = state;
                                    record.turn.failure = observed.failure;
                                    record.turn.updated_at_ms =
                                        now_ms()?.max(record.turn.updated_at_ms);
                                    if state == TurnState::Cancelled {
                                        record.turn.cancellation = CancellationState::Confirmed;
                                    }
                                } else {
                                    let now = now_ms()?;
                                    worker.turns.push(TurnRecord {
                                        provider_turn_id: Some(observed.identity),
                                        turn: Turn {
                                            id: TurnId::new(),
                                            work_session_id: id,
                                            state,
                                            cancellation: if state == TurnState::Cancelled {
                                                CancellationState::Confirmed
                                            } else {
                                                CancellationState::NotRequested
                                            },
                                            created_at_ms: now,
                                            updated_at_ms: now,
                                            failure: observed.failure,
                                        },
                                    });
                                }
                            }
                            let next = if active {
                                WorkSessionState::Running
                            } else if resume {
                                WorkSessionState::Idle
                            } else {
                                WorkSessionState::Ready
                            };
                            if session.state() != next {
                                session.transition(next)?;
                            }
                            Ok(if resume {
                                vec![EventKind::BackendReconnected]
                            } else {
                                vec![]
                            })
                        })
                        .await;
                    let successful = result.is_ok();
                    ready_tx.send_replace(Some(successful));
                    let _ = started.send(result);
                    if successful {
                        run(kernel, id, connected.worker, receiver, idle_timeout).await;
                    } else {
                        let _ = connected.worker.shutdown().await;
                    }
                }
                Err(error) => {
                    drop(receiver);
                    let _ = disconnected(&kernel, id).await;
                    ready_tx.send_replace(Some(false));
                    let _ = started.send(Err(error));
                }
            }
            done_tx.send_replace(true);
            if let Some(owner) = owner.upgrade() {
                let mut runtimes = owner.runtimes.lock().await;
                if runtimes
                    .get(&id)
                    .is_some_and(|r| r.generation == generation)
                {
                    runtimes.remove(&id);
                }
            }
        });
        runtimes.insert(
            id,
            Runtime {
                generation,
                sender,
                task,
                ready: ready_rx,
                done: done_rx,
            },
        );
        drop(runtimes);
        ready.await.context("worker startup task stopped")?
    }

    async fn send(
        self: &Arc<Self>,
        id: WorkSessionId,
        mut command: Command,
        attach: bool,
    ) -> Result<()> {
        ensure!(
            blocking(&self.kernel, Kernel::state).await? == KernelState::Ready,
            ControlError::Unavailable
        );
        ensure!(
            blocking(&self.kernel, move |k| k.session(id))
                .await?
                .is_some(),
            ControlError::NotFound
        );
        loop {
            if attach {
                self.launch(id, true).await?;
            }
            let runtimes = self.runtimes.lock().await;
            let Some(runtime) = runtimes.get(&id) else {
                if attach {
                    drop(runtimes);
                    continue;
                }
                return Err(ControlError::Unavailable.into());
            };
            match runtime.sender.try_send(command) {
                Ok(()) => return Ok(()),
                Err(mpsc::error::TrySendError::Closed(returned)) if attach => {
                    command = returned;
                    drop(runtimes);
                }
                Err(_) => return Err(ControlError::Unavailable.into()),
            }
        }
    }

    pub async fn prompt(self: &Arc<Self>, id: WorkSessionId, text: String) -> Result<Turn> {
        self.prompt_with_origin(id, text, None).await
    }
    pub(crate) async fn prompt_for_actor(
        self: &Arc<Self>,
        id: WorkSessionId,
        text: String,
        actor_turn_id: crate::actor::domain::ActorTurnId,
        action_id: crate::actor::domain::ActionId,
    ) -> Result<Turn> {
        self.prompt_with_origin(
            id,
            text,
            Some(crate::delegation::Origin {
                actor_turn_id,
                action_id,
            }),
        )
        .await
    }
    async fn prompt_with_origin(
        self: &Arc<Self>,
        id: WorkSessionId,
        text: String,
        origin: Option<crate::delegation::Origin>,
    ) -> Result<Turn> {
        ensure!(
            !text.trim().is_empty()
                && text.len() <= 32 * 1024
                && !text.contains('\0')
                && self.redactor.for_runtime(None).text(&text) == text,
            ControlError::InvalidInput
        );
        let (reply, result) = oneshot::channel();
        self.send(id, Command::Prompt(text, origin, reply), true)
            .await?;
        result.await.context("worker stopped during prompt")?
    }
    pub async fn cancel(self: &Arc<Self>, id: WorkSessionId) -> Result<Turn> {
        let (reply, result) = oneshot::channel();
        self.send(id, Command::Cancel(reply), false).await?;
        result.await.context("worker stopped during cancellation")?
    }
    pub async fn decide(
        self: &Arc<Self>,
        id: WorkSessionId,
        approval: ApprovalId,
        decision: ApprovalDecision,
    ) -> Result<Approval> {
        let (reply, result) = oneshot::channel();
        self.send(id, Command::Decide(approval, decision, reply), false)
            .await?;
        result.await.context("worker stopped during approval")?
    }
    pub async fn detach(self: &Arc<Self>, id: WorkSessionId) -> Result<()> {
        let (reply, result) = oneshot::channel();
        let sender = self
            .runtimes
            .lock()
            .await
            .get(&id)
            .map(|r| r.sender.clone())
            .ok_or(ControlError::Unavailable)?;
        sender
            .send(Command::Detach(reply))
            .await
            .map_err(|_| ControlError::Unavailable)?;
        result.await.context("worker stopped during detach")?
    }
    pub async fn shutdown(&self) -> Result<()> {
        let runtimes = std::mem::take(&mut *self.runtimes.lock().await);
        let mut tasks = Vec::new();
        for (_, runtime) in runtimes {
            let (reply, _) = oneshot::channel();
            let _ = runtime.sender.send(Command::Stop(reply)).await;
            tasks.push(runtime.task);
        }
        for task in tasks {
            task.await.context("worker supervisor task failed")?;
        }
        Ok(())
    }
}

async fn mutate(
    kernel: &Arc<Kernel>,
    id: WorkSessionId,
    turn: Option<TurnId>,
    change: impl FnOnce(&mut WorkSession, &mut WorkerState) -> Result<Vec<EventKind>> + Send + 'static,
) -> Result<()> {
    blocking(kernel, move |k| k.mutate_worker(id, turn, change)).await
}

fn failure() -> WorkerFailure {
    WorkerFailure {
        kind: FailureKind::Unavailable,
        message: "worker connection lost; outcome requires reconciliation".into(),
        code: None,
        retryable: None,
    }
}

async fn disconnected(kernel: &Arc<Kernel>, id: WorkSessionId) -> Result<()> {
    mutate(kernel, id, None, |session, worker| {
        let idle = is_idle(session, worker);
        let mut events = worker.recover(now_ms()?);
        session.runtime = RuntimeAttachment::Lost;
        if !idle && session.state() != WorkSessionState::Unknown {
            session.transition(WorkSessionState::Unknown)?;
        }
        events.push(EventKind::BackendDisconnected { failure: failure() });
        Ok(events)
    })
    .await
}

fn is_idle(session: &WorkSession, worker: &WorkerState) -> bool {
    matches!(
        session.state(),
        WorkSessionState::Idle | WorkSessionState::Ready
    ) && worker.provider_session_id.is_some()
        && worker.turns.iter().all(|r| r.turn.state.is_terminal())
        && !worker.approvals.iter().any(|a| {
            matches!(
                a.status,
                ApprovalStatus::Pending | ApprovalStatus::Responding
            )
        })
}

async fn idle(kernel: &Arc<Kernel>, id: WorkSessionId) -> Result<bool> {
    blocking(kernel, move |k| {
        let session = k.session(id)?.context("session missing")?;
        Ok(is_idle(&session, &k.worker_state(id)?))
    })
    .await
}

async fn detached(kernel: &Arc<Kernel>, id: WorkSessionId) -> Result<()> {
    mutate(kernel, id, None, |session, worker| {
        let mut events = Vec::new();
        let next = if is_idle(session, worker) {
            WorkSessionState::Idle
        } else {
            events = worker.recover(now_ms()?);
            WorkSessionState::Unknown
        };
        session.runtime = RuntimeAttachment::Detached;
        if session.state() != next {
            session.transition(next)?;
        }
        Ok(events)
    })
    .await
}

async fn run(
    kernel: Arc<Kernel>,
    id: WorkSessionId,
    mut worker: Codex,
    mut commands: mpsc::Receiver<Command>,
    idle_timeout: Duration,
) {
    let mut planned = false;
    let mut detach_reply = None;
    let mut idle_deadline = None;
    loop {
        match idle(&kernel, id).await {
            Ok(true) => {
                idle_deadline.get_or_insert_with(|| tokio::time::Instant::now() + idle_timeout);
            }
            Ok(false) => idle_deadline = None,
            Err(_) => break,
        }
        let result = tokio::select! {
            biased;
            command=commands.recv()=>match command {
                Some(Command::Prompt(text,origin,reply))=>{let result=prompt(&kernel,id,&worker,text,origin).await; let fatal=is_fatal(&result); let _=reply.send(result); if fatal {break;} idle_deadline=None; Ok(())},
                Some(Command::Cancel(reply))=>{let result=cancel(&kernel,id,&worker).await; let fatal=is_fatal(&result); let _=reply.send(result); if fatal {break;} Ok(())},
                Some(Command::Decide(approval,decision,reply))=>{let result=decide(&kernel,id,&worker,approval,decision).await;let fatal=is_fatal(&result);let _=reply.send(result);if fatal {break;} Ok(())},
                Some(Command::Stop(reply))=>{planned=true;detach_reply=Some(reply);break;},
                Some(Command::Detach(reply))=>{
                    if idle(&kernel,id).await.unwrap_or(false) {planned=true;detach_reply=Some(reply);break;}
                    let _=reply.send(Err(ControlError::Conflict.into())); Ok(())
                },
                None=>{planned=true;break;},
            },
            incoming=worker.receive()=>match incoming {
                Ok(incoming)=>match worker.decode(incoming).await {
                    Ok(Some(notice))=>apply_notice(&kernel,id,notice).await,
                    Ok(None)=>Ok(()),
                    Err(error)=>Err(error),
                },
                Err(error)=>Err(error),
            },
            _=async {match idle_deadline {Some(deadline)=>tokio::time::sleep_until(deadline).await,None=>std::future::pending().await}}=>{planned=true;break;},
        };
        if let Err(error) = result {
            tracing::warn!(session_id=%id,error=%error,"worker supervision stopped");
            break;
        }
    }
    commands.close();
    if planned {
        let _ = mutate(&kernel, id, None, |session, _| {
            session.runtime = RuntimeAttachment::Stopping;
            Ok(vec![])
        })
        .await;
    }
    let shutdown = worker.shutdown().await;
    drop(commands);
    let result = if planned && shutdown.is_ok() {
        detached(&kernel, id).await
    } else {
        disconnected(&kernel, id).await
    };
    if let Err(error) = &result {
        tracing::error!(session_id=%id,error=%error,"cannot persist worker attachment state");
    }
    if let Some(reply) = detach_reply {
        let _ = reply.send(shutdown.and(result));
    }
}

fn is_fatal<T>(result: &Result<T>) -> bool {
    result.as_ref().is_err_and(|error| {
        error.downcast_ref::<ControlError>().is_none()
            && error.downcast_ref::<rpc::Rejected>().is_none()
    })
}

async fn prompt(
    kernel: &Arc<Kernel>,
    id: WorkSessionId,
    worker: &Codex,
    text: String,
    origin: Option<crate::delegation::Origin>,
) -> Result<Turn> {
    let turn_id = TurnId::new();
    let submitted_text = text.clone();
    blocking(kernel, move |kernel| {
        kernel.mutate_worker_with_origin(
            id,
            Some(turn_id),
            origin.map(|origin| (origin, submitted_text)),
            move |session, state| {
                ensure!(
                    matches!(
                        session.state(),
                        WorkSessionState::Ready | WorkSessionState::Idle
                    ),
                    ControlError::Conflict
                );
                let now = now_ms()?;
                state.turns.push(TurnRecord {
                    provider_turn_id: None,
                    turn: Turn {
                        id: turn_id,
                        work_session_id: id,
                        state: TurnState::Starting,
                        cancellation: CancellationState::NotRequested,
                        created_at_ms: now,
                        updated_at_ms: now,
                        failure: None,
                    },
                });
                session.transition(WorkSessionState::Running)?;
                Ok(vec![])
            },
        )
    })
    .await?;
    match worker.prompt(&text).await {
        Ok(observed) => {
            mutate(kernel, id, Some(turn_id), move |session, state| {
                let record = state
                    .turns
                    .iter_mut()
                    .find(|r| r.turn.id == turn_id)
                    .context("pending turn missing")?;
                record.provider_turn_id = Some(observed.identity);
                record.turn.state = observed.state;
                record.turn.failure = observed.failure;
                record.turn.updated_at_ms = now_ms()?.max(record.turn.updated_at_ms);
                if observed.state.is_terminal() {
                    session.transition(WorkSessionState::Idle)?;
                }
                Ok(vec![if observed.state.is_terminal() {
                    EventKind::TurnCompleted {
                        turn: record.turn.clone(),
                    }
                } else {
                    EventKind::TurnStarted {
                        turn: record.turn.clone(),
                    }
                }])
            })
            .await?;
            blocking(kernel, move |k| k.turns(id))
                .await?
                .into_iter()
                .find(|t| t.id == turn_id)
                .context("started turn missing")
        }
        Err(error) => {
            if let Some(rejected) = error.downcast_ref::<rpc::Rejected>() {
                let code = rejected.code;
                mutate(kernel, id, Some(turn_id), move |session, state| {
                    let record = state
                        .turns
                        .iter_mut()
                        .find(|r| r.turn.id == turn_id)
                        .context("pending turn missing")?;
                    record.turn.state = TurnState::Failed;
                    record.turn.updated_at_ms = now_ms()?.max(record.turn.updated_at_ms);
                    record.turn.failure = Some(WorkerFailure {
                        kind: FailureKind::Rejected,
                        message: "worker rejected prompt".into(),
                        code: Some(code),
                        retryable: Some(false),
                    });
                    session.transition(WorkSessionState::Idle)?;
                    Ok(vec![EventKind::TurnCompleted {
                        turn: record.turn.clone(),
                    }])
                })
                .await?;
            }
            Err(error)
        }
    }
}

async fn cancel(kernel: &Arc<Kernel>, id: WorkSessionId, worker: &Codex) -> Result<Turn> {
    let state = blocking(kernel, move |k| k.worker_state(id)).await?;
    let record = state
        .turns
        .iter()
        .rev()
        .find(|r| r.turn.state == TurnState::Running)
        .ok_or(ControlError::Conflict)?;
    let turn_id = record.turn.id;
    let provider = record
        .provider_turn_id
        .clone()
        .ok_or(ControlError::Conflict)?;
    mutate(kernel, id, Some(turn_id), move |_, state| {
        let turn = &mut state
            .turns
            .iter_mut()
            .find(|r| r.turn.id == turn_id)
            .context("active turn missing")?
            .turn;
        ensure!(
            matches!(
                turn.cancellation,
                CancellationState::NotRequested | CancellationState::Rejected
            ),
            ControlError::Conflict
        );
        turn.cancellation = CancellationState::Requested;
        turn.updated_at_ms = now_ms()?.max(turn.updated_at_ms);
        Ok(vec![EventKind::CancellationChanged {
            state: CancellationState::Requested,
        }])
    })
    .await?;
    if let Err(error) = worker.cancel(&provider).await {
        if error.downcast_ref::<rpc::Rejected>().is_some() {
            mutate(kernel, id, Some(turn_id), move |_, state| {
                let turn = &mut state
                    .turns
                    .iter_mut()
                    .find(|r| r.turn.id == turn_id)
                    .context("active turn missing")?
                    .turn;
                turn.cancellation = CancellationState::Rejected;
                turn.updated_at_ms = now_ms()?.max(turn.updated_at_ms);
                Ok(vec![EventKind::CancellationChanged {
                    state: CancellationState::Rejected,
                }])
            })
            .await?;
        }
        return Err(error);
    }
    mutate(kernel, id, Some(turn_id), move |_, state| {
        let turn = &mut state
            .turns
            .iter_mut()
            .find(|r| r.turn.id == turn_id)
            .context("active turn missing")?
            .turn;
        turn.cancellation = CancellationState::Acknowledged;
        turn.updated_at_ms = now_ms()?.max(turn.updated_at_ms);
        Ok(vec![EventKind::CancellationChanged {
            state: CancellationState::Acknowledged,
        }])
    })
    .await?;
    blocking(kernel, move |k| k.turns(id))
        .await?
        .into_iter()
        .find(|t| t.id == turn_id)
        .context("cancelled turn missing")
}

async fn decide(
    kernel: &Arc<Kernel>,
    id: WorkSessionId,
    worker: &Codex,
    approval_id: ApprovalId,
    decision: ApprovalDecision,
) -> Result<Approval> {
    mutate(kernel, id, None, move |_, state| {
        let approval = state
            .approvals
            .iter_mut()
            .find(|a| a.id == approval_id)
            .ok_or(ControlError::NotFound)?;
        ensure!(
            approval.status == ApprovalStatus::Pending && approval.decisions.contains(&decision),
            ControlError::Conflict
        );
        approval.status = ApprovalStatus::Responding;
        approval.decision = Some(decision);
        approval.updated_at_ms = now_ms()?.max(approval.updated_at_ms);
        Ok(vec![EventKind::ApprovalDecisionSubmitted { approval_id }])
    })
    .await?;
    worker.decide(approval_id, decision).await?;
    blocking(kernel, move |k| k.approvals(id))
        .await?
        .into_iter()
        .find(|a| a.id == approval_id)
        .context("approval missing")
}

async fn apply_notice(kernel: &Arc<Kernel>, id: WorkSessionId, notice: Notice) -> Result<()> {
    match notice {
        Notice::Started(turn) | Notice::Finished(turn) => apply_turn(kernel, id, turn).await,
        Notice::Live {
            provider_turn_id,
            kind,
        } => {
            let turn = blocking(kernel, move |k| {
                Ok(provider_turn_id.and_then(|native| {
                    k.worker_state(id)
                        .ok()?
                        .turns
                        .into_iter()
                        .find(|r| r.provider_turn_id.as_deref() == Some(&native))
                        .map(|r| r.turn.id)
                }))
            })
            .await?;
            if matches!(kind, EventKind::OutputProgress { .. }) {
                kernel.publish_live(id, turn, kind)
            } else {
                blocking(kernel, move |k| k.observe(id, turn, kind)).await
            }
        }
        Notice::Approval {
            provider_turn_id,
            id: approval_id,
            kind,
            description,
            decisions,
        } => {
            mutate(kernel, id, None, move |session, state| {
                let turn = state
                    .turns
                    .iter()
                    .find(|r| r.provider_turn_id.as_deref() == Some(&provider_turn_id))
                    .context("approval for unknown turn")?
                    .turn
                    .id;
                let now = now_ms()?;
                let approval = Approval {
                    id: approval_id,
                    work_session_id: id,
                    turn_id: Some(turn),
                    kind,
                    description,
                    decisions,
                    status: ApprovalStatus::Pending,
                    decision: None,
                    created_at_ms: now,
                    updated_at_ms: now,
                };
                state.approvals.push(approval.clone());
                if session.state() != WorkSessionState::WaitingForApproval {
                    session.transition(WorkSessionState::WaitingForApproval)?;
                }
                Ok(vec![EventKind::ApprovalRequested { approval }])
            })
            .await
        }
        Notice::ApprovalCleared(approval_id) => {
            mutate(kernel, id, None, move |session, state| {
                let approval = state
                    .approvals
                    .iter_mut()
                    .find(|a| a.id == approval_id)
                    .context("resolved approval missing")?;
                if matches!(
                    approval.status,
                    ApprovalStatus::Expired | ApprovalStatus::Unknown | ApprovalStatus::Resolved
                ) {
                    return Ok(vec![]);
                }
                approval.status = if approval.decision.is_some() {
                    ApprovalStatus::Resolved
                } else {
                    ApprovalStatus::Expired
                };
                approval.updated_at_ms = now_ms()?.max(approval.updated_at_ms);
                let event = EventKind::ApprovalResolved {
                    approval: approval.clone(),
                };
                if session.state() == WorkSessionState::WaitingForApproval
                    && !state.approvals.iter().any(|a| {
                        matches!(
                            a.status,
                            ApprovalStatus::Pending | ApprovalStatus::Responding
                        )
                    })
                {
                    session.transition(WorkSessionState::Running)?;
                }
                Ok(vec![event])
            })
            .await
        }
    }
}

async fn apply_turn(kernel: &Arc<Kernel>, id: WorkSessionId, observed: ObservedTurn) -> Result<()> {
    let record = blocking(kernel, move |k| k.worker_state(id))
        .await?
        .turns
        .into_iter()
        .find(|r| r.provider_turn_id.as_deref() == Some(&observed.identity))
        .context("notification for unknown turn")?;
    if record.turn.state == observed.state {
        return Ok(());
    }
    if record.turn.state.is_terminal() {
        return Ok(());
    }
    let turn_id = record.turn.id;
    mutate(kernel, id, Some(turn_id), move |session, state| {
        let record = state
            .turns
            .iter_mut()
            .find(|r| r.turn.id == turn_id)
            .context("turn missing")?;
        record.turn.state = observed.state;
        record.turn.failure = observed.failure;
        record.turn.updated_at_ms = now_ms()?.max(record.turn.updated_at_ms);
        if observed.state == TurnState::Cancelled {
            record.turn.cancellation = CancellationState::Confirmed;
        }
        let event = if observed.state.is_terminal() {
            EventKind::TurnCompleted {
                turn: record.turn.clone(),
            }
        } else {
            EventKind::TurnStarted {
                turn: record.turn.clone(),
            }
        };
        let mut events = vec![event];
        if observed.state.is_terminal() {
            for approval in &mut state.approvals {
                if approval.turn_id == Some(turn_id)
                    && matches!(
                        approval.status,
                        ApprovalStatus::Pending | ApprovalStatus::Responding
                    )
                {
                    approval.status = if approval.decision.is_some() {
                        ApprovalStatus::Unknown
                    } else {
                        ApprovalStatus::Expired
                    };
                    approval.updated_at_ms = now_ms()?.max(approval.updated_at_ms);
                    events.push(EventKind::ApprovalResolved {
                        approval: approval.clone(),
                    });
                }
            }
            if matches!(
                session.state(),
                WorkSessionState::Running | WorkSessionState::WaitingForApproval
            ) {
                session.transition(WorkSessionState::Idle)?;
            }
        }
        Ok(events)
    })
    .await
}
