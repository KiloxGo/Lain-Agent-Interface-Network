use super::*;
use crate::backend::ControlError;
use lain_wire::{DeviceId, DeviceWorkspaceId, ProjectId, RequestId, devices::*, projects::*};

fn device(s: &mut Store, n: u8) -> Device {
    let token = format!("lain-device-{}", format!("{n:02x}").repeat(32));
    let r = RegisterDevice {
        id: DeviceId::new(),
        request_id: RequestId::new(),
        label: format!("test-pc-{n}"),
        platform: DevicePlatform::Windows,
        credential_fingerprint: device_credential_fingerprint(&token).unwrap(),
        workspaces: vec![DeviceWorkspace {
            // Even a repeated workspace UUID is scoped to its admitted device.
            id: DeviceWorkspaceId(uuid::Uuid::from_u128(77)),
            path: "C:\\synthetic\\same-path".into(),
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
    .unwrap()
    .device
}
fn create(s: &mut Store, name: &str) -> Project {
    s.project_create(CreateProject {
        request_id: RequestId::new(),
        id: ProjectId::new(),
        name: name.into(),
    })
    .unwrap()
}
fn decision(d: &Device, p: &Project, expected_revision: u64) -> SetProjectWorkspace {
    SetProjectWorkspace {
        request_id: RequestId::new(),
        device_id: d.registration.id,
        workspace_id: d.registration.workspaces[0].id,
        expected_device_revision: d.revision,
        project_id: Some(p.id),
        expected_project_revision: Some(p.revision),
        expected_revision,
    }
}

#[test]
fn projects_same_path_never_maps_implicitly_and_exact_retry_returns_original_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("state.db");
    let mut s = Store::open(&db).unwrap();
    let a = device(&mut s, 101);
    let b = device(&mut s, 102);
    let p = create(&mut s, "explicit shared project");
    let q = create(&mut s, "another project");
    assert_eq!(
        a.registration.workspaces[0].path,
        b.registration.workspaces[0].path
    );
    assert!(
        s.project_workspace(b.registration.id, b.registration.workspaces[0].id)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        a.registration.workspaces[0].id,
        b.registration.workspaces[0].id
    );
    let input = decision(&a, &p, 0);
    let first = s.set_project_workspace(input.clone()).unwrap();
    assert_eq!(first.revision, 1);
    assert_eq!(first.project_id, Some(p.id));
    let mut changed = input.clone();
    changed.project_id = Some(q.id);
    assert!(matches!(
        s.set_project_workspace(changed).unwrap_err().downcast_ref(),
        Some(ControlError::Conflict)
    ));
    let updated = s.set_project_workspace(decision(&a, &q, 1)).unwrap();
    assert_eq!(updated.revision, 2);
    assert_eq!(s.set_project_workspace(input.clone()).unwrap(), first);
    assert_eq!(
        s.project_workspace(a.registration.id, a.registration.workspaces[0].id)
            .unwrap(),
        Some(updated)
    );
    assert!(
        s.project_workspace(b.registration.id, b.registration.workspaces[0].id)
            .unwrap()
            .is_none()
    );
    let shared = s.set_project_workspace(decision(&b, &p, 0)).unwrap();
    assert_eq!(shared.project_id, Some(p.id));
    assert_eq!(
        s.project_workspace(a.registration.id, a.registration.workspaces[0].id)
            .unwrap()
            .unwrap()
            .project_id,
        Some(q.id)
    );
    drop(s);
    let mut s = Store::open(&db).unwrap();
    assert_eq!(s.set_project_workspace(input).unwrap(), first);
    assert_eq!(s.project(p.id).unwrap(), p);
}

#[test]
fn projects_schema28_migration_preserves_legacy_memory_and_adds_no_assignments() {
    use lain_wire::memory::*;
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("old.db");
    let mut s = Store::open(&db).unwrap();
    let mut saved = Vec::new();
    for scope in [
        MemoryScope::Global,
        MemoryScope::Workspace {
            path: dir.path().to_str().unwrap().into(),
        },
    ] {
        saved.push(
            s.add_memory(&AddMemory {
                request_id: RequestId::new(),
                draft: MemoryDraft {
                    content: "legacy selected observation".into(),
                    kind: MemoryKind::Preference,
                    scope,
                },
            })
            .unwrap(),
        );
    }
    let snapshots: Vec<String> = s
        .connection
        .prepare("SELECT snapshot FROM lain_memories ORDER BY sequence")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    s.connection.execute_batch("DROP TABLE actor_memory_contexts; DROP TABLE memory_context_requests; DROP TABLE conversation_memory_contexts; DROP TABLE project_requests; DROP TABLE project_workspaces; DROP TABLE projects; DROP TABLE briefing_relations; DROP TABLE conversation_memory_effects; DROP TABLE briefing_memory_suppressions; DROP TABLE briefing_disclosures; DROP TABLE briefing_runs; DROP TABLE briefing_batches; DROP TABLE briefing_manifests; DROP TABLE briefing_grants; PRAGMA user_version=28;").unwrap();
    drop(s);
    let s = Store::open(&db).unwrap();
    assert_eq!(
        s.connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        crate::persistence::SCHEMA_VERSION
    );
    assert!(s.projects().unwrap().is_empty());
    assert_eq!(
        s.connection
            .query_row("SELECT count(*) FROM project_workspaces", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    let after: Vec<String> = s
        .connection
        .prepare("SELECT snapshot FROM lain_memories ORDER BY sequence")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert_eq!(after, snapshots);
    for record in saved {
        assert_eq!(
            serde_json::to_value(s.memory(record.id).unwrap()).unwrap(),
            serde_json::to_value(record).unwrap()
        );
    }
}

#[test]
fn projects_expired_device_cannot_add_assignment_but_owner_can_clear_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let d = device(&mut s, 104);
    let p = create(&mut s, "expiry fixture");
    s.set_project_workspace(decision(&d, &p, 0)).unwrap();
    let mut expired = d.clone();
    expired.registration.expires_at_ms = now_ms().unwrap() - 1;
    s.connection
        .execute(
            "UPDATE devices SET snapshot=?2 WHERE id=?1",
            (
                d.registration.id.to_string(),
                serde_json::to_string(&expired).unwrap(),
            ),
        )
        .unwrap();
    assert!(s.set_project_workspace(decision(&d, &p, 1)).is_err());
    let mut clear = decision(&d, &p, 1);
    clear.project_id = None;
    clear.expected_project_revision = None;
    assert_eq!(s.set_project_workspace(clear).unwrap().project_id, None);
}

#[test]
fn projects_display_names_are_not_identity_and_request_ids_cannot_cross_operations() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let input = CreateProject {
        id: ProjectId::new(),
        request_id: RequestId::new(),
        name: "equal display names".into(),
    };
    let p = s.project_create(input.clone()).unwrap();
    let q = create(&mut s, &p.name);
    assert_ne!(p.id, q.id);
    assert!(
        s.project_change(
            p.id,
            ChangeProject {
                request_id: input.request_id,
                expected_revision: 1,
                name: p.name.clone(),
                state: ProjectState::Archived
            }
        )
        .is_err()
    );
    let mut same_id = input.clone();
    same_id.request_id = RequestId::new();
    assert!(s.project_create(same_id).is_err());
    for name in ["".into(), "\ncontrol".into(), "x".repeat(129)] {
        assert!(
            s.project_create(CreateProject {
                id: ProjectId::new(),
                request_id: RequestId::new(),
                name
            })
            .is_err()
        );
    }
    let mut nil = input;
    nil.id = ProjectId(uuid::Uuid::nil());
    nil.request_id = RequestId::new();
    assert!(s.project_create(nil).is_err());
    assert_eq!(s.projects().unwrap().len(), 2);
}

#[test]
fn projects_mapping_checks_exact_scope_and_cas_and_retains_unbound_tombstone() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let d = device(&mut s, 103);
    let p = create(&mut s, "fixture");
    let input = decision(&d, &p, 0);
    let mut wrong = input.clone();
    wrong.workspace_id = DeviceWorkspaceId::new();
    assert!(s.set_project_workspace(wrong).is_err());
    let first = s.set_project_workspace(input).unwrap();
    assert!(s.set_project_workspace(decision(&d, &p, 0)).is_err());
    let mut clear = decision(&d, &p, first.revision);
    clear.project_id = None;
    clear.expected_project_revision = None;
    let cleared = s.set_project_workspace(clear.clone()).unwrap();
    assert_eq!(cleared.revision, 2);
    assert_eq!(cleared.project_id, None);
    assert_eq!(s.set_project_workspace(clear).unwrap(), cleared);
    assert!(s.set_project_workspace(decision(&d, &p, 0)).is_err());
    let archived = s
        .project_change(
            p.id,
            ChangeProject {
                request_id: RequestId::new(),
                expected_revision: p.revision,
                name: p.name.clone(),
                state: ProjectState::Archived,
            },
        )
        .unwrap();
    let mut against_archived = decision(&d, &archived, cleared.revision);
    assert!(s.set_project_workspace(against_archived.clone()).is_err());
    against_archived.project_id = None;
    against_archived.expected_project_revision = Some(archived.revision);
    assert!(s.set_project_workspace(against_archived).is_err());
    s.device_revoke(
        d.registration.id,
        RevokeDevice {
            expected_revision: d.revision,
        },
    )
    .unwrap();
    assert!(
        s.set_project_workspace(decision(&d, &p, cleared.revision))
            .is_err()
    );
}

#[test]
fn projects_creation_and_changes_are_idempotent_and_reject_reused_or_stale_requests() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let create = CreateProject {
        request_id: RequestId::new(),
        id: ProjectId::new(),
        name: "original".into(),
    };
    let p = s.project_create(create.clone()).unwrap();
    let change = ChangeProject {
        request_id: RequestId::new(),
        expected_revision: p.revision,
        name: "renamed".into(),
        state: ProjectState::Archived,
    };
    let archived = s.project_change(p.id, change.clone()).unwrap();
    assert_eq!(archived.revision, 2);
    assert_eq!(s.project_create(create.clone()).unwrap(), p);
    assert_eq!(s.project_change(p.id, change).unwrap(), archived);
    let mut duplicate = create;
    duplicate.name = "changed payload".into();
    assert!(s.project_create(duplicate).is_err());
    assert!(
        s.project_change(
            p.id,
            ChangeProject {
                request_id: RequestId::new(),
                expected_revision: 1,
                name: "stale".into(),
                state: ProjectState::Active
            }
        )
        .is_err()
    );
    assert_eq!(s.projects().unwrap(), vec![archived]);
}
