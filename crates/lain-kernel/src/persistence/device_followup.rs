use super::{
    Store,
    channels::decode,
    device_commands::{command, maintain, publish},
    device_work::{linked_queue, save_dispatch_receipt},
};
use crate::{
    actor::{domain::*, tools::Tool},
    backend::ControlError,
    delegation::Origin,
    domain::now_ms,
};
use anyhow::{Result, ensure};
use lain_wire::{
    DeviceCommandId, DeviceId, RequestId, device_commands::*, device_execution::ExecutionPhase,
    device_work::*,
};
use rusqlite::OptionalExtension;

impl Store {
    pub fn device_work_followup_receipt(
        &mut self,
        conversation: ConversationId,
        device: DeviceId,
        parent: DeviceCommandId,
    ) -> Result<Option<DeviceCommand>> {
        self.device_work_inspect(conversation, device, parent)?;
        let child: Option<String> = self
            .connection
            .query_row(
                "SELECT command_id FROM device_work_continuations WHERE parent_command_id=?1",
                [parent.to_string()],
                |r| r.get(0),
            )
            .optional()?;
        child
            .map(|id| {
                self.device_work_inspect(conversation, device, id.parse()?)
                    .map(|r| r.command)
            })
            .transpose()
    }
    pub fn device_work_followup(
        &mut self,
        conversation: ConversationId,
        device: DeviceId,
        parent: DeviceCommandId,
        input: FollowupDeviceWork,
        actor_origin: Option<Origin>,
    ) -> Result<DeviceCommand> {
        let origin = DeviceWorkOrigin {
            conversation_id: conversation,
            actor_turn_id: actor_origin.map(|o| o.actor_turn_id),
            action_id: actor_origin.map(|o| o.action_id),
        };
        let mut actor = if let Some(o) = actor_origin {
            let turn = self.actor_turn(o.actor_turn_id)?;
            let a = turn
                .actions
                .iter()
                .find(|a| a.id == o.action_id)
                .ok_or(ControlError::NotFound)?;
            let Tool::DeviceSend(args) = &a.tool else {
                anyhow::bail!(ControlError::Conflict)
            };
            ensure!(
                turn.conversation_id == conversation
                    && turn.state == TurnState::Running
                    && a.state == ActionState::Executing
                    && args.device_id == device
                    && args.parent_command_id == parent
                    && args.request(RequestId(o.action_id.0)) == input,
                ControlError::Conflict
            );
            Some(turn)
        } else {
            None
        };
        let tx = self.connection.transaction()?;
        let linked:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM device_work_links WHERE command_id=?1 AND device_id=?2 AND conversation_id=?3)",
            (parent.to_string(),device.to_string(),conversation.to_string()),|r|r.get(0))?;
        ensure!(linked, ControlError::NotFound);
        maintain(&tx, device, now_ms()?)?;
        let mut p = command(&tx, device, parent)?;
        let prior: Option<String> = tx
            .query_row(
                "SELECT snapshot FROM device_commands WHERE device_id=?1 AND request_id=?2",
                (device.to_string(), input.request_id.to_string()),
                |r| r.get(0),
            )
            .optional()?;
        let fresh = prior.is_none();
        let lineage = if let Some(prior) = prior {
            let prior: DeviceCommand = decode(prior)?;
            let lineage = prior.continuation.ok_or(ControlError::Conflict)?;
            ensure!(
                lineage.parent_command_id == parent
                    && lineage.expected_parent_revision == input.expected_parent_revision,
                ControlError::Conflict
            );
            lineage
        } else {
            ensure!(
                p.revision == input.expected_parent_revision
                    && p.continued_by.is_none()
                    && !p.cancel_requested
                    && p.interruption.is_none()
                    && matches!(p.state, CommandState::Succeeded | CommandState::Failed),
                ControlError::Conflict
            );
            let e = p.execution.as_ref().ok_or(ControlError::Conflict)?;
            ensure!(
                matches!(e.phase, ExecutionPhase::Completed | ExecutionPhase::Failed)
                    && e.pending_approvals.is_empty(),
                ControlError::Conflict
            );
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM device_work_continuations WHERE parent_command_id=?1)",
                [parent.to_string()],
                |r| r.get(0),
            )?;
            ensure!(!exists, ControlError::Conflict);
            DeviceContinuation {
                root_command_id: p.continuation.as_ref().map_or(p.id, |f| f.root_command_id),
                parent_command_id: parent,
                expected_parent_revision: input.expected_parent_revision,
                session_id: e.session_id.ok_or(ControlError::Conflict)?,
                parent_turn_id: e.turn_id.ok_or(ControlError::Conflict)?,
            }
        };
        let work = QueueDeviceWork {
            request_id: input.request_id,
            expected_device_revision: input.expected_device_revision,
            workspace_id: p.input.workspace_id,
            profile_id: p.input.profile_id.clone(),
            sandbox: p.input.sandbox,
            task: input.text,
            expires_at_ms: input.expires_at_ms,
        };
        let c = linked_queue(&tx, &origin, device, work, Some(lineage.clone()))?;
        tx.execute("INSERT INTO device_work_continuations VALUES(?1,?2,?3,?4) ON CONFLICT(command_id) DO NOTHING",
            (c.id.to_string(),lineage.root_command_id.to_string(),parent.to_string(),serde_json::to_string(&lineage)?))?;
        if fresh {
            p.continued_by = Some(c.id);
            p.revision += 1;
            publish(&tx, &p)?;
        }
        save_dispatch_receipt(&tx, actor.as_mut(), &origin, &c)?;
        tx.commit()?;
        Ok(c)
    }
}
