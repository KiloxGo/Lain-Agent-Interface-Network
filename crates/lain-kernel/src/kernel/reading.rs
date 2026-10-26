use super::{Kernel, KernelState};
use crate::{backend::ControlError, domain::now_ms};
use anyhow::{Result, ensure};
use lain_wire::{ReadingId, reading::*};

impl Kernel {
    pub fn reading_budget(&self) -> Result<serde_json::Value> {
        self.lock()?.store.reading_budget()
    }
    pub fn reading_audit(&self) -> Result<serde_json::Value> {
        self.lock()?.store.reading_audit()
    }
    pub fn experience_history(&self, id: lain_wire::ExperienceId) -> Result<serde_json::Value> {
        self.lock()?.store.experience_history(id)
    }
    pub(crate) fn correct_experience(
        &self,
        id: lain_wire::ExperienceId,
        q: CorrectExperience,
    ) -> Result<ReadingExperience> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        let result = inner.store.correct_experience(id, q)?;
        let _ = self.channel_events.send(());
        Ok(result)
    }
    pub fn reading_suppressions(&self) -> Result<serde_json::Value> {
        self.lock()?.store.reading_suppressions()
    }
    pub fn restore_reading_source(&self, q: RestoreReadingSource) -> Result<()> {
        crate::reading::validate_url(&q.url)?;
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner.store.restore_reading_source(q.url)
    }
    pub(crate) fn experience_actor_scope(
        &self,
        id: lain_wire::ActorTurnId,
    ) -> Result<serde_json::Value> {
        let inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner.store.experience_actor_scope(id)
    }
    pub(crate) fn experience_actor_lookup(
        &self,
        id: lain_wire::ActorTurnId,
        action: lain_wire::ActionId,
        redactor: crate::redaction::Redactor,
    ) -> Result<(Vec<lain_wire::ExperienceId>, serde_json::Value)> {
        self.lock()?
            .store
            .experience_actor_lookup(id, action, &redactor)
    }
    pub(crate) fn experience_actor_disclosure(
        &self,
        id: lain_wire::ActorTurnId,
        ids: Vec<lain_wire::ExperienceId>,
        request: u32,
        redactor: crate::redaction::Redactor,
    ) -> Result<serde_json::Value> {
        self.lock()?
            .store
            .experience_actor_disclosure(id, &ids, request, &redactor)
    }
    pub(crate) fn create_reading_job(
        &self,
        id: ReadingId,
        redactor: crate::redaction::Redactor,
    ) -> Result<(crate::actor::domain::ActorTurn, bool)> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner.store.prune_reading(now_ms()?)?;
        inner.store.create_reading_job(id, &redactor)
    }
    pub(crate) fn reading_actor_input(
        &self,
        id: lain_wire::ActorTurnId,
        redactor: crate::redaction::Redactor,
    ) -> Result<serde_json::Value> {
        let mut inner = self.lock()?;
        inner.store.prune_reading(now_ms()?)?;
        let turn = inner.store.actor_turn(id)?;
        inner.store.reading_actor_input(&turn, &redactor)
    }
    pub(crate) fn experiences(
        &self,
        q: ExperienceQuery,
        redactor: crate::redaction::Redactor,
    ) -> Result<Vec<ReadingExperience>> {
        self.lock()?.store.experiences(q, None, &redactor)
    }
    pub(crate) fn experience(
        &self,
        id: lain_wire::ExperienceId,
        redactor: crate::redaction::Redactor,
    ) -> Result<ReadingExperience> {
        self.lock()?.store.experience(id, &redactor)
    }
    pub fn withdraw_experience(
        &self,
        id: lain_wire::ExperienceId,
        revision: u64,
    ) -> Result<ReadingExperience> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        let result = inner.store.withdraw_experience(id, revision)?;
        let _ = self.reading_events.send(());
        let _ = self.channel_events.send(());
        Ok(result)
    }
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
        if let Some(p) = change.scope.as_ref().and_then(|s| s.processing.as_ref()) {
            inner.store.channel_binding(p.channel_id)?;
        }
        let g = inner.store.set_reading_grant(change, now)?;
        let _ = self.reading_events.send(());
        let _ = self.channel_events.send(());
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
        let now = now_ms()?;
        inner.store.prune_reading(now)?;
        inner.store.claim_reading(id, now)
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
        let _ = self.channel_events.send(());
        inner.store.reading_deadline()
    }
}
