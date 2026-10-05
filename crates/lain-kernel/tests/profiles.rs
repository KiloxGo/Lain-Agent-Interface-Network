use lain_kernel::{
    backend::Supervisor,
    domain::*,
    events::{Event, EventKind},
    kernel::Kernel,
    profiles::*,
};
use serde_json::{Value, json};
use std::{path::Path, sync::Arc, time::Duration};
const TOKEN: &str = "fixture-owner-token-0123456789abcdef";

fn definition(id: &str, model: &str) -> ProfileDefinition {
    ProfileDefinition {
        id: id.parse().unwrap(),
        model: Some(model.into()),
        ..ProfileDefinition::local()
    }
}
fn gateway(id: &str, model: &str, key: &str, endpoint: &str) -> ProfileDefinition {
    ProfileDefinition {
        provider: Some("fixture-gateway".into()),
        base_url: Some(endpoint.into()),
        secret_ref: Some(key.parse().unwrap()),
        ..definition(id, model)
    }
}
async fn supervisor(kernel: Arc<Kernel>) -> Arc<Supervisor> {
    Supervisor::new(
        kernel,
        Some(Path::new(env!("CARGO_BIN_EXE_lain-test-app-server")).to_owned()),
        TOKEN.parse().unwrap(),
        Duration::from_secs(300),
    )
    .await
}
async fn completed(events: &mut tokio::sync::broadcast::Receiver<Event>, turn: TurnId) -> Turn {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let EventKind::TurnCompleted { turn: t } = events.recv().await.unwrap().kind
                && t.id == turn
            {
                return t;
            }
        }
    })
    .await
    .unwrap()
}
#[test]
fn registry_revisions_validation_and_namespaced_secrets() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("registry.sqlite3");
    let kernel = Kernel::open(&path).unwrap();
    kernel.mark_ready().unwrap();
    let first = kernel
        .save_profile(definition("codex/a", "model-a"), None)
        .unwrap();
    assert_eq!(first.revision, 1);
    assert!(kernel.save_profile(first.definition.clone(), None).is_err());
    let mut edited = first.definition;
    edited.model = Some("model-b".into());
    assert!(kernel.save_profile(edited.clone(), Some(9)).is_err());
    assert_eq!(kernel.save_profile(edited, Some(1)).unwrap().revision, 2);
    for endpoint in [
        "ftp://example.com",
        "http://example.com",
        "https://user:password@example.com",
        "https://example.com?q=secret",
        "https://example.com/#secret",
        "https://",
        "https://example.com\\evil",
        "https://example.com\n",
    ] {
        assert!(validate_endpoint(endpoint).is_err());
    }
    for endpoint in [
        "https://api.example.com/v1",
        "http://127.0.0.1:3456/v1",
        "http://[::1]:3456/v1",
        "http://localhost:3456",
    ] {
        validate_endpoint(endpoint).unwrap();
    }
    for reference in [
        "secret://env/PATH",
        "secret://env/../../home",
        "secret://env/a_b",
        "secret://env/-x",
        "secret://PATH",
        "LAIN_API_TOKEN",
    ] {
        assert!(reference.parse::<SecretRef>().is_err());
    }
    let reference: SecretRef = "secret://env/phase5-missing-credential".parse().unwrap();
    assert_eq!(
        reference.env_name(),
        "LAIN_SECRET_PHASE5_MISSING_CREDENTIAL"
    );
    assert_eq!(secret_status(Some(&reference)), "missing");
    let profile = gateway(
        "codex/missing",
        "fixture",
        "secret://env/phase5-missing-credential",
        "https://example.com/v1",
    );
    let mut official = profile.clone();
    official.base_url = None;
    let resolved = RuntimeProfile {
        definition: official,
        revision: 1,
    }
    .resolve(&SessionOptions::default())
    .unwrap();
    assert_eq!(
        resolved.base_url.as_deref(),
        Some("https://api.openai.com/v1")
    );
    kernel.save_profile(profile, None).unwrap();
    let mut bad = definition("codex/invalid", "safe");
    bad.model = Some("sk-forbidden-value".into());
    assert!(kernel.save_profile(bad, None).is_err());
    let mut bad = definition("codex/invalid", "safe");
    bad.driver = "arbitrary-argv".into();
    assert!(bad.validate().is_err());
    drop(kernel);
    let kernel = Kernel::open(&path).unwrap();
    assert_eq!(
        kernel
            .profiles()
            .unwrap()
            .iter()
            .find(|p| p.definition.id.to_string() == "codex/a")
            .unwrap()
            .revision,
        2
    );
}

#[tokio::test]
async fn frozen_profiles_policy_retry_terminal_failure_and_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("state.sqlite3");
    let workspace = dir.path().join("work");
    std::fs::create_dir(&workspace).unwrap();
    let kernel = Arc::new(Kernel::open(&database).unwrap());
    kernel.mark_ready().unwrap();
    kernel
        .save_profile(definition("codex/a", "model-a"), None)
        .unwrap();
    kernel
        .save_profile(definition("codex/b", "model-b"), None)
        .unwrap();
    kernel
        .save_workspace_policy(WorkspacePolicy {
            workspace: workspace.clone(),
            default_profile: "codex/a".parse().unwrap(),
            allowed_profiles: vec!["codex/a".parse().unwrap(), "codex/local".parse().unwrap()],
        })
        .unwrap();
    let workers = supervisor(kernel.clone()).await;
    let mut events = kernel.subscribe();
    assert!(
        workers
            .create_with_profile(
                workspace.clone(),
                Some("codex/b".parse().unwrap()),
                SessionOptions::default()
            )
            .await
            .is_err()
    );
    assert!(
        workers
            .create_with_profile(
                workspace.clone(),
                Some("codex/absent".parse().unwrap()),
                SessionOptions::default()
            )
            .await
            .is_err()
    );
    assert!(kernel.list_sessions().unwrap().is_empty());
    let a = workers
        .create(workspace.clone(), SessionOptions::default())
        .await
        .unwrap();
    let original = kernel.resolved_profile(a.id()).unwrap();
    assert_eq!(original.profile_id.to_string(), "codex/a");
    assert_eq!(original.model.as_deref(), Some("model-a"));
    let identity = std::fs::read(workspace.join("fixture-history.json")).unwrap();
    kernel
        .save_profile(definition("codex/a", "new-model"), Some(1))
        .unwrap();
    assert_eq!(kernel.resolved_profile(a.id()).unwrap(), original);
    let new_workspace = dir.path().join("new-work");
    std::fs::create_dir(&new_workspace).unwrap();
    let newer = workers
        .create_with_profile(
            new_workspace,
            Some("codex/a".parse().unwrap()),
            SessionOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(kernel.resolved_profile(newer.id()).unwrap().revision, 2);
    assert_eq!(
        kernel
            .resolved_profile(newer.id())
            .unwrap()
            .model
            .as_deref(),
        Some("new-model")
    );
    workers.detach(newer.id()).await.unwrap();
    let mut disabled = definition("codex/a", "new-model");
    disabled.enabled = false;
    kernel.save_profile(disabled, Some(2)).unwrap();
    assert!(
        workers
            .create(workspace.clone(), SessionOptions::default())
            .await
            .is_err()
    );
    let retry = workers
        .prompt(a.id(), "retry-success".into())
        .await
        .unwrap();
    assert_eq!(
        completed(&mut events, retry.id).await.state,
        TurnState::Completed
    );
    let history = kernel.history(a.id(), 0, None, 200).unwrap();
    assert!(history.events.iter().any(|e| matches!(&e.kind, EventKind::ErrorObserved { failure } if failure.kind == FailureKind::ProviderUnavailable && failure.retryable == Some(true) && failure.message.contains("will retry"))));
    workers.detach(a.id()).await.unwrap();
    workers.resume(a.id()).await.unwrap();
    assert_eq!(kernel.resolved_profile(a.id()).unwrap(), original);
    let config: Value =
        serde_json::from_slice(&std::fs::read(workspace.join("fixture-config.json")).unwrap())
            .unwrap();
    assert_eq!(config["model"], "model-a");
    assert_eq!(
        serde_json::from_slice::<Value>(&identity).unwrap()["id"],
        serde_json::from_slice::<Value>(
            &std::fs::read(workspace.join("fixture-history.json")).unwrap()
        )
        .unwrap()["id"]
    );
    let failed = workers.prompt(a.id(), "auth-failure".into()).await.unwrap();
    let failed = completed(&mut events, failed.id).await;
    assert_eq!(failed.state, TurnState::Failed);
    assert_eq!(
        failed.failure.as_ref().unwrap().kind,
        FailureKind::AuthenticationFailed
    );
    assert_eq!(failed.failure.as_ref().unwrap().retryable, Some(false));
    kernel
        .save_workspace_policy(WorkspacePolicy {
            workspace: workspace.clone(),
            default_profile: "codex/local".parse().unwrap(),
            allowed_profiles: vec!["codex/local".parse().unwrap()],
        })
        .unwrap();
    assert!(workers.prompt(a.id(), "continue".into()).await.is_err()); // also attached runtimes
    workers.detach(a.id()).await.unwrap();
    assert!(workers.resume(a.id()).await.is_err());
    assert_eq!(kernel.resolved_profile(a.id()).unwrap(), original);
    workers.shutdown().await.unwrap();
    drop(workers);
    drop(kernel);
    let kernel = Kernel::open(&database).unwrap();
    assert_eq!(kernel.turns(a.id()).unwrap().last().unwrap(), &failed);
    assert!(
        serde_json::to_string(&kernel.snapshot_and_subscribe().unwrap().0)
            .unwrap()
            .contains("authentication_failed")
    );
    assert!(
        !serde_json::to_string(&kernel.history(a.id(), 0, None, 200).unwrap())
            .unwrap()
            .contains("secret=hidden")
    );
}

#[tokio::test]
async fn simultaneous_secret_profiles_are_isolated_and_redacted_before_commit() {
    // Give a private test subprocess its own environment; never mutate the test
    // runner's environment while other async tests are running.
    if std::env::var_os("LAIN_PHASE5_TEST_CHILD").is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "simultaneous_secret_profiles_are_isolated_and_redacted_before_commit",
                "--nocapture",
            ])
            .env("LAIN_PHASE5_TEST_CHILD", "1")
            .env("LAIN_SECRET_PHASE5_QUOTED", "fixture\"quoted-value")
            .env("LAIN_SECRET_PHASE5_A", "fixture-private-alpha-value")
            .env("LAIN_SECRET_PHASE5_B", "fixture-private-beta-value")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("state.sqlite3");
    let kernel = Arc::new(Kernel::open(&db).unwrap());
    kernel.mark_ready().unwrap();
    for (id, model, key, endpoint) in [
        (
            "codex/a",
            "model-a",
            "secret://env/phase5-a",
            "https://alpha.example.com/v1",
        ),
        (
            "codex/b",
            "model-b",
            "secret://env/phase5-b",
            "https://beta.example.com/v1",
        ),
    ] {
        let mut profile = gateway(id, model, key, endpoint);
        profile.reasoning_effort = Some(if model == "model-a" { "high" } else { "medium" }.into());
        if model == "model-b" {
            profile.approval_policy = ApprovalPolicy::Untrusted;
        }
        kernel.save_profile(profile, None).unwrap();
    }
    assert!(
        kernel
            .save_profile(
                definition("codex/leak", "fixture-private-alpha-value"),
                None
            )
            .is_err()
    );
    assert!(
        kernel
            .save_profile(
                gateway(
                    "codex/quoted-leak",
                    "fixture-model",
                    "secret://env/phase5-a",
                    "https://example.com/fixture\"quoted-value"
                ),
                None
            )
            .is_err()
    );
    let workers = supervisor(kernel.clone()).await;
    let paths: Vec<_> = ["a", "b"]
        .iter()
        .map(|s| {
            let p = dir.path().join(s);
            std::fs::create_dir(&p).unwrap();
            p
        })
        .collect();
    let (a, b) = tokio::join!(
        workers.create_with_profile(
            paths[0].clone(),
            Some("codex/a".parse().unwrap()),
            SessionOptions::default()
        ),
        workers.create_with_profile(
            paths[1].clone(),
            Some("codex/b".parse().unwrap()),
            SessionOptions::default()
        )
    );
    let sessions = [a.unwrap(), b.unwrap()];
    let mut events = kernel.subscribe();
    for (i, session) in sessions.iter().enumerate() {
        let value: Value =
            serde_json::from_slice(&std::fs::read(paths[i].join("fixture-config.json")).unwrap())
                .unwrap();
        assert_eq!(value["model"], ["model-a", "model-b"][i]);
        assert_eq!(
            value["config"]["model_reasoning_effort"],
            ["high", "medium"][i]
        );
        assert_eq!(value["approval_policy"], ["on-request", "untrusted"][i]);
        let argv = value["args"].to_string();
        assert!(argv.contains(["alpha.example.com", "beta.example.com"][i]));
        assert!(!argv.contains(["beta.example.com", "alpha.example.com"][i]));
        workers
            .prompt(session.id(), "profile-echo".into())
            .await
            .unwrap();
    }
    let mut observed = Vec::new();
    for _ in 0..2 {
        loop {
            let e = tokio::time::timeout(Duration::from_secs(10), events.recv())
                .await
                .unwrap()
                .unwrap();
            observed.push(e.clone());
            if matches!(e.kind, EventKind::ApprovalRequested { .. }) {
                break;
            }
        }
    }
    for session in &sessions {
        let approval = kernel.approvals(session.id()).unwrap().pop().unwrap();
        assert_eq!(approval.description, "[sensitive output redacted]");
        workers
            .decide(session.id(), approval.id, ApprovalDecision::Deny)
            .await
            .unwrap();
    }
    for session in &sessions {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                if kernel.session(session.id()).unwrap().unwrap().state() == WorkSessionState::Idle
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }
    let all = format!(
        "{}{}{}",
        serde_json::to_string(&observed).unwrap(),
        serde_json::to_string(&workers.profiles().await.unwrap()).unwrap(),
        serde_json::to_string(&kernel.snapshot_and_subscribe().unwrap().0).unwrap()
    );
    for key in [
        "fixture-private-alpha-value",
        "fixture-private-beta-value",
        TOKEN,
    ] {
        assert!(!all.contains(key));
    }
    for session in &sessions {
        let history =
            serde_json::to_string(&kernel.history(session.id(), 0, None, 200).unwrap()).unwrap();
        assert!(history.contains("sensitive output redacted"));
        for key in [
            "fixture-private-alpha-value",
            "fixture-private-beta-value",
            TOKEN,
        ] {
            assert!(!history.contains(key));
        }
    }
    workers.shutdown().await.unwrap();
    drop(workers);
    drop(kernel);
    let db = rusqlite::Connection::open(db).unwrap();
    for table in ["work_sessions", "worker_state", "runtime_profiles"] {
        let mut query = db
            .prepare(&format!("SELECT snapshot FROM {table}"))
            .unwrap();
        for value in query.query_map([], |r| r.get::<_, String>(0)).unwrap() {
            let value = value.unwrap();
            assert!(
                !value.contains("fixture-private-alpha-value")
                    && !value.contains("fixture-private-beta-value")
            );
        }
    }
}

#[tokio::test]
async fn phase4_database_migration_preserves_identity_history_and_local_resume() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.sqlite3");
    let workspace = dir.path().join("legacy");
    std::fs::create_dir(&workspace).unwrap();
    let id = WorkSessionId::new();
    let tid = TurnId::new();
    let event = uuid::Uuid::new_v4().to_string();
    let native = uuid::Uuid::new_v4().to_string();
    std::fs::write(
        workspace.join("fixture-history.json"),
        json!({"id":native,"turns":[]}).to_string(),
    )
    .unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch(concat!(
        include_str!("../migrations/001_sessions.sql"),
        include_str!("../migrations/002_worker_state.sql"),
        include_str!("../migrations/003_history.sql")
    ))
    .unwrap();
    let snapshot = json!({"id":id,"backend":"codex","runtime_profile_id":"codex/local","workspace":workspace.canonicalize().unwrap(),"state":"idle","runtime":"detached","created_at_ms":1,"updated_at_ms":2});
    let old_failure = json!({"kind":"execution","message":"safe old failure","code":null});
    let turn = json!({"id":tid,"work_session_id":id,"state":"failed","cancellation":"not_requested","created_at_ms":1,"updated_at_ms":2,"failure":old_failure});
    let aid = ApprovalId::new();
    let approval = json!({"id":aid,"work_session_id":id,"turn_id":tid,"kind":"command","description":"old safe operation","decisions":["deny"],"status":"resolved","decision":"deny","created_at_ms":1,"updated_at_ms":2});
    let worker = json!({"provider_session_id":native,"options":{"model":"legacy-model","sandbox":"read_only"},"turns":[{"turn":turn,"provider_turn_id":"old-native-turn"}],"approvals":[approval]});
    db.execute(
        "INSERT INTO work_sessions VALUES (?1, ?2)",
        (id.to_string(), snapshot.to_string()),
    )
    .unwrap();
    db.execute(
        "INSERT INTO worker_state VALUES (?1, ?2)",
        (id.to_string(), worker.to_string()),
    )
    .unwrap();
    let history = json!({"id":event,"history_order":null,"timestamp_ms":2,"work_session_id":id,"turn_id":tid,"kind":"assistant_message_completed","text":"historical-message"});
    db.execute("INSERT INTO operational_history (event_id,session_id,turn_id,timestamp_ms,kind,payload) VALUES (?1,?2,?3,2,'assistant_message_completed',?4)", (&event, id.to_string(), tid.to_string(), history.to_string())).unwrap();
    drop(db);
    let kernel = Arc::new(Kernel::open(&path).unwrap());
    kernel.mark_ready().unwrap();
    assert_eq!(kernel.session(id).unwrap().unwrap().id(), id);
    let resolved = kernel.resolved_profile(id).unwrap();
    assert_eq!(resolved.revision, 0);
    assert_eq!(resolved.model.as_deref(), Some("legacy-model"));
    assert_eq!(
        kernel.turns(id).unwrap()[0]
            .failure
            .as_ref()
            .unwrap()
            .retryable,
        None
    );
    assert_eq!(kernel.approvals(id).unwrap()[0].id, aid);
    assert_eq!(
        kernel.history(id, 0, None, 200).unwrap().events[0]
            .id
            .to_string(),
        event
    );
    let workers = supervisor(kernel.clone()).await;
    workers.resume(id).await.unwrap();
    workers.detach(id).await.unwrap();
    workers.shutdown().await.unwrap();
    drop(workers);
    drop(kernel);
    let db = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        4
    );
    let persisted: String = db
        .query_row("SELECT snapshot FROM worker_state", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&persisted).unwrap()["provider_session_id"],
        native
    );
}

#[test]
fn malformed_phase4_worker_rolls_back_profile_migration() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("corrupt.sqlite3");
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch(concat!(
        include_str!("../migrations/001_sessions.sql"),
        include_str!("../migrations/002_worker_state.sql"),
        include_str!("../migrations/003_history.sql")
    ))
    .unwrap();
    let id = WorkSessionId::new();
    let session = json!({"id":id,"backend":"codex","runtime_profile_id":"codex/local","workspace":dir.path(),"state":"idle","created_at_ms":1,"updated_at_ms":2});
    db.execute(
        "INSERT INTO work_sessions VALUES (?1, ?2)",
        (id.to_string(), session.to_string()),
    )
    .unwrap();
    db.execute(
        "INSERT INTO worker_state VALUES (?1, '{}')",
        [id.to_string()],
    )
    .unwrap();
    assert!(Kernel::open(&path).is_err());
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        3
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM sqlite_master WHERE name = 'runtime_profiles'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}
