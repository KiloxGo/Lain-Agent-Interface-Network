use super::{Kernel, KernelState};
use crate::{backend::ControlError, domain::now_ms};
use anyhow::{Result, ensure};
use lain_wire::{ActorTurnId, InferenceBudgetId, inference::*};

impl Kernel {
    pub fn inference_snapshot(&self) -> Result<InferenceSnapshot> {
        self.lock()?.store.inference_snapshot()
    }
    pub fn inference_budgets(&self) -> Result<Vec<InferenceBudgetStatus>> {
        self.lock()?.store.inference_budgets()
    }
    pub fn inference_requests(
        &self,
        id: Option<InferenceBudgetId>,
    ) -> Result<Vec<InferenceRequest>> {
        self.lock()?.store.inference_requests(id)
    }
    pub fn set_inference_budget(&self, q: SetInferenceBudget) -> Result<InferenceDecision> {
        if let Some(s) = &q.limit {
            ensure!(
                (1..=100000).contains(&s.max_requests)
                    && s.expires_at_ms > now_ms()?
                    && s.expires_at_ms <= i64::MAX as u64,
                ControlError::InvalidInput
            );
        }
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner.store.set_inference_budget(q, now_ms()?)
    }
    pub(crate) fn begin_inference(&self, id: ActorTurnId, n: u32) -> Result<()> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner.store.begin_inference(id, n)
    }
    pub(crate) fn inference_returned(&self, id: ActorTurnId, n: u32, tokens: u64) -> Result<()> {
        self.lock()?.store.inference_returned(id, n, tokens)
    }
}
