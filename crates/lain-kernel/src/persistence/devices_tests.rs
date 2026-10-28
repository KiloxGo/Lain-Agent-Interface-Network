use super::*;
use lain_wire::{DeviceId, DeviceWorkspaceId, RequestId, devices::*};

fn registration(n: u8) -> (RegisterDevice, String) {
    let token = format!("lain-device-{}", format!("{n:02x}").repeat(32));
    (
        RegisterDevice {
            id: DeviceId::new(),
            request_id: RequestId::new(),
            label: format!("fixture-{n}"),
            platform: DevicePlatform::Windows,
            credential_fingerprint: device_credential_fingerprint(&token).unwrap(),
            workspaces: vec![DeviceWorkspace {
                id: DeviceWorkspaceId::new(),
                path: "C:\\fixture\\workspace".into(),
            }],
            capabilities: vec![DeviceCapability::CodexVisibleArchive],
            expires_at_ms: now_ms().unwrap() + 600000,
        },
        token,
    )
}
fn report(r: &RegisterDevice, sequence: u64) -> ReportDevice {
    ReportDevice {
        request_id: RequestId::new(),
        sequence,
        reported_at_ms: 100,
        available_workspaces: vec![r.workspaces[0].id],
        capabilities: vec![DeviceCapabilityReport {
            capability: DeviceCapability::CodexVisibleArchive,
            version: "fixture-only-not-runtime-proof".into(),
        }],
    }
}
#[test]
fn explicit_registration_claim_and_verifier_isolation_are_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    assert!(s.devices().unwrap().is_empty());
    let (r, token) = registration(1);
    let g = s.device_register(r.clone()).unwrap();
    assert_eq!(g.device.state, DeviceState::Pending);
    assert_eq!(g.device.revision, 1);
    assert_eq!(s.device_register(r.clone()).unwrap(), g);
    let mut changed = r.clone();
    changed.label = "changed under same request".into();
    assert!(s.device_register(changed).is_err());
    let mut reused = r.clone();
    reused.id = DeviceId::new();
    reused.request_id = RequestId::new();
    assert!(s.device_register(reused).is_err());
    assert!(
        s.device_own_snapshot(r.id, &r.credential_fingerprint)
            .is_err()
    );
    let claim = ClaimDevice {
        request_id: RequestId::new(),
        registration_fingerprint: registration_fingerprint(&r),
    };
    let mut changed_claim = claim.clone();
    changed_claim.registration_fingerprint = "0".repeat(64);
    assert!(
        s.device_claim(r.id, &r.credential_fingerprint, changed_claim)
            .is_err()
    );
    assert_eq!(
        s.device_snapshot(r.id).unwrap().device.state,
        DeviceState::Pending
    );
    assert!(
        s.device_claim(r.id, &"0".repeat(64), claim.clone())
            .is_err()
    );
    let claimed = s
        .device_claim(r.id, &r.credential_fingerprint, claim.clone())
        .unwrap();
    assert_eq!(claimed.device.state, DeviceState::Active);
    assert_eq!(claimed.device.revision, 2);
    assert_eq!(
        s.device_claim(r.id, &r.credential_fingerprint, claim)
            .unwrap(),
        claimed
    );
    assert!(
        s.device_claim(
            r.id,
            &r.credential_fingerprint,
            ClaimDevice {
                request_id: RequestId::new(),
                registration_fingerprint: registration_fingerprint(&r)
            }
        )
        .is_err()
    );
    let raw: String = s
        .connection
        .query_row("SELECT snapshot FROM devices", [], |r| r.get(0))
        .unwrap();
    assert!(!raw.contains(&token));
    assert!(raw.contains(&r.credential_fingerprint));
    assert!(
        s.device_own_snapshot(DeviceId::new(), &r.credential_fingerprint)
            .is_err()
    );
}
#[test]
fn monotonic_reports_survive_restart_and_revocation_blocks_even_old_receipt_replays() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let (r, _) = registration(2);
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
    let input = report(&r, 1);
    let receipt = s
        .device_report(r.id, &r.credential_fingerprint, input.clone())
        .unwrap();
    assert_eq!(
        s.device_report(r.id, &r.credential_fingerprint, input.clone())
            .unwrap(),
        receipt
    );
    let mut changed = input.clone();
    changed.reported_at_ms += 1;
    assert!(
        s.device_report(r.id, &r.credential_fingerprint, changed)
            .is_err()
    );
    assert!(
        s.device_report(r.id, &r.credential_fingerprint, report(&r, 1))
            .is_err()
    );
    assert!(
        s.device_report(r.id, &r.credential_fingerprint, report(&r, 3))
            .is_err()
    );
    drop(s);
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    assert_eq!(
        s.device_snapshot(r.id).unwrap().latest_report,
        Some(receipt)
    );
    s.device_report(r.id, &r.credential_fingerprint, report(&r, 2))
        .unwrap();
    assert_eq!(s.device_snapshot(r.id).unwrap().device.revision, 2);
    let revoked = s
        .device_revoke(
            r.id,
            RevokeDevice {
                expected_revision: 2,
            },
        )
        .unwrap();
    assert_eq!(revoked.device.revision, 3);
    assert_eq!(revoked.device.state, DeviceState::Revoked);
    assert!(
        s.device_report(r.id, &r.credential_fingerprint, input)
            .is_err()
    );
    assert!(
        s.device_report(r.id, &r.credential_fingerprint, report(&r, 3))
            .is_err()
    );
    assert!(
        s.device_own_snapshot(r.id, &r.credential_fingerprint)
            .is_err()
    );
    assert_eq!(
        s.device_register(r).unwrap().device.state,
        DeviceState::Revoked
    );
}
#[test]
fn unauthorized_inventory_and_atomic_failure_cannot_advance_a_device() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let (r, _) = registration(3);
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
    for case in 0..4 {
        let mut input = report(&r, 1);
        match case {
            0 => input.available_workspaces.push(DeviceWorkspaceId::new()),
            1 => input.available_workspaces.push(r.workspaces[0].id),
            2 => input.capabilities[0].capability = DeviceCapability::CodexManagedWork,
            _ => input.capabilities[0].version = format!("lain-device-{}", "ab".repeat(32)),
        }
        assert!(
            s.device_report(r.id, &r.credential_fingerprint, input)
                .is_err()
        );
    }
    s.connection.execute_batch("CREATE TRIGGER reject_report BEFORE INSERT ON device_reports BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    let input = report(&r, 1);
    assert!(
        s.device_report(r.id, &r.credential_fingerprint, input.clone())
            .is_err()
    );
    assert!(s.device_snapshot(r.id).unwrap().latest_report.is_none());
    s.connection
        .execute_batch("DROP TRIGGER reject_report")
        .unwrap();
    assert_eq!(
        s.device_report(r.id, &r.credential_fingerprint, input)
            .unwrap()
            .report
            .sequence,
        1
    );
}
#[test]
fn pending_expiry_active_expiry_and_wrong_device_key_are_not_report_authority() {
    for active in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(&dir.path().join("state.db")).unwrap();
        let (r, _) = registration(4);
        s.device_register(r.clone()).unwrap();
        if active {
            s.device_claim(
                r.id,
                &r.credential_fingerprint,
                ClaimDevice {
                    request_id: RequestId::new(),
                    registration_fingerprint: registration_fingerprint(&r),
                },
            )
            .unwrap();
        }
        let field = if active {
            "$.registration.expires_at_ms"
        } else {
            "$.claim_until_ms"
        };
        s.connection
            .execute(
                "UPDATE devices SET snapshot=json_set(snapshot,?2,0) WHERE id=?1",
                (r.id.to_string(), field),
            )
            .unwrap();
        assert!(s.device_snapshot(r.id).unwrap().expired);
        assert!(
            s.device_claim(
                r.id,
                &r.credential_fingerprint,
                ClaimDevice {
                    request_id: RequestId::new(),
                    registration_fingerprint: registration_fingerprint(&r)
                }
            )
            .is_err()
        );
        assert!(
            s.device_report(r.id, &r.credential_fingerprint, report(&r, 1))
                .is_err()
        );
        assert_eq!(
            s.device_revoke(
                r.id,
                RevokeDevice {
                    expected_revision: if active { 2 } else { 1 }
                }
            )
            .unwrap()
            .device
            .state,
            DeviceState::Revoked
        );
    }
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let (a, _) = registration(5);
    let (b, _) = registration(6);
    for r in [&a, &b] {
        s.device_register(r.clone()).unwrap();
        s.device_claim(
            r.id,
            &r.credential_fingerprint,
            ClaimDevice {
                request_id: RequestId::new(),
                registration_fingerprint: registration_fingerprint(r),
            },
        )
        .unwrap();
    }
    assert!(
        s.device_report(a.id, &b.credential_fingerprint, report(&a, 1))
            .is_err()
    );
    assert!(
        s.device_own_snapshot(b.id, &a.credential_fingerprint)
            .is_err()
    );
}
#[test]
fn invalid_registration_and_full_ledgers_do_not_prevent_revoke() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let (r, _) = registration(7);
    for case in 0..5 {
        let mut bad = r.clone();
        match case {
            0 => bad.expires_at_ms = 0,
            1 => bad.expires_at_ms = now_ms().unwrap() + 32 * 86400000,
            2 => bad.workspaces[0].path = "relative".into(),
            3 => bad.workspaces.push(bad.workspaces[0].clone()),
            _ => bad.credential_fingerprint = "not a verifier".into(),
        }
        assert!(s.device_register(bad).is_err());
    }
    assert!(s.devices().unwrap().is_empty());
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
    let tx = s.connection.transaction().unwrap();
    for sequence in 1..=4096 {
        let input = report(&r, sequence);
        let receipt = DeviceReport {
            device_id: r.id,
            device_revision: 2,
            received_at_ms: 100,
            report: input,
        };
        tx.execute(
            "INSERT INTO device_reports VALUES(?1,?2,?3,?4)",
            (
                r.id.to_string(),
                sequence as i64,
                receipt.report.request_id.to_string(),
                serde_json::to_string(&receipt).unwrap(),
            ),
        )
        .unwrap();
    }
    tx.commit().unwrap();
    assert!(
        s.device_report(r.id, &r.credential_fingerprint, report(&r, 4097))
            .is_err()
    );
    assert_eq!(
        s.device_revoke(
            r.id,
            RevokeDevice {
                expected_revision: 2
            }
        )
        .unwrap()
        .device
        .state,
        DeviceState::Revoked
    );
}
#[test]
fn schema22_upgrade_is_byte_preserving_and_creates_no_device_or_report() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let m = s
        .add_memory(&lain_wire::memory::AddMemory {
            request_id: RequestId::new(),
            draft: lain_wire::memory::MemoryDraft {
                content: "explicit pre-device owner memory".into(),
                kind: lain_wire::memory::MemoryKind::Preference,
                scope: lain_wire::memory::MemoryScope::Global,
            },
        })
        .unwrap();
    let raw: String = s
        .connection
        .query_row(
            "SELECT snapshot FROM lain_memories WHERE id=?1",
            [m.id.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    s.connection
        .execute_batch("DROP TABLE device_reports; DROP TABLE devices; PRAGMA user_version=22;")
        .unwrap();
    drop(s);
    let s = Store::open(&dir.path().join("state.db")).unwrap();
    assert!(s.devices().unwrap().is_empty());
    assert_eq!(s.memory(m.id).unwrap(), m);
    assert_eq!(
        s.connection
            .query_row(
                "SELECT snapshot FROM lain_memories WHERE id=?1",
                [m.id.to_string()],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        raw
    );
    assert_eq!(
        s.connection
            .query_row("SELECT count(*) FROM device_reports", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
