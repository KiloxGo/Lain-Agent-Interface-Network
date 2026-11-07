use super::*;
use crate::{
    actor::{domain::*, tools::Tool},
    channels::domain::*,
    memory::*,
};
use lain_wire::{ActionId, ActorTurnId, ArchiveRecordId, ArchiveSourceId, RequestId, archives::*};

fn setup(
    max: u32,
) -> (
    tempfile::TempDir,
    Store,
    Binding,
    ArchiveSourceId,
    ArchiveRecordId,
) {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let b = s
        .channel_bind(
            BindingSpec {
                name: "memory".into(),
                adapter: "fixture".into(),
                account: "1001".into(),
                peer: "2002".into(),
            },
            true,
        )
        .unwrap();
    s.set_memory_grant(
        b.id,
        SetMemoryGrant {
            enabled: true,
            expected_revision: 0,
            expires_at_ms: None,
        },
    )
    .unwrap();
    let now = now_ms().unwrap();
    let expiry = now + 600000;
    let source = s
        .archive_register(
            ArchiveSourceKey::QqGroup {
                observer_account: "1001".into(),
                group_id: "3003".into(),
            },
            now,
        )
        .unwrap()
        .source
        .id;
    s.archive_set_grant(
        source,
        SetArchiveGrant {
            expected_revision: 0,
            scope: Some(ArchiveScope {
                expires_at_ms: expiry,
                retention_ms: 600000,
                max_records: 100,
            }),
        },
        now,
    )
    .unwrap();
    let record = s
        .archive_import(
            source,
            ImportArchive {
                expected_revision: 1,
                records: vec![ImportArchiveRecord {
                    external_id: "first".into(),
                    source_position: None,
                    occurred_at_ms: Some(now),
                    speaker: "reported-person".into(),
                    kind: VisibleRecordKind::Message,
                    text: "测试偏好喜欢蓝色，这是外部报告".into(),
                }],
            },
            now,
        )
        .unwrap()
        .records[0]
        .id;
    s.archive_recall_set(
        b.id,
        SetArchiveRecall {
            expected_revision: 0,
            scope: Some(ArchiveRecallScope {
                sources: vec![ArchiveRecallSource {
                    source_id: source,
                    grant_revision: 1,
                }],
                expires_at_ms: expiry,
                utc_offset_minutes: 480,
            }),
        },
    )
    .unwrap();
    if max > 0 {
        s.archive_memory_set(
            b.id,
            SetArchiveMemory {
                expected_revision: 0,
                scope: Some(ArchiveMemoryScope {
                    sources: vec![ArchiveRecallSource {
                        source_id: source,
                        grant_revision: 1,
                    }],
                    expires_at_ms: expiry,
                    retention_ms: 300000,
                    max_writes: max,
                }),
            },
        )
        .unwrap();
    }
    (dir, s, b, source, record)
}
fn turn(s: &mut Store, b: &Binding, text: &str) -> (ActorTurn, MemoryEvidence) {
    let i = s
        .channel_receive(
            b.id,
            Input {
                segments: vec![],
                account: b.spec.account.clone(),
                peer: b.spec.peer.clone(),
                external_id: uuid::Uuid::new_v4().to_string(),
                text: text.into(),
            },
        )
        .unwrap();
    let batch = s
        .channel_next_batch(b.id, i.received_at_ms + 1000)
        .unwrap()
        .unwrap();
    let mut t = ActorTurn {
        reading_activity: None,
        briefing_activity: None,
        sharing_activity: None,
        id: ActorTurnId::new(),
        conversation_id: b.conversation_id,
        request_id: batch.id.0,
        sequence: 0,
        revision: 0,
        state: TurnState::Running,
        messages: vec![],
        actions: vec![],
        error: None,
        model_requests: 1,
        usage_tokens: 0,
        timing: Default::default(),
        channel_memory: s.channel_memory_context(b.id, batch.id).unwrap(),
        channel_source: None,
        private_chat: Some(s.private_chat_context(b.id, batch.id).unwrap()),
    };
    t.message(MessageRole::User, text.into()).unwrap();
    s.save_actor_turn(&mut t, true).unwrap();
    s.channel_link(batch.id, &t).unwrap();
    (
        t,
        MemoryEvidence {
            incoming_id: i.id,
            quote: text.into(),
        },
    )
}
fn action(s: &mut Store, t: &mut ActorTurn, tool: Tool) -> ActionId {
    *t = s.actor_turn(t.id).unwrap();
    let id = ActionId::new();
    t.actions.push(Action {
        id,
        call_id: id.to_string(),
        tool,
        state: ActionState::Executing,
        result: None,
    });
    s.save_actor_turn(t, false).unwrap();
    id
}
fn inspect(s: &mut Store, t: &mut ActorTurn, record: ArchiveRecordId) {
    let id = action(s, t, Tool::ArchiveInspect { record_id: record });
    let result = s
        .archive_actor_lookup(t.id, id, &crate::redaction::Redactor::new(vec![]))
        .unwrap();
    *t = s.actor_turn(t.id).unwrap();
    let a = t.actions.iter_mut().find(|a| a.id == id).unwrap();
    a.state = ActionState::Executed;
    a.result = Some(result);
    s.save_actor_turn(t, false).unwrap();
    s.archive_actor_disclosure(
        t.id,
        t.model_requests,
        &crate::redaction::Redactor::new(vec![]),
    )
    .unwrap();
    let _ = s.archive_memory_scope(t.id);
}
fn finish(s: &mut Store, t: &mut ActorTurn) {
    *t = s.actor_turn(t.id).unwrap();
    t.state = TurnState::Completed;
    t.private_chat.as_mut().unwrap().decision = lain_wire::chat::ChatDecision::Reply;
    for a in &mut t.actions {
        if a.result.is_none() {
            let receipt: String = s
                .connection
                .query_row(
                    "SELECT snapshot FROM channel_memory_effects WHERE action_id=?1",
                    [a.id.to_string()],
                    |r| r.get(0),
                )
                .unwrap();
            a.result = Some(serde_json::from_str(&receipt).unwrap());
        }
        a.state = ActionState::Executed;
    }
    s.save_actor_turn(t, false).unwrap();
    s.channel_finish(BatchId(t.request_id), t).unwrap();
}
fn remember(s: &mut Store, b: &Binding, record: ArchiveRecordId) -> MemoryRecord {
    let (mut t, _) = turn(s, b, "保存这条有来源的测试认识");
    inspect(s, &mut t, record);
    let id = action(
        s,
        &mut t,
        Tool::ArchiveRemember(RememberArchive {
            content: "测试偏好喜欢蓝色，据外部报告，尚未核实".into(),
            kind: MemoryKind::Preference,
        }),
    );
    let e = s.private_memory_effect(t.id, id).unwrap();
    let m = s.memory(e.memory_id).unwrap();
    finish(s, &mut t);
    m
}
#[test]
fn archive_selection_is_default_off_inferred_atomic_idempotent_and_budgeted() {
    let (_dir, mut s, b, source, record) = setup(0);
    let (mut t, _) = turn(&mut s, &b, "选择测试认识");
    inspect(&mut s, &mut t, record);
    let id = action(
        &mut s,
        &mut t,
        Tool::ArchiveRemember(RememberArchive {
            content: "测试偏好喜欢蓝色".into(),
            kind: MemoryKind::Preference,
        }),
    );
    assert!(s.private_memory_effect(t.id, id).is_err());
    let expiry = s
        .archive_source(source)
        .unwrap()
        .grant
        .scope
        .unwrap()
        .expires_at_ms;
    s.archive_memory_set(
        b.id,
        SetArchiveMemory {
            expected_revision: 0,
            scope: Some(ArchiveMemoryScope {
                sources: vec![ArchiveRecallSource {
                    source_id: source,
                    grant_revision: 1,
                }],
                expires_at_ms: expiry,
                retention_ms: 300000,
                max_writes: 1,
            }),
        },
    )
    .unwrap();
    s.archive_memory_scope(t.id).unwrap();
    s.connection.execute_batch("CREATE TRIGGER reject_effect BEFORE INSERT ON channel_memory_effects BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    assert!(s.private_memory_effect(t.id, id).is_err());
    assert_eq!(s.archive_memory_grant(b.id).unwrap().used_writes, 0);
    assert_eq!(
        s.connection
            .query_row("SELECT count(*) FROM lain_memories", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    s.connection
        .execute_batch("DROP TRIGGER reject_effect")
        .unwrap();
    let effect = s.private_memory_effect(t.id, id).unwrap();
    assert_eq!(
        effect.memory_id,
        s.private_memory_effect(t.id, id).unwrap().memory_id
    );
    let m = s.memory(effect.memory_id).unwrap();
    assert!(m.owner_confirmed_at_ms.is_none());
    assert_eq!(m.provenance.authority, MemoryAuthority::Inferred);
    let MemorySource::ArchiveSelection { evidence: e } = m.provenance.source else {
        panic!("wrong provenance")
    };
    assert_eq!(e.records[0].record_id, record);
    assert_eq!(e.incoming_ids.len(), 1);
    assert_eq!(effect.archive_grant_revision, Some(1));
    assert_eq!(s.archive_memory_grant(b.id).unwrap().used_writes, 1);
    let again = action(
        &mut s,
        &mut t,
        Tool::ArchiveRemember(RememberArchive {
            content: "another selection".into(),
            kind: MemoryKind::Episodic,
        }),
    );
    assert!(s.private_memory_effect(t.id, again).is_err());
}
#[test]
fn source_and_policy_changes_withhold_memory_and_invalidate_prepared_speech() {
    for case in 0..7 {
        let (_dir, mut s, b, source, record) = setup(10);
        let m = remember(&mut s, &b, record);
        let (t, _) = turn(&mut s, &b, "蓝色偏好");
        let context = s.private_memory_context(t.id, &m.id.to_string()).unwrap();
        assert_eq!(context["records"].as_array().unwrap().len(), 1);
        assert!(s.private_chat_memory_current(t.id).unwrap());
        match case {
            0 => {
                s.archive_correct(
                    record,
                    CorrectArchiveRecord {
                        expected_revision: 1,
                        text: "更正为另一条报告".into(),
                    },
                    now_ms().unwrap(),
                )
                .unwrap();
            }
            1 => {
                s.archive_forget(record, 1, now_ms().unwrap()).unwrap();
            }
            2 => {
                s.archive_set_grant(
                    source,
                    SetArchiveGrant {
                        expected_revision: 1,
                        scope: None,
                    },
                    now_ms().unwrap(),
                )
                .unwrap();
            }
            3 => {
                s.archive_recall_set(
                    b.id,
                    SetArchiveRecall {
                        expected_revision: 1,
                        scope: None,
                    },
                )
                .unwrap();
            }
            4 => {
                s.archive_memory_set(
                    b.id,
                    SetArchiveMemory {
                        expected_revision: 1,
                        scope: None,
                    },
                )
                .unwrap();
            }
            5 => {
                let MemorySource::ArchiveSelection { evidence: e } = &m.provenance.source else {
                    panic!()
                };
                s.archive_prune(e.expires_at_ms).unwrap();
            }
            _ => {
                s.set_memory_grant(
                    b.id,
                    SetMemoryGrant {
                        enabled: false,
                        expected_revision: 1,
                        expires_at_ms: None,
                    },
                )
                .unwrap();
            }
        }
        assert!(s.memory(m.id).is_err());
        assert!(!s.private_chat_memory_current(t.id).unwrap());
        let page = s
            .memory_page(&MemoryQuery {
                scope: MemoryScope::Channel { channel_id: b.id },
                kind: None,
                status: None,
                text: None,
                after: 0,
                limit: 10,
            })
            .unwrap();
        assert!(page.records.is_empty());
    }
}
#[test]
fn prepared_selection_cannot_adopt_replacement_grant_or_missing_disclosure() {
    for replace_policy in [false, true] {
        let (_dir, mut s, b, _, record) = setup(10);
        let (mut t, _) = turn(&mut s, &b, "保存测试认识");
        inspect(&mut s, &mut t, record);
        // Prepare while authorized, then invalidate before the actual effect.
        // New lineage also rejects adding an action after the scope changed.
        let id = action(
            &mut s,
            &mut t,
            Tool::ArchiveRemember(RememberArchive {
                content: "不应落库的认识".into(),
                kind: MemoryKind::Preference,
            }),
        );
        if replace_policy {
            let g = s.archive_memory_grant(b.id).unwrap();
            s.archive_memory_set(
                b.id,
                SetArchiveMemory {
                    expected_revision: g.revision,
                    scope: g.scope,
                },
            )
            .unwrap();
            assert!(s.archive_memory_scope(t.id).is_err());
            assert!(!s.private_chat_memory_current(t.id).unwrap());
        } else {
            s.connection
                .execute(
                    "DELETE FROM archive_recall_reads WHERE model_request IS NOT NULL",
                    [],
                )
                .unwrap();
        }
        assert!(s.private_memory_effect(t.id, id).is_err());
        assert_eq!(s.archive_memory_grant(b.id).unwrap().used_writes, 0);
        assert_eq!(
            s.connection
                .query_row("SELECT count(*) FROM lain_memories", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
#[test]
fn flattened_lineage_withholds_descendants_after_parent_change_and_survives_restart() {
    let (dir, mut s, b, _, record) = setup(10);
    let parent = remember(&mut s, &b, record);
    let (mut t, _) = turn(&mut s, &b, "结合之前的测试认识");
    s.private_memory_context(t.id, &parent.id.to_string())
        .unwrap();
    inspect(&mut s, &mut t, record);
    let id = action(
        &mut s,
        &mut t,
        Tool::ArchiveRemember(RememberArchive {
            content: "结合外部报告和先前认识形成的另一条测试印象".into(),
            kind: MemoryKind::Episodic,
        }),
    );
    let effect = s.private_memory_effect(t.id, id).unwrap();
    let child = s.memory(effect.memory_id).unwrap();
    let MemorySource::ArchiveSelection { evidence: e } = &child.provenance.source else {
        panic!()
    };
    assert!(e.memories.iter().any(|v| v.memory_id == parent.id));
    assert_eq!(e.incoming_ids.len(), 2);
    finish(&mut s, &mut t);
    drop(s);
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    assert_eq!(s.archive_memory_grant(b.id).unwrap().used_writes, 2);
    assert_eq!(s.memory(child.id).unwrap(), child);
    s.change_memory(
        parent.id,
        &ChangeMemory {
            expected_revision: 1,
            status: MemoryStatus::Disabled,
            superseded_by: None,
        },
    )
    .unwrap();
    assert!(s.memory(child.id).is_err());
    assert_eq!(s.archive_memory_grant(b.id).unwrap().used_writes, 2);
    let (t, _) = turn(&mut s, &b, "忘掉不可用的测试认识");
    let metadata = s
        .private_memory_context(t.id, &child.id.to_string())
        .unwrap();
    assert!(metadata["records"].as_array().unwrap().is_empty());
    assert_eq!(
        metadata["withheld_targets"][0]["memory_id"],
        child.id.to_string()
    );
    assert!(
        !serde_json::to_string(&metadata)
            .unwrap()
            .contains(&child.draft.content)
    );
}
#[test]
fn stale_upstream_lease_withholds_without_changing_memory_revision_or_renewing_retention() {
    let (_dir, mut s, b, source, record) = setup(10);
    let m = remember(&mut s, &b, record);
    let now = now_ms().unwrap();
    let state = KnowledgeArchiveState {
        coverage: None,
        source_id: source,
        local_grant_revision: 1,
        reader_revision: "a".repeat(64),
        store_id: uuid::Uuid::new_v4().to_string(),
        remote_source_revision: 1,
        remote_source_enabled: true,
        through: 1,
        last_read: RequestId::new(),
        observed_at_ms: now,
        fresh_until_ms: now + 30000,
    };
    s.connection
        .execute(
            "INSERT INTO knowledge_sources VALUES (?1,?2)",
            (source.to_string(), serde_json::to_string(&state).unwrap()),
        )
        .unwrap();
    s.connection
        .execute(
            "INSERT INTO knowledge_records VALUES (?1,?2,1,'fixture','hash',0)",
            (record.to_string(), source.to_string()),
        )
        .unwrap();
    assert!(s.memory(m.id).is_ok());
    s.connection
        .execute(
            "UPDATE knowledge_sources SET snapshot=json_set(snapshot,'$.fresh_until_ms',0)",
            [],
        )
        .unwrap();
    assert!(s.memory(m.id).is_err());
    s.connection
        .execute(
            "UPDATE knowledge_sources SET snapshot=json_set(snapshot,'$.fresh_until_ms',?1)",
            [i64::try_from(now + 30000).unwrap()],
        )
        .unwrap();
    assert_eq!(s.memory(m.id).unwrap(), m);
}
#[test]
fn owner_correction_replaces_inference_and_rebuilds_context_without_old_source() {
    let (_dir, mut s, b, source, record) = setup(10);
    let m = remember(&mut s, &b, record);
    let (mut t, e) = turn(&mut s, &b, "更正一下，我的测试偏好是绿色");
    s.private_memory_context(t.id, &m.id.to_string()).unwrap();
    t = s.actor_turn(t.id).unwrap();
    let id = action(
        &mut s,
        &mut t,
        Tool::MemoryCorrect(CorrectMemory {
            memory_id: m.id,
            expected_revision: 1,
            selection: SelectMemory {
                content: "测试偏好喜欢绿色".into(),
                kind: MemoryKind::Preference,
                evidence: e,
            },
        }),
    );
    let effect = s.private_memory_effect(t.id, id).unwrap();
    assert!(s.memory(m.id).is_err());
    let new = s.memory(effect.memory_id).unwrap();
    assert!(matches!(
        new.provenance.source,
        MemorySource::ChannelSelection { .. }
    ));
    let t = s.actor_turn(t.id).unwrap();
    let chat = t.private_chat.unwrap();
    assert!(
        chat.archive_memory_tainted
            && chat.archive_memory_versions.is_empty()
            && chat.memory_versions.is_empty()
    );
    assert!(s.private_chat_memory_current(t.id).unwrap());
    s.archive_set_grant(
        source,
        SetArchiveGrant {
            expected_revision: 1,
            scope: None,
        },
        now_ms().unwrap(),
    )
    .unwrap();
    assert_eq!(s.memory(new.id).unwrap().draft.content, "测试偏好喜欢绿色");
}
#[test]
fn forgotten_derived_selection_suppresses_sources_without_deleting_original_archive() {
    let (_dir, mut s, b, _, record) = setup(10);
    let m = remember(&mut s, &b, record);
    let (mut t, e) = turn(&mut s, &b, "忘掉那条测试认识");
    s.private_memory_context(t.id, &m.id.to_string()).unwrap();
    t = s.actor_turn(t.id).unwrap();
    let id = action(
        &mut s,
        &mut t,
        Tool::MemoryForget(ForgetMemory {
            memory_id: m.id,
            expected_revision: 1,
            evidence: e,
        }),
    );
    s.private_memory_effect(t.id, id).unwrap();
    assert!(s.memory(m.id).is_err());
    assert!(
        s.archive_record(record, now_ms().unwrap())
            .unwrap()
            .text
            .is_some()
    );
    assert!(archive_memory::suppressed(&s.connection, b.id, record).unwrap());
    assert_eq!(s.archive_memory_grant(b.id).unwrap().used_writes, 1);
    assert!(
        s.memory_effects(b.id, 0).unwrap()["effects"]
            .as_array()
            .unwrap()
            .len()
            >= 2
    );
}
#[test]
fn schema21_upgrade_preserves_owner_memory_and_keeps_derived_scope_disabled() {
    let (dir, mut s, b, _, _) = setup(0);
    let m = s
        .add_memory(&AddMemory {
            request_id: RequestId::new(),
            draft: MemoryDraft {
                content: "ordinary explicit owner record".into(),
                kind: MemoryKind::Preference,
                scope: MemoryScope::Global,
            },
        })
        .unwrap();
    let before: String = s
        .connection
        .query_row(
            "SELECT snapshot FROM lain_memories WHERE id=?1",
            [m.id.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    s.connection.execute_batch("DROP TABLE actor_memory_contexts; DROP TABLE memory_context_requests; DROP TABLE conversation_memory_contexts; DROP TABLE project_requests; DROP TABLE project_workspaces; DROP TABLE projects; DROP TABLE device_work_continuations; DROP TABLE device_work_links; DROP TABLE device_approval_decisions; DROP TABLE device_executions; DROP TABLE device_command_changes; DROP TABLE device_command_events; DROP TABLE device_commands; DROP TABLE channel_media_frames; DROP TABLE channel_media; DROP TABLE device_reports; DROP TABLE devices; DROP TABLE archive_memory_suppressions; DROP TABLE archive_memory_grants; DROP TABLE briefing_relations; DROP TABLE conversation_memory_effects; DROP TABLE briefing_memory_suppressions; DROP TABLE briefing_disclosures; DROP TABLE briefing_runs; DROP TABLE briefing_batches; DROP TABLE briefing_manifests; DROP TABLE briefing_grants; PRAGMA user_version=21;").unwrap();
    drop(s);
    let s = Store::open(&dir.path().join("state.db")).unwrap();
    assert_eq!(s.memory(m.id).unwrap(), m);
    assert_eq!(
        s.connection
            .query_row(
                "SELECT snapshot FROM lain_memories WHERE id=?1",
                [m.id.to_string()],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        before
    );
    assert!(s.archive_memory_grant(b.id).unwrap().scope.is_none());
}

#[test]
fn unavailable_source_body_is_not_needed_for_explicit_private_correction_or_forget() {
    for forget in [false, true] {
        let (_dir, mut s, b, source, record) = setup(10);
        let m = remember(&mut s, &b, record);
        s.archive_set_grant(
            source,
            SetArchiveGrant {
                expected_revision: 1,
                scope: None,
            },
            now_ms().unwrap(),
        )
        .unwrap();
        let (mut t, proof) = turn(&mut s, &b, "更正或忘掉这条测试认识");
        let context = s.private_memory_context(t.id, &m.id.to_string()).unwrap();
        assert!(context["records"].as_array().unwrap().is_empty());
        assert_eq!(context["withheld_targets"][0]["revision"], 1);
        assert!(
            !serde_json::to_string(&context)
                .unwrap()
                .contains(&m.draft.content)
        );
        let tool = if forget {
            Tool::MemoryForget(ForgetMemory {
                memory_id: m.id,
                expected_revision: 1,
                evidence: proof,
            })
        } else {
            Tool::MemoryCorrect(CorrectMemory {
                memory_id: m.id,
                expected_revision: 1,
                selection: SelectMemory {
                    content: "只依据当前用户纠正的新认识".into(),
                    kind: MemoryKind::Preference,
                    evidence: proof,
                },
            })
        };
        let action = action(&mut s, &mut t, tool);
        let effect = s.private_memory_effect(t.id, action).unwrap();
        assert!(s.memory(m.id).is_err());
        if !forget {
            assert_eq!(
                s.memory(effect.memory_id).unwrap().draft.content,
                "只依据当前用户纠正的新认识"
            );
        }
        assert!(s.private_chat_memory_current(t.id).unwrap());
    }
}

#[test]
fn invalid_scope_or_interpretation_never_spends_or_persists() {
    let (_dir, mut s, b, _, _) = setup(10);
    let grant = s.archive_memory_grant(b.id).unwrap();
    for case in 0..6 {
        let mut scope = grant.scope.clone().unwrap();
        match case {
            0 => scope.max_writes = 0,
            1 => scope.retention_ms = 0,
            2 => scope.expires_at_ms = 0,
            3 => scope.sources.push(scope.sources[0].clone()),
            4 => scope.sources[0].grant_revision += 1,
            _ => scope.expires_at_ms += 1,
        }
        assert!(
            s.archive_memory_set(
                b.id,
                SetArchiveMemory {
                    expected_revision: 1,
                    scope: Some(scope)
                }
            )
            .is_err()
        );
        assert_eq!(s.archive_memory_grant(b.id).unwrap().revision, 1);
    }
    for (content, kind) in [
        (" ".into(), MemoryKind::Preference),
        ("x".repeat(1025), MemoryKind::Preference),
        ("valid content".into(), MemoryKind::Reflection),
        ("line\nbreak".into(), MemoryKind::Preference),
    ] {
        let (_dir, mut s, b, _, record) = setup(10);
        let (mut t, _) = turn(&mut s, &b, "测试无效选择");
        inspect(&mut s, &mut t, record);
        let id = action(
            &mut s,
            &mut t,
            Tool::ArchiveRemember(RememberArchive { content, kind }),
        );
        assert!(s.private_memory_effect(t.id, id).is_err());
        assert_eq!(s.archive_memory_grant(b.id).unwrap().used_writes, 0);
        assert_eq!(
            s.connection
                .query_row("SELECT count(*) FROM lain_memories", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    assert_eq!(s.archive_memory_grant(b.id).unwrap().used_writes, 0);
    assert_eq!(
        s.connection
            .query_row("SELECT count(*) FROM lain_memories", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
