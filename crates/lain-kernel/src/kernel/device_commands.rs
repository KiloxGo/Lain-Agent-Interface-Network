use super::*;
use lain_wire::device_execution::*;
use lain_wire::{DeviceCommandId, DeviceId, RequestId, device_commands::*};
impl Kernel {
    pub(crate) fn device_command_summaries(
        &self,
        id: DeviceId,
    ) -> Result<Vec<DeviceCommandSummary>> {
        let mut k = self.lock()?;
        ensure!(
            k.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        k.store.device_command_summaries(id)
    }
    pub(crate) fn device_approval_decide(
        &self,
        id: DeviceId,
        cid: DeviceCommandId,
        aid: lain_wire::WorkerApprovalId,
        input: DecideDeviceApproval,
    ) -> Result<DeviceApprovalDecision> {
        let mut k = self.lock()?;
        ensure!(
            k.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        let result = k.store.device_approval_decide(id, cid, aid, input)?;
        let _ = self.device_events.send(());
        Ok(result)
    }
    pub(crate) fn device_approval_decisions(
        &self,
        id: DeviceId,
        cid: DeviceCommandId,
        proof: Option<&str>,
    ) -> Result<Vec<DeviceApprovalDecision>> {
        let mut k = self.lock()?;
        ensure!(
            k.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        k.store.device_approval_decisions(id, cid, proof)
    }
    pub(crate) fn device_commands_subscribe(&self) -> broadcast::Receiver<()> {
        self.device_events.subscribe()
    }
    pub(crate) fn device_command_queue(
        &self,
        id: DeviceId,
        input: QueueDeviceWork,
    ) -> Result<DeviceCommand> {
        let mut k = self.lock()?;
        ensure!(
            k.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        let result = k.store.device_command_queue(id, input)?;
        let _ = self.device_events.send(());
        Ok(result)
    }
    pub(crate) fn device_command_get(
        &self,
        id: DeviceId,
        cid: DeviceCommandId,
        proof: Option<&str>,
    ) -> Result<DeviceCommand> {
        let mut k = self.lock()?;
        ensure!(
            k.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        k.store.device_command_get(id, cid, proof)
    }
    pub(crate) fn device_command_cancel(
        &self,
        id: DeviceId,
        cid: DeviceCommandId,
        input: CancelDeviceCommand,
    ) -> Result<DeviceCommand> {
        let mut k = self.lock()?;
        ensure!(
            k.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        let result = k.store.device_command_cancel(id, cid, input)?;
        let _ = self.device_events.send(());
        Ok(result)
    }
    pub(crate) fn device_command_changes(
        &self,
        id: DeviceId,
        proof: Option<&str>,
        input: CommandChangesQuery,
    ) -> Result<CommandChanges> {
        let mut k = self.lock()?;
        ensure!(
            k.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        k.store.device_command_changes(id, proof, input)
    }
    pub(crate) fn device_command_event(
        &self,
        id: DeviceId,
        cid: DeviceCommandId,
        proof: &str,
        input: ReportCommandEvent,
    ) -> Result<CommandEventReceipt> {
        let mut k = self.lock()?;
        ensure!(
            k.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        let result = k.store.device_command_event(id, cid, proof, input)?;
        let _ = self.device_events.send(());
        Ok(result)
    }
    pub(crate) fn device_command_receipt(
        &self,
        id: DeviceId,
        cid: DeviceCommandId,
        rid: RequestId,
        proof: Option<&str>,
    ) -> Result<CommandEventReceipt> {
        let mut k = self.lock()?;
        ensure!(
            k.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        k.store.device_command_receipt(id, cid, rid, proof)
    }
}
