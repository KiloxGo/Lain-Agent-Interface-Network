//! Bounded owner-imported visible records; connector collection is separately implemented.
use crate::{
    backend::ControlError,
    kernel::{Kernel, blocking},
    redaction::Redactor,
};
use anyhow::{Result, ensure};
use lain_wire::archives::*;
use std::{sync::Arc, time::Duration};

pub(crate) fn safe(value: &impl serde::Serialize, redactor: &Redactor) -> Result<()> {
    let text = serde_json::to_string(value)?;
    ensure!(redactor.text(&text) == text, ControlError::InvalidInput);
    crate::profiles::check_sensitive(value).map_err(|_| ControlError::InvalidInput.into())
}
fn identifier(s: &str, max: usize) -> bool {
    !s.trim().is_empty() && s.len() <= max && !s.chars().any(char::is_control)
}
pub(crate) fn validate_key(key: &ArchiveSourceKey) -> Result<()> {
    let valid = match key {
        ArchiveSourceKey::QqPrivate {
            observer_account,
            peer_id: group_id,
        }
        | ArchiveSourceKey::QqGroup {
            observer_account,
            group_id,
        } => [observer_account, group_id].iter().all(|s| {
            s.parse::<u64>()
                .is_ok_and(|n| n > 0 && n.to_string() == **s)
        }),
        ArchiveSourceKey::CodexSession {
            installation,
            session_id,
            workspace,
        } => {
            identifier(installation, 128)
                && identifier(session_id, 256)
                && identifier(workspace, 4096)
        }
    };
    ensure!(valid, ControlError::InvalidInput);
    Ok(())
}
pub(crate) fn validate_scope(scope: Option<&ArchiveScope>, now: u64) -> Result<()> {
    if let Some(s) = scope {
        const MONTH: u64 = 31 * 86400000;
        ensure!(
            s.expires_at_ms > now
                && s.expires_at_ms <= now.saturating_add(MONTH)
                && s.expires_at_ms <= i64::MAX as u64
                && (1..=MONTH).contains(&s.retention_ms)
                && (1..=200_000).contains(&s.max_records),
            ControlError::InvalidInput
        );
    }
    Ok(())
}
pub(crate) fn validate_import(key: &ArchiveSourceKey, q: &ImportArchive) -> Result<()> {
    ensure!(
        (1..=32).contains(&q.records.len()) && serde_json::to_vec(q)?.len() <= 65536,
        ControlError::InvalidInput
    );
    for r in &q.records {
        ensure!(
            identifier(&r.external_id, 256)
                && identifier(&r.speaker, 256)
                && r.source_position
                    .as_ref()
                    .is_none_or(|p| identifier(p, 256))
                && r.occurred_at_ms.is_none_or(|n| n <= i64::MAX as u64),
            ControlError::InvalidInput
        );
        validate_text(&r.text)?;
        ensure!(
            match key {
                ArchiveSourceKey::QqGroup { .. } | ArchiveSourceKey::QqPrivate { .. } =>
                    r.kind == VisibleRecordKind::Message,
                ArchiveSourceKey::CodexSession { .. } => r.kind != VisibleRecordKind::Message,
            },
            ControlError::InvalidInput
        );
    }
    Ok(())
}
pub(crate) fn validate_text(text: &str) -> Result<()> {
    ensure!(
        !text.trim().is_empty()
            && text.len() <= 8192
            && !text
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t')),
        ControlError::InvalidInput
    );
    Ok(())
}
pub async fn maintain(kernel: Arc<Kernel>) {
    let mut changes = kernel.subscribe_archives();
    let mut shutdown = kernel.subscribe_shutdown();
    loop {
        if *shutdown.borrow() {
            return;
        }
        let deadline = match blocking(&kernel, Kernel::archive_deadline).await {
            Ok(t) => t,
            Err(_) => {
                tracing::error!("archive retention maintenance stopped; storage unavailable");
                return;
            }
        };
        let timer = async {
            if let Some(t) = deadline {
                tokio::time::sleep(Duration::from_millis(
                    t.saturating_sub(crate::domain::now_ms().unwrap_or(0)),
                ))
                .await;
            } else {
                std::future::pending::<()>().await;
            }
        };
        tokio::select! {_=timer=>{},_=changes.recv()=>{},_=shutdown.changed()=>{}}
    }
}
#[cfg(test)]
#[path = "archives/recall_actor_tests.rs"]
mod recall_actor;
#[cfg(test)]
mod tests;
