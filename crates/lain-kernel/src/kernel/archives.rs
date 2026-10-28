use super::{Kernel, KernelState};
use crate::{archives, backend::ControlError, domain::now_ms, redaction::Redactor};
use anyhow::{Result, ensure};
use lain_wire::{ArchiveRecordId, ArchiveSourceId, RequestId, archives::*};
impl Kernel {
    pub(crate) fn archive_memory_grant(
        &self,
        id: lain_wire::ChannelId,
    ) -> Result<ArchiveMemoryGrant> {
        self.lock()?.store.archive_memory_grant(id)
    }
    pub(crate) fn archive_memory_grants(
        &self,
        id: lain_wire::ChannelId,
    ) -> Result<Vec<ArchiveMemoryGrant>> {
        self.lock()?.store.archive_memory_grants(id)
    }
    pub(crate) fn archive_memory_set(
        &self,
        id: lain_wire::ChannelId,
        q: SetArchiveMemory,
    ) -> Result<ArchiveMemoryGrant> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        let g = inner.store.archive_memory_set(id, q)?;
        let _ = self.archive_events.send(());
        let _ = self.channel_events.send(());
        Ok(g)
    }
    pub(crate) fn archive_memory_scope(
        &self,
        id: lain_wire::ActorTurnId,
    ) -> Result<serde_json::Value> {
        self.lock()?.store.archive_memory_scope(id)
    }
    pub(crate) fn knowledge_sync_grants(
        &self,
        id: ArchiveSourceId,
    ) -> Result<Vec<KnowledgeSyncGrant>> {
        self.lock()?.store.knowledge_sync_grants(id)
    }
    pub(crate) fn knowledge_sync_grant(&self, id: ArchiveSourceId) -> Result<KnowledgeSyncGrant> {
        self.lock()?.store.knowledge_sync_grant(id)
    }
    pub(crate) fn knowledge_sync_set(
        &self,
        id: ArchiveSourceId,
        q: SetKnowledgeSync,
        reader: Option<String>,
    ) -> Result<KnowledgeSyncGrant> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        let g = inner.store.knowledge_sync_set(id, q, reader, now_ms()?)?;
        let _ = self.archive_events.send(());
        let _ = self.channel_events.send(());
        Ok(g)
    }
    pub(crate) fn knowledge_sync_plan(
        &self,
        reader: Option<String>,
    ) -> Result<(Option<ArchiveReadJob>, Option<u64>)> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner.store.knowledge_sync_plan(reader, now_ms()?)
    }
    pub(crate) fn knowledge_state(
        &self,
        id: ArchiveSourceId,
    ) -> Result<Option<KnowledgeArchiveState>> {
        self.lock()?.store.knowledge_state(id)
    }
    pub(crate) fn knowledge_admit(
        &self,
        id: ArchiveSourceId,
        q: ReadArchive,
        revision: String,
    ) -> Result<(ArchiveReadJob, bool)> {
        ensure!((1..=32).contains(&q.limit), ControlError::InvalidInput);
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        crate::knowledge::source_key(&inner.store.archive_source(id)?.source.key)?;
        inner.store.knowledge_admit(id, q, revision)
    }
    pub(crate) fn knowledge_finish(
        &self,
        id: RequestId,
        page: crate::knowledge::Page,
        r: Redactor,
    ) -> Result<ArchiveReadJob> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        for change in &page.items {
            archives::safe(&change.message, &r)?;
        }
        let j = inner.store.knowledge_finish(id, page)?;
        let _ = self.archive_events.send(());
        let _ = self.channel_events.send(());
        Ok(j)
    }
    pub(crate) fn archive_recall_grant(
        &self,
        id: lain_wire::ChannelId,
    ) -> Result<ArchiveRecallGrant> {
        self.lock()?.store.archive_recall_grant(id)
    }
    pub(crate) fn archive_recall_set(
        &self,
        id: lain_wire::ChannelId,
        q: SetArchiveRecall,
    ) -> Result<ArchiveRecallGrant> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        let g = inner.store.archive_recall_set(id, q)?;
        let _ = self.channel_events.send(());
        Ok(g)
    }
    pub(crate) fn archive_recall_audit(
        &self,
        id: lain_wire::ChannelId,
        after: u64,
    ) -> Result<serde_json::Value> {
        self.lock()?.store.archive_recall_audit(id, after)
    }
    pub(crate) fn archive_actor_scope(
        &self,
        id: lain_wire::ActorTurnId,
        r: Redactor,
    ) -> Result<serde_json::Value> {
        let data = self.lock()?.store.archive_actor_scope(id)?;
        archives::safe(&data, &r)?;
        Ok(data)
    }
    pub(crate) fn archive_actor_lookup(
        &self,
        id: lain_wire::ActorTurnId,
        action: lain_wire::ActionId,
        r: Redactor,
    ) -> Result<serde_json::Value> {
        self.lock()?.store.archive_actor_lookup(id, action, &r)
    }
    pub(crate) fn archive_actor_disclosure(
        &self,
        id: lain_wire::ActorTurnId,
        request: u32,
        r: Redactor,
    ) -> Result<serde_json::Value> {
        self.lock()?.store.archive_actor_disclosure(id, request, &r)
    }
    pub(crate) fn archive_read_admit(
        &self,
        id: ArchiveSourceId,
        q: ReadWorkerArchive,
    ) -> Result<(ArchiveReadJob, bool)> {
        ensure!((1..=8).contains(&q.limit), ControlError::InvalidInput);
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        let source = inner.store.archive_source(id)?;
        ensure!(
            matches!(&source.source.key,ArchiveSourceKey::CodexSession{installation,..} if installation=="codex/local"),
            ControlError::InvalidInput
        );
        inner.store.archive_read_admit(id, q)
    }
    pub(crate) fn archive_read_claim(
        &self,
        id: RequestId,
    ) -> Result<(ArchiveReadJob, ArchiveSource, Option<String>)> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner.store.archive_read_claim(id)
    }
    pub(crate) fn archive_read_job(&self, id: RequestId) -> Result<ArchiveReadJob> {
        self.lock()?.store.archive_read_job(id)
    }
    pub(crate) fn archive_read_fail(
        &self,
        id: RequestId,
        state: ArchiveReadState,
        reason: ArchiveReadFailure,
        code: Option<i64>,
    ) -> Result<ArchiveReadJob> {
        let result = self
            .lock()?
            .store
            .archive_read_fail(id, state, reason, code)?;
        let _ = self.archive_events.send(());
        Ok(result)
    }
    pub(crate) fn archive_read_current(&self, id: RequestId) -> Result<bool> {
        let inner = self.lock()?;
        let j = inner.store.archive_read_job(id)?;
        let g = inner.store.archive_source(j.source_id)?.grant;
        Ok(inner.state == KernelState::Ready
            && j.state == ArchiveReadState::Reading
            && inner.store.knowledge_read_allowed(&j, now_ms()?)?
            && g.revision == j.request.expected_revision
            && g.scope
                .is_some_and(|s| s.expires_at_ms > now_ms().unwrap_or(u64::MAX)))
    }
    pub(crate) fn archive_read_finish(
        &self,
        id: RequestId,
        batch: crate::backend::archive_read::Batch,
        r: Redactor,
    ) -> Result<ArchiveReadJob> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        let job = inner.store.archive_read_job(id)?;
        archives::safe(&batch.next, &r)?;
        ensure!(
            batch.records.len() <= usize::from(job.request.limit)
                && batch.omitted <= job.request.limit,
            ControlError::InvalidInput
        );
        if !batch.records.is_empty() {
            let source = inner.store.archive_source(job.source_id)?;
            let q = ImportArchive {
                expected_revision: job.request.expected_revision,
                records: batch.records.clone(),
            };
            archives::validate_import(&source.source.key, &q)?;
            archives::safe(&q, &r)?;
        }
        let result = inner.store.archive_read_finish(id, batch)?;
        let _ = self.archive_events.send(());
        Ok(result)
    }
    pub(crate) fn archive_grants(&self, id: ArchiveSourceId) -> Result<Vec<ArchiveGrant>> {
        self.lock()?.store.archive_grants(id)
    }
    pub(crate) fn archive_edits(&self, id: ArchiveRecordId) -> Result<Vec<ArchiveEdit>> {
        self.lock()?.store.archive_edits(id)
    }
    pub(crate) fn archive_register(
        &self,
        key: ArchiveSourceKey,
        r: Redactor,
    ) -> Result<ArchiveSourceView> {
        archives::validate_key(&key)?;
        archives::safe(&key, &r)?;
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner.store.archive_register(key, now_ms()?)
    }
    pub(crate) fn archive_sources(&self) -> Result<Vec<ArchiveSourceView>> {
        self.lock()?.store.archive_sources()
    }
    pub(crate) fn archive_source(&self, id: ArchiveSourceId) -> Result<ArchiveSourceView> {
        self.lock()?.store.archive_source(id)
    }
    pub(crate) fn archive_set_grant(
        &self,
        id: ArchiveSourceId,
        q: SetArchiveGrant,
    ) -> Result<ArchiveGrant> {
        let now = now_ms()?;
        archives::validate_scope(q.scope.as_ref(), now)?;
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        let result = inner.store.archive_set_grant(id, q, now)?;
        let _ = self.archive_events.send(());
        Ok(result)
    }
    pub(crate) fn archive_import(
        &self,
        id: ArchiveSourceId,
        q: ImportArchive,
        r: Redactor,
    ) -> Result<ArchiveImportReceipt> {
        archives::safe(&q, &r)?;
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        let now = now_ms()?;
        inner.store.archive_prune(now)?;
        let source = inner.store.archive_source(id)?;
        archives::validate_import(&source.source.key, &q)?;
        let result = inner.store.archive_import(id, q, now)?;
        let _ = self.archive_events.send(());
        Ok(result)
    }
    pub(crate) fn archive_query(
        &self,
        id: ArchiveSourceId,
        q: ArchiveQuery,
    ) -> Result<ArchivePage> {
        let mut inner = self.lock()?;
        let now = now_ms()?;
        inner.store.archive_prune(now)?;
        inner.store.archive_query(id, q, now)
    }
    pub(crate) fn archive_record(&self, id: ArchiveRecordId) -> Result<ArchiveRecord> {
        let mut inner = self.lock()?;
        let now = now_ms()?;
        inner.store.archive_prune(now)?;
        inner.store.archive_record(id, now)
    }
    pub(crate) fn archive_forget(
        &self,
        id: ArchiveRecordId,
        q: ArchiveRecordRevision,
    ) -> Result<ArchiveRecord> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner
            .store
            .archive_forget(id, q.expected_revision, now_ms()?)
    }
    pub(crate) fn archive_correct(
        &self,
        id: ArchiveRecordId,
        q: CorrectArchiveRecord,
        r: Redactor,
    ) -> Result<ArchiveRecord> {
        archives::safe(&q, &r)?;
        archives::validate_text(&q.text)?;
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner.store.archive_correct(id, q, now_ms()?)
    }
    pub(crate) fn archive_deadline(&self) -> Result<Option<u64>> {
        self.lock()?.store.archive_deadline(now_ms()?)
    }
    pub(crate) fn subscribe_archives(&self) -> tokio::sync::broadcast::Receiver<()> {
        self.archive_events.subscribe()
    }
}
