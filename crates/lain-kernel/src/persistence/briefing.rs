//! Durable selected-day evidence. Model processing and semantic memory are separate.
use super::{Store, archives, channels::decode};
use crate::{backend::ControlError, redaction::Redactor};
use anyhow::{Result, ensure};
use lain_wire::{ConversationId, RequestId, archives::*, briefing::*};
use rusqlite::{Connection, OptionalExtension};
use serde_json::json;

pub(super) fn active(
    db: &Connection,
    conversation: ConversationId,
    revision: u64,
    now: u64,
) -> Result<BriefingScope> {
    let grant:BriefingGrant=decode(db.query_row("SELECT snapshot FROM briefing_grants WHERE conversation_id=?1 ORDER BY revision DESC LIMIT 1",[conversation.to_string()],|r|r.get::<_,String>(0)).optional()?.ok_or(ControlError::Unavailable)?)?;
    ensure!(grant.revision == revision, ControlError::Conflict);
    let scope = grant
        .scope
        .filter(|s| now < s.expires_at_ms)
        .ok_or(ControlError::Unavailable)?;
    for selected in &scope.sources {
        let g = archives::grant(db, selected.source_id)?;
        ensure!(
            g.revision == selected.grant_revision,
            ControlError::Conflict
        );
        archives::active(&g, now)?;
        if let Some(upstream) = super::knowledge::state(db, selected.source_id)? {
            ensure!(
                upstream.local_grant_revision == g.revision
                    && upstream.remote_source_enabled
                    && now < upstream.fresh_until_ms,
                ControlError::Unavailable
            );
        }
    }
    Ok(scope)
}
pub(super) fn current(db: &Connection, m: &BriefingManifest, now: u64) -> Result<()> {
    active(db, m.conversation_id, m.request.grant_revision, now)?;
    ensure!(now < m.expires_at_ms, ControlError::Conflict);
    for v in &m.records {
        let r = archives::record(db, v.record_id)?;
        ensure!(
            r.availability == ArchiveAvailability::Stored
                && r.text.is_some()
                && now < r.retain_until_ms
                && version(&r) == *v
                && super::knowledge::available(db, r.id, now)?,
            ControlError::Conflict
        );
    }
    Ok(())
}
/// Reuse an immutable report only while its bounded selected inputs and meaningful
/// coverage are unchanged. Arrival/observation clocks alone do not change yesterday.
pub(super) fn same_inputs(left: &BriefingManifest, right: &BriefingManifest) -> bool {
    let coverage = |m: &BriefingManifest| {
        m.sources
            .iter()
            .map(|s| {
                json!({
                    "source_id":s.status.view.source.id,
                    "coverage":s.status.view.coverage,
                    "read":s.status.latest_read.as_ref().map(|r|json!({
                        "failure":r.failure,"omitted_items":r.omitted_items,
                        "waiting_for_source":r.waiting_for_source,"coverage":r.coverage
                    })),
                    "upstream":s.status.upstream_snapshot.as_ref().map(|u|json!({
                        "store_id":u.store_id,"reader_revision":u.reader_revision,
                        "source_revision":u.remote_source_revision,
                        "coverage":u.coverage.as_ref().map(|c|json!({
                            "bounded_history":c.bounded_history,
                            "source_epochs_untracked":c.source_epochs_untracked,
                            "events":c.events
                        }))
                    }))
                })
            })
            .collect::<Vec<_>>()
    };
    left.conversation_id == right.conversation_id
        && left.request.grant_revision == right.request.grant_revision
        && left.day_from_ms == right.day_from_ms
        && left.day_to_ms == right.day_to_ms
        && left.utc_offset_minutes == right.utc_offset_minutes
        && left.permitted_from_ms == right.permitted_from_ms
        && left.scope_clipped == right.scope_clipped
        && left.record_limit_reached == right.record_limit_reached
        && left.unavailable_records == right.unavailable_records
        && left.receipt_time_fallback_records == right.receipt_time_fallback_records
        && left.records == right.records
        && coverage(left) == coverage(right)
}
pub(super) fn insert_manifest(db: &Connection, manifest: &BriefingManifest) -> Result<()> {
    db.execute(
        "INSERT INTO briefing_manifests VALUES (?1,?2,?3,?4)",
        (
            manifest.request.request_id.to_string(),
            manifest.conversation_id.to_string(),
            serde_json::to_string(&manifest.request)?,
            serde_json::to_string(manifest)?,
        ),
    )?;
    Ok(())
}
pub(super) fn page(
    db: &Connection,
    m: &BriefingManifest,
    from_index: u16,
    now: u64,
    redactor: &Redactor,
) -> Result<BriefingPage> {
    current(db, m, now)?;
    ensure!(
        usize::from(from_index) <= m.records.len(),
        ControlError::Conflict
    );
    let mut records = vec![];
    let mut bytes = 0;
    for v in m.records.iter().skip(usize::from(from_index)).take(8) {
        let r = archives::record(db, v.record_id)?;
        crate::archives::safe(&r, redactor)?;
        let size = r.text.as_ref().ok_or(ControlError::Conflict)?.len();
        ensure!(size <= 8192, ControlError::InvalidInput);
        if bytes + size > 16384 {
            break;
        }
        bytes += size;
        records.push(r);
    }
    let end = usize::from(from_index) + records.len();
    Ok(BriefingPage {
        briefing_id: m.request.request_id,
        conversation_id: m.conversation_id,
        from_index,
        next_index: (end < m.records.len()).then_some(u16::try_from(end)?),
        total_selected: u16::try_from(m.records.len())?,
        text_bytes: u32::try_from(bytes)?,
        records,
    })
}

fn version(r: &ArchiveRecord) -> ArchiveRecallVersion {
    ArchiveRecallVersion {
        record_id: r.id,
        revision: r.revision,
        source_id: r.source_id,
        source_grant_revision: r.grant_revision,
    }
}
impl Store {
    fn briefing_conversation(&self, conversation: ConversationId) -> Result<()> {
        ensure!(
            self.connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM lain_conversations WHERE id=?1)",
                [conversation.to_string()],
                |r| r.get::<_, bool>(0)
            )?,
            ControlError::NotFound
        );
        Ok(())
    }
    pub fn briefing_grant(&self, conversation: ConversationId) -> Result<BriefingGrant> {
        self.briefing_conversation(conversation)?;
        self.connection.query_row("SELECT snapshot FROM briefing_grants WHERE conversation_id=?1 ORDER BY revision DESC LIMIT 1", [conversation.to_string()], |r|r.get::<_,String>(0))
            .optional()?.map(decode).transpose().map(|g|g.unwrap_or(BriefingGrant { conversation_id:conversation, revision:0, scope:None, changed_at_ms:0 }))
    }
    pub fn briefing_grant_receipt(
        &self,
        conversation: ConversationId,
        request_id: RequestId,
    ) -> Result<BriefingGrantReceipt> {
        self.briefing_conversation(conversation)?;
        let (request, snapshot) = self.connection.query_row(
            "SELECT request,snapshot FROM briefing_grants WHERE conversation_id=?1 AND request_id=?2",
            (conversation.to_string(), request_id.to_string()),
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        ).optional()?.ok_or(ControlError::NotFound)?;
        Ok(BriefingGrantReceipt {
            request: decode(request)?,
            grant: decode(snapshot)?,
        })
    }
    pub fn briefing_set(
        &mut self,
        conversation: ConversationId,
        q: SetBriefingGrant,
        now: u64,
    ) -> Result<BriefingGrant> {
        self.briefing_conversation(conversation)?;
        if let Some((c, request, snapshot)) = self
            .connection
            .query_row(
                "SELECT conversation_id,request,snapshot FROM briefing_grants WHERE request_id=?1",
                [q.request_id.to_string()],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
        {
            ensure!(
                c == conversation.to_string() && decode::<SetBriefingGrant>(request)? == q,
                ControlError::Conflict
            );
            return decode(snapshot);
        }
        let old = self.briefing_grant(conversation)?;
        ensure!(old.revision == q.expected_revision, ControlError::Conflict);
        if let Some(scope) = &q.scope {
            ensure!(
                (1..=8).contains(&scope.sources.len())
                    && (1..=4096).contains(&scope.max_records)
                    && (-840..=840).contains(&scope.utc_offset_minutes)
                    && scope.from_ms <= now
                    && scope.expires_at_ms > now
                    && scope.expires_at_ms <= now.saturating_add(30 * 86400000)
                    && scope.expires_at_ms <= i64::MAX as u64,
                ControlError::InvalidInput
            );
            let mut seen = std::collections::BTreeSet::new();
            for selected in &scope.sources {
                ensure!(seen.insert(selected.source_id), ControlError::InvalidInput);
                let grant = archives::grant(&self.connection, selected.source_id)?;
                ensure!(
                    grant.revision == selected.grant_revision,
                    ControlError::Conflict
                );
                let import = archives::active(&grant, now)?;
                ensure!(
                    scope.expires_at_ms <= import.expires_at_ms,
                    ControlError::InvalidInput
                );
                ensure!(
                    matches!(
                        archives::source(&self.connection, selected.source_id)?.key,
                        ArchiveSourceKey::QqGroup { .. } | ArchiveSourceKey::QqPrivate { .. }
                    ),
                    ControlError::InvalidInput
                );
            }
        }
        let revision = old
            .revision
            .checked_add(1)
            .filter(|n| *n <= i64::MAX as u64)
            .ok_or(ControlError::Conflict)?;
        let grant = BriefingGrant {
            conversation_id: conversation,
            revision,
            scope: q.scope.clone(),
            changed_at_ms: now,
        };
        self.connection.execute(
            "INSERT INTO briefing_grants VALUES (?1,?2,?3,?4,?5)",
            (
                conversation.to_string(),
                i64::try_from(revision)?,
                q.request_id.to_string(),
                serde_json::to_string(&q)?,
                serde_json::to_string(&grant)?,
            ),
        )?;
        Ok(grant)
    }
    fn briefing_active(
        &self,
        conversation: ConversationId,
        revision: u64,
        now: u64,
    ) -> Result<BriefingScope> {
        active(&self.connection, conversation, revision, now)
    }
    pub fn briefing_start(
        &mut self,
        conversation: ConversationId,
        q: StartBriefing,
        now: u64,
    ) -> Result<BriefingManifest> {
        if let Some((c, request, snapshot)) = self
            .connection
            .query_row(
                "SELECT conversation_id,request,snapshot FROM briefing_manifests WHERE id=?1",
                [q.request_id.to_string()],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
        {
            ensure!(
                c == conversation.to_string() && decode::<StartBriefing>(request)? == q,
                ControlError::Conflict
            );
            // An exact historical receipt is metadata only; it cannot grant body disclosure.
            return decode(snapshot);
        }
        let manifest = self.prepare_briefing_snapshot(conversation, &q, now)?;
        insert_manifest(&self.connection, &manifest)?;
        Ok(manifest)
    }
    pub(super) fn prepare_briefing_snapshot(
        &self,
        conversation: ConversationId,
        q: &StartBriefing,
        now: u64,
    ) -> Result<BriefingManifest> {
        let scope = self.briefing_active(conversation, q.grant_revision, now)?;
        let (from, to, _) = crate::reading::day_bounds(now, scope.utc_offset_minutes);
        let day_from_ms = u64::try_from(from)?;
        let day_to_ms = u64::try_from(to)?;
        let permitted_from_ms = day_from_ms.max(scope.from_ms).min(day_to_ms);
        let ids = scope
            .sources
            .iter()
            .map(|s| s.source_id)
            .collect::<Vec<_>>();
        // SQL bounds candidate scanning independently of selected-body/model bounds.
        let candidates = {
            let mut st=self.connection.prepare("SELECT id FROM archive_records WHERE source_id IN (SELECT value FROM json_each(?1)) AND COALESCE(json_extract(snapshot,'$.occurred_at_ms'),json_extract(snapshot,'$.received_at_ms'))>=?2 AND COALESCE(json_extract(snapshot,'$.occurred_at_ms'),json_extract(snapshot,'$.received_at_ms'))<?3 ORDER BY sequence LIMIT 8193")?;
            st.query_map(
                (
                    serde_json::to_string(&ids)?,
                    i64::try_from(permitted_from_ms)?,
                    i64::try_from(day_to_ms)?,
                ),
                |r| r.get::<_, String>(0),
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?
        };
        let mut manifest = BriefingManifest {
            conversation_id: conversation,
            request: q.clone(),
            created_at_ms: now,
            expires_at_ms: scope.expires_at_ms.min(now.saturating_add(86400000)),
            utc_offset_minutes: scope.utc_offset_minutes,
            day_from_ms,
            day_to_ms,
            permitted_from_ms,
            scope_clipped: permitted_from_ms > day_from_ms,
            record_limit_reached: candidates.len() > 8192,
            unavailable_records: 0,
            receipt_time_fallback_records: 0,
            records: vec![],
            sources: vec![],
            complete_upstream_history: false,
        };
        for candidate in candidates.iter().take(8192) {
            let r = self.archive_record(candidate.parse()?, now)?;
            if r.availability != ArchiveAvailability::Stored || r.text.is_none() {
                manifest.unavailable_records += 1;
                continue;
            }
            if manifest.records.len() == usize::from(scope.max_records) {
                manifest.record_limit_reached = true;
                break;
            }
            manifest.expires_at_ms = manifest.expires_at_ms.min(r.retain_until_ms);
            manifest.receipt_time_fallback_records += u64::from(r.occurred_at_ms.is_none());
            manifest.records.push(version(&r));
        }
        for selected in &scope.sources {
            let status = self.archive_source_status(selected.source_id, now)?;
            manifest.sources.push(BriefingSource {
                through_sequence: status.view.committed_through,
                status,
            });
        }
        Ok(manifest)
    }
    pub fn briefing_manifest(
        &self,
        conversation: ConversationId,
        id: RequestId,
    ) -> Result<BriefingManifest> {
        self.briefing_conversation(conversation)?;
        decode(
            self.connection
                .query_row(
                    "SELECT snapshot FROM briefing_manifests WHERE id=?1 AND conversation_id=?2",
                    (id.to_string(), conversation.to_string()),
                    |r| r.get::<_, String>(0),
                )
                .optional()?
                .ok_or(ControlError::NotFound)?,
        )
    }
    pub fn briefing_page(
        &self,
        conversation: ConversationId,
        id: RequestId,
        from_index: u16,
        now: u64,
        redactor: &Redactor,
    ) -> Result<BriefingPage> {
        let manifest = self.briefing_manifest(conversation, id)?;
        page(&self.connection, &manifest, from_index, now, redactor)
    }
}
