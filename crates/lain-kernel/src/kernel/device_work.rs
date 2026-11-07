use super::*;
use crate::{
    actor::{domain::*, tools::Tool},
    delegation::Origin,
};
use lain_wire::{DeviceCommandId, DeviceId, RequestId, device_commands::*, device_work::*};

impl Kernel {
    pub(crate) fn device_work_followup_receipt(
        &self,
        conversation: ConversationId,
        device: DeviceId,
        parent: DeviceCommandId,
    ) -> Result<Option<DeviceCommand>> {
        let mut k = self.lock()?;
        ensure!(
            k.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        k.store
            .device_work_followup_receipt(conversation, device, parent)
    }
    pub(crate) fn device_work_followup(
        &self,
        conversation: ConversationId,
        device: DeviceId,
        parent: DeviceCommandId,
        input: FollowupDeviceWork,
    ) -> Result<DeviceCommand> {
        let mut k = self.lock()?;
        ensure!(
            k.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        let c = k
            .store
            .device_work_followup(conversation, device, parent, input, None)?;
        let _ = self.device_events.send(());
        Ok(c)
    }
    pub(crate) fn actor_device_send(
        &self,
        actor: ActorTurnId,
        action: ActionId,
    ) -> Result<DeviceCommand> {
        let mut k = self.lock()?;
        ensure!(
            k.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        let t = k.store.actor_turn(actor)?;
        let a = t
            .actions
            .iter()
            .find(|a| a.id == action)
            .ok_or(crate::backend::ControlError::NotFound)?;
        let Tool::DeviceSend(args) = &a.tool else {
            anyhow::bail!(crate::backend::ControlError::Conflict)
        };
        let c = k.store.device_work_followup(
            t.conversation_id,
            args.device_id,
            args.parent_command_id,
            args.request(RequestId(action.0)),
            Some(Origin {
                actor_turn_id: actor,
                action_id: action,
            }),
        )?;
        let _ = self.device_events.send(());
        Ok(c)
    }
    pub(crate) fn device_work_dispatch(
        &self,
        conversation: ConversationId,
        device: DeviceId,
        input: QueueDeviceWork,
    ) -> Result<DeviceCommand> {
        let mut k = self.lock()?;
        ensure!(
            k.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        let c = k
            .store
            .device_work_dispatch(conversation, device, input, None)?;
        let _ = self.device_events.send(());
        Ok(c)
    }
    pub(crate) fn actor_device_queue(
        &self,
        actor: ActorTurnId,
        action: ActionId,
    ) -> Result<DeviceCommand> {
        let mut k = self.lock()?;
        ensure!(
            k.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        let t = k.store.actor_turn(actor)?;
        let a = t
            .actions
            .iter()
            .find(|a| a.id == action)
            .ok_or(crate::backend::ControlError::NotFound)?;
        let Tool::DeviceQueue(args) = &a.tool else {
            anyhow::bail!(crate::backend::ControlError::Conflict)
        };
        let c = k.store.device_work_dispatch(
            t.conversation_id,
            args.device_id,
            args.queue(RequestId(action.0)),
            Some(Origin {
                actor_turn_id: actor,
                action_id: action,
            }),
        )?;
        let _ = self.device_events.send(());
        Ok(c)
    }
    pub(crate) fn device_work_page(&self, conversation: ConversationId) -> Result<DeviceWorkPage> {
        let mut k = self.lock()?;
        ensure!(
            k.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        k.store.device_work_page(conversation)
    }
    pub(crate) fn device_work_request_receipt(
        &self,
        conversation: ConversationId,
        device: DeviceId,
        request: RequestId,
    ) -> Result<Option<DeviceWorkRecord>> {
        let mut k = self.lock()?;
        ensure!(
            k.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        k.store
            .device_work_request_receipt(conversation, device, request)
    }
    pub(crate) fn device_work_inspect(
        &self,
        conversation: ConversationId,
        device: DeviceId,
        command: DeviceCommandId,
    ) -> Result<DeviceWorkRecord> {
        let mut k = self.lock()?;
        ensure!(
            k.state == KernelState::Ready,
            crate::backend::ControlError::Unavailable
        );
        k.store.device_work_inspect(conversation, device, command)
    }
}
