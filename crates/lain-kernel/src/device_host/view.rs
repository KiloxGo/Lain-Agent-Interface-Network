use super::*;
use crate::events::EventKind;
pub(super) fn base(j: &Journal, phase: ExecutionPhase, submitted: bool) -> ExecutionEvidence {
    ExecutionEvidence {
        execution_id: j.intent.execution,
        session_id: Some(j.intent.session),
        turn_id: submitted.then_some(j.intent.turn),
        phase,
        pending_approvals: vec![],
        approval_reports: vec![],
        result: None,
    }
}
pub(super) fn phase(turn: Option<TurnState>) -> ExecutionPhase {
    match turn {
        Some(TurnState::Completed) => ExecutionPhase::Completed,
        Some(TurnState::Cancelled) => ExecutionPhase::Cancelled,
        Some(TurnState::Failed) => ExecutionPhase::Failed,
        _ => ExecutionPhase::Unknown,
    }
}
pub(super) fn permission(
    e: &ExecutionEvidence,
    a: &domain::Approval,
    redact: &Redactor,
) -> Result<DeviceApproval> {
    let (description, changed) = safe_text(redact, &a.description, 2048, false);
    let truncated = changed
        || a.description.contains("[sensitive output redacted]")
        || a.description.contains("[description truncated]")
        || a.description.contains("(not provided)")
        || a.kind == domain::ApprovalKind::FileChange;
    let choices = a
        .decisions
        .iter()
        .filter_map(|d| match d {
            ApprovalDecision::ApproveOnce if !truncated => Some(DeviceApprovalChoice::ApproveOnce),
            ApprovalDecision::Deny => Some(DeviceApprovalChoice::Deny),
            ApprovalDecision::Cancel => Some(DeviceApprovalChoice::Cancel),
            _ => None,
        })
        .collect::<Vec<_>>();
    ensure!(
        !choices.is_empty(),
        "incomplete approval scope has no safe choice"
    );
    let kind = match a.kind {
        domain::ApprovalKind::Command => "command",
        domain::ApprovalKind::FileChange => "file_change",
        domain::ApprovalKind::Permissions => "permissions",
    };
    let mut a = DeviceApproval {
        id: a.id.to_string().parse()?,
        kind: kind.into(),
        description,
        truncated,
        choices,
        fingerprint: String::new(),
    };
    a.fingerprint = approval_fingerprint(e.execution_id, e.session_id, e.turn_id, &a);
    Ok(a)
}
pub(super) fn permission_report(
    e: &ExecutionEvidence,
    a: &domain::Approval,
    receipt: &DeviceApprovalDecision,
    redact: &Redactor,
) -> Result<DeviceApprovalReport> {
    let scope = permission(e, a, redact)?;
    ensure!(
        Some(a.work_session_id.to_string().parse()?) == e.session_id
            && a.turn_id.map(|id| id.to_string().parse()).transpose()? == e.turn_id
            && receipt.approval_id == scope.id
            && receipt.decision.fingerprint == scope.fingerprint
            && scope.choices.contains(&receipt.decision.choice),
        "decision differs from original native permission"
    );
    let choice = match receipt.decision.choice {
        DeviceApprovalChoice::ApproveOnce => ApprovalDecision::ApproveOnce,
        DeviceApprovalChoice::Deny => ApprovalDecision::Deny,
        DeviceApprovalChoice::Cancel => ApprovalDecision::Cancel,
    };
    ensure!(
        a.decision.is_none_or(|d| d == choice),
        "native permission decision differs from cloud receipt"
    );
    let stage = if a.status == ApprovalStatus::Resolved {
        ensure!(
            a.decision == Some(choice),
            "resolution without matching native choice"
        );
        DeviceApprovalStage::Applied
    } else {
        DeviceApprovalStage::Received
    };
    Ok(DeviceApprovalReport {
        approval_id: receipt.approval_id,
        decision: receipt.decision.clone(),
        stage,
    })
}
pub(super) fn reports(
    j: &Journal,
    k: &Kernel,
    redact: &Redactor,
) -> Result<Vec<DeviceApprovalReport>> {
    let e = base(j, ExecutionPhase::Running, true);
    let mut reports = j
        .last
        .as_ref()
        .and_then(|last| last.execution.as_ref())
        .map_or_else(Vec::new, |last| last.approval_reports.clone());
    for a in k
        .approvals(j.intent.session())?
        .into_iter()
        .filter(|a| a.turn_id == Some(j.intent.turn()))
    {
        let path = j.path.join(format!("decision-{}.json", a.id));
        if !path.try_exists()? {
            continue;
        }
        let receipt: DeviceApprovalDecision = journal::read(&path)?;
        let report = permission_report(&e, &a, &receipt, redact)?;
        if let Some(prior) = reports
            .iter_mut()
            .find(|r| r.approval_id == report.approval_id)
        {
            ensure!(prior.decision == report.decision, "device decision changed");
            if prior.stage != DeviceApprovalStage::Applied {
                *prior = report;
            }
        } else {
            reports.push(report);
        }
    }
    reports.sort_by_key(|r| r.approval_id);
    ensure!(reports.len() <= 256, "permission report capacity reached");
    Ok(reports)
}
pub(super) fn observe(j: &Journal, k: &Kernel, redact: &Redactor) -> Result<ExecutionEvidence> {
    let mut e = base(j, ExecutionPhase::Running, true);
    e.approval_reports = reports(j, k, redact)?;
    if let Some(a) = k
        .approvals(j.intent.session())?
        .into_iter()
        .find(|a| a.turn_id == Some(j.intent.turn()) && a.status == ApprovalStatus::Pending)
    {
        e.pending_approvals.push(permission(&e, &a, redact)?);
        e.phase = ExecutionPhase::WaitingForApproval;
    }
    Ok(e)
}
pub(super) fn result(j: &Journal, k: &Kernel, redact: &Redactor) -> Result<DeviceWorkResult> {
    let page = k.history_before(j.intent.session(), i64::MAX as u64, None, 200)?;
    let session = k
        .session(j.intent.session())?
        .context("result session missing")?;
    let mut result = DeviceWorkResult {
        output_tail: String::new(),
        command_failures: 0,
        reported_paths: vec![],
        paths_withheld: 0,
        history_truncated: page.has_more
            || page
                .events
                .iter()
                .any(|e| matches!(e.kind, EventKind::BackendReconnected)),
    };
    for event in page
        .events
        .into_iter()
        .filter(|e| e.turn_id == Some(j.intent.turn()))
    {
        result.history_truncated |= event.truncated;
        match event.kind {
            EventKind::AssistantMessageCompleted { text } => {
                let (tail, changed) = safe_text(redact, &text, 2048, true);
                result.output_tail = tail;
                result.history_truncated |= changed;
            }
            EventKind::CommandCompleted { exit_code, .. } => {
                if exit_code.is_some_and(|n| n != 0) {
                    result.command_failures += 1;
                }
            }
            EventKind::FileChanged { paths } => {
                for p in paths {
                    let path = Path::new(&p);
                    let relative = if path.is_absolute() {
                        path.strip_prefix(&session.workspace).ok()
                    } else {
                        Some(path)
                    };
                    let relative = relative
                        .and_then(Path::to_str)
                        .map(|s| s.replace(std::path::MAIN_SEPARATOR, "/"));
                    if let Some(p) = relative.filter(|p| {
                        !p.is_empty()
                            && p.len() <= 512
                            && !p.contains(['\\', ':'])
                            && !p.chars().any(char::is_control)
                            && p.split('/').all(|s| !s.is_empty() && s != "." && s != "..")
                            && redact.text(p) == *p
                    }) && result.reported_paths.len() < 8
                    {
                        if !result.reported_paths.contains(&p) {
                            result.reported_paths.push(p);
                        }
                    } else {
                        result.paths_withheld += 1;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(result)
}
