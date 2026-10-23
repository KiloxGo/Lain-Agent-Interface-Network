use super::{Kernel, KernelState};
use crate::{actor::domain::ActorTurn, channels::domain::*};
use anyhow::{Result, ensure};

impl Kernel {
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
    pub(crate) fn channel_settle(&self, id: DeliveryId, outcome: SendOutcome) -> Result<()> {
        self.lock()?.store.channel_settle(id, outcome)
    }
    pub fn channel_snapshot(&self, id: ChannelId) -> Result<ChannelSnapshot> {
        self.lock()?.store.channel_snapshot(id)
    }
}
