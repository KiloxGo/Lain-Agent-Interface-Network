use super::*;
use crate::backend::ControlError;
use lain_wire::{client_sync::*, device_commands::DeviceCommand};
use rusqlite::Transaction;
#[cfg(test)]
#[path = "client_sync_tests.rs"]
mod tests;

pub(super) fn install(tx: &Transaction<'_>) -> Result<()> {
    tx.execute(
        "INSERT OR IGNORE INTO client_sync_state VALUES(1,?1,0)",
        [uuid::Uuid::new_v4().to_string()],
    )?;
    let tables = {
        let mut q = tx.prepare("SELECT name FROM pragma_table_list WHERE schema='main' AND type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE 'client_sync_%'")?;
        q.query_map([], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    tx.execute("DELETE FROM client_sync_deadlines", [])?;
    for table in tables {
        ensure!(
            table
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_'),
            "invalid persisted table name"
        );
        let snapshot: bool = tx.query_row(
            &format!(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info('{table}') WHERE name='snapshot')"
            ),
            [],
            |r| r.get(0),
        )?;
        let expiry = |reference: &str| {
            format!(
                "SELECT DISTINCT '{table}',{reference}.rowid,CAST(j.atom AS INTEGER) FROM json_tree({reference}.snapshot) j WHERE j.key IN ('expires_at_ms','retain_until_ms','fresh_until_ms') AND j.type='integer' AND CAST(j.atom AS INTEGER)>0"
            )
        };
        if snapshot {
            tx.execute_batch(&format!("INSERT INTO client_sync_deadlines SELECT DISTINCT '{table}',s.rowid,CAST(j.atom AS INTEGER) FROM \"{table}\" s,json_tree(s.snapshot) j WHERE j.key IN ('expires_at_ms','retain_until_ms','fresh_until_ms') AND j.type='integer' AND CAST(j.atom AS INTEGER)>0;"))?;
        }
        for (suffix, operation) in [("i", "INSERT"), ("u", "UPDATE"), ("d", "DELETE")] {
            let mut body =
                String::from("UPDATE client_sync_state SET revision=revision+1 WHERE singleton=1;");
            if snapshot {
                if suffix != "i" {
                    body.push_str(&format!("DELETE FROM client_sync_deadlines WHERE source_table='{table}' AND source_rowid=OLD.rowid;"));
                }
                if suffix != "d" {
                    body.push_str(&format!(
                        "INSERT OR REPLACE INTO client_sync_deadlines {};",
                        expiry("NEW")
                    ));
                }
            }
            tx.execute_batch(&format!("CREATE TRIGGER IF NOT EXISTS lain_sync_{table}_{suffix} AFTER {operation} ON \"{table}\" BEGIN {body} END;"))?;
        }
    }
    Ok(())
}
impl Store {
    pub fn sync_revision(&self) -> Result<u64> {
        Ok(self
            .connection
            .query_row(
                "SELECT revision FROM client_sync_state WHERE singleton=1",
                [],
                |r| r.get::<_, i64>(0),
            )?
            .try_into()?)
    }
    pub fn sync_status(&self, boot_id: uuid::Uuid, now: u64) -> Result<SyncStatus> {
        let (id, revision): (String, i64) = self.connection.query_row(
            "SELECT instance_id,revision FROM client_sync_state WHERE singleton=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let deadline: Option<i64> = self.connection.query_row(
            "SELECT min(deadline_ms) FROM client_sync_deadlines WHERE deadline_ms>?1",
            [i64::try_from(now)?],
            |r| r.get(0),
        )?;
        let instance_id = id.parse::<uuid::Uuid>()?;
        ensure!(!instance_id.is_nil(), "invalid persistent Kernel identity");
        Ok(SyncStatus {
            cursor: SyncCursor {
                instance_id,
                boot_id,
                revision: revision.try_into()?,
            },
            server_ms: now,
            valid_until_ms: deadline
                .map(|v| v as u64)
                .unwrap_or(u64::MAX)
                .min(now.saturating_add(MAX_LEASE_MS)),
        })
    }
    pub fn bootstrap(
        &self,
        request: &BootstrapRequest,
        resources: Resources,
        boot_id: uuid::Uuid,
    ) -> Result<BootstrapSnapshot> {
        ensure!(request.protocol == PROTOCOL, ControlError::InvalidInput);
        let status = self.sync_status(boot_id, now_ms()?)?;
        let devices = self
            .devices()?
            .into_iter()
            .map(|s| DeviceHeader {
                id: s.device.registration.id,
                revision: s.device.revision,
                label: s.device.registration.label,
                state: s.device.state,
                expires_at_ms: s.device.registration.expires_at_ms,
                workspace_ids: s
                    .device
                    .registration
                    .workspaces
                    .into_iter()
                    .map(|w| w.id)
                    .collect(),
                report_sequence: s.latest_report.map(|r| r.report.sequence),
            })
            .collect::<Vec<_>>();
        let mut mappings = Vec::new();
        for device in &devices {
            for w in &device.workspace_ids {
                if let Some(m) = self.project_workspace(device.id, *w)? {
                    mappings.push(m);
                }
            }
        }
        let mut out = BootstrapSnapshot {
            cursor: status.cursor,
            issued_at_ms: status.server_ms,
            valid_until_ms: status.valid_until_ms,
            resources,
            conversation_id: request.conversation_id,
            memory_context: None,
            turns: vec![],
            history_truncated: false,
            devices,
            projects: self.projects()?,
            mappings,
            work: vec![],
            work_truncated: false,
            notices: vec![],
            notice_cursor: 0,
            notices_truncated: false,
        };
        if let Some(id) = request.conversation_id {
            out.memory_context = self.memory_context(id)?;
            let count: i64 = self.connection.query_row(
                "SELECT count(*) FROM lain_actor_turns WHERE conversation_id=?1",
                [id.to_string()],
                |r| r.get(0),
            )?;
            let mut turns = self.actor_recent(id)?;
            if turns.len() > 20 {
                turns.drain(..turns.len() - 20);
            }
            out.history_truncated = count > turns.len() as i64;
            for t in turns {
                out.turns.push(self.actor_view(t.id)?);
            }
            let mut q=self.connection.prepare("SELECT c.snapshot FROM device_work_links l JOIN device_commands c ON c.id=l.command_id AND c.device_id=l.device_id WHERE l.conversation_id=?1 ORDER BY l.created_at_ms DESC,l.command_id DESC LIMIT 33")?;
            let rows = q
                .query_map([id.to_string()], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            out.work_truncated = rows.len() > 32;
            for raw in rows.into_iter().take(32) {
                let c: DeviceCommand = serde_json::from_str(&raw)
                    .map_err(|_| anyhow::anyhow!("invalid device work snapshot"))?;
                out.work.push(WorkPointer {
                    device_id: c.device_id,
                    command_id: c.id,
                    revision: c.revision,
                    state: c.state,
                    expires_at_ms: c.input.expires_at_ms,
                    pending_approvals: c
                        .execution
                        .map(|e| e.pending_approvals.into_iter().map(|a| a.id).collect())
                        .unwrap_or_default(),
                });
            }
            let mut q=self.connection.prepare("SELECT id,sequence,acknowledged FROM lain_conversation_notices WHERE conversation_id=?1 ORDER BY sequence DESC LIMIT 101")?;
            let rows = q
                .query_map([id.to_string()], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, bool>(2)?,
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            out.notices_truncated = rows.len() > 100;
            out.notice_cursor = rows.first().map_or(0, |r| r.1 as u64);
            for (id, cursor, acknowledged) in rows.into_iter().take(100).rev() {
                out.notices.push(NoticePointer {
                    id: id.parse()?,
                    cursor: cursor.try_into()?,
                    acknowledged,
                });
            }
        }
        while serde_json::to_vec(&out)?.len() > MAX_BYTES && !out.turns.is_empty() {
            out.turns.remove(0);
            out.history_truncated = true;
        }
        out.validate().map_err(anyhow::Error::msg)?;
        ensure!(out.valid_until_ms > now_ms()?, ControlError::Conflict);
        Ok(out)
    }
}
