use super::{Kernel, KernelState};
use crate::{actor::domain::ActorTurn, channels::domain::*};
use anyhow::{Result, ensure};

impl Kernel {
    pub fn memory_grant(&self, id: ChannelId) -> Result<crate::memory::MemoryGrant> {
        self.lock()?.store.memory_grant(id)
    }
    pub fn memory_grant_history(&self, id: ChannelId) -> Result<Vec<crate::memory::MemoryGrant>> {
        self.lock()?.store.memory_grant_history(id)
    }
    pub fn set_memory_grant(
        &self,
        id: ChannelId,
        change: crate::memory::SetMemoryGrant,
    ) -> Result<crate::memory::MemoryGrant> {
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        let result = inner.store.set_memory_grant(id, change)?;
        let _ = self.channel_events.send(());
        Ok(result)
    }
    pub fn memory_effects(&self, id: ChannelId, after: u64) -> Result<serde_json::Value> {
        self.lock()?.store.memory_effects(id, after)
    }
    pub(crate) fn private_memory_context(
        &self,
        turn: crate::actor::domain::ActorTurnId,
        text: &str,
    ) -> Result<serde_json::Value> {
        self.lock()?.store.private_memory_context(turn, text)
    }
    pub(crate) fn private_memory_effect(
        &self,
        turn: crate::actor::domain::ActorTurnId,
        action: crate::actor::domain::ActionId,
    ) -> Result<crate::memory::MemoryEffect> {
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        inner.store.private_memory_effect(turn, action)
    }
    pub(crate) fn channel_deadline(&self, id: ChannelId, now: u64) -> Result<Option<u64>> {
        self.lock()?.store.channel_deadline(id, now)
    }
    pub fn subscribe_channels(&self) -> tokio::sync::broadcast::Receiver<()> {
        self.channel_events.subscribe()
    }
    pub fn channel_bind(&self, spec: BindingSpec, enabled: bool) -> Result<Binding> {
        spec.validate()?;
        crate::profiles::check_sensitive(&spec)?;
        let mut inner = self.lock()?;
        ensure!(inner.state != KernelState::ShuttingDown, "Kernel stopping");
        inner.store.channel_bind(spec, enabled)
    }
    pub fn channel_bindings(&self) -> Result<Vec<Binding>> {
        self.lock()?.store.channel_bindings()
    }
    pub fn channel_statuses(&self) -> Result<Vec<ChannelStatus>> {
        let inner = self.lock()?;
        let connections = self
            .channel_connections
            .lock()
            .map_err(|_| anyhow::anyhow!("channel observation lock poisoned"))?;
        Ok(inner
            .store
            .channel_bindings()?
            .into_iter()
            .map(|binding| {
                let connection = connections.get(&binding.id).cloned().unwrap_or_default();
                ChannelStatus {
                    binding,
                    connection,
                }
            })
            .collect())
    }
    pub fn channel_binding(&self, id: ChannelId) -> Result<Binding> {
        self.lock()?.store.channel_binding(id)
    }
    pub fn channel_grant(&self, id: ChannelId, enabled: bool, revision: u64) -> Result<Binding> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, "Kernel not ready");
        let result = inner.store.channel_grant(id, enabled, revision)?;
        let _ = self.channel_events.send(());
        Ok(result)
    }
    pub(crate) fn channel_receive(&self, id: ChannelId, input: Input) -> Result<Incoming> {
        crate::profiles::check_sensitive(&input)?;
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, "Kernel not ready");
        let result = inner.store.channel_receive(id, input)?;
        let _ = self.channel_events.send(());
        Ok(result)
    }
    pub(crate) fn channel_next_batch(&self, id: ChannelId) -> Result<Option<Batch>> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, "Kernel not ready");
        inner.store.channel_next_batch(id, crate::domain::now_ms()?)
    }
    pub(crate) fn channel_actor_request(&self, batch: &Batch) -> Result<Option<ActorTurn>> {
        self.lock()?.store.channel_actor_request(batch)
    }
    pub(crate) fn channel_link(&self, id: BatchId, turn: &ActorTurn) -> Result<()> {
        self.lock()?.store.channel_link(id, turn)
    }
    pub(crate) fn channel_finish(&self, id: BatchId, turn: &ActorTurn) -> Result<()> {
        self.lock()?.store.channel_finish(id, turn)
    }
    pub(crate) fn channel_claim_delivery(&self, id: ChannelId) -> Result<Option<Delivery>> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, "Kernel not ready");
        inner.store.channel_claim_delivery(id)
    }
    pub(crate) fn channel_may_send(&self, delivery: &Delivery) -> Result<bool> {
        self.lock()?.store.channel_may_send(delivery)
    }
    pub(crate) fn channel_settle(&self, id: DeliveryId, outcome: SendOutcome) -> Result<()> {
        self.lock()?.store.channel_settle(id, outcome)
    }
    pub fn channel_snapshot(&self, id: ChannelId) -> Result<ChannelSnapshot> {
        let inner = self.lock()?;
        let mut snapshot = inner.store.channel_snapshot(id)?;
        snapshot.connection = self
            .channel_connections
            .lock()
            .map_err(|_| anyhow::anyhow!("channel observation lock poisoned"))?
            .get(&id)
            .cloned()
            .unwrap_or_default();
        Ok(snapshot)
    }

    pub(crate) fn channel_connection(
        &self,
        id: ChannelId,
        phase: ConnectionPhase,
        failure: Option<ConnectionFailure>,
    ) -> Result<()> {
        let mut connections = self
            .channel_connections
            .lock()
            .map_err(|_| anyhow::anyhow!("channel observation lock poisoned"))?;
        let phase = if *self.shutdown_signal.borrow() {
            ConnectionPhase::Stopped
        } else {
            phase
        };
        let old = connections.entry(id).or_default();
        let failure = if phase == ConnectionPhase::Online {
            None
        } else {
            failure.or(old.failure)
        };
        if old.phase != phase || old.failure != failure {
            old.phase = phase;
            old.failure = failure;
            old.changed_at_ms = crate::domain::now_ms()?.max(old.changed_at_ms);
            let _ = self.channel_events.send(());
        }
        Ok(())
    }
}
