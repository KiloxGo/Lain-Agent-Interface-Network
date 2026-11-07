use super::*;
use crate::backend::ControlError;
use lain_wire::client_sync::*;
impl Kernel {
    pub(crate) fn subscribe_sync(&self) -> tokio::sync::watch::Receiver<()> {
        self.sync_events.subscribe()
    }
    pub(crate) fn sync_status(&self) -> Result<SyncStatus> {
        let inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner
            .store
            .sync_status(self.boot_id, crate::domain::now_ms()?)
    }
    pub(crate) fn bootstrap(
        &self,
        request: BootstrapRequest,
        resources: Resources,
    ) -> Result<BootstrapSnapshot> {
        let inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner.store.bootstrap(&request, resources, self.boot_id)
    }
}
