use super::{
    Store,
    device_commands_tests::{event, setup, task},
};
use lain_wire::{device_commands::*, device_execution::*, device_work::*, *};

fn evidence(session: WorkSessionId) -> ExecutionEvidence {
    ExecutionEvidence {
        execution_id: DeviceExecutionId::new(),
        session_id: Some(session),
        turn_id: Some(WorkerTurnId::new()),
        phase: ExecutionPhase::Running,
        pending_approvals: vec![],
        approval_reports: vec![],
        result: None,
    }
}
fn finish(
    s: &mut Store,
    r: &devices::RegisterDevice,
    c: &DeviceCommand,
    e: &ExecutionEvidence,
) -> DeviceCommand {
    for (seq, kind, phase) in [
        (1, CommandEventKind::Accepted, ExecutionPhase::Preparing),
        (2, CommandEventKind::Started, ExecutionPhase::Running),
        (3, CommandEventKind::Succeeded, ExecutionPhase::Completed),
    ] {
        let mut ev = event(seq, kind);
        let mut proof = e.clone();
        proof.phase = phase;
        if seq == 1 {
            proof.turn_id = None;
        }
        if seq == 3 {
            proof.result = Some(DeviceWorkResult {
                output_tail: "factual completion".into(),
                command_failures: 0,
                reported_paths: vec![],
                paths_withheld: 0,
                history_truncated: false,
            });
        }
        ev.execution = Some(proof);
        s.device_command_event(r.id, c.id, &r.credential_fingerprint, ev)
            .unwrap();
    }
    s.device_command_get(r.id, c.id, None).unwrap()
}
fn next(parent: &DeviceCommand) -> FollowupDeviceWork {
    FollowupDeviceWork {
        request_id: RequestId::new(),
        expected_device_revision: 2,
        expected_parent_revision: parent.revision,
        text: "second exact instruction\n原文".into(),
        expires_at_ms: parent.input.expires_at_ms,
    }
}

#[test]
fn followup_keeps_one_actual_session_with_distinct_turns_and_exact_restart_retry() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.db");
    let mut s = Store::open(&path).unwrap();
    let r = setup(&mut s, 31);
    let conversation = s.create_conversation().unwrap().id;
    let root = s
        .device_work_dispatch(conversation, r.id, task(&r), None)
        .unwrap();
    let e = evidence(WorkSessionId::new());
    let root = finish(&mut s, &r, &root, &e);
    let request = next(&root);
    let child = s
        .device_work_followup(conversation, r.id, root.id, request.clone(), None)
        .unwrap();
    let updated = s.device_command_get(r.id, root.id, None).unwrap();
    assert_eq!(updated.continued_by, Some(child.id));
    assert!(updated.revision > root.revision);
    assert_eq!(child.input.task, request.text);
    assert_eq!(child.input.workspace_id, root.input.workspace_id);
    assert_eq!(child.input.sandbox, root.input.sandbox);
    let lineage = child.continuation.as_ref().unwrap();
    assert_eq!(lineage.root_command_id, root.id);
    assert_eq!(lineage.parent_command_id, root.id);
    assert_eq!(lineage.session_id, e.session_id.unwrap());
    assert_eq!(lineage.parent_turn_id, e.turn_id.unwrap());
    let child_evidence = evidence(e.session_id.unwrap());
    let child = finish(&mut s, &r, &child, &child_evidence);
    assert_ne!(
        child.execution.as_ref().unwrap().turn_id,
        root.execution.as_ref().unwrap().turn_id
    );
    let grandchild = s
        .device_work_followup(conversation, r.id, child.id, next(&child), None)
        .unwrap();
    assert_eq!(
        grandchild.continuation.as_ref().unwrap().root_command_id,
        root.id
    );
    drop(s);
    let mut s = Store::open(&path).unwrap();
    assert_eq!(
        s.device_work_followup(conversation, r.id, root.id, request.clone(), None)
            .unwrap()
            .id,
        child.id
    );
    let mut different = request;
    different.text.push('!');
    assert!(
        s.device_work_followup(conversation, r.id, root.id, different, None)
            .is_err()
    );
    assert!(
        s.device_work_followup(conversation, r.id, root.id, next(&root), None)
            .is_err(),
        "a parent cannot silently fork"
    );
    assert_eq!(s.device_work_page(conversation).unwrap().work.len(), 3);
}

#[test]
fn followup_rejects_busy_stale_foreign_and_unrelated_session_binding_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let r = setup(&mut s, 32);
    let a = s.create_conversation().unwrap().id;
    let b = s.create_conversation().unwrap().id;
    let root = s.device_work_dispatch(a, r.id, task(&r), None).unwrap();
    assert!(
        s.device_work_followup(a, r.id, root.id, next(&root), None)
            .is_err()
    );
    let e = evidence(WorkSessionId::new());
    let root = finish(&mut s, &r, &root, &e);
    assert!(
        s.device_work_followup(b, r.id, root.id, next(&root), None)
            .is_err()
    );
    let mut stale = next(&root);
    stale.expected_parent_revision -= 1;
    assert!(
        s.device_work_followup(a, r.id, root.id, stale, None)
            .is_err()
    );
    let other = s.device_work_dispatch(a, r.id, task(&r), None).unwrap();
    let mut report = event(1, CommandEventKind::Accepted);
    let mut reused = e.clone();
    reused.phase = ExecutionPhase::Preparing;
    reused.turn_id = None;
    report.execution = Some(reused);
    assert!(
        s.device_command_event(r.id, other.id, &r.credential_fingerprint, report)
            .is_err()
    );
    s.connection.execute_batch("CREATE TRIGGER fail_continuation BEFORE INSERT ON device_work_continuations BEGIN SELECT RAISE(ABORT,'fixture failure'); END;").unwrap();
    assert!(
        s.device_work_followup(a, r.id, root.id, next(&root), None)
            .is_err()
    );
    assert_eq!(
        s.device_work_page(a).unwrap().work.len(),
        2,
        "failed lineage must not leave an executable child"
    );
}

#[test]
fn actor_followup_requires_confirmation_and_commits_exact_action_receipt_with_lineage() {
    use crate::actor::{domain as a, tools::Tool};
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let r = setup(&mut s, 33);
    let conversation = s.create_conversation().unwrap().id;
    let root = s
        .device_work_dispatch(conversation, r.id, task(&r), None)
        .unwrap();
    let proof = evidence(WorkSessionId::new());
    let parent = finish(&mut s, &r, &root, &proof);
    let mut request = next(&parent);
    let aid = a::ActionId::new();
    request.request_id = RequestId(aid.0);
    let args = serde_json::json!({"device_id":r.id,"parent_command_id":parent.id,"expected_device_revision":2,
        "expected_parent_revision":parent.revision,"text":request.text,"expires_at_ms":request.expires_at_ms});
    let tool = Tool::parse("device_send", &args.to_string()).unwrap();
    assert!(tool.mutating());
    let mut actor = a::ActorTurn {
        id: a::ActorTurnId::new(),
        conversation_id: conversation,
        request_id: uuid::Uuid::new_v4(),
        sequence: 0,
        revision: 0,
        state: a::TurnState::PendingConfirmation,
        messages: vec![],
        actions: vec![a::Action {
            id: aid,
            call_id: "device-send-fixture".into(),
            tool,
            state: a::ActionState::Proposed,
            result: None,
        }],
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
        .message(a::MessageRole::User, "在原 Codex 会话里继续检查结果".into())
        .unwrap();
    s.save_actor_turn(&mut actor, true).unwrap();
    let origin = crate::delegation::Origin {
        actor_turn_id: actor.id,
        action_id: aid,
    };
    assert!(
        s.device_work_followup(conversation, r.id, parent.id, request.clone(), Some(origin))
            .is_err()
    );
    actor.state = a::TurnState::Running;
    actor.actions[0].state = a::ActionState::Executing;
    s.save_actor_turn(&mut actor, false).unwrap();
    let child = s
        .device_work_followup(conversation, r.id, parent.id, request.clone(), Some(origin))
        .unwrap();
    let receipt = s.actor_turn(actor.id).unwrap().actions[0]
        .result
        .clone()
        .unwrap();
    assert_eq!(receipt["command_id"], serde_json::json!(child.id));
    assert_eq!(
        receipt["continuation"]["session_id"],
        serde_json::json!(proof.session_id)
    );
    assert_eq!(child.input.request_id, RequestId(aid.0));
    assert_eq!(
        s.device_work_followup(conversation, r.id, parent.id, request, Some(origin))
            .unwrap()
            .id,
        child.id
    );
}
