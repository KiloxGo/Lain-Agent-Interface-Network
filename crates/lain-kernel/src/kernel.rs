use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};

use anyhow::{Context, Result, anyhow, ensure};
use serde::Serialize;
use tokio::sync::broadcast;

use crate::{
    domain::{
        BackendKind, EventId, RuntimeProfileId, WorkSession, WorkSessionId, WorkSessionState,
    },
    events::{Event, EventKind},
    persistence::Store,
    worker::WorkerState,
};

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
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
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
        tracing::info!(state = ?inner.state, "Kernel shutting down");
        Ok(())
    }

    pub fn list_sessions(&self) -> Result<Vec<WorkSession>> {
        Ok(self.lock()?.sessions.values().cloned().collect())
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
        let mut inner = self.lock()?;
        let mut session = inner
            .sessions
            .get(&id)
            .context("session not found")?
            .clone();
        let from = session.state;
        let mut worker = inner.workers.get(&id).cloned().unwrap_or_default();
        let events = change(&mut session, &mut worker)?;
        session.updated_at_ms = crate::domain::now_ms()?.max(session.updated_at_ms);
        session.validate()?;
        inner.store.save_worker(&session, &worker)?;
        inner.sessions.insert(id, session.clone());
        inner.workers.insert(id, worker);
        if from != session.state {
            self.publish(
                &session,
                EventKind::SessionStateChanged {
                    from,
                    to: session.state,
                },
            );
        }
        for kind in events {
            self.publish_for_turn(&session, turn_id, kind);
        }
        Ok(())
    }

    pub(crate) fn publish_live(
        &self,
        id: WorkSessionId,
        turn_id: Option<crate::domain::TurnId>,
        kind: EventKind,
    ) -> Result<()> {
        let _ = self.events.send(Event {
            id: EventId::new(),
            timestamp_ms: crate::domain::now_ms()?,
            work_session_id: id,
            turn_id,
            kind,
        });
        Ok(())
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
        inner.store.insert(&session)?;
        inner.sessions.insert(session.id, session.clone());
        self.publish(&session, EventKind::SessionCreated);
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
        inner.store.update(&session)?;
        inner.sessions.insert(id, session.clone());
        self.publish(&session, EventKind::SessionStateChanged { from, to: next });
        Ok(session)
    }

    fn publish(&self, session: &WorkSession, kind: EventKind) {
        tracing::info!(session_id = %session.id, state = ?session.state, "session committed");
        self.publish_for_turn(session, None, kind);
    }

    fn publish_for_turn(
        &self,
        session: &WorkSession,
        turn_id: Option<crate::domain::TurnId>,
        kind: EventKind,
    ) {
        // No listeners is normal. Slow subscribers receive Lagged and must resnapshot.
        let _ = self.events.send(Event {
            id: EventId::new(),
            timestamp_ms: session.updated_at_ms,
            work_session_id: session.id,
            turn_id,
            kind,
        });
    }
}
