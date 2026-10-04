use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
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
}

pub struct Kernel {
    // ponytail: one lock serializes durable writes and event order; move writes to a
    // dedicated storage task when real backend traffic requires async throughput.
    inner: Mutex<Inner>,
    events: broadcast::Sender<Event>,
}

impl Kernel {
    /// Blocking startup: call before serving, or from spawn_blocking.
    pub fn open(path: &Path) -> Result<Self> {
        tracing::info!(state = ?KernelState::Booting, "opening operational storage");
        let mut store = Store::open(path)?;
        tracing::info!(state = ?KernelState::Recovering, "recovering sessions");
        let sessions = store.recover().context("recover Kernel state")?;
        tracing::info!(sessions = sessions.len(), "session recovery committed");
        Ok(Self {
            inner: Mutex::new(Inner {
                state: KernelState::Recovering,
                sessions: sessions.into_iter().map(|s| (s.id, s)).collect(),
                store,
            }),
            events: broadcast::channel(256).0,
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
        // No listeners is normal. Slow subscribers receive Lagged and must resnapshot.
        let _ = self.events.send(Event {
            id: EventId::new(),
            timestamp_ms: session.updated_at_ms,
            work_session_id: session.id,
            turn_id: None,
            kind,
        });
    }
}
