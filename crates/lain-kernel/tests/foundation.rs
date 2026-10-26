use std::{ffi::OsString, sync::Arc, time::Duration};

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use lain_kernel::{
    api,
    config::{ApiToken, Config},
    domain::{
        ApprovalId, EventId, RuntimeProfileId, TurnId, WorkSession, WorkSessionId,
        WorkSessionState::*,
    },
    events::EventKind,
    kernel::{Kernel, KernelState},
};
use tempfile::TempDir;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::broadcast::error::TryRecvError,
};
use tower::ServiceExt;

const TOKEN: &str = "test-token-with-at-least-32-characters";

fn kernel(dir: &TempDir) -> Kernel {
    let kernel = Kernel::open(&dir.path().join("state.sqlite3")).unwrap();
    kernel.mark_ready().unwrap();
    kernel
}

fn create(kernel: &Kernel, dir: &TempDir) -> WorkSession {
    kernel
        .create_session(
            "test-worker".parse().unwrap(),
            "test/default".parse().unwrap(),
            dir.path().to_owned(),
        )
        .unwrap()
}

#[test]
fn ids_and_complete_transition_matrix() {
    let id = WorkSessionId::new();
    assert_eq!(id.to_string().parse::<WorkSessionId>().unwrap(), id);
    assert_eq!(
        serde_json::from_str::<WorkSessionId>(&serde_json::to_string(&id).unwrap()).unwrap(),
        id
    );
    assert!("invalid".parse::<WorkSessionId>().is_err());
    assert!(serde_json::from_str::<WorkSessionId>("\"bad\"").is_err());
    for id in [
        TurnId::new().to_string(),
        EventId::new().to_string(),
        ApprovalId::new().to_string(),
    ] {
        assert_eq!(id.len(), 36);
    }
    assert!("worker/official".parse::<RuntimeProfileId>().is_ok());
    for invalid in ["", " / ", "a//b", "https://secret", "a\nb"] {
        assert!(invalid.parse::<RuntimeProfileId>().is_err());
        assert!(serde_json::from_value::<RuntimeProfileId>(serde_json::json!(invalid)).is_err());
    }
    let cases: &[_] = &[
        (Created, vec![Starting, Cancelled, Failed]),
        (
            Starting,
            vec![Ready, Cancelled, Failed, Disconnected, Unknown],
        ),
        (
            Ready,
            vec![Idle, Running, Cancelled, Failed, Disconnected, Unknown],
        ),
        (
            Running,
            vec![
                WaitingForApproval,
                Idle,
                Completed,
                Cancelled,
                Failed,
                Disconnected,
                Unknown,
            ],
        ),
        (
            WaitingForApproval,
            vec![Running, Idle, Cancelled, Failed, Disconnected, Unknown],
        ),
        (
            Idle,
            vec![Running, Completed, Cancelled, Failed, Disconnected, Unknown],
        ),
        (Completed, vec![Archived]),
        (Cancelled, vec![Archived]),
        (Failed, vec![Archived]),
        (
            Disconnected,
            vec![
                Unknown,
                Ready,
                Running,
                WaitingForApproval,
                Idle,
                Completed,
                Cancelled,
                Failed,
            ],
        ),
        (
            Unknown,
            vec![
                Disconnected,
                Ready,
                Running,
                WaitingForApproval,
                Idle,
                Completed,
                Cancelled,
                Failed,
            ],
        ),
        (Archived, vec![]),
    ];
    for (from, allowed) in cases {
        for (to, _) in cases {
            assert_eq!(
                from.can_transition_to(*to),
                allowed.contains(to),
                "{from:?} -> {to:?}"
            );
        }
    }
}

#[test]
fn config_defaults_overrides_validation_and_redaction() {
    let lookup = |key: &str| (key == "LAIN_API_TOKEN").then(|| OsString::from(TOKEN));
    let config = Config::from_lookup(lookup).unwrap();
    assert_eq!(config.listen.to_string(), "127.0.0.1:8787");
    assert_eq!(
        config.worker_idle_timeout,
        std::time::Duration::from_secs(300)
    );
    assert_eq!(config.database_path.to_str().unwrap(), "data/lain.sqlite3");
    assert!(!format!("{config:?}").contains(TOKEN));
    assert!(Config::from_lookup(|_| None).is_err());
    for (key, value) in [
        ("LAIN_LISTEN", "0.0.0.0:8787"),
        ("LAIN_LISTEN", "bad"),
        ("LAIN_DATABASE_PATH", ""),
        ("LAIN_DATABASE_PATH", ":memory:"),
        ("LAIN_LOG", "bad"),
        ("LAIN_API_TOKEN", "bad-secret"),
        ("LAIN_WORKER_IDLE_TIMEOUT", "0"),
        ("LAIN_WORKER_IDLE_TIMEOUT", "86401"),
        ("LAIN_WORKER_IDLE_TIMEOUT", "invalid"),
    ] {
        let error = Config::from_lookup(|k| {
            if k == key {
                Some(value.into())
            } else {
                lookup(k)
            }
        })
        .unwrap_err();
        assert!(!error.to_string().contains("bad-secret"));
        assert!(!format!("{error:?}").contains("bad-secret"));
    }
    let config = Config::from_lookup(|key| match key {
        "LAIN_LISTEN" => Some("[::1]:0".into()),
        "LAIN_DATABASE_PATH" => Some("custom/state.db".into()),
        "LAIN_LOG" => Some("debug".into()),
        "LAIN_WORKER_IDLE_TIMEOUT" => Some("5".into()),
        _ => lookup(key),
    })
    .unwrap();
    assert_eq!(config.listen.to_string(), "[::1]:0");
    assert_eq!(
        config.worker_idle_timeout,
        std::time::Duration::from_secs(5)
    );
    assert_eq!(config.database_path.to_str().unwrap(), "custom/state.db");
    assert!("x".repeat(257).parse::<ApiToken>().is_err());
}

#[test]
fn persistence_restart_reconciles_every_state_and_preserves_identity() {
    let dir = TempDir::new().unwrap();
    let kernel = kernel(&dir);
    let paths = [
        (vec![], Created),
        (vec![Starting], Unknown),
        (vec![Starting, Ready], Unknown),
        (vec![Starting, Ready, Running], Unknown),
        (vec![Starting, Ready, Running, WaitingForApproval], Unknown),
        (vec![Starting, Ready, Running, Idle], Unknown),
        (vec![Starting, Ready, Running, Completed], Completed),
        (vec![Cancelled], Cancelled),
        (vec![Failed], Failed),
        (vec![Starting, Disconnected], Unknown),
        (vec![Starting, Unknown], Unknown),
        (vec![Cancelled, Archived], Archived),
    ];
    let mut snapshots = Vec::new();
    for (path, expected) in paths {
        let mut session = create(&kernel, &dir);
        for state in path {
            session = kernel.transition_session(session.id(), state).unwrap();
        }
        snapshots.push((session, expected));
    }
    assert!(Kernel::open(&dir.path().join("state.sqlite3")).is_err());
    drop(kernel); // Simulate a restart without a clean lifecycle transition.
    let reopened = Kernel::open(&dir.path().join("state.sqlite3")).unwrap();
    assert_eq!(reopened.state().unwrap(), KernelState::Recovering);
    for (before, expected) in snapshots {
        let after = reopened.session(before.id()).unwrap().unwrap();
        assert_eq!(after.state(), expected);
        let mut before_json = serde_json::to_value(&before).unwrap();
        let after_json = serde_json::to_value(&after).unwrap();
        assert!(after_json["updated_at_ms"].as_u64() >= before_json["updated_at_ms"].as_u64());
        before_json["state"] = serde_json::json!(expected);
        before_json["updated_at_ms"] = after_json["updated_at_ms"].clone();
        assert_eq!(before_json, after_json);
    }
    let recovered = reopened.list_sessions().unwrap();
    drop(reopened);
    let reopened = Kernel::open(&dir.path().join("state.sqlite3")).unwrap();
    assert_eq!(recovered, reopened.list_sessions().unwrap());
}

#[test]
fn workspace_validation_and_persisted_record_invariants() {
    let dir = TempDir::new().unwrap();
    let kernel = kernel(&dir);
    let workspace = dir.path().join("workspace");
    assert!(
        kernel
            .create_session(
                "test".parse().unwrap(),
                "test".parse().unwrap(),
                workspace.clone()
            )
            .is_err()
    );
    std::fs::write(&workspace, "file, not directory").unwrap();
    assert!(
        kernel
            .create_session(
                "test".parse().unwrap(),
                "test".parse().unwrap(),
                workspace.clone()
            )
            .is_err()
    );
    std::fs::remove_file(&workspace).unwrap();
    std::fs::create_dir(&workspace).unwrap();
    let session = kernel
        .create_session(
            "test".parse().unwrap(),
            "test".parse().unwrap(),
            workspace.join("."),
        )
        .unwrap();
    let snapshot = serde_json::to_value(&session).unwrap();
    assert_eq!(
        snapshot["workspace"],
        serde_json::json!(workspace.canonicalize().unwrap())
    );
    drop(kernel);
    std::fs::remove_dir(&workspace).unwrap();
    let path = dir.path().join("state.sqlite3");
    let reopened = Kernel::open(&path).unwrap();
    assert_eq!(
        reopened.session(session.id()).unwrap(),
        Some(session.clone())
    );
    drop(reopened);
    let db = rusqlite::Connection::open(&path).unwrap();
    for (field, invalid) in [
        ("workspace", serde_json::json!("relative/path")),
        ("updated_at_ms", serde_json::json!(0)),
        ("id", serde_json::json!(WorkSessionId::new())),
        ("backend", serde_json::json!("bad\nname")),
    ] {
        let mut corrupt = snapshot.clone();
        corrupt[field] = invalid;
        db.execute(
            "UPDATE work_sessions SET snapshot = ?1",
            [corrupt.to_string()],
        )
        .unwrap();
        assert!(Kernel::open(&path).is_err(), "accepted invalid {field}");
    }
}

#[test]
fn mutations_commit_before_events_and_fail_without_publishing() {
    let dir = TempDir::new().unwrap();
    let kernel = kernel(&dir);
    let mut first = kernel.subscribe();
    let mut second = kernel.subscribe();
    let session = create(&kernel, &dir);
    let event = first.try_recv().unwrap();
    assert_eq!(event, second.try_recv().unwrap());
    assert_eq!(
        event.kind,
        EventKind::SessionCreated {
            session: session.clone()
        }
    );
    assert_eq!(
        kernel.session(event.work_session_id).unwrap(),
        Some(session.clone())
    );
    assert!(kernel.transition_session(session.id(), Completed).is_err());
    assert_eq!(first.try_recv(), Err(TryRecvError::Empty));
    let db = rusqlite::Connection::open(dir.path().join("state.sqlite3")).unwrap();
    db.execute_batch("CREATE TRIGGER reject_update BEFORE UPDATE ON work_sessions BEGIN SELECT RAISE(ABORT, 'test failure'); END;").unwrap();
    assert!(kernel.transition_session(session.id(), Starting).is_err());
    assert_eq!(kernel.session(session.id()).unwrap(), Some(session.clone()));
    assert_eq!(first.try_recv(), Err(TryRecvError::Empty));
    db.execute_batch("DROP TRIGGER reject_update;").unwrap();
    for state in [
        Starting,
        Ready,
        Running,
        WaitingForApproval,
        Running,
        Idle,
        Running,
        Cancelled,
        Archived,
    ] {
        let updated = kernel.transition_session(session.id(), state).unwrap();
        let event = first.try_recv().unwrap();
        assert_eq!(event, second.try_recv().unwrap());
        assert_eq!(kernel.session(session.id()).unwrap(), Some(updated));
        let persisted: WorkSession = serde_json::from_str(
            &db.query_row(
                "SELECT snapshot FROM work_sessions WHERE id = ?1",
                [session.id().to_string()],
                |r| r.get::<_, String>(0),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(persisted.state(), state);
    }
    assert!(kernel.transition_session(session.id(), Running).is_err());
    assert!(
        kernel
            .transition_session(WorkSessionId::new(), Starting)
            .is_err()
    );
    kernel.shutdown().unwrap();
    assert!(kernel.mark_ready().is_err());
    assert!(kernel.transition_session(session.id(), Running).is_err());
    assert!(
        kernel
            .create_session(
                "test".parse().unwrap(),
                "test".parse().unwrap(),
                dir.path().to_owned()
            )
            .is_err()
    );
}

#[test]
fn migration_and_corrupt_recovery_fail_closed() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("state.sqlite3");
    let kernel = kernel(&dir);
    let session = create(&kernel, &dir);
    kernel.transition_session(session.id(), Starting).unwrap();
    drop(kernel);
    let db = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        15
    );
    db.execute(
        "INSERT INTO work_sessions VALUES (?1, ?2)",
        (WorkSessionId::new().to_string(), "{}"),
    )
    .unwrap();
    assert!(Kernel::open(&path).is_err());
    let snapshot: String = db
        .query_row(
            "SELECT snapshot FROM work_sessions WHERE id = ?1",
            [session.id().to_string()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<WorkSession>(&snapshot)
            .unwrap()
            .state(),
        Starting
    );
    db.execute_batch("PRAGMA user_version = 99;").unwrap();
    assert!(Kernel::open(&path).is_err());
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        99
    );
}

#[test]
fn event_lag_is_explicit_and_no_subscribers_is_normal() {
    let dir = TempDir::new().unwrap();
    let kernel = kernel(&dir);
    let session = create(&kernel, &dir);
    let mut events = kernel.subscribe();
    kernel.transition_session(session.id(), Starting).unwrap();
    kernel.transition_session(session.id(), Ready).unwrap();
    for _ in 0..130 {
        kernel.transition_session(session.id(), Running).unwrap();
        kernel.transition_session(session.id(), Idle).unwrap();
    }
    assert!(matches!(events.try_recv(), Err(TryRecvError::Lagged(_))));
    assert_eq!(kernel.session(session.id()).unwrap().unwrap().state(), Idle);
}

#[test]
fn failed_history_commit_rolls_back_state_and_does_not_publish() {
    let dir = TempDir::new().unwrap();
    let kernel = kernel(&dir);
    let session = create(&kernel, &dir);
    let before = kernel.history(session.id(), 0, None, 100).unwrap();
    let mut events = kernel.subscribe();
    let db = rusqlite::Connection::open(dir.path().join("state.sqlite3")).unwrap();
    db.execute_batch("CREATE TRIGGER reject_history BEFORE INSERT ON operational_history BEGIN SELECT RAISE(ABORT, 'fixture failure'); END;").unwrap();
    assert!(kernel.transition_session(session.id(), Starting).is_err());
    assert_eq!(
        kernel.session(session.id()).unwrap().unwrap().state(),
        Created
    );
    assert!(events.try_recv().is_err());
    assert_eq!(
        kernel.history(session.id(), 0, None, 100).unwrap().events,
        before.events
    );
    let snapshot: String = db
        .query_row(
            "SELECT snapshot FROM work_sessions WHERE id = ?1",
            [session.id().to_string()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<WorkSession>(&snapshot)
            .unwrap()
            .state(),
        Created
    );
}

#[test]
fn phase_one_migration_and_worker_recovery_are_atomic() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("state.sqlite3");
    let original = kernel(&dir);
    let session = create(&original, &dir);
    original.transition_session(session.id(), Starting).unwrap();
    drop(original);
    let db = rusqlite::Connection::open(&path).unwrap();
    // Reconstruct the exact schema of an existing Phase 1 database.
    db.execute_batch(
        "DROP TABLE share_reads; DROP TABLE share_model_requests; DROP TABLE share_jobs; DROP TABLE share_deliveries; DROP TABLE share_candidates; DROP TABLE channel_attention; DROP TABLE share_grants; DROP TABLE sharing_runtime; DROP TABLE reading_source_decisions; DROP TABLE experience_edits; DROP TABLE reading_source_suppressions; DROP TABLE experience_reads; DROP TABLE reading_model_requests; DROP TABLE reading_jobs; DROP TABLE reading_experiences; DROP TABLE reading_withdrawals; DROP TABLE reading_results; DROP TABLE reading_activities; DROP TABLE reading_grants; DROP TABLE channel_source_reads; DROP TABLE channel_source_grants; DROP TABLE channel_memory_inspections; DROP TABLE channel_memory_suppressions; DROP TABLE channel_memory_effects; DROP TABLE channel_memory_grants; DROP TABLE channel_deliveries; DROP TABLE channel_batches; DROP TABLE channel_incoming; DROP TABLE channel_bindings; DROP TABLE lain_memory_fts; DROP TABLE lain_memories; DROP TABLE lain_memory_requests; DROP TABLE lain_memory_candidates; DROP TABLE lain_session_summaries; DROP TABLE lain_conversation_notices; DROP TABLE lain_delegations; DROP TABLE lain_actor_turns; DROP TABLE lain_conversations; DROP TABLE operational_history; DROP TABLE worker_state; DROP TABLE runtime_profiles; DROP TABLE workspace_profile_policies; PRAGMA user_version = 1;",
    )
    .unwrap();
    let migrated = Kernel::open(&path).unwrap();
    assert_eq!(
        migrated.session(session.id()).unwrap().unwrap().state(),
        Unknown
    );
    assert!(migrated.turns(session.id()).unwrap().is_empty());
    migrated.mark_ready().unwrap();
    migrated.transition_session(session.id(), Running).unwrap();
    drop(migrated);
    db.execute(
        "INSERT INTO worker_state VALUES (?1, '{}')",
        [session.id().to_string()],
    )
    .unwrap();
    assert!(Kernel::open(&path).is_err());
    let snapshot: String = db
        .query_row("SELECT snapshot FROM work_sessions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        serde_json::from_str::<WorkSession>(&snapshot)
            .unwrap()
            .state(),
        Running
    );
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        15
    );
}

async fn request(
    app: axum::Router,
    path: &str,
    token: Option<&str>,
) -> (StatusCode, serde_json::Value) {
    let mut request = Request::builder().uri(path);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = app
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains(TOKEN));
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn api_auth_readiness_listing_inspection_and_errors() {
    let dir = TempDir::new().unwrap();
    let kernel = Arc::new(Kernel::open(&dir.path().join("state.sqlite3")).unwrap());
    let app = api::router(kernel.clone(), TOKEN.parse().unwrap());
    for path in ["/health", "/ready", "/v1/work-sessions", "/missing"] {
        assert_eq!(
            request(app.clone(), path, None).await.0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            request(app.clone(), path, Some("wrong-token")).await.0,
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        request(app.clone(), "/health", Some(TOKEN)).await.0,
        StatusCode::OK
    );
    assert_eq!(
        request(app.clone(), "/ready", Some(TOKEN)).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        request(app.clone(), "/v1/work-sessions", Some(TOKEN))
            .await
            .1,
        serde_json::json!([])
    );
    kernel.mark_ready().unwrap();
    let session = create(&kernel, &dir);
    assert_eq!(
        request(app.clone(), "/ready", Some(TOKEN)).await.1,
        serde_json::json!({"state": "ready"})
    );
    assert_eq!(
        request(app.clone(), "/v1/work-sessions", Some(TOKEN))
            .await
            .1,
        serde_json::json!([session])
    );
    assert_eq!(
        request(
            app.clone(),
            &format!("/v1/work-sessions/{}", session.id()),
            Some(TOKEN)
        )
        .await
        .1,
        serde_json::json!(session)
    );
    assert_eq!(
        request(app.clone(), "/v1/work-sessions/bad", Some(TOKEN))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        request(
            app.clone(),
            &format!("/v1/work-sessions/{}", WorkSessionId::new()),
            Some(TOKEN)
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(app.clone(), "/missing", Some(TOKEN)).await.0,
        StatusCode::NOT_FOUND
    );
    let duplicate = Request::builder()
        .uri("/health")
        .header("authorization", format!("Bearer {TOKEN}"))
        .header("authorization", format!("Bearer {TOKEN}"))
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(duplicate).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    kernel.shutdown().unwrap();
    assert_eq!(
        request(app, "/ready", Some(TOKEN)).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn real_http_shutdown_and_reopen() {
    let dir = TempDir::new().unwrap();
    let kernel = Arc::new(Kernel::open(&dir.path().join("state.sqlite3")).unwrap());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(api::serve(
        listener,
        kernel.clone(),
        TOKEN.parse().unwrap(),
        async {
            stopped.await.unwrap();
        },
    ));
    let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
    stream.write_all(format!("GET /ready HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {TOKEN}\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
    let mut response = String::new();
    tokio::time::timeout(Duration::from_secs(5), stream.read_to_string(&mut response))
        .await
        .unwrap()
        .unwrap();
    assert!(response.starts_with("HTTP/1.1 200"));
    let session = create(&kernel, &dir);
    stop.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(kernel.state().unwrap(), KernelState::ShuttingDown);
    assert!(tokio::net::TcpStream::connect(address).await.is_err());
    drop(kernel);
    let reopened = Kernel::open(&dir.path().join("state.sqlite3")).unwrap();
    assert_eq!(reopened.session(session.id()).unwrap(), Some(session));
}
