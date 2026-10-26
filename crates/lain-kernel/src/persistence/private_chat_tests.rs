use super::*;
use crate::actor::domain::Action;
use crate::memory::*;
use lain_wire::source::{InspectSource, SetSourceGrant, SourceWindow};

fn setup(text: &str) -> (tempfile::TempDir, Store, Binding, Batch, ActorTurn) {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("chat.db")).unwrap();
    let b = s
        .channel_bind(
            BindingSpec {
                name: "chat".into(),
                adapter: "fixture".into(),
                account: "1001".into(),
                peer: "2002".into(),
            },
            true,
        )
        .unwrap();
    let i = receive(&mut s, &b, text);
    let batch = s
        .channel_next_batch(b.id, i.received_at_ms + MAX_MERGE_MS)
        .unwrap()
        .unwrap();
    let mut t = ActorTurn {
        reading_activity: None,
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
        usage_tokens: 7,
        timing: Default::default(),
        channel_memory: None,
        channel_source: None,
        private_chat: Some(s.private_chat_context(b.id, batch.id).unwrap()),
    };
    t.message(MessageRole::User, text.into()).unwrap();
    s.save_actor_turn(&mut t, true).unwrap();
    s.channel_link(batch.id, &t).unwrap();
    (dir, s, b, batch, t)
}
fn receive(s: &mut Store, b: &Binding, text: &str) -> Incoming {
    s.channel_receive(
        b.id,
        Input {
            account: b.spec.account.clone(),
            peer: b.spec.peer.clone(),
            external_id: uuid::Uuid::new_v4().to_string(),
            text: text.into(),
        },
    )
    .unwrap()
}
fn action(s: &mut Store, t: &mut ActorTurn, tool: Tool) -> ActionId {
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
fn finish_args(kind: ChatInputKind) -> FinishChat {
    FinishChat {
        input_kind: kind,
        reason: SilenceReason::BriefAcknowledgement,
    }
}
fn complete(s: &mut Store, t: &mut ActorTurn, batch: &Batch, decision: ChatDecision) {
    t.message(MessageRole::Assistant, "recorded draft".into())
        .unwrap();
    t.state = TurnState::Completed;
    t.private_chat.as_mut().unwrap().decision = decision;
    s.save_actor_turn(t, false).unwrap();
    s.channel_finish(batch.id, t).unwrap();
}

#[test]
fn quiet_window_extends_only_live_windows_and_has_a_hard_cap() {
    assert_eq!(merge_deadline([]), None);
    assert_eq!(merge_deadline([100]), Some(850));
    assert_eq!(merge_deadline([100, 800]), Some(1550));
    assert_eq!(merge_deadline([100, 851, 900]), Some(850));
    assert_eq!(
        merge_deadline([100, 700, 1300, 1900, 2500, 3000]),
        Some(3100)
    );
    assert_eq!(merge_deadline([100, 90]), Some(850));
    assert_eq!(merge_deadline([u64::MAX - 1]), Some(u64::MAX));
}

#[test]
fn supplements_are_atomic_originals_stay_immutable_and_budgets_do_not_reset() {
    let (_dir, mut s, b, batch, t) = setup("original question?");
    let supplement = receive(&mut s, &b, "correction: second choice");
    assert!(s.private_chat_check(&s.actor_turn(t.id).unwrap()).is_err());
    assert!(matches!(
        s.private_chat_refresh(t.id, supplement.received_at_ms)
            .unwrap(),
        ChatRefresh::WaitUntil(_)
    ));
    let due = supplement.received_at_ms + MAX_MERGE_MS;
    s.connection.execute_batch("CREATE TRIGGER refuse_chat BEFORE UPDATE ON lain_actor_turns BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    assert!(s.private_chat_refresh(t.id, due).is_err());
    let snap = s.channel_snapshot(b.id).unwrap();
    assert_eq!(snap.incoming[1].state, InputState::Queued);
    assert_eq!(s.actor_turn(t.id).unwrap().messages.len(), 1);
    s.connection
        .execute_batch("DROP TRIGGER refuse_chat;")
        .unwrap();
    assert!(matches!(
        s.private_chat_refresh(t.id, due).unwrap(),
        ChatRefresh::Updated
    ));
    let saved = s.actor_turn(t.id).unwrap();
    assert_eq!(saved.messages[0].content, "original question?");
    assert_eq!(saved.messages[1].content, "correction: second choice");
    assert_eq!(saved.request_id, batch.id.0);
    assert_eq!(saved.usage_tokens, 7);
    assert_eq!(saved.model_requests, 1);
    assert_eq!(saved.private_chat.as_ref().unwrap().reconsiderations, 1);
    let snap = s.channel_snapshot(b.id).unwrap();
    assert_eq!(snap.batches[0].text, "original question?");
    assert_eq!(snap.incoming[1].batch_id, Some(batch.id));
    assert_eq!(snap.incoming[1].id, supplement.id);
    let next = receive(&mut s, &b, "another correction");
    assert!(matches!(
        s.private_chat_refresh(t.id, next.received_at_ms + MAX_MERGE_MS)
            .unwrap(),
        ChatRefresh::Updated
    ));
    let final_input = receive(&mut s, &b, "must still be handled");
    assert!(matches!(
        s.private_chat_refresh(t.id, final_input.received_at_ms + MAX_MERGE_MS)
            .unwrap(),
        ChatRefresh::Superseded
    ));
    let saved = s.actor_turn(t.id).unwrap();
    assert_eq!(saved.private_chat.unwrap().reconsiderations, 2);
    assert_eq!(
        s.channel_snapshot(b.id)
            .unwrap()
            .incoming
            .last()
            .unwrap()
            .state,
        InputState::Queued
    );
}

#[test]
fn silent_finish_requires_social_interpretation_and_cannot_discard_explicit_requests() {
    for text in [
        "怎么配置",
        "please check",
        "任务失败",
        "把灯关了",
        "approval?",
        "a".repeat(65).as_str(),
    ] {
        let (_dir, mut s, _b, _batch, mut t) = setup(text);
        let args = finish_args(ChatInputKind::Social);
        let a = action(&mut s, &mut t, Tool::ChatFinish(args.clone()));
        assert!(s.private_chat_finish(t.id, a, args).is_err(), "{text}");
        assert_eq!(s.actor_turn(t.id).unwrap().state, TurnState::Running);
    }
    for kind in [
        ChatInputKind::Question,
        ChatInputKind::Request,
        ChatInputKind::Work,
        ChatInputKind::Approval,
        ChatInputKind::Fault,
    ] {
        let (_dir, mut s, _b, _batch, mut t) = setup("ok");
        let args = finish_args(kind);
        let a = action(&mut s, &mut t, Tool::ChatFinish(args.clone()));
        assert!(s.private_chat_finish(t.id, a, args).is_err());
    }
    let (_dir, mut s, b, batch, mut t) = setup("晚安");
    let args = finish_args(ChatInputKind::Social);
    let a = action(&mut s, &mut t, Tool::ChatFinish(args.clone()));
    let receipt = s.private_chat_finish(t.id, a, args).unwrap();
    assert_eq!(receipt["interpretation"], "model_inferred");
    s.channel_finish(batch.id, &s.actor_turn(t.id).unwrap())
        .unwrap();
    assert!(s.channel_snapshot(b.id).unwrap().deliveries.is_empty());
    let next = receive(&mut s, &b, "new question?");
    assert!(
        s.channel_next_batch(b.id, next.received_at_ms + MAX_MERGE_MS)
            .unwrap()
            .is_some()
    );
}

#[test]
fn wait_is_once_per_turn_wakes_on_new_input_and_restart_is_unknown() {
    let (dir, mut s, b, _batch, mut t) = setup("one moment");
    let a = action(&mut s, &mut t, Tool::ChatWait(WaitChat { seconds: 5 }));
    assert!(s.private_chat_wait(t.id, a, 0).is_err());
    assert!(s.private_chat_wait(t.id, a, 6).is_err());
    let due = s.private_chat_wait(t.id, a, 5).unwrap();
    assert!(due <= now_ms().unwrap() + 5000);
    assert!(s.private_chat_wait_done(t.id, a).is_err());
    let i = receive(&mut s, &b, "actually continue");
    let receipt = s.private_chat_wait_done(t.id, a).unwrap();
    assert_eq!(receipt["wake"], "new_input");
    assert_eq!(receipt["incoming_ids"][0], i.id.to_string());
    assert!(s.private_chat_wait(t.id, a, 1).is_err());
    drop(s);
    let mut s = Store::open(&dir.path().join("chat.db")).unwrap();
    s.recover_actor().unwrap();
    let saved = s.actor_turn(t.id).unwrap();
    assert_eq!(saved.state, TurnState::Unknown);
    let c = saved.private_chat.unwrap();
    assert_eq!(c.decision, ChatDecision::Interrupted);
    assert_eq!(c.waits, 1);
    assert!(c.wait_until_ms.is_none());
}

#[test]
fn new_input_invalidates_ready_and_prewrite_drafts_but_preserves_transport_truth() {
    for outcome in [
        None,
        Some(SendOutcome::NotSent),
        Some(SendOutcome::Confirmed("echo".into())),
        Some(SendOutcome::Unknown),
    ] {
        let (_dir, mut s, b, batch, mut t) = setup("hello");
        complete(&mut s, &mut t, &batch, ChatDecision::Reply);
        let claimed = if outcome.is_some() {
            let d = s.channel_claim_delivery(b.id).unwrap().unwrap();
            assert!(s.channel_may_send(&d).unwrap());
            Some(d)
        } else {
            None
        };
        let expected = match outcome {
            None => DeliveryState::Superseded,
            Some(SendOutcome::NotSent) => DeliveryState::NotSent,
            Some(SendOutcome::Confirmed(_)) => DeliveryState::Confirmed,
            Some(SendOutcome::Unknown) => DeliveryState::Unknown,
            _ => unreachable!(),
        };
        if let Some(d) = &claimed
            && (expected == DeliveryState::Confirmed || expected == DeliveryState::Unknown)
        {
            s.channel_settle(d.id, outcome.clone().unwrap()).unwrap();
        }
        let incoming = receive(&mut s, &b, "wait, correction");
        if let Some(d) = &claimed {
            assert!(!s.channel_may_send(d).unwrap());
            if expected == DeliveryState::NotSent {
                s.channel_settle(d.id, SendOutcome::NotSent).unwrap();
            }
        }
        let snap = s.channel_snapshot(b.id).unwrap();
        assert_eq!(snap.deliveries[0].state, expected);
        // Duplicate notifications must not create new inputs or mutate send truth.
        assert_eq!(
            s.channel_receive(b.id, incoming.input).unwrap().id,
            incoming.id
        );
        assert_eq!(
            s.channel_snapshot(b.id).unwrap().deliveries[0].state,
            expected
        );
        assert!(s.channel_claim_delivery(b.id).unwrap().is_none());
    }
}

#[test]
fn failure_and_approval_notices_are_not_silently_superseded() {
    for approval in [false, true] {
        let (_dir, mut s, b, batch, mut t) = setup("check work");
        if approval {
            let id = action(
                &mut s,
                &mut t,
                Tool::WorkCancel(crate::actor::tools::SessionArg {
                    session_id: crate::domain::WorkSessionId::new(),
                }),
            );
            t.actions.iter_mut().find(|a| a.id == id).unwrap().state = ActionState::Proposed;
            t.state = TurnState::PendingConfirmation;
            s.save_actor_turn(&mut t, false).unwrap();
            s.channel_finish(batch.id, &t).unwrap();
        } else {
            complete(&mut s, &mut t, &batch, ChatDecision::Notice);
        }
        receive(&mut s, &b, "one more thing");
        let d = s.channel_claim_delivery(b.id).unwrap().unwrap();
        assert_eq!(
            d.purpose,
            if approval {
                DeliveryPurpose::Approval
            } else {
                DeliveryPurpose::Failure
            }
        );
        assert!(s.channel_may_send(&d).unwrap());
    }
}

#[test]
fn selected_memory_is_rechecked_for_generation_and_delivery() {
    for change in ["delete", "revoke", "correct"] {
        let (_dir, mut s, b, batch, mut t) = setup("lemon");
        let g = s
            .set_memory_grant(
                b.id,
                SetMemoryGrant {
                    enabled: true,
                    expected_revision: 0,
                    expires_at_ms: None,
                },
            )
            .unwrap();
        t.channel_memory = s.channel_memory_context(b.id, batch.id).unwrap();
        s.save_actor_turn(&mut t, false).unwrap();
        let record = s
            .add_memory(&AddMemory {
                request_id: lain_wire::RequestId::new(),
                draft: MemoryDraft {
                    content: "lemon preference".into(),
                    kind: MemoryKind::Preference,
                    scope: MemoryScope::Channel { channel_id: b.id },
                },
            })
            .unwrap();
        assert_eq!(
            s.private_memory_context(t.id, "lemon").unwrap()["records"][0]["id"],
            json!(record.id)
        );
        t = s.actor_turn(t.id).unwrap();
        assert_eq!(
            t.private_chat.as_ref().unwrap().memory_versions[0].memory_id,
            record.id
        );
        assert!(s.private_chat_check(&t).is_ok());
        complete(&mut s, &mut t, &batch, ChatDecision::Reply);
        match change {
            "delete" => s.delete_memory(record.id, record.revision).unwrap(),
            "revoke" => {
                s.set_memory_grant(
                    b.id,
                    SetMemoryGrant {
                        enabled: false,
                        expected_revision: g.revision,
                        expires_at_ms: None,
                    },
                )
                .unwrap();
            }
            "correct" => {
                let replacement = s
                    .add_memory(&AddMemory {
                        request_id: lain_wire::RequestId::new(),
                        draft: MemoryDraft {
                            content: "water preference".into(),
                            kind: MemoryKind::Preference,
                            scope: MemoryScope::Channel { channel_id: b.id },
                        },
                    })
                    .unwrap();
                s.change_memory(
                    record.id,
                    &ChangeMemory {
                        expected_revision: record.revision,
                        status: MemoryStatus::Superseded,
                        superseded_by: Some(replacement.id),
                    },
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        assert!(!s.private_chat_memory_current(t.id).unwrap());
        assert!(
            s.private_chat_check(&t)
                .unwrap_err()
                .is::<ChangedChatMemory>()
        );
        assert!(s.channel_claim_delivery(b.id).unwrap().is_none());
        assert_eq!(
            s.channel_snapshot(b.id).unwrap().deliveries[0].state,
            DeliveryState::Superseded
        );
    }
}

#[test]
fn schema_ten_upgrade_adds_only_the_input_index_and_preserves_old_actor_snapshots() {
    let (dir, mut s, b, _batch, mut t) = setup("legacy question?");
    t.private_chat = None;
    s.save_actor_turn(&mut t, false).unwrap();
    let before: String = s
        .connection
        .query_row(
            "SELECT snapshot FROM lain_actor_turns WHERE id=?1",
            [t.id.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    s.connection
        .execute_batch("DROP TABLE archive_edits; DROP TABLE archive_records; DROP TABLE archive_grants; DROP TABLE archive_sources; DROP TABLE inference_requests; DROP TABLE inference_decisions; DROP TABLE inference_budgets; DROP TABLE share_reads; DROP TABLE share_model_requests; DROP TABLE share_jobs; DROP TABLE share_deliveries; DROP TABLE share_candidates; DROP TABLE channel_attention; DROP TABLE share_grants; DROP TABLE sharing_runtime; DROP TABLE reading_source_decisions; DROP TABLE experience_edits; DROP TABLE reading_source_suppressions; DROP TABLE experience_reads; DROP TABLE reading_model_requests; DROP TABLE reading_jobs; DROP TABLE reading_experiences; DROP TABLE reading_withdrawals; DROP TABLE reading_results; DROP TABLE reading_activities; DROP TABLE reading_grants; DROP INDEX channel_incoming_batch; PRAGMA user_version=10;")
        .unwrap();
    drop(s);
    let s = Store::open(&dir.path().join("chat.db")).unwrap();
    let after: String = s
        .connection
        .query_row(
            "SELECT snapshot FROM lain_actor_turns WHERE id=?1",
            [t.id.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(before, after);
    assert!(s.actor_turn(t.id).unwrap().private_chat.is_none());
    assert_eq!(s.channel_snapshot(b.id).unwrap().incoming.len(), 1);
    assert_eq!(
        s.connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        crate::persistence::SCHEMA_VERSION
    );
    assert!(
        s.connection
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='index' AND name='channel_incoming_batch'",
                [],
                |r| r.get::<_, u8>(0)
            )
            .is_ok()
    );
}

#[test]
fn later_private_history_uses_confirmed_speech_and_keeps_unsent_snapshots_only_for_inspection() {
    for state in [
        DeliveryState::Ready,
        DeliveryState::Confirmed,
        DeliveryState::Unknown,
        DeliveryState::Superseded,
    ] {
        let (_dir, mut s, b, batch, mut previous) = setup("previous question?");
        complete(&mut s, &mut previous, &batch, ChatDecision::Reply);
        if matches!(state, DeliveryState::Confirmed | DeliveryState::Unknown) {
            let d = s.channel_claim_delivery(b.id).unwrap().unwrap();
            let result = if state == DeliveryState::Confirmed {
                SendOutcome::Confirmed("echo".into())
            } else {
                SendOutcome::Unknown
            };
            s.channel_settle(d.id, result).unwrap();
        }
        let incoming = receive(&mut s, &b, "next question?");
        if state == DeliveryState::Ready {
            // Retain a Ready row to test history independently of receive-time invalidation.
            let mut d = s.channel_snapshot(b.id).unwrap().deliveries[0].clone();
            d.state = DeliveryState::Ready;
            super::super::channels::save_delivery(&s.connection, &d).unwrap();
        }
        let next = s
            .channel_next_batch(b.id, incoming.received_at_ms + MAX_MERGE_MS)
            .unwrap()
            .unwrap();
        let mut current = previous.clone();
        current.id = ActorTurnId::new();
        current.request_id = next.id.0;
        current.sequence = 0;
        current.revision = 0;
        current.state = TurnState::Running;
        current.messages.clear();
        current.message(MessageRole::User, next.text).unwrap();
        current.private_chat = Some(s.private_chat_context(b.id, next.id).unwrap());
        s.save_actor_turn(&mut current, true).unwrap();
        let history = s.private_chat_history(current.id).unwrap();
        let past = history.iter().find(|t| t.id == previous.id).unwrap();
        assert_eq!(
            past.messages
                .iter()
                .any(|m| m.role == MessageRole::Assistant),
            state == DeliveryState::Confirmed
        );
        assert!(
            s.actor_turn(previous.id)
                .unwrap()
                .messages
                .iter()
                .any(|m| m.content == "recorded draft")
        );
    }
}

#[test]
fn source_grant_or_suppression_changes_invalidate_ready_and_prewrite_quotes_not_confirmed_sends() {
    for change in ["revoke", "suppress", "expire", "prewrite", "confirmed"] {
        let (_dir, mut s, b, seed_batch, mut seed) = setup("old source phrase");
        let seed_input = s.channel_snapshot(b.id).unwrap().incoming[0].id;
        complete(&mut s, &mut seed, &seed_batch, ChatDecision::Reply);
        let old = s.channel_claim_delivery(b.id).unwrap().unwrap();
        s.channel_settle(old.id, SendOutcome::Confirmed("old-echo".into()))
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let end = now_ms().unwrap();
        let grant = s
            .set_source_grant(
                b.id,
                SetSourceGrant {
                    enabled: true,
                    expected_revision: 0,
                    window: Some(SourceWindow {
                        from_received_at_ms: end - 60000,
                        to_received_at_ms: end,
                    }),
                    expires_at_ms: (change == "expire").then_some(end + 1500),
                },
            )
            .unwrap();
        let input = receive(&mut s, &b, "inspect historical source?");
        let batch = s
            .channel_next_batch(b.id, input.received_at_ms + MAX_MERGE_MS)
            .unwrap()
            .unwrap();
        let mut t = seed.clone();
        t.id = ActorTurnId::new();
        t.sequence = 0;
        t.revision = 0;
        t.request_id = batch.id.0;
        t.state = TurnState::Running;
        t.messages.clear();
        t.message(MessageRole::User, batch.text.clone()).unwrap();
        t.private_chat = Some(s.private_chat_context(b.id, batch.id).unwrap());
        t.channel_source = s.source_context(b.id, batch.id).unwrap();
        s.save_actor_turn(&mut t, true).unwrap();
        s.channel_link(batch.id, &t).unwrap();
        let a = action(
            &mut s,
            &mut t,
            Tool::SourceInspect(InspectSource {
                incoming_id: seed_input,
                window: None,
            }),
        );
        let redactor = crate::redaction::Redactor::new(vec![]);
        let (selection, receipt) = s.source_actor_lookup(t.id, a, &redactor).unwrap();
        t = s.actor_turn(t.id).unwrap();
        t.actions[0].state = ActionState::Executed;
        t.actions[0].result = Some(receipt);
        s.save_actor_turn(&mut t, false).unwrap();
        if change == "revoke" {
            let reads: i64 = s
                .connection
                .query_row("SELECT count(*) FROM channel_source_reads", [], |r| {
                    r.get(0)
                })
                .unwrap();
            s.connection.execute_batch("CREATE TRIGGER refuse_prepared BEFORE UPDATE ON lain_actor_turns BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
            assert!(
                s.source_actor_disclosure(
                    t.id,
                    std::slice::from_ref(&selection),
                    Some(1),
                    &redactor
                )
                .is_err()
            );
            assert!(
                s.actor_turn(t.id)
                    .unwrap()
                    .private_chat
                    .unwrap()
                    .source_versions
                    .is_empty()
            );
            assert_eq!(
                s.connection
                    .query_row("SELECT count(*) FROM channel_source_reads", [], |r| r
                        .get::<_, i64>(0))
                    .unwrap(),
                reads
            );
            s.connection
                .execute_batch("DROP TRIGGER refuse_prepared;")
                .unwrap();
        }
        s.source_actor_disclosure(t.id, &[selection], Some(1), &redactor)
            .unwrap();
        t = s.actor_turn(t.id).unwrap();
        assert_eq!(t.private_chat.as_ref().unwrap().source_versions.len(), 1);
        assert!(s.private_chat_check(&t).is_ok());
        complete(&mut s, &mut t, &batch, ChatDecision::Reply);
        let claimed = if matches!(change, "prewrite" | "confirmed") {
            let d = s.channel_claim_delivery(b.id).unwrap().unwrap();
            assert!(s.channel_may_send(&d).unwrap());
            if change == "confirmed" {
                s.channel_settle(d.id, SendOutcome::Confirmed("new-echo".into()))
                    .unwrap();
            }
            Some(d)
        } else {
            None
        };
        match change {
            "suppress" => {
                s.connection
                    .execute(
                        "INSERT INTO channel_memory_suppressions VALUES(?1,?2)",
                        params![b.id.to_string(), seed_input.to_string()],
                    )
                    .unwrap();
            }
            "expire" => {
                std::thread::sleep(std::time::Duration::from_millis(
                    (end + 1501).saturating_sub(now_ms().unwrap()),
                ));
            }
            _ => {
                s.set_source_grant(
                    b.id,
                    SetSourceGrant {
                        enabled: false,
                        expected_revision: grant.revision,
                        window: None,
                        expires_at_ms: None,
                    },
                )
                .unwrap();
            }
        }
        assert!(
            s.private_chat_check(&t)
                .unwrap_err()
                .is::<ChangedChatMemory>()
        );
        if let Some(d) = claimed {
            assert!(!s.channel_may_send(&d).unwrap());
            if change == "prewrite" {
                s.channel_settle(d.id, SendOutcome::NotSent).unwrap();
            }
        }
        assert!(s.channel_claim_delivery(b.id).unwrap().is_none());
        let snapshot = s.channel_snapshot(b.id).unwrap();
        assert_eq!(snapshot.deliveries[0].state, DeliveryState::Confirmed);
        assert_eq!(
            snapshot.deliveries[1].state,
            match change {
                "confirmed" => DeliveryState::Confirmed,
                "prewrite" => DeliveryState::NotSent,
                _ => DeliveryState::Superseded,
            }
        );
    }
}
