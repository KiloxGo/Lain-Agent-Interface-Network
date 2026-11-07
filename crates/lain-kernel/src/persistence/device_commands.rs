use super::{
    Store,
    channels::decode,
    devices::{authorized, device, safe},
};
use crate::{backend::ControlError, domain::now_ms};
use anyhow::{Result, ensure};
use lain_wire::device_execution::*;
use lain_wire::{DeviceCommandId, DeviceId, RequestId, device_commands::*, devices::DeviceState};
use rusqlite::{Connection, OptionalExtension};

fn safe_text(text: &str) -> bool {
    text.chars()
        .all(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
}

pub(super) fn command(
    db: &Connection,
    device_id: DeviceId,
    id: DeviceCommandId,
) -> Result<DeviceCommand> {
    let c: DeviceCommand = decode(
        db.query_row(
            "SELECT snapshot FROM device_commands WHERE id=?1 AND device_id=?2",
            (id.to_string(), device_id.to_string()),
            |r| r.get(0),
        )
        .optional()?
        .ok_or(ControlError::NotFound)?,
    )?;
    ensure!(
        c.id == id && c.device_id == device_id && c.revision > 0,
        "invalid device command identity"
    );
    Ok(c)
}
pub(super) fn publish(db: &Connection, c: &DeviceCommand) -> Result<()> {
    db.execute(
        "UPDATE device_commands SET snapshot=?2 WHERE id=?1",
        (c.id.to_string(), serde_json::to_string(c)?),
    )?;
    let last: i64 = db.query_row(
        "SELECT coalesce(max(sequence),0) FROM device_command_changes WHERE device_id=?1",
        [c.device_id.to_string()],
        |r| r.get(0),
    )?;
    let seq = last.checked_add(1).ok_or(ControlError::Unavailable)?;
    let change = CommandChange {
        sequence: seq.try_into()?,
        command_id: c.id,
        revision: c.revision,
        state: c.state,
        cancel_requested: c.cancel_requested,
    };
    db.execute(
        "INSERT INTO device_command_changes VALUES(?1,?2,?3)",
        (
            c.device_id.to_string(),
            seq,
            serde_json::to_string(&change)?,
        ),
    )?;
    Ok(())
}
/// Must share the transaction which changes device authority or command state.
pub(super) fn maintain(db: &Connection, id: DeviceId, now: u64) -> Result<()> {
    let d = device(db, id)?;
    let mut q = db.prepare("SELECT snapshot FROM device_commands WHERE device_id=?1")?;
    let commands = q
        .query_map([id.to_string()], |r| r.get::<_, String>(0))?
        .map(|r| decode(r?))
        .collect::<Result<Vec<DeviceCommand>>>()?;
    for mut c in commands {
        if c.state.terminal() {
            continue;
        }
        let revoked = d.state == DeviceState::Revoked;
        if revoked || now >= d.registration.expires_at_ms || now >= c.input.expires_at_ms {
            c.state = if c.state != CommandState::Queued {
                CommandState::Unknown
            } else if revoked {
                CommandState::Revoked
            } else {
                CommandState::Expired
            };
            c.interruption = Some(if revoked {
                CommandInterruption::DeviceRevoked
            } else {
                CommandInterruption::Deadline
            });
            c.revision += 1;
            publish(db, &c)?;
        }
    }
    Ok(())
}

impl Store {
    pub fn device_command_summaries(&mut self, id: DeviceId) -> Result<Vec<DeviceCommandSummary>> {
        let tx = self.connection.transaction()?;
        maintain(&tx, id, now_ms()?)?;
        let mut commands = {
            let mut q = tx.prepare("SELECT snapshot FROM device_commands WHERE device_id=?1")?;
            q.query_map([id.to_string()], |r| r.get::<_, String>(0))?
                .map(|r| decode(r?))
                .collect::<Result<Vec<DeviceCommand>>>()?
        };
        commands.sort_by_key(|c| std::cmp::Reverse((c.created_at_ms, c.id)));
        let result = commands
            .into_iter()
            .map(|c| DeviceCommandSummary {
                id: c.id,
                revision: c.revision,
                state: c.state,
                cancel_requested: c.cancel_requested,
                profile_id: c.input.profile_id,
                workspace_id: c.input.workspace_id,
                created_at_ms: c.created_at_ms,
                expires_at_ms: c.input.expires_at_ms,
            })
            .collect();
        tx.commit()?;
        Ok(result)
    }
    /// An immutable decision receipt is not a lease to replay a Worker mutation.
    pub fn device_approval_decide(
        &mut self,
        id: DeviceId,
        cid: DeviceCommandId,
        aid: lain_wire::WorkerApprovalId,
        input: DecideDeviceApproval,
    ) -> Result<DeviceApprovalDecision> {
        safe(&input)?;
        ensure!(
            input.fingerprint.len() == 64
                && input.fingerprint.bytes().all(|b| b.is_ascii_hexdigit()),
            ControlError::InvalidInput
        );
        let now = now_ms()?;
        let tx = self.connection.transaction()?;
        command(&tx, id, cid)?;
        let old: Option<String> = tx.query_row(
            "SELECT snapshot FROM device_approval_decisions WHERE command_id=?1 AND (approval_id=?2 OR request_id=?3)",
            (cid.to_string(), aid.to_string(), input.request_id.to_string()), |r| r.get(0)
        ).optional()?;
        if let Some(old) = old {
            let old: DeviceApprovalDecision = decode(old)?;
            ensure!(
                old.approval_id == aid && old.decision == input,
                ControlError::Conflict
            );
            return Ok(old);
        }
        maintain(&tx, id, now)?;
        let c = command(&tx, id, cid)?;
        let d = device(&tx, id)?;
        // Commit deadline/revocation projection even when this decision is stale.
        if d.state != DeviceState::Active
            || now >= d.registration.expires_at_ms
            || now >= c.input.expires_at_ms
            || c.state != CommandState::Running
            || c.cancel_requested
        {
            tx.commit()?;
            anyhow::bail!(ControlError::Conflict);
        }
        let approval = c
            .execution
            .as_ref()
            .filter(|e| e.valid() && e.phase == ExecutionPhase::WaitingForApproval)
            .and_then(|e| e.pending_approvals.iter().find(|a| a.id == aid))
            .ok_or(ControlError::Conflict)?;
        ensure!(
            approval.fingerprint == input.fingerprint && approval.choices.contains(&input.choice),
            ControlError::Conflict
        );
        let receipt = DeviceApprovalDecision {
            approval_id: aid,
            decision: input,
            received_at_ms: now,
        };
        tx.execute(
            "INSERT INTO device_approval_decisions VALUES(?1,?2,?3,?4)",
            (
                cid.to_string(),
                aid.to_string(),
                receipt.decision.request_id.to_string(),
                serde_json::to_string(&receipt)?,
            ),
        )?;
        let mut c = c;
        c.revision += 1;
        publish(&tx, &c)?;
        tx.commit()?;
        Ok(receipt)
    }
    pub fn device_approval_decisions(
        &mut self,
        id: DeviceId,
        cid: DeviceCommandId,
        proof: Option<&str>,
    ) -> Result<Vec<DeviceApprovalDecision>> {
        let tx = self.connection.transaction()?;
        if let Some(p) = proof {
            authorized(&tx, id, p, false, now_ms()?)?;
        }
        command(&tx, id, cid)?;
        let rows = {
            let mut q = tx.prepare(
                "SELECT snapshot FROM device_approval_decisions WHERE command_id=?1 ORDER BY rowid",
            )?;
            q.query_map([cid.to_string()], |r| r.get::<_, String>(0))?
                .map(|r| decode(r?))
                .collect::<Result<Vec<_>>>()?
        };
        tx.commit()?;
        Ok(rows)
    }
    pub fn device_command_queue(
        &mut self,
        id: DeviceId,
        input: QueueDeviceWork,
    ) -> Result<DeviceCommand> {
        let tx = self.connection.transaction()?;
        let c = queue_in_transaction(&tx, id, input, now_ms()?)?;
        tx.commit()?;
        Ok(c)
    }
    pub fn device_command_get(
        &mut self,
        id: DeviceId,
        cid: DeviceCommandId,
        proof: Option<&str>,
    ) -> Result<DeviceCommand> {
        let now = now_ms()?;
        let tx = self.connection.transaction()?;
        if let Some(p) = proof {
            authorized(&tx, id, p, false, now)?;
        }
        maintain(&tx, id, now)?;
        let result = command(&tx, id, cid)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn device_command_cancel(
        &mut self,
        id: DeviceId,
        cid: DeviceCommandId,
        input: CancelDeviceCommand,
    ) -> Result<DeviceCommand> {
        let tx = self.connection.transaction()?;
        maintain(&tx, id, now_ms()?)?;
        let mut c = command(&tx, id, cid)?;
        ensure!(
            c.revision == input.expected_revision,
            ControlError::Conflict
        );
        if (!c.state.terminal() || c.state == CommandState::Unknown) && !c.cancel_requested {
            c.cancel_requested = true;
            c.revision += 1;
            if c.state == CommandState::Queued {
                c.state = CommandState::Cancelled;
            }
            publish(&tx, &c)?;
        }
        tx.commit()?;
        Ok(c)
    }
    pub fn device_command_changes(
        &mut self,
        id: DeviceId,
        proof: Option<&str>,
        input: CommandChangesQuery,
    ) -> Result<CommandChanges> {
        ensure!(
            (1..=32).contains(&input.limit) && input.wait_ms <= 25000,
            ControlError::InvalidInput
        );
        let now = now_ms()?;
        let tx = self.connection.transaction()?;
        if let Some(p) = proof {
            authorized(&tx, id, p, false, now)?;
        }
        maintain(&tx, id, now)?;
        let max: i64 = tx.query_row(
            "SELECT coalesce(max(sequence),0) FROM device_command_changes WHERE device_id=?1",
            [id.to_string()],
            |r| r.get(0),
        )?;
        let max = u64::try_from(max)?;
        let through = input.through.unwrap_or(max);
        ensure!(
            input.after <= through && through <= max,
            ControlError::InvalidInput
        );
        let changes = {
            let mut q=tx.prepare("SELECT snapshot FROM device_command_changes WHERE device_id=?1 AND sequence>?2 AND sequence<=?3 ORDER BY sequence LIMIT ?4")?;
            q.query_map(
                (
                    id.to_string(),
                    i64::try_from(input.after)?,
                    i64::try_from(through)?,
                    i64::from(input.limit),
                ),
                |r| r.get::<_, String>(0),
            )?
            .map(|r| decode(r?))
            .collect::<Result<Vec<CommandChange>>>()?
        };
        let next_after = changes.last().map_or(through, |c| c.sequence);
        let result = CommandChanges {
            has_more: next_after < through,
            changes,
            next_after,
            through,
        };
        tx.commit()?;
        Ok(result)
    }
    pub fn device_command_receipt(
        &mut self,
        id: DeviceId,
        cid: DeviceCommandId,
        rid: RequestId,
        proof: Option<&str>,
    ) -> Result<CommandEventReceipt> {
        let tx = self.connection.transaction()?;
        if let Some(p) = proof {
            authorized(&tx, id, p, false, now_ms()?)?;
        }
        command(&tx, id, cid)?;
        let value = decode(
            tx.query_row(
                "SELECT snapshot FROM device_command_events WHERE command_id=?1 AND request_id=?2",
                (cid.to_string(), rid.to_string()),
                |r| r.get(0),
            )
            .optional()?
            .ok_or(ControlError::NotFound)?,
        )?;
        tx.commit()?;
        Ok(value)
    }
    pub fn device_command_event(
        &mut self,
        id: DeviceId,
        cid: DeviceCommandId,
        proof: &str,
        input: ReportCommandEvent,
    ) -> Result<CommandEventReceipt> {
        safe(&input)?;
        ensure!(
            input.detail.len() <= 4096 && safe_text(&input.detail),
            ControlError::InvalidInput
        );
        let now = now_ms()?;
        let tx = self.connection.transaction()?;
        authorized(&tx, id, proof, false, now)?;
        command(&tx, id, cid)?;
        if let Some(old) = tx
            .query_row(
                "SELECT snapshot FROM device_command_events WHERE command_id=?1 AND request_id=?2",
                (cid.to_string(), input.request_id.to_string()),
                |r| r.get::<_, String>(0),
            )
            .optional()?
        {
            let old: CommandEventReceipt = decode(old)?;
            ensure!(old.event == input, ControlError::Conflict);
            return Ok(old);
        }
        // Persist deadline projection even if the new event is rejected.
        maintain(&tx, id, now)?;
        let mut c = command(&tx, id, cid)?;
        // An observed event is not permission to perform its external effect.
        // A late Started/Progress must consume its original sequence even after
        // expiry; otherwise a lost earlier receipt could strand the final result.
        let reconciling = c.state == CommandState::Unknown && c.last_event.is_some();
        if c.state.terminal() && !reconciling {
            tx.commit()?;
            anyhow::bail!(ControlError::Conflict);
        }
        let last = c.last_event.as_ref().map_or(0, |e| e.event.sequence);
        if let Some(e) = &input.execution {
            ensure!(e.valid(), ControlError::InvalidInput);
            if let Some(old) = &c.execution {
                ensure!(
                    e.execution_id == old.execution_id
                        && old.session_id.is_none_or(|id| e.session_id == Some(id))
                        && old.turn_id.is_none_or(|id| e.turn_id == Some(id)),
                    ControlError::Conflict
                );
                for prior in &old.approval_reports {
                    ensure!(
                        e.approval_reports.iter().any(|r| {
                            r.approval_id == prior.approval_id
                                && r.decision == prior.decision
                                && (prior.stage != DeviceApprovalStage::Applied
                                    || r.stage == DeviceApprovalStage::Applied)
                        }),
                        ControlError::Conflict
                    );
                }
            }
            for report in &e.approval_reports {
                let recorded: Option<String> = tx.query_row(
                    "SELECT snapshot FROM device_approval_decisions WHERE command_id=?1 AND approval_id=?2",
                    (cid.to_string(), report.approval_id.to_string()), |r| r.get(0)
                ).optional()?;
                let recorded: DeviceApprovalDecision =
                    decode(recorded.ok_or(ControlError::Conflict)?)?;
                ensure!(
                    recorded.approval_id == report.approval_id
                        && recorded.decision == report.decision,
                    ControlError::Conflict
                );
            }
            use lain_wire::device_execution::ExecutionPhase as P;
            let valid = match input.kind {
                CommandEventKind::Succeeded => e.phase == P::Completed && e.result.is_some(),
                CommandEventKind::Failed => e.phase == P::Failed,
                CommandEventKind::Cancelled => e.phase == P::Cancelled,
                CommandEventKind::Unknown => e.phase == P::Unknown,
                CommandEventKind::Started => {
                    matches!(e.phase, P::Running | P::WaitingForApproval) && e.turn_id.is_some()
                }
                CommandEventKind::Accepted => e.phase == P::Preparing && e.turn_id.is_none(),
                CommandEventKind::Rejected => e.phase == P::Failed,
                CommandEventKind::Progress => {
                    if c.state == CommandState::Accepted
                        || (c.state == CommandState::Unknown && e.phase == P::Preparing)
                    {
                        e.phase == P::Preparing && e.turn_id.is_none()
                    } else {
                        matches!(e.phase, P::Running | P::WaitingForApproval) && e.turn_id.is_some()
                    }
                }
            };
            ensure!(valid, ControlError::InvalidInput);
            // Executions/turns remain exclusive. Only an explicitly linked
            // continuation may reuse its immutable family's original session.
            if let Some(lineage) = &c.continuation {
                ensure!(
                    e.session_id == Some(lineage.session_id),
                    ControlError::Conflict
                );
            }
            let root = c.continuation.as_ref().map_or(c.id, |p| p.root_command_id);
            let session = e.session_id.map(|v| v.to_string());
            let turn = e.turn_id.map(|v| v.to_string());
            let collision: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM device_executions e LEFT JOIN device_work_continuations f ON f.command_id=e.command_id WHERE e.device_id=?1 AND e.command_id!=?2 AND (e.execution_id=?3 OR e.turn_id=?5 OR (e.session_id=?4 AND coalesce(f.root_command_id,e.command_id)!=?6)))",
                (id.to_string(),cid.to_string(),e.execution_id.to_string(),&session,&turn,root.to_string()), |r|r.get(0))?;
            ensure!(!collision, ControlError::Conflict);
            for a in &e.pending_approvals {
                let mut q =
                    tx.prepare("SELECT snapshot FROM device_command_events WHERE command_id=?1")?;
                for row in q.query_map([cid.to_string()], |r| r.get::<_, String>(0))? {
                    let receipt: CommandEventReceipt = decode(row?)?;
                    if let Some(prior) = receipt
                        .event
                        .execution
                        .as_ref()
                        .and_then(|e| e.pending_approvals.iter().find(|old| old.id == a.id))
                    {
                        ensure!(prior == a, ControlError::Conflict);
                    }
                }
            }
        } else {
            ensure!(c.execution.is_none(), ControlError::Conflict);
        }
        use CommandEventKind as E;
        use CommandState as S;
        let final_fact = matches!(
            input.kind,
            E::Succeeded | E::Failed | E::Cancelled | E::Rejected
        );
        ensure!(
            input.sequence == last + 1
                && (input.sequence <= 256 || (reconciling && final_fact && input.sequence == 257)),
            ControlError::Conflict
        );
        let next = match (c.state, input.kind) {
            (S::Queued, E::Accepted) if !c.cancel_requested => S::Accepted,
            (S::Queued, E::Rejected) => S::Rejected,
            (S::Accepted, E::Started) => S::Running,
            (S::Accepted | S::Running, E::Progress) => c.state,
            (S::Accepted | S::Running | S::Unknown, E::Succeeded) => S::Succeeded,
            (S::Unknown, E::Failed) => S::Failed,
            (S::Unknown, E::Started | E::Progress | E::Unknown) => S::Unknown,
            (S::Unknown, E::Cancelled) => S::Cancelled,
            (S::Accepted | S::Running, E::Failed) => S::Failed,
            (S::Accepted | S::Running, E::Cancelled) => S::Cancelled,
            (S::Accepted, E::Rejected) => S::Rejected,
            (S::Accepted | S::Running, E::Unknown) => S::Unknown,
            _ => anyhow::bail!(ControlError::Conflict),
        };
        // Keep the last slot for a terminal observation, even if progress floods.
        ensure!(
            input.sequence < 256 || final_fact || input.kind == E::Unknown,
            ControlError::Unavailable
        );
        c.state = next;
        c.execution = input.execution.clone();
        if let Some(e) = &c.execution {
            tx.execute("INSERT INTO device_executions VALUES(?1,?2,?3,?4,?5) ON CONFLICT(command_id) DO UPDATE SET session_id=excluded.session_id,turn_id=excluded.turn_id",
                (cid.to_string(),id.to_string(),e.execution_id.to_string(),e.session_id.map(|v|v.to_string()),e.turn_id.map(|v|v.to_string())))?;
        }
        c.revision += 1;
        let receipt = CommandEventReceipt {
            event: input,
            received_at_ms: now,
            command_revision: c.revision,
            state: next,
        };
        c.last_event = Some(receipt.clone());
        tx.execute(
            "INSERT INTO device_command_events VALUES(?1,?2,?3,?4)",
            (
                cid.to_string(),
                i64::try_from(receipt.event.sequence)?,
                receipt.event.request_id.to_string(),
                serde_json::to_string(&receipt)?,
            ),
        )?;
        publish(&tx, &c)?;
        tx.commit()?;
        Ok(receipt)
    }
}

pub(super) fn queue_in_transaction(
    tx: &rusqlite::Transaction<'_>,
    id: DeviceId,
    input: QueueDeviceWork,
    now: u64,
) -> Result<DeviceCommand> {
    safe(&input)?;
    ensure!(
        !input.task.trim().is_empty()
            && input.task.len() <= 8192
            && safe_text(&input.task)
            && !input.profile_id.is_empty()
            && input.profile_id.len() <= 128
            && input
                .profile_id
                .split('/')
                .all(|s| !s.is_empty() && s != "." && s != "..")
            && input
                .profile_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"/_-.".contains(&b)),
        ControlError::InvalidInput
    );
    let d = device(tx, id)?;
    ensure!(
        d.state == DeviceState::Active && now < d.registration.expires_at_ms,
        ControlError::Conflict
    );
    if let Some(old) = tx
        .query_row(
            "SELECT snapshot FROM device_commands WHERE device_id=?1 AND request_id=?2",
            (id.to_string(), input.request_id.to_string()),
            |r| r.get::<_, String>(0),
        )
        .optional()?
    {
        let old: DeviceCommand = decode(old)?;
        ensure!(old.input == input, ControlError::Conflict);
        maintain(tx, id, now)?;
        let result = command(tx, id, old.id)?;
        return Ok(result);
    }
    ensure!(
        input.expected_device_revision == d.revision,
        ControlError::Conflict
    );
    ensure!(
        d.registration
            .workspaces
            .iter()
            .any(|w| w.id == input.workspace_id)
            && input.expires_at_ms > now
            && input.expires_at_ms <= now.saturating_add(86400000)
            && input.expires_at_ms <= d.registration.expires_at_ms,
        ControlError::InvalidInput
    );
    let count: i64 = tx.query_row(
        "SELECT count(*) FROM device_commands WHERE device_id=?1",
        [id.to_string()],
        |r| r.get(0),
    )?;
    ensure!(count < 128, ControlError::Unavailable);
    let c = DeviceCommand {
        id: DeviceCommandId::new(),
        device_id: id,
        input,
        created_at_ms: now,
        revision: 1,
        state: CommandState::Queued,
        cancel_requested: false,
        interruption: None,
        last_event: None,
        execution: None,
        continuation: None,
        continued_by: None,
    };
    tx.execute(
        "INSERT INTO device_commands VALUES(?1,?2,?3,?4)",
        (
            c.id.to_string(),
            id.to_string(),
            c.input.request_id.to_string(),
            serde_json::to_string(&c)?,
        ),
    )?;
    publish(tx, &c)?;
    Ok(c)
}
