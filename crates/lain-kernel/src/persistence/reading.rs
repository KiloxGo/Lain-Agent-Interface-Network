use super::Store;
use crate::backend::ControlError;
use anyhow::{Result, ensure};
use lain_wire::{ReadingId, reading::*};
use rusqlite::{Connection, OptionalExtension};

fn decode<T: serde::de::DeserializeOwned>(s: String) -> Result<T> {
    serde_json::from_str(&s).map_err(|_| anyhow::anyhow!("invalid reading state"))
}
fn save(c: &Connection, a: &ReadingActivity) -> Result<()> {
    c.execute(
        "UPDATE reading_activities SET snapshot=?2, unfinished=?3 WHERE id=?1",
        (
            a.id.to_string(),
            serde_json::to_string(a)?,
            a.state.unfinished(),
        ),
    )?;
    Ok(())
}

impl Store {
    pub fn reading_grant(&self) -> Result<ReadingGrant> {
        self.connection
            .query_row(
                "SELECT snapshot FROM reading_grants ORDER BY revision DESC LIMIT 1",
                [],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .map(decode)
            .transpose()
            .map(Option::unwrap_or_default)
    }
    pub fn reading_grants(&self, after: u64) -> Result<Vec<ReadingGrant>> {
        ensure!(after <= i64::MAX as u64, ControlError::InvalidInput);
        let mut q = self.connection.prepare(
            "SELECT snapshot FROM reading_grants WHERE revision>?1 ORDER BY revision LIMIT 100",
        )?;
        q.query_map([i64::try_from(after)?], |r| r.get::<_, String>(0))?
            .map(|r| decode(r?))
            .collect()
    }
    pub fn set_reading_grant(&mut self, change: SetReadingGrant, now: u64) -> Result<ReadingGrant> {
        let old = self.reading_grant()?;
        ensure!(
            old.revision == change.expected_revision,
            ControlError::Conflict
        );
        let grant = ReadingGrant {
            revision: old
                .revision
                .checked_add(1)
                .filter(|v| *v <= i64::MAX as u64)
                .ok_or(ControlError::Conflict)?,
            changed_at_ms: now,
            scope: change.scope,
        };
        let tx = self.connection.transaction()?;
        tx.execute(
            "INSERT INTO reading_grants VALUES (?1,?2)",
            (
                i64::try_from(grant.revision)?,
                serde_json::to_string(&grant)?,
            ),
        )?;
        // Replacement is a new authorization boundary, never an extension of old work.
        tx.execute("DELETE FROM reading_results", [])?;
        let rows = {
            let mut q = tx.prepare("SELECT snapshot FROM reading_activities WHERE unfinished=1")?;
            q.query_map([], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        for row in rows {
            let mut a: ReadingActivity = decode(row)?;
            a.state = ReadingState::Revoked;
            a.finished_at_ms = Some(now);
            save(&tx, &a)?;
        }
        tx.commit()?;
        Ok(grant)
    }
    pub fn reading_activity(&self, id: ReadingId) -> Result<ReadingActivity> {
        let s = self
            .connection
            .query_row(
                "SELECT snapshot FROM reading_activities WHERE id=?1",
                [id.to_string()],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .ok_or(ControlError::NotFound)?;
        decode(s)
    }
    pub fn reading_activities(&self) -> Result<Vec<ReadingActivity>> {
        let mut q = self.connection.prepare(
            "SELECT snapshot FROM reading_activities ORDER BY created_at_ms DESC,id LIMIT 50",
        )?;
        q.query_map([], |r| r.get::<_, String>(0))?
            .map(|r| decode(r?))
            .collect()
    }
    pub fn admit_reading(
        &mut self,
        request: StartReading,
        now: u64,
    ) -> Result<(ReadingActivity, bool)> {
        if let Some(s) = self
            .connection
            .query_row(
                "SELECT snapshot FROM reading_activities WHERE request_id=?1",
                [request.request_id.to_string()],
                |r| r.get::<_, String>(0),
            )
            .optional()?
        {
            let a: ReadingActivity = decode(s)?;
            ensure!(a.request == request, ControlError::Conflict);
            return Ok((a, false));
        }
        let grant = self.reading_grant()?;
        ensure!(
            grant.revision == request.grant_revision && grant.active(now),
            ControlError::Conflict
        );
        let scope = grant.scope.ok_or(ControlError::InvalidInput)?;
        ensure!(
            scope.urls.contains(&request.url),
            ControlError::InvalidInput
        );
        let attempts: i64 = self.connection.query_row(
            "SELECT count(*) FROM reading_activities WHERE grant_revision=?1",
            [i64::try_from(grant.revision)?],
            |r| r.get(0),
        )?;
        let running: i64 = self.connection.query_row(
            "SELECT count(*) FROM reading_activities WHERE unfinished=1",
            [],
            |r| r.get(0),
        )?;
        ensure!(
            attempts < i64::from(scope.max_requests) && running == 0,
            ControlError::Conflict
        );
        let a = ReadingActivity {
            id: ReadingId::new(),
            request,
            state: ReadingState::Pending,
            created_at_ms: now,
            started_at_ms: None,
            finished_at_ms: None,
            retain_until_ms: now
                .saturating_add(scope.retention_ms)
                .min(scope.expires_at_ms),
            acquired_bytes: 0,
            failure: None,
        };
        self.connection.execute(
            "INSERT INTO reading_activities VALUES (?1,?2,?3,?4,1,?5)",
            (
                a.id.to_string(),
                a.request.request_id.to_string(),
                i64::try_from(grant.revision)?,
                i64::try_from(now)?,
                serde_json::to_string(&a)?,
            ),
        )?;
        Ok((a, true))
    }
    pub fn claim_reading(&mut self, id: ReadingId, now: u64) -> Result<Option<ReadingScope>> {
        let mut a = self.reading_activity(id)?;
        let grant = self.reading_grant()?;
        if a.state != ReadingState::Pending {
            return Ok(None);
        }
        if !grant.active(now)
            || grant.revision != a.request.grant_revision
            || now >= a.retain_until_ms
        {
            a.state = ReadingState::Expired;
            a.finished_at_ms = Some(now);
            save(&self.connection, &a)?;
            return Ok(None);
        }
        a.state = ReadingState::Running;
        a.started_at_ms = Some(now);
        save(&self.connection, &a)?;
        Ok(grant.scope)
    }
    pub fn reading_running(&self, id: ReadingId, now: u64) -> Result<bool> {
        let a = self.reading_activity(id)?;
        let g = self.reading_grant()?;
        Ok(a.state == ReadingState::Running
            && g.revision == a.request.grant_revision
            && g.active(now)
            && now < a.retain_until_ms)
    }
    pub fn finish_reading(
        &mut self,
        id: ReadingId,
        outcome: std::result::Result<String, ReadingFailure>,
        now: u64,
    ) -> Result<()> {
        let mut a = self.reading_activity(id)?;
        if a.state != ReadingState::Running {
            return Ok(());
        }
        let tx = self.connection.transaction()?;
        a.finished_at_ms = Some(now);
        match outcome {
            Ok(text) => {
                a.acquired_bytes = text.len().try_into()?;
                a.state = ReadingState::Completed;
                tx.execute(
                    "INSERT INTO reading_results VALUES (?1,?2)",
                    (id.to_string(), text),
                )?;
            }
            Err(failure) => {
                a.state = ReadingState::Failed;
                a.failure = Some(failure);
            }
        }
        save(&tx, &a)?;
        tx.commit()?;
        Ok(())
    }
    pub fn cancel_reading(&mut self, id: ReadingId, now: u64) -> Result<ReadingActivity> {
        let mut a = self.reading_activity(id)?;
        if a.state.unfinished() {
            a.state = ReadingState::Cancelled;
            a.finished_at_ms = Some(now);
            save(&self.connection, &a)?;
        }
        Ok(a)
    }
    pub fn reading_result(&mut self, id: ReadingId, now: u64) -> Result<ReadingResult> {
        self.prune_reading(now)?;
        let activity = self.reading_activity(id)?;
        let text = self
            .connection
            .query_row(
                "SELECT text FROM reading_results WHERE activity_id=?1",
                [id.to_string()],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        let processing = if activity.state == ReadingState::Completed {
            ReadingProcessing::AcquiredTextOnly
        } else {
            ReadingProcessing::NotAcquired
        };
        Ok(ReadingResult {
            activity,
            text,
            processing,
        })
    }
    pub fn prune_reading(&mut self, now: u64) -> Result<()> {
        let grant = self.reading_grant()?;
        let rows = {
            let mut q = self.connection.prepare("SELECT snapshot FROM reading_activities WHERE unfinished=1 OR id IN (SELECT activity_id FROM reading_results)")?;
            q.query_map([], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        let tx = self.connection.transaction()?;
        for row in rows {
            let mut a: ReadingActivity = decode(row)?;
            if !grant.active(now)
                || a.request.grant_revision != grant.revision
                || now >= a.retain_until_ms
            {
                tx.execute(
                    "DELETE FROM reading_results WHERE activity_id=?1",
                    [a.id.to_string()],
                )?;
                if a.state.unfinished() {
                    a.state = ReadingState::Expired;
                    a.finished_at_ms = Some(now);
                    save(&tx, &a)?;
                }
            }
        }
        tx.commit()?;
        Ok(())
    }
    pub fn recover_reading(&mut self, now: u64) -> Result<()> {
        self.prune_reading(now)?;
        let rows = {
            let mut q = self
                .connection
                .prepare("SELECT snapshot FROM reading_activities WHERE unfinished=1")?;
            q.query_map([], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        let tx = self.connection.transaction()?;
        for row in rows {
            let mut a: ReadingActivity = decode(row)?;
            a.state = ReadingState::Unknown;
            a.finished_at_ms = Some(now);
            save(&tx, &a)?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn reading_deadline(&self) -> Result<Option<u64>> {
        let mut q = self.connection.prepare("SELECT snapshot FROM reading_activities WHERE unfinished=1 OR id IN (SELECT activity_id FROM reading_results)")?;
        let mut deadline = None;
        for row in q.query_map([], |r| r.get::<_, String>(0))? {
            let a: ReadingActivity = decode(row?)?;
            deadline =
                Some(deadline.map_or(a.retain_until_ms, |old: u64| old.min(a.retain_until_ms)));
        }
        Ok(deadline)
    }
}
