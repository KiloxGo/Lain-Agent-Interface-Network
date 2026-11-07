use super::{Store, channels::decode};
use crate::{backend::ControlError, domain::now_ms};
use anyhow::{Result, ensure};
use lain_wire::{DeviceId, devices::*};
use rusqlite::{Connection, OptionalExtension};
use subtle::ConstantTimeEq;

pub(super) fn device(db: &Connection, id: DeviceId) -> Result<Device> {
    let d: Device = decode(
        db.query_row(
            "SELECT snapshot FROM devices WHERE id=?1",
            [id.to_string()],
            |r| r.get(0),
        )
        .optional()?
        .ok_or(ControlError::NotFound)?,
    )?;
    ensure!(
        d.registration.id == id
            && d.registration.valid()
            && match d.state {
                DeviceState::Pending =>
                    d.revision == 1
                        && d.claimed_at_ms.is_none()
                        && d.claim_request_id.is_none()
                        && d.revoked_at_ms.is_none(),
                DeviceState::Active =>
                    d.revision == 2
                        && d.claimed_at_ms.is_some()
                        && d.claim_request_id.is_some()
                        && d.revoked_at_ms.is_none(),
                DeviceState::Revoked =>
                    d.revoked_at_ms.is_some()
                        && if d.claimed_at_ms.is_some() {
                            d.revision == 3 && d.claim_request_id.is_some()
                        } else {
                            d.revision == 2 && d.claim_request_id.is_none()
                        },
            },
        "invalid device storage identity or lifecycle"
    );
    Ok(d)
}
pub(super) fn safe(value: &impl serde::Serialize) -> Result<()> {
    crate::profiles::check_sensitive(value).map_err(|_| ControlError::InvalidInput)?;
    let json = serde_json::to_string(value)?;
    ensure!(
        json.len() <= 24576
            && crate::redaction::Redactor::new(vec![])
                .for_runtime(None)
                .text(&json)
                == json,
        ControlError::InvalidInput
    );
    Ok(())
}
pub(super) fn authorized(
    db: &Connection,
    id: DeviceId,
    fingerprint: &str,
    pending: bool,
    now: u64,
) -> Result<Device> {
    let d = match device(db, id) {
        Err(e)
            if matches!(
                e.downcast_ref::<ControlError>(),
                Some(ControlError::NotFound)
            ) =>
        {
            anyhow::bail!(super::DeviceAuthenticationDenied)
        }
        result => result?,
    };
    ensure!(
        bool::from(
            d.registration
                .credential_fingerprint
                .as_bytes()
                .ct_eq(fingerprint.as_bytes())
        ) && now < d.registration.expires_at_ms
            && d.state != DeviceState::Revoked
            && (d.state == DeviceState::Active || (pending && now < d.claim_until_ms)),
        super::DeviceAuthenticationDenied
    );
    Ok(d)
}
fn snapshot(db: &Connection, id: DeviceId, now: u64) -> Result<DeviceSnapshot> {
    let d = device(db, id)?;
    let latest_report = db
        .query_row(
            "SELECT snapshot FROM device_reports WHERE device_id=?1 ORDER BY sequence DESC LIMIT 1",
            [id.to_string()],
            |r| r.get::<_, String>(0),
        )
        .optional()?
        .map(decode)
        .transpose()?;
    let expired = now >= d.registration.expires_at_ms
        || (d.state == DeviceState::Pending && now >= d.claim_until_ms);
    Ok(DeviceSnapshot {
        device: d,
        expired,
        latest_report,
    })
}
impl Store {
    pub fn device_register(&mut self, input: RegisterDevice) -> Result<DeviceSnapshot> {
        ensure!(input.valid(), ControlError::InvalidInput);
        safe(&input)?;
        let now = now_ms()?;
        let tx = self.connection.transaction()?;
        if let Some(old) = tx
            .query_row(
                "SELECT snapshot FROM devices WHERE request_id=?1",
                [input.request_id.to_string()],
                |r| r.get::<_, String>(0),
            )
            .optional()?
        {
            let old: Device = decode(old)?;
            ensure!(old.registration == input, ControlError::Conflict);
            return snapshot(&tx, old.registration.id, now);
        }
        ensure!(
            input.expires_at_ms > now && input.expires_at_ms <= now.saturating_add(31 * 86400000),
            ControlError::InvalidInput
        );
        ensure!(
            !tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM devices WHERE id=?1 OR fingerprint=?2)",
                (input.id.to_string(), &input.credential_fingerprint),
                |r| r.get::<_, bool>(0)
            )?,
            ControlError::Conflict
        );
        ensure!(
            tx.query_row("SELECT count(*) FROM devices", [], |r| r.get::<_, i64>(0))? < 128,
            ControlError::Unavailable
        );
        let d = Device {
            claim_until_ms: input.expires_at_ms.min(now.saturating_add(600000)),
            registration: input,
            state: DeviceState::Pending,
            revision: 1,
            registered_at_ms: now,
            claimed_at_ms: None,
            claim_request_id: None,
            revoked_at_ms: None,
        };
        tx.execute(
            "INSERT INTO devices VALUES(?1,?2,?3,?4)",
            (
                d.registration.id.to_string(),
                d.registration.request_id.to_string(),
                &d.registration.credential_fingerprint,
                serde_json::to_string(&d)?,
            ),
        )?;
        let result = snapshot(&tx, d.registration.id, now)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn devices(&self) -> Result<Vec<DeviceSnapshot>> {
        let mut q = self
            .connection
            .prepare("SELECT id FROM devices ORDER BY id LIMIT 128")?;
        q.query_map([], |r| r.get::<_, String>(0))?
            .map(|r| snapshot(&self.connection, r?.parse()?, now_ms()?))
            .collect()
    }
    pub fn device_snapshot(&self, id: DeviceId) -> Result<DeviceSnapshot> {
        snapshot(&self.connection, id, now_ms()?)
    }
    pub fn device_revoke(&mut self, id: DeviceId, input: RevokeDevice) -> Result<DeviceSnapshot> {
        let tx = self.connection.transaction()?;
        let mut d = device(&tx, id)?;
        ensure!(
            input.expected_revision == d.revision,
            ControlError::Conflict
        );
        if d.state != DeviceState::Revoked {
            d.state = DeviceState::Revoked;
            d.revision = d.revision.checked_add(1).ok_or(ControlError::Conflict)?;
            d.revoked_at_ms = Some(now_ms()?);
            tx.execute(
                "UPDATE devices SET snapshot=?2 WHERE id=?1",
                (id.to_string(), serde_json::to_string(&d)?),
            )?;
        }
        super::device_commands::maintain(&tx, id, now_ms()?)?;
        let result = snapshot(&tx, id, now_ms()?)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn device_claim(
        &mut self,
        id: DeviceId,
        fingerprint: &str,
        input: ClaimDevice,
    ) -> Result<DeviceSnapshot> {
        let tx = self.connection.transaction()?;
        let now = now_ms()?;
        let mut d = authorized(&tx, id, fingerprint, true, now)?;
        ensure!(
            input.registration_fingerprint == registration_fingerprint(&d.registration),
            ControlError::Conflict
        );
        if d.state == DeviceState::Active {
            ensure!(
                d.claim_request_id == Some(input.request_id),
                ControlError::Conflict
            );
            return snapshot(&tx, id, now);
        }
        d.state = DeviceState::Active;
        d.revision += 1;
        d.claimed_at_ms = Some(now);
        d.claim_request_id = Some(input.request_id);
        tx.execute(
            "UPDATE devices SET snapshot=?2 WHERE id=?1",
            (id.to_string(), serde_json::to_string(&d)?),
        )?;
        let result = snapshot(&tx, id, now)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn device_own_snapshot(&self, id: DeviceId, fingerprint: &str) -> Result<DeviceSnapshot> {
        let now = now_ms()?;
        authorized(&self.connection, id, fingerprint, false, now)?;
        snapshot(&self.connection, id, now)
    }
    pub fn device_report(
        &mut self,
        id: DeviceId,
        fingerprint: &str,
        input: ReportDevice,
    ) -> Result<DeviceReport> {
        safe(&input)?;
        let tx = self.connection.transaction()?;
        let now = now_ms()?;
        let d = authorized(&tx, id, fingerprint, false, now)?;
        if let Some(old) = tx
            .query_row(
                "SELECT snapshot FROM device_reports WHERE device_id=?1 AND request_id=?2",
                (id.to_string(), input.request_id.to_string()),
                |r| r.get::<_, String>(0),
            )
            .optional()?
        {
            let old: DeviceReport = decode(old)?;
            ensure!(old.report == input, ControlError::Conflict);
            return Ok(old);
        }
        let (last, count): (i64, i64) = tx.query_row(
            "SELECT coalesce(max(sequence),0),count(*) FROM device_reports WHERE device_id=?1",
            [id.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        ensure!(
            input.sequence == u64::try_from(last)?.saturating_add(1)
                && input.sequence <= i64::MAX as u64
                && count < 4096,
            ControlError::Conflict
        );
        let mut workspaces = std::collections::BTreeSet::new();
        let mut capabilities = std::collections::BTreeSet::new();
        ensure!(
            input.available_workspaces.len() <= 8
                && input.capabilities.len() <= 4
                && input
                    .available_workspaces
                    .iter()
                    .all(|id| workspaces.insert(*id)
                        && d.registration.workspaces.iter().any(|w| w.id == *id))
                && input
                    .capabilities
                    .iter()
                    .all(|c| capabilities.insert(c.capability)
                        && d.registration.capabilities.contains(&c.capability)
                        && !c.version.trim().is_empty()
                        && c.version.len() <= 128
                        && !c.version.chars().any(char::is_control)),
            ControlError::InvalidInput
        );
        let receipt = DeviceReport {
            device_id: id,
            device_revision: d.revision,
            received_at_ms: now,
            report: input,
        };
        tx.execute(
            "INSERT INTO device_reports VALUES(?1,?2,?3,?4)",
            (
                id.to_string(),
                i64::try_from(receipt.report.sequence)?,
                receipt.report.request_id.to_string(),
                serde_json::to_string(&receipt)?,
            ),
        )?;
        tx.commit()?;
        Ok(receipt)
    }
}
