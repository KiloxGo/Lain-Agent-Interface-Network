use lain_kernel::{
    backend::Supervisor,
    domain::*,
    events::{Event, EventKind},
    kernel::Kernel,
};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::broadcast;

const TOKEN: &str = "fixture-owner-token-0123456789abcdef";

async fn setup() -> (tempfile::TempDir, Arc<Kernel>, Arc<Supervisor>) {
    let dir = tempfile::tempdir().unwrap();
    let kernel = Arc::new(Kernel::open(&dir.path().join("state.sqlite3")).unwrap());
    kernel.mark_ready().unwrap();
    let workers = Supervisor::new(
        kernel.clone(),
        Some(PathBuf::from(env!("CARGO_BIN_EXE_lain-test-app-server"))),
        TOKEN.parse().unwrap(),
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
        WorkSessionState::Unknown
    );
    let workers = Supervisor::new(
        kernel.clone(),
        Some(PathBuf::from(env!("CARGO_BIN_EXE_lain-test-app-server"))),
        TOKEN.parse().unwrap(),
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
