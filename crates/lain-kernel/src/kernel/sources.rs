use super::{Kernel, KernelState};
use crate::redaction::Redactor;
use anyhow::{Result, ensure};
use lain_wire::{ActionId, ActorTurnId, ChannelId, source::*};
use serde_json::Value;

impl Kernel {
    pub fn source_grant(&self, id: ChannelId) -> Result<SourceGrant> {
        self.lock()?.store.source_grant(id)
    }
    pub fn source_grants(&self, id: ChannelId) -> Result<Vec<SourceGrant>> {
        self.lock()?.store.source_grants(id)
    }
    pub fn set_source_grant(&self, id: ChannelId, change: SetSourceGrant) -> Result<SourceGrant> {
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        let result = inner.store.set_source_grant(id, change)?;
        let _ = self.channel_events.send(());
        Ok(result)
    }
    pub(crate) fn source_search(
        &self,
        id: ChannelId,
        q: SourceQuery,
        redactor: Redactor,
    ) -> Result<SourcePage> {
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        inner.store.source_search(id, q, &redactor)
    }
    pub(crate) fn source_inspect(
        &self,
        id: ChannelId,
        q: InspectSource,
        redactor: Redactor,
    ) -> Result<SourceRecord> {
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        inner.store.source_inspect(id, q, &redactor)
    }
    pub fn source_reads(&self, id: ChannelId, after: u64) -> Result<Value> {
        self.lock()?.store.source_reads(id, after)
    }
    pub(crate) fn source_actor_scope(&self, id: ActorTurnId) -> Result<SourceGrant> {
        let inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        inner.store.source_actor_scope(id)
    }
    pub(crate) fn source_actor_lookup(
        &self,
        id: ActorTurnId,
        action: ActionId,
        redactor: Redactor,
    ) -> Result<(SourceSelection, Value)> {
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        inner.store.source_actor_lookup(id, action, &redactor)
    }
    pub(crate) fn source_actor_disclosure(
        &self,
        id: ActorTurnId,
        selections: &[SourceSelection],
        request: Option<u32>,
        redactor: Redactor,
    ) -> Result<(Vec<String>, Value)> {
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        inner
            .store
            .source_actor_disclosure(id, selections, request, &redactor)
    }
}
