use super::{Kernel, KernelState};
use crate::{archives, backend::ControlError, domain::now_ms, redaction::Redactor};
use anyhow::{Result, ensure};
use lain_wire::{ArchiveRecordId, ArchiveSourceId, archives::*};
impl Kernel {
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
        inner.store.archive_set_grant(id, q, now)
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
