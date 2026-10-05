use lain_kernel::{
    backend::Supervisor,
    domain::*,
    events::{Event, EventKind},
    kernel::Kernel,
};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::broadcast;

const TOKEN: &str = "fixture-owner-token-0123456789abcdef";

#[tokio::test]
async fn idle_detach_lazy_attachment_approval_and_shutdown_are_separate_from_work() {
    let (dir, kernel, workers) = setup_with_timeout(Duration::from_millis(60)).await;
    let mut events = kernel.subscribe();
    let session = workers
        .create(dir.path().to_owned(), SessionOptions::default())
        .await
        .unwrap();
    let identity = || {
        let value: serde_json::Value = serde_json::from_slice(
            &std::fs::read(dir.path().join("fixture-history.json")).unwrap(),
        )
        .unwrap();
        value["id"].clone()
    };
    let original = identity();
    wait(&mut events, |e| {
        e.work_session_id == session.id()
            && matches!(
                e.kind,
                EventKind::RuntimeAttachmentChanged {
                    to: RuntimeAttachment::Detached,
                    ..
                }
            )
    })
    .await;
    assert_eq!(
        kernel.session(session.id()).unwrap().unwrap().state(),
        WorkSessionState::Idle
    );
    let turn = workers.prompt(session.id(), "wait".into()).await.unwrap();
    assert_eq!(identity(), original);
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(
        kernel.session(session.id()).unwrap().unwrap().runtime(),
        RuntimeAttachment::Attached
    );
    assert!(workers.detach(session.id()).await.is_err());
    workers.cancel(session.id()).await.unwrap();
    wait(&mut events, |e| matches!(&e.kind, EventKind::TurnCompleted {turn: completed} if completed.id == turn.id)).await;
    let turn = workers
        .prompt(session.id(), "approve".into())
        .await
        .unwrap();
    let approval = wait(&mut events, |e| {
        matches!(e.kind, EventKind::ApprovalRequested { .. })
    })
    .await;
    let EventKind::ApprovalRequested { approval } = approval.kind else {
        unreachable!()
    };
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(
        kernel.session(session.id()).unwrap().unwrap().runtime(),
        RuntimeAttachment::Attached
    );
    assert!(workers.detach(session.id()).await.is_err());
    workers
        .decide(session.id(), approval.id, ApprovalDecision::Deny)
        .await
        .unwrap();
    wait(&mut events, |e| matches!(&e.kind, EventKind::TurnCompleted {turn: completed} if completed.id == turn.id)).await;
    workers.shutdown().await.unwrap();
    let snapshot = kernel.session(session.id()).unwrap().unwrap();
    assert_eq!(snapshot.state(), WorkSessionState::Idle);
    assert_eq!(snapshot.runtime(), RuntimeAttachment::Detached);
    while let Ok(event) = events.try_recv() {
        assert!(!matches!(event.kind, EventKind::BackendDisconnected { .. }));
    }
    drop(workers);
    drop(kernel);
    let kernel = Arc::new(Kernel::open(&dir.path().join("state.sqlite3")).unwrap());
    let workers = Supervisor::new(
        kernel.clone(),
        Some(PathBuf::from(env!("CARGO_BIN_EXE_lain-test-app-server"))),
        TOKEN.parse().unwrap(),
        Duration::from_secs(300),
    )
    .await;
    workers.recover().await;
    kernel.mark_ready().unwrap();
    assert_eq!(
        kernel.session(session.id()).unwrap().unwrap().runtime(),
        RuntimeAttachment::Detached
    );
    assert_eq!(
        kernel.session(session.id()).unwrap().unwrap().state(),
        WorkSessionState::Idle
    );
    workers.prompt(session.id(), "wait".into()).await.unwrap();
    assert_eq!(identity(), original);
    workers.shutdown().await.unwrap();
    assert_eq!(
        kernel.session(session.id()).unwrap().unwrap().state(),
        WorkSessionState::Unknown
    );
    assert_eq!(
        kernel.turns(session.id()).unwrap().last().unwrap().state,
        TurnState::Unknown
    );
}

async fn setup() -> (tempfile::TempDir, Arc<Kernel>, Arc<Supervisor>) {
    setup_with_timeout(Duration::from_secs(300)).await
}

#[tokio::test]
async fn unexpected_idle_loss_keeps_logical_idle_and_can_lazy_resume() {
    let (dir, kernel, workers) = setup().await;
    let session = workers
        .create(dir.path().to_owned(), SessionOptions::default())
        .await
        .unwrap();
    let mut events = kernel.subscribe();
    workers
        .prompt(session.id(), "idle-crash".into())
        .await
        .unwrap();
    wait(&mut events, |e| {
        matches!(e.kind, EventKind::BackendDisconnected { .. })
    })
    .await;
    let snapshot = kernel.session(session.id()).unwrap().unwrap();
    assert_eq!(snapshot.state(), WorkSessionState::Idle);
    assert_eq!(snapshot.runtime(), RuntimeAttachment::Lost);
    let turn = workers
        .prompt(session.id(), "after-idle-loss".into())
        .await
        .unwrap();
    wait(
        &mut events,
        |e| matches!(&e.kind, EventKind::TurnCompleted {turn: t} if t.id == turn.id),
    )
    .await;
    workers.shutdown().await.unwrap();
}

#[tokio::test]
async fn history_is_durable_ordered_isolated_redacted_and_progress_is_live_only() {
    let (dir, kernel, workers) = setup().await;
    let session = workers
        .create(dir.path().to_owned(), SessionOptions::default())
        .await
        .unwrap();
    let mut events = kernel.subscribe();
    let text = "中".repeat(12_000);
    // Input is bounded too; ask the fixture for a bounded, multi-byte completed record.
    let turn = workers
        .prompt(session.id(), "中".repeat(10_000))
        .await
        .unwrap();
    wait(
        &mut events,
        |e| matches!(&e.kind, EventKind::TurnCompleted {turn: t} if t.id == turn.id),
    )
    .await;
    let safe = workers
        .prompt(session.id(), "history-output".into())
        .await
        .unwrap();
    wait(
        &mut events,
        |e| matches!(&e.kind, EventKind::TurnCompleted {turn: t} if t.id == safe.id),
    )
    .await;
    let secret_after_limit = format!("{text}sk-secret-split-across-chunks");
    assert!(
        workers
            .prompt(session.id(), secret_after_limit)
            .await
            .is_err()
    );
    let mut cursor = 0;
    let mut history = Vec::new();
    loop {
        let page = kernel.history(session.id(), cursor, None, 3).unwrap();
        cursor = page.next_cursor;
        history.extend(page.events);
        if !page.has_more {
            break;
        }
    }
    assert!(
        history
            .windows(2)
            .all(|pair| pair[0].history_order < pair[1].history_order)
    );
    assert!(
        history
            .iter()
            .any(|e| matches!(e.kind, EventKind::AssistantMessageCompleted { .. }))
    );
    assert!(
        history
            .iter()
            .all(|e| !matches!(e.kind, EventKind::OutputProgress { .. }))
    );
    assert!(
        history
            .iter()
            .all(|e| !serde_json::to_string(e).unwrap().contains("sk-secret-"))
    );
    assert!(history.iter().any(|e| e.truncated
        && matches!(&e.kind, EventKind::AssistantMessageCompleted {text} if text.len() <= 32768)));
    assert!(history.iter().any(|e| e.truncated && matches!(&e.kind, EventKind::CommandCompleted {output,exit_code:Some(0),..} if output.len() <= 16384)));
    assert!(history.iter().any(|e| matches!(&e.kind, EventKind::CommandCompleted {output,..} if output == "[sensitive output redacted]")));
    assert!(
        history.iter().any(|e| e.truncated
            && matches!(&e.kind, EventKind::FileChanged {paths} if paths.len() == 16))
    );
    assert!(history.iter().any(|e| matches!(
        e.kind,
        EventKind::ToolActivity {
            completed: true,
            ..
        }
    )));
    let frozen = kernel.snapshot_and_subscribe().unwrap().0.history_cursor;
    let recent = kernel
        .history_before(session.id(), frozen + 1, Some(frozen), 3)
        .unwrap();
    assert_eq!(recent.events, history[history.len() - 3..]);
    assert!(recent.has_more);
    let before = recent.events[0].history_order.unwrap();
    let older = kernel
        .history_before(session.id(), before, None, 3)
        .unwrap();
    assert!(
        older
            .events
            .iter()
            .all(|e| e.history_order.unwrap() < before)
    );
    workers.prompt(session.id(), "wait".into()).await.unwrap();
    assert_eq!(
        kernel
            .history(session.id(), 0, Some(frozen), 200)
            .unwrap()
            .events,
        history
    );
    workers.shutdown().await.unwrap();
    let persisted = kernel.history(session.id(), 0, None, 200).unwrap().events;
    drop(workers);
    drop(kernel);
    let reopened = Kernel::open(&dir.path().join("state.sqlite3")).unwrap();
    assert_eq!(
        reopened.history(session.id(), 0, None, 200).unwrap().events,
        persisted
    );
    reopened.mark_ready().unwrap();
    let other = reopened
        .create_session(
            "codex".parse().unwrap(),
            "codex/local".parse().unwrap(),
            dir.path().to_owned(),
        )
        .unwrap();
    assert!(
        reopened
            .history(other.id(), 0, None, 200)
            .unwrap()
            .events
            .iter()
            .all(|e| e.work_session_id == other.id())
    );
    assert!(reopened.history(session.id(), 0, None, 0).is_err());
    assert!(
        reopened
            .history(WorkSessionId::new(), 0, None, 100)
            .is_err()
    );
}

async fn setup_with_timeout(
    timeout: Duration,
) -> (tempfile::TempDir, Arc<Kernel>, Arc<Supervisor>) {
    let dir = tempfile::tempdir().unwrap();
    let kernel = Arc::new(Kernel::open(&dir.path().join("state.sqlite3")).unwrap());
    kernel.mark_ready().unwrap();
    let workers = Supervisor::new(
        kernel.clone(),
        Some(PathBuf::from(env!("CARGO_BIN_EXE_lain-test-app-server"))),
        TOKEN.parse().unwrap(),
        timeout,
    )
    .await;
    assert!(workers.info().available);
    (dir, kernel, workers)
}

async fn wait(events: &mut broadcast::Receiver<Event>, matches: impl Fn(&Event) -> bool) -> Event {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let event = events.recv().await.unwrap();
            if matches(&event) {
                return event;
            }
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn two_workers_followup_cancellation_approval_crash_and_recovery() {
    let (dir, kernel, workers) = setup().await;
    let first = dir.path().join("first");
    let second = dir.path().join("second");
    std::fs::create_dir(&first).unwrap();
    std::fs::create_dir(&second).unwrap();
    let a = workers
        .create(first, SessionOptions::default())
        .await
        .unwrap();
    let b = workers
        .create(second, SessionOptions::default())
        .await
        .unwrap();
    let mut events = kernel.subscribe();
    let a_turn = workers.prompt(a.id(), "wait".into()).await.unwrap();
    let b_turn = workers.prompt(b.id(), "wait".into()).await.unwrap();
    let requested = workers.cancel(a.id()).await.unwrap();
    assert_eq!(requested.cancellation, CancellationState::Acknowledged);
    wait(&mut events, |e| {
        e.work_session_id == a.id()
            && matches!(&e.kind,EventKind::TurnCompleted{turn} if turn.state==TurnState::Cancelled)
    })
    .await;
    assert_eq!(kernel.turns(b.id()).unwrap()[0].state, TurnState::Running);
    assert_ne!(a_turn.id, b_turn.id);
    assert_eq!(
        kernel.session(a.id()).unwrap().unwrap().state(),
        WorkSessionState::Idle
    );
    drop(events); // No frontend/observer is required for the other worker to remain active.
    let mut events = kernel.subscribe();
    workers.cancel(b.id()).await.unwrap();
    wait(&mut events, |e| {
        e.work_session_id == b.id() && matches!(e.kind, EventKind::TurnCompleted { .. })
    })
    .await;
    for text in ["approve", "approve-string", "permissions", "file-approval"] {
        let turn = workers.prompt(a.id(), text.into()).await.unwrap();
        let event = wait(&mut events, |e| {
            e.work_session_id == a.id() && matches!(e.kind, EventKind::ApprovalRequested { .. })
        })
        .await;
        let EventKind::ApprovalRequested { approval } = event.kind else {
            unreachable!()
        };
        assert_eq!(approval.turn_id, Some(turn.id));
        assert_eq!(
            kernel.session(a.id()).unwrap().unwrap().state(),
            WorkSessionState::WaitingForApproval
        );
        assert!(
            workers
                .decide(b.id(), approval.id, ApprovalDecision::ApproveOnce)
                .await
                .is_err()
        );
        workers
            .decide(a.id(), approval.id, ApprovalDecision::ApproveOnce)
            .await
            .unwrap();
        wait(&mut events,|e|matches!(&e.kind,EventKind::ApprovalResolved{approval:resolved} if resolved.id==approval.id && resolved.status==ApprovalStatus::Resolved)).await;
        wait(
            &mut events,
            |e| matches!(&e.kind,EventKind::TurnCompleted{turn:finished} if finished.id==turn.id),
        )
        .await;
        assert!(
            workers
                .decide(a.id(), approval.id, ApprovalDecision::ApproveOnce)
                .await
                .is_err()
        );
    }
    let completed = workers.prompt(a.id(), "marker-alpha".into()).await.unwrap();
    wait(&mut events, |e| {
        matches!(&e.kind,EventKind::AssistantMessageCompleted{text} if text=="marker-alpha")
            && e.work_session_id == a.id()
    })
    .await;
    wait(
        &mut events,
        |e| matches!(&e.kind,EventKind::TurnCompleted{turn} if turn.id==completed.id),
    )
    .await;
    assert!(workers.prompt(a.id(), "reject".into()).await.is_err());
    assert_eq!(
        kernel.session(a.id()).unwrap().unwrap().state(),
        WorkSessionState::Idle
    );
    let crashed = workers.prompt(a.id(), "crash".into()).await.unwrap();
    assert!(workers.cancel(a.id()).await.is_err());
    wait(&mut events, |e| {
        e.work_session_id == a.id() && matches!(e.kind, EventKind::BackendDisconnected { .. })
    })
    .await;
    assert_eq!(
        kernel.turns(a.id()).unwrap().last().unwrap().state,
        TurnState::Unknown
    );
    workers.resume(a.id()).await.unwrap();
    assert_eq!(
        kernel
            .turns(a.id())
            .unwrap()
            .iter()
            .find(|t| t.id == crashed.id)
            .unwrap()
            .state,
        TurnState::Cancelled
    );
    workers.shutdown().await.unwrap();
    let before = kernel.turns(a.id()).unwrap();
    drop(workers);
    drop(kernel);
    let kernel = Arc::new(Kernel::open(&dir.path().join("state.sqlite3")).unwrap());
    assert_eq!(
        kernel.session(a.id()).unwrap().unwrap().state(),
        WorkSessionState::Idle
    );
    let workers = Supervisor::new(
        kernel.clone(),
        Some(PathBuf::from(env!("CARGO_BIN_EXE_lain-test-app-server"))),
        TOKEN.parse().unwrap(),
        Duration::from_secs(300),
    )
    .await;
    workers.recover().await;
    kernel.mark_ready().unwrap();
    assert_eq!(kernel.list_sessions().unwrap().len(), 2);
    assert_eq!(
        kernel
            .turns(a.id())
            .unwrap()
            .iter()
            .map(|t| t.id)
            .collect::<Vec<_>>(),
        before.iter().map(|t| t.id).collect::<Vec<_>>()
    );
    assert_eq!(
        kernel.session(a.id()).unwrap().unwrap().state(),
        WorkSessionState::Idle
    );
    let mut events = kernel.subscribe();
    let turn = workers
        .prompt(a.id(), "after-restart".into())
        .await
        .unwrap();
    wait(
        &mut events,
        |e| matches!(&e.kind,EventKind::TurnCompleted{turn:t} if t.id==turn.id),
    )
    .await;
    workers.shutdown().await.unwrap();
}

#[tokio::test]
async fn cancel_race_invalid_frames_and_secret_output() {
    let (dir, kernel, workers) = setup().await;
    let session = workers
        .create(dir.path().to_owned(), SessionOptions::default())
        .await
        .unwrap();
    let mut events = kernel.subscribe();
    assert!(workers.prompt(session.id(), TOKEN.into()).await.is_err());
    let rejected = workers
        .prompt(session.id(), "reject-cancel".into())
        .await
        .unwrap();
    assert!(workers.cancel(session.id()).await.is_err());
    assert_eq!(
        kernel.turns(session.id()).unwrap()[0].cancellation,
        CancellationState::Rejected
    );
    workers.cancel(session.id()).await.unwrap();
    wait(
        &mut events,
        |e| matches!(&e.kind, EventKind::TurnCompleted {turn} if turn.id == rejected.id),
    )
    .await;
    let turn = workers
        .prompt(session.id(), "cancel-race".into())
        .await
        .unwrap();
    workers.cancel(session.id()).await.unwrap();
    wait(
        &mut events,
        |e| matches!(&e.kind,EventKind::TurnCompleted{turn:t} if t.id==turn.id),
    )
    .await;
    assert_eq!(
        kernel.turns(session.id()).unwrap()[1].state,
        TurnState::Completed
    );
    assert_eq!(
        kernel.turns(session.id()).unwrap()[1].cancellation,
        CancellationState::Acknowledged
    );
    let turn = workers
        .prompt(session.id(), "safe-output".into())
        .await
        .unwrap();
    loop {
        let event = tokio::time::timeout(Duration::from_secs(10), events.recv())
            .await
            .unwrap()
            .unwrap();
        let encoded = serde_json::to_string(&event).unwrap();
        assert!(!encoded.contains("sk-secret-") && !encoded.contains("WRONG SESSION"));
        if matches!(&event.kind,EventKind::TurnCompleted{turn:t} if t.id==turn.id) {
            break;
        }
    }
    let unsupported = workers
        .prompt(session.id(), "unsupported".into())
        .await
        .unwrap();
    wait(&mut events, |e| {
        matches!(e.kind, EventKind::ErrorObserved { .. })
    })
    .await;
    wait(
        &mut events,
        |e| matches!(&e.kind, EventKind::TurnCompleted {turn} if turn.id == unsupported.id),
    )
    .await;
    workers
        .prompt(session.id(), "bad-frame".into())
        .await
        .unwrap();
    assert!(workers.cancel(session.id()).await.is_err());
    wait(&mut events, |e| {
        matches!(e.kind, EventKind::BackendDisconnected { .. })
    })
    .await;
    assert_eq!(
        kernel.turns(session.id()).unwrap().last().unwrap().state,
        TurnState::Unknown
    );
    workers.resume(session.id()).await.unwrap();
    workers
        .prompt(session.id(), "overflow".into())
        .await
        .unwrap();
    assert!(workers.cancel(session.id()).await.is_err());
    wait(&mut events, |e| {
        matches!(e.kind, EventKind::BackendDisconnected { .. })
    })
    .await;
    assert_eq!(
        kernel.turns(session.id()).unwrap().last().unwrap().state,
        TurnState::Unknown
    );
    workers.shutdown().await.unwrap();
}

#[tokio::test]
async fn authenticated_control_and_sse_snapshot_lag_and_detach() {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use serde_json::{Value, json};
    use tokio_stream::StreamExt;
    use tower::ServiceExt;
    let (dir, kernel, workers) = setup().await;
    let app = lain_kernel::api::controlled_router(
        kernel.clone(),
        TOKEN.parse().unwrap(),
        Some(workers.clone()),
    );
    async fn call(app: axum::Router, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
        let response = app
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header("authorization", format!("Bearer {TOKEN}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        (
            status,
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned())),
        )
    }
    for path in ["/v1/events", "/v1/backends", "/v1/work-sessions"] {
        assert_eq!(
            app.clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    let (status, session) = call(
        app.clone(),
        "POST",
        "/v1/work-sessions",
        json!({"workspace":dir.path()}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let id: WorkSessionId = serde_json::from_value(session["id"].clone()).unwrap();
    let base = format!("/v1/work-sessions/{id}");
    let (_, history) = call(
        app.clone(),
        "GET",
        &format!("{base}/history?limit=2"),
        Value::Null,
    )
    .await;
    assert_eq!(history["events"].as_array().unwrap().len(), 2);
    assert_eq!(history["has_more"], true);
    let cursor = history["next_cursor"].as_u64().unwrap();
    let (_, next) = call(
        app.clone(),
        "GET",
        &format!("{base}/history?after={cursor}"),
        Value::Null,
    )
    .await;
    assert!(
        next["events"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["history_order"].as_u64().unwrap() > cursor)
    );
    for query in [
        "limit=0",
        "limit=201",
        "after=1&before=2",
        "after=18446744073709551615",
    ] {
        assert_eq!(
            call(
                app.clone(),
                "GET",
                &format!("{base}/history?{query}"),
                Value::Null
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        call(
            app.clone(),
            "GET",
            &format!("/v1/work-sessions/{}/history", WorkSessionId::new()),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.clone()
            .oneshot(
                Request::builder()
                    .uri(format!("{base}/history"))
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            app.clone(),
            "POST",
            &format!("{base}/turns"),
            json!({"text":"", "unknown":true})
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/events")
                .header("authorization", format!("Bearer {TOKEN}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.headers()["content-type"], "text/event-stream");
    let mut stream = response.into_body().into_data_stream();
    let chunk = stream.next().await.unwrap().unwrap();
    assert!(String::from_utf8_lossy(&chunk).contains("event: snapshot"));
    drop(stream);
    let mut events = kernel.subscribe();
    let (status, turn) = call(
        app.clone(),
        "POST",
        &format!("{base}/turns"),
        json!({"text":"approve"}),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert!(turn.get("provider_turn_id").is_none());
    wait(&mut events, |e| {
        matches!(e.kind, EventKind::ApprovalRequested { .. })
    })
    .await;
    let (_, approvals) = call(
        app.clone(),
        "GET",
        &format!("{base}/approvals"),
        Value::Null,
    )
    .await;
    let approval = approvals[0]["id"].as_str().unwrap();
    assert_eq!(
        call(
            app.clone(),
            "POST",
            &format!("{base}/approvals/{approval}/decision"),
            json!({"decision":"deny"})
        )
        .await
        .0,
        StatusCode::ACCEPTED
    );
    wait(&mut events, |e| {
        matches!(e.kind, EventKind::TurnCompleted { .. })
    })
    .await;
    assert_eq!(
        call(app.clone(), "GET", &format!("{base}/turns"), Value::Null)
            .await
            .1[0]["state"],
        "completed"
    );
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/events")
                .header("authorization", format!("Bearer {TOKEN}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let mut stream = response.into_body().into_data_stream();
    let initial = stream.next().await.unwrap().unwrap();
    assert!(String::from_utf8_lossy(&initial).contains("completed"));
    // Deliberately overrun both bounded channels without consuming SSE frames.
    for _ in 0..180 {
        kernel
            .transition_session(id, WorkSessionState::Running)
            .unwrap();
        kernel
            .transition_session(id, WorkSessionState::Idle)
            .unwrap();
    }
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let chunk = stream.next().await.unwrap().unwrap();
            if String::from_utf8_lossy(&chunk).contains("resync_required") {
                break;
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(
        kernel.session(id).unwrap().unwrap().state(),
        WorkSessionState::Idle
    );
    workers.shutdown().await.unwrap();
}
