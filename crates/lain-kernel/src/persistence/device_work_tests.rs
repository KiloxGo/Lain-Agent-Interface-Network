use super::Store;
use super::device_commands_tests::{event, setup, task};
use crate::actor::{domain::*, tools::Tool};
use lain_wire::{RequestId, device_commands::CommandEventKind};

#[test]
fn direct_dispatch_receipt_is_scoped_durable_and_does_not_queue_again() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.db");
    let mut s = Store::open(&path).unwrap();
    let r = setup(&mut s, 31);
    let conversation = s.create_conversation().unwrap().id;
    let other = s.create_conversation().unwrap().id;
    let input = task(&r);
    let request = input.request_id;
    assert!(
        s.device_work_request_receipt(conversation, r.id, request)
            .unwrap()
            .is_none()
    );
    let c = s
        .device_work_dispatch(conversation, r.id, input, None)
        .unwrap();
    let unlinked = s.device_command_queue(r.id, task(&r)).unwrap();
    assert!(
        s.device_work_request_receipt(conversation, r.id, unlinked.input.request_id)
            .unwrap()
            .is_none()
    );
    assert!(
        s.device_work_request_receipt(other, r.id, request)
            .unwrap()
            .is_none()
    );
    let count = s.device_command_summaries(r.id).unwrap().len();
    drop(s);
    let mut s = Store::open(&path).unwrap();
    assert_eq!(
        s.device_work_request_receipt(conversation, r.id, request)
            .unwrap()
            .unwrap()
            .command
            .id,
        c.id
    );
    assert_eq!(s.device_command_summaries(r.id).unwrap().len(), count);
}

#[test]
fn dispatch_link_is_atomic_exact_and_cannot_adopt_or_move_an_unrelated_command() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let r = setup(&mut s, 21);
    let input = task(&r);
    assert!(
        s.device_work_dispatch(ConversationId::new(), r.id, input.clone(), None)
            .is_err()
    );
    assert!(s.device_command_summaries(r.id).unwrap().is_empty());
    let a = s.create_conversation().unwrap().id;
    let b = s.create_conversation().unwrap().id;
    let c = s
        .device_work_dispatch(a, r.id, input.clone(), None)
        .unwrap();
    assert_eq!(
        s.device_work_dispatch(a, r.id, input.clone(), None)
            .unwrap()
            .id,
        c.id
    );
    assert!(
        s.device_work_dispatch(b, r.id, input.clone(), None)
            .is_err()
    );
    let mut changed = input;
    changed.task.push_str(" silently wider scope");
    assert!(s.device_work_dispatch(a, r.id, changed, None).is_err());
    let unlinked = s.device_command_queue(r.id, task(&r)).unwrap();
    assert!(
        s.device_work_dispatch(a, r.id, unlinked.input, None)
            .is_err()
    );
    assert!(s.device_work_page(b).unwrap().work.is_empty());
    assert_eq!(s.device_work_page(a).unwrap().work.len(), 1);
    let local: i64 = s
        .connection
        .query_row("SELECT count(*) FROM work_sessions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        local, 0,
        "remote work must not fabricate a cloud-local session"
    );
}

#[test]
fn failed_link_commit_leaves_no_command_or_change_for_a_device_to_execute() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let r = setup(&mut s, 22);
    let conversation = s.create_conversation().unwrap().id;
    s.connection.execute_batch("CREATE TRIGGER fail_link BEFORE INSERT ON device_work_links BEGIN SELECT RAISE(ABORT,'fixture injected failure'); END;").unwrap();
    assert!(
        s.device_work_dispatch(conversation, r.id, task(&r), None)
            .is_err()
    );
    assert!(s.device_command_summaries(r.id).unwrap().is_empty());
    assert!(
        s.device_command_changes(r.id, None, Default::default())
            .unwrap()
            .changes
            .is_empty()
    );
}

#[test]
fn device_facts_recover_for_the_right_conversation_without_reexecution_or_new_actor() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.db");
    let mut s = Store::open(&path).unwrap();
    let r = setup(&mut s, 23);
    let conversation = s.create_conversation().unwrap().id;
    let c = s
        .device_work_dispatch(conversation, r.id, task(&r), None)
        .unwrap();
    for (sequence, kind) in [
        (1, CommandEventKind::Accepted),
        (2, CommandEventKind::Started),
        (3, CommandEventKind::Succeeded),
    ] {
        s.device_command_event(r.id, c.id, &r.credential_fingerprint, event(sequence, kind))
            .unwrap();
    }
    drop(s);
    let mut s = Store::open(&path).unwrap();
    let page = s.device_work_page(conversation).unwrap();
    assert_eq!(page.work.len(), 1);
    assert_eq!(
        page.work[0].command.state,
        lain_wire::device_commands::CommandState::Succeeded
    );
    assert_eq!(page.work[0].origin.conversation_id, conversation);
    assert_eq!(s.device_command_summaries(r.id).unwrap().len(), 1);
    let count: i64 = s
        .connection
        .query_row("SELECT count(*) FROM lain_actor_turns", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0, "device observations must not wake a model");
}

#[test]
fn confirmed_actor_dispatch_persists_its_partial_receipt_with_the_command() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let r = setup(&mut s, 24);
    let conversation = s.create_conversation().unwrap().id;
    let mut actor = ActorTurn {
        id: ActorTurnId::new(),
        conversation_id: conversation,
        request_id: uuid::Uuid::new_v4(),
        sequence: 0,
        revision: 0,
        state: TurnState::Running,
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
    };
    actor
        .message(
            MessageRole::User,
            "请让指定电脑上的 Codex 读取这个项目".into(),
        )
        .unwrap();
    let input = task(&r);
    let args = serde_json::json!({"device_id":r.id,"expected_device_revision":2,
        "workspace_id":input.workspace_id,"profile_id":"codex/local","sandbox":"read_only",
        "task":input.task,"expires_at_ms":input.expires_at_ms});
    let tool = Tool::parse("device_queue", &args.to_string()).unwrap();
    assert!(tool.mutating());
    let aid = ActionId::new();
    actor.actions.push(Action {
        id: aid,
        call_id: "fixture".into(),
        tool,
        state: ActionState::Proposed,
        result: None,
    });
    actor.state = TurnState::PendingConfirmation;
    s.save_actor_turn(&mut actor, true).unwrap();
    let mut input = input;
    input.request_id = RequestId(aid.0);
    let origin = crate::delegation::Origin {
        actor_turn_id: actor.id,
        action_id: aid,
    };
    assert!(
        s.device_work_dispatch(conversation, r.id, input.clone(), Some(origin))
            .is_err()
    );
    actor.state = TurnState::Running;
    actor.actions[0].state = ActionState::Executing;
    s.save_actor_turn(&mut actor, false).unwrap();
    let c = s
        .device_work_dispatch(conversation, r.id, input.clone(), Some(origin))
        .unwrap();
    let recorded = s.actor_turn(actor.id).unwrap();
    assert_eq!(
        recorded.actions[0].result.as_ref().unwrap()["command_id"],
        serde_json::json!(c.id)
    );
    assert_eq!(c.input.request_id, RequestId(aid.0));
    assert_eq!(
        s.device_work_dispatch(conversation, r.id, input, Some(origin))
            .unwrap()
            .id,
        c.id
    );
}
