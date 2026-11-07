use super::*;
use lain_wire::{DeviceId, DeviceWorkspaceId, RequestId, device_commands::*, devices::*};

#[test]
fn progress_cannot_exhaust_the_terminal_and_uncertainty_resolution_slots() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let r = setup(&mut s, 9);
    let c = s.device_command_queue(r.id, task(&r)).unwrap();
    s.device_command_event(
        r.id,
        c.id,
        &r.credential_fingerprint,
        event(1, CommandEventKind::Accepted),
    )
    .unwrap();
    s.device_command_event(
        r.id,
        c.id,
        &r.credential_fingerprint,
        event(2, CommandEventKind::Started),
    )
    .unwrap();
    for sequence in 3..256 {
        s.device_command_event(
            r.id,
            c.id,
            &r.credential_fingerprint,
            event(sequence, CommandEventKind::Progress),
        )
        .unwrap();
    }
    assert!(
        s.device_command_event(
            r.id,
            c.id,
            &r.credential_fingerprint,
            event(256, CommandEventKind::Progress)
        )
        .is_err()
    );
    s.device_command_event(
        r.id,
        c.id,
        &r.credential_fingerprint,
        event(256, CommandEventKind::Unknown),
    )
    .unwrap();
    assert!(
        s.device_command_event(
            r.id,
            c.id,
            &r.credential_fingerprint,
            event(257, CommandEventKind::Progress)
        )
        .is_err()
    );
    assert_eq!(
        s.device_command_event(
            r.id,
            c.id,
            &r.credential_fingerprint,
            event(257, CommandEventKind::Succeeded)
        )
        .unwrap()
        .state,
        CommandState::Succeeded
    );
}

#[test]
fn deadlines_do_not_claim_execution_stopped_and_late_final_facts_can_reconcile() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let r = setup(&mut s, 6);
    let a = s.device_command_queue(r.id, task(&r)).unwrap();
    let b = s.device_command_queue(r.id, task(&r)).unwrap();
    s.device_command_event(
        r.id,
        b.id,
        &r.credential_fingerprint,
        event(1, CommandEventKind::Accepted),
    )
    .unwrap();
    s.device_command_event(
        r.id,
        b.id,
        &r.credential_fingerprint,
        event(2, CommandEventKind::Started),
    )
    .unwrap();
    let tx = s.connection.transaction().unwrap();
    super::device_commands::maintain(
        &tx,
        r.id,
        a.input.expires_at_ms.max(b.input.expires_at_ms) + 1,
    )
    .unwrap();
    tx.commit().unwrap();
    assert_eq!(
        s.device_command_get(r.id, a.id, None).unwrap().state,
        CommandState::Expired
    );
    assert_eq!(
        s.device_command_get(r.id, b.id, None).unwrap().state,
        CommandState::Unknown
    );
    assert_eq!(
        s.device_command_event(
            r.id,
            b.id,
            &r.credential_fingerprint,
            event(3, CommandEventKind::Started)
        )
        .unwrap()
        .state,
        CommandState::Unknown
    );
    assert_eq!(
        s.device_command_event(
            r.id,
            b.id,
            &r.credential_fingerprint,
            event(4, CommandEventKind::Succeeded)
        )
        .unwrap()
        .state,
        CommandState::Succeeded
    );
}

#[test]
fn command_scope_secret_guards_capacity_and_device_revoke_are_atomic() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let r = setup(&mut s, 7);
    for bad in 0..6 {
        let mut input = task(&r);
        match bad {
            0 => input.workspace_id = DeviceWorkspaceId::new(),
            1 => input.task = "sk-this-is-a-secret-shaped-credential".into(),
            2 => input.expires_at_ms = r.expires_at_ms + 1,
            3 => input.expected_device_revision = 1,
            4 => input.profile_id = "../codex".into(),
            _ => input.task = "\u{1b}[2J".into(),
        };
        assert!(s.device_command_queue(r.id, input).is_err());
    }
    let mut original = task(&r);
    original.task = "first line\r\nsecond line\n\tindented".into();
    let c = s.device_command_queue(r.id, original.clone()).unwrap();
    assert_eq!(c.input.task, original.task);
    let mut bad = event(1, CommandEventKind::Accepted);
    bad.detail = "Bearer fixture-credential".into();
    assert!(
        s.device_command_event(r.id, c.id, &r.credential_fingerprint, bad)
            .is_err()
    );
    s.connection.execute_batch("CREATE TRIGGER revoke_rollback BEFORE INSERT ON device_command_changes BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    assert!(
        s.device_revoke(
            r.id,
            RevokeDevice {
                expected_revision: 2
            }
        )
        .is_err()
    );
    assert_eq!(
        s.device_snapshot(r.id).unwrap().device.state,
        DeviceState::Active
    );
    assert_eq!(
        s.device_command_get(r.id, c.id, None).unwrap().state,
        CommandState::Queued
    );
    s.connection
        .execute_batch("DROP TRIGGER revoke_rollback")
        .unwrap();
    for _ in 1..128 {
        s.device_command_queue(r.id, task(&r)).unwrap();
    }
    assert!(s.device_command_queue(r.id, task(&r)).is_err());
    s.device_revoke(
        r.id,
        RevokeDevice {
            expected_revision: 2,
        },
    )
    .unwrap();
    assert_eq!(
        s.device_command_get(r.id, c.id, None).unwrap().state,
        CommandState::Revoked
    );
}

#[test]
fn schema24_upgrade_preserves_device_conversation_and_grant_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.db");
    let mut s = Store::open(&path).unwrap();
    let r = setup(&mut s, 8);
    s.create_conversation().unwrap();
    let before: String = s
        .connection
        .query_row("SELECT snapshot FROM devices", [], |r| r.get(0))
        .unwrap();
    s.connection.execute_batch("DROP TABLE actor_memory_contexts; DROP TABLE memory_context_requests; DROP TABLE conversation_memory_contexts; DROP TABLE project_requests; DROP TABLE project_workspaces; DROP TABLE projects; DROP TABLE device_work_continuations; DROP TABLE device_work_links; DROP TABLE device_approval_decisions; DROP TABLE device_executions; DROP TABLE device_command_changes;DROP TABLE device_command_events;DROP TABLE device_commands;DROP TABLE briefing_relations; DROP TABLE conversation_memory_effects; DROP TABLE briefing_memory_suppressions; DROP TABLE briefing_disclosures; DROP TABLE briefing_runs; DROP TABLE briefing_batches; DROP TABLE briefing_manifests; DROP TABLE briefing_grants; PRAGMA user_version=24;").unwrap();
    drop(s);
    let mut s = Store::open(&path).unwrap();
    assert_eq!(
        s.connection
            .query_row("SELECT snapshot FROM devices", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        before
    );
    assert_eq!(s.conversations().unwrap().len(), 1);
    assert!(
        s.device_command_changes(r.id, None, Default::default())
            .unwrap()
            .changes
            .is_empty()
    );
    assert_eq!(
        s.connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        super::SCHEMA_VERSION
    );
}
pub(super) fn setup(s: &mut Store, n: u8) -> RegisterDevice {
    let r = RegisterDevice {
        id: DeviceId::new(),
        request_id: RequestId::new(),
        label: format!("device-{n}"),
        platform: DevicePlatform::Linux,
        credential_fingerprint: device_credential_fingerprint(&format!(
            "lain-device-{}",
            format!("{n:02x}").repeat(32)
        ))
        .unwrap(),
        workspaces: vec![DeviceWorkspace {
            id: DeviceWorkspaceId::new(),
            path: "/device-only/not-on-server".into(),
        }],
        capabilities: vec![],
        expires_at_ms: now_ms().unwrap() + 600000,
    };
    s.device_register(r.clone()).unwrap();
    s.device_claim(
        r.id,
        &r.credential_fingerprint,
        ClaimDevice {
            request_id: RequestId::new(),
            registration_fingerprint: registration_fingerprint(&r),
        },
    )
    .unwrap();
    r
}
pub(super) fn task(r: &RegisterDevice) -> QueueDeviceWork {
    QueueDeviceWork {
        request_id: RequestId::new(),
        expected_device_revision: 2,
        workspace_id: r.workspaces[0].id,
        profile_id: "codex/local".into(),
        sandbox: lain_wire::Sandbox::ReadOnly,
        task: "Inspect the selected workspace".into(),
        expires_at_ms: now_ms().unwrap() + 120000,
    }
}
pub(super) fn event(sequence: u64, kind: CommandEventKind) -> ReportCommandEvent {
    ReportCommandEvent {
        request_id: RequestId::new(),
        sequence,
        kind,
        detail: "device-reported fixture observation".into(),
        execution: None,
    }
}

#[test]
fn command_identity_order_and_restart_do_not_requeue_accepted_work() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.db");
    let mut s = Store::open(&path).unwrap();
    let r = setup(&mut s, 1);
    let input = task(&r);
    let c = s.device_command_queue(r.id, input.clone()).unwrap();
    assert_eq!(s.device_command_queue(r.id, input.clone()).unwrap(), c);
    let mut changed = input;
    changed.task = "changed intent".into();
    assert!(s.device_command_queue(r.id, changed).is_err());
    assert!(
        s.device_command_event(
            r.id,
            c.id,
            &r.credential_fingerprint,
            event(2, CommandEventKind::Started)
        )
        .is_err()
    );
    let accepted = event(1, CommandEventKind::Accepted);
    let receipt = s
        .device_command_event(r.id, c.id, &r.credential_fingerprint, accepted.clone())
        .unwrap();
    assert_eq!(
        s.device_command_event(r.id, c.id, &r.credential_fingerprint, accepted.clone())
            .unwrap(),
        receipt
    );
    drop(s);
    let mut s = Store::open(&path).unwrap();
    assert_eq!(
        s.device_command_get(r.id, c.id, Some(&r.credential_fingerprint))
            .unwrap()
            .state,
        CommandState::Accepted
    );
    assert_eq!(
        s.device_command_receipt(
            r.id,
            c.id,
            accepted.request_id,
            Some(&r.credential_fingerprint)
        )
        .unwrap(),
        receipt
    );
    let mut changed = accepted;
    changed.detail = "changed receipt".into();
    assert!(
        s.device_command_event(r.id, c.id, &r.credential_fingerprint, changed)
            .is_err()
    );
    for (seq, kind) in [
        (2, CommandEventKind::Started),
        (3, CommandEventKind::Progress),
        (4, CommandEventKind::Succeeded),
    ] {
        s.device_command_event(r.id, c.id, &r.credential_fingerprint, event(seq, kind))
            .unwrap();
    }
    let result = s.device_command_get(r.id, c.id, None).unwrap();
    assert_eq!(result.state, CommandState::Succeeded);
    assert!(
        s.device_command_event(
            r.id,
            c.id,
            &r.credential_fingerprint,
            event(5, CommandEventKind::Started)
        )
        .is_err()
    );
    assert_eq!(
        s.connection
            .query_row("SELECT count(*) FROM work_sessions", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
#[test]
fn cancel_requires_device_stop_evidence_and_revocation_retains_uncertainty() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let r = setup(&mut s, 2);
    let queued = s.device_command_queue(r.id, task(&r)).unwrap();
    let cancelled = s
        .device_command_cancel(
            r.id,
            queued.id,
            CancelDeviceCommand {
                expected_revision: 1,
            },
        )
        .unwrap();
    assert_eq!(cancelled.state, CommandState::Cancelled);
    assert!(
        s.device_command_event(
            r.id,
            queued.id,
            &r.credential_fingerprint,
            event(1, CommandEventKind::Accepted)
        )
        .is_err()
    );
    let c = s.device_command_queue(r.id, task(&r)).unwrap();
    s.device_command_event(
        r.id,
        c.id,
        &r.credential_fingerprint,
        event(1, CommandEventKind::Accepted),
    )
    .unwrap();
    let requested = s
        .device_command_cancel(
            r.id,
            c.id,
            CancelDeviceCommand {
                expected_revision: 2,
            },
        )
        .unwrap();
    assert_eq!(requested.state, CommandState::Accepted);
    assert!(requested.cancel_requested);
    assert_eq!(
        s.device_command_event(
            r.id,
            c.id,
            &r.credential_fingerprint,
            event(2, CommandEventKind::Started)
        )
        .unwrap()
        .state,
        CommandState::Running
    );
    assert_eq!(
        s.device_command_event(
            r.id,
            c.id,
            &r.credential_fingerprint,
            event(3, CommandEventKind::Cancelled)
        )
        .unwrap()
        .state,
        CommandState::Cancelled
    );
    let a = s.device_command_queue(r.id, task(&r)).unwrap();
    let b = s.device_command_queue(r.id, task(&r)).unwrap();
    let old = event(1, CommandEventKind::Accepted);
    s.device_command_event(r.id, a.id, &r.credential_fingerprint, old.clone())
        .unwrap();
    s.device_revoke(
        r.id,
        RevokeDevice {
            expected_revision: 2,
        },
    )
    .unwrap();
    assert_eq!(
        s.device_command_get(r.id, a.id, None).unwrap().state,
        CommandState::Unknown
    );
    assert_eq!(
        s.device_command_get(r.id, b.id, None).unwrap().state,
        CommandState::Revoked
    );
    assert!(
        s.device_command_event(r.id, a.id, &r.credential_fingerprint, old)
            .is_err()
    );
    assert!(
        s.device_command_get(r.id, a.id, Some(&r.credential_fingerprint))
            .is_err()
    );
}
#[test]
fn durable_change_pages_keep_fixed_high_water_and_isolate_devices() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let r = setup(&mut s, 3);
    let other = setup(&mut s, 4);
    let a = s.device_command_queue(r.id, task(&r)).unwrap();
    s.device_command_event(
        r.id,
        a.id,
        &r.credential_fingerprint,
        event(1, CommandEventKind::Accepted),
    )
    .unwrap();
    let first = s
        .device_command_changes(
            r.id,
            Some(&r.credential_fingerprint),
            CommandChangesQuery {
                limit: 1,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(first.has_more);
    assert_eq!(first.through, 2);
    s.device_command_cancel(
        r.id,
        a.id,
        CancelDeviceCommand {
            expected_revision: 2,
        },
    )
    .unwrap();
    let second = s
        .device_command_changes(
            r.id,
            None,
            CommandChangesQuery {
                after: first.next_after,
                through: Some(first.through),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(second.changes.len(), 1);
    assert!(!second.changes[0].cancel_requested);
    assert!(!second.has_more);
    let next = s
        .device_command_changes(
            r.id,
            None,
            CommandChangesQuery {
                after: second.next_after,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(next.changes[0].cancel_requested);
    assert!(
        s.device_command_get(other.id, a.id, Some(&other.credential_fingerprint))
            .is_err()
    );
    assert!(
        s.device_command_changes(
            other.id,
            Some(&r.credential_fingerprint),
            Default::default()
        )
        .is_err()
    );
    assert!(
        s.device_command_changes(
            r.id,
            None,
            CommandChangesQuery {
                after: 999,
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn failed_event_publication_rolls_back_state_receipt_and_cursor() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let r = setup(&mut s, 5);
    let c = s.device_command_queue(r.id, task(&r)).unwrap();
    s.connection.execute_batch("CREATE TRIGGER fail_command_change BEFORE INSERT ON device_command_changes BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    let e = event(1, CommandEventKind::Accepted);
    assert!(
        s.device_command_event(r.id, c.id, &r.credential_fingerprint, e.clone())
            .is_err()
    );
    assert_eq!(s.device_command_get(r.id, c.id, None).unwrap(), c);
    assert!(
        s.device_command_receipt(r.id, c.id, e.request_id, None)
            .is_err()
    );
    s.connection
        .execute_batch("DROP TRIGGER fail_command_change")
        .unwrap();
    s.device_command_event(r.id, c.id, &r.credential_fingerprint, e)
        .unwrap();
    assert_eq!(
        s.device_command_changes(r.id, None, Default::default())
            .unwrap()
            .through,
        2
    );
}
