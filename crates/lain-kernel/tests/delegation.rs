//! End-to-end deterministic dispatch and result projection; no provider quota.
use anyhow::Result;
use lain_kernel::{
    actor::{
        Actor,
        domain::*,
        model::{ActorModel, Call, Reply, Request},
    },
    backend::Supervisor,
    delegation::{NoticeOutcome, SubmissionState},
    kernel::Kernel,
};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use uuid::Uuid;
const TOKEN: &str = "fixture-owner-token-0123456789abcdef";
#[derive(Default)]
struct Model {
    calls: AtomicUsize,
    tools: Mutex<VecDeque<(String, Value)>>,
}
impl ActorModel for Model {
    fn respond(&self, _: Request) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            let (name, arguments) = self
                .tools
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected model inference");
            Ok(Reply {
                text: String::new(),
                calls: vec![Call {
                    id: Uuid::new_v4().to_string(),
                    name,
                    arguments: arguments.to_string(),
                }],
                usage_tokens: 1,
                continuation: vec![],
            })
        })
    }
}
fn open(path: &std::path::Path) -> Arc<Kernel> {
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
async fn propose(
    actor: &Arc<Actor>,
    model: &Model,
    k: &Kernel,
    c: ConversationId,
    name: &str,
    args: Value,
    approve: bool,
) -> ActorTurn {
    model.tools.lock().unwrap().push_back((name.into(), args));
    let initial = actor
        .submit(c, Uuid::new_v4(), "owner request".into())
        .await
        .unwrap();
    let pending = settled(k, initial.id).await;
    assert_eq!(pending.state, TurnState::PendingConfirmation);
    actor
        .decide(pending.id, pending.actions[0].id, approve)
        .await
        .unwrap();
    settled(k, pending.id).await
}
async fn notices(
    k: &Kernel,
    c: ConversationId,
    count: usize,
) -> Vec<lain_kernel::delegation::ConversationNotice> {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let page = k.notices(c, 0, 100).unwrap();
            if page.notices.len() >= count {
                return page.notices;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap()
}
async fn workers(k: &Arc<Kernel>) -> Arc<Supervisor> {
    Supervisor::new(
        k.clone(),
        Some(env!("CARGO_BIN_EXE_lain-test-app-server").into()),
        TOKEN.parse().unwrap(),
        Duration::from_secs(300),
    )
    .await
}
fn create(workspace: &std::path::Path, task: &str) -> Value {
    json!({"workspace":workspace,"profile_id":"codex/local","sandbox":"read_only","task":task})
}

#[tokio::test]
async fn terminal_delivery_preserves_exact_dispatch_and_completed_actor_without_inference() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.sqlite3");
    let k = open(&path);
    let w = workers(&k).await;
    let m = Arc::new(Model::default());
    let actor = Actor::with_model(
        k.clone(),
        Some(w.clone()),
        TOKEN.parse().unwrap(),
        m.clone(),
    );
    let c = k.create_conversation().unwrap();
    let other = k.create_conversation().unwrap();
    let turn = propose(
        &actor,
        &m,
        &k,
        c.id,
        "work_create",
        create(dir.path(), "result-evidence"),
        true,
    )
    .await;
    let frozen = serde_json::to_value(&turn).unwrap();
    let first = notices(&k, c.id, 1).await.remove(0);
    assert_eq!(
        first.outcome,
        NoticeOutcome::Worker {
            state: lain_kernel::domain::TurnState::Completed
        }
    );
    assert_eq!(
        first.evidence.final_assistant_message.as_deref(),
        Some("result-evidence")
    );
    assert_eq!(first.evidence.failed_command_count, 12);
    assert_eq!(first.evidence.failed_commands.len(), 8);
    assert!(first.evidence.truncated);
    let link = k.delegations(c.id).unwrap().remove(0);
    assert_eq!(link.actor_turn_id, turn.id);
    assert_eq!(link.action_id, turn.actions[0].id);
    assert_eq!(link.worker_turn_id, first.worker_turn_id);
    assert_eq!(link.instruction, "result-evidence");
    assert_eq!(link.submission, SubmissionState::Accepted);
    assert_eq!(m.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        serde_json::to_value(k.actor_turn(turn.id).unwrap()).unwrap(),
        frozen
    );
    assert!(k.notices(other.id, 0, 100).unwrap().notices.is_empty());
    assert!(k.acknowledge_notice(other.id, first.id).is_err());
    k.acknowledge_notice(c.id, first.id).unwrap();
    k.acknowledge_notice(c.id, first.id).unwrap();
    assert!(k.notices(c.id, 0, 100).unwrap().notices[0].acknowledged);
    assert!(
        actor
            .decide(turn.id, turn.actions[0].id, true)
            .await
            .is_err()
    );
    assert_eq!(k.turns(link.work_session_id).unwrap().len(), 1);
    // A later Actor action targets a distinct Turn in the same WorkSession.
    let followup = propose(
        &actor,
        &m,
        &k,
        c.id,
        "work_send",
        json!({"session_id":link.work_session_id,"text":"second exact instruction"}),
        true,
    )
    .await;
    let both = notices(&k, c.id, 2).await;
    assert_eq!(both[0].id, first.id);
    assert!(both[1].cursor > first.cursor);
    assert_ne!(both[1].worker_turn_id, first.worker_turn_id);
    assert_eq!(both[1].actor_turn_id, followup.id);
    assert_eq!(
        both[1].evidence.final_assistant_message.as_deref(),
        Some("second exact instruction")
    );
    assert_eq!(both[1].evidence.failed_command_count, 0);
    assert_eq!(
        k.notices(c.id, first.cursor, 1).unwrap().notices[0].id,
        both[1].id
    );
    assert_eq!(m.calls.load(Ordering::SeqCst), 2);
    w.shutdown().await.unwrap();
    drop(actor);
    drop(w);
    drop(k);
    tokio::time::sleep(Duration::from_millis(25)).await;
    let recovered = open(&path);
    let restored = recovered.notices(c.id, 0, 100).unwrap();
    assert_eq!(restored.notices.len(), 2);
    assert_eq!(restored.notices[0].id, first.id);
    assert!(restored.notices[0].acknowledged);
    assert!(!restored.notices[1].acknowledged);
    assert_eq!(recovered.delegations(c.id).unwrap().len(), 2);
}

#[tokio::test]
async fn denial_failure_cancellation_and_restart_uncertainty_are_distinct() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.sqlite3");
    let k = open(&path);
    let w = workers(&k).await;
    let m = Arc::new(Model::default());
    let actor = Actor::with_model(
        k.clone(),
        Some(w.clone()),
        TOKEN.parse().unwrap(),
        m.clone(),
    );
    let c = k.create_conversation().unwrap();
    propose(
        &actor,
        &m,
        &k,
        c.id,
        "work_create",
        create(dir.path(), "hello"),
        false,
    )
    .await;
    assert!(k.delegations(c.id).unwrap().is_empty());
    assert!(k.list_sessions().unwrap().is_empty());
    for (task, expected) in [
        ("reject", lain_kernel::domain::TurnState::Failed),
        ("auth-failure", lain_kernel::domain::TurnState::Failed),
        ("history-output", lain_kernel::domain::TurnState::Completed),
    ] {
        let turn = propose(
            &actor,
            &m,
            &k,
            c.id,
            "work_create",
            create(dir.path(), task),
            true,
        )
        .await;
        let list = notices(&k, c.id, k.delegations(c.id).unwrap().len()).await;
        let notice = list.iter().find(|n| n.actor_turn_id == turn.id).unwrap();
        assert_eq!(notice.outcome, NoticeOutcome::Worker { state: expected });
        assert!(!serde_json::to_string(notice).unwrap().contains("sk-secret"));
        if task == "history-output" {
            assert!(notice.evidence.truncated);
            assert!(
                notice
                    .evidence
                    .final_assistant_message
                    .as_ref()
                    .unwrap()
                    .len()
                    <= 8192
            );
            assert_eq!(notice.evidence.changed_files.len(), 16);
        }
        if task == "reject" {
            assert_eq!(
                k.delegations(c.id)
                    .unwrap()
                    .iter()
                    .find(|l| l.actor_turn_id == turn.id)
                    .unwrap()
                    .submission,
                SubmissionState::Rejected
            );
        }
    }
    let cancelled = propose(
        &actor,
        &m,
        &k,
        c.id,
        "work_create",
        create(dir.path(), "wait"),
        true,
    )
    .await;
    let sid = k
        .delegations(c.id)
        .unwrap()
        .iter()
        .find(|l| l.actor_turn_id == cancelled.id)
        .unwrap()
        .work_session_id;
    w.cancel(sid).await.unwrap();
    let all = notices(&k, c.id, 4).await;
    assert_eq!(
        all.last().unwrap().outcome,
        NoticeOutcome::Worker {
            state: lain_kernel::domain::TurnState::Cancelled
        }
    );
    let uncertain = propose(
        &actor,
        &m,
        &k,
        c.id,
        "work_create",
        create(dir.path(), "crash"),
        true,
    )
    .await;
    let sid = k
        .delegations(c.id)
        .unwrap()
        .iter()
        .find(|l| l.actor_turn_id == uncertain.id)
        .unwrap()
        .work_session_id;
    let _ = w.cancel(sid).await;
    let all = notices(&k, c.id, 5).await;
    assert_eq!(
        all.last().unwrap().outcome,
        NoticeOutcome::Worker {
            state: lain_kernel::domain::TurnState::Unknown
        }
    );
    let lost_ack = propose(
        &actor,
        &m,
        &k,
        c.id,
        "work_create",
        create(dir.path(), "lost-prompt-ack"),
        true,
    )
    .await;
    let all = notices(&k, c.id, 6).await;
    let lost = all.iter().find(|n| n.actor_turn_id == lost_ack.id).unwrap();
    assert_eq!(
        lost.outcome,
        NoticeOutcome::Worker {
            state: lain_kernel::domain::TurnState::Unknown
        }
    );
    let link = k
        .delegations(c.id)
        .unwrap()
        .into_iter()
        .find(|l| l.actor_turn_id == lost_ack.id)
        .unwrap();
    assert_eq!(link.submission, SubmissionState::Unknown);
    assert_eq!(k.turns(link.work_session_id).unwrap().len(), 1);
    assert!(
        actor
            .decide(lost_ack.id, lost_ack.actions[0].id, true)
            .await
            .is_err()
    );
    assert_eq!(m.calls.load(Ordering::SeqCst), 7);
    w.shutdown().await.unwrap();
    drop(actor);
    drop(w);
    drop(k);
    tokio::time::sleep(Duration::from_millis(25)).await;
    let recovered = open(&path);
    assert_eq!(recovered.notices(c.id, 0, 100).unwrap().notices.len(), 6);
}

#[tokio::test]
async fn independent_delegations_return_out_of_order_in_one_conversation() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    std::fs::create_dir(&a).unwrap();
    std::fs::create_dir(&b).unwrap();
    let k = open(&dir.path().join("state.sqlite3"));
    let w = workers(&k).await;
    let m = Arc::new(Model::default());
    let actor = Actor::with_model(
        k.clone(),
        Some(w.clone()),
        TOKEN.parse().unwrap(),
        m.clone(),
    );
    let c = k.create_conversation().unwrap();
    let first = propose(
        &actor,
        &m,
        &k,
        c.id,
        "work_create",
        create(&a, "wait"),
        true,
    )
    .await;
    let second = propose(
        &actor,
        &m,
        &k,
        c.id,
        "work_create",
        create(&b, "second task finishes first"),
        true,
    )
    .await;
    let arrived = notices(&k, c.id, 1).await;
    assert_eq!(arrived[0].actor_turn_id, second.id);
    let sid = k
        .delegations(c.id)
        .unwrap()
        .iter()
        .find(|l| l.actor_turn_id == first.id)
        .unwrap()
        .work_session_id;
    assert_eq!(
        k.turns(sid).unwrap()[0].state,
        lain_kernel::domain::TurnState::Running
    );
    w.cancel(sid).await.unwrap();
    let both = notices(&k, c.id, 2).await;
    assert_eq!(both[1].actor_turn_id, first.id);
    assert!(both[1].cursor > both[0].cursor);
    assert_eq!(m.calls.load(Ordering::SeqCst), 2);
    w.shutdown().await.unwrap();
}

#[test]
fn phase_6_5_migration_preserves_completed_conversation_without_guessing_links() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("phase65.sqlite3");
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch(concat!(
        include_str!("../migrations/001_sessions.sql"),
        include_str!("../migrations/002_worker_state.sql"),
        include_str!("../migrations/003_history.sql"),
        include_str!("../migrations/004_profiles.sql"),
        include_str!("../migrations/005_actor.sql")
    ))
    .unwrap();
    let c = ConversationId::new();
    let tid = ActorTurnId::new();
    let mid = MessageId::new();
    let snapshot = json!({"_message_format":2,"id":tid,"conversation_id":c,"request_id":Uuid::new_v4(),"sequence":1,"revision":3,"state":"completed","messages":[{"id":mid,"conversation_id":c,"role":"assistant","content":"Phase 6.5 body retained","timestamp_ms":1,"status":"completed"}],"actions":[],"error":null,"model_requests":1,"usage_tokens":12,"timing":lain_wire::ActorTiming::default()});
    db.execute(
        "INSERT INTO lain_conversations VALUES (?1, 1, 1)",
        [c.to_string()],
    )
    .unwrap();
    db.execute("INSERT INTO lain_actor_turns(id, conversation_id, request_id, snapshot) VALUES (?1, ?2, ?3, ?4)", rusqlite::params![tid.to_string(),c.to_string(),snapshot["request_id"].as_str().unwrap(),snapshot.to_string()]).unwrap();
    drop(db);
    let k = open(&path);
    assert_eq!(k.actor_turn(tid).unwrap().messages[0].id, mid);
    assert_eq!(
        k.actor_turn(tid).unwrap().messages[0].content,
        "Phase 6.5 body retained"
    );
    assert_eq!(k.actor_turn(tid).unwrap().revision, 3);
    assert!(k.delegations(c).unwrap().is_empty());
    assert!(k.notices(c, 0, 100).unwrap().notices.is_empty());
}

async fn sse_page(response: &mut reqwest::Response, pending: &mut Vec<u8>) -> Value {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(end) = pending.windows(2).position(|s| s == b"\n\n") {
                let frame: Vec<_> = pending.drain(..end + 2).collect();
                let frame = std::str::from_utf8(&frame).unwrap();
                if let Some(data) = frame.lines().find_map(|line| line.strip_prefix("data: ")) {
                    return serde_json::from_str(data).unwrap();
                }
            } else {
                pending
                    .extend_from_slice(&response.chunk().await.unwrap().expect("SSE closed early"));
            }
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn conversation_sse_concurrent_results_reconnect_and_authenticated_acknowledgements() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    std::fs::create_dir(&a).unwrap();
    std::fs::create_dir(&b).unwrap();
    let k = open(&dir.path().join("state.sqlite3"));
    let w = workers(&k).await;
    let m = Arc::new(Model::default());
    let actor = Actor::with_model(
        k.clone(),
        Some(w.clone()),
        TOKEN.parse().unwrap(),
        m.clone(),
    );
    let c = k.create_conversation().unwrap();
    let other = k.create_conversation().unwrap();
    let first = propose(
        &actor,
        &m,
        &k,
        c.id,
        "work_create",
        create(&a, "wait"),
        true,
    )
    .await;
    let second = propose(
        &actor,
        &m,
        &k,
        other.id,
        "work_create",
        create(&b, "wait"),
        true,
    )
    .await;
    assert_eq!(first.state, TurnState::Completed);
    assert_eq!(second.state, TurnState::Completed);
    let sid_a = k.delegations(c.id).unwrap()[0].work_session_id;
    let sid_b = k.delegations(other.id).unwrap()[0].work_session_id;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let app = lain_kernel::api::actor_router(
        k.clone(),
        TOKEN.parse().unwrap(),
        Some(w.clone()),
        actor.clone(),
    );
    let (stop, shutdown) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown.await;
            })
            .await
            .unwrap();
    });
    let http = reqwest::Client::new();
    let url = format!("{base}/v1/conversations/{}/notices", c.id);
    assert_eq!(http.get(&url).send().await.unwrap().status(), 401);
    assert_eq!(
        http.get(format!("{url}?limit=0"))
            .bearer_auth(TOKEN)
            .send()
            .await
            .unwrap()
            .status(),
        400
    );
    let mut stream = http
        .get(format!("{url}/events"))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap();
    let mut pending = vec![];
    assert!(
        sse_page(&mut stream, &mut pending).await["notices"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let (cancel_a, cancel_b) = tokio::join!(w.cancel(sid_a), w.cancel(sid_b));
    cancel_a.unwrap();
    cancel_b.unwrap();
    let live = loop {
        let page = sse_page(&mut stream, &mut pending).await;
        if !page["notices"].as_array().unwrap().is_empty() {
            break page;
        }
    };
    let wire: lain_wire::notices::NoticePage = serde_json::from_value(live.clone()).unwrap();
    wire.validate(c.id, 0).unwrap();
    assert_eq!(wire.notices.len(), 1);
    assert_eq!(live["notices"][0]["work_session_id"], sid_a.to_string());
    assert_eq!(notices(&k, other.id, 1).await[0].work_session_id, sid_b);
    let id = wire.notices[0].id;
    let cursor = wire.next_cursor;
    drop(stream); // Receive-before-ack loss leaves the actual notice pending.
    let restored: Value = http
        .get(&url)
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(restored["notices"][0]["id"], id.to_string());
    assert_eq!(restored["notices"][0]["acknowledged"], false);
    assert_eq!(
        http.post(format!(
            "{base}/v1/conversations/{}/notices/{id}/ack",
            other.id
        ))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap()
        .status(),
        404
    );
    for _ in 0..2 {
        assert_eq!(
            http.post(format!("{url}/{id}/ack"))
                .bearer_auth(TOKEN)
                .send()
                .await
                .unwrap()
                .status(),
            200
        );
    }
    let mut reconnected = http
        .get(format!("{url}/events?after={cursor}"))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap();
    assert!(
        sse_page(&mut reconnected, &mut vec![]).await["notices"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    drop(reconnected);
    assert!(k.notices(c.id, 0, 100).unwrap().notices[0].acknowledged);
    assert_eq!(m.calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        serde_json::to_value(k.actor_turn(first.id).unwrap()).unwrap(),
        serde_json::to_value(first).unwrap()
    );
    stop.send(()).unwrap();
    server.await.unwrap();
    w.shutdown().await.unwrap();
}
