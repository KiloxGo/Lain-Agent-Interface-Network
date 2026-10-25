use super::*;
use crate::{
    actor::domain::Action,
    channels::domain::{BindingSpec, Input, MERGE_MS, SendOutcome},
};
use std::time::Duration;

fn bind(s: &mut Store, peer: &str) -> Binding {
    s.channel_bind(
        BindingSpec {
            name: peer.into(),
            adapter: "fixture".into(),
            account: "1001".into(),
            peer: peer.into(),
        },
        true,
    )
    .unwrap()
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
fn enable(s: &mut Store, b: &Binding) -> SourceGrant {
    std::thread::sleep(Duration::from_millis(2));
    let end = now_ms().unwrap();
    s.set_source_grant(
        b.id,
        SetSourceGrant {
            enabled: true,
            expected_revision: s.source_grant(b.id).unwrap().revision,
            window: Some(SourceWindow {
                from_received_at_ms: end - 60000,
                to_received_at_ms: end,
            }),
            expires_at_ms: None,
        },
    )
    .unwrap()
}
fn query(text: Option<&str>, limit: u16) -> SourceQuery {
    SourceQuery {
        text: text.map(str::to_owned),
        window: None,
        after: None,
        limit,
    }
}

#[test]
fn newly_configured_credentials_are_checked_in_full_sources_before_excerpting() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let b = bind(&mut s, "2002");
    let credential = "opaque-configured-value-1234567890";
    let input = receive(
        &mut s,
        &b,
        &format!("rotated {}{credential}", "x".repeat(1005)),
    );
    enable(&mut s, &b);
    // Simulate a credential now known to the request owner, without mutating process environment.
    let redactor = Redactor::new(vec![credential.into()]);
    assert!(
        s.source_inspect(
            b.id,
            InspectSource {
                incoming_id: input.id,
                window: None
            },
            &redactor
        )
        .is_err()
    );
    assert!(
        s.source_search(b.id, query(Some("rotated"), 3), &redactor)
            .unwrap()
            .records
            .is_empty()
    );
    assert!(
        s.source_search(b.id, query(Some(credential), 3), &redactor)
            .is_err()
    );
}

#[test]
fn source_cursor_handles_receipt_ties_clock_rollback_and_late_backdated_inputs() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let b = bind(&mut s, "2002");
    let first = receive(&mut s, &b, "first committed");
    let second = receive(&mut s, &b, "second committed");
    let now = now_ms().unwrap();
    s.connection.execute("UPDATE channel_incoming SET snapshot=json_set(snapshot,'$.received_at_ms',?2) WHERE id=?1",params![first.id.to_string(),(now-10) as i64]).unwrap();
    s.connection.execute("UPDATE channel_incoming SET snapshot=json_set(snapshot,'$.received_at_ms',?2) WHERE id=?1",params![second.id.to_string(),(now-30) as i64]).unwrap();
    enable(&mut s, &b);
    let page = s
        .source_search(b.id, query(None, 1), &Redactor::new(vec![]))
        .unwrap();
    assert_eq!(page.records[0].incoming_id, second.id);
    let late = receive(&mut s, &b, "late outside captured incoming high-water");
    s.connection.execute("UPDATE channel_incoming SET snapshot=json_set(snapshot,'$.received_at_ms',?2) WHERE id=?1",params![late.id.to_string(),(now-20) as i64]).unwrap();
    let mut q = query(None, 1);
    q.after = Some(page.next_cursor);
    let page = s.source_search(b.id, q, &Redactor::new(vec![])).unwrap();
    assert_eq!(page.records[0].incoming_id, first.id);
    assert_eq!(page.examined, 2);
    assert!(!page.has_more);
    assert!(
        s.source_inspect(
            b.id,
            InspectSource {
                incoming_id: late.id,
                window: None
            },
            &Redactor::new(vec![])
        )
        .is_err()
    );
}
fn inspect(s: &mut Store, b: &Binding, id: IncomingId) -> Result<SourceRecord> {
    s.source_inspect(
        b.id,
        InspectSource {
            incoming_id: id,
            window: None,
        },
        &Redactor::new(vec![]),
    )
}
fn running(s: &mut Store, b: &Binding) -> ActorTurn {
    let input = receive(s, b, "current lookup request");
    let batch = s
        .channel_next_batch(b.id, input.received_at_ms + MERGE_MS + 1)
        .unwrap()
        .unwrap();
    let mut turn = ActorTurn {
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
        channel_memory: None,
        channel_source: s.source_context(b.id, batch.id).unwrap(),
        private_chat: None,
    };
    turn.message(MessageRole::User, batch.text).unwrap();
    s.save_actor_turn(&mut turn, true).unwrap();
    s.channel_link(batch.id, &turn).unwrap();
    turn
}
fn lookup(s: &mut Store, turn: &mut ActorTurn, tool: Tool) -> (SourceSelection, Value) {
    let id = ActionId::new();
    turn.actions.push(Action {
        id,
        call_id: id.to_string(),
        tool,
        state: ActionState::Executing,
        result: None,
    });
    s.save_actor_turn(turn, false).unwrap();
    s.source_actor_lookup(turn.id, id, &Redactor::new(vec![]))
        .unwrap()
}

#[test]
fn source_grants_default_off_revision_persistence_expiry_and_no_memory_expansion() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.db");
    let mut s = Store::open(&path).unwrap();
    let b = bind(&mut s, "2002");
    let old = receive(&mut s, &b, "old statement");
    assert_eq!(s.source_grant(b.id).unwrap().revision, 0);
    assert!(
        s.source_search(b.id, query(None, 1), &Redactor::new(vec![]))
            .is_err()
    );
    let g = enable(&mut s, &b);
    assert!(!s.memory_grant(b.id).unwrap().enabled);
    assert!(
        s.set_source_grant(
            b.id,
            SetSourceGrant {
                enabled: false,
                expected_revision: 0,
                window: None,
                expires_at_ms: None
            }
        )
        .is_err()
    );
    assert_eq!(
        inspect(&mut s, &b, old.id).unwrap().incoming.text,
        "old statement"
    );
    let late = receive(&mut s, &b, "new collection is not authorized by old grant");
    assert!(inspect(&mut s, &b, late.id).is_err());
    drop(s);
    let mut s = Store::open(&path).unwrap();
    assert_eq!(json!(s.source_grant(b.id).unwrap()), json!(g));
    assert_eq!(inspect(&mut s, &b, old.id).unwrap().incoming_id, old.id);
    let disabled = s
        .set_source_grant(
            b.id,
            SetSourceGrant {
                enabled: false,
                expected_revision: g.revision,
                window: None,
                expires_at_ms: None,
            },
        )
        .unwrap();
    assert!(inspect(&mut s, &b, old.id).is_err());
    let g = s
        .set_source_grant(
            b.id,
            SetSourceGrant {
                enabled: true,
                expected_revision: disabled.revision,
                window: g.window,
                expires_at_ms: Some(now_ms().unwrap() + 30),
            },
        )
        .unwrap();
    assert!(g.active(now_ms().unwrap()));
    std::thread::sleep(Duration::from_millis(35));
    assert!(inspect(&mut s, &b, old.id).is_err());
    assert_eq!(s.source_grants(b.id).unwrap().len(), 3);
    let end = now_ms().unwrap();
    for w in [
        SourceWindow {
            from_received_at_ms: end,
            to_received_at_ms: end,
        },
        SourceWindow {
            from_received_at_ms: 0,
            to_received_at_ms: MAX_WINDOW_MS + 1,
        },
        SourceWindow {
            from_received_at_ms: end,
            to_received_at_ms: end + 10000,
        },
    ] {
        assert!(
            s.set_source_grant(
                b.id,
                SetSourceGrant {
                    enabled: true,
                    expected_revision: g.revision,
                    window: Some(w),
                    expires_at_ms: None
                }
            )
            .is_err()
        );
    }
}

#[test]
fn source_window_cursor_binding_suppression_and_sensitive_boundaries() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let b = bind(&mut s, "2002");
    let foreign = bind(&mut s, "3003");
    let first = receive(&mut s, &b, "needle first");
    std::thread::sleep(Duration::from_millis(2));
    let second = receive(&mut s, &b, "needle second");
    let other = receive(&mut s, &foreign, "needle other");
    let secret = receive(&mut s, &b, "sk-fixtureSourceSensitive1234567890");
    let g = enable(&mut s, &b);
    enable(&mut s, &foreign);
    let page = s
        .source_search(b.id, query(Some("needle"), 1), &Redactor::new(vec![]))
        .unwrap();
    assert_eq!(page.records[0].incoming_id, first.id);
    assert!(page.has_more);
    let mut q = query(Some("needle"), 1);
    q.after = Some(page.next_cursor.clone());
    assert_eq!(
        s.source_search(b.id, q.clone(), &Redactor::new(vec![]))
            .unwrap()
            .records[0]
            .incoming_id,
        second.id
    );
    assert!(
        s.source_search(foreign.id, q.clone(), &Redactor::new(vec![]))
            .is_err()
    );
    let mut changed = q.clone();
    changed.text = Some("other".into());
    assert!(
        s.source_search(b.id, changed, &Redactor::new(vec![]))
            .is_err()
    );
    let mut changed = q.clone();
    changed.after.as_mut().unwrap().through_sequence += 1;
    assert!(
        s.source_search(b.id, changed, &Redactor::new(vec![]))
            .is_err()
    );
    assert!(inspect(&mut s, &b, other.id).is_err());
    assert!(inspect(&mut s, &b, secret.id).is_err());
    let mut narrow = query(None, 3);
    narrow.window = Some(SourceWindow {
        from_received_at_ms: second.received_at_ms,
        to_received_at_ms: g.window.unwrap().to_received_at_ms,
    });
    let page = s
        .source_search(b.id, narrow, &Redactor::new(vec![]))
        .unwrap();
    assert_eq!(page.records.len(), 1);
    assert_eq!(page.records[0].incoming_id, second.id);
    let mut broad = query(None, 3);
    broad.window = Some(SourceWindow {
        from_received_at_ms: 0,
        ..g.window.unwrap()
    });
    assert!(
        s.source_search(b.id, broad, &Redactor::new(vec![]))
            .is_err()
    );
    s.connection
        .execute(
            "INSERT INTO channel_memory_suppressions(channel_id,incoming_id) VALUES(?1,?2)",
            params![b.id.to_string(), first.id.to_string()],
        )
        .unwrap();
    assert!(inspect(&mut s, &b, first.id).is_err());
    let reads = s.source_reads(b.id, 0).unwrap().to_string();
    assert!(!reads.contains("needle") && !reads.contains("Sensitive"));
    let no_match = s
        .source_search(b.id, query(Some("absent"), 3), &Redactor::new(vec![]))
        .unwrap();
    assert!(no_match.records.is_empty());
    assert_eq!(no_match.coverage, SourceCoverage::PartialNoBackfill);
    let newer = enable(&mut s, &b);
    assert!(newer.revision > g.revision);
    assert!(s.source_search(b.id, q, &Redactor::new(vec![])).is_err());
}

#[test]
fn source_scan_count_bytes_utf8_and_stable_cursor_are_bounded() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let b = bind(&mut s, "2002");
    for n in 0..514 {
        receive(&mut s, &b, &format!("item {n} {}", "中\n".repeat(600)));
        if n % 64 == 63 {
            let revision = s.channel_binding(b.id).unwrap().grant_revision;
            let off = s.channel_grant(b.id, false, revision).unwrap();
            s.channel_grant(b.id, true, off.grant_revision).unwrap();
        }
    }
    enable(&mut s, &b);
    let mut q = query(Some("absent"), 3);
    let p = s
        .source_search(b.id, q.clone(), &Redactor::new(vec![]))
        .unwrap();
    assert_eq!(p.examined, 512);
    assert!(p.scan_limit_reached && p.has_more);
    q.after = Some(p.next_cursor);
    let p = s.source_search(b.id, q, &Redactor::new(vec![])).unwrap();
    assert_eq!(p.examined, 2);
    assert!(!p.has_more);
    let mut plan = s
        .connection
        .prepare(&format!("EXPLAIN QUERY PLAN {SOURCE_PAGE_SQL}"))
        .unwrap();
    let details = plan
        .query_map(
            params![
                b.id.to_string(),
                0i64,
                0i64,
                now_ms().unwrap() as i64,
                513i64
            ],
            |r| r.get::<_, String>(3),
        )
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
        .join("\n");
    assert!(details.contains("channel_source_window"));
    assert!(!details.contains("TEMP B-TREE"));
    drop(plan);
    let p = s
        .source_search(b.id, query(None, 10), &Redactor::new(vec![]))
        .unwrap();
    assert!(!p.records.is_empty() && p.records.len() <= 10);
    assert!(serde_json::to_vec(&p.records).unwrap().len() <= MAX_PAGE_BYTES);
    for r in &p.records {
        assert!(r.incoming.truncated);
        assert!(r.incoming.text.len() <= 1024);
        assert!(serde_json::to_string(&r.incoming.text).unwrap().len() <= 1026);
    }
    for q in [
        query(None, 0),
        query(None, 11),
        query(Some(" "), 1),
        query(Some("secret\n"), 1),
        query(Some(&"x".repeat(257)), 1),
    ] {
        assert!(s.source_search(b.id, q, &Redactor::new(vec![])).is_err());
    }
}

#[test]
fn source_prepared_disclosure_rechecks_every_selection_and_audit_failure_is_atomic() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let b = bind(&mut s, "2002");
    let source = receive(&mut s, &b, "a historical source");
    enable(&mut s, &b);
    let mut turn = running(&mut s, &b);
    let (selection, receipt) = lookup(
        &mut s,
        &mut turn,
        Tool::SourceSearch(query(Some("historical"), 3)),
    );
    assert!(!receipt.to_string().contains("a historical source"));
    let (hashes, evidence) = s
        .source_actor_disclosure(
            turn.id,
            std::slice::from_ref(&selection),
            Some(1),
            &Redactor::new(vec![]),
        )
        .unwrap();
    assert_eq!(evidence["records"][0]["incoming_id"], json!(source.id));
    assert_eq!(hashes.len(), 1);
    let (second, _) = lookup(
        &mut s,
        &mut turn,
        Tool::SourceInspect(InspectSource {
            incoming_id: source.id,
            window: None,
        }),
    );
    let selections = vec![selection, second];
    let (two, _) = s
        .source_actor_disclosure(turn.id, &selections, None, &Redactor::new(vec![]))
        .unwrap();
    assert!(two.starts_with(&hashes));
    s.connection
        .execute(
            "INSERT INTO channel_memory_suppressions(channel_id,incoming_id) VALUES(?1,?2)",
            params![b.id.to_string(), source.id.to_string()],
        )
        .unwrap();
    assert!(
        s.source_actor_disclosure(turn.id, &selections, None, &Redactor::new(vec![]))
            .is_err()
    );
    s.connection
        .execute("DELETE FROM channel_memory_suppressions", [])
        .unwrap();
    s.connection.execute_batch("CREATE TRIGGER reject_source_read BEFORE INSERT ON channel_source_reads BEGIN SELECT RAISE(ABORT,'fixture audit failure'); END;").unwrap();
    turn.model_requests = 2;
    s.save_actor_turn(&mut turn, false).unwrap();
    assert!(
        s.source_actor_disclosure(turn.id, &selections, Some(2), &Redactor::new(vec![]))
            .is_err()
    );
    assert_eq!(
        s.source_reads(b.id, 0).unwrap()["reads"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["operation"] == "prepared_for_model_request")
            .count(),
        1
    );
    s.connection
        .execute_batch("DROP TRIGGER reject_source_read;")
        .unwrap();
    let g = s.source_grant(b.id).unwrap();
    s.set_source_grant(
        b.id,
        SetSourceGrant {
            enabled: false,
            expected_revision: g.revision,
            window: None,
            expires_at_ms: None,
        },
    )
    .unwrap();
    assert!(
        s.source_actor_disclosure(turn.id, &selections, None, &Redactor::new(vec![]))
            .is_err()
    );
}

#[test]
fn source_projection_keeps_generated_delivered_unknown_and_merged_scope_distinct() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let b = bind(&mut s, "2002");
    let first = receive(&mut s, &b, "first half");
    std::thread::sleep(Duration::from_millis(2));
    let second = receive(&mut s, &b, "second half");
    enable(&mut s, &b);
    let mut turn = running(&mut s, &b);
    turn.state = TurnState::Completed;
    turn.message(MessageRole::Assistant, "generated response".into())
        .unwrap();
    s.save_actor_turn(&mut turn, false).unwrap();
    s.channel_finish(BatchId(turn.request_id), &turn).unwrap();
    enable(&mut s, &b);
    let r = inspect(&mut s, &b, first.id).unwrap();
    assert_eq!(r.reply.generated.unwrap().text, "generated response");
    assert_eq!(r.reply.delivery_state, Some(DeliveryState::Ready));
    let d = s.channel_claim_delivery(b.id).unwrap().unwrap();
    s.channel_settle(d.id, SendOutcome::Confirmed("platform-41".into()))
        .unwrap();
    let r = inspect(&mut s, &b, first.id).unwrap();
    assert_eq!(r.reply.delivery_state, Some(DeliveryState::Confirmed));
    assert_eq!(r.reply.platform_receipt.as_deref(), Some("platform-41"));
    let r = s
        .source_inspect(
            b.id,
            InspectSource {
                incoming_id: second.id,
                window: Some(SourceWindow {
                    from_received_at_ms: second.received_at_ms,
                    ..s.source_grant(b.id).unwrap().window.unwrap()
                }),
            },
            &Redactor::new(vec![]),
        )
        .unwrap();
    assert_eq!(r.reply.visibility, ReplyVisibility::BatchOutsideWindow);
    assert!(r.reply.generated.is_none() && r.reply.delivery.is_none());
    // The normal recovery path preserves uncertainty rather than claiming a send.
    let mut next = running(&mut s, &b);
    next.state = TurnState::Completed;
    next.message(MessageRole::Assistant, "unconfirmed reply".into())
        .unwrap();
    s.save_actor_turn(&mut next, false).unwrap();
    s.channel_finish(BatchId(next.request_id), &next).unwrap();
    s.channel_claim_delivery(b.id).unwrap().unwrap();
    s.recover_channels().unwrap();
    enable(&mut s, &b);
    let pending = s
        .channel_snapshot(b.id)
        .unwrap()
        .incoming
        .last()
        .unwrap()
        .id;
    let r = inspect(&mut s, &b, pending).unwrap();
    assert_eq!(r.reply.delivery_state, Some(DeliveryState::Unknown));
    let memory = s
        .add_memory(&crate::memory::AddMemory {
            request_id: lain_wire::RequestId::new(),
            draft: crate::memory::MemoryDraft {
                content: "inspected evidence".into(),
                kind: crate::memory::MemoryKind::Preference,
                scope: crate::memory::MemoryScope::Global,
            },
        })
        .unwrap();
    s.connection.execute("INSERT INTO channel_memory_inspections(actor_turn_id,memory_id,revision) VALUES(?1,?2,?3)",params![turn.id.to_string(),memory.id.to_string(),memory.revision as i64]).unwrap();
    assert_eq!(
        inspect(&mut s, &b, first.id).unwrap().reply.visibility,
        ReplyVisibility::Visible
    );
    let changed = s
        .change_memory(
            memory.id,
            &crate::memory::ChangeMemory {
                expected_revision: memory.revision,
                status: crate::memory::MemoryStatus::Disabled,
                superseded_by: None,
            },
        )
        .unwrap();
    assert_eq!(
        inspect(&mut s, &b, first.id).unwrap().reply.visibility,
        ReplyVisibility::MemoryChanged
    );
    s.delete_memory(memory.id, changed.revision).unwrap();
    let r = inspect(&mut s, &b, first.id).unwrap();
    assert_eq!(r.reply.visibility, ReplyVisibility::MemoryChanged);
    assert!(r.reply.generated.is_none());
    turn.actions.push(Action {
        id: ActionId::new(),
        call_id: "old-source".into(),
        tool: Tool::SourceSearch(query(None, 1)),
        state: ActionState::Executed,
        result: Some(json!({"incoming_ids":[]})),
    });
    s.save_actor_turn(&mut turn, false).unwrap();
    assert_eq!(
        inspect(&mut s, &b, first.id).unwrap().reply.visibility,
        ReplyVisibility::SourceDerived
    );
}

#[test]
fn schema_nine_source_migration_preserves_snapshots_and_creates_no_fake_reads() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.db");
    let mut s = Store::open(&path).unwrap();
    let b = bind(&mut s, "2002");
    let input = receive(&mut s, &b, "old source");
    let snapshot: String = s
        .connection
        .query_row(
            "SELECT snapshot FROM channel_incoming WHERE id=?1",
            [input.id.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    s.connection.execute_batch("DROP TABLE channel_source_reads; DROP TABLE channel_source_grants; DROP INDEX channel_source_window; DROP INDEX channel_incoming_batch; PRAGMA user_version=9;").unwrap();
    drop(s);
    let s = Store::open(&path).unwrap();
    let after: String = s
        .connection
        .query_row(
            "SELECT snapshot FROM channel_incoming WHERE id=?1",
            [input.id.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(after, snapshot);
    assert!(!s.source_grant(b.id).unwrap().enabled);
    assert!(s.source_grants(b.id).unwrap().is_empty());
    assert!(
        s.source_reads(b.id, 0).unwrap()["reads"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        s.connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        11
    );
}
