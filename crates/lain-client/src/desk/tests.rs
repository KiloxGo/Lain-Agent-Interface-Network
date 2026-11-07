use super::*;

fn forget_review_fixture() -> ActorSnapshot {
    use lain_wire::{ActionId, ActorAction, ToolCall, memory::*};
    let mut turn = running(ConversationId::new());
    turn.state = ActorTurnState::PendingConfirmation;
    let message = MessageId::new();
    let quote = "请忘记那条来源通知。";
    turn.messages.push(lain_wire::ActorMessage {
        id: message,
        conversation_id: turn.conversation_id,
        role: MessageRole::User,
        content: quote.into(),
        timestamp_ms: 1,
        status: lain_wire::MessageStatus::Completed,
    });
    let action = ActionId::new();
    let memory = lain_wire::MemoryId::new();
    let q = ChangeMemorySelection {
        memory_id: memory,
        expected_revision: 1,
        operation: MemoryOperation::Forget,
        message_id: message,
        quote: quote.into(),
    };
    turn.actions.push(ActorAction {
        id: action,
        call_id: "forget-preview".into(),
        tool: ToolCall::MemorySelectionChange(q.clone()),
        state: ActionState::Proposed,
        result: None,
        memory_review: None,
        memory_change_review: Some(MemorySelectionReview {
            actor_turn_id: turn.id,
            action_id: action,
            request: q,
            replacement: None,
            target: MemorySelectionTarget {
                memory_id: memory,
                conversation_id: turn.conversation_id,
                revision: 1,
                status: MemoryStatus::Active,
                recorded_at_ms: 1,
                expires_at_ms: 1000,
            },
        }),
    });
    turn
}

#[test]
fn reviewed_forget_displays_exact_user_intent_scope_and_refuses_missing_preview() {
    use lain_wire::ToolCall;
    let mut turn = forget_review_fixture();
    let quote = "请忘记那条来源通知。";
    let mut app = Model::new(turn.conversation_id);
    app.snapshot(turn.clone())
        .expect("full TUI snapshot must retain user quotation for shared review validation");
    let text = dispatch::review_text(&turn, &turn.actions[0]).unwrap();
    assert!(text.contains("忘记来源事项") && text.contains("仅当前会话") && text.contains(quote));
    let mut legacy = Renderer::new(Vec::new());
    legacy.snapshot(&turn).unwrap();
    assert!(legacy.drain_text().contains("忘记来源事项"));
    // A genuine substring can omit negation or other constraints. Review must
    // preserve the complete immutable current request as well as the excerpt.
    turn.messages[0].content = "请不要忘记这条来源通知；先让我确认具体范围。".into();
    if let ToolCall::MemorySelectionChange(q) = &mut turn.actions[0].tool {
        q.quote = "忘记".into();
    }
    turn.actions[0]
        .memory_change_review
        .as_mut()
        .unwrap()
        .request
        .quote = "忘记".into();
    let text = dispatch::review_text(&turn, &turn.actions[0]).unwrap();
    assert!(
        text.contains(&turn.messages[0].content),
        "substring authentication must not hide the full user's negation and constraints"
    );
    turn.actions[0].memory_change_review = None;
    assert!(dispatch::review_text(&turn, &turn.actions[0]).is_err());
    assert!(Renderer::new(Vec::new()).snapshot(&turn).is_err());
}

#[test]
fn reviewed_correction_shows_exact_replacement_lifetime_and_rejects_mismatched_draft() {
    use lain_wire::{CandidateId, ToolCall, memory::*};
    let mut t = forget_review_fixture();
    let text = "不要报名。更正为周日见面，尚未承诺参加。";
    let quote = "更正为周日见面，尚未承诺参加。";
    t.messages[0].content = text.into();
    let a = &mut t.actions[0];
    let ToolCall::MemorySelectionChange(q) = &mut a.tool else {
        panic!()
    };
    q.operation = MemoryOperation::Correct;
    q.quote = quote.into();
    let review = a.memory_change_review.as_mut().unwrap();
    review.request = q.clone();
    review.replacement = Some(MemoryCandidate {
        id: CandidateId(a.id.0),
        draft: MemoryDraft {
            content: quote.into(),
            kind: MemoryKind::Episodic,
            scope: MemoryScope::Conversation {
                conversation_id: t.conversation_id,
            },
        },
        provenance: MemoryProvenance {
            authority: MemoryAuthority::Explicit,
            source: MemorySource::ConversationCorrection {
                evidence: ConversationCorrectionEvidence {
                    conversation_id: t.conversation_id,
                    previous_memory_id: q.memory_id,
                    previous_revision: q.expected_revision,
                    actor_turn_id: t.id,
                    action_id: a.id,
                    message_id: q.message_id,
                    expires_at_ms: review.target.expires_at_ms,
                },
            },
        },
        origin: Some(MemoryOrigin::Conversation {
            conversation_id: t.conversation_id,
        }),
        status: CandidateStatus::Candidate,
        memory_id: None,
        created_at_ms: 2,
        reviewed_at_ms: None,
        actor_action: Some(a.id),
    });
    let text_review = dispatch::review_text(&t, &t.actions[0]).unwrap();
    assert!(text_review.contains(text) && text_review.contains(quote));
    assert!(text_review.contains("有效期不延长") && text_review.contains("用户更正，未独立核实"));
    let mut app = Model::new(t.conversation_id);
    app.snapshot(t.clone()).unwrap();
    let mut legacy = Renderer::new(Vec::new());
    legacy.snapshot(&t).unwrap();
    assert!(legacy.drain_text().contains("更正已保存事项"));
    t.actions[0]
        .memory_change_review
        .as_mut()
        .unwrap()
        .replacement
        .as_mut()
        .unwrap()
        .draft
        .content = "模型偷偷改写后的内容".into();
    assert!(dispatch::review_text(&t, &t.actions[0]).is_err());
    assert!(Renderer::new(Vec::new()).snapshot(&t).is_err());
}

#[test]
fn checked_withdrawn_history_retains_earlier_save_receipts_after_later_withheld_turns() {
    let mut app = Model::new(ConversationId::new());
    let mut saved = running(app.conversation);
    saved.state = ActorTurnState::Completed;
    saved.availability = Some(lain_wire::ActorAvailability::SourceUnavailable);
    saved.error = Some("Source unavailable; previous content withheld.".into());
    let action = lain_wire::ActionId::new();
    saved
        .reviewed_memory_receipts
        .push(lain_wire::memory::ReviewedMemoryReceipt {
            action_id: action,
            actor_turn_id: saved.id,
            conversation_id: app.conversation,
            candidate_id: lain_wire::CandidateId(action.0),
            memory_id: lain_wire::MemoryId::new(),
            recorded_at_ms: 1234567,
        });
    let mut later = running(app.conversation);
    later.sequence = 2;
    later.state = ActorTurnState::Completed;
    later.availability = saved.availability;
    later.error = saved.error.clone();
    app.replace_history(vec![saved, later]).unwrap();
    assert!(
        app.text_view
            .doc
            .notes()
            .iter()
            .any(|e| e.3.contains("已保存来源事项")),
        "a later withheld turn must not clear an earlier content-free receipt in the same checked history"
    );
}

#[test]
fn briefing_memory_review_displays_the_fixed_source_draft_and_refuses_missing_preview() {
    use lain_wire::{
        ActionId, ActorAction, CandidateId, ToolCall, briefing::RememberBriefing, memory::*,
    };
    let mut app = Model::new(ConversationId::new());
    let mut turn = running(app.conversation);
    let action = ActionId::new();
    let brief = lain_wire::RequestId::new();
    let c = MemoryCandidate {
        id: CandidateId(action.0),
        draft: MemoryDraft {
            content: "测试安排：周四讨论；这只是来源报告，未确认参加。".into(),
            kind: MemoryKind::Episodic,
            scope: MemoryScope::Conversation {
                conversation_id: app.conversation,
            },
        },
        provenance: MemoryProvenance {
            authority: MemoryAuthority::UnverifiedSelection,
            source: MemorySource::BriefingSelection {
                evidence: BriefingMemoryEvidence {
                    reconciliation: None,
                    conversation_id: app.conversation,
                    briefing_id: brief,
                    item_id: lain_wire::BriefingItemId::new(),
                    extraction_turn_id: lain_wire::ActorTurnId::new(),
                    batch_from_index: 0,
                    actor_turn_id: turn.id,
                    action_id: action,
                    grant_revision: 1,
                    expires_at_ms: 1000,
                    records: vec![lain_wire::archives::ArchiveRecallVersion {
                        record_id: lain_wire::ArchiveRecordId::new(),
                        source_id: lain_wire::ArchiveSourceId::new(),
                        revision: 1,
                        source_grant_revision: 1,
                    }],
                },
            },
        },
        origin: Some(MemoryOrigin::Conversation {
            conversation_id: app.conversation,
        }),
        status: CandidateStatus::Candidate,
        memory_id: None,
        created_at_ms: 1,
        reviewed_at_ms: None,
        actor_action: Some(action),
    };
    turn.state = ActorTurnState::PendingConfirmation;
    turn.actions.push(ActorAction {
        memory_review: Some(c),
        memory_change_review: None,
        id: action,
        call_id: "remember".into(),
        tool: ToolCall::BriefingRemember(RememberBriefing {
            include_related: false,
            briefing_id: brief,
            item_number: 1,
        }),
        state: ActionState::Proposed,
        result: None,
    });
    app.snapshot(turn.clone()).unwrap();
    app.open_review().unwrap();
    let text = &app.review.as_ref().unwrap().text;
    assert!(
        text.contains("测试安排") && text.contains("仅当前会话") && text.contains("未独立核实")
    );
    let mut legacy = Renderer::new(Vec::new());
    legacy.snapshot(&turn).unwrap();
    let text = legacy.drain_text();
    assert!(
        text.contains("测试安排") && text.contains("仅当前会话") && text.contains("有效期"),
        "legacy chat must display the same actual draft before confirmation"
    );
    if let ToolCall::BriefingRemember(q) = &mut turn.actions[0].tool {
        q.include_related = true;
    }
    let MemorySource::BriefingSelection { evidence: e } = &mut turn.actions[0]
        .memory_review
        .as_mut()
        .unwrap()
        .provenance
        .source
    else {
        panic!()
    };
    e.reconciliation = Some(BriefingMemoryReconciliation {
        index_turn_id: lain_wire::ActorTurnId::new(),
        fingerprint: "a".repeat(64),
        member_numbers: vec![1, 2],
    });
    let text = dispatch::review_text(&turn, &turn.actions[0]).unwrap();
    assert!(text.contains("整组关联") && text.contains("完整发现/核对依赖"));
    if let ToolCall::BriefingRemember(q) = &mut turn.actions[0].tool {
        q.include_related = false;
    }
    assert!(
        dispatch::review_text(&turn, &turn.actions[0]).is_err(),
        "group preview cannot masquerade as exact-original selection"
    );
    turn.actions[0].memory_review = None;
    assert!(dispatch::review_text(&turn, &turn.actions[0]).is_err());
    assert!(Renderer::new(Vec::new()).snapshot(&turn).is_err());
}

#[tokio::test]
async fn activity_baseline_is_quiet_and_live_task_card_keeps_its_place() {
    let mut app = Model::new(ConversationId::new());
    let client = Client::new("http://127.0.0.1:1", "fixture-activity-token-0123456789").unwrap();
    let (tx, _) = mpsc::channel(8);
    let mut jobs = JoinSet::new();
    let mut old = remote_record(app.conversation);
    old.command.state = lain_wire::device_commands::CommandState::Cancelled;
    let send = |app: &mut Model, rows: Vec<DeviceWorkRecord>, jobs: &mut JoinSet<()>| {
        handle_complete(
            &client,
            app,
            &tx,
            jobs,
            Complete::RemoteWork(Ok(DeviceWorkPage {
                work: rows,
                truncated: false,
            })),
        )
        .unwrap();
    };
    send(&mut app, vec![old.clone()], &mut jobs);
    assert!(
        app.text_view.doc.entries.is_empty(),
        "initial task snapshot must not replay old work into chat"
    );
    let mut live = remote_record(app.conversation);
    send(&mut app, vec![live.clone(), old.clone()], &mut jobs);
    assert!(
        app.text_view.doc.entries.is_empty(),
        "queued/running work belongs to sidebar"
    );
    live.command.state = lain_wire::device_commands::CommandState::Unknown;
    live.command.revision += 1;
    send(&mut app, vec![live.clone(), old.clone()], &mut jobs);
    assert_eq!(app.text_view.doc.entries.len(), 1);
    let card = app.text_view.doc.entries[0].id;
    assert!(!app.text_view.doc.entries[0].raw.contains("Kernel state"));
    app.text_view.doc.upsert(
        uuid::Uuid::new_v4(),
        rich::Role::Assistant,
        "later conversation",
        true,
    );
    live.command.state = lain_wire::device_commands::CommandState::Cancelled;
    live.command.revision += 1;
    send(&mut app, vec![live.clone(), old], &mut jobs);
    assert_eq!(app.text_view.doc.entries.len(), 2);
    assert_eq!(
        app.text_view.doc.entries[0].id, card,
        "updates must not move the card below later conversation"
    );
    assert!(
        app.text_view
            .doc
            .layout(90)
            .rows
            .iter()
            .any(|r| r.label.as_ref().is_some_and(|s| s.contains("已取消")))
    );
    assert!(jobs.is_empty() && app.pending.is_none());
}

#[test]
fn missing_memory_lookup_is_neutral_but_failed_receipt_remains_inspectable() {
    use lain_wire::{ActionId, ActorAction, MemoryId, ToolCall, memory::MemoryScope};
    let conversation = ConversationId::new();
    let mut turn = running(conversation);
    let result = json!({"success":false,"error":"not_found",
        "retry":"never automatically replay an uncertain mutation"});
    turn.actions.push(ActorAction {
        memory_review: None,
        memory_change_review: None,
        id: ActionId::new(),
        call_id: "missing-memory".into(),
        tool: ToolCall::MemoryInspect {
            memory_id: MemoryId::new(),
            scope: MemoryScope::Conversation {
                conversation_id: conversation,
            },
        },
        state: ActionState::Failed,
        result: Some(result.clone()),
    });
    let mut app = Model::new(conversation);
    app.snapshot(turn.clone()).unwrap();
    let has_title = |app: &mut Model, title: &str| {
        app.text_view
            .doc
            .layout(100)
            .rows
            .iter()
            .any(|row| row.label.as_ref().is_some_and(|label| label == title))
    };
    assert!(has_title(&mut app, "记忆查询 · 未找到或不可见"));
    let collapsed = &app.text_view.doc.entries.last().unwrap().raw;
    assert!(collapsed.contains("未找到或不可见"));
    assert!(!collapsed.contains("异常") && !collapsed.contains("已忘记"));
    let note = app.text_view.doc.notes()[0].clone();
    activity::activate(&mut app, note.0, note.1, note.2);
    let expanded = &app.text_view.doc.entries.last().unwrap().raw;
    assert!(expanded.contains("not_found") && expanded.contains("state: failed"));
    assert_eq!(turn.actions[0].state, ActionState::Failed);
    assert_eq!(turn.actions[0].result, Some(result));

    // A genuine failure alongside a missing lookup keeps its error card.
    let mut failure = turn.actions[0].clone();
    failure.id = ActionId::new();
    failure.call_id = "denied-memory".into();
    failure.result = Some(json!({"success":false,"error":"forbidden"}));
    turn.actions.push(failure);
    let mut app = Model::new(conversation);
    app.snapshot(turn.clone()).unwrap();
    assert!(has_title(&mut app, "工具执行异常"));
    let collapsed = &app.text_view.doc.entries.last().unwrap().raw;
    assert!(collapsed.contains("1 项失败"));
    assert!(collapsed.contains("1 项未找到或不可见"));

    turn.error = Some("actual inference failure".into());
    let mut app = Model::new(conversation);
    app.snapshot(turn).unwrap();
    assert!(has_title(&mut app, "对话异常"));
}

#[test]
fn tool_details_are_folded_and_task_navigation_does_not_submit_or_lose_chat_draft() {
    use lain_wire::{ActionId, ActorAction, ToolCall};
    let mut app = Model::new(ConversationId::new());
    app.input.insert("keep chat draft");
    let mut turn = running(app.conversation);
    turn.actions.push(ActorAction {
        memory_review: None,
        memory_change_review: None,
        id: ActionId::new(),
        call_id: "read".into(),
        tool: ToolCall::WorkList {},
        state: ActionState::Executed,
        result: Some(json!([])),
    });
    app.snapshot(turn).unwrap();
    let note = app.text_view.doc.notes()[0].clone();
    assert!(
        !app.text_view
            .doc
            .entries
            .last()
            .unwrap()
            .raw
            .contains("Receipt")
    );
    let collapsed = app.text_view.doc.entries.last().unwrap().raw.clone();
    activity::activate(&mut app, note.0, note.1, note.2);
    assert!(app.text_view.doc.entries.last().unwrap().raw.len() > collapsed.len());
    let note = app.text_view.doc.notes()[0].clone();
    activity::activate(&mut app, note.0, note.1, note.2);
    assert_eq!(app.text_view.doc.entries.last().unwrap().raw, collapsed);
    let r = remote_record(app.conversation);
    app.remote_work = vec![r.clone()];
    activity::task(&mut app.text_view.doc, &r.command);
    let note = app.text_view.doc.notes()[0].clone();
    activity::activate(&mut app, note.0, note.1, note.2);
    assert_eq!(
        app.surface,
        Surface::Task(r.command.device_id, r.command.id)
    );
    assert_eq!(app.task.as_ref().unwrap().id, r.command.id);
    assert_eq!(app.input.text, "keep chat draft");
    assert!(
        app.pending.is_none() && app.new_work_pending.is_none() && app.remote_pending.is_none()
    );
}

#[tokio::test]
async fn device_stream_reconnect_does_not_announce_offline_completions_as_new() {
    let mut app = Model::new(ConversationId::new());
    let client = Client::new("http://127.0.0.1:1", "fixture-activity-token-0123456789").unwrap();
    let (tx, _) = mpsc::channel(8);
    let mut jobs = JoinSet::new();
    let mut record = remote_record(app.conversation);
    let update = |r| {
        Complete::RemoteWork(Ok(DeviceWorkPage {
            work: vec![r],
            truncated: false,
        }))
    };
    handle_complete(&client, &mut app, &tx, &mut jobs, update(record.clone())).unwrap();
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::RemoteWork(Err(anyhow::anyhow!("fixture stream ended"))),
    )
    .unwrap();
    record.command.revision += 1;
    record.command.state = lain_wire::device_commands::CommandState::Succeeded;
    handle_complete(&client, &mut app, &tx, &mut jobs, update(record)).unwrap();
    assert!(app.text_view.doc.entries.is_empty());
    assert!(app.remote_connected && !app.activity.reconnecting && jobs.is_empty());
}

#[test]
fn work_followup_preserves_each_instruction_and_result_in_order() {
    use lain_wire::device_work::DeviceContinuation;
    let mut app = Model::new(ConversationId::new());
    let (mut first, device) = idle_remote(app.conversation);
    first.command.input.task = "inspect original project".into();
    first
        .command
        .execution
        .as_mut()
        .unwrap()
        .result
        .as_mut()
        .unwrap()
        .output_tail = "first result retained".into();
    let mut second = first.clone();
    second.command.id = DeviceCommandId::new();
    second.command.created_at_ms += 60000;
    second.command.input.task = "explain follow-up".into();
    second
        .command
        .execution
        .as_mut()
        .unwrap()
        .result
        .as_mut()
        .unwrap()
        .output_tail = "second result retained".into();
    let execution = first.command.execution.as_ref().unwrap();
    second.command.continuation = Some(DeviceContinuation {
        root_command_id: first.command.id,
        parent_command_id: first.command.id,
        expected_parent_revision: first.command.revision,
        session_id: execution.session_id.unwrap(),
        parent_turn_id: execution.turn_id.unwrap(),
    });
    first.command.continued_by = Some(second.command.id);
    app.surface = Surface::Task(second.command.device_id, second.command.id);
    app.task = Some(second.command.clone());
    app.devices = vec![device];
    app.remote_work = vec![second, first];
    activity::prepare_output(&mut app);
    let raw = app
        .work_text
        .doc
        .entries
        .iter()
        .map(|e| e.raw.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let positions = [
        "inspect original project",
        "first result retained",
        "explain follow-up",
        "second result retained",
    ]
    .map(|text| raw.find(text).expect("each turn must remain visible"));
    assert!(positions.windows(2).all(|p| p[0] < p[1]));
    assert!(
        raw.contains("C:\\fixture"),
        "use the registered device directory"
    );
    assert_eq!(
        app.task.as_ref().unwrap().input.task,
        "explain follow-up",
        "controls remain attached to the latest selected turn"
    );
}

#[test]
fn work_result_has_independent_markdown_copy_and_is_cleared_on_source_loss() {
    use lain_wire::{DeviceExecutionId, WorkSessionId, WorkerTurnId, device_execution::*};
    let mut app = Model::new(ConversationId::new());
    app.text_view.doc.upsert(
        uuid::Uuid::new_v4(),
        rich::Role::Assistant,
        "chat only",
        true,
    );
    let mut c = remote_record(app.conversation).command;
    let raw = "**最终结果**\n\n```rust\nlet x = 1;\n```";
    c.execution = Some(ExecutionEvidence {
        execution_id: DeviceExecutionId::new(),
        session_id: Some(WorkSessionId::new()),
        turn_id: Some(WorkerTurnId::new()),
        phase: ExecutionPhase::Completed,
        pending_approvals: vec![],
        approval_reports: vec![],
        result: Some(DeviceWorkResult {
            output_tail: raw.into(),
            command_failures: 0,
            reported_paths: vec![],
            paths_withheld: 0,
            history_truncated: true,
        }),
    });
    app.surface = Surface::Task(c.device_id, c.id);
    app.task = Some(c.clone());
    activity::prepare_output(&mut app);
    assert_eq!(
        app.work_text
            .doc
            .copy(c.id.0, rich::CopyKind::Markdown)
            .as_deref(),
        Some(raw)
    );
    let plain = app
        .work_text
        .doc
        .copy(c.id.0, rich::CopyKind::Plain)
        .unwrap();
    assert!(plain.contains("最终结果") && plain.contains("let x = 1;"));
    assert!(!plain.contains("**") && !plain.contains("```"));
    assert!(
        app.work_text
            .doc
            .layout(80)
            .rows
            .iter()
            .any(|r| r.label.as_ref().is_some_and(|s| s.contains("节选")))
    );
    text_view::key(&mut app, KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
    assert!(app.work_text.menu.is_some() && app.text_view.menu.is_none());
    assert_eq!(app.text_view.doc.entries[0].raw, "chat only");
    live::retire(&mut app);
    assert!(app.work_text.doc.entries.is_empty() && app.text_view.doc.entries.is_empty());
}

#[tokio::test]
async fn quiet_kernel_notices_are_not_acknowledged_and_single_stream_reconnect_rebases() {
    use lain_wire::{ActionId, ActorTurnId, NoticeId, WorkSessionId, notices::*};
    let c = ConversationId::new();
    let mut app = Model::new(c);
    let notice = |cursor| ConversationNotice {
        id: NoticeId::new(),
        cursor,
        conversation_id: c,
        actor_turn_id: ActorTurnId::new(),
        action_id: ActionId::new(),
        work_session_id: WorkSessionId::new(),
        worker_turn_id: None,
        timestamp_ms: 1,
        source_history_order: None,
        outcome: NoticeOutcome::Worker {
            state: WorkerOutcome::Completed,
        },
        evidence: ResultEvidence::default(),
        acknowledged: false,
    };
    let old = notice(1);
    let mut snapshot = bootstrap_page(c, vec![]);
    snapshot.notices = vec![lain_wire::client_sync::NoticePointer {
        id: old.id,
        cursor: 1,
        acknowledged: false,
    }];
    snapshot.notice_cursor = 1;
    app.bootstrap(snapshot.clone()).unwrap();
    let client = Client::new(
        "http://127.0.0.1:1",
        "fixture-notice-token-0123456789-abcdef",
    )
    .unwrap();
    let (tx, _) = mpsc::channel(8);
    let mut jobs = JoinSet::new();
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::Notice(NoticeUpdate::Notice(Box::new(old))),
    )
    .unwrap();
    assert!(
        app.notices.is_empty(),
        "/seen must not acknowledge a hidden baseline receipt"
    );
    assert!(app.text_view.doc.entries.is_empty());
    let live = notice(2);
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::Notice(NoticeUpdate::Notice(Box::new(live.clone()))),
    )
    .unwrap();
    assert_eq!(app.text_view.doc.entries.len(), 1);
    let offline = notice(3);
    activity::notice_reconnect(&mut app);
    snapshot
        .notices
        .push(lain_wire::client_sync::NoticePointer {
            id: offline.id,
            cursor: 3,
            acknowledged: false,
        });
    activity::bootstrap_notices(&mut app, &snapshot);
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::Notice(NoticeUpdate::Notice(Box::new(offline.clone()))),
    )
    .unwrap();
    assert!(!app.notices.contains(&offline.id));
    let mut revised = live;
    revised.id = NoticeId::new();
    revised.outcome = NoticeOutcome::Worker {
        state: WorkerOutcome::Failed,
    };
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::Notice(NoticeUpdate::Notice(Box::new(revised))),
    )
    .unwrap();
    assert_eq!(
        app.text_view.doc.entries.len(),
        1,
        "one action keeps one card"
    );
    assert!(jobs.is_empty());
}

#[test]
fn wheel_scrolls_only_chat_body_without_touching_drafts_or_reviews() {
    use crossterm::event::{MouseEvent, MouseEventKind};
    use ratatui::{Terminal, backend::TestBackend};
    let mut app = Model::new(ConversationId::new());
    app.append(
        &(0..180)
            .map(|n| format!("history-row-{n:03}\n"))
            .collect::<String>(),
    );
    app.input.insert("保留草稿");
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    terminal.draw(|f| draw::render(f, &mut app)).unwrap();
    let end = app.scroll;
    let wheel = |kind, column, row| MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    };
    handle_mouse(&mut app, wheel(MouseEventKind::ScrollUp, 80, 12));
    assert!(!app.follow && app.scroll < end);
    let reading = app.scroll;
    app.append("new live message\n");
    terminal.draw(|f| draw::render(f, &mut app)).unwrap();
    assert_eq!(
        app.scroll, reading,
        "live output must not steal the reading position"
    );
    handle_mouse(&mut app, wheel(MouseEventKind::ScrollUp, 80, 38));
    assert_eq!(
        app.scroll, reading,
        "wheel over the composer must not scroll chat"
    );
    app.open_review().err();
    app.surface = Surface::Work;
    handle_mouse(&mut app, wheel(MouseEventKind::ScrollUp, 80, 12));
    assert_eq!(
        app.scroll, reading,
        "wheel must not act as permission/list interaction"
    );
    assert_eq!(app.input.text, "保留草稿");
}

#[test]
fn wide_sidebar_renders_local_facts_and_running_tasks_without_clipping_portrait() {
    use ratatui::{Terminal, backend::TestBackend};
    let mut app = Model::new(ConversationId::new());
    app.portrait = Some(
        (0..18)
            .map(|_| "A".repeat(29))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    app.local_info = telemetry::LocalInfo {
        system: "FixtureOS · x86_64".into(),
        logical_cpus: Some(8),
        directory: Some("D:\\fixture\\repo".into()),
        memory: Some(telemetry::Memory {
            total: 16 * 1024 * 1024 * 1024,
            available: Some(8 * 1024 * 1024 * 1024),
        }),
    };
    let session = Session {
        id: uuid::Uuid::new_v4(),
        backend: "codex".into(),
        runtime_profile_id: "codex/local".into(),
        workspace: "/srv/fixture/work".into(),
        state: "running".into(),
        runtime: "attached".into(),
    };
    app.worker_status
        .apply(crate::api::Update::Snapshot(crate::api::Snapshot {
            history_cursor: 1,
            sessions: vec![session],
            turns: vec![],
            approvals: vec![],
        }))
        .unwrap();
    let mut terminal = Terminal::new(TestBackend::new(150, 55)).unwrap();
    terminal.draw(|f| draw::render(f, &mut app)).unwrap();
    let buffer = terminal.backend().buffer();
    let screen = (0..55)
        .map(|y| {
            let mut line = String::new();
            let mut x = 0;
            while x < 32 {
                let symbol = buffer[(x, y)].symbol();
                line.push_str(symbol);
                x += unicode_width::UnicodeWidthStr::width(symbol).max(1) as u16;
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n");
    for expected in [
        "本机",
        "FixtureOS",
        "8.0 / 16.0 GiB",
        "D:\\fixture\\repo",
        "进行中的任务",
        "codex",
        "运行中",
    ] {
        assert!(screen.contains(expected), "missing {expected}: {screen}");
    }
    assert_eq!(screen.matches(&"A".repeat(29)).count(), 18);
    app.local_info.directory =
        Some("D:\\fixture\\a-long-directory-for-a-real-project\\repository".into());
    app.workspace_context = Some("device fixture / a-long-selected-workspace-identifier".into());
    assert!(
        sidebar::rows(&app, 30, 14)
            .into_iter()
            .take(14)
            .collect::<Vec<_>>()
            .join("\n")
            .contains("运行中"),
        "normal-height sidebar must leave room for actual tasks"
    );
    assert!(app.pending.is_none());
}

#[tokio::test]
async fn ctrl_j_and_ctrl_enter_edit_multiline_draft_without_submitting() {
    for key in [
        KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL),
        KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL),
    ] {
        for busy in [false, true] {
            let mut app = Model::new(ConversationId::new());
            app.input.insert("第一行");
            if busy {
                app.turn = Some(running(app.conversation));
            }
            let client = Client::new(
                "http://127.0.0.1:1",
                "fixture-multiline-owner-123456789012345",
            )
            .unwrap();
            let (tx, _rx) = mpsc::channel(8);
            let mut jobs = JoinSet::new();
            handle_key(&client, &mut app, &tx, &mut jobs, key).unwrap();
            assert_eq!(app.input.text, "第一行\n");
            assert_eq!(app.input.cursor, app.input.text.len());
            assert!(app.pending.is_none() && jobs.is_empty());
        }
    }
}

#[tokio::test]
async fn live_sync_connection_loss_retires_body_and_keeps_the_exact_uncertain_request() {
    let mut app = Model::new(ConversationId::new());
    let mut t = running(app.conversation);
    t.state = ActorTurnState::Completed;
    t.messages.push(lain_wire::ActorMessage {
        id: MessageId::new(),
        conversation_id: app.conversation,
        role: MessageRole::Assistant,
        content: "offline-private-marker".into(),
        timestamp_ms: 1,
        status: lain_wire::MessageStatus::Completed,
    });
    app.snapshot(t).unwrap();
    app.input.insert("my next draft");
    let id = RequestId::new();
    app.pending = Some(Pending {
        id,
        text: "original uncertain request".into(),
        uncertain: true,
    });
    let client = Client::new(
        "http://127.0.0.1:1",
        "sync-fixture-owner-12345678901234567890",
    )
    .unwrap();
    let (tx, _rx) = mpsc::channel(8);
    let mut jobs = JoinSet::new();
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::Notice(NoticeUpdate::Offline),
    )
    .unwrap();
    assert!(!app.transcript.contains("offline-private-marker"));
    assert_eq!(app.input.text, "my next draft");
    assert_eq!(app.pending.as_ref().unwrap().id, id);
    assert!(app.pending.as_ref().unwrap().uncertain);
    assert!(jobs.is_empty());
}

pub(super) fn bootstrap_page(
    conversation: ConversationId,
    turns: Vec<ActorSnapshot>,
) -> lain_wire::client_sync::BootstrapSnapshot {
    use lain_wire::client_sync::*;
    BootstrapSnapshot {
        cursor: SyncCursor {
            instance_id: uuid::Uuid::new_v4(),
            boot_id: uuid::Uuid::new_v4(),
            revision: 1,
        },
        issued_at_ms: 1,
        valid_until_ms: 1 + MAX_LEASE_MS,
        resources: Resources {
            protocol: PROTOCOL,
            configuration_revision: "ab".repeat(32),
            persona_revision: "bc".repeat(32),
            policy_revision: "cd".repeat(32),
            capabilities: vec![
                "client_sync/1".into(),
                "source_guarded_publication/1".into(),
            ],
        },
        conversation_id: Some(conversation),
        memory_context: None,
        turns,
        history_truncated: false,
        devices: vec![],
        projects: vec![],
        mappings: vec![],
        work: vec![],
        work_truncated: false,
        notices: vec![],
        notice_cursor: 0,
        notices_truncated: false,
    }
}

pub(super) fn remote_record(
    conversation: ConversationId,
) -> lain_wire::device_work::DeviceWorkRecord {
    use lain_wire::{DeviceWorkspaceId, Sandbox, device_commands::*, device_work::*};
    DeviceWorkRecord {
        origin: DeviceWorkOrigin {
            conversation_id: conversation,
            actor_turn_id: None,
            action_id: None,
        },
        command: DeviceCommand {
            id: DeviceCommandId::new(),
            device_id: DeviceId::new(),
            created_at_ms: 1,
            revision: 1,
            state: CommandState::Queued,
            cancel_requested: false,
            interruption: None,
            last_event: None,
            execution: None,
            continuation: None,
            continued_by: None,
            input: QueueDeviceWork {
                request_id: RequestId::new(),
                expected_device_revision: 2,
                workspace_id: DeviceWorkspaceId::new(),
                profile_id: "codex/local".into(),
                sandbox: Sandbox::ReadOnly,
                task: "original remote task".into(),
                expires_at_ms: 9999999999999,
            },
        },
    }
}

#[tokio::test]
async fn remote_facts_do_not_replay_work_or_regress_during_chat_and_reconnect() {
    use lain_wire::device_work::DeviceWorkPage;
    let mut app = Model::new(ConversationId::new());
    app.input.insert("继续和 Lain 聊天的草稿");
    let record = remote_record(app.conversation);
    let page = DeviceWorkPage {
        work: vec![record.clone()],
        truncated: false,
    };
    let client = Client::new("http://127.0.0.1:1", "fixture-test-only-credential-123456").unwrap();
    let (tx, _) = mpsc::channel(8);
    let mut jobs = JoinSet::new();
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::RemoteWork(Ok(page.clone())),
    )
    .unwrap();
    let first = app.transcript.clone();
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::RemoteWork(Ok(page.clone())),
    )
    .unwrap();
    assert_eq!(
        app.transcript, first,
        "reconnect must not duplicate an identical snapshot notice"
    );
    let mut newer = record.clone();
    newer.command.revision = 2;
    newer.command.state = lain_wire::device_commands::CommandState::Succeeded;
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::RemoteWork(Ok(DeviceWorkPage {
            work: vec![newer],
            truncated: false,
        })),
    )
    .unwrap();
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::RemoteWork(Ok(page)),
    )
    .unwrap();
    assert_eq!(
        app.remote_work[0].command.state,
        lain_wire::device_commands::CommandState::Succeeded
    );
    let wrong = DeviceWorkPage {
        work: vec![remote_record(ConversationId::new())],
        truncated: false,
    };
    assert!(
        handle_complete(
            &client,
            &mut app,
            &tx,
            &mut jobs,
            Complete::RemoteWork(Ok(wrong))
        )
        .is_err()
    );
    assert_eq!(app.input.text, "继续和 Lain 聊天的草稿");
    assert!(jobs.is_empty() && app.pending.is_none() && app.turn.is_none());
}

#[tokio::test]
async fn selecting_remote_work_opens_device_detail_and_never_a_cloud_worker_session() {
    let mut app = Model::new(ConversationId::new());
    let r = remote_record(app.conversation);
    let d = r.command.device_id;
    let c = r.command.id;
    app.remote_work.push(r);
    app.navigate(Surface::Work);
    app.input.insert("my chat draft");
    let client = Client::new("http://127.0.0.1:1", "fixture-test-only-credential-123456").unwrap();
    let (tx, _) = mpsc::channel(8);
    let mut jobs = JoinSet::new();
    let flow = handle_key(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    )
    .unwrap();
    assert!(matches!(flow, Flow::Continue));
    assert_eq!(app.surface, Surface::Task(d, c));
    assert_eq!(app.input.text, "my chat draft");
}

#[tokio::test]
async fn restored_device_work_keeps_full_ids_in_diagnostics_without_chat_replay() {
    let mut app = Model::new(ConversationId::new());
    let mut old = remote_record(app.conversation);
    old.command.state = lain_wire::device_commands::CommandState::Succeeded;
    let mut recent = remote_record(app.conversation);
    recent.command.created_at_ms = 2;
    let id = recent.command.id;
    let client = Client::new("http://127.0.0.1:1", "fixture-test-only-credential-123456").unwrap();
    let (tx, _) = mpsc::channel(8);
    let mut jobs = JoinSet::new();
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::RemoteWork(Ok(lain_wire::device_work::DeviceWorkPage {
            work: vec![recent, old],
            truncated: false,
        })),
    )
    .unwrap();
    assert!(app.text_view.doc.entries.is_empty());
    assert!(draw::task_text(&app.remote_work[0].command, None).contains(&id.to_string()));
    assert_eq!(
        app.remote_work[0].command.id, id,
        "Work list still sorts newest first"
    );
}

#[test]
fn page_navigation_restores_read_position_without_losing_chat_draft() {
    let mut app = Model::new(ConversationId::new());
    app.input.insert("保留这份草稿");
    app.scroll = 12;
    app.follow = false;
    app.navigate(Surface::Work);
    app.selected = 3;
    app.back();
    assert_eq!(app.scroll, 12);
    assert!(!app.follow);
    assert_eq!(app.input.text, "保留这份草稿");
    app.navigate(Surface::Work);
    assert_eq!(app.selected, 3);
}

#[test]
fn portrait_asset_rejects_terminal_controls_and_incomplete_or_double_width_rows() {
    let grid = vec!["⣿".repeat(29); 18].join("\n");
    assert_eq!(portrait::parse(&grid).unwrap(), grid);
    assert!(portrait::parse(&grid.replace('⣿', "界")).is_err());
    assert!(portrait::parse(&grid.replace('⣿', "\u{1b}")).is_err());
    assert!(portrait::parse(&grid.replace('⣿', "\u{202e}")).is_err());
    assert!(portrait::parse(&grid.lines().take(17).collect::<Vec<_>>().join("\n")).is_err());
}

#[test]
fn compact_welcome_shows_full_portrait_then_yields_to_chat_without_small_face() {
    use ratatui::{Terminal, backend::TestBackend};
    fn text(terminal: &Terminal<TestBackend>) -> String {
        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for row in buffer.content.chunks(usize::from(buffer.area.width)) {
            let mut column = 0;
            while column < row.len() {
                let symbol = row[column].symbol();
                text.push_str(symbol);
                column += unicode_width::UnicodeWidthStr::width(symbol).max(1);
            }
            text.push('\n');
        }
        text
    }
    let mut app = Model::new(ConversationId::new());
    app.portrait = Some(vec!["⣿".repeat(29); 18].join("\n"));
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|f| draw::render(f, &mut app)).unwrap();
    assert_eq!(text(&terminal).matches('⣿').count(), 29 * 18);
    app.input.insert("开始聊天");
    app.prepare().unwrap();
    app.input.insert("下一条草稿");
    terminal.draw(|f| draw::render(f, &mut app)).unwrap();
    assert!(!text(&terminal).contains('⣿'));
    assert!(text(&terminal).contains("下一条草稿"));
    let mut wide = Terminal::new(TestBackend::new(120, 40)).unwrap();
    wide.draw(|f| draw::render(f, &mut app)).unwrap();
    assert_eq!(text(&wide).matches('⣿').count(), 29 * 18);
}

#[test]
fn composer_preserves_pasted_request_and_unicode_editing() {
    let mut input = Composer::default();
    input.insert("你好\r\nread src/main.rs\n");
    assert_eq!(input.text, "你好\r\nread src/main.rs\n");
    input.key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Home,
        crossterm::event::KeyModifiers::NONE,
    ));
    input.key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Delete,
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(input.text.starts_with("好\r\n"));
    input.insert("\u{1b}]52;clipboard");
    assert!(!input.text.contains('\u{1b}'));
}

#[test]
fn navigating_keeps_draft_and_never_creates_a_worker_mutation() {
    let mut app = Model::new(lain_wire::ConversationId::new());
    app.input.insert("next question 草稿");
    app.surface = Surface::Work;
    app.back();
    assert_eq!(app.surface, Surface::Chat);
    assert_eq!(app.input.text, "next question 草稿");
    app.surface = Surface::Devices;
    app.back();
    assert_eq!(app.input.text, "next question 草稿");
    assert!(app.pending.is_none());
    assert!(app.turn.is_none());
}

#[test]
fn leaving_a_loading_page_allows_chat_recovery_and_discards_other_device_rows() {
    let mut app = Model::new(ConversationId::new());
    app.navigate(Surface::Tasks(DeviceId::new()));
    app.refreshing = true;
    app.back();
    assert!(
        !app.refreshing,
        "a departed page must not block chat refresh"
    );
    app.tasks.push(DeviceCommandSummary {
        id: DeviceCommandId::new(),
        revision: 1,
        state: lain_wire::device_commands::CommandState::Queued,
        cancel_requested: false,
        profile_id: "fixture".into(),
        workspace_id: lain_wire::DeviceWorkspaceId::new(),
        created_at_ms: 1,
        expires_at_ms: 2,
    });
    app.navigate(Surface::Tasks(DeviceId::new()));
    assert!(
        app.tasks.is_empty(),
        "another device's cached tasks must not be displayed"
    );
}

#[test]
fn unknown_submission_retains_its_identity_without_rewriting_new_draft() {
    let mut app = Model::new(lain_wire::ConversationId::new());
    app.input.insert("original exact request");
    let p = app.prepare().unwrap();
    app.input.insert("new unsent draft");
    app.submission_failed();
    assert_eq!(app.pending.as_ref().unwrap().id, p.id);
    assert_eq!(app.pending.as_ref().unwrap().text, "original exact request");
    assert_eq!(app.input.text, "new unsent draft");
    assert!(app.prepare().is_none());
    assert_eq!(app.pending.as_ref().unwrap().id, p.id);
}

#[tokio::test]
async fn missing_chat_model_preserves_draft_and_does_not_submit_or_claim_disconnection() {
    let mut app = Model::new(ConversationId::new());
    app.input.insert("hello 保留原文");
    let client = Client::new("http://127.0.0.1:1", "fixture-chat-status-only-credential").unwrap();
    let (tx, _) = mpsc::channel(8);
    let mut jobs = JoinSet::new();
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::ChatStatus(Ok(chat_status::Availability::NotConfigured)),
    )
    .unwrap();
    handle_key(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    )
    .unwrap();
    assert!(
        app.pending.is_none() && jobs.is_empty(),
        "known missing model must not make an uncertain request"
    );
    assert_eq!(app.input.text, "hello 保留原文");
    assert!(
        app.connected,
        "model readiness is separate from Kernel transport"
    );
    assert!(app.status.contains("model is not configured"));
}

#[tokio::test]
async fn repeated_enter_preserves_exact_recovery_instructions_and_new_draft() {
    let mut app = Model::new(ConversationId::new());
    app.input.insert("original request");
    let original = app.prepare().unwrap();
    app.submission_failed();
    app.input.insert("new draft");
    let client = Client::new("http://127.0.0.1:1", "fixture-chat-status-only-credential").unwrap();
    let (tx, _) = mpsc::channel(8);
    let mut jobs = JoinSet::new();
    handle_key(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    )
    .unwrap();
    assert!(
        app.status.contains("/refresh") && app.status.contains("/retry"),
        "recovery guidance was lost: {}",
        app.status
    );
    assert_eq!(app.pending.as_ref().unwrap().id, original.id);
    assert_eq!(app.pending.as_ref().unwrap().text, original.text);
    assert_eq!(app.input.text, "new draft");
    assert!(jobs.is_empty());
}

pub(super) fn running(conversation: ConversationId) -> ActorSnapshot {
    ActorSnapshot {
        availability: None,
        effect_receipts: vec![],
        reviewed_memory_receipts: vec![],
        memory_change_receipts: vec![],
        id: lain_wire::ActorTurnId::new(),
        conversation_id: conversation,
        request_id: RequestId::new(),
        sequence: 1,
        revision: 1,
        state: ActorTurnState::Running,
        messages: vec![],
        actions: vec![],
        error: None,
        model_requests: 1,
        usage_tokens: 0,
        timing: Default::default(),
        reading_activity: None,
        briefing_activity: None,
        sharing_activity: None,
        channel_memory: None,
        channel_source: None,
        private_chat: None,
    }
}

#[test]
fn startup_sync_same_revision_withholding_erases_old_rendering_and_rejects_late_body() {
    let mut app = Model::new(ConversationId::new());
    app.input.insert("keep this next draft");
    let mut t = running(app.conversation);
    t.state = ActorTurnState::Completed;
    t.messages.push(lain_wire::ActorMessage {
        id: MessageId::new(),
        conversation_id: app.conversation,
        role: MessageRole::Assistant,
        content: "withdrawn-ui-marker".into(),
        timestamp_ms: 1,
        status: lain_wire::MessageStatus::Completed,
    });
    app.snapshot(t.clone()).unwrap();
    assert!(app.transcript.contains("withdrawn-ui-marker"));
    let mut withheld = t.clone();
    withheld.availability = Some(lain_wire::ActorAvailability::SourceUnavailable);
    withheld.messages.clear();
    withheld.actions.clear();
    withheld.error = Some("Source unavailable; previous content withheld.".into());
    app.snapshot(withheld).unwrap();
    assert!(!app.transcript.contains("withdrawn-ui-marker"));
    app.snapshot(t).unwrap();
    assert!(!app.transcript.contains("withdrawn-ui-marker"));
    assert_eq!(app.input.text, "keep this next draft");
}

#[tokio::test]
async fn startup_sync_checked_refresh_replaces_history_and_rejects_late_generations() {
    let mut app = Model::new(ConversationId::new());
    app.input.insert("next draft stays");
    let mut old = running(app.conversation);
    old.state = ActorTurnState::Completed;
    old.messages.push(lain_wire::ActorMessage {
        id: MessageId::new(),
        conversation_id: app.conversation,
        role: MessageRole::Assistant,
        content: "old-bootstrap-marker".into(),
        timestamp_ms: 1,
        status: lain_wire::MessageStatus::Completed,
    });
    let mut snapshot = bootstrap_page(app.conversation, vec![old.clone()]);
    snapshot.cursor.revision = 7;
    app.bootstrap(snapshot.clone()).unwrap();
    let previous_generation = app.generation;
    let mut withheld = old.clone();
    withheld.availability = Some(lain_wire::ActorAvailability::SourceUnavailable);
    withheld.messages.clear();
    withheld.error = Some("Source unavailable; previous content withheld.".into());
    app.snapshot(withheld.clone()).unwrap();
    assert!(app.source_invalidated);
    let client = Client::new(
        "http://127.0.0.1:1",
        "sync-fixture-owner-12345678901234567890",
    )
    .unwrap();
    let (tx, _rx) = mpsc::channel(8);
    let mut jobs = JoinSet::new();
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::History(previous_generation, Ok(snapshot.clone())),
    )
    .unwrap();
    assert!(!app.transcript.contains("old-bootstrap-marker"));
    assert!(jobs.is_empty());
    let mut fresh = snapshot.clone();
    fresh.cursor.revision = 8;
    fresh.turns = vec![withheld];
    let mut next = running(app.conversation);
    next.sequence = 2;
    next.state = ActorTurnState::Completed;
    next.messages.push(lain_wire::ActorMessage {
        id: MessageId::new(),
        conversation_id: app.conversation,
        role: MessageRole::Assistant,
        content: "fresh permitted reply".into(),
        timestamp_ms: 2,
        status: lain_wire::MessageStatus::Completed,
    });
    fresh.turns.push(next);
    let generation = app.generation;
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::History(generation, Ok(fresh)),
    )
    .unwrap();
    assert!(!app.transcript.contains("old-bootstrap-marker"));
    assert!(app.transcript.contains("fresh permitted reply"));
    assert!(!app.bootstrap(snapshot).unwrap());
    assert!(!app.transcript.contains("old-bootstrap-marker"));
    assert_eq!(app.input.text, "next draft stays");
    assert!(app.pending.is_none() && jobs.is_empty());
}

#[tokio::test]
async fn startup_sync_late_history_cannot_regress_a_newer_live_completion() {
    let mut app = Model::new(ConversationId::new());
    let mut turn = running(app.conversation);
    let initial = bootstrap_page(app.conversation, vec![turn.clone()]);
    app.bootstrap(initial.clone()).unwrap();
    let generation = app.generation;
    app.refreshing = true;
    let mut delayed = initial;
    delayed.cursor.revision += 1;
    turn.revision += 2;
    turn.state = ActorTurnState::Completed;
    turn.messages.push(lain_wire::ActorMessage {
        id: MessageId::new(),
        conversation_id: app.conversation,
        role: MessageRole::Assistant,
        content: "newer-live-completion".into(),
        timestamp_ms: 3,
        status: lain_wire::MessageStatus::Completed,
    });
    app.snapshot(turn).unwrap();
    let client = Client::new(
        "http://127.0.0.1:1",
        "sync-fixture-owner-12345678901234567890",
    )
    .unwrap();
    let (tx, _rx) = mpsc::channel(8);
    let mut jobs = JoinSet::new();
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::History(generation, Ok(delayed)),
    )
    .unwrap();
    assert_eq!(app.turn.as_ref().unwrap().state, ActorTurnState::Completed);
    assert!(app.transcript.contains("newer-live-completion"));
}

pub(super) fn idle_remote(
    conversation: ConversationId,
) -> (lain_wire::device_work::DeviceWorkRecord, DeviceSnapshot) {
    use lain_wire::{device_execution::*, devices::*};
    let mut r = remote_record(conversation);
    r.command.state = lain_wire::device_commands::CommandState::Succeeded;
    r.command.execution = Some(ExecutionEvidence {
        execution_id: lain_wire::DeviceExecutionId::new(),
        session_id: Some(lain_wire::WorkSessionId::new()),
        turn_id: Some(lain_wire::WorkerTurnId::new()),
        phase: ExecutionPhase::Completed,
        pending_approvals: vec![],
        approval_reports: vec![],
        result: Some(DeviceWorkResult {
            output_tail: "done".into(),
            command_failures: 0,
            reported_paths: vec![],
            paths_withheld: 0,
            history_truncated: false,
        }),
    });
    let snapshot = DeviceSnapshot {
        device: Device {
            registration: RegisterDevice {
                id: r.command.device_id,
                request_id: RequestId::new(),
                label: "fixture".into(),
                platform: DevicePlatform::Windows,
                credential_fingerprint: "0".repeat(64),
                workspaces: vec![DeviceWorkspace {
                    id: r.command.input.workspace_id,
                    path: "C:\\fixture".into(),
                }],
                capabilities: vec![],
                expires_at_ms: u64::MAX,
            },
            state: DeviceState::Active,
            revision: 2,
            registered_at_ms: 1,
            claim_until_ms: 1,
            claimed_at_ms: Some(1),
            claim_request_id: Some(RequestId::new()),
            revoked_at_ms: None,
        },
        expired: false,
        latest_report: None,
    };
    (r, snapshot)
}

#[test]
fn remote_followup_draft_and_exact_preview_stay_separate_from_lain_chat() {
    let mut app = Model::new(ConversationId::new());
    let (r, d) = idle_remote(app.conversation);
    app.input.insert("Lain 的聊天草稿");
    followup::opened(&mut app, r.clone(), d.clone()).unwrap();
    app.followup
        .as_mut()
        .unwrap()
        .input
        .insert("Worker 原文\nnext instruction");
    followup::preview(&mut app).unwrap();
    let p = &app
        .followup
        .as_ref()
        .unwrap()
        .preview
        .as_ref()
        .unwrap()
        .pending;
    assert_eq!(p.input.text, "Worker 原文\nnext instruction");
    assert_eq!(p.parent, r.command.id);
    assert!(
        app.followup
            .as_ref()
            .unwrap()
            .preview
            .as_ref()
            .unwrap()
            .text
            .contains("Unchanged")
    );
    followup::close(&mut app);
    app.navigate(Surface::Chat);
    assert_eq!(app.input.text, "Lain 的聊天草稿");
    followup::opened(&mut app, r, d).unwrap();
    assert_eq!(
        app.followup.as_ref().unwrap().input.text,
        "Worker 原文\nnext instruction"
    );
    assert!(app.followup.as_ref().unwrap().preview.is_none());
}

#[test]
fn remote_followup_refuses_busy_or_already_continued_target_and_keeps_unknown_identity() {
    let mut app = Model::new(ConversationId::new());
    let (mut r, d) = idle_remote(app.conversation);
    r.command.state = lain_wire::device_commands::CommandState::Running;
    assert!(followup::opened(&mut app, r.clone(), d.clone()).is_err());
    r.command.state = lain_wire::device_commands::CommandState::Succeeded;
    r.command.continued_by = Some(DeviceCommandId::new());
    assert!(followup::opened(&mut app, r.clone(), d.clone()).is_err());
    r.command.continued_by = None;
    followup::opened(&mut app, r, d).unwrap();
    app.followup
        .as_mut()
        .unwrap()
        .input
        .insert("unchanged pending request");
    followup::preview(&mut app).unwrap();
    let p = app
        .followup
        .as_ref()
        .unwrap()
        .preview
        .as_ref()
        .unwrap()
        .pending
        .clone();
    app.followup_pending = Some(p.clone());
    app.followup_busy = true;
    followup::complete(
        &mut app,
        p.clone(),
        Err(followup::Failure::Uncertain(anyhow::anyhow!(
            "fixture response lost"
        ))),
    )
    .unwrap();
    assert_eq!(
        app.followup_pending.as_ref().unwrap().input.request_id,
        p.input.request_id
    );
    assert_eq!(
        app.followup_pending.as_ref().unwrap().input.text,
        "unchanged pending request"
    );
    assert!(!app.followup_busy);
}

#[tokio::test]
async fn remote_followup_requires_contiguous_scope_read_before_any_submission() {
    use ratatui::{Terminal, backend::TestBackend};
    let mut app = Model::new(ConversationId::new());
    let (r, d) = idle_remote(app.conversation);
    followup::opened(&mut app, r, d).unwrap();
    app.followup
        .as_mut()
        .unwrap()
        .input
        .insert(&"worker line\n".repeat(200));
    followup::preview(&mut app).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let client = Client::new("http://127.0.0.1:1", "fixture-test-only-credential-123456").unwrap();
    let (tx, _) = mpsc::channel(8);
    let mut jobs = JoinSet::new();
    terminal.draw(|f| draw::render(f, &mut app)).unwrap();
    handle_key(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
    )
    .unwrap();
    assert!(jobs.is_empty() && app.followup_pending.is_none());
    app.followup
        .as_mut()
        .unwrap()
        .preview
        .as_mut()
        .unwrap()
        .scroll = u16::MAX;
    terminal.draw(|f| draw::render(f, &mut app)).unwrap();
    assert!(
        !app.followup
            .as_ref()
            .unwrap()
            .preview
            .as_ref()
            .unwrap()
            .read,
        "jumping over unread scope must not enable send"
    );
    {
        let p = app.followup.as_mut().unwrap().preview.as_mut().unwrap();
        p.scroll = 0;
        p.seen_to = 0;
    }
    for _ in 0..100 {
        terminal.draw(|f| draw::render(f, &mut app)).unwrap();
        if app
            .followup
            .as_ref()
            .unwrap()
            .preview
            .as_ref()
            .unwrap()
            .read
        {
            break;
        }
        handle_key(
            &client,
            &mut app,
            &tx,
            &mut jobs,
            KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE),
        )
        .unwrap();
    }
    assert!(
        app.followup
            .as_ref()
            .unwrap()
            .preview
            .as_ref()
            .unwrap()
            .read
    );
    handle_key(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
    )
    .unwrap();
    assert!(app.followup_pending.is_some() && jobs.len() == 1);
    jobs.abort_all();
}

#[tokio::test]
async fn remote_work_family_is_one_worker_row_and_enter_targets_its_current_turn() {
    let mut app = Model::new(ConversationId::new());
    let (mut parent, _) = idle_remote(app.conversation);
    let mut child = parent.clone();
    child.command.id = DeviceCommandId::new();
    let e = child.command.execution.as_mut().unwrap();
    e.execution_id = lain_wire::DeviceExecutionId::new();
    e.turn_id = Some(lain_wire::WorkerTurnId::new());
    child.command.continuation = Some(lain_wire::device_work::DeviceContinuation {
        root_command_id: parent.command.id,
        parent_command_id: parent.command.id,
        expected_parent_revision: parent.command.revision,
        session_id: e.session_id.unwrap(),
        parent_turn_id: parent.command.execution.as_ref().unwrap().turn_id.unwrap(),
    });
    parent.command.continued_by = Some(child.command.id);
    let target = (child.command.device_id, child.command.id);
    app.remote_work = vec![child, parent];
    assert_eq!(app.remote_heads().count(), 1);
    app.navigate(Surface::Work);
    app.input.insert("Lain draft");
    let client = Client::new("http://127.0.0.1:1", "fixture-test-only-credential-123456").unwrap();
    let (tx, _) = mpsc::channel(8);
    let mut jobs = JoinSet::new();
    let flow = handle_key(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    )
    .unwrap();
    assert!(matches!(flow, Flow::Continue));
    assert_eq!(app.surface, Surface::Task(target.0, target.1));
    assert_eq!(app.input.text, "Lain draft");
    jobs.abort_all();
}

#[test]
fn late_followup_receipt_does_not_regress_newer_completed_device_evidence() {
    let mut app = Model::new(ConversationId::new());
    let (parent, d) = idle_remote(app.conversation);
    followup::opened(&mut app, parent.clone(), d).unwrap();
    app.followup
        .as_mut()
        .unwrap()
        .input
        .insert("exact delayed request");
    followup::preview(&mut app).unwrap();
    let p = app
        .followup
        .as_ref()
        .unwrap()
        .preview
        .as_ref()
        .unwrap()
        .pending
        .clone();
    app.followup_pending = Some(p.clone());
    app.followup_busy = true;
    let mut done = parent.clone();
    done.command.id = DeviceCommandId::new();
    done.command.revision = 4;
    done.command.input.request_id = p.input.request_id;
    done.command.input.task = p.input.text.clone();
    done.command.input.expires_at_ms = p.input.expires_at_ms;
    done.command.continuation = Some(lain_wire::device_work::DeviceContinuation {
        root_command_id: parent.command.id,
        parent_command_id: parent.command.id,
        expected_parent_revision: parent.command.revision,
        session_id: parent
            .command
            .execution
            .as_ref()
            .unwrap()
            .session_id
            .unwrap(),
        parent_turn_id: parent.command.execution.as_ref().unwrap().turn_id.unwrap(),
    });
    let e = done.command.execution.as_mut().unwrap();
    e.execution_id = lain_wire::DeviceExecutionId::new();
    e.turn_id = Some(lain_wire::WorkerTurnId::new());
    let mut queued = done.command.clone();
    queued.state = lain_wire::device_commands::CommandState::Queued;
    queued.execution = None;
    queued.revision = 1;
    app.remote_work = vec![done];
    app.navigate(Surface::Task(p.device, p.parent));
    followup::complete(&mut app, p, Ok(Some(queued))).unwrap();
    assert_eq!(
        app.task.as_ref().unwrap().state,
        lain_wire::device_commands::CommandState::Succeeded
    );
    assert_eq!(app.task.as_ref().unwrap().revision, 4);
}

#[tokio::test]
async fn remote_followup_refresh_retains_both_drafts_and_requires_a_fresh_target_read() {
    let mut app = Model::new(ConversationId::new());
    let (r, d) = idle_remote(app.conversation);
    app.input.insert("Lain draft");
    followup::opened(&mut app, r, d).unwrap();
    app.followup.as_mut().unwrap().input.insert("Worker draft");
    let client = Client::new("http://127.0.0.1:1", "fixture-test-only-credential-123456").unwrap();
    let (tx, _) = mpsc::channel(8);
    let mut jobs = JoinSet::new();
    handle_key(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
    )
    .unwrap();
    assert!(
        app.followup.is_none() && jobs.len() == 1,
        "refresh must reread target instead of keeping stale approval scope"
    );
    assert_eq!(app.input.text, "Lain draft");
    assert!(app.worker_drafts.values().any(|c| c.text == "Worker draft"));
    assert!(
        app.followup_pending.is_none(),
        "refresh must not submit work"
    );
    jobs.abort_all();
}

fn dispatch_turn(originals: &[&str], task: &str) -> ActorSnapshot {
    use lain_wire::{ActionId, ActorAction, ActorMessage, MessageStatus, ToolCall};
    let mut turn = running(ConversationId::new());
    turn.state = ActorTurnState::PendingConfirmation;
    turn.messages = originals
        .iter()
        .map(|text| ActorMessage {
            id: MessageId::new(),
            conversation_id: turn.conversation_id,
            role: MessageRole::User,
            content: (*text).into(),
            timestamp_ms: 1,
            status: MessageStatus::Completed,
        })
        .collect();
    turn.actions.push(ActorAction {
        memory_review: None,
        memory_change_review: None,
        id: ActionId::new(),
        call_id: "dispatch-fixture".into(),
        tool: ToolCall::DeviceQueue(lain_wire::device_work::ProposeDeviceWork {
            device_id: DeviceId::new(),
            expected_device_revision: 2,
            workspace_id: lain_wire::DeviceWorkspaceId::new(),
            profile_id: "codex/local".into(),
            sandbox: lain_wire::Sandbox::ReadOnly,
            task: task.into(),
            expires_at_ms: 9999999999999,
        }),
        state: ActionState::Proposed,
        result: None,
    });
    turn
}

#[test]
fn dispatch_review_compares_verbatim_inputs_and_actual_outbound_payload() {
    let original = "读取项目\n保留原文\t  ";
    let mut turn = dispatch_turn(&[original], original);
    let mut app = Model::new(turn.conversation_id);
    app.snapshot(turn.clone()).unwrap();
    app.open_review().unwrap();
    let exact = &app.review.as_ref().unwrap().text;
    assert!(exact.contains("Unchanged"));
    assert!(exact.contains(&serde_json::to_string(original).unwrap()));
    assert!(
        exact.contains(&turn.actions[0].id.to_string()),
        "actual request identity must be inspectable"
    );
    if let lain_wire::ToolCall::DeviceQueue(p) = &mut turn.actions[0].tool {
        p.task = "检查项目并修改代码".into();
    }
    turn.revision += 1;
    app.snapshot(turn).unwrap();
    app.open_review().unwrap();
    let changed = &app.review.as_ref().unwrap().text;
    assert!(changed.contains("Changed"));
    assert!(
        changed.contains("- ") && changed.contains("+ "),
        "show the replacement, not only a status label"
    );
    assert!(changed.contains("检查项目并修改代码"));
}

#[test]
fn dispatch_review_does_not_invent_a_single_original_or_hide_terminal_controls() {
    let turn = dispatch_turn(&["one", "two\u{1b}[31m"], "one\ntwo");
    let mut app = Model::new(turn.conversation_id);
    app.snapshot(turn.clone()).unwrap();
    app.open_review().unwrap();
    let text = &app.review.as_ref().unwrap().text;
    assert!(text.contains("Changed"));
    for msg in &turn.messages {
        assert!(text.contains(&msg.id.to_string()));
        assert!(text.contains(&serde_json::to_string(&msg.content).unwrap()));
    }
    assert!(!text.contains('\u{1b}'));
    let no_origin = dispatch_turn(&[], "unattributed task");
    app.turn = Some(no_origin);
    assert!(
        app.open_review().is_err(),
        "cannot approve an unattributed dispatch"
    );
}

#[tokio::test]
async fn changed_original_invalidates_dispatch_review_even_at_the_same_revision() {
    let turn = dispatch_turn(&["read only"], "read only");
    let mut app = Model::new(turn.conversation_id);
    app.snapshot(turn).unwrap();
    app.open_review().unwrap();
    app.review.as_mut().unwrap().read = true;
    app.turn.as_mut().unwrap().messages[0].content = "different original".into();
    let client = Client::new("http://127.0.0.1:1", "fixture-test-only-credential-123456").unwrap();
    let (tx, _) = mpsc::channel(8);
    let mut jobs = JoinSet::new();
    handle_key(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
    )
    .unwrap();
    assert!(!app.request_busy && jobs.is_empty());
}

#[tokio::test]
async fn busy_enter_retains_next_draft_without_spawning_a_post() {
    let mut app = Model::new(ConversationId::new());
    app.snapshot(running(app.conversation)).unwrap();
    app.input.insert("next request\n原文");
    let client = Client::new("http://127.0.0.1:1", "fixture-test-only-credential-123456").unwrap();
    let (tx, _rx) = mpsc::channel(8);
    let mut jobs = JoinSet::new();
    handle_key(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    )
    .unwrap();
    assert_eq!(app.input.text, "next request\n原文");
    assert!(app.pending.is_none() && jobs.is_empty());
}

#[tokio::test]
async fn late_device_page_cannot_replace_current_selection_or_clear_its_refresh() {
    let mut app = Model::new(ConversationId::new());
    let first = DeviceId::new();
    let second = DeviceId::new();
    app.navigate(Surface::Tasks(first));
    let old = app.generation;
    app.navigate(Surface::Tasks(second));
    app.refreshing = true;
    let client = Client::new("http://127.0.0.1:1", "fixture-test-only-credential-123456").unwrap();
    let (tx, _rx) = mpsc::channel(8);
    let mut jobs = JoinSet::new();
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::Tasks(old, first, Ok(vec![])),
    )
    .unwrap();
    assert_eq!(app.surface, Surface::Tasks(second));
    assert!(app.refreshing);
    assert!(jobs.is_empty());
}

#[tokio::test]
async fn failed_history_read_marks_connection_unavailable_and_same_revision_can_reobserve() {
    let mut app = Model::new(ConversationId::new());
    let turn = running(app.conversation);
    let id = turn.id;
    app.snapshot(turn.clone()).unwrap();
    let client = Client::new("http://127.0.0.1:1", "fixture-test-only-credential-123456").unwrap();
    let (tx, _rx) = mpsc::channel(8);
    let mut jobs = JoinSet::new();
    let generation = app.generation;
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::History(generation, Err(anyhow::anyhow!("fixture unavailable"))),
    )
    .unwrap();
    assert!(!app.connected);
    let fresh_history = bootstrap_page(app.conversation, vec![turn]);
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::History(generation, Ok(fresh_history)),
    )
    .unwrap();
    assert!(app.connected && app.observing.contains(&id));
    assert_eq!(jobs.len(), 1);
}

#[tokio::test]
async fn stale_action_view_cannot_send_a_decision() {
    use lain_wire::{ActionId, ActorAction, Sandbox, ToolCall};
    let mut app = Model::new(ConversationId::new());
    let mut turn = running(app.conversation);
    turn.messages = dispatch_turn(&["exact scope"], "unused").messages;
    turn.messages[0].conversation_id = app.conversation;
    turn.state = ActorTurnState::PendingConfirmation;
    turn.actions.push(ActorAction {
        memory_review: None,
        memory_change_review: None,
        id: ActionId::new(),
        call_id: "test".into(),
        tool: ToolCall::WorkCreate {
            workspace: "/fixture".into(),
            profile_id: "codex/local".into(),
            sandbox: Sandbox::ReadOnly,
            task: "exact scope".into(),
        },
        state: ActionState::Proposed,
        result: None,
    });
    app.snapshot(turn.clone()).unwrap();
    app.open_review().unwrap();
    app.review.as_mut().unwrap().read = true;
    app.turn.as_mut().unwrap().revision += 1;
    let client = Client::new("http://127.0.0.1:1", "fixture-test-only-credential-123456").unwrap();
    let (tx, _rx) = mpsc::channel(8);
    let mut jobs = JoinSet::new();
    handle_key(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
    )
    .unwrap();
    assert!(!app.request_busy && jobs.is_empty());
    turn.revision += 2;
    app.snapshot(turn).unwrap();
    assert!(app.review.is_none());
}
