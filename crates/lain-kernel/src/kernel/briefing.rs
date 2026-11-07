use super::{Kernel, KernelState};
use crate::{backend::ControlError, domain::now_ms, redaction::Redactor};
use anyhow::{Result, ensure};
use lain_wire::{ConversationId, RequestId, briefing::*};

impl Kernel {
    pub(crate) fn next_briefing_relation(
        &self,
        c: ConversationId,
        id: RequestId,
    ) -> Result<Option<BriefingStage>> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner.store.next_briefing_relation(c, id)
    }
    pub(crate) fn briefing_actor_scope(
        &self,
        id: lain_wire::ActorTurnId,
    ) -> Result<serde_json::Value> {
        self.lock()?.store.briefing_actor_scope(id)
    }
    pub(crate) fn prepare_actor_briefing(
        &self,
        id: lain_wire::ActorTurnId,
        action: lain_wire::ActionId,
    ) -> Result<RequestId> {
        self.lock()?.store.prepare_actor_briefing(id, action)
    }
    pub(crate) fn briefing_actor_lookup(
        &self,
        id: lain_wire::ActorTurnId,
        action: lain_wire::ActionId,
        r: Redactor,
    ) -> Result<serde_json::Value> {
        self.lock()?.store.briefing_actor_lookup(id, action, &r)
    }
    pub(crate) fn briefing_report(
        &self,
        c: ConversationId,
        id: RequestId,
        after: u16,
        limit: u16,
        r: Redactor,
    ) -> Result<serde_json::Value> {
        self.lock()?.store.briefing_report(c, id, after, limit, &r)
    }
    pub(crate) fn begin_briefing_run(&self, c: ConversationId, id: RequestId) -> Result<bool> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner.store.begin_briefing_run(c, id)
    }
    pub(crate) fn briefing_run_status(
        &self,
        c: ConversationId,
        id: RequestId,
    ) -> Result<BriefingRunStatus> {
        self.lock()?.store.briefing_run_status(c, id)
    }
    pub(crate) fn advance_briefing_run(
        &self,
        c: ConversationId,
        id: RequestId,
    ) -> Result<BriefingRunStatus> {
        self.lock()?.store.advance_briefing_run(c, id)
    }
    pub(crate) fn stop_briefing_run(
        &self,
        c: ConversationId,
        id: RequestId,
        stop: BriefingRunStop,
    ) -> Result<()> {
        self.lock()?.store.stop_briefing_run(c, id, stop)
    }
    pub(crate) fn create_briefing_job(
        &self,
        a: BriefingActivity,
        r: Redactor,
    ) -> Result<(crate::actor::domain::ActorTurn, bool)> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner.store.create_briefing_job(a, &r)
    }
    pub(crate) fn briefing_actor_input(
        &self,
        id: lain_wire::ActorTurnId,
        r: Redactor,
    ) -> Result<serde_json::Value> {
        self.lock()?.store.briefing_actor_input(id, &r)
    }
    pub(crate) fn briefing_batch(
        &self,
        c: ConversationId,
        id: RequestId,
        from_index: u16,
        r: Redactor,
    ) -> Result<BriefingBatch> {
        self.lock()?.store.briefing_batch(
            BriefingActivity {
                stage: lain_wire::briefing::BriefingStage::Extract,
                conversation_id: c,
                briefing_id: id,
                from_index,
            },
            &r,
        )
    }
    pub(crate) fn briefing_grant(&self, c: ConversationId) -> Result<BriefingGrant> {
        self.lock()?.store.briefing_grant(c)
    }
    pub(crate) fn briefing_grant_receipt(
        &self,
        c: ConversationId,
        request: RequestId,
    ) -> Result<BriefingGrantReceipt> {
        self.lock()?.store.briefing_grant_receipt(c, request)
    }
    pub(crate) fn briefing_set(
        &self,
        c: ConversationId,
        q: SetBriefingGrant,
    ) -> Result<BriefingGrant> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner.store.briefing_set(c, q, now_ms()?)
    }
    pub(crate) fn briefing_start(
        &self,
        c: ConversationId,
        q: StartBriefing,
    ) -> Result<BriefingManifest> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner.store.briefing_start(c, q, now_ms()?)
    }
    pub(crate) fn briefing_manifest(
        &self,
        c: ConversationId,
        id: RequestId,
    ) -> Result<BriefingManifest> {
        self.lock()?.store.briefing_manifest(c, id)
    }
    pub(crate) fn briefing_page(
        &self,
        c: ConversationId,
        id: RequestId,
        after: u16,
        r: Redactor,
    ) -> Result<BriefingPage> {
        self.lock()?
            .store
            .briefing_page(c, id, after, now_ms()?, &r)
    }
}
