use super::{Kernel, KernelState};
use crate::{backend::ControlError, domain::now_ms, redaction::Redactor};
use anyhow::{Result, ensure};
use lain_wire::{ActorTurnId, ChannelId, DeliveryId, ExperienceId, ShareId, sharing::*};

pub(crate) fn validate_scope(scope: Option<&ShareScope>, now: u64) -> Result<()> {
    let Some(s) = scope else {
        return Ok(());
    };
    ensure!(
        !s.urls.is_empty()
            && s.urls.len() <= 8
            && s.expires_at_ms > now
            && s.expires_at_ms <= i64::MAX as u64
            && (-720..=840).contains(&s.utc_offset_minutes)
            && s.quiet_windows.len() <= 8
            && (1..=100).contains(&s.max_per_day)
            && (1..=100).contains(&s.max_send_attempts)
            && (1..=100).contains(&s.max_model_requests)
            && (1..=31 * 86400000).contains(&s.cooldown_ms)
            && (1..=31 * 86400000).contains(&s.candidate_ttl_ms)
            && (1..=31 * 86400000).contains(&s.source_max_age_ms)
            && (1..=86400000).contains(&s.user_quiet_ms),
        ControlError::InvalidInput
    );
    for (i, url) in s.urls.iter().enumerate() {
        crate::reading::validate_url(url)?;
        ensure!(!s.urls[..i].contains(url), ControlError::InvalidInput);
    }
    for w in &s.quiet_windows {
        ensure!(
            w.from_minute < 1440 && w.to_minute < 1440 && w.from_minute != w.to_minute,
            ControlError::InvalidInput
        );
    }
    Ok(())
}

impl Kernel {
    pub fn sharing_snapshot(&self, id: ChannelId) -> Result<SharingSnapshot> {
        let mut inner = self.lock()?;
        inner.store.share_refresh(id, now_ms()?)?;
        inner.store.sharing_snapshot(id)
    }
    pub fn set_share_grant(&self, id: ChannelId, q: SetShareGrant) -> Result<ShareGrant> {
        let now = now_ms()?;
        validate_scope(q.scope.as_ref(), now)?;
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        let result = inner.store.set_share_grant(id, q, now)?;
        let _ = self.channel_events.send(());
        Ok(result)
    }
    pub fn set_attention(&self, id: ChannelId, q: SetAttention) -> Result<Attention> {
        let now = now_ms()?;
        ensure!(
            q.until_ms
                .is_none_or(|t| q.do_not_disturb && t > now && t <= i64::MAX as u64),
            ControlError::InvalidInput
        );
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        let result = inner.store.set_attention(id, q, now)?;
        let _ = self.channel_events.send(());
        Ok(result)
    }
    pub fn share_consider(&self, id: ChannelId, e: ExperienceId) -> Result<ShareCandidate> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        let c = inner.store.share_consider(id, e, now_ms()?)?;
        let _ = self.channel_events.send(());
        Ok(c)
    }
    pub(crate) fn create_share_job(
        &self,
        id: ShareId,
        redactor: Redactor,
    ) -> Result<(crate::actor::domain::ActorTurn, bool)> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner.store.create_share_job(id, &redactor)
    }
    pub(crate) fn share_actor_input(
        &self,
        id: ActorTurnId,
        redactor: Redactor,
    ) -> Result<serde_json::Value> {
        self.lock()?.store.share_actor_input(id, &redactor)
    }
    pub(crate) fn share_next_job(&self, id: ChannelId) -> Result<Option<ShareId>> {
        self.lock()?.store.share_next_job(id, now_ms()?)
    }
    pub(crate) fn share_fail_start(&self, id: ShareId) -> Result<()> {
        self.lock()?.store.share_fail_start(id)
    }
    pub(crate) fn share_claim(&self, id: ChannelId) -> Result<Option<ShareDelivery>> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner.store.share_claim(id, now_ms()?)
    }
    pub(crate) fn share_may_send(&self, d: &ShareDelivery) -> Result<bool> {
        let inner = self.lock()?;
        Ok(inner.state == KernelState::Ready && inner.store.share_may_send(d, now_ms()?)?)
    }
    pub(crate) fn share_settle(
        &self,
        id: DeliveryId,
        outcome: crate::channels::domain::SendOutcome,
    ) -> Result<()> {
        self.lock()?.store.share_settle(id, outcome)
    }
    pub(crate) fn share_deadline(&self, id: ChannelId, now: u64) -> Result<Option<u64>> {
        self.lock()?.store.share_deadline(id, now)
    }
}
