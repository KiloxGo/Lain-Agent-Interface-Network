use super::*;
use crate::{actor::domain::*, memory::*};
use lain_wire::{
    DeviceId, DeviceWorkspaceId, ProjectId, RequestId, devices::*, memory_context::*, projects::*,
};

fn device(s: &mut Store, n: u8) -> Device {
    let token = format!("lain-device-{}", format!("{n:02x}").repeat(32));
    let r = RegisterDevice {
        id: DeviceId::new(),
        request_id: RequestId::new(),
        label: format!("scope-test-{n}"),
        platform: DevicePlatform::Windows,
        credential_fingerprint: device_credential_fingerprint(&token).unwrap(),
        workspaces: vec![DeviceWorkspace {
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
fn selected(d: &Device) -> MemoryContextSelection {
    MemoryContextSelection {
        device_id: d.registration.id,
        workspace_id: d.registration.workspaces[0].id,
        device_revision: d.revision,
        project: None,
    }
}
fn bind(
    s: &mut Store,
    conversation: ConversationId,
    selection: Option<MemoryContextSelection>,
    revision: u64,
) -> ConversationMemoryContext {
    s.set_memory_context(
        conversation,
        SetMemoryContext {
            request_id: RequestId::new(),
            expected_revision: revision,
            selection,
        },
    )
    .unwrap()
}
fn turn(s: &mut Store, conversation: ConversationId) -> ActorTurn {
    let mut t = ActorTurn {
        id: ActorTurnId::new(),
        conversation_id: conversation,
        request_id: uuid::Uuid::new_v4(),
        sequence: 0,
        revision: 0,
        state: TurnState::Running,
        messages: vec![],
        actions: vec![],
        error: None,
        model_requests: 0,
        usage_tokens: 0,
        timing: Default::default(),
        channel_memory: None,
        channel_source: None,
        private_chat: None,
        reading_activity: None,
        briefing_activity: None,
        sharing_activity: None,
    };
    t.message(MessageRole::User, "synthetic question".into())
        .unwrap();
    s.save_actor_turn(&mut t, true).unwrap();
    t
}
fn memory(s: &mut Store, scope: MemoryScope, text: &str) -> MemoryRecord {
    s.add_memory(&AddMemory {
        request_id: RequestId::new(),
        draft: MemoryDraft {
            content: text.into(),
            kind: MemoryKind::Episodic,
            scope,
        },
    })
    .unwrap()
}
fn query(scope: MemoryScope) -> MemoryQuery {
    MemoryQuery {
        scope,
        kind: None,
        status: None,
        text: None,
        after: 0,
        limit: 10,
    }
}

#[test]
fn scoped_memory_same_windows_path_stays_private_and_only_explicit_project_is_shared() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let a = device(&mut s, 111);
    let b = device(&mut s, 112);
    let ca = s.create_conversation().unwrap().id;
    let cb = s.create_conversation().unwrap().id;
    bind(&mut s, ca, Some(selected(&a)), 0);
    bind(&mut s, cb, Some(selected(&b)), 0);
    let local = MemoryScope::DeviceWorkspace {
        device_id: a.registration.id,
        workspace_id: a.registration.workspaces[0].id,
    };
    let r = memory(&mut s, local.clone(), "A private fact");
    assert_eq!(r.origin, Some(MemoryOrigin::OwnerApi));
    let mut forged_origin = r.clone();
    forged_origin.origin = Some(MemoryOrigin::Channel {
        channel_id: lain_wire::ChannelId::new(),
    });
    assert!(forged_origin.validate().is_err());
    let mut ta = turn(&mut s, ca);
    let mut tb = turn(&mut s, cb);
    assert_eq!(
        s.actor_memory_page(ta.id, &query(local.clone()))
            .unwrap()
            .records[0]
            .id,
        r.id
    );
    assert!(s.actor_memory_page(tb.id, &query(local)).is_err());
    assert!(s.actor_memory_inspect(tb.id, r.id, &r.draft.scope).is_err());
    let p = s
        .project_create(CreateProject {
            request_id: RequestId::new(),
            id: ProjectId::new(),
            name: "shared".into(),
        })
        .unwrap();
    let project_scope = MemoryScope::Project { project_id: p.id };
    let shared = memory(&mut s, project_scope.clone(), "shared project fact");
    assert!(
        s.actor_memory_page(ta.id, &query(project_scope.clone()))
            .is_err()
    );
    for t in [&mut ta, &mut tb] {
        t.state = TurnState::Completed;
        s.save_actor_turn(t, false).unwrap();
    }
    for (d, c) in [(&a, ca), (&b, cb)] {
        let m = s
            .set_project_workspace(SetProjectWorkspace {
                request_id: RequestId::new(),
                device_id: d.registration.id,
                workspace_id: d.registration.workspaces[0].id,
                expected_device_revision: d.revision,
                project_id: Some(p.id),
                expected_project_revision: Some(p.revision),
                expected_revision: 0,
            })
            .unwrap();
        let mut selection = selected(d);
        selection.project = Some(MemoryProjectContext {
            project_id: p.id,
            project_revision: p.revision,
            mapping_revision: m.revision,
        });
        bind(&mut s, c, Some(selection), 1);
    }
    let ta = turn(&mut s, ca);
    let tb = turn(&mut s, cb);
    for t in [&ta, &tb] {
        assert_eq!(
            s.actor_memory_page(t.id, &query(project_scope.clone()))
                .unwrap()
                .records[0]
                .id,
            shared.id
        );
    }
    assert!(s.actor_memory_inspect(tb.id, r.id, &r.draft.scope).is_err());
    let channel = memory(
        &mut s,
        MemoryScope::Channel {
            channel_id: lain_wire::ChannelId::new(),
        },
        "private synthetic channel fact",
    );
    assert!(
        s.actor_memory_inspect(tb.id, channel.id, &channel.draft.scope)
            .is_err()
    );
    let unrelated = s.create_conversation().unwrap().id;
    let unbound = turn(&mut s, unrelated);
    assert!(
        s.actor_memory_page(unbound.id, &query(project_scope))
            .is_err()
    );
    assert!(
        s.actor_memory_page(
            tb.id,
            &query(MemoryScope::Channel {
                channel_id: lain_wire::ChannelId::new()
            })
        )
        .is_err()
    );
}

#[test]
fn scoped_memory_correction_invalidates_derived_history_and_pending_candidates_after_restart() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("state.db");
    let mut s = Store::open(&db).unwrap();
    let c = s.create_conversation().unwrap().id;
    let r = memory(&mut s, MemoryScope::Global, "original synthetic fact");
    let mut first = turn(&mut s, c);
    s.actor_memory_inspect(first.id, r.id, &r.draft.scope)
        .unwrap();
    first
        .message(MessageRole::Assistant, "derived old fact".into())
        .unwrap();
    first.state = TurnState::Completed;
    s.save_actor_turn(&mut first, false).unwrap();
    let mut second = turn(&mut s, c);
    let history = s
        .memory_context_history(second.id, s.actor_recent(c).unwrap())
        .unwrap();
    assert!(
        history[0]
            .messages
            .iter()
            .any(|m| m.content == "derived old fact")
    );
    // The second turn inherits the first turn's source versions before model input.
    s.change_memory(
        r.id,
        &ChangeMemory {
            expected_revision: r.revision,
            status: MemoryStatus::Disabled,
            superseded_by: None,
        },
    )
    .unwrap();
    assert!(s.check_actor_memory(second.id).is_err());
    second
        .message(
            MessageRole::Assistant,
            "must not commit stale evidence".into(),
        )
        .unwrap();
    assert!(s.save_actor_turn(&mut second, false).is_err());
    second.messages.pop();
    second.state = TurnState::Unknown;
    s.save_actor_turn(&mut second, false).unwrap();
    drop(s);
    let mut s = Store::open(&db).unwrap();
    let third = turn(&mut s, c);
    let history = s
        .memory_context_history(third.id, s.actor_recent(c).unwrap())
        .unwrap();
    assert!(history.iter().all(|t| {
        t.messages
            .iter()
            .all(|m| !m.content.contains("derived old fact"))
    }));
    assert!(s.check_actor_memory(third.id).is_ok());
}

#[test]
fn scoped_memory_context_receipts_are_exact_and_revocation_cannot_be_undone_by_retry() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("state.db");
    let mut s = Store::open(&db).unwrap();
    let d = device(&mut s, 113);
    let c = s.create_conversation().unwrap().id;
    let input = SetMemoryContext {
        request_id: RequestId::new(),
        expected_revision: 0,
        selection: Some(selected(&d)),
    };
    let first = s.set_memory_context(c, input.clone()).unwrap();
    let t = turn(&mut s, c);
    assert!(
        s.set_memory_context(
            c,
            SetMemoryContext {
                request_id: RequestId::new(),
                expected_revision: 1,
                selection: None
            }
        )
        .is_err()
    );
    s.device_revoke(
        d.registration.id,
        RevokeDevice {
            expected_revision: d.revision,
        },
    )
    .unwrap();
    assert!(s.check_actor_memory(t.id).is_err());
    assert_eq!(s.set_memory_context(c, input.clone()).unwrap(), first);
    assert!(s.check_actor_memory(t.id).is_err());
    let mut changed = input;
    changed.selection = None;
    assert!(s.set_memory_context(c, changed).is_err());
}

#[test]
fn scoped_memory_candidates_keep_lineage_reject_widening_and_remain_declineable_when_stale() {
    use crate::actor::tools::Tool;
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let d = device(&mut s, 114);
    let c = s.create_conversation().unwrap().id;
    let selection = selected(&d);
    bind(&mut s, c, Some(selection.clone()), 0);
    let scope = MemoryScope::Device {
        device_id: d.registration.id,
    };
    let r = memory(&mut s, scope.clone(), "source device fact");
    let mut t = turn(&mut s, c);
    s.actor_memory_inspect(t.id, r.id, &scope).unwrap();
    let action = Action {
        id: ActionId::new(),
        call_id: "synthetic-proposal".into(),
        state: ActionState::Proposed,
        result: None,
        tool: Tool::MemoryPropose(ProposeMemory {
            draft: MemoryDraft {
                content: "derived fact".into(),
                kind: MemoryKind::Episodic,
                scope: scope.clone(),
            },
        }),
    };
    t.actions.push(action.clone());
    t.state = TurnState::PendingConfirmation;
    s.save_actor_turn(&mut t, false).unwrap();
    let candidate = s.candidate(CandidateId(action.id.0)).unwrap();
    assert_eq!(
        candidate.origin,
        Some(MemoryOrigin::OwnerSelectedActorContext {
            selection: Some(selection)
        })
    );
    s.change_memory(
        r.id,
        &ChangeMemory {
            expected_revision: 1,
            status: MemoryStatus::Disabled,
            superseded_by: None,
        },
    )
    .unwrap();
    assert!(
        s.review_candidate(candidate.id, true, Some(action.id))
            .is_err()
    );
    // Revocation must block acceptance, not block cancellation/denial cleanup.
    t.actions[0].state = ActionState::Rejected;
    t.state = TurnState::Completed;
    s.save_actor_turn(&mut t, false).unwrap();
    assert_eq!(
        s.candidate(candidate.id).unwrap().status,
        CandidateStatus::Rejected
    );
    let mut next = turn(&mut s, c);
    let mut wider = action;
    wider.id = ActionId::new();
    if let Tool::MemoryPropose(q) = &mut wider.tool {
        q.draft.scope = MemoryScope::Global;
    }
    next.actions.push(wider);
    next.state = TurnState::PendingConfirmation;
    assert!(s.save_actor_turn(&mut next, false).is_err());
}

#[test]
fn scoped_memory_derived_records_expire_with_source_and_context_switch_drops_private_history() {
    use crate::actor::tools::Tool;
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let a = device(&mut s, 115);
    let b = device(&mut s, 116);
    let c = s.create_conversation().unwrap().id;
    bind(&mut s, c, Some(selected(&a)), 0);
    let scope = MemoryScope::Device {
        device_id: a.registration.id,
    };
    let r = memory(&mut s, scope.clone(), "synthetic private source");
    let mut t = turn(&mut s, c);
    s.actor_memory_inspect(t.id, r.id, &scope).unwrap();
    let action = Action {
        id: ActionId::new(),
        call_id: "derive".into(),
        state: ActionState::Proposed,
        result: None,
        tool: Tool::MemoryPropose(ProposeMemory {
            draft: MemoryDraft {
                content: "derived private summary".into(),
                kind: MemoryKind::Episodic,
                scope: scope.clone(),
            },
        }),
    };
    t.actions.push(action.clone());
    t.state = TurnState::PendingConfirmation;
    s.save_actor_turn(&mut t, false).unwrap();
    let accepted = s
        .review_candidate(CandidateId(action.id.0), true, Some(action.id))
        .unwrap();
    let derived = accepted.memory_id.unwrap();
    assert!(s.memory(derived).is_ok());
    t.actions[0].state = ActionState::Executed;
    t.actions[0].result = Some(serde_json::json!(accepted));
    t.state = TurnState::Completed;
    t.message(MessageRole::Assistant, "private historical response".into())
        .unwrap();
    s.save_actor_turn(&mut t, false).unwrap();
    bind(&mut s, c, Some(selected(&b)), 1);
    let next = turn(&mut s, c);
    let history = s
        .memory_context_history(next.id, s.actor_recent(c).unwrap())
        .unwrap();
    assert!(
        history
            .iter()
            .find(|past| past.id == t.id)
            .unwrap()
            .messages
            .is_empty()
    );
    s.delete_memory(r.id, 1).unwrap();
    assert!(s.memory(derived).is_err());
    assert!(s.memory_page(&query(scope)).unwrap().records.is_empty());
    assert!(s.check_actor_memory(next.id).is_ok());
}

#[test]
fn scoped_memory_schema29_keeps_legacy_bytes_but_untracked_generated_history_cannot_be_reused() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("state.db");
    let mut s = Store::open(&db).unwrap();
    let c = s.create_conversation().unwrap().id;
    let mut old = turn(&mut s, c);
    old.message(
        MessageRole::Assistant,
        "legacy untracked generated content".into(),
    )
    .unwrap();
    old.state = TurnState::Completed;
    s.save_actor_turn(&mut old, false).unwrap();
    let tx = s.connection.transaction().unwrap();
    let r = super::memory::insert_memory(
        &tx,
        MemoryDraft {
            content: "legacy derived memory".into(),
            kind: MemoryKind::Episodic,
            scope: MemoryScope::Global,
        },
        MemoryProvenance {
            authority: MemoryAuthority::Inferred,
            source: MemorySource::LainGenerated {
                conversation_id: c,
                actor_turn_id: old.id,
                action_id: ActionId::new(),
            },
        },
    )
    .unwrap();
    let mut raw = serde_json::to_value(&r).unwrap();
    raw.as_object_mut().unwrap().remove("origin");
    let raw = serde_json::to_string(&raw).unwrap();
    tx.execute(
        "UPDATE lain_memories SET snapshot=?2 WHERE id=?1",
        (r.id.to_string(), &raw),
    )
    .unwrap();
    tx.commit().unwrap();
    s.connection.execute_batch("DROP TABLE actor_memory_contexts; DROP TABLE memory_context_requests; DROP TABLE conversation_memory_contexts; DROP TABLE briefing_relations; DROP TABLE conversation_memory_effects; DROP TABLE briefing_memory_suppressions; DROP TABLE briefing_disclosures; DROP TABLE briefing_runs; DROP TABLE briefing_batches; DROP TABLE briefing_manifests; DROP TABLE briefing_grants; PRAGMA user_version=29;").unwrap();
    drop(s);
    let mut s = Store::open(&db).unwrap();
    assert_eq!(s.memory(r.id).unwrap().origin, None); // owner can still inspect unknown-origin history
    let stored: String = s
        .connection
        .query_row(
            "SELECT snapshot FROM lain_memories WHERE id=?1",
            [r.id.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(stored, raw);
    let next = turn(&mut s, c);
    assert!(
        s.actor_memory_page(next.id, &query(MemoryScope::Global))
            .unwrap()
            .records
            .is_empty()
    );
    assert!(
        s.actor_memory_inspect(next.id, r.id, &MemoryScope::Global)
            .is_err()
    );
    let history = s
        .memory_context_history(next.id, s.actor_recent(c).unwrap())
        .unwrap();
    let legacy = history.iter().find(|t| t.id == old.id).unwrap();
    assert!(legacy.messages.iter().all(|m| m.role == MessageRole::User));
}

#[test]
fn bridge_schema30_upgrade_gates_the_new_context_format_without_rewriting_legacy_rows() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("state.db");
    let mut s = Store::open(&db).unwrap();
    let c = s.create_conversation().unwrap().id;
    let mut old = turn(&mut s, c);
    old.state = TurnState::Completed;
    s.save_actor_turn(&mut old, false).unwrap();
    let raw: String = s
        .connection
        .query_row(
            "SELECT snapshot FROM actor_memory_contexts WHERE actor_turn_id=?1",
            [old.id.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    let mut legacy: serde_json::Value = serde_json::from_str(&raw).unwrap();
    legacy.as_object_mut().unwrap().remove("private_lineage");
    let legacy = serde_json::to_string(&legacy).unwrap();
    s.connection
        .execute(
            "UPDATE actor_memory_contexts SET snapshot=?2 WHERE actor_turn_id=?1",
            (old.id.to_string(), &legacy),
        )
        .unwrap();
    s.connection
        .execute_batch(
            "DROP TABLE briefing_relations; DROP TABLE conversation_memory_effects; DROP TABLE briefing_memory_suppressions; DROP TABLE briefing_disclosures; DROP TABLE briefing_runs; DROP TABLE briefing_batches; DROP TABLE briefing_manifests; DROP TABLE briefing_grants; PRAGMA user_version=30;",
        )
        .unwrap();
    drop(s);
    let mut s = Store::open(&db).unwrap();
    assert_eq!(
        s.connection
            .query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))
            .unwrap(),
        crate::persistence::SCHEMA_VERSION
    );
    let after: String = s
        .connection
        .query_row(
            "SELECT snapshot FROM actor_memory_contexts WHERE actor_turn_id=?1",
            [old.id.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(after, legacy);
    assert!(s.check_actor_memory(old.id).is_err());
    let fresh = turn(&mut s, c);
    assert!(s.check_actor_memory(fresh.id).is_ok());
}
#[test]
fn bridge_completed_transition_rechecks_sources_after_body_was_saved() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let c = s.create_conversation().unwrap().id;
    let record = memory(&mut s, MemoryScope::Global, "source fact");
    let mut t = turn(&mut s, c);
    s.actor_memory_inspect(t.id, record.id, &MemoryScope::Global)
        .unwrap();
    t.message(
        MessageRole::Assistant,
        "body saved before source withdrawal".into(),
    )
    .unwrap();
    s.save_actor_turn(&mut t, false).unwrap();
    s.delete_memory(record.id, record.revision).unwrap();
    t.state = TurnState::Completed;
    assert!(s.save_actor_turn(&mut t, false).is_err());
}
