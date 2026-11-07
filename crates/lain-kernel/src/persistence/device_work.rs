use super::{
    Store,
    channels::decode,
    device_commands::{command, maintain, queue_in_transaction},
};
use crate::{
    actor::{domain::*, tools::Tool},
    backend::ControlError,
    delegation::Origin,
    domain::now_ms,
};
use anyhow::{Result, ensure};
use lain_wire::{DeviceCommandId, DeviceId, RequestId, device_commands::*, device_work::*};
use rusqlite::OptionalExtension;

impl Store {
    pub fn device_work_request_receipt(
        &mut self,
        conversation: ConversationId,
        device: DeviceId,
        request: RequestId,
    ) -> Result<Option<DeviceWorkRecord>> {
        let id: Option<String> = self.connection.query_row(
            "SELECT c.id FROM device_commands c JOIN device_work_links l ON l.command_id=c.id WHERE l.conversation_id=?1 AND l.device_id=?2 AND c.device_id=?2 AND c.request_id=?3",
            (conversation.to_string(), device.to_string(), request.to_string()), |r| r.get(0)
        ).optional()?;
        id.map(|id| self.device_work_inspect(conversation, device, id.parse()?))
            .transpose()
    }
    pub fn device_work_dispatch(
        &mut self,
        conversation: ConversationId,
        device: DeviceId,
        input: QueueDeviceWork,
        actor_origin: Option<Origin>,
    ) -> Result<DeviceCommand> {
        let origin = DeviceWorkOrigin {
            conversation_id: conversation,
            actor_turn_id: actor_origin.map(|o| o.actor_turn_id),
            action_id: actor_origin.map(|o| o.action_id),
        };
        let mut actor = if let Some(o) = actor_origin {
            let turn = self.actor_turn(o.actor_turn_id)?;
            let action = turn
                .actions
                .iter()
                .find(|a| a.id == o.action_id)
                .ok_or(ControlError::NotFound)?;
            ensure!(
                turn.conversation_id == conversation
                    && turn.state == TurnState::Running
                    && action.state == ActionState::Executing,
                ControlError::Conflict
            );
            let Tool::DeviceQueue(args) = &action.tool else {
                anyhow::bail!(ControlError::Conflict)
            };
            ensure!(
                args.device_id == device && args.queue(RequestId(o.action_id.0)) == input,
                ControlError::Conflict
            );
            Some(turn)
        } else {
            None
        };
        let tx = self.connection.transaction()?;
        let c = linked_queue(&tx, &origin, device, input, None)?;
        save_dispatch_receipt(&tx, actor.as_mut(), &origin, &c)?;
        tx.commit()?;
        Ok(c)
    }

    pub fn device_work_page(&mut self, conversation: ConversationId) -> Result<DeviceWorkPage> {
        let tx = self.connection.transaction()?;
        ensure!(
            tx.query_row(
                "SELECT 1 FROM lain_conversations WHERE id=?1",
                [conversation.to_string()],
                |r| r.get::<_, u8>(0)
            )
            .optional()?
            .is_some(),
            ControlError::NotFound
        );
        let links = {
            let mut q=tx.prepare("SELECT device_id,command_id,snapshot FROM device_work_links WHERE conversation_id=?1 ORDER BY created_at_ms DESC,command_id DESC LIMIT 33")?;
            q.query_map([conversation.to_string()], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
        };
        let truncated = links.len() > 32;
        let mut devices = std::collections::BTreeSet::new();
        let mut work = Vec::new();
        for (device, cid, origin) in links.into_iter().take(32) {
            let device: DeviceId = device.parse()?;
            if devices.insert(device) {
                maintain(&tx, device, now_ms()?)?;
            }
            work.push(DeviceWorkRecord {
                origin: decode(origin)?,
                command: command(&tx, device, cid.parse()?)?,
            });
        }
        tx.commit()?;
        Ok(DeviceWorkPage { work, truncated })
    }

    pub fn device_work_inspect(
        &mut self,
        conversation: ConversationId,
        device: DeviceId,
        cid: DeviceCommandId,
    ) -> Result<DeviceWorkRecord> {
        let origin:String=self.connection.query_row("SELECT snapshot FROM device_work_links WHERE conversation_id=?1 AND device_id=?2 AND command_id=?3",
            (conversation.to_string(),device.to_string(),cid.to_string()),|r|r.get(0)).optional()?.ok_or(ControlError::NotFound)?;
        Ok(DeviceWorkRecord {
            origin: decode(origin)?,
            command: self.device_command_get(device, cid, None)?,
        })
    }
}

pub(super) fn linked_queue(
    tx: &rusqlite::Transaction<'_>,
    origin: &DeviceWorkOrigin,
    device: DeviceId,
    input: QueueDeviceWork,
    continuation: Option<DeviceContinuation>,
) -> Result<DeviceCommand> {
    ensure!(
        tx.query_row(
            "SELECT 1 FROM lain_conversations WHERE id=?1",
            [origin.conversation_id.to_string()],
            |r| r.get::<_, u8>(0)
        )
        .optional()?
        .is_some(),
        ControlError::NotFound
    );
    let previous: Option<(String,Option<String>)> = tx.query_row(
        "SELECT c.id,l.snapshot FROM device_commands c LEFT JOIN device_work_links l ON l.command_id=c.id WHERE c.device_id=?1 AND c.request_id=?2",
        (device.to_string(),input.request_id.to_string()),|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
    if let Some((id, stored)) = &previous {
        let old: DeviceWorkOrigin =
            decode(stored.as_deref().ok_or(ControlError::Conflict)?.to_owned())?;
        ensure!(
            old == *origin && command(tx, device, id.parse()?)?.continuation == continuation,
            ControlError::Conflict
        );
    }
    let mut c = queue_in_transaction(tx, device, input, now_ms()?)?;
    if previous.is_none() {
        c.continuation = continuation;
        tx.execute(
            "UPDATE device_commands SET snapshot=?2 WHERE id=?1",
            (c.id.to_string(), serde_json::to_string(&c)?),
        )?;
        tx.execute(
            "INSERT INTO device_work_links VALUES(?1,?2,?3,?4,?5)",
            (
                c.id.to_string(),
                device.to_string(),
                origin.conversation_id.to_string(),
                i64::try_from(c.created_at_ms)?,
                serde_json::to_string(origin)?,
            ),
        )?;
    }
    Ok(c)
}

pub(super) fn save_dispatch_receipt(
    tx: &rusqlite::Transaction<'_>,
    actor: Option<&mut ActorTurn>,
    origin: &DeviceWorkOrigin,
    c: &DeviceCommand,
) -> Result<()> {
    if let Some(turn) = actor {
        let action = turn
            .actions
            .iter_mut()
            .find(|a| Some(a.id) == origin.action_id)
            .ok_or(ControlError::NotFound)?;
        let mut receipt = serde_json::json!({"device_id":c.device_id,"command_id":c.id,"request_id":c.input.request_id,"stage":"queued","worker_completion":"not_observed"});
        if let Some(lineage) = &c.continuation {
            receipt["continuation"] = serde_json::to_value(lineage)?;
        }
        ensure!(
            action.result.as_ref().is_none_or(|old| *old == receipt),
            ControlError::Conflict
        );
        action.result = Some(receipt);
        Store::save_actor_in_transaction(tx, turn, false)?;
    }
    Ok(())
}
