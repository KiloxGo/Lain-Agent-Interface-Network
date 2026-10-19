//! Independent, owner-controlled memory. History and summaries are evidence only.
use crate::{
    backend::ControlError,
    delegation::ResultEvidence,
    domain::{TurnId, TurnState, WorkSessionId},
};
use anyhow::{Result, ensure};
pub use lain_wire::memory::*;
pub use lain_wire::{CandidateId, MemoryId, SummaryId};
use serde::{Deserialize, Serialize};

pub(crate) fn validate(draft: &MemoryDraft, provenance: &MemoryProvenance) -> Result<()> {
    provenance
        .validate()
        .map_err(|_| ControlError::InvalidInput)?;
    validate_draft(draft)
}

pub(crate) fn validate_draft(draft: &MemoryDraft) -> Result<()> {
    ensure!(
        !draft.content.trim().is_empty()
            && draft.content.len() <= 4096
            && !draft
                .content
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\t')),
        ControlError::InvalidInput
    );
    ensure!(
        draft.kind != MemoryKind::ProjectKnowledge
            || matches!(draft.scope, MemoryScope::Workspace { .. }),
        ControlError::InvalidInput
    );
    ensure!(
        draft.kind != MemoryKind::Reflection || draft.scope == MemoryScope::Internal,
        ControlError::InvalidInput
    );
    crate::profiles::check_sensitive(draft).map_err(|_| ControlError::InvalidInput)?;
    let encoded = serde_json::to_string(draft)?;
    ensure!(
        crate::redaction::Redactor::new(vec![])
            .for_runtime(None)
            .text(&encoded)
            == encoded,
        ControlError::InvalidInput
    );
    Ok(())
}
pub(crate) fn canonical_scope(scope: MemoryScope) -> Result<MemoryScope> {
    if let MemoryScope::Workspace { path } = scope {
        ensure!(
            path.len() <= 4096 && !path.chars().any(char::is_control),
            ControlError::InvalidInput
        );
        let p = std::path::Path::new(&path);
        ensure!(p.is_absolute(), ControlError::InvalidInput);
        let p = p.canonicalize().map_err(|_| ControlError::InvalidInput)?;
        ensure!(p.is_dir(), ControlError::InvalidInput);
        let path = p.to_str().ok_or(ControlError::InvalidInput)?.to_owned();
        ensure!(
            path.len() <= 4096 && !path.chars().any(char::is_control),
            ControlError::InvalidInput
        );
        ensure!(
            crate::redaction::Redactor::new(vec![])
                .for_runtime(None)
                .text(&path)
                == path,
            ControlError::InvalidInput
        );
        Ok(MemoryScope::Workspace { path })
    } else {
        Ok(scope)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: SummaryId,
    pub work_session_id: WorkSessionId,
    pub worker_turn_id: TurnId,
    pub workspace: std::path::PathBuf,
    pub outcome: TurnState,
    /// Immutable history watermark and worker revision time at creation.
    pub source_history_cursor: u64,
    pub source_turn_updated_at_ms: u64,
    pub created_at_ms: u64,
    /// A worker report is a claim. Completed does not imply passing commands/tests.
    pub evidence: ResultEvidence,
    pub uncertainty: String,
}
