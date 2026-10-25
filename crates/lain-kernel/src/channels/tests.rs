use super::*;
use crate::actor::domain::MessageRole;
use crate::actor::model::{ActorModel, Reply, Request};
use crate::persistence::Store;
use serde_json::{Value, json};
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::Semaphore;

const TOKEN: &str = "fixture-owner-token-0123456789abcdef";
#[path = "chat_tests.rs"]
mod chat;
#[path = "diagnostics_tests.rs"]
mod diagnostics;
#[path = "private_memory_tests.rs"]
mod private_memory;
#[path = "source_tests.rs"]
mod source;
fn spec() -> BindingSpec {
    BindingSpec {
        name: "fixture".into(),
        adapter: "fixture".into(),
        account: "1001".into(),
        peer: "2002".into(),
    }
}
fn input(id: &str, text: &str) -> Input {
    Input {
        account: "1001".into(),
        peer: "2002".into(),
        external_id: id.into(),
        text: text.into(),
    }
}
struct Model {
    requests: Mutex<Vec<Request>>,
    calls: AtomicUsize,
    gate: Semaphore,
}
impl ActorModel for Model {
    fn respond(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
        self.requests.lock().unwrap().push(request);
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            self.gate.acquire().await.unwrap().forget();
            Ok(Reply {
                text: "在这里。刚才的话我听到了。".into(),
                calls: vec![],
                usage_tokens: 10,
                continuation: vec![],
            })
        })
    }
}
struct FixtureChannel {
    connected: watch::Sender<bool>,
    sent: Mutex<Vec<Delivery>>,
    outcome: SendOutcome,
}
impl ConversationChannel for FixtureChannel {
    fn connected(&self) -> watch::Receiver<bool> {
        self.connected.subscribe()
    }
    fn send(&self, delivery: Delivery) -> Pin<Box<dyn Future<Output = SendOutcome> + Send + '_>> {
        self.sent.lock().unwrap().push(delivery);
        Box::pin(async { self.outcome.clone() })
    }
}
fn fixture_channel(online: bool, outcome: SendOutcome) -> Arc<FixtureChannel> {
    Arc::new(FixtureChannel {
        connected: watch::channel(online).0,
        sent: Mutex::default(),
        outcome,
    })
}
fn kernel(path: &std::path::Path) -> Arc<Kernel> {
    let kernel = Arc::new(Kernel::open(path).unwrap());
    kernel.mark_ready().unwrap();
    kernel
}
fn service(kernel: &Arc<Kernel>, permits: usize) -> (Arc<ChannelService>, Arc<Model>) {
    let model = Arc::new(Model {
        requests: Mutex::default(),
        calls: AtomicUsize::new(0),
        gate: Semaphore::new(permits),
    });
    let actor = Actor::with_model(kernel.clone(), None, TOKEN.parse().unwrap(), model.clone());
    let binding = kernel.channel_bind(spec(), true).unwrap();
    (
        ChannelService::new(kernel.clone(), actor, binding, &TOKEN.parse().unwrap()),
        model,
    )
}
async fn eventually(check: impl Fn() -> bool) {
    tokio::time::timeout(Duration::from_secs(15), async {
        while !check() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("fixture condition timed out");
}

#[test]
fn durable_binding_dedup_grants_and_restart_never_resurrect_sends() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.sqlite3");
    let mut store = Store::open(&path).unwrap();
    let binding = store.channel_bind(spec(), true).unwrap();
    let incoming = store
        .channel_receive(binding.id, input("1", "晚上好"))
        .unwrap();
    assert_eq!(
        incoming.id,
        store
            .channel_receive(binding.id, input("1", "晚上好"))
            .unwrap()
            .id
    );
    assert!(
        store
            .channel_receive(binding.id, input("1", "different"))
            .is_err()
    );
    assert!(
        store
            .channel_receive(
                binding.id,
                Input {
                    peer: "9999".into(),
                    ..input("2", "wrong")
                }
            )
            .is_err()
    );
    let now = incoming.received_at_ms + MERGE_MS + 1;
    let batch = store.channel_next_batch(binding.id, now).unwrap().unwrap();
    let mut turn = crate::actor::domain::ActorTurn {
        id: crate::actor::domain::ActorTurnId::new(),
        conversation_id: binding.conversation_id,
        request_id: batch.id.0,
        sequence: 0,
        revision: 0,
        state: TurnState::Completed,
        messages: vec![],
        actions: vec![],
        error: None,
        model_requests: 1,
        usage_tokens: 1,
        timing: Default::default(),
        channel_memory: None,
        channel_source: None,
        private_chat: None,
    };
    turn.message(MessageRole::User, batch.text.clone()).unwrap();
    turn.message(MessageRole::Assistant, "晚上好。".into())
        .unwrap();
    store.save_actor_turn(&mut turn, true).unwrap();
    // Crash after Actor commit but before link/outbox: recover the exact request.
    drop(store);
    let mut store = Store::open(&path).unwrap();
    store.recover_actor().unwrap();
    store.recover_channels().unwrap();
    let restored = store.channel_next_batch(binding.id, now).unwrap().unwrap();
    assert_eq!(restored.id, batch.id);
    assert_eq!(
        store.channel_actor_request(&restored).unwrap().unwrap().id,
        turn.id
    );
    store.channel_finish(batch.id, &turn).unwrap();
    store.channel_finish(batch.id, &turn).unwrap();
    let delivery = store.channel_claim_delivery(binding.id).unwrap().unwrap();
    assert!(store.channel_claim_delivery(binding.id).unwrap().is_none());
    // Crash after claiming, including after platform acceptance but before receipt commit.
    drop(store);
    let mut store = Store::open(&path).unwrap();
    store.recover_channels().unwrap();
    let snapshot = store.channel_snapshot(binding.id).unwrap();
    assert_eq!(snapshot.deliveries.len(), 1);
    assert_eq!(snapshot.deliveries[0].id, delivery.id);
    assert_eq!(snapshot.deliveries[0].state, DeliveryState::Unknown);
    assert!(store.channel_claim_delivery(binding.id).unwrap().is_none());
    assert_eq!(
        store.channel_bind(spec(), true).unwrap().conversation_id,
        binding.conversation_id
    );
    let revoked = store
        .channel_grant(binding.id, false, binding.grant_revision)
        .unwrap();
    assert!(!store.channel_bind(spec(), true).unwrap().reply_enabled);
    assert!(
        store
            .channel_receive(binding.id, input("3", "not authorized"))
            .is_err()
    );
    assert!(
        store
            .channel_grant(binding.id, true, binding.grant_revision)
            .is_err()
    );
    assert!(
        store
            .channel_grant(binding.id, true, revoked.grant_revision)
            .unwrap()
            .reply_enabled
    );
    assert!(
        store
            .channel_bind(
                BindingSpec {
                    peer: "3333".into(),
                    ..spec()
                },
                true
            )
            .is_err()
    );
}

#[tokio::test]
async fn batching_and_supplements_reconsider_unsent_text_without_blocking() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.sqlite3"));
    let (s, m) = service(&k, 0);
    let channel = fixture_channel(true, SendOutcome::Confirmed("31".into()));
    let task = tokio::spawn(s.clone().run(channel.clone()));
    let first = s.receive(input("1", "今天")).await.unwrap();
    s.receive(input("2", "有点累")).await.unwrap();
    eventually(|| m.calls.load(Ordering::SeqCst) == 1).await;
    s.receive(input("3", "但是项目有进展")).await.unwrap();
    assert!(k.conversations().is_ok());
    assert!(k.list_sessions().unwrap().is_empty());
    assert_eq!(m.calls.load(Ordering::SeqCst), 1);
    m.gate.add_permits(2);
    eventually(|| {
        k.channel_snapshot(s.id()).unwrap().deliveries.len() == 1
            && channel.sent.lock().unwrap().len() == 1
    })
    .await;
    let snapshot = k.channel_snapshot(s.id()).unwrap();
    assert_eq!(snapshot.incoming[0].id, first.id);
    assert_eq!(snapshot.batches[0].text, "今天\n有点累");
    assert_eq!(snapshot.batches.len(), 1);
    assert_eq!(snapshot.incoming[2].batch_id, Some(snapshot.batches[0].id));
    let turn = k
        .actor_turn(snapshot.batches[0].actor_turn_id.unwrap())
        .unwrap();
    assert_eq!(turn.model_requests, 2);
    assert_eq!(turn.usage_tokens, 20);
    assert_eq!(turn.private_chat.as_ref().unwrap().reconsiderations, 1);
    assert_eq!(
        turn.messages
            .iter()
            .filter(|m| m.role == MessageRole::Assistant)
            .count(),
        1
    );
    assert!(
        snapshot
            .batches
            .iter()
            .all(|b| b.conversation_id == s.binding.conversation_id)
    );
    assert_eq!(m.requests.lock().unwrap().len(), 2);
    {
        let requests = m.requests.lock().unwrap();
        assert!(requests[0].persona.contains("equals"));
        assert!(
            serde_json::to_string(&requests[1].input)
                .unwrap()
                .contains("有点累")
        );
    }
    k.shutdown().unwrap();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn offline_draft_revocation_and_explicit_regrant_do_not_replay_old_work() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.sqlite3"));
    let (s, _) = service(&k, 10);
    let channel = fixture_channel(false, SendOutcome::Confirmed("32".into()));
    let task = tokio::spawn(s.clone().run(channel.clone()));
    s.receive(input("1", "你好")).await.unwrap();
    eventually(|| !k.channel_snapshot(s.id()).unwrap().deliveries.is_empty()).await;
    let grant = k.channel_grant(s.id(), false, 1).unwrap();
    channel.connected.send_replace(true);
    assert_eq!(
        k.channel_snapshot(s.id()).unwrap().deliveries[0].state,
        DeliveryState::Revoked
    );
    k.channel_grant(s.id(), true, grant.grant_revision).unwrap();
    s.receive(input("2", "新的交流")).await.unwrap();
    eventually(|| channel.sent.lock().unwrap().len() == 1).await;
    assert_eq!(k.channel_snapshot(s.id()).unwrap().deliveries.len(), 2);
    k.shutdown().unwrap();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn uncertain_transport_is_attempted_once_and_sensitive_input_is_not_saved() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.sqlite3"));
    let (s, _) = service(&k, 10);
    let channel = fixture_channel(true, SendOutcome::Unknown);
    assert!(s.receive(input("secret", TOKEN)).await.is_err());
    assert!(
        s.receive(input("password", "password=do-not-save-this"))
            .await
            .is_err()
    );
    assert!(k.channel_snapshot(s.id()).unwrap().incoming.is_empty());
    let task = tokio::spawn(s.clone().run(channel.clone()));
    s.receive(input("1", "在吗")).await.unwrap();
    eventually(|| {
        k.channel_snapshot(s.id())
            .unwrap()
            .deliveries
            .first()
            .is_some_and(|d| d.state == DeliveryState::Unknown)
    })
    .await;
    channel.connected.send_replace(false);
    channel.connected.send_replace(true);
    s.receive(input("1", "在吗")).await.unwrap();
    s.receive(input("2", "第二条")).await.unwrap();
    eventually(|| {
        k.channel_snapshot(s.id()).unwrap().deliveries.len() == 2
            && channel.sent.lock().unwrap().len() == 2
    })
    .await;
    assert_eq!(k.channel_snapshot(s.id()).unwrap().incoming.len(), 2);
    k.shutdown().unwrap();
    task.await.unwrap().unwrap();
}

fn qq_config(endpoint: String) -> napcat::NapCatConfig {
    napcat::NapCatConfig {
        name: "qq-fixture".into(),
        endpoint,
        self_id: "1001".into(),
        user_id: "2002".into(),
        secret_ref: "secret://env/q1-fixture".parse().unwrap(),
        allow_reply: true,
    }
}
fn event(id: i64, text: &str) -> Value {
    json!({"post_type":"message","message_type":"private","sub_type":"friend","self_id":1001,"user_id":2002,"sender":{"user_id":2002},"message_id":id,"message":[{"type":"text","data":{"text":text}}]})
}
#[test]
fn napcat_filter_is_fail_closed_and_literal_text_cannot_become_actions() {
    let config = qq_config("ws://127.0.0.1:3001".into());
    config.validate().unwrap();
    let mut alternate_id = event(1, "same event");
    alternate_id["message_id"] = json!("0001");
    assert_eq!(
        napcat::parse_private(&config, &alternate_id)
            .unwrap()
            .external_id,
        "1"
    );
    assert_eq!(
        napcat::parse_private(&config, &event(1, "[CQ:at,qq=all]"))
            .unwrap()
            .text,
        "[CQ:at,qq=all]"
    );
    for (key, value) in [
        ("post_type", json!("message_sent")),
        ("message_type", json!("group")),
        ("self_id", json!(999)),
        ("user_id", json!(1001)),
        ("sub_type", json!("group")),
        ("group_id", json!(1)),
        (
            "message",
            json!([{ "type":"image","data":{"file":"secret"}}]),
        ),
    ] {
        let mut invalid = event(2, "test");
        invalid[key] = value;
        assert!(napcat::parse_private(&config, &invalid).is_none());
    }
    assert!(qq_config("ws://example.com/".into()).validate().is_err());
    assert!(
        qq_config("wss://example.com/?access_token=bad".into())
            .validate()
            .is_err()
    );
    assert!(
        qq_config("wss://user:secret@example.com/".into())
            .validate()
            .is_err()
    );
}

#[test]
fn queue_and_batch_limits_expire_without_catchup_or_snapshot_starvation() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("state.sqlite3")).unwrap();
    let binding = store.channel_bind(spec(), true).unwrap();
    let first = store
        .channel_receive(binding.id, input("0", "hello"))
        .unwrap();
    for n in 1..MAX_QUEUED {
        store
            .channel_receive(binding.id, input(&n.to_string(), "hello"))
            .unwrap();
    }
    assert!(
        store
            .channel_receive(binding.id, input("overflow", "hello"))
            .is_err()
    );
    assert_eq!(
        store.channel_snapshot(binding.id).unwrap().incoming.len(),
        100
    );
    // The oldest item is outside the inspection page, but still drives the timer.
    let due = store
        .channel_deadline(binding.id, first.received_at_ms)
        .unwrap()
        .unwrap();
    assert!((first.received_at_ms + MERGE_MS..=first.received_at_ms + MAX_MERGE_MS).contains(&due));
    assert!(
        store
            .channel_next_batch(binding.id, first.received_at_ms)
            .unwrap()
            .is_none()
    );
    let batch = store.channel_next_batch(binding.id, due).unwrap().unwrap();
    assert_eq!(batch.text.split('\n').count(), MAX_BATCH);
    let future = crate::domain::now_ms().unwrap() + REPLY_TTL_MS + 1;
    assert!(
        store
            .channel_next_batch(binding.id, future)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        store.channel_snapshot(binding.id).unwrap().batches[0].state,
        BatchState::Expired
    );
    assert_eq!(store.channel_deadline(binding.id, future).unwrap(), None);
    store
        .channel_receive(binding.id, input("new", "x"))
        .unwrap();
    // Expired input remains deduplicated; it cannot create a new request.
    assert_eq!(
        store
            .channel_receive(binding.id, input("127", "hello"))
            .unwrap()
            .state,
        InputState::Expired
    );
}

#[tokio::test]
async fn restart_resumes_prepared_work_but_never_regenerates_interrupted_or_ready_work() {
    for phase in ["prepared", "running", "ready"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.sqlite3");
        let mut store = Store::open(&path).unwrap();
        let binding = store.channel_bind(spec(), true).unwrap();
        let incoming = store
            .channel_receive(binding.id, input("1", "重启前的消息"))
            .unwrap();
        let batch = store
            .channel_next_batch(binding.id, incoming.received_at_ms + MERGE_MS)
            .unwrap()
            .unwrap();
        if phase != "prepared" {
            let mut turn = crate::actor::domain::ActorTurn {
                id: crate::actor::domain::ActorTurnId::new(),
                conversation_id: binding.conversation_id,
                request_id: batch.id.0,
                sequence: 0,
                revision: 0,
                state: if phase == "running" {
                    TurnState::Running
                } else {
                    TurnState::Completed
                },
                messages: vec![],
                actions: vec![],
                error: None,
                model_requests: 1,
                usage_tokens: 1,
                timing: Default::default(),
                channel_memory: None,
                channel_source: None,
                private_chat: None,
            };
            turn.message(MessageRole::User, batch.text.clone()).unwrap();
            if phase == "ready" {
                turn.message(MessageRole::Assistant, "已保存的回答".into())
                    .unwrap();
            }
            store.save_actor_turn(&mut turn, true).unwrap();
            if phase == "ready" {
                store.channel_finish(batch.id, &turn).unwrap();
            }
        }
        drop(store);
        let k = kernel(&path);
        let (s, model) = service(&k, 1);
        assert_eq!(s.binding.conversation_id, binding.conversation_id);
        let channel = fixture_channel(true, SendOutcome::Confirmed("receipt".into()));
        let task = tokio::spawn(s.clone().run(channel.clone()));
        eventually(|| channel.sent.lock().unwrap().len() == 1).await;
        assert_eq!(
            model.calls.load(Ordering::SeqCst),
            usize::from(phase == "prepared")
        );
        let snapshot = k.channel_snapshot(s.id()).unwrap();
        assert_eq!(snapshot.batches.len(), 1);
        assert_eq!(snapshot.batches[0].id, batch.id);
        assert_eq!(k.actor_recent(binding.conversation_id).unwrap().len(), 1);
        if phase == "running" {
            assert_eq!(
                k.actor_recent(binding.conversation_id).unwrap()[0].state,
                TurnState::Unknown
            );
            assert!(snapshot.deliveries[0].text.contains("没有可靠完成"));
        } else if phase == "ready" {
            assert_eq!(snapshot.deliveries[0].text, "已保存的回答");
        }
        k.shutdown().unwrap();
        task.await.unwrap().unwrap();
    }
}

#[tokio::test]
async fn revocation_during_generation_prevents_delivery_and_stale_submission() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.sqlite3"));
    let (s, m) = service(&k, 0);
    let channel = fixture_channel(true, SendOutcome::Confirmed("1".into()));
    let task = tokio::spawn(s.clone().run(channel.clone()));
    s.receive(input("1", "正在想的事情")).await.unwrap();
    eventually(|| m.calls.load(Ordering::SeqCst) == 1).await;
    let batch = k.channel_snapshot(s.id()).unwrap().batches[0].clone();
    let mut expired = batch.clone();
    expired.id = BatchId::new();
    expired.expires_at_ms = 0;
    let error = s.actor.submit_channel(expired).await.unwrap_err();
    assert!(matches!(
        error.downcast_ref::<crate::backend::ControlError>(),
        Some(crate::backend::ControlError::InvalidInput)
    ));
    let grant = k.channel_grant(s.id(), false, 1).unwrap();
    m.gate.add_permits(1);
    eventually(|| {
        k.actor_recent(s.binding.conversation_id).unwrap()[0].state == TurnState::Completed
    })
    .await;
    assert_eq!(
        k.channel_snapshot(s.id()).unwrap().batches[0].state,
        BatchState::Revoked
    );
    k.channel_grant(s.id(), true, grant.grant_revision).unwrap();
    let mut stale = batch;
    stale.id = BatchId::new(); // No previously committed request to recover.
    assert!(s.actor.submit_channel(stale).await.is_err());
    s.receive(input("2", "新的问题")).await.unwrap();
    m.gate.add_permits(1);
    eventually(|| channel.sent.lock().unwrap().len() == 1).await;
    assert_eq!(k.channel_snapshot(s.id()).unwrap().deliveries.len(), 1);
    k.shutdown().unwrap();
    task.await.unwrap().unwrap();
}

struct MutationModel;
impl ActorModel for MutationModel {
    fn respond(&self, _: Request) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
        Box::pin(async {
            let arguments = json!({"draft":{"content":"偏好简洁报告","kind":"preference","scope":{"kind":"global"}}}).to_string();
            Ok(Reply {
                text: String::new(),
                calls: vec![crate::actor::model::Call {
                    id: "call_fixture".into(),
                    name: "memory_propose".into(),
                    arguments: arguments.clone(),
                }],
                usage_tokens: 10,
                continuation: vec![
                    json!({"type":"function_call","name":"memory_propose","call_id":"call_fixture","arguments":arguments}),
                ],
            })
        })
    }
}

#[tokio::test]
async fn chat_agreement_cannot_confirm_memory_mutation() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.sqlite3"));
    let actor = Actor::with_model(
        k.clone(),
        None,
        TOKEN.parse().unwrap(),
        Arc::new(MutationModel),
    );
    let binding = k.channel_bind(spec(), true).unwrap();
    let s = ChannelService::new(k.clone(), actor.clone(), binding, &TOKEN.parse().unwrap());
    let channel = fixture_channel(true, SendOutcome::Confirmed("1".into()));
    let task = tokio::spawn(s.clone().run(channel.clone()));
    s.receive(input("1", "记住我的偏好")).await.unwrap();
    eventually(|| channel.sent.lock().unwrap().len() == 1).await;
    let turn = k.actor_recent(s.binding.conversation_id).unwrap().remove(0);
    assert_eq!(turn.state, TurnState::PendingConfirmation);
    assert_eq!(
        turn.actions[0].state,
        crate::actor::domain::ActionState::Proposed
    );
    assert!(channel.sent.lock().unwrap()[0].text.contains("本地终端"));
    s.receive(input("2", "同意")).await.unwrap();
    assert!(k.channel_next_batch(s.id()).unwrap().is_none());
    assert_eq!(
        k.actor_turn(turn.id).unwrap().state,
        TurnState::PendingConfirmation
    );
    let candidate_id = crate::memory::CandidateId(turn.actions[0].id.0);
    let candidate = k.memory_candidate(candidate_id).unwrap();
    assert_eq!(candidate.status, crate::memory::CandidateStatus::Candidate);
    assert!(candidate.memory_id.is_none());
    k.channel_grant(s.id(), false, 1).unwrap();
    let cancelled = actor.cancel(turn.id).await.unwrap();
    assert_eq!(
        cancelled.actions[0].state,
        crate::actor::domain::ActionState::Rejected
    );
    assert!(
        k.memory_candidate(candidate_id)
            .unwrap()
            .memory_id
            .is_none()
    );
    assert!(
        k.memory_page(crate::memory::MemoryQuery {
            scope: crate::memory::MemoryScope::Global,
            kind: None,
            status: None,
            text: None,
            after: 0,
            limit: 50,
        })
        .unwrap()
        .records
        .is_empty()
    );
    k.shutdown().unwrap();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn grant_api_requires_kernel_auth_and_compare_and_swap() {
    use axum::{
        body::Body,
        http::{Request as HttpRequest, StatusCode},
    };
    use tower::ServiceExt;
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.sqlite3"));
    let (s, _) = service(&k, 0);
    let app = crate::api::actor_router(k.clone(), TOKEN.parse().unwrap(), None, s.actor.clone());
    let uri = format!("/v1/channels/{}/reply-grant", s.id());
    for (token, revision, status) in [
        ("", 1, StatusCode::UNAUTHORIZED),
        ("fixture-napcat-token", 1, StatusCode::UNAUTHORIZED),
        (TOKEN, 1, StatusCode::OK),
        (TOKEN, 1, StatusCode::CONFLICT),
    ] {
        let request = HttpRequest::post(&uri)
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(
                json!({"enabled":false,"expected_revision":revision}).to_string(),
            ))
            .unwrap();
        assert_eq!(app.clone().oneshot(request).await.unwrap().status(), status);
    }
    assert!(!k.channel_binding(s.id()).unwrap().reply_enabled);
    assert_eq!(k.channel_binding(s.id()).unwrap().grant_revision, 2);
    k.shutdown().unwrap();
}

async fn ws_json(socket: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>) -> Value {
    use futures_util::{SinkExt, StreamExt};
    loop {
        match tokio::time::timeout(Duration::from_secs(15), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
        {
            tokio_tungstenite::tungstenite::Message::Text(text) => {
                return serde_json::from_str(&text).unwrap();
            }
            tokio_tungstenite::tungstenite::Message::Ping(bytes) => socket
                .send(tokio_tungstenite::tungstenite::Message::Pong(bytes))
                .await
                .unwrap(),
            other => panic!("unexpected fixture frame {other:?}"),
        }
    }
}
async fn ws_send(
    socket: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    value: Value,
) {
    use futures_util::SinkExt;
    socket
        .send(tokio_tungstenite::tungstenite::Message::Text(
            value.to_string().into(),
        ))
        .await
        .unwrap();
}
#[tokio::test]
#[allow(clippy::result_large_err)] // tungstenite's server callback fixes the error type.
async fn napcat_wire_identity_reconnect_receive_during_model_and_lost_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.sqlite3"));
    let model = Arc::new(Model {
        requests: Mutex::default(),
        calls: AtomicUsize::new(0),
        gate: Semaphore::new(0),
    });
    let actor = Actor::with_model(k.clone(), None, TOKEN.parse().unwrap(), model.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = qq_config(format!("ws://{}/", listener.local_addr().unwrap()));
    let binding = k.channel_bind(config.binding_spec(), true).unwrap();
    let s = ChannelService::new(k.clone(), actor, binding, &TOKEN.parse().unwrap());
    let observed = Arc::new(AtomicUsize::new(0));
    let observed_server = observed.clone();
    let model_server = model.clone();
    let server = tokio::spawn(async move {
        for attempt in 0..3 {
            let (tcp, _) = tokio::time::timeout(Duration::from_secs(15), listener.accept())
                .await
                .unwrap()
                .unwrap();
            let mut ws = tokio_tungstenite::accept_hdr_async(
                tcp,
                |request: &tokio_tungstenite::tungstenite::handshake::server::Request, response| {
                    assert_eq!(
                        request.headers().get("authorization").unwrap(),
                        "Bearer fixture-napcat-token"
                    );
                    Ok(response)
                },
            )
            .await
            .unwrap();
            let login = ws_json(&mut ws).await;
            assert_eq!(login["action"], "get_login_info");
            if attempt == 0 {
                ws_send(
                    &mut ws,
                    event(999, "must not ingest before mismatched identity"),
                )
                .await;
                ws_send(
                    &mut ws,
                    json!({"status":"ok","retcode":0,"data":{"user_id":9999},"echo":login["echo"]}),
                )
                .await;
                continue;
            }
            ws_send(
                &mut ws,
                json!({"status":"ok","retcode":0,"data":{"user_id":1001},"echo":login["echo"]}),
            )
            .await;
            if attempt == 1 {
                for (key, value) in [
                    ("user_id", json!(9999)),
                    ("post_type", json!("message_sent")),
                    ("message_type", json!("group")),
                    ("self_id", json!(9999)),
                ] {
                    let mut invalid = event(77, "ignore");
                    invalid[key] = value;
                    ws_send(&mut ws, invalid).await;
                }
                ws_send(&mut ws, event(1, "今天")).await;
                ws_send(&mut ws, event(1, "今天")).await;
                ws_send(&mut ws, event(2, "不错")).await;
                eventually(|| model_server.calls.load(Ordering::SeqCst) == 1).await;
                ws_send(&mut ws, event(3, "还有一件事")).await;
                // Keep generation in flight until the caller proves the new input is stored.
                let first = ws_json(&mut ws).await;
                assert_eq!(first["action"], "send_private_msg");
                assert_eq!(first["params"]["user_id"].as_u64(), Some(2002));
                assert_eq!(first["params"]["message"][0]["type"], "text");
                observed_server.fetch_add(1, Ordering::SeqCst);
                ws_send(&mut ws,json!({"status":"ok","retcode":0,"data":{"message_id":11},"echo":first["echo"]})).await;
                // The in-flight supplement now updates the first draft, so a new
                // input after its confirmed reply exercises a separate uncertain send.
                ws_send(&mut ws, event(4, "接着聊")).await;
                let second = ws_json(&mut ws).await;
                assert_eq!(second["action"], "send_private_msg");
                observed_server.fetch_add(1, Ordering::SeqCst);
                // Platform accepted this request; deliberately drop before its response.
                drop(ws);
            } else {
                ws_send(&mut ws, event(3, "还有一件事")).await;
                ws_send(&mut ws, event(5, "重连后的新消息")).await;
                let third = ws_json(&mut ws).await;
                assert_eq!(third["action"], "send_private_msg");
                observed_server.fetch_add(1, Ordering::SeqCst);
                ws_send(&mut ws,json!({"status":"ok","retcode":0,"data":{"message_id":13},"echo":third["echo"]})).await;
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    });
    let (channel, transport) = napcat::start_authenticated(
        config,
        s.clone(),
        k.subscribe_shutdown(),
        crate::profiles::SecretValue(zeroize::Zeroizing::new("fixture-napcat-token".into())),
    );
    let app = tokio::spawn(s.clone().run(channel));
    eventually(|| k.channel_snapshot(s.id()).unwrap().incoming.len() == 3).await;
    assert_eq!(model.calls.load(Ordering::SeqCst), 1);
    model.gate.add_permits(4);
    eventually(|| {
        k.channel_snapshot(s.id()).unwrap().deliveries.len() == 3
            && k.channel_snapshot(s.id()).unwrap().deliveries[2].state == DeliveryState::Confirmed
    })
    .await;
    let snapshot = k.channel_snapshot(s.id()).unwrap();
    assert_eq!(observed.load(Ordering::SeqCst), 3);
    assert_eq!(snapshot.incoming.len(), 5);
    assert_eq!(snapshot.batches[0].text, "今天\n不错");
    assert_eq!(snapshot.deliveries[0].state, DeliveryState::Confirmed);
    assert_eq!(snapshot.deliveries[1].state, DeliveryState::Unknown);
    assert_eq!(snapshot.deliveries[2].receipt.as_deref(), Some("13"));
    assert_eq!(model.calls.load(Ordering::SeqCst), 4);
    k.shutdown().unwrap();
    app.await.unwrap().unwrap();
    transport.await.unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn napcat_failed_malformed_and_missing_receipts_never_retry() {
    for (case, expected) in [
        (0, DeliveryState::Rejected),
        (1, DeliveryState::Unknown),
        (2, DeliveryState::Unknown),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let k = kernel(&dir.path().join("state.sqlite3"));
        let (s, model) = service(&k, 1);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let config = qq_config(format!("ws://{}/", listener.local_addr().unwrap()));
        let (release, stop) = tokio::sync::oneshot::channel::<()>();
        let observed = Arc::new(AtomicUsize::new(0));
        let count = observed.clone();
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(tcp).await.unwrap();
            let login = ws_json(&mut ws).await;
            ws_send(
                &mut ws,
                json!({"status":"ok","retcode":0,"data":{"user_id":1001},"echo":login["echo"]}),
            )
            .await;
            ws_send(&mut ws, event(1, "回执检查")).await;
            let sent = ws_json(&mut ws).await;
            assert_eq!(sent["action"], "send_private_msg");
            count.fetch_add(1, Ordering::SeqCst);
            match case {
                0 => ws_send(&mut ws, json!({"status":"failed","retcode":1200,"echo":sent["echo"]})).await,
                1 => ws_send(&mut ws, json!({"status":"ok","retcode":0,"data":{},"echo":sent["echo"]})).await,
                _ => ws_send(&mut ws, json!({"status":"ok","retcode":0,"data":{"message_id":5},"echo":"unrelated-response"})).await,
            }
            // Hold the connection open. Missing receipts must time out independently.
            let _ = stop.await;
        });
        let (channel, transport) = napcat::start_authenticated(
            config,
            s.clone(),
            k.subscribe_shutdown(),
            crate::profiles::SecretValue(zeroize::Zeroizing::new("fixture-token".into())),
        );
        let app = tokio::spawn(s.clone().run(channel));
        tokio::time::timeout(Duration::from_secs(25), async {
            loop {
                if k.channel_snapshot(s.id())
                    .unwrap()
                    .deliveries
                    .first()
                    .is_some_and(|d| d.state == expected)
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        s.receive(input("1", "回执检查")).await.unwrap();
        assert!(k.channel_claim_delivery(s.id()).unwrap().is_none());
        assert_eq!(observed.load(Ordering::SeqCst), 1);
        assert_eq!(model.calls.load(Ordering::SeqCst), 1);
        k.shutdown().unwrap();
        let _ = release.send(());
        app.await.unwrap().unwrap();
        transport.await.unwrap();
        server.await.unwrap();
    }
}
