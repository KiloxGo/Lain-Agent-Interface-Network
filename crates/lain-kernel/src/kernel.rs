use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};

use anyhow::{Context, Result, anyhow, ensure};
use serde::Serialize;
use tokio::sync::broadcast;

use crate::{
    domain::{BackendKind, RuntimeProfileId, WorkSession, WorkSessionId, WorkSessionState},
    events::{Event, EventKind, HistoryPage},
    persistence::Store,
    worker::WorkerState,
};
mod channels;
mod private_chat;
mod reading;
pub(crate) mod sharing;
mod sources;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelState {
    Booting,
    Recovering,
    Ready,
    ShuttingDown,
}

struct Inner {
    state: KernelState,
    sessions: BTreeMap<WorkSessionId, WorkSession>,
    store: Store,
    workers: BTreeMap<WorkSessionId, WorkerState>,
}

pub struct Kernel {
    // ponytail: one lock orders durable writes; async access uses blocking tasks.
    // Split storage/projections only if measured contention warrants it.
    inner: Mutex<Inner>,
    events: broadcast::Sender<Event>,
    shutdown_signal: tokio::sync::watch::Sender<bool>,
    channel_events: broadcast::Sender<()>,
    reading_events: broadcast::Sender<()>,
    // Ephemeral transport observations never wait on SQLite or wake the Actor model.
    channel_connections: Mutex<
        BTreeMap<crate::channels::domain::ChannelId, crate::channels::domain::ConnectionStatus>,
    >,
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub history_cursor: u64,
    pub sessions: Vec<WorkSession>,
    pub turns: Vec<crate::domain::Turn>,
    pub approvals: Vec<crate::domain::Approval>,
}

/// Even a projection read can wait behind a SQLite writer. Keep that wait off
/// async runtime threads so protocol readers never depend on storage latency.
pub(crate) async fn blocking<T: Send + 'static>(
    kernel: &Arc<Kernel>,
    operation: impl FnOnce(&Kernel) -> Result<T> + Send + 'static,
) -> Result<T> {
    let kernel = kernel.clone();
    tokio::task::spawn_blocking(move || operation(&kernel))
        .await
        .context("Kernel storage task failed")?
}

impl Kernel {
    /// Blocking startup: call before serving, or from spawn_blocking.
    pub fn open(path: &Path) -> Result<Self> {
        tracing::info!(state = ?KernelState::Booting, "opening operational storage");
        let mut store = Store::open(path)?;
        store.profiles().context("load Profile Registry")?;
        store
            .recover_actor()
            .context("recover Lain conversations")?;
        store
            .recover_channels()
            .context("recover channel deliveries")?;
        store.recover_reading(crate::domain::now_ms()?)?;
        store.recover_sharing()?;
        tracing::info!(state = ?KernelState::Recovering, "recovering sessions");
        let (sessions, workers) = store.recover().context("recover Kernel state")?;
        tracing::info!(sessions = sessions.len(), "session recovery committed");
        Ok(Self {
            inner: Mutex::new(Inner {
                state: KernelState::Recovering,
                sessions: sessions.into_iter().map(|s| (s.id, s)).collect(),
                store,
                workers,
            }),
            events: broadcast::channel(256).0,
            shutdown_signal: tokio::sync::watch::channel(false).0,
            channel_events: broadcast::channel(64).0,
            reading_events: broadcast::channel(16).0,
            channel_connections: Mutex::new(BTreeMap::new()),
        })
    }

    fn lock(&self) -> Result<MutexGuard<'_, Inner>> {
        self.inner
            .lock()
            .map_err(|_| anyhow!("Kernel state lock poisoned"))
    }

    pub fn state(&self) -> Result<KernelState> {
        Ok(self.lock()?.state)
    }

    /// Called only once the API listener is bound.
    pub fn mark_ready(&self) -> Result<()> {
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Recovering,
            "Kernel is not recovering"
        );
        inner.state = KernelState::Ready;
        tracing::info!(state = ?inner.state, "Kernel ready");
        Ok(())
    }

    pub fn shutdown(&self) -> Result<()> {
        let mut inner = self.lock()?;
        inner.state = KernelState::ShuttingDown;
        self.shutdown_signal.send_replace(true);
        for status in self
            .channel_connections
            .lock()
            .map_err(|_| anyhow!("channel observation lock poisoned"))?
            .values_mut()
        {
            status.phase = crate::channels::domain::ConnectionPhase::Stopped;
            status.changed_at_ms = crate::domain::now_ms()?.max(status.changed_at_ms);
        }
        tracing::info!(state = ?inner.state, "Kernel shutting down");
        Ok(())
    }

    pub fn list_sessions(&self) -> Result<Vec<WorkSession>> {
        Ok(self.lock()?.sessions.values().cloned().collect())
    }

    pub fn profiles(&self) -> Result<Vec<crate::profiles::RuntimeProfile>> {
        self.lock()?.store.profiles()
    }

    pub fn save_profile(
        &self,
        definition: crate::profiles::ProfileDefinition,
        expected_revision: Option<u64>,
    ) -> Result<crate::profiles::RuntimeProfile> {
        definition.validate()?;
        crate::profiles::check_sensitive(&definition)?;
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            "Kernel is not ready for mutations"
        );
        let old = inner
            .store
            .profiles()?
            .into_iter()
            .find(|p| p.definition.id == definition.id);
        ensure!(
            old.as_ref().map(|p| p.revision) == expected_revision,
            crate::backend::ControlError::Conflict
        );
        let revision = expected_revision
            .unwrap_or(0)
            .checked_add(1)
            .context("profile revision exhausted")?;
        let profile = crate::profiles::RuntimeProfile {
            definition,
            revision,
        };
        inner.store.save_profile(&profile, old.is_none())?;
        Ok(profile)
    }

    pub fn workspace_policy(
        &self,
        workspace: PathBuf,
    ) -> Result<Option<crate::profiles::WorkspacePolicy>> {
        let workspace = workspace
            .canonicalize()
            .map_err(|_| crate::backend::ControlError::InvalidInput)?;
        self.lock()?.store.policy(&workspace)
    }
    pub fn save_workspace_policy(
        &self,
        mut policy: crate::profiles::WorkspacePolicy,
    ) -> Result<crate::profiles::WorkspacePolicy> {
        policy.workspace = policy
            .workspace
            .canonicalize()
            .map_err(|_| crate::backend::ControlError::InvalidInput)?;
        ensure!(
            policy.workspace.is_dir(),
            crate::backend::ControlError::InvalidInput
        );
        policy.validate()?;
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            "Kernel is not ready for mutations"
        );
        let profiles = inner.store.profiles()?;
        ensure!(
            policy
                .allowed_profiles
                .iter()
                .all(|id| profiles.iter().any(|p| p.definition.id == *id)),
            crate::backend::ControlError::NotFound
        );
        inner.store.save_policy(&policy)?;
        Ok(policy)
    }
    pub(crate) fn check_workspace_profile(
        &self,
        workspace: &Path,
        profile: &RuntimeProfileId,
    ) -> Result<()> {
        if let Some(policy) = self.lock()?.store.policy(workspace)? {
            policy.validate()?;
            ensure!(
                policy.allowed_profiles.contains(profile),
                crate::backend::ControlError::ProfileDenied
            );
        }
        Ok(())
    }
    /// Profile selection and frozen metadata commit under the same registry/storage lock.
    pub(crate) fn create_profiled_session(
        &self,
        workspace: PathBuf,
        requested: Option<RuntimeProfileId>,
        options: crate::domain::SessionOptions,
        receipt: Option<(
            crate::actor::domain::ActorTurnId,
            crate::actor::domain::ActionId,
        )>,
    ) -> Result<WorkSession> {
        crate::profiles::check_sensitive(&options)?;
        let workspace = workspace
            .canonicalize()
            .map_err(|_| crate::backend::ControlError::InvalidInput)?;
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        let policy = inner.store.policy(&workspace)?;
        let profile_id = requested
            .or_else(|| policy.as_ref().map(|p| p.default_profile.clone()))
            .unwrap_or_else(|| crate::profiles::ProfileDefinition::local().id);
        if let Some(policy) = policy {
            policy.validate()?;
            ensure!(
                policy.allowed_profiles.contains(&profile_id),
                crate::backend::ControlError::ProfileDenied
            );
        }
        let profile = inner
            .store
            .profiles()?
            .into_iter()
            .find(|p| p.definition.id == profile_id)
            .ok_or(crate::backend::ControlError::NotFound)?;
        ensure!(
            profile.definition.enabled,
            crate::backend::ControlError::Unavailable
        );
        let resolved = profile.resolve(&options)?;
        ensure!(
            crate::profiles::secret_status(resolved.secret_ref.as_ref()) != "missing",
            crate::backend::ControlError::Unavailable
        );
        let session = WorkSession::new(resolved.backend.clone(), profile_id, workspace)?;
        let worker = WorkerState {
            options,
            resolved_profile: Some(resolved),
            ..Default::default()
        };
        let mut actor_receipt = if let Some((actor_id, action_id)) = receipt {
            use crate::actor::domain::{ActionState, TurnState};
            let mut turn = inner.store.actor_turn(actor_id)?;
            ensure!(
                turn.state == TurnState::Running,
                crate::backend::ControlError::Conflict
            );
            let action = turn
                .actions
                .iter_mut()
                .find(|a| a.id == action_id)
                .ok_or(crate::backend::ControlError::NotFound)?;
            ensure!(
                action.state == ActionState::Executing && action.result.is_none(),
                crate::backend::ControlError::Conflict
            );
            let crate::actor::tools::Tool::WorkCreate(arguments) = &action.tool else {
                anyhow::bail!(crate::backend::ControlError::InvalidInput);
            };
            ensure!(
                arguments.workspace.canonicalize()? == session.workspace
                    && arguments.profile_id == session.runtime_profile_id
                    && worker.options.sandbox == Some(arguments.sandbox)
                    && worker.options.model.is_none(),
                crate::backend::ControlError::InvalidInput
            );
            action.result = Some(
                serde_json::json!({"work_session_id":session.id,"stage":"created_not_submitted"}),
            );
            Some(turn)
        } else {
            None
        };
        let mut events = vec![Event::new(
            session.id,
            None,
            session.updated_at_ms,
            EventKind::SessionCreated {
                session: session.clone(),
            },
        )];
        inner.store.save_with_actor_receipt(
            &session,
            Some(&worker),
            true,
            &mut events,
            actor_receipt.as_mut(),
        )?;
        inner.sessions.insert(session.id, session.clone());
        inner.workers.insert(session.id, worker);
        self.publish(events);
        Ok(session)
    }
    pub fn resolved_profile(
        &self,
        id: WorkSessionId,
    ) -> Result<crate::profiles::ResolvedRuntimeProfile> {
        self.lock()?
            .workers
            .get(&id)
            .and_then(|w| w.resolved_profile.clone())
            .ok_or_else(|| crate::backend::ControlError::NotFound.into())
    }

    pub fn session(&self, id: WorkSessionId) -> Result<Option<WorkSession>> {
        Ok(self.lock()?.sessions.get(&id).cloned())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.events.subscribe()
    }

    pub fn subscribe_shutdown(&self) -> tokio::sync::watch::Receiver<bool> {
        self.shutdown_signal.subscribe()
    }

    pub fn snapshot_and_subscribe(&self) -> Result<(Snapshot, broadcast::Receiver<Event>)> {
        let inner = self.lock()?;
        let events = self.events.subscribe();
        Ok((
            Snapshot {
                history_cursor: inner.store.history_cursor()?,
                sessions: inner.sessions.values().cloned().collect(),
                turns: inner
                    .workers
                    .values()
                    .flat_map(|w| w.turns.iter().map(|r| r.turn.clone()))
                    .collect(),
                approvals: inner
                    .workers
                    .values()
                    .flat_map(|w| w.approvals.clone())
                    .collect(),
            },
            events,
        ))
    }

    pub fn turns(&self, id: WorkSessionId) -> Result<Vec<crate::domain::Turn>> {
        let inner = self.lock()?;
        ensure!(inner.sessions.contains_key(&id), "session not found");
        Ok(inner
            .workers
            .get(&id)
            .map(|w| w.turns.iter().map(|t| t.turn.clone()).collect())
            .unwrap_or_default())
    }

    pub fn approvals(&self, id: WorkSessionId) -> Result<Vec<crate::domain::Approval>> {
        let inner = self.lock()?;
        ensure!(inner.sessions.contains_key(&id), "session not found");
        Ok(inner
            .workers
            .get(&id)
            .map(|w| w.approvals.clone())
            .unwrap_or_default())
    }

    pub(crate) fn worker_state(&self, id: WorkSessionId) -> Result<WorkerState> {
        Ok(self
            .lock()?
            .workers
            .get(&id)
            .context("worker state not found")?
            .clone())
    }

    /// Call from spawn_blocking. One transaction commits session/turn/approval metadata.
    pub(crate) fn mutate_worker(
        &self,
        id: WorkSessionId,
        turn_id: Option<crate::domain::TurnId>,
        change: impl FnOnce(&mut WorkSession, &mut WorkerState) -> Result<Vec<EventKind>>,
    ) -> Result<()> {
        self.mutate_worker_with_origin(id, turn_id, None, change)
    }

    pub(crate) fn mutate_worker_with_origin(
        &self,
        id: WorkSessionId,
        turn_id: Option<crate::domain::TurnId>,
        origin: Option<(crate::delegation::Origin, String)>,
        change: impl FnOnce(&mut WorkSession, &mut WorkerState) -> Result<Vec<EventKind>>,
    ) -> Result<()> {
        let mut inner = self.lock()?;
        let mut session = inner
            .sessions
            .get(&id)
            .context("session not found")?
            .clone();
        let from = session.state;
        let runtime_from = session.runtime;
        let mut worker = inner.workers.get(&id).cloned().unwrap_or_default();
        let mut kinds = change(&mut session, &mut worker)?;
        // Reconciliation and pre-submission transitions also need durable turn
        // observations; do not rely on a frontend refresh after transport changes.
        for record in &worker.turns {
            let before = inner
                .workers
                .get(&id)
                .and_then(|w| w.turns.iter().find(|r| r.turn.id == record.turn.id));
            if before.is_none_or(|r| r.turn.state != record.turn.state)
                && !kinds.iter().any(|kind| matches!(kind, EventKind::TurnStarted {turn} | EventKind::TurnCompleted {turn} | EventKind::TurnStateChanged {turn} if turn.id == record.turn.id)) {
                kinds.push(EventKind::TurnStateChanged { turn: record.turn.clone() });
            }
        }
        session.updated_at_ms = crate::domain::now_ms()?.max(session.updated_at_ms);
        session.validate()?;
        let mut events = Vec::new();
        if from != session.state {
            events.push(Event::new(
                id,
                None,
                session.updated_at_ms,
                EventKind::SessionStateChanged {
                    from,
                    to: session.state,
                },
            ));
        }
        if runtime_from != session.runtime {
            events.push(Event::new(
                id,
                None,
                session.updated_at_ms,
                EventKind::RuntimeAttachmentChanged {
                    from: runtime_from,
                    to: session.runtime,
                },
            ));
        }
        events.extend(kinds.into_iter().map(|kind| {
            let turn = turn_id.or(match &kind {
                EventKind::ApprovalDecisionSubmitted { approval_id } => worker
                    .approvals
                    .iter()
                    .find(|a| a.id == *approval_id)
                    .and_then(|a| a.turn_id),
                _ => None,
            });
            Event::new(id, turn, session.updated_at_ms, kind)
        }));
        let mut receipt = if let Some((origin, text)) = origin {
            use crate::actor::{
                domain::{ActionState, TurnState},
                tools::Tool,
            };
            let mut actor = inner.store.actor_turn(origin.actor_turn_id)?;
            ensure!(
                actor.state == TurnState::Running,
                crate::backend::ControlError::Conflict
            );
            let action = actor
                .actions
                .iter_mut()
                .find(|a| a.id == origin.action_id)
                .ok_or(crate::backend::ControlError::NotFound)?;
            ensure!(
                action.state == ActionState::Executing,
                crate::backend::ControlError::Conflict
            );
            let valid = match &action.tool {
                Tool::WorkCreate(args) => {
                    args.task == text
                        && args.workspace.canonicalize()? == session.workspace
                        && args.profile_id == session.runtime_profile_id
                        && worker.options.sandbox == Some(args.sandbox)
                        && action
                            .result
                            .as_ref()
                            .and_then(|v| v.get("stage"))
                            .and_then(|v| v.as_str())
                            == Some("created_not_submitted")
                        && action
                            .result
                            .as_ref()
                            .and_then(|v| v.get("turn_id"))
                            .is_none()
                        && action
                            .result
                            .as_ref()
                            .and_then(|v| v.get("work_session_id"))
                            .and_then(|v| v.as_str())
                            == Some(&id.to_string())
                }
                Tool::WorkSend(args) => {
                    args.session_id == id && args.text == text && action.result.is_none()
                }
                _ => false,
            };
            ensure!(valid, crate::backend::ControlError::Conflict);
            let turn_id = turn_id.context("delegated prompt lacks worker turn")?;
            ensure!(
                worker
                    .turns
                    .iter()
                    .any(|r| r.turn.id == turn_id
                        && r.turn.state == crate::domain::TurnState::Starting),
                crate::backend::ControlError::Conflict
            );
            action.result = Some(
                serde_json::json!({"work_session_id":id,"turn_id":turn_id,"stage":"submitting","worker_completion":"not_observed"}),
            );
            Some(actor)
        } else {
            None
        };
        inner.store.save_with_actor_receipt(
            &session,
            Some(&worker),
            false,
            &mut events,
            receipt.as_mut(),
        )?;
        inner.sessions.insert(id, session);
        inner.workers.insert(id, worker);
        self.publish(events);
        Ok(())
    }

    pub(crate) fn publish_live(
        &self,
        id: WorkSessionId,
        turn_id: Option<crate::domain::TurnId>,
        kind: EventKind,
    ) -> Result<()> {
        ensure!(
            matches!(kind, EventKind::OutputProgress { .. }),
            "only progress is ephemeral"
        );
        let _ = self
            .events
            .send(Event::new(id, turn_id, crate::domain::now_ms()?, kind));
        Ok(())
    }

    pub(crate) fn observe(
        &self,
        id: WorkSessionId,
        turn_id: Option<crate::domain::TurnId>,
        kind: EventKind,
    ) -> Result<()> {
        let mut inner = self.lock()?;
        ensure!(inner.sessions.contains_key(&id), "session not found");
        let mut events = vec![Event::new(id, turn_id, crate::domain::now_ms()?, kind)];
        inner.store.append_events(&mut events)?;
        self.publish(events);
        Ok(())
    }

    pub fn history(
        &self,
        id: WorkSessionId,
        after: u64,
        until: Option<u64>,
        limit: usize,
    ) -> Result<HistoryPage> {
        ensure!(
            (1..=200).contains(&limit),
            "history limit must be between 1 and 200"
        );
        ensure!(
            after <= i64::MAX as u64 && until.is_none_or(|n| n <= i64::MAX as u64),
            "invalid history cursor"
        );
        let inner = self.lock()?;
        ensure!(inner.sessions.contains_key(&id), "session not found");
        inner.store.history(
            id,
            after,
            until.unwrap_or(inner.store.history_cursor()?),
            limit,
            false,
        )
    }

    pub fn history_before(
        &self,
        id: WorkSessionId,
        before: u64,
        until: Option<u64>,
        limit: usize,
    ) -> Result<HistoryPage> {
        ensure!(
            (1..=200).contains(&limit)
                && before > 0
                && before <= i64::MAX as u64
                && until.is_none_or(|n| n <= i64::MAX as u64),
            "invalid history query"
        );
        let inner = self.lock()?;
        ensure!(inner.sessions.contains_key(&id), "session not found");
        inner.store.history(
            id,
            0,
            until
                .unwrap_or(inner.store.history_cursor()?)
                .min(before.saturating_sub(1)),
            limit,
            true,
        )
    }

    /// Internal foundation operation; creates metadata, never starts a worker.
    pub fn create_session(
        &self,
        backend: BackendKind,
        profile: RuntimeProfileId,
        workspace: PathBuf,
    ) -> Result<WorkSession> {
        let session = WorkSession::new(backend, profile, workspace)?;
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            "Kernel is not ready for mutations"
        );
        let mut events = vec![Event::new(
            session.id,
            None,
            session.updated_at_ms,
            EventKind::SessionCreated {
                session: session.clone(),
            },
        )];
        inner.store.save(&session, None, true, &mut events)?;
        inner.sessions.insert(session.id, session.clone());
        self.publish(events);
        Ok(session)
    }

    /// State observations require evidence from the future supervisor/adapter.
    /// This is not a cancellation command or a worker execution API.
    pub fn transition_session(
        &self,
        id: WorkSessionId,
        next: WorkSessionState,
    ) -> Result<WorkSession> {
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            "Kernel is not ready for mutations"
        );
        let mut session = inner
            .sessions
            .get(&id)
            .context("session not found")?
            .clone();
        let from = session.state;
        session.transition(next)?;
        let mut events = vec![Event::new(
            id,
            None,
            session.updated_at_ms,
            EventKind::SessionStateChanged { from, to: next },
        )];
        inner.store.save(&session, None, false, &mut events)?;
        inner.sessions.insert(id, session.clone());
        self.publish(events);
        Ok(session)
    }

    fn publish(&self, events: Vec<Event>) {
        // No listeners is normal. Slow subscribers receive Lagged and must resnapshot.
        for event in events {
            let _ = self.events.send(event);
        }
    }
}

impl Kernel {
    pub fn delegations(
        &self,
        conversation: crate::actor::domain::ConversationId,
    ) -> Result<Vec<crate::delegation::Delegation>> {
        let inner = self.lock()?;
        inner.store.actor_page(conversation, 0, 1)?;
        inner.store.delegations(conversation)
    }
    pub fn notices(
        &self,
        conversation: crate::actor::domain::ConversationId,
        after: u64,
        limit: usize,
    ) -> Result<crate::delegation::NoticePage> {
        self.lock()?.store.notices(conversation, after, limit)
    }
    pub fn notices_and_subscribe(
        &self,
        conversation: crate::actor::domain::ConversationId,
        after: u64,
    ) -> Result<(crate::delegation::NoticePage, broadcast::Receiver<Event>)> {
        let inner = self.lock()?;
        let receiver = self.events.subscribe();
        Ok((inner.store.notices(conversation, after, 100)?, receiver))
    }
    pub fn acknowledge_notice(
        &self,
        conversation: crate::actor::domain::ConversationId,
        id: crate::delegation::NoticeId,
    ) -> Result<()> {
        self.lock()?.store.acknowledge_notice(conversation, id)
    }
    pub fn create_conversation(&self) -> Result<crate::actor::domain::Conversation> {
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        inner.store.create_conversation()
    }
    pub fn conversations(&self) -> Result<Vec<crate::actor::domain::Conversation>> {
        self.lock()?.store.conversations()
    }
    pub fn actor_page(
        &self,
        id: crate::actor::domain::ConversationId,
        after: u64,
        limit: usize,
    ) -> Result<crate::actor::domain::ConversationPage> {
        ensure!(
            after <= i64::MAX as u64 && (1..=100).contains(&limit),
            crate::backend::ControlError::InvalidInput
        );
        self.lock()?.store.actor_page(id, after, limit)
    }
    pub fn actor_recent(
        &self,
        id: crate::actor::domain::ConversationId,
    ) -> Result<Vec<crate::actor::domain::ActorTurn>> {
        self.lock()?.store.actor_recent(id)
    }
    pub fn actor_turn(
        &self,
        id: crate::actor::domain::ActorTurnId,
    ) -> Result<crate::actor::domain::ActorTurn> {
        self.lock()?.store.actor_turn(id)
    }
    pub fn actor_submit(
        &self,
        conversation: crate::actor::domain::ConversationId,
        request: uuid::Uuid,
        text: String,
    ) -> Result<(crate::actor::domain::ActorTurn, bool)> {
        self.actor_submit_scoped(conversation, request, text, None)
    }
    pub(crate) fn actor_submit_scoped(
        &self,
        conversation: crate::actor::domain::ConversationId,
        request: uuid::Uuid,
        text: String,
        channel: Option<(crate::channels::domain::ChannelId, u64, u64)>,
    ) -> Result<(crate::actor::domain::ActorTurn, bool)> {
        use crate::actor::domain::*;
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        if let Some(turn) = inner.store.actor_duplicate(conversation, request)? {
            ensure!(
                turn.messages.first().is_some_and(|m| m.content == text),
                crate::backend::ControlError::Conflict
            );
            return Ok((turn, false));
        }
        if let Some((id, revision, expires_at_ms)) = channel {
            let binding = inner.store.channel_binding(id)?;
            ensure!(
                binding.reply_enabled
                    && binding.grant_revision == revision
                    && binding.conversation_id == conversation
                    && crate::domain::now_ms()? < expires_at_ms,
                crate::backend::ControlError::InvalidInput
            );
        }
        let channel_memory = channel
            .map(|(id, _, _)| {
                inner
                    .store
                    .channel_memory_context(id, lain_wire::BatchId(request))
            })
            .transpose()?
            .flatten();
        let channel_source = channel
            .map(|(id, _, _)| inner.store.source_context(id, lain_wire::BatchId(request)))
            .transpose()?
            .flatten();
        let private_chat = channel
            .map(|(id, _, _)| {
                inner
                    .store
                    .private_chat_context(id, lain_wire::BatchId(request))
            })
            .transpose()?;
        inner.store.actor_page(conversation, 0, 1)?;
        ensure!(
            !inner
                .store
                .actor_recent(conversation)?
                .iter()
                .any(|t| t.state.active()),
            crate::backend::ControlError::Conflict
        );
        let mut turn = ActorTurn {
            reading_activity: None,
            sharing_activity: None,
            id: ActorTurnId::new(),
            conversation_id: conversation,
            request_id: request,
            sequence: 0,
            revision: 0,
            state: TurnState::Running,
            messages: Vec::new(),
            actions: Vec::new(),
            error: None,
            model_requests: 0,
            usage_tokens: 0,
            timing: Default::default(),
            channel_memory,
            channel_source,
            private_chat,
        };
        turn.message(MessageRole::User, text)?;
        inner.store.save_actor_turn(&mut turn, true)?;
        Ok((turn, true))
    }
    pub(crate) fn actor_update(
        &self,
        id: crate::actor::domain::ActorTurnId,
        change: impl FnOnce(&mut crate::actor::domain::ActorTurn) -> Result<()>,
    ) -> Result<crate::actor::domain::ActorTurn> {
        let mut inner = self.lock()?;
        let mut turn = inner.store.actor_turn(id)?;
        let reading_activity = turn.reading_activity;
        let sharing_activity = turn.sharing_activity;
        let was_running = turn.state == crate::actor::domain::TurnState::Running;
        let old_actions = turn.actions.len();
        let old_assistant = turn
            .messages
            .iter()
            .filter(|m| m.role == crate::actor::domain::MessageRole::Assistant)
            .count();
        change(&mut turn)?;
        ensure!(
            turn.reading_activity == reading_activity && turn.sharing_activity == sharing_activity,
            crate::backend::ControlError::InvalidInput
        );
        if turn
            .private_chat
            .as_ref()
            .is_some_and(|c| c.experience_grant_revision.is_some())
        {
            for action in turn.actions.iter().skip(old_actions) {
                ensure!(
                    !matches!(
                        action.tool,
                        crate::actor::tools::Tool::MemoryPropose(_)
                            | crate::actor::tools::Tool::MemoryRemember(_)
                            | crate::actor::tools::Tool::MemoryCorrect(_)
                    ),
                    crate::backend::ControlError::InvalidInput
                );
            }
        }
        if turn.private_chat.is_some()
            && (turn.actions.len() > old_actions
                || (turn.private_chat.as_ref().is_some_and(|c| {
                    !matches!(
                        c.decision,
                        lain_wire::chat::ChatDecision::Notice
                            | lain_wire::chat::ChatDecision::Interrupted
                            | lain_wire::chat::ChatDecision::Superseded
                    )
                }) && (turn
                    .messages
                    .iter()
                    .filter(|m| m.role == crate::actor::domain::MessageRole::Assistant)
                    .count()
                    > old_assistant
                    || (was_running && turn.state == crate::actor::domain::TurnState::Completed))))
        {
            inner.store.private_chat_check(&turn)?;
        }
        inner.store.save_actor_turn(&mut turn, false)?;
        if turn.reading_activity.is_some()
            && turn.state == crate::actor::domain::TurnState::Completed
        {
            let _ = self.channel_events.send(());
        }
        Ok(turn)
    }
    pub fn storage_healthy(&self) -> Result<bool> {
        self.lock()?.store.storage_healthy()
    }
}

impl Kernel {
    pub fn memory(&self, id: crate::memory::MemoryId) -> Result<crate::memory::MemoryRecord> {
        self.lock()?.store.memory(id)
    }
    pub fn memory_page(
        &self,
        mut query: crate::memory::MemoryQuery,
    ) -> Result<crate::memory::MemoryPage> {
        query.scope = crate::memory::canonical_scope(query.scope)?;
        self.lock()?.store.memory_page(&query)
    }
    pub fn add_memory(
        &self,
        mut input: crate::memory::AddMemory,
    ) -> Result<crate::memory::MemoryRecord> {
        input.draft.scope = crate::memory::canonical_scope(input.draft.scope)?;
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        inner.store.add_memory(&input)
    }
    pub fn change_memory(
        &self,
        id: crate::memory::MemoryId,
        change: crate::memory::ChangeMemory,
    ) -> Result<crate::memory::MemoryRecord> {
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        inner.store.change_memory(id, &change)
    }
    pub fn delete_memory(&self, id: crate::memory::MemoryId, revision: u64) -> Result<()> {
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        inner.store.delete_memory(id, revision)
    }
    pub fn memory_candidate(
        &self,
        id: crate::memory::CandidateId,
    ) -> Result<crate::memory::MemoryCandidate> {
        self.lock()?.store.candidate(id)
    }
    pub fn memory_candidates(&self, after: u64, limit: u16) -> Result<serde_json::Value> {
        self.lock()?.store.candidates(after, limit)
    }
    pub fn propose_summary_memory(
        &self,
        request: lain_wire::RequestId,
        summary: crate::memory::SummaryId,
        mut draft: crate::memory::MemoryDraft,
    ) -> Result<crate::memory::MemoryCandidate> {
        draft.scope = crate::memory::canonical_scope(draft.scope)?;
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        inner.store.summary_candidate(request, summary, draft)
    }
    pub fn review_memory_candidate(
        &self,
        id: crate::memory::CandidateId,
        accept: bool,
    ) -> Result<crate::memory::MemoryCandidate> {
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        inner.store.review_candidate(id, accept, None)
    }
    pub(crate) fn accept_actor_memory(
        &self,
        turn: crate::actor::domain::ActorTurnId,
        action: crate::actor::domain::ActionId,
    ) -> Result<crate::memory::MemoryCandidate> {
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        let t = inner.store.actor_turn(turn)?;
        ensure!(
            t.state == crate::actor::domain::TurnState::Running
                && t.actions.iter().any(|a| a.id == action
                    && a.state == crate::actor::domain::ActionState::Executing
                    && matches!(a.tool, crate::actor::tools::Tool::MemoryPropose(_))),
            crate::backend::ControlError::Conflict
        );
        inner
            .store
            .review_candidate(crate::memory::CandidateId(action.0), true, Some(action))
    }
    pub fn session_summary(
        &self,
        id: crate::memory::SummaryId,
    ) -> Result<crate::memory::SessionSummary> {
        self.lock()?.store.summary(id)
    }
    pub fn session_summaries(
        &self,
        session: WorkSessionId,
    ) -> Result<Vec<crate::memory::SessionSummary>> {
        self.lock()?.store.summaries(session)
    }
    pub fn create_session_summary(
        &self,
        session: WorkSessionId,
        turn: crate::domain::TurnId,
    ) -> Result<crate::memory::SessionSummary> {
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        let session = inner
            .sessions
            .get(&session)
            .ok_or(crate::backend::ControlError::NotFound)?
            .clone();
        let turn = inner
            .workers
            .get(&session.id)
            .and_then(|w| w.turns.iter().find(|t| t.turn.id == turn))
            .ok_or(crate::backend::ControlError::NotFound)?
            .turn
            .clone();
        inner.store.create_summary(&session, &turn)
    }
}
