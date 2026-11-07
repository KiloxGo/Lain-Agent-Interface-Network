use super::{
    Store,
    device_commands_tests::{event, setup, task},
};
use crate::backend::ControlError;
use lain_wire::{
    DeviceCommandId, DeviceId, RequestId, device_commands::*, device_execution::*, devices::*,
};
use lain_wire::{DeviceExecutionId, WorkSessionId, WorkerApprovalId, WorkerTurnId};

fn waiting() -> ExecutionEvidence {
    let mut e = ExecutionEvidence {
        execution_id: DeviceExecutionId::new(),
        session_id: Some(WorkSessionId::new()),
        turn_id: Some(WorkerTurnId::new()),
        phase: ExecutionPhase::WaitingForApproval,
        pending_approvals: vec![],
        approval_reports: vec![],
        result: None,
    };
    e.pending_approvals.push(DeviceApproval {
        id: WorkerApprovalId::new(),
        kind: "command_execution".into(),
        description: "Run the selected fixture check in the registered workspace".into(),
        truncated: false,
        choices: vec![
            DeviceApprovalChoice::ApproveOnce,
            DeviceApprovalChoice::Deny,
            DeviceApprovalChoice::Cancel,
        ],
        fingerprint: String::new(),
    });
    fingerprint(&mut e);
    e
}
fn fingerprint(e: &mut ExecutionEvidence) {
    for a in &mut e.pending_approvals {
        a.fingerprint = approval_fingerprint(e.execution_id, e.session_id, e.turn_id, a);
    }
}
fn decision(e: &ExecutionEvidence) -> DecideDeviceApproval {
    DecideDeviceApproval {
        request_id: RequestId::new(),
        fingerprint: e.pending_approvals[0].fingerprint.clone(),
        choice: DeviceApprovalChoice::ApproveOnce,
    }
}
fn report(
    s: &mut Store,
    r: &RegisterDevice,
    cid: DeviceCommandId,
    sequence: u64,
    kind: CommandEventKind,
    e: &ExecutionEvidence,
) -> anyhow::Result<CommandEventReceipt> {
    let mut input = event(sequence, kind);
    input.execution = Some(e.clone());
    s.device_command_event(r.id, cid, &r.credential_fingerprint, input)
}
fn begin(s: &mut Store, r: &RegisterDevice, e: &ExecutionEvidence) -> DeviceCommandId {
    let c = s.device_command_queue(r.id, task(r)).unwrap();
    let mut preparing = e.clone();
    preparing.phase = ExecutionPhase::Preparing;
    preparing.pending_approvals.clear();
    preparing.turn_id = None;
    report(s, r, c.id, 1, CommandEventKind::Accepted, &preparing).unwrap();
    report(s, r, c.id, 2, CommandEventKind::Started, e).unwrap();
    c.id
}
fn running(e: &ExecutionEvidence) -> ExecutionEvidence {
    let mut e = e.clone();
    e.phase = ExecutionPhase::Running;
    e.pending_approvals.clear();
    e
}
fn completed(e: &ExecutionEvidence) -> ExecutionEvidence {
    let mut e = running(e);
    e.phase = ExecutionPhase::Completed;
    e.result = Some(DeviceWorkResult {
        output_tail: "Worker finished its turn; see reported failures before accepting the goal"
            .into(),
        command_failures: 1,
        reported_paths: vec!["reports/result.txt".into()],
        paths_withheld: 2,
        history_truncated: true,
    });
    e
}

#[test]
fn device_permission_reports_require_exact_owner_decisions_and_never_regress() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let r = setup(&mut s, 39);
    let e = waiting();
    let cid = begin(&mut s, &r, &e);
    let aid = e.pending_approvals[0].id;
    let d = decision(&e);
    let reported = |stage: &str, decision: &DecideDeviceApproval| {
        let mut value = serde_json::to_value(running(&e)).unwrap();
        value["approval_reports"] = serde_json::json!([{
            "approval_id": aid, "decision": decision, "stage": stage
        }]);
        serde_json::from_value::<ExecutionEvidence>(value).unwrap()
    };
    let received = reported("received", &d);
    assert!(
        report(&mut s, &r, cid, 3, CommandEventKind::Progress, &received).is_err(),
        "a device cannot invent an owner decision"
    );
    s.device_approval_decide(r.id, cid, aid, d.clone()).unwrap();
    let mut wrong = d.clone();
    wrong.request_id = RequestId::new();
    assert!(
        report(
            &mut s,
            &r,
            cid,
            3,
            CommandEventKind::Progress,
            &reported("received", &wrong)
        )
        .is_err()
    );
    wrong = d.clone();
    wrong.choice = DeviceApprovalChoice::Deny;
    assert!(
        report(
            &mut s,
            &r,
            cid,
            3,
            CommandEventKind::Progress,
            &reported("applied", &wrong)
        )
        .is_err()
    );
    report(&mut s, &r, cid, 3, CommandEventKind::Progress, &received).unwrap();
    assert!(
        report(&mut s, &r, cid, 4, CommandEventKind::Progress, &running(&e)).is_err(),
        "known device receipt must not disappear"
    );
    let applied = reported("applied", &d);
    report(&mut s, &r, cid, 4, CommandEventKind::Progress, &applied).unwrap();
    assert!(
        report(&mut s, &r, cid, 5, CommandEventKind::Progress, &received).is_err(),
        "application evidence cannot regress to mere receipt"
    );
    let mut final_evidence = completed(&e);
    let mut value = serde_json::to_value(&final_evidence).unwrap();
    value["approval_reports"] = serde_json::to_value(&applied).unwrap()["approval_reports"].clone();
    final_evidence = serde_json::from_value(value).unwrap();
    report(
        &mut s,
        &r,
        cid,
        5,
        CommandEventKind::Succeeded,
        &final_evidence,
    )
    .unwrap();
    drop(s);
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    assert_eq!(
        s.device_command_get(r.id, cid, None).unwrap().execution,
        Some(final_evidence)
    );
}

#[test]
fn explicit_decision_is_immutable_and_survives_restart_without_claiming_application() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.db");
    let mut s = Store::open(&path).unwrap();
    let r = setup(&mut s, 11);
    let e = waiting();
    let cid = begin(&mut s, &r, &e);
    let a = e.pending_approvals[0].id;
    let d = decision(&e);
    let before = s.device_command_get(r.id, cid, None).unwrap();
    let receipt = s.device_approval_decide(r.id, cid, a, d.clone()).unwrap();
    let after = s.device_command_get(r.id, cid, None).unwrap();
    assert_eq!(after.revision, before.revision + 1);
    assert_eq!(after.state, CommandState::Running);
    assert_eq!(
        after.execution.as_ref().unwrap().pending_approvals,
        e.pending_approvals
    );
    assert_eq!(
        s.device_approval_decide(r.id, cid, a, d.clone()).unwrap(),
        receipt
    );
    assert_eq!(s.device_command_get(r.id, cid, None).unwrap(), after);
    let mut changed = d.clone();
    changed.choice = DeviceApprovalChoice::Deny;
    assert!(s.device_approval_decide(r.id, cid, a, changed).is_err());
    let mut other_id = d.clone();
    other_id.request_id = RequestId::new();
    assert!(s.device_approval_decide(r.id, cid, a, other_id).is_err());
    report(
        &mut s,
        &r,
        cid,
        3,
        CommandEventKind::Succeeded,
        &completed(&e),
    )
    .unwrap();
    drop(s);
    let mut s = Store::open(&path).unwrap();
    assert_eq!(s.device_approval_decide(r.id, cid, a, d).unwrap(), receipt);
    assert_eq!(
        s.device_approval_decisions(r.id, cid, Some(&r.credential_fingerprint))
            .unwrap(),
        vec![receipt]
    );
    assert_eq!(
        s.device_command_get(r.id, cid, None).unwrap().state,
        CommandState::Succeeded
    );
}

#[test]
fn stale_or_crossed_scopes_never_produce_a_decision_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let r = setup(&mut s, 12);
    let e = waiting();
    let cid = begin(&mut s, &r, &e);
    let aid = e.pending_approvals[0].id;
    let mut wrong = decision(&e);
    wrong.fingerprint = "a".repeat(64);
    assert!(s.device_approval_decide(r.id, cid, aid, wrong).is_err());
    assert!(
        s.device_approval_decide(r.id, cid, WorkerApprovalId::new(), decision(&e))
            .is_err()
    );
    let other = waiting();
    let other_cid = begin(&mut s, &r, &other);
    assert!(
        s.device_approval_decide(r.id, other_cid, aid, decision(&e))
            .is_err()
    );
    report(&mut s, &r, cid, 3, CommandEventKind::Progress, &running(&e)).unwrap();
    assert!(
        s.device_approval_decide(r.id, cid, aid, decision(&e))
            .is_err()
    );
    assert!(
        s.device_approval_decisions(r.id, cid, None)
            .unwrap()
            .is_empty()
    );
    assert!(
        s.device_approval_decisions(DeviceId::new(), cid, Some(&r.credential_fingerprint))
            .is_err()
    );
    assert!(
        s.device_approval_decisions(r.id, cid, Some(&"b".repeat(64)))
            .is_err()
    );
}

#[test]
fn cancellation_expiry_revocation_and_unknown_block_new_decisions() {
    for mode in ["cancel", "expire", "revoke", "unknown"] {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(&dir.path().join("state.db")).unwrap();
        let r = setup(&mut s, 13);
        let e = waiting();
        let cid = begin(&mut s, &r, &e);
        match mode {
            "cancel" => {
                let c = s.device_command_get(r.id, cid, None).unwrap();
                s.device_command_cancel(
                    r.id,
                    cid,
                    CancelDeviceCommand {
                        expected_revision: c.revision,
                    },
                )
                .unwrap();
            }
            "expire" => {
                let mut c = s.device_command_get(r.id, cid, None).unwrap();
                c.input.expires_at_ms = 1;
                s.connection
                    .execute(
                        "UPDATE device_commands SET snapshot=?2 WHERE id=?1",
                        (cid.to_string(), serde_json::to_string(&c).unwrap()),
                    )
                    .unwrap();
            }
            "revoke" => {
                s.device_revoke(
                    r.id,
                    RevokeDevice {
                        expected_revision: 2,
                    },
                )
                .unwrap();
            }
            _ => {
                let mut u = running(&e);
                u.phase = ExecutionPhase::Unknown;
                report(&mut s, &r, cid, 3, CommandEventKind::Unknown, &u).unwrap();
            }
        }
        assert!(
            s.device_approval_decide(r.id, cid, e.pending_approvals[0].id, decision(&e))
                .is_err(),
            "{mode}"
        );
        assert!(
            s.device_approval_decisions(r.id, cid, None)
                .unwrap()
                .is_empty()
        );
        let c = s.device_command_get(r.id, cid, None).unwrap();
        assert!(
            c.cancel_requested || c.state == CommandState::Unknown,
            "{mode}"
        );
    }
}

#[test]
fn execution_identity_cannot_be_removed_rebound_or_reused_by_another_command() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let r = setup(&mut s, 14);
    let e = waiting();
    let cid = begin(&mut s, &r, &e);
    assert!(
        s.device_command_event(
            r.id,
            cid,
            &r.credential_fingerprint,
            event(3, CommandEventKind::Progress)
        )
        .is_err()
    );
    for field in ["execution", "session", "turn"] {
        let mut bad = running(&e);
        match field {
            "execution" => bad.execution_id = DeviceExecutionId::new(),
            "session" => bad.session_id = Some(WorkSessionId::new()),
            _ => bad.turn_id = Some(WorkerTurnId::new()),
        }
        assert!(report(&mut s, &r, cid, 3, CommandEventKind::Progress, &bad).is_err());
        let mut bad = waiting();
        match field {
            "execution" => bad.execution_id = e.execution_id,
            "session" => bad.session_id = e.session_id,
            _ => bad.turn_id = e.turn_id,
        };
        fingerprint(&mut bad);
        let c = s.device_command_queue(r.id, task(&r)).unwrap();
        s.device_command_event(
            r.id,
            c.id,
            &r.credential_fingerprint,
            event(1, CommandEventKind::Accepted),
        )
        .unwrap();
        assert!(report(&mut s, &r, c.id, 2, CommandEventKind::Started, &bad).is_err());
    }
    report(
        &mut s,
        &r,
        cid,
        3,
        CommandEventKind::Succeeded,
        &completed(&e),
    )
    .unwrap();
    assert_eq!(
        s.device_command_get(r.id, cid, None)
            .unwrap()
            .execution
            .unwrap()
            .result
            .unwrap()
            .command_failures,
        1
    );
}

#[test]
fn hidden_or_changed_permission_scope_cannot_be_approved() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let r = setup(&mut s, 15);
    let mut e = waiting();
    e.pending_approvals[0].truncated = true;
    fingerprint(&mut e);
    assert!(!e.valid());
    e.pending_approvals[0].choices = vec![DeviceApprovalChoice::Deny, DeviceApprovalChoice::Cancel];
    fingerprint(&mut e);
    let cid = begin(&mut s, &r, &e);
    let aid = e.pending_approvals[0].id;
    assert!(
        s.device_approval_decide(r.id, cid, aid, decision(&e))
            .is_err()
    );
    let mut deny = decision(&e);
    deny.choice = DeviceApprovalChoice::Deny;
    s.device_approval_decide(r.id, cid, aid, deny).unwrap();
    report(&mut s, &r, cid, 3, CommandEventKind::Progress, &running(&e)).unwrap();
    let mut changed = e.clone();
    changed.pending_approvals[0].description =
        "a different permission with the old identity".into();
    fingerprint(&mut changed);
    assert!(report(&mut s, &r, cid, 4, CommandEventKind::Progress, &changed).is_err());
    let mut second = e.clone();
    second.pending_approvals[0].id = WorkerApprovalId::new();
    fingerprint(&mut second);
    report(&mut s, &r, cid, 4, CommandEventKind::Progress, &second).unwrap();
    let old = s
        .device_approval_decisions(r.id, cid, None)
        .unwrap()
        .remove(0);
    let mut reused = old.decision;
    reused.fingerprint = second.pending_approvals[0].fingerprint.clone();
    assert!(
        s.device_approval_decide(r.id, cid, second.pending_approvals[0].id, reused)
            .is_err()
    );
}

#[test]
fn invalid_reports_are_atomic_and_secrets_cannot_enter_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let r = setup(&mut s, 16);
    let e = waiting();
    let cid = begin(&mut s, &r, &e);
    let before = s.device_command_get(r.id, cid, None).unwrap();
    let mut invalid = completed(&e);
    invalid.result.as_mut().unwrap().reported_paths = vec!["../outside.txt".into()];
    assert!(report(&mut s, &r, cid, 3, CommandEventKind::Succeeded, &invalid).is_err());
    let secret = format!("lain-device-{}", "de".repeat(32));
    for field in ["description", "output", "path"] {
        let mut invalid = if field == "description" {
            e.clone()
        } else {
            completed(&e)
        };
        match field {
            "description" => {
                invalid.pending_approvals[0].description = secret.clone();
                fingerprint(&mut invalid);
            }
            "output" => invalid.result.as_mut().unwrap().output_tail = secret.clone(),
            _ => invalid.result.as_mut().unwrap().reported_paths = vec![secret.clone()],
        }
        let kind = if field == "description" {
            CommandEventKind::Progress
        } else {
            CommandEventKind::Succeeded
        };
        assert!(report(&mut s, &r, cid, 3, kind, &invalid).is_err());
    }
    assert_eq!(s.device_command_get(r.id, cid, None).unwrap(), before);
    assert_eq!(
        s.connection
            .query_row(
                "SELECT count(*) FROM device_command_events WHERE command_id=?1",
                [cid.to_string()],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        2
    );
    report(
        &mut s,
        &r,
        cid,
        3,
        CommandEventKind::Succeeded,
        &completed(&e),
    )
    .unwrap();
}

#[test]
fn schema25_upgrade_keeps_legacy_event_receipts_and_accepts_new_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.db");
    let mut s = Store::open(&path).unwrap();
    let r = setup(&mut s, 17);
    let c = s.device_command_queue(r.id, task(&r)).unwrap();
    let input = event(1, CommandEventKind::Accepted);
    let receipt = s
        .device_command_event(r.id, c.id, &r.credential_fingerprint, input.clone())
        .unwrap();
    s.connection.execute_batch("DROP TABLE actor_memory_contexts; DROP TABLE memory_context_requests; DROP TABLE conversation_memory_contexts; DROP TABLE project_requests; DROP TABLE project_workspaces; DROP TABLE projects; DROP TABLE device_work_continuations; DROP TABLE device_work_links; DROP TABLE device_approval_decisions; DROP TABLE device_executions; DROP TABLE briefing_relations; DROP TABLE conversation_memory_effects; DROP TABLE briefing_memory_suppressions; DROP TABLE briefing_disclosures; DROP TABLE briefing_runs; DROP TABLE briefing_batches; DROP TABLE briefing_manifests; DROP TABLE briefing_grants; PRAGMA user_version=25;").unwrap();
    drop(s);
    let mut s = Store::open(&path).unwrap();
    assert_eq!(
        s.connection
            .query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))
            .unwrap(),
        super::SCHEMA_VERSION
    );
    assert_eq!(
        s.device_command_event(r.id, c.id, &r.credential_fingerprint, input)
            .unwrap(),
        receipt
    );
    let e = waiting();
    report(&mut s, &r, c.id, 2, CommandEventKind::Started, &e).unwrap();
    s.device_approval_decide(r.id, c.id, e.pending_approvals[0].id, decision(&e))
        .unwrap();
    assert!(
        !s.device_approval_decisions(r.id, c.id, None)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn late_observations_can_reconcile_without_reopening_permission() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let r = setup(&mut s, 18);
    let e = waiting();
    let cid = begin(&mut s, &r, &e);
    let mut c = s.device_command_get(r.id, cid, None).unwrap();
    c.input.expires_at_ms = 1;
    s.connection
        .execute(
            "UPDATE device_commands SET snapshot=?2 WHERE id=?1",
            (cid.to_string(), serde_json::to_string(&c).unwrap()),
        )
        .unwrap();
    report(&mut s, &r, cid, 3, CommandEventKind::Progress, &e).unwrap();
    assert!(
        s.device_approval_decide(r.id, cid, e.pending_approvals[0].id, decision(&e))
            .unwrap_err()
            .is::<ControlError>()
    );
    assert_eq!(
        report(
            &mut s,
            &r,
            cid,
            4,
            CommandEventKind::Succeeded,
            &completed(&e)
        )
        .unwrap()
        .state,
        CommandState::Succeeded
    );
}

#[test]
fn failed_decision_publication_rolls_back_receipt_and_revision() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let r = setup(&mut s, 19);
    let e = waiting();
    let cid = begin(&mut s, &r, &e);
    let before = s.device_command_get(r.id, cid, None).unwrap();
    s.connection.execute_batch("CREATE TRIGGER fail_decision_publish BEFORE INSERT ON device_command_changes BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    let d = decision(&e);
    let aid = e.pending_approvals[0].id;
    assert!(s.device_approval_decide(r.id, cid, aid, d.clone()).is_err());
    assert!(
        s.device_approval_decisions(r.id, cid, None)
            .unwrap()
            .is_empty()
    );
    assert_eq!(s.device_command_get(r.id, cid, None).unwrap(), before);
    s.connection
        .execute_batch("DROP TRIGGER fail_decision_publish;")
        .unwrap();
    s.device_approval_decide(r.id, cid, aid, d).unwrap();
    assert_eq!(
        s.device_command_get(r.id, cid, None).unwrap().revision,
        before.revision + 1
    );
}
