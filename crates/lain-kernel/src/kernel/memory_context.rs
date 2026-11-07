use super::*;
use crate::{actor::domain::*, backend::ControlError, memory::*};
use lain_wire::memory_context::*;

impl Kernel {
    pub(crate) fn memory_selection_available(&self, id: ActorTurnId) -> Result<bool> {
        self.lock()?.store.selection_available(id)
    }
    pub(crate) fn memory_selection_inspect(
        &self,
        id: ActorTurnId,
        action: ActionId,
    ) -> Result<crate::memory::MemorySelectionTarget> {
        self.lock()?.store.selection_inspect(id, action)
    }
    pub(crate) fn memory_selection_change(
        &self,
        id: ActorTurnId,
        action: ActionId,
    ) -> Result<crate::memory::MemorySelectionEffect> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner.store.selection_change(id, action)
    }
    pub(crate) fn actor_view(&self, id: ActorTurnId) -> Result<lain_wire::ActorSnapshot> {
        self.lock()?.store.actor_view(id)
    }
    pub(crate) fn actor_source_notice(
        &self,
        id: ActorTurnId,
        usage: Option<(u64, lain_wire::ModelTiming)>,
    ) -> Result<ActorTurn> {
        self.lock()?.store.actor_source_notice(id, usage)
    }
    pub(crate) fn actor_memory_proposal_available(&self, id: ActorTurnId) -> Result<bool> {
        self.lock()?.store.actor_memory_proposal_available(id)
    }
    pub(crate) fn actor_memory_scopes(&self, id: ActorTurnId) -> Result<Vec<MemoryScope>> {
        self.lock()?.store.actor_memory_scopes(id)
    }
    pub(crate) fn memory_context(
        &self,
        id: ConversationId,
    ) -> Result<Option<ConversationMemoryContext>> {
        self.lock()?.store.memory_context(id)
    }
    pub(crate) fn set_memory_context(
        &self,
        id: ConversationId,
        input: SetMemoryContext,
    ) -> Result<ConversationMemoryContext> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner.store.set_memory_context(id, input)
    }
    pub(crate) fn check_actor_memory(&self, id: ActorTurnId) -> Result<()> {
        self.lock()?.store.check_actor_memory(id)
    }
    #[cfg(test)]
    pub(crate) fn actor_memory_page(
        &self,
        id: ActorTurnId,
        mut query: MemoryQuery,
    ) -> Result<MemoryPage> {
        query.scope = crate::memory::canonical_scope(query.scope)?;
        self.lock()?.store.actor_memory_page(id, &query)
    }
    #[cfg(test)]
    pub(crate) fn actor_memory_inspect(
        &self,
        id: ActorTurnId,
        memory: MemoryId,
        scope: MemoryScope,
    ) -> Result<MemoryRecord> {
        let scope = crate::memory::canonical_scope(scope)?;
        self.lock()?.store.actor_memory_inspect(id, memory, &scope)
    }
    pub(crate) fn actor_memory_page_with_captures(
        &self,
        id: ActorTurnId,
        mut query: MemoryQuery,
    ) -> Result<(MemoryPage, Vec<MemoryStatementCapture>)> {
        query.scope = crate::memory::canonical_scope(query.scope)?;
        let mut inner = self.lock()?;
        let page = inner.store.actor_memory_page(id, &query)?;
        let captures = inner.store.memory_statement_captures(&page.records)?;
        Ok((page, captures))
    }
    pub(crate) fn actor_memory_inspect_with_captures(
        &self,
        id: ActorTurnId,
        memory: MemoryId,
        scope: MemoryScope,
    ) -> Result<(MemoryRecord, Vec<MemoryStatementCapture>)> {
        let scope = crate::memory::canonical_scope(scope)?;
        let mut inner = self.lock()?;
        let record = inner.store.actor_memory_inspect(id, memory, &scope)?;
        let captures = inner
            .store
            .memory_statement_captures(std::slice::from_ref(&record))?;
        Ok((record, captures))
    }
    pub(crate) fn memory_context_history(
        &self,
        id: ActorTurnId,
        history: Vec<ActorTurn>,
    ) -> Result<Vec<ActorTurn>> {
        self.lock()?.store.memory_context_history(id, history)
    }
    pub(crate) fn actor_memory_requires_hold(&self, id: ActorTurnId) -> Result<bool> {
        self.lock()?.store.actor_memory_requires_hold(id)
    }
}
