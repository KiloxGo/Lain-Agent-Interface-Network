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
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, mpsc, oneshot};

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
}

#[derive(Debug, Clone, Serialize)]
pub struct BackendInfo {
    pub backend: &'static str,
    pub version: Option<String>,
    pub available: bool,
    pub capabilities: Vec<&'static str>,
}

enum Command {
    Prompt(String, oneshot::Sender<Result<Turn>>),
    Cancel(oneshot::Sender<Result<Turn>>),
    Decide(
        ApprovalId,
        ApprovalDecision,
        oneshot::Sender<Result<Approval>>,
    ),
    Stop(oneshot::Sender<Result<()>>),
}

struct Runtime {
    sender: mpsc::Sender<Command>,
    task: tokio::task::JoinHandle<()>,
}

pub struct Supervisor {
    kernel: Arc<Kernel>,
    executable: Option<PathBuf>,
    info: BackendInfo,
    redactor: Redactor,
    runtimes: Mutex<BTreeMap<WorkSessionId, Runtime>>,
}

impl Supervisor {
    pub async fn new(
        kernel: Arc<Kernel>,
        executable: Option<PathBuf>,
        api_token: crate::config::ApiToken,
    ) -> Arc<Self> {
        let probe = async {
            let path = codex::discover(executable)?;
            let version = codex::probe(&path).await?;
            Ok::<_, anyhow::Error>((path, version))
        }
        .await;
        let (executable, version) = match probe {
            Ok((path, version)) => (Some(path), Some(version)),
            Err(error) => {
                tracing::warn!(error=%error,"Codex unavailable; query API remains available");
                (None, None)
            }
        };
        let info = BackendInfo {
            backend: "codex",
            available: executable.is_some(),
            version,
            capabilities: if executable.is_some() {
                vec![
                    "session_create",
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
            executable,
            info,
            redactor: Redactor::new(api_token.redaction_values()),
            runtimes: Mutex::new(BTreeMap::new()),
        })
    }

    pub fn info(&self) -> BackendInfo {
        self.info.clone()
    }

    pub async fn create(&self, workspace: PathBuf, options: SessionOptions) -> Result<WorkSession> {
        ensure!(self.executable.is_some(), ControlError::Unavailable);
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
            kernel.create_session("codex".parse()?, "codex/local".parse()?, workspace)
        })
        .await??;
        let id = session.id();
        mutate(&self.kernel, id, None, move |session, worker| {
            worker.options = options;
            session.transition(WorkSessionState::Starting)?;
            Ok(vec![])
        })
        .await?;
        self.launch(id, false).await?;
        blocking(&self.kernel, move |k| k.session(id))
            .await?
            .context("created session missing")
    }

    pub async fn resume(&self, id: WorkSessionId) -> Result<WorkSession> {
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
                WorkSessionState::Unknown | WorkSessionState::Disconnected
            ),
            ControlError::Conflict
        );
        self.launch(id, true).await?;
        blocking(&self.kernel, move |k| k.session(id))
            .await?
            .context("resumed session missing")
    }

    pub async fn recover(&self) {
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
                && let Err(error) = self.launch(session.id(), true).await
            {
                tracing::warn!(session_id=%session.id(),error=%error,"worker resume failed; uncertainty retained");
            }
        }
    }

    async fn launch(&self, id: WorkSessionId, resume: bool) -> Result<()> {
        let path = self.executable.clone().ok_or(ControlError::Unavailable)?;
        let mut runtimes = self.runtimes.lock().await;
        ensure!(
            !runtimes.get(&id).is_some_and(|r| !r.sender.is_closed()),
            ControlError::Conflict
        );
        let session = blocking(&self.kernel, move |k| k.session(id))
            .await?
            .ok_or(ControlError::NotFound)?;
        let state = blocking(&self.kernel, move |k| k.worker_state(id)).await?;
        if resume {
            ensure!(state.provider_session_id.is_some(), ControlError::Conflict);
        }
        let (sender, receiver) = mpsc::channel(16);
        let (started, ready) = oneshot::channel();
        let kernel = self.kernel.clone();
        let redactor = self.redactor.clone();
        let task = tokio::spawn(async move {
            let connected = Codex::connect(
                &path,
                &session.workspace,
                &state.options,
                state.provider_session_id.as_deref(),
                redactor,
            )
            .await;
            match connected {
                Ok(mut connected) => {
                    let identity = connected.worker.identity.clone();
                    let active = connected.active;
                    let history = connected.history;
                    let result =
                        mutate(&kernel, id, None, move |session, worker| {
                            worker.provider_session_id = Some(identity);
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
                            session.transition(if active {
                                WorkSessionState::Running
                            } else if resume {
                                WorkSessionState::Idle
                            } else {
                                WorkSessionState::Ready
                            })?;
                            Ok(if resume {
                                vec![EventKind::BackendReconnected]
                            } else {
                                vec![]
                            })
                        })
                        .await;
                    let successful = result.is_ok();
                    let _ = started.send(result);
                    if successful {
                        run(kernel, id, connected.worker, receiver).await;
                    } else {
                        let _ = connected.worker.shutdown().await;
                    }
                }
                Err(error) => {
                    drop(receiver);
                    let _ = disconnected(&kernel, id).await;
                    let _ = started.send(Err(error));
                }
            }
        });
        runtimes.insert(id, Runtime { sender, task });
        drop(runtimes);
        ready.await.context("worker startup task stopped")?
    }

    async fn send(&self, id: WorkSessionId, command: Command) -> Result<()> {
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
        let runtimes = self.runtimes.lock().await;
        let runtime = runtimes.get(&id).ok_or(ControlError::Unavailable)?;
        runtime
            .sender
            .try_send(command)
            .map_err(|_| ControlError::Unavailable)?;
        Ok(())
    }

    pub async fn prompt(&self, id: WorkSessionId, text: String) -> Result<Turn> {
        ensure!(
            !text.trim().is_empty()
                && text.len() <= 32 * 1024
                && !text.contains('\0')
                && self.redactor.text(&text) == text,
            ControlError::InvalidInput
        );
        let (reply, result) = oneshot::channel();
        self.send(id, Command::Prompt(text, reply)).await?;
        result.await.context("worker stopped during prompt")?
    }
    pub async fn cancel(&self, id: WorkSessionId) -> Result<Turn> {
        let (reply, result) = oneshot::channel();
        self.send(id, Command::Cancel(reply)).await?;
        result.await.context("worker stopped during cancellation")?
    }
    pub async fn decide(
        &self,
        id: WorkSessionId,
        approval: ApprovalId,
        decision: ApprovalDecision,
    ) -> Result<Approval> {
        let (reply, result) = oneshot::channel();
        self.send(id, Command::Decide(approval, decision, reply))
            .await?;
        result.await.context("worker stopped during approval")?
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
    }
}

async fn disconnected(kernel: &Arc<Kernel>, id: WorkSessionId) -> Result<()> {
    mutate(kernel, id, None, |session, worker| {
        worker.recover(now_ms()?);
        if session.state() != WorkSessionState::Disconnected {
            session.transition(WorkSessionState::Disconnected)?;
        }
        Ok(vec![EventKind::BackendDisconnected { failure: failure() }])
    })
    .await
}

async fn run(
    kernel: Arc<Kernel>,
    id: WorkSessionId,
    mut worker: Codex,
    mut commands: mpsc::Receiver<Command>,
) {
    loop {
        let result = tokio::select! {
            incoming=worker.receive()=>match incoming {
                Ok(incoming)=>match worker.decode(incoming).await {
                    Ok(Some(notice))=>apply_notice(&kernel,id,notice).await,
                    Ok(None)=>Ok(()),
                    Err(error)=>Err(error),
                },
                Err(error)=>Err(error),
            },
            command=commands.recv()=>match command {
                Some(Command::Prompt(text,reply))=>{let result=prompt(&kernel,id,&worker,text).await; let fatal=is_fatal(&result); let _=reply.send(result); if fatal {break;} Ok(())},
                Some(Command::Cancel(reply))=>{let result=cancel(&kernel,id,&worker).await; let fatal=is_fatal(&result); let _=reply.send(result); if fatal {break;} Ok(())},
                Some(Command::Decide(approval,decision,reply))=>{let result=decide(&kernel,id,&worker,approval,decision).await;let fatal=is_fatal(&result);let _=reply.send(result);if fatal {break;} Ok(())},
                Some(Command::Stop(reply))=>{let result=worker.shutdown().await;let _=reply.send(result);break;},
                None=>break,
            }
        };
        if let Err(error) = result {
            tracing::warn!(session_id=%id,error=%error,"worker supervision stopped");
            break;
        }
    }
    let _ = worker.shutdown().await;
    drop(commands);
    if let Err(error) = disconnected(&kernel, id).await {
        tracing::error!(session_id=%id,error=%error,"cannot persist disconnected worker state");
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
) -> Result<Turn> {
    let turn_id = TurnId::new();
    mutate(kernel, id, Some(turn_id), move |session, state| {
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
            kernel.publish_live(id, turn, kind)
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
