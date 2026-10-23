use anyhow::Result;
use lain_kernel::{
    actor::{
        Actor,
        domain::*,
        model::{ActorModel, Call, Reply, Request},
        tools::Tool,
    },
    backend::Supervisor,
    kernel::Kernel,
    profiles::{ProfileDefinition, WorkspacePolicy},
};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};
use uuid::Uuid;
const TOKEN: &str = "fixture-owner-token-0123456789abcdef";
#[derive(Default)]
struct Model {
    replies: Mutex<VecDeque<Reply>>,
    requests: Mutex<Vec<Request>>,
    hang: bool,
}
impl ActorModel for Model {
    fn respond(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
        self.requests.lock().unwrap().push(request);
        Box::pin(async move {
            if self.hang {
                std::future::pending::<()>().await;
            }
            self.replies
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| anyhow::anyhow!("fixture exhausted"))
        })
    }
}
fn prose(text: &str) -> Reply {
    Reply {
        text: text.into(),
        calls: vec![],
        usage_tokens: 10,
        continuation: vec![],
    }
}
fn call(name: &str, args: Value, n: usize) -> Reply {
    Reply {
        text: String::new(),
        calls: vec![Call {
            id: format!("call_{n}"),
            name: name.into(),
            arguments: args.to_string(),
        }],
        usage_tokens: 10,
        continuation: vec![
            json!({"type":"function_call","name":name,"call_id":format!("call_{n}"),"arguments":args.to_string()}),
        ],
    }
}
fn model(replies: Vec<Reply>) -> Arc<Model> {
    Arc::new(Model {
        replies: Mutex::new(replies.into()),
        ..Default::default()
    })
}
fn kernel(path: &std::path::Path) -> Arc<Kernel> {
    let k = Arc::new(Kernel::open(path).unwrap());
    k.mark_ready().unwrap();
    k
}
async fn wait(kernel: &Arc<Kernel>, id: ActorTurnId) -> ActorTurn {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let turn = kernel.actor_turn(id).unwrap();
            if turn.state != TurnState::Running {
                return turn;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap()
}
async fn workers(kernel: &Arc<Kernel>) -> Arc<Supervisor> {
    Supervisor::new(
        kernel.clone(),
        Some(env!("CARGO_BIN_EXE_lain-test-app-server").into()),
        TOKEN.parse().unwrap(),
        Duration::from_secs(300),
    )
    .await
}

#[tokio::test]
async fn channel_chat_and_revocation_leave_running_worker_and_terminal_chat_independent() {
    use lain_kernel::channels::{ChannelService, ConversationChannel, domain::*};
    struct Channel {
        online: tokio::sync::watch::Sender<bool>,
        output: tokio::sync::mpsc::Sender<Delivery>,
    }
    impl ConversationChannel for Channel {
        fn connected(&self) -> tokio::sync::watch::Receiver<bool> {
            self.online.subscribe()
        }
        fn send(
            &self,
            delivery: Delivery,
        ) -> Pin<Box<dyn Future<Output = SendOutcome> + Send + '_>> {
            Box::pin(async move {
                self.output.send(delivery).await.unwrap();
                SendOutcome::Confirmed("fixture".into())
            })
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.sqlite3"));
    let w = workers(&k).await;
    let worker = w
        .create(dir.path().to_owned(), Default::default())
        .await
        .unwrap();
    w.prompt(worker.id(), "wait".into()).await.unwrap();
    let actor = Actor::with_model(
        k.clone(),
        Some(w.clone()),
        TOKEN.parse().unwrap(),
        model(vec![prose("聊一会儿。"), prose("终端也在这里。")]),
    );
    let binding = k
        .channel_bind(
            BindingSpec {
                name: "fixture".into(),
                adapter: "fixture".into(),
                account: "a".into(),
                peer: "b".into(),
            },
            true,
        )
        .unwrap();
    let service = ChannelService::new(
        k.clone(),
        actor.clone(),
        binding.clone(),
        &TOKEN.parse().unwrap(),
    );
    let (output, mut replies) = tokio::sync::mpsc::channel(1);
    let channel = Arc::new(Channel {
        online: tokio::sync::watch::channel(true).0,
        output,
    });
    let task = tokio::spawn(service.clone().run(channel));
    service
        .receive(Input {
            account: "a".into(),
            peer: "b".into(),
            external_id: "1".into(),
            text: "工作时也能聊天吗".into(),
        })
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(10), replies.recv())
            .await
            .unwrap()
            .is_some()
    );
    k.channel_grant(binding.id, false, 1).unwrap();
    let terminal = k.create_conversation().unwrap();
    let turn = actor
        .submit(terminal.id, Uuid::new_v4(), "从终端继续".into())
        .await
        .unwrap();
    assert_eq!(wait(&k, turn.id).await.state, TurnState::Completed);
    assert_eq!(
        k.session(worker.id()).unwrap().unwrap().state(),
        lain_kernel::domain::WorkSessionState::Running
    );
    assert_eq!(
        k.session(worker.id()).unwrap().unwrap().runtime(),
        lain_kernel::domain::RuntimeAttachment::Attached
    );
    k.shutdown().unwrap();
    task.await.unwrap().unwrap();
    w.shutdown().await.unwrap();
}
#[tokio::test]
async fn persistent_conversation_typed_reads_and_idempotent_submissions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.sqlite3");
    let k = kernel(&path);
    let m = model(vec![
        call("work_list", json!({}), 1),
        prose("目前没有工作会话。"),
    ]);
    let actor = Actor::with_model(k.clone(), None, TOKEN.parse().unwrap(), m.clone());
    let c = k.create_conversation().unwrap();
    let request = Uuid::new_v4();
    let submitted = actor
        .submit(c.id, request, "看看目前有哪些任务。".into())
        .await
        .unwrap();
    let duplicate = actor
        .submit(c.id, request, "看看目前有哪些任务。".into())
        .await
        .unwrap();
    assert_eq!(submitted.id, duplicate.id);
    assert!(
        actor
            .submit(c.id, request, "different".into())
            .await
            .is_err()
    );
    let turn = wait(&k, submitted.id).await;
    assert_eq!(turn.state, TurnState::Completed);
    assert_eq!(turn.actions.len(), 1);
    assert_eq!(turn.actions[0].state, ActionState::Executed);
    assert_eq!(turn.messages.last().unwrap().content, "目前没有工作会话。");
    assert_eq!(m.requests.lock().unwrap().len(), 2);
    {
        let requests = m.requests.lock().unwrap();
        assert!(
            serde_json::to_string(&requests[1].input)
                .unwrap()
                .contains("function_call_output")
        );
        assert!(
            !serde_json::to_string(&requests[0].input)
                .unwrap()
                .contains(TOKEN)
        );
    }
    // Wait until detached task ownership is released before reopening the same database.
    drop(actor);
    drop(k);
    tokio::time::sleep(Duration::from_millis(25)).await;
    let k = kernel(&path);
    assert_eq!(
        k.actor_turn(turn.id)
            .unwrap()
            .messages
            .last()
            .unwrap()
            .content,
        "目前没有工作会话。"
    );
    assert_eq!(k.conversations().unwrap()[0].id, c.id);
    let next = model(vec![prose("还在这里。")]);
    let actor = Actor::with_model(k.clone(), None, TOKEN.parse().unwrap(), next.clone());
    let turn = actor
        .submit(c.id, Uuid::new_v4(), "继续".into())
        .await
        .unwrap();
    wait(&k, turn.id).await;
    assert!(
        serde_json::to_string(&next.requests.lock().unwrap()[0].input)
            .unwrap()
            .contains("目前没有工作会话")
    );
}
#[tokio::test]
async fn confirmation_creates_real_work_once_and_owner_rejection_executes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.sqlite3"));
    let w = workers(&k).await;
    let args = json!({"workspace":dir.path(),"profile_id":"codex/local","sandbox":"read_only","task":"hello"});
    let m = model(vec![
        call("work_create", args.clone(), 1),
        call("work_create", args, 2),
    ]);
    let actor = Actor::with_model(k.clone(), Some(w.clone()), TOKEN.parse().unwrap(), m);
    let c = k.create_conversation().unwrap();
    let turn = actor
        .submit(c.id, Uuid::new_v4(), "让 Codex 检查项目".into())
        .await
        .unwrap();
    let turn = wait(&k, turn.id).await;
    assert_eq!(turn.state, TurnState::PendingConfirmation);
    assert!(k.list_sessions().unwrap().is_empty());
    let action = turn.actions[0].id;
    let (first, second) = tokio::join!(
        actor.decide(turn.id, action, true),
        actor.decide(turn.id, action, true)
    );
    assert_ne!(first.is_ok(), second.is_ok());
    let turn = wait(&k, turn.id).await;
    assert_eq!(turn.state, TurnState::Completed);
    assert_eq!(turn.actions[0].state, ActionState::Executed);
    let sid = turn.actions[0].result.as_ref().unwrap()["outcome"]["work_session_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!(k.session(sid).unwrap().is_some());
    assert_eq!(k.list_sessions().unwrap().len(), 1);
    assert_eq!(
        turn.actions[0].result.as_ref().unwrap()["outcome"]["worker_completion"],
        "not_observed"
    );
    assert!(actor.decide(turn.id, action, true).await.is_err());
    let denied = actor
        .submit(c.id, Uuid::new_v4(), "再创建一个".into())
        .await
        .unwrap();
    let denied = wait(&k, denied.id).await;
    actor
        .decide(denied.id, denied.actions[0].id, false)
        .await
        .unwrap();
    assert_eq!(
        k.actor_turn(denied.id).unwrap().actions[0].state,
        ActionState::Rejected
    );
    assert_eq!(k.list_sessions().unwrap().len(), 1);
    w.shutdown().await.unwrap();
}
#[tokio::test]
async fn delegation_reuses_workspace_profile_policy_and_reports_failures_honestly() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.sqlite3"));
    let w = workers(&k).await;
    let mut alternate = ProfileDefinition::local();
    alternate.id = "codex/allowed".parse().unwrap();
    k.save_profile(alternate, None).unwrap();
    k.save_workspace_policy(WorkspacePolicy {
        workspace: dir.path().to_owned(),
        default_profile: "codex/allowed".parse().unwrap(),
        allowed_profiles: vec!["codex/allowed".parse().unwrap()],
    })
    .unwrap();
    for (workspace, profile, expected) in [
        (
            dir.path().to_owned(),
            "codex/local",
            "workspace_profile_denied",
        ),
        (dir.path().join("absent"), "codex/local", "invalid_input"),
    ] {
        let m = model(vec![call(
            "work_create",
            json!({"workspace":workspace,"profile_id":profile,"sandbox":"read_only","task":"hello"}),
            1,
        )]);
        let actor = Actor::with_model(k.clone(), Some(w.clone()), TOKEN.parse().unwrap(), m);
        let c = k.create_conversation().unwrap();
        let turn = actor
            .submit(c.id, Uuid::new_v4(), "delegate".into())
            .await
            .unwrap();
        let turn = wait(&k, turn.id).await;
        actor
            .decide(turn.id, turn.actions[0].id, true)
            .await
            .unwrap();
        let turn = wait(&k, turn.id).await;
        assert_eq!(turn.state, TurnState::Unknown);
        assert_eq!(turn.actions[0].state, ActionState::Unknown);
        assert_eq!(
            turn.actions[0].result.as_ref().unwrap()["outcome"]["error"],
            expected
        );
        assert!(k.list_sessions().unwrap().is_empty());
    }
    let other = tempfile::tempdir().unwrap();
    let m = model(vec![call(
        "work_create",
        json!({"workspace":other.path(),"profile_id":"codex/missing","sandbox":"read_only","task":"hello"}),
        1,
    )]);
    let actor = Actor::with_model(k.clone(), Some(w.clone()), TOKEN.parse().unwrap(), m);
    let c = k.create_conversation().unwrap();
    let turn = actor
        .submit(c.id, Uuid::new_v4(), "delegate".into())
        .await
        .unwrap();
    let turn = wait(&k, turn.id).await;
    actor
        .decide(turn.id, turn.actions[0].id, true)
        .await
        .unwrap();
    let turn = wait(&k, turn.id).await;
    assert_eq!(
        turn.actions[0].result.as_ref().unwrap()["outcome"]["error"],
        "not_found"
    );
    w.shutdown().await.unwrap();
}
#[tokio::test]
async fn interrupted_generation_and_restart_never_replay_actions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.sqlite3");
    let k = kernel(&path);
    let m = Arc::new(Model {
        hang: true,
        ..Default::default()
    });
    let actor = Actor::with_model(k.clone(), None, TOKEN.parse().unwrap(), m);
    let c = k.create_conversation().unwrap();
    let turn = actor
        .submit(c.id, Uuid::new_v4(), "hello".into())
        .await
        .unwrap();
    actor.cancel(turn.id).await.unwrap();
    let turn = wait(&k, turn.id).await;
    assert_eq!(turn.state, TurnState::Unknown);
    let (uncertain, _) = k
        .actor_submit(
            c.id,
            Uuid::new_v4(),
            "persist before model submission".into(),
        )
        .unwrap();
    drop(actor);
    drop(k);
    tokio::time::sleep(Duration::from_millis(30)).await;
    let k = kernel(&path);
    let recovered = k.actor_turn(uncertain.id).unwrap();
    assert_eq!(recovered.state, TurnState::Unknown);
    assert!(recovered.actions.is_empty());
    assert_eq!(recovered.model_requests, 0);
}
#[tokio::test]
async fn malformed_calls_secret_rejection_and_tool_budgets_are_bounded() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.sqlite3"));
    let m = model(vec![
        call("shell", json!({"command":"whoami"}), 1),
        call("work_list", json!({"unexpected":true}), 2),
        prose("must not run"),
    ]);
    let actor = Actor::with_model(k.clone(), None, TOKEN.parse().unwrap(), m.clone());
    let c = k.create_conversation().unwrap();
    assert!(
        actor
            .submit(c.id, Uuid::new_v4(), format!("show {TOKEN}"))
            .await
            .is_err()
    );
    let turn = actor
        .submit(c.id, Uuid::new_v4(), "hello".into())
        .await
        .unwrap();
    let turn = wait(&k, turn.id).await;
    assert_eq!(turn.state, TurnState::Unknown);
    assert_eq!(m.requests.lock().unwrap().len(), 2);
    assert!(turn.actions.is_empty());
    assert!(
        !serde_json::to_string(&k.actor_page(c.id, 0, 100).unwrap())
            .unwrap()
            .contains(TOKEN)
    );
    for tool in [
        "work_approve",
        "profile_update",
        "http_request",
        "file_read",
    ] {
        assert!(Tool::parse(tool, "{}").is_err());
    }
    let looping = model((1..=8).map(|n| call("work_list", json!({}), n)).collect());
    let actor = Actor::with_model(k.clone(), None, TOKEN.parse().unwrap(), looping.clone());
    let turn = actor
        .submit(c.id, Uuid::new_v4(), "loop".into())
        .await
        .unwrap();
    let turn = wait(&k, turn.id).await;
    assert_eq!(turn.state, TurnState::Unknown);
    assert_eq!(looping.requests.lock().unwrap().len(), 4);
    assert_eq!(turn.actions.len(), 4);
}
#[tokio::test]
async fn authenticated_actor_api_streams_snapshots_and_survives_observer_disconnect() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.sqlite3"));
    let m = model(vec![prose("hello")]);
    let actor = Actor::with_model(k.clone(), None, TOKEN.parse().unwrap(), m);
    let router = lain_kernel::api::actor_router(k.clone(), TOKEN.parse().unwrap(), None, actor);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let http = reqwest::Client::new();
    assert_eq!(
        http.post(format!("{base}/v1/conversations"))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    let c: Conversation = http
        .post(format!("{base}/v1/conversations"))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let request = Uuid::new_v4();
    let turn: ActorTurn = http
        .post(format!("{base}/v1/conversations/{}/turns", c.id))
        .bearer_auth(TOKEN)
        .json(&json!({"request_id":request,"text":"hello"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let response = http
        .get(format!("{base}/v1/actor-turns/{}/events", turn.id))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap();
    drop(response);
    let final_turn = wait(&k, turn.id).await;
    assert_eq!(final_turn.state, TurnState::Completed);
    let stream = http
        .get(format!("{base}/v1/actor-turns/{}/events", turn.id))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(stream.contains("event: turn"));
    assert!(stream.contains("completed"));
    assert!(!stream.contains(TOKEN));
    let diagnostic: Value = http
        .get(format!("{base}/v1/diagnostics"))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(diagnostic["model_calls"], 0);
    assert_eq!(diagnostic["storage"], "healthy");
    server.abort();
}

#[tokio::test]
async fn partial_delegation_keeps_the_created_work_identity_and_is_not_replayed() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.sqlite3"));
    let w = workers(&k).await;
    let m = model(vec![call(
        "work_create",
        json!({"workspace":dir.path(),"profile_id":"codex/local","sandbox":"read_only","task":"bad-frame"}),
        1,
    )]);
    let actor = Actor::with_model(k.clone(), Some(w.clone()), TOKEN.parse().unwrap(), m);
    let c = k.create_conversation().unwrap();
    let turn = actor
        .submit(c.id, Uuid::new_v4(), "delegate".into())
        .await
        .unwrap();
    let turn = wait(&k, turn.id).await;
    actor
        .decide(turn.id, turn.actions[0].id, true)
        .await
        .unwrap();
    let turn = wait(&k, turn.id).await;
    // Submission may be acknowledged just before the broken frame arrives. Both
    // outcomes retain the real session; neither can claim worker completion.
    assert_eq!(k.list_sessions().unwrap().len(), 1);
    let result = turn.actions[0].result.as_ref().unwrap();
    let sid = result["partial"]["work_session_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!(k.session(sid).unwrap().is_some());
    assert!(
        actor
            .decide(turn.id, turn.actions[0].id, true)
            .await
            .is_err()
    );
    assert!(
        !serde_json::to_string(result)
            .unwrap()
            .contains("worker_completed")
    );
    w.shutdown().await.unwrap();
}

#[tokio::test]
async fn confirmed_action_without_a_backend_records_failure_without_false_success() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.sqlite3"));
    let m = model(vec![call(
        "work_cancel",
        json!({"session_id":Uuid::new_v4()}),
        1,
    )]);
    let actor = Actor::with_model(k.clone(), None, TOKEN.parse().unwrap(), m);
    let c = k.create_conversation().unwrap();
    let turn = actor
        .submit(c.id, Uuid::new_v4(), "cancel work".into())
        .await
        .unwrap();
    let turn = wait(&k, turn.id).await;
    actor
        .decide(turn.id, turn.actions[0].id, true)
        .await
        .unwrap();
    let turn = wait(&k, turn.id).await;
    assert_eq!(turn.state, TurnState::Unknown);
    assert_eq!(
        turn.actions[0].result.as_ref().unwrap()["outcome"]["error"],
        "backend_unavailable"
    );
    assert_eq!(
        turn.actions[0].result.as_ref().unwrap()["outcome"]["success"],
        false
    );
    assert!(k.list_sessions().unwrap().is_empty());
}

#[tokio::test]
async fn profile_health_remains_useful_without_model_secret_handles_or_missing_credentials() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.sqlite3"));
    let w = workers(&k).await;
    let mut profile = ProfileDefinition::local();
    profile.id = "codex/missing-credential".parse().unwrap();
    profile.model = Some("fixture".into());
    profile.provider = Some("fixture".into());
    let reference = format!("secret://env/phase-six-missing-{}", Uuid::new_v4().simple());
    profile.secret_ref = Some(reference.parse().unwrap());
    k.save_profile(profile, None).unwrap();
    let m = model(vec![
        call("profile_list", json!({}), 1),
        prose("The profile credential is missing."),
        call(
            "work_create",
            json!({"workspace":dir.path(),"profile_id":"codex/missing-credential","sandbox":"read_only","task":"hello"}),
            2,
        ),
    ]);
    let actor = Actor::with_model(
        k.clone(),
        Some(w.clone()),
        TOKEN.parse().unwrap(),
        m.clone(),
    );
    let c = k.create_conversation().unwrap();
    let turn = actor
        .submit(c.id, Uuid::new_v4(), "inspect profiles".into())
        .await
        .unwrap();
    let turn = wait(&k, turn.id).await;
    assert_eq!(turn.state, TurnState::Completed);
    let result = turn.actions[0].result.as_ref().unwrap();
    let missing = result["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "codex/missing-credential")
        .unwrap();
    assert_eq!(missing["secret_status"], "missing");
    assert!(missing.get("secret_ref").is_none());
    assert!(
        !serde_json::to_string(&m.requests.lock().unwrap()[1].input)
            .unwrap()
            .contains(&reference)
    );
    let turn = actor
        .submit(c.id, Uuid::new_v4(), "delegate with that profile".into())
        .await
        .unwrap();
    let turn = wait(&k, turn.id).await;
    actor
        .decide(turn.id, turn.actions[0].id, true)
        .await
        .unwrap();
    let turn = wait(&k, turn.id).await;
    assert_eq!(
        turn.actions[0].result.as_ref().unwrap()["outcome"]["error"],
        "backend_unavailable"
    );
    assert!(k.list_sessions().unwrap().is_empty());
    w.shutdown().await.unwrap();
}
