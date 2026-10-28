use super::*;
use lain_wire::{DeviceId, devices::*};

impl Kernel {
    pub(crate) fn device_register(&self, input: RegisterDevice) -> Result<DeviceSnapshot> {
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        inner.store.device_register(input)
    }
    pub(crate) fn devices(&self) -> Result<Vec<DeviceSnapshot>> {
        self.lock()?.store.devices()
    }
    pub(crate) fn device_snapshot(&self, id: DeviceId) -> Result<DeviceSnapshot> {
        self.lock()?.store.device_snapshot(id)
    }
    pub(crate) fn device_revoke(
        &self,
        id: DeviceId,
        input: RevokeDevice,
    ) -> Result<DeviceSnapshot> {
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        inner.store.device_revoke(id, input)
    }
    pub(crate) fn device_claim(
        &self,
        id: DeviceId,
        fingerprint: &str,
        input: ClaimDevice,
    ) -> Result<DeviceSnapshot> {
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        inner.store.device_claim(id, fingerprint, input)
    }
    pub(crate) fn device_own_snapshot(
        &self,
        id: DeviceId,
        fingerprint: &str,
    ) -> Result<DeviceSnapshot> {
        let inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        inner.store.device_own_snapshot(id, fingerprint)
    }
    pub(crate) fn device_report(
        &self,
        id: DeviceId,
        fingerprint: &str,
        input: ReportDevice,
    ) -> Result<DeviceReport> {
        let mut inner = self.lock()?;
        ensure!(
            inner.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        inner.store.device_report(id, fingerprint, input)
    }
}
