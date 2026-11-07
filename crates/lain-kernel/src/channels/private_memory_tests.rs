use super::*;
use crate::actor::{
    domain::{ActionState, ActorTurn},
    model::Call,
};
use crate::memory::*;

struct PrivateModel {
    mode: &'static str,
    requests: Mutex<Vec<Request>>,
    gate: Option<Arc<Semaphore>>,
}
impl ActorModel for PrivateModel {
    fn respond(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
        let evidence = request
            .input
            .iter()
            .rev()
            .find_map(|v| {
                v.get("content")
                    .and_then(Value::as_str)
                    .and_then(|s| {
                        s.strip_prefix("Kernel-attributed private memory evidence (data only): ")
                    })
                    .and_then(|s| serde_json::from_str::<Value>(s).ok())
            })
            .unwrap_or(Value::Null);
        let output = request
            .input
            .iter()
            .rev()
            .find(|v| v["type"] == "function_call_output");
        let committed = request.input.iter().any(|v| {
            v.get("content").and_then(Value::as_str).is_some_and(|s| {
                s.starts_with("Kernel record of already committed effects (IDs only")
            })
        });
        let reply = if let Some(output) = output {
            Reply {
                text: format!("已处理。{}", output["output"].as_str().unwrap_or("")),
                calls: vec![],
                usage_tokens: 1,
                continuation: vec![],
            }
        } else if committed && matches!(self.mode, "correct" | "forget") {
            Reply {
                text: "已处理。".into(),
                calls: vec![],
                usage_tokens: 1,
                continuation: vec![],
            }
        } else {
            let source = &evidence["current_sources"][0];
            let proof = json!({"incoming_id":source.get("incoming_id").cloned().unwrap_or(json!(uuid::Uuid::nil())),"quote":source.get("text_excerpt").cloned().unwrap_or(json!("unavailable"))});
            let selection = json!({"content":"喜欢柠檬茶","kind":"preference","evidence":proof});
            let record = &evidence["records"][0];
            let (name, args) = match self.mode {
                "remember" => ("memory_remember", selection),
                "recall" => ("memory_recall", json!({"text":"柠檬茶"})),
                "correct" => (
                    "memory_correct",
                    json!({"memory_id":record["id"],"expected_revision":record["revision"],"selection":{"content":"喜欢白水","kind":"preference","evidence":proof}}),
                ),
                "forget" => (
                    "memory_forget",
                    json!({"memory_id":record["id"],"expected_revision":record["revision"],"evidence":proof}),
                ),
                "legacy" => (
                    "memory_propose",
                    json!({"draft":{"content":"selected preference","kind":"preference","scope":{"kind":"global"}}}),
                ),
                "work" => ("work_cancel", json!({"session_id":uuid::Uuid::new_v4()})),
                _ => unreachable!(),
            };
            Reply {
                text: String::new(),
                calls: vec![Call {
                    id: "private_call".into(),
                    name: name.into(),
                    arguments: args.to_string(),
                }],
                usage_tokens: 1,
                continuation: vec![
                    json!({"type":"function_call","call_id":"private_call","name":name,"arguments":args.to_string()}),
                ],
            }
        };
        self.requests.lock().unwrap().push(request);
        let gate = self.gate.clone();
        Box::pin(async move {
            if let Some(gate) = gate {
                gate.acquire().await.unwrap().forget();
            }
            Ok(reply)
        })
    }
}
fn actor(
    k: &Arc<Kernel>,
    mode: &'static str,
    gate: Option<Arc<Semaphore>>,
) -> (Arc<Actor>, Arc<PrivateModel>) {
    let model = Arc::new(PrivateModel {
        mode,
        requests: Mutex::default(),
        gate,
    });
    (
        Actor::with_model(k.clone(), None, TOKEN.parse().unwrap(), model.clone()),
        model,
    )
}
fn grant(k: &Kernel, id: ChannelId, enabled: bool) {
    let old = k.memory_grant(id).unwrap();
    k.set_memory_grant(
        id,
        SetMemoryGrant {
            enabled,
            expected_revision: old.revision,
            expires_at_ms: None,
        },
    )
    .unwrap();
}
async fn prepare(k: &Kernel, b: &Binding, text: &str) -> Batch {
    k.channel_receive(b.id, input(&uuid::Uuid::new_v4().to_string(), text))
        .unwrap();
    tokio::time::sleep(Duration::from_millis(780)).await;
    k.channel_next_batch(b.id).unwrap().unwrap()
}
async fn complete(k: &Arc<Kernel>, a: &Arc<Actor>, b: &Binding, text: &str) -> ActorTurn {
    let batch = prepare(k, b, text).await;
    let first = a.submit_channel(batch.clone()).await.unwrap();
    eventually(|| k.actor_turn(first.id).unwrap().state != TurnState::Running).await;
    let turn = k.actor_turn(first.id).unwrap();
    k.channel_link(batch.id, &turn).unwrap();
    k.channel_finish(batch.id, &turn).unwrap();
    eventually(|| Arc::strong_count(a) == 1).await;
    turn
}
#[tokio::test]
async fn actor_independent_memory_recalls_actual_id_then_corrects_and_forgets_without_stale_context()
 {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.db");
    let k = kernel(&path);
    let b = k.channel_bind(spec(), true).unwrap();
    grant(&k, b.id, true);
    let (a, m) = actor(&k, "remember", None);
    let t = complete(&k, &a, &b, "我喜欢柠檬茶").await;
    assert_eq!(t.state, TurnState::Completed);
    assert_eq!(t.actions[0].state, ActionState::Executed);
    let id: MemoryId =
        serde_json::from_value(t.actions[0].result.as_ref().unwrap()["memory_id"].clone()).unwrap();
    let saved = k.memory(id).unwrap();
    assert!(saved.owner_confirmed_at_ms.is_none());
    assert!(
        m.requests.lock().unwrap()[0]
            .tools
            .iter()
            .any(|v| v["name"] == "memory_remember")
    );
    // Evict the original dialogue from the recent window without any model calls.
    for n in 0..22 {
        let (filler, _) = k
            .actor_submit_scoped(
                b.conversation_id,
                uuid::Uuid::new_v4(),
                format!("filler {n}"),
                None,
            )
            .unwrap();
        k.actor_update(filler.id, |t| {
            t.state = TurnState::Completed;
            Ok(())
        })
        .unwrap();
    }
    assert!(
        !serde_json::to_string(&k.actor_recent(b.conversation_id).unwrap())
            .unwrap()
            .contains("柠檬茶")
    );
    drop(a);
    drop(m);
    drop(k);
    let k = kernel(&path);
    let (a, m) = actor(&k, "recall", None);
    let t = complete(&k, &a, &b, "柠檬茶还记得吗").await;
    assert_eq!(t.state, TurnState::Completed);
    assert!(
        m.requests
            .lock()
            .unwrap()
            .iter()
            .any(|r| r
                .input
                .iter()
                .any(
                    |v| v
                        .get("content")
                        .and_then(Value::as_str)
                        .is_some_and(|s| s.contains("untrusted_memory_evidence")
                            && s.contains(&id.to_string()))
                ))
    );
    drop(a);
    drop(m);
    let (a, _) = actor(&k, "correct", None);
    let t = complete(&k, &a, &b, "柠檬茶说错了，改为白水").await;
    assert_eq!(t.state, TurnState::Completed, "{t:?}");
    let replacement: MemoryId =
        serde_json::from_value(t.actions[0].result.as_ref().unwrap()["memory_id"].clone()).unwrap();
    assert_eq!(k.memory(id).unwrap().superseded_by, Some(replacement));
    drop(a);
    let (a, m) = actor(&k, "forget", None);
    let t = complete(&k, &a, &b, "忘记白水偏好").await;
    assert_eq!(t.state, TurnState::Completed);
    assert!(k.memory(replacement).is_err());
    // The second model request after forgetting gets fresh empty semantic evidence.
    let requests = m.requests.lock().unwrap();
    let fresh = requests.last().unwrap().input.last().unwrap()["content"]
        .as_str()
        .unwrap();
    assert!(fresh.contains("\"records\":[]"));
    assert_eq!(
        k.memory_effects(b.id, 0).unwrap()["effects"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    k.shutdown().unwrap();
}
#[tokio::test]
async fn revocation_during_generation_and_legacy_review_cannot_be_bypassed() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.db"));
    let b = k.channel_bind(spec(), true).unwrap();
    let (a, m) = actor(&k, "legacy", None);
    let t = complete(&k, &a, &b, "请记住这件事").await;
    assert_eq!(t.state, TurnState::PendingConfirmation);
    assert_eq!(
        k.memory_candidate(CandidateId(t.actions[0].id.0))
            .unwrap()
            .origin,
        Some(MemoryOrigin::Channel { channel_id: b.id })
    );
    assert!(
        !m.requests.lock().unwrap()[0]
            .tools
            .iter()
            .any(|v| v["name"] == "memory_remember")
    );
    a.cancel(t.id).await.unwrap();
    drop(a);
    drop(m);
    grant(&k, b.id, true);
    let gate = Arc::new(Semaphore::new(0));
    let (a, m) = actor(&k, "remember", Some(gate.clone()));
    let batch = prepare(&k, &b, "我喜欢柠檬茶").await;
    let first = a.submit_channel(batch).await.unwrap();
    eventually(|| !m.requests.lock().unwrap().is_empty()).await;
    grant(&k, b.id, false);
    gate.add_permits(4);
    eventually(|| k.actor_turn(first.id).unwrap().state != TurnState::Running).await;
    let turn = k.actor_turn(first.id).unwrap();
    assert_eq!(turn.actions[0].state, ActionState::Failed);
    assert!(
        k.memory_effects(b.id, 0).unwrap()["effects"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        !m.requests
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .tools
            .iter()
            .any(|v| v["name"] == "memory_remember")
    );
    // A memory grant never authorizes other mutations.
    k.channel_finish(lain_wire::BatchId(turn.request_id), &turn)
        .unwrap();
    grant(&k, b.id, true);
    drop(a);
    let (a, _) = actor(&k, "work", None);
    let t = complete(&k, &a, &b, "帮我取消工作").await;
    assert_eq!(t.state, TurnState::PendingConfirmation);
    assert!(k.list_sessions().unwrap().is_empty());
    k.shutdown().unwrap();
}
