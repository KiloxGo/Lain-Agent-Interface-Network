use super::*;

#[test]
fn permission_report_requires_matching_native_scope_choice_and_resolution() {
    let root = tempfile::tempdir().unwrap();
    let c = command();
    let j = Journal::open(root.path(), &c, "manifest".into(), "policy".into()).unwrap();
    let e = view::base(&j, ExecutionPhase::Running, true);
    let redact = Redactor::new(vec![]);
    let mut a = domain::Approval {
        id: domain::ApprovalId::new(),
        work_session_id: j.intent.session(),
        turn_id: Some(j.intent.turn()),
        kind: domain::ApprovalKind::Command,
        description: "echo fixture in explicit workspace".into(),
        decisions: vec![ApprovalDecision::ApproveOnce, ApprovalDecision::Deny],
        status: ApprovalStatus::Pending,
        decision: None,
        created_at_ms: 1,
        updated_at_ms: 1,
    };
    let scope = view::permission(&e, &a, &redact).unwrap();
    let receipt = DeviceApprovalDecision {
        approval_id: scope.id,
        decision: DecideDeviceApproval {
            request_id: RequestId::new(),
            fingerprint: scope.fingerprint,
            choice: DeviceApprovalChoice::ApproveOnce,
        },
        received_at_ms: 1,
    };
    for status in [
        ApprovalStatus::Pending,
        ApprovalStatus::Responding,
        ApprovalStatus::Unknown,
        ApprovalStatus::Expired,
    ] {
        a.status = status;
        assert_eq!(
            view::permission_report(&e, &a, &receipt, &redact)
                .unwrap()
                .stage,
            DeviceApprovalStage::Received
        );
    }
    a.status = ApprovalStatus::Resolved;
    a.decision = Some(ApprovalDecision::Deny);
    assert!(view::permission_report(&e, &a, &receipt, &redact).is_err());
    a.decision = Some(ApprovalDecision::ApproveOnce);
    assert_eq!(
        view::permission_report(&e, &a, &receipt, &redact)
            .unwrap()
            .stage,
        DeviceApprovalStage::Applied
    );
    a.description.push_str(" changed");
    assert!(view::permission_report(&e, &a, &receipt, &redact).is_err());
    a.description = "echo fixture in explicit workspace".into();
    a.turn_id = Some(domain::TurnId::new());
    assert!(view::permission_report(&e, &a, &receipt, &redact).is_err());
}

#[test]
fn restart_never_resubmits_a_claimed_or_unmapped_turn() {
    assert_eq!(
        recovery_plan(false, false, false, false, false, false),
        RecoveryPlan::Launch
    );
    assert_eq!(
        recovery_plan(true, false, false, false, false, false),
        RecoveryPlan::Unknown
    );
    assert_eq!(
        recovery_plan(true, false, true, false, false, false),
        RecoveryPlan::ResumeThenSubmit
    );
    assert_eq!(
        recovery_plan(true, true, true, true, false, false),
        RecoveryPlan::ResumeObserve
    );
    assert_eq!(
        recovery_plan(true, true, true, false, false, false),
        RecoveryPlan::Unknown
    );
    assert_eq!(
        recovery_plan(true, true, true, true, true, false),
        RecoveryPlan::Observe
    );
    assert_eq!(
        recovery_plan(false, true, false, false, false, false),
        RecoveryPlan::Unknown
    );
}

#[test]
fn idle_continuation_recovery_distinguishes_parent_binding_from_an_unclaimed_new_thread() {
    assert_eq!(
        recovery_plan(false, false, true, false, false, true),
        RecoveryPlan::Launch,
        "an independently verified idle parent is expected to be bound before the child's launch fence"
    );
    assert_eq!(
        recovery_plan(false, false, true, false, false, false),
        RecoveryPlan::Unknown,
        "a new root's unexplained binding must still fail closed"
    );
    assert_eq!(
        recovery_plan(false, true, true, false, false, true),
        RecoveryPlan::Unknown,
        "even a verified parent never authorizes replay of an uncertain child submission"
    );
}

#[test]
fn hidden_scope_removes_approval_and_redacts_before_taking_a_tail() {
    let redact = crate::redaction::Redactor::new(vec!["fixture-private-value".into()]);
    let (text, changed) = safe_text(
        &redact,
        &format!("{}fixture-private-value", "a".repeat(3000)),
        2048,
        false,
    );
    assert!(changed);
    assert!(!text.contains("private-value"));
    assert_eq!(text, "[sensitive output redacted]");
    let (text, changed) = safe_text(&redact, &"中".repeat(1000), 2048, true);
    assert!(changed && text.len() <= 2048);
    assert!(text.chars().all(|c| c == '中'));
    let (text, changed) = safe_text(&redact, "safe\u{1b}[31m", 2048, false);
    assert!(changed);
    assert!(!text.contains('\u{1b}'));
}

fn command() -> DeviceCommand {
    DeviceCommand {
        id: DeviceCommandId::new(),
        device_id: DeviceId::new(),
        input: QueueDeviceWork {
            request_id: RequestId::new(),
            expected_device_revision: 2,
            workspace_id: lain_wire::DeviceWorkspaceId::new(),
            profile_id: "codex/local".into(),
            sandbox: lain_wire::Sandbox::ReadOnly,
            task: "synthetic native intent".into(),
            expires_at_ms: domain::now_ms().unwrap() + 60000,
        },
        created_at_ms: domain::now_ms().unwrap(),
        revision: 1,
        state: CommandState::Queued,
        cancel_requested: false,
        interruption: None,
        last_event: None,
        execution: None,
        continuation: None,
        continued_by: None,
    }
}
#[test]
fn immutable_journal_preserves_effect_fences_and_rejects_changed_task() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("execution");
    journal::directory(&dir).unwrap();
    let c = command();
    let j = Journal::open(&dir, &c, "manifest-v1".into(), "policy-v1".into()).unwrap();
    let original = (j.intent.execution, j.intent.session, j.intent.turn);
    j.claim("launch").unwrap();
    drop(j);
    let j = Journal::open(&dir, &c, "manifest-v1".into(), "policy-v1".into()).unwrap();
    assert_eq!(
        original,
        (j.intent.execution, j.intent.session, j.intent.turn)
    );
    assert!(j.claimed("launch").unwrap());
    assert!(!j.claimed("submission").unwrap());
    let mut changed = c.clone();
    changed.input.task = "different scope".into();
    assert!(Journal::open(&dir, &changed, "manifest-v1".into(), "policy-v1".into()).is_err());
    assert!(Journal::open(&dir, &c, "manifest-v1".into(), "changed-policy".into()).is_err());
    assert!(!j.claimed("submission").unwrap());
}
#[tokio::test]
async fn unresolved_or_mismatched_receipts_never_advance_an_outbox() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("execution");
    journal::directory(&dir).unwrap();
    let c = command();
    let mut j = Journal::open(&dir, &c, "manifest".into(), "policy".into()).unwrap();
    j.event(
        CommandEventKind::Accepted,
        Some(view::base(&j, ExecutionPhase::Preparing, false)),
        "test acceptance",
    )
    .unwrap();
    let saved = std::fs::read(j.path.join("event-1.json")).unwrap();
    assert!(
        j.event(
            CommandEventKind::Unknown,
            Some(view::base(&j, ExecutionPhase::Unknown, false)),
            "unknown"
        )
        .is_err()
    );
    let mut wrong = j.last.clone().unwrap();
    wrong.request_id = RequestId::new();
    journal::write(
        &j.path.join("receipt-1.json"),
        &CommandEventReceipt {
            event: wrong,
            received_at_ms: 1,
            command_revision: 2,
            state: CommandState::Accepted,
        },
    )
    .unwrap();
    let cloud = cloud::Cloud::new(
        "http://127.0.0.1:1",
        &format!("lain-device-{}", "ab".repeat(32)),
        c.device_id,
    )
    .unwrap();
    assert!(j.flush(&cloud).await.is_err());
    assert!(
        j.event(
            CommandEventKind::Unknown,
            Some(view::base(&j, ExecutionPhase::Unknown, false)),
            "unknown"
        )
        .is_err()
    );
    assert_eq!(std::fs::read(j.path.join("event-1.json")).unwrap(), saved);
}
#[test]
fn stable_session_preparation_is_metadata_only_and_cannot_rebind_scope() {
    let dir = tempfile::tempdir().unwrap();
    let k = Kernel::open(&dir.path().join("worker.db")).unwrap();
    k.mark_ready().unwrap();
    let id = domain::WorkSessionId::new();
    let options = domain::SessionOptions {
        model: None,
        sandbox: Some(domain::SandboxMode::ReadOnly),
    };
    let first = k
        .prepare_device_session(id, dir.path().to_path_buf(), options.clone())
        .unwrap();
    let same = k
        .prepare_device_session(id, dir.path().to_path_buf(), options)
        .unwrap();
    assert_eq!(first, same);
    assert_eq!(first.state(), domain::WorkSessionState::Created);
    assert_eq!(k.list_sessions().unwrap().len(), 1);
    assert!(k.turns(id).unwrap().is_empty());
    assert!(
        k.prepare_device_session(
            id,
            dir.path().to_path_buf(),
            domain::SessionOptions {
                model: None,
                sandbox: Some(domain::SandboxMode::WorkspaceWrite)
            }
        )
        .is_err()
    );
    assert_eq!(k.history(id, 0, None, 200).unwrap().events.len(), 1);
}

#[test]
fn interrupted_staging_never_publishes_a_partial_effect_fence() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("execution");
    journal::directory(&dir).unwrap();
    let c = command();
    let j = Journal::open(&dir, &c, "manifest".into(), "policy".into()).unwrap();
    let selected = j.path.join("launch.json");
    let staged = journal::stage(&selected, b"true").unwrap();
    assert!(!selected.exists());
    assert!(staged.exists());
    assert!(!j.claimed("launch").unwrap());
    drop(j);
    let j = Journal::open(&dir, &c, "manifest".into(), "policy".into()).unwrap();
    assert!(!j.claimed("launch").unwrap());
    j.claim("launch").unwrap();
    assert!(j.claimed("launch").unwrap());
    assert_eq!(std::fs::read(&selected).unwrap(), b"true");
    assert!(staged.exists()); // Interrupted staging is private evidence, never a logical fence.
}

#[test]
fn native_continuation_requires_immutable_parent_and_reuses_its_exact_session() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("execution");
    journal::directory(&path).unwrap();
    let parent = command();
    let parent_session =
        lain_wire::WorkSessionId(uuid::Uuid::new_v5(&parent.id.0, b"lain-device-session-v1"));
    let parent_turn =
        lain_wire::WorkerTurnId(uuid::Uuid::new_v5(&parent.id.0, b"lain-device-turn-v1"));
    let mut child = parent.clone();
    child.id = DeviceCommandId::new();
    child.input.request_id = RequestId::new();
    child.input.task = "follow-up exact text".into();
    child.continuation = Some(lain_wire::device_work::DeviceContinuation {
        root_command_id: parent.id,
        parent_command_id: parent.id,
        expected_parent_revision: parent.revision,
        session_id: parent_session,
        parent_turn_id: parent_turn,
    });
    assert!(
        Journal::open(&path, &child, "manifest".into(), "policy".into()).is_err(),
        "missing original journal must not create a replacement thread"
    );
    let p = Journal::open(&path, &parent, "manifest".into(), "policy".into()).unwrap();
    assert!(
        Journal::open(&path, &child, "manifest".into(), "policy".into()).is_err(),
        "unsubmitted parent is not a continuation target"
    );
    p.claim("submission").unwrap();
    let c = Journal::open(&path, &child, "manifest".into(), "policy".into()).unwrap();
    assert_eq!(c.intent.session, p.intent.session);
    assert_ne!(c.intent.turn, p.intent.turn);
    assert_ne!(c.intent.execution, p.intent.execution);
    assert!(Journal::open(&path, &child, "manifest".into(), "other-policy".into()).is_err());
    let mut wider = child;
    wider.input.sandbox = lain_wire::Sandbox::WorkspaceWrite;
    assert!(Journal::open(&path, &wider, "manifest".into(), "policy".into()).is_err());
}
