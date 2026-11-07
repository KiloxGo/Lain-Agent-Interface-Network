//! Durable finite day coordination; source bodies remain in archives.
use super::{Store, briefing, channels::decode};
use crate::{backend::ControlError, domain::now_ms, redaction::Redactor};
use anyhow::Result;
use lain_wire::{ActorTurnState, ConversationId, RequestId, briefing::*};
use rusqlite::OptionalExtension;

impl Store {
    fn save_briefing_run(&self, r: &BriefingRun) -> Result<()> {
        self.connection.execute(
            "UPDATE briefing_runs SET snapshot=?2 WHERE briefing_id=?1",
            (r.briefing_id.to_string(), serde_json::to_string(r)?),
        )?;
        Ok(())
    }
    pub fn briefing_run_status(
        &self,
        c: ConversationId,
        id: RequestId,
    ) -> Result<BriefingRunStatus> {
        let m = self.briefing_manifest(c, id)?;
        let run = decode(
            self.connection
                .query_row(
                    "SELECT snapshot FROM briefing_runs WHERE briefing_id=?1",
                    [id.to_string()],
                    |r| r.get::<_, String>(0),
                )
                .optional()?
                .ok_or(ControlError::NotFound)?,
        )?;
        Ok(BriefingRunStatus {
            run,
            source_current: briefing::current(&self.connection, &m, now_ms()?).is_ok(),
            day_from_ms: m.day_from_ms,
            day_to_ms: m.day_to_ms,
            day_from_local: super::briefing_conversation::local_time(
                m.day_from_ms,
                m.utc_offset_minutes,
            ),
            day_to_local_exclusive: super::briefing_conversation::local_time(
                m.day_to_ms,
                m.utc_offset_minutes,
            ),
            utc_offset_minutes: m.utc_offset_minutes,
            unavailable_records: m.unavailable_records,
            complete_upstream_history: false,
        })
    }
    pub fn begin_briefing_run(&mut self, c: ConversationId, id: RequestId) -> Result<bool> {
        let m = self.briefing_manifest(c, id)?;
        let exists: bool = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM briefing_runs WHERE briefing_id=?1)",
            [id.to_string()],
            |r| r.get(0),
        )?;
        if exists {
            return Ok(false);
        }
        let now = now_ms()?;
        briefing::current(&self.connection, &m, now)?;
        let empty = m.records.is_empty();
        let run = BriefingRun {
            conversation_id: c,
            briefing_id: id,
            phase: if empty {
                BriefingRunPhase::Completed
            } else {
                BriefingRunPhase::Running
            },
            stop: None,
            started_at_ms: now,
            finished_at_ms: empty.then_some(now),
            expires_at_ms: m.expires_at_ms.min(now.saturating_add(20 * 60000)),
            selected_records: u16::try_from(m.records.len())?,
            processed_records: 0,
            completed_batches: 0,
            next_index: if empty { None } else { Some(0) },
            important_items_omitted: false,
            scope_partial: m.scope_clipped || m.record_limit_reached || m.unavailable_records > 0,
            skipped_records: 0,
            skipped_batches: 0,
        };
        self.connection.execute(
            "INSERT INTO briefing_runs VALUES (?1,?2)",
            (id.to_string(), serde_json::to_string(&run)?),
        )?;
        Ok(!empty)
    }
    pub fn advance_briefing_run(
        &mut self,
        c: ConversationId,
        id: RequestId,
    ) -> Result<BriefingRunStatus> {
        let status = self.briefing_run_status(c, id)?;
        let mut run = status.run;
        if run.phase != BriefingRunPhase::Running {
            return self.briefing_run_status(c, id);
        }
        let now = now_ms()?;
        if !status.source_current {
            run.phase = BriefingRunPhase::SourceUnavailable;
            run.stop = Some(BriefingRunStop::SourceChanged);
        } else {
            let index = run.next_index.ok_or(ControlError::Conflict)?;
            let batch = self.briefing_batch(
                BriefingActivity {
                    stage: lain_wire::briefing::BriefingStage::Extract,
                    conversation_id: c,
                    briefing_id: id,
                    from_index: index,
                },
                &Redactor::new(vec![]),
            )?;
            match batch.state {
                ActorTurnState::Completed if batch.items.is_some() => {
                    run.processed_records = batch.next_index.unwrap_or(run.selected_records);
                    run.completed_batches += 1;
                    run.next_index = batch.next_index;
                    run.important_items_omitted |= batch.important_items_omitted.unwrap_or(true);
                    if run.next_index.is_none() {
                        run.phase = BriefingRunPhase::Reconciling;
                    } else if run.completed_batches >= 64 {
                        run.phase = BriefingRunPhase::Partial;
                        run.stop = Some(BriefingRunStop::BatchLimit);
                    }
                }
                ActorTurnState::Running | ActorTurnState::PendingConfirmation => {
                    return self.briefing_run_status(c, id);
                }
                _ if batch.source_current && run.skipped_batches < 8 => {
                    // One unusable answer or failed request should not cost the rest of
                    // the span. Extraction has no side effects. Nothing from the batch is
                    // published and it is not retried; the reply reports the gap.
                    let next = batch.next_index.unwrap_or(run.selected_records);
                    run.skipped_records += next - index;
                    run.skipped_batches += 1;
                    run.processed_records = next;
                    run.next_index = batch.next_index;
                    if run.next_index.is_some() {
                    } else if run.completed_batches > 0 {
                        run.phase = BriefingRunPhase::Reconciling;
                    } else {
                        run.processed_records = 0;
                        run.phase = BriefingRunPhase::Unknown;
                        run.stop = Some(BriefingRunStop::ModelFailed);
                    }
                }
                _ => {
                    run.phase = BriefingRunPhase::Unknown;
                    run.stop = Some(BriefingRunStop::ModelFailed);
                }
            }
        }
        if run.phase.terminal() {
            run.finished_at_ms = Some(now);
        }
        self.save_briefing_run(&run)?;
        self.briefing_run_status(c, id)
    }
    pub fn stop_briefing_run(
        &mut self,
        c: ConversationId,
        id: RequestId,
        reason: BriefingRunStop,
    ) -> Result<()> {
        let status = self.briefing_run_status(c, id)?;
        let mut run = status.run;
        if !run.phase.terminal() {
            // Every item already exists while reconciling, so a failed comparison costs
            // only the grouping of related notices, not the report.
            let items_complete = run.phase == BriefingRunPhase::Reconciling
                && reason == BriefingRunStop::ModelFailed;
            run.phase = if !status.source_current {
                BriefingRunPhase::SourceUnavailable
            } else if items_complete
                || matches!(
                    reason,
                    BriefingRunStop::Deadline | BriefingRunStop::BatchLimit
                )
            {
                BriefingRunPhase::Partial
            } else {
                BriefingRunPhase::Unknown
            };
            run.stop = Some(if status.source_current {
                reason
            } else {
                BriefingRunStop::SourceChanged
            });
            run.finished_at_ms = Some(now_ms()?);
            self.save_briefing_run(&run)?;
        }
        Ok(())
    }
    pub fn recover_briefing_runs(&mut self) -> Result<()> {
        let rows = {
            let mut q=self.connection.prepare("SELECT snapshot FROM briefing_runs WHERE json_extract(snapshot,'$.phase') IN ('running','reconciling')")?;
            q.query_map([], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        for row in rows {
            let mut run: BriefingRun = decode(row)?;
            run.phase = BriefingRunPhase::Unknown;
            run.stop = Some(BriefingRunStop::Interrupted);
            run.finished_at_ms = Some(now_ms()?);
            self.save_briefing_run(&run)?;
        }
        Ok(())
    }
}
