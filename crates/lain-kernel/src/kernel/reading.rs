use super::{Kernel, KernelState};
use crate::{backend::ControlError, domain::now_ms};
use anyhow::{Result, ensure};
use lain_wire::{ReadingId, reading::*};

impl Kernel {
    pub fn reading_grant(&self) -> Result<ReadingGrant> {
        self.lock()?.store.reading_grant()
    }
    pub fn reading_grants(&self, after: u64) -> Result<Vec<ReadingGrant>> {
        self.lock()?.store.reading_grants(after)
    }
    pub fn set_reading_grant(&self, change: SetReadingGrant) -> Result<ReadingGrant> {
        let now = now_ms()?;
        crate::reading::validate_scope(change.scope.as_ref(), now)?;
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        let g = inner.store.set_reading_grant(change, now)?;
        let _ = self.reading_events.send(());
        Ok(g)
    }
    pub(crate) fn admit_reading(&self, request: StartReading) -> Result<(ReadingActivity, bool)> {
        crate::reading::validate_url(&request.url)?;
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        let now = now_ms()?;
        inner.store.prune_reading(now)?;
        let result = inner.store.admit_reading(request, now)?;
        let _ = self.reading_events.send(());
        Ok(result)
    }
    pub(crate) fn claim_reading(&self, id: ReadingId) -> Result<Option<ReadingScope>> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner.store.claim_reading(id, now_ms()?)
    }
    pub(crate) fn reading_running(&self, id: ReadingId) -> Result<bool> {
        let mut inner = self.lock()?;
        let now = now_ms()?;
        inner.store.prune_reading(now)?;
        Ok(inner.state == KernelState::Ready && inner.store.reading_running(id, now)?)
    }
    pub(crate) fn finish_reading(
        &self,
        id: ReadingId,
        result: std::result::Result<String, ReadingFailure>,
    ) -> Result<()> {
        let mut inner = self.lock()?;
        let now = now_ms()?;
        inner.store.prune_reading(now)?;
        if inner.state != KernelState::Ready || !inner.store.reading_running(id, now)? {
            return Ok(());
        }
        inner.store.finish_reading(id, result, now)?;
        let _ = self.reading_events.send(());
        Ok(())
    }
    pub fn reading_activities(&self) -> Result<Vec<ReadingActivity>> {
        let mut inner = self.lock()?;
        inner.store.prune_reading(now_ms()?)?;
        inner.store.reading_activities()
    }
    pub(crate) fn reading_result(
        &self,
        id: ReadingId,
        redactor: crate::redaction::Redactor,
    ) -> Result<ReadingResult> {
        let mut r = self.lock()?.store.reading_result(id, now_ms()?)?;
        if r.text.as_ref().is_some_and(|t| redactor.text(t) != *t) {
            r.text = None;
        }
        Ok(r)
    }
    pub fn cancel_reading(&self, id: ReadingId) -> Result<ReadingActivity> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        let a = inner.store.cancel_reading(id, now_ms()?)?;
        let _ = self.reading_events.send(());
        Ok(a)
    }
    pub(crate) fn subscribe_reading(&self) -> tokio::sync::broadcast::Receiver<()> {
        self.reading_events.subscribe()
    }
    pub(crate) fn reading_deadline(&self) -> Result<Option<u64>> {
        let mut inner = self.lock()?;
        inner.store.prune_reading(now_ms()?)?;
        inner.store.reading_deadline()
    }
}
