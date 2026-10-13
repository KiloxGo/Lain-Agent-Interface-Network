//! Regressions for cancelled callers, atomic delegation receipts and bounded history.
//! Reproduced from the independent workspace audit probes, then fixed.
use anyhow::Result;
use lain_kernel::{
    actor::{
        Actor,
        domain::*,
        model::{ActorModel, Call, Reply, Request},
    },
    backend::Supervisor,
    kernel::Kernel,
};
use serde_json::json;
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use uuid::Uuid;

const TOKEN: &str = "fixture-owner-token-0123456789abcdef";
struct Model {
    calls: AtomicUsize,
    tool: Option<(&'static str, String)>,
}
impl ActorModel for Model {
    fn respond(&self, _: Request) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            Ok(Reply {
                text: "audit".into(),
                calls: self
                    .tool
                    .as_ref()
                    .map(|(name, arguments)| Call {
                        id: "audit_call".into(),
                        name: (*name).into(),
                        arguments: arguments.clone(),
                    })
                    .into_iter()
                    .collect(),
                usage_tokens: 1,
                continuation: vec![],
            })
        })
    }
}
fn kernel(path: &std::path::Path) -> Arc<Kernel> {
    let k = Arc::new(Kernel::open(path).unwrap());
    k.mark_ready().unwrap();
    k
}
async fn settled(k: &Kernel, id: ActorTurnId) -> ActorTurn {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let turn = k.actor_turn(id).unwrap();
            if turn.state != TurnState::Running {
                return turn;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap()
}

#[test]
fn dropped_submit_still_commits_and_runs_once_under_kernel_ownership() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let k = kernel(&dir.path().join("audit.sqlite3"));
        let m = Arc::new(Model {
            calls: AtomicUsize::new(0),
            tool: None,
        });
        let actor = Actor::with_model(k.clone(), None, TOKEN.parse().unwrap(), m.clone());
        let c = k.create_conversation().unwrap();
        let request = Uuid::new_v4();
        let (release, blocked) = std::sync::mpsc::channel();
        let (entered, entering) = tokio::sync::oneshot::channel();
        let blocker = tokio::task::spawn_blocking(move || {
            entered.send(()).unwrap();
            blocked.recv().unwrap();
        });
        entering.await.unwrap();
        {
            let mut submit = Box::pin(actor.submit(c.id, request, "hello".into()));
            std::future::poll_fn(|cx| {
                assert!(submit.as_mut().poll(cx).is_pending());
                std::task::Poll::Ready(())
            })
            .await;
            // Allow the detached owner to queue storage behind the barrier.
            tokio::task::yield_now().await;
            // Drop the caller while its storage task is queued.
        }
        release.send(()).unwrap();
        blocker.await.unwrap();
        tokio::task::spawn_blocking(|| ()).await.unwrap();
        let existing = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                if let Some(turn) = k.actor_recent(c.id).unwrap().first() {
                    break turn.clone();
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let completed = settled(&k, existing.id).await;
        assert_eq!(completed.state, TurnState::Completed);
        let duplicate = actor.submit(c.id, request, "hello".into()).await.unwrap();
        assert_eq!(duplicate.id, existing.id);
        assert_eq!(m.calls.load(Ordering::SeqCst), 1);
        assert!(actor.cancel(existing.id).await.is_err());
        let next = actor
            .submit(c.id, Uuid::new_v4(), "next".into())
            .await
            .unwrap();
        assert_eq!(settled(&k, next.id).await.state, TurnState::Completed);
    });
}

#[tokio::test]
async fn failed_worker_start_retains_atomic_created_session_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("audit.sqlite3"));
    // The existing protocol fixture cannot write its startup marker to a directory.
    std::fs::create_dir(dir.path().join("fixture-config.json")).unwrap();
    let workers = Supervisor::new(
        k.clone(),
        Some(env!("CARGO_BIN_EXE_lain-test-app-server").into()),
        TOKEN.parse().unwrap(),
        Duration::from_secs(300),
    )
    .await;
    let m = Arc::new(Model { calls: AtomicUsize::new(0), tool: Some(("work_create", json!({"workspace":dir.path(),"profile_id":"codex/local","sandbox":"read_only","task":"hello"}).to_string())) });
    let actor = Actor::with_model(k.clone(), Some(workers.clone()), TOKEN.parse().unwrap(), m);
    let c = k.create_conversation().unwrap();
    let turn = actor
        .submit(c.id, Uuid::new_v4(), "delegate".into())
        .await
        .unwrap();
    let turn = settled(&k, turn.id).await;
    actor
        .decide(turn.id, turn.actions[0].id, true)
        .await
        .unwrap();
    let turn = settled(&k, turn.id).await;
    assert_eq!(turn.state, TurnState::Unknown);
    let sessions = k.list_sessions().unwrap();
    assert_eq!(sessions.len(), 1);
    let receipt = serde_json::to_string(&turn.actions[0].result).unwrap();
    assert!(receipt.contains(&sessions[0].id().to_string()));
    assert_eq!(
        turn.actions[0].result.as_ref().unwrap()["partial"]["stage"],
        "created_not_submitted"
    );
    assert!(
        actor
            .decide(turn.id, turn.actions[0].id, true)
            .await
            .is_err()
    );
    assert_eq!(k.list_sessions().unwrap().len(), 1);
    workers.shutdown().await.unwrap();
}

#[test]
fn recovery_rejects_contradictory_worker_status_and_decision_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audit.sqlite3");
    let k = kernel(&path);
    let session = k
        .create_session(
            "codex".parse().unwrap(),
            "codex/local".parse().unwrap(),
            dir.path().into(),
        )
        .unwrap();
    let turn_id = Uuid::new_v4();
    let worker = json!({"provider_session_id":"native","options":{},"resolved_profile":null,
        "turns":[{"provider_turn_id":"native-turn","turn":{"id":turn_id,"work_session_id":session.id(),"state":"completed","cancellation":"not_requested","failure":null,"created_at_ms":1,"updated_at_ms":1}}],
        "approvals":[{"id":Uuid::new_v4(),"work_session_id":session.id(),"turn_id":turn_id,"kind":"command","description":"audit","decisions":["deny"],"status":"resolved","decision":"deny","created_at_ms":1,"updated_at_ms":1}]});
    drop(k);
    for invalid in [
        {
            let mut value = worker.clone();
            value["turns"][0]["turn"]["cancellation"] = "confirmed".into();
            value
        },
        {
            let mut value = worker.clone();
            value["approvals"][0]["decision"] = serde_json::Value::Null;
            value
        },
        {
            let mut value = worker.clone();
            value["approvals"][0]["decision"] = "approve_once".into();
            value
        },
        {
            let mut value = worker.clone();
            value["approvals"][0]["status"] = "pending".into();
            value
        },
    ] {
        let db = rusqlite::Connection::open(&path).unwrap();
        db.execute(
            "INSERT OR REPLACE INTO worker_state(session_id,snapshot) VALUES (?1,?2)",
            (session.id().to_string(), invalid.to_string()),
        )
        .unwrap();
        drop(db);
        assert!(Kernel::open(&path).is_err());
        let db = rusqlite::Connection::open(&path).unwrap();
        let saved: String = db
            .query_row(
                "SELECT snapshot FROM worker_state WHERE session_id = ?1",
                [session.id().to_string()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&saved).unwrap(),
            invalid
        );
    }
}

#[tokio::test]
async fn bounded_actor_history_advances_only_through_retained_records() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("audit.sqlite3"));
    let workers = Supervisor::new(
        k.clone(),
        Some(env!("CARGO_BIN_EXE_lain-test-app-server").into()),
        TOKEN.parse().unwrap(),
        Duration::from_secs(300),
    )
    .await;
    let session = workers
        .create(dir.path().into(), Default::default())
        .await
        .unwrap();
    let turn = workers
        .prompt(session.id(), "history-output".into())
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if k.turns(session.id())
                .unwrap()
                .iter()
                .any(|t| t.id == turn.id && t.state.is_terminal())
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let model = Arc::new(Model {
        calls: AtomicUsize::new(0),
        tool: Some((
            "work_history",
            json!({"session_id":session.id(),"after":0}).to_string(),
        )),
    });
    let actor = Actor::with_model(
        k.clone(),
        Some(workers.clone()),
        TOKEN.parse().unwrap(),
        model,
    );
    let conversation = k.create_conversation().unwrap();
    let turn = actor
        .submit(conversation.id, Uuid::new_v4(), "history".into())
        .await
        .unwrap();
    let turn = settled(&k, turn.id).await;
    let result = turn.actions[0].result.as_ref().unwrap();
    assert_eq!(result["truncated"], true);
    let data = &result["data"];
    let events = data["events"].as_array().unwrap();
    assert!(
        events.last().unwrap()["history_order"].as_u64().unwrap()
            == data["next_cursor"].as_u64().unwrap()
    );
    assert_eq!(data["has_more"], true);
    let retained = data["next_cursor"].as_u64().unwrap();
    assert!(
        !k.history(session.id(), retained, None, 200)
            .unwrap()
            .events
            .is_empty()
    );
    workers.shutdown().await.unwrap();
}

#[test]
fn dropped_owner_decision_still_executes_once_or_records_uncertainty() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let k = kernel(&dir.path().join("decision.sqlite3"));
        let model = Arc::new(Model { calls: AtomicUsize::new(0), tool: Some(("work_create",
            json!({"workspace":dir.path(),"profile_id":"codex/local","sandbox":"read_only","task":"hello"}).to_string())) });
        let actor = Actor::with_model(k.clone(), None, TOKEN.parse().unwrap(), model.clone());
        let c = k.create_conversation().unwrap();
        let turn = actor.submit(c.id, Uuid::new_v4(), "delegate".into()).await.unwrap();
        let turn = settled(&k, turn.id).await;
        assert_eq!(turn.state, TurnState::PendingConfirmation);
        let (release, blocked) = std::sync::mpsc::channel();
        let (entered, entering) = tokio::sync::oneshot::channel();
        let blocker = tokio::task::spawn_blocking(move || { entered.send(()).unwrap(); blocked.recv().unwrap(); });
        entering.await.unwrap();
        {
            let mut decision = Box::pin(actor.decide(turn.id, turn.actions[0].id, true));
            std::future::poll_fn(|cx| {
                assert!(decision.as_mut().poll(cx).is_pending());
                std::task::Poll::Ready(())
            }).await;
            tokio::task::yield_now().await;
        }
        release.send(()).unwrap();
        blocker.await.unwrap();
        let outcome = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let saved = k.actor_turn(turn.id).unwrap();
                if saved.state == TurnState::Unknown { break saved; }
                tokio::task::yield_now().await;
            }
        }).await.unwrap();
        assert_eq!(outcome.actions[0].state, ActionState::Unknown);
        assert!(outcome.actions[0].result.is_some());
        assert!(actor.decide(turn.id, turn.actions[0].id, true).await.is_err());
        assert_eq!(model.calls.load(Ordering::SeqCst), 1);
    });
}

#[tokio::test]
async fn failed_atomic_actor_receipt_rolls_back_worker_creation_before_start() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("atomic.sqlite3");
    let k = kernel(&path);
    let workers = Supervisor::new(
        k.clone(),
        Some(env!("CARGO_BIN_EXE_lain-test-app-server").into()),
        TOKEN.parse().unwrap(),
        Duration::from_secs(300),
    )
    .await;
    let model = Arc::new(Model { calls: AtomicUsize::new(0), tool: Some(("work_create",
        json!({"workspace":dir.path(),"profile_id":"codex/local","sandbox":"read_only","task":"hello"}).to_string())) });
    let actor = Actor::with_model(
        k.clone(),
        Some(workers.clone()),
        TOKEN.parse().unwrap(),
        model,
    );
    let c = k.create_conversation().unwrap();
    let turn = actor
        .submit(c.id, Uuid::new_v4(), "delegate".into())
        .await
        .unwrap();
    let turn = settled(&k, turn.id).await;
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch(
        "CREATE TRIGGER fail_actor_receipt BEFORE UPDATE ON lain_actor_turns
        WHEN json_extract(NEW.snapshot, '$.actions[0].result.stage') = 'created_not_submitted'
        BEGIN SELECT RAISE(ABORT, 'fixture receipt failure'); END;",
    )
    .unwrap();
    actor
        .decide(turn.id, turn.actions[0].id, true)
        .await
        .unwrap();
    let outcome = settled(&k, turn.id).await;
    assert_eq!(outcome.state, TurnState::Unknown);
    assert_eq!(outcome.actions[0].state, ActionState::Unknown);
    assert!(outcome.actions[0].result.as_ref().unwrap()["partial"].is_null());
    assert!(k.list_sessions().unwrap().is_empty());
    let count: i64 = db
        .query_row("SELECT count(*) FROM work_sessions", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
    assert!(!dir.path().join("fixture-config.json").exists());
    workers.shutdown().await.unwrap();
}

#[tokio::test]
async fn failed_final_metadata_commit_never_leaves_a_false_completed_turn() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("final.sqlite3");
    let k = kernel(&path);
    let model = Arc::new(Model {
        calls: AtomicUsize::new(0),
        tool: None,
    });
    let actor = Actor::with_model(k.clone(), None, TOKEN.parse().unwrap(), model);
    let c = k.create_conversation().unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch(
        "CREATE TRIGGER fail_final_metadata BEFORE UPDATE ON lain_actor_turns
        WHEN json_extract(NEW.snapshot, '$.state') = 'completed'
          AND json_extract(NEW.snapshot, '$.timing.total_ms') IS NOT NULL
        BEGIN SELECT RAISE(ABORT, 'fixture final metadata failure'); END;",
    )
    .unwrap();
    let initial = actor
        .submit(c.id, Uuid::new_v4(), "hello".into())
        .await
        .unwrap();
    let outcome = settled(&k, initial.id).await;
    assert_eq!(outcome.state, TurnState::Unknown);
    let completed_bodies: Vec<_> = outcome
        .messages
        .iter()
        .filter(|m| m.role == MessageRole::Assistant)
        .collect();
    assert_eq!(completed_bodies.len(), 1);
    assert_eq!(completed_bodies[0].content, "audit");
    assert_eq!(completed_bodies[0].status, MessageStatus::Completed);
}
