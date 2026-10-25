use super::{Kernel, KernelState};
use crate::{actor::domain::ActorTurn, persistence::private_chat::ChatRefresh};
use anyhow::{Result, ensure};
use lain_wire::{ActionId, ActorTurnId, chat::FinishChat};
use serde_json::Value;

impl Kernel {
    pub(crate) fn private_chat_memory_current(&self, id: ActorTurnId) -> Result<bool> {
        self.lock()?.store.private_chat_memory_current(id)
    }
    pub(crate) fn private_chat_refresh(&self, id: ActorTurnId) -> Result<ChatRefresh> {
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        inner
            .store
            .private_chat_refresh(id, crate::domain::now_ms()?)
    }
    pub(crate) fn private_chat_history(&self, id: ActorTurnId) -> Result<Vec<ActorTurn>> {
        self.lock()?.store.private_chat_history(id)
    }
    pub(crate) fn private_chat_has_pending(&self, id: ActorTurnId) -> Result<bool> {
        self.lock()?.store.private_chat_has_pending(id)
    }
    pub(crate) fn private_chat_wait(
        &self,
        id: ActorTurnId,
        action: ActionId,
        seconds: u8,
    ) -> Result<u64> {
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        inner.store.private_chat_wait(id, action, seconds)
    }
    pub(crate) fn private_chat_wait_done(
        &self,
        id: ActorTurnId,
        action: ActionId,
    ) -> Result<Value> {
        self.lock()?.store.private_chat_wait_done(id, action)
    }
    pub(crate) fn private_chat_finish(
        &self,
        id: ActorTurnId,
        action: ActionId,
        args: FinishChat,
    ) -> Result<Value> {
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        inner.store.private_chat_finish(id, action, args)
    }
}
