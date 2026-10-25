//! Scripted replies prove protocol/state behavior, not natural-language quality.
use super::*;
use crate::actor::{domain::ActionState, model::Call};
use lain_wire::chat::ChatDecision;
use std::collections::VecDeque;

struct Scripted {
    replies: Mutex<VecDeque<Reply>>,
    requests: Mutex<Vec<Request>>,
    release: Semaphore,
}
impl ActorModel for Scripted {
    fn respond_stream(
        &self,
        request: Request,
        preview: crate::actor::preview::PreviewSink,
    ) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
        let response = self.respond(request);
        preview.first_byte();
        preview.delta("fixture in-flight draft must stay private");
        Box::pin(async move {
            let reply = response.await?;
            preview.provider_completed();
            Ok(reply)
        })
    }
    fn respond(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
        self.requests.lock().unwrap().push(request);
        let reply = self
            .replies
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected model invocation");
        Box::pin(async move {
            self.release.acquire().await.unwrap().forget();
            Ok(reply)
        })
    }
}
fn text(value: &str) -> Reply {
    Reply {
        text: value.into(),
        calls: vec![],
        continuation: vec![],
        usage_tokens: 3,
    }
}
fn call(name: &str, args: Value) -> Reply {
    Reply {
        text: "intermediate tool commentary must not become a delivered reply".into(),
        calls: vec![Call {
            id: name.into(),
            name: name.into(),
            arguments: args.to_string(),
        }],
        continuation: vec![
            json!({"type":"function_call","call_id":name,"name":name,"arguments":args.to_string()}),
        ],
        usage_tokens: 2,
    }
}
fn scripted(
    k: &Arc<Kernel>,
    replies: Vec<Reply>,
    permits: usize,
) -> (Arc<ChannelService>, Arc<Scripted>) {
    let model = Arc::new(Scripted {
        replies: Mutex::new(replies.into()),
        requests: Mutex::default(),
        release: Semaphore::new(permits),
    });
    let actor = Actor::with_model(k.clone(), None, TOKEN.parse().unwrap(), model.clone());
    let b = k.channel_bind(spec(), true).unwrap();
    (
        ChannelService::new(k.clone(), actor, b, &TOKEN.parse().unwrap()),
        model,
    )
}
fn turn(k: &Kernel, s: &ChannelService) -> crate::actor::domain::ActorTurn {
    let batch = k
        .channel_snapshot(s.id())
        .unwrap()
        .batches
        .last()
        .unwrap()
        .clone();
    // The Actor may start before ChannelService commits its reverse batch link.
    k.actor_recent(s.binding.conversation_id)
        .unwrap()
        .into_iter()
        .find(|t| t.request_id == batch.id.0)
        .unwrap()
}

#[tokio::test]
async fn silent_social_close_and_followup_question_are_separate_bounded_turns() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.db"));
    let (s, m) = scripted(
        &k,
        vec![
            call(
                "chat_finish",
                json!({"input_kind":"social","reason":"conversation_closed"}),
            ),
            text("new answer"),
        ],
        2,
    );
    let c = fixture_channel(true, SendOutcome::Confirmed("1".into()));
    let task = tokio::spawn(s.clone().run(c.clone()));
    s.receive(input("1", "晚安")).await.unwrap();
    eventually(|| {
        k.channel_snapshot(s.id())
            .unwrap()
            .batches
            .first()
            .is_some_and(|b| b.state == BatchState::Finished)
    })
    .await;
    assert_eq!(
        turn(&k, &s).private_chat.unwrap().decision,
        ChatDecision::NoReply
    );
    assert!(c.sent.lock().unwrap().is_empty());
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(m.requests.lock().unwrap().len(), 1);
    s.receive(input("2", "还有个问题，怎么配置？"))
        .await
        .unwrap();
    eventually(|| c.sent.lock().unwrap().len() == 1).await;
    assert_eq!(c.sent.lock().unwrap()[0].text, "new answer");
    assert_eq!(m.requests.lock().unwrap().len(), 2);
    assert_eq!(k.channel_snapshot(s.id()).unwrap().batches.len(), 2);
    k.shutdown().unwrap();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn mistaken_silent_finish_is_denied_and_the_request_gets_a_reply() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.db"));
    let (s, m) = scripted(
        &k,
        vec![
            call(
                "chat_finish",
                json!({"input_kind":"social","reason":"brief_acknowledgement"}),
            ),
            text("answered request"),
        ],
        2,
    );
    let c = fixture_channel(true, SendOutcome::Confirmed("1".into()));
    let task = tokio::spawn(s.clone().run(c.clone()));
    s.receive(input("1", "帮我检查任务失败的原因"))
        .await
        .unwrap();
    eventually(|| c.sent.lock().unwrap().len() == 1).await;
    let t = turn(&k, &s);
    assert_eq!(t.actions[0].state, ActionState::Failed);
    assert_eq!(t.private_chat.unwrap().decision, ChatDecision::Reply);
    {
        let requests = m.requests.lock().unwrap();
        assert!(requests[1].tools.iter().all(|t| t["name"] != "chat_finish"));
        assert!(requests[1].persona.contains("Kernel denied"));
    }
    assert_eq!(c.sent.lock().unwrap()[0].text, "answered request");
    k.shutdown().unwrap();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn wait_wakes_by_deadline_or_supplement_without_extra_idle_calls() {
    for supplement in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let k = kernel(&dir.path().join("state.db"));
        let (s, m) = scripted(
            &k,
            vec![
                call("chat_wait", json!({"seconds":if supplement {5} else {1}})),
                text("after wait"),
            ],
            2,
        );
        let c = fixture_channel(true, SendOutcome::Confirmed("1".into()));
        let task = tokio::spawn(s.clone().run(c.clone()));
        s.receive(input("1", "我还有一句")).await.unwrap();
        eventually(|| {
            let snapshot = k.channel_snapshot(s.id()).unwrap();
            snapshot
                .batches
                .first()
                .and_then(|b| b.actor_turn_id)
                .is_some_and(|id| {
                    k.actor_turn(id)
                        .unwrap()
                        .private_chat
                        .as_ref()
                        .is_some_and(|c| c.decision == ChatDecision::Waiting)
                })
        })
        .await;
        assert_eq!(m.requests.lock().unwrap().len(), 1);
        if supplement {
            s.receive(input("2", "补充一下，选第二个")).await.unwrap();
        }
        eventually(|| c.sent.lock().unwrap().len() == 1).await;
        let t = turn(&k, &s);
        assert_eq!(t.model_requests, 2);
        assert_eq!(t.private_chat.as_ref().unwrap().waits, 1);
        assert_eq!(
            t.actions[0].result.as_ref().unwrap()["wake"],
            if supplement { "new_input" } else { "deadline" }
        );
        {
            let requests = m.requests.lock().unwrap();
            assert!(requests[1].tools.iter().all(|t| t["name"] != "chat_wait"));
            if supplement {
                let encoded = serde_json::to_string(&requests[1].input).unwrap();
                assert!(encoded.contains("选第二个"));
                assert!(!encoded.contains("intermediate tool commentary"));
                assert!(!encoded.contains("function_call"));
            }
        }
        assert!(t.private_chat.unwrap().wait_until_ms.is_none());
        k.shutdown().unwrap();
        task.await.unwrap().unwrap();
    }
}

#[tokio::test]
async fn new_input_discards_an_unsent_work_proposal_and_keeps_original_request() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.db"));
    let (s, m) = scripted(
        &k,
        vec![
            call("work_cancel", json!({"session_id":uuid::Uuid::new_v4()})),
            text("reconsidered response"),
        ],
        0,
    );
    let c = fixture_channel(true, SendOutcome::Confirmed("1".into()));
    let task = tokio::spawn(s.clone().run(c.clone()));
    s.receive(input("1", "先考虑取消工作")).await.unwrap();
    eventually(|| m.requests.lock().unwrap().len() == 1).await;
    let in_flight = turn(&k, &s);
    let checkpoint = s.actor.preview_checkpoint(in_flight.id).unwrap();
    assert!(
        !serde_json::to_string(&checkpoint)
            .unwrap()
            .contains("fixture in-flight draft")
    );
    s.receive(input("2", "等等，不取消了，继续聊"))
        .await
        .unwrap();
    m.release.add_permits(2);
    eventually(|| c.sent.lock().unwrap().len() == 1).await;
    let t = turn(&k, &s);
    assert!(t.actions.is_empty());
    assert_eq!(t.model_requests, 2);
    assert_eq!(t.usage_tokens, 5);
    assert_eq!(t.messages[0].content, "先考虑取消工作");
    {
        let requests = m.requests.lock().unwrap();
        let context = serde_json::to_string(&requests[1].input).unwrap();
        assert!(context.contains("不取消了"));
        assert!(!context.contains("work_cancel"));
        assert!(requests[1].persona.contains("equals"));
        assert!(requests[1].persona.contains("Kernel private turn limits"));
    }
    assert_eq!(c.sent.lock().unwrap()[0].text, "reconsidered response");
    k.shutdown().unwrap();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn confirmed_approval_proposal_survives_followup_without_automatic_execution() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.db"));
    let (s, m) = scripted(
        &k,
        vec![call(
            "work_cancel",
            json!({"session_id":uuid::Uuid::new_v4()}),
        )],
        1,
    );
    let c = fixture_channel(true, SendOutcome::Confirmed("1".into()));
    let task = tokio::spawn(s.clone().run(c.clone()));
    s.receive(input("1", "取消工作")).await.unwrap();
    eventually(|| c.sent.lock().unwrap().len() == 1).await;
    let initial = turn(&k, &s);
    s.receive(input("2", "同意，然后聊点别的")).await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let saved = k.actor_turn(initial.id).unwrap();
    assert_eq!(saved.state, TurnState::PendingConfirmation);
    assert_eq!(saved.actions[0].state, ActionState::Proposed);
    assert_eq!(saved.revision, initial.revision);
    assert_eq!(m.requests.lock().unwrap().len(), 1);
    assert_eq!(
        k.channel_snapshot(s.id()).unwrap().incoming[1].state,
        InputState::Queued
    );
    k.shutdown().unwrap();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn memory_revoke_or_forget_during_generation_discards_the_old_recollection() {
    use crate::memory::*;
    for forget in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let k = kernel(&dir.path().join("state.db"));
        let (s, m) = scripted(&k, vec![text("stale lemon fact must not be delivered")], 0);
        k.set_memory_grant(
            s.id(),
            SetMemoryGrant {
                enabled: true,
                expected_revision: 0,
                expires_at_ms: None,
            },
        )
        .unwrap();
        let record = k
            .add_memory(AddMemory {
                request_id: lain_wire::RequestId::new(),
                draft: MemoryDraft {
                    content: "lemon preference".into(),
                    kind: MemoryKind::Preference,
                    scope: MemoryScope::Channel { channel_id: s.id() },
                },
            })
            .unwrap();
        let c = fixture_channel(true, SendOutcome::Confirmed("1".into()));
        let task = tokio::spawn(s.clone().run(c.clone()));
        s.receive(input("1", "lemon")).await.unwrap();
        eventually(|| m.requests.lock().unwrap().len() == 1).await;
        assert!(
            serde_json::to_string(&m.requests.lock().unwrap()[0].input)
                .unwrap()
                .contains("lemon preference")
        );
        if forget {
            k.delete_memory(record.id, record.revision).unwrap();
        } else {
            k.set_memory_grant(
                s.id(),
                SetMemoryGrant {
                    enabled: false,
                    expected_revision: 1,
                    expires_at_ms: None,
                },
            )
            .unwrap();
        }
        m.release.add_permits(1);
        eventually(|| c.sent.lock().unwrap().len() == 1).await;
        let t = turn(&k, &s);
        assert_eq!(t.private_chat.unwrap().decision, ChatDecision::Notice);
        assert_eq!(t.usage_tokens, 3);
        assert!(!c.sent.lock().unwrap()[0].text.contains("stale lemon"));
        assert_eq!(m.requests.lock().unwrap().len(), 1);
        k.shutdown().unwrap();
        task.await.unwrap().unwrap();
    }
}

#[tokio::test]
async fn repeated_interruptions_keep_cumulative_budgets_and_leave_excess_input_for_the_next_turn() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.db"));
    let (s, m) = scripted(
        &k,
        vec![
            text("stale1"),
            text("stale2"),
            text("stale3"),
            text("latest response"),
        ],
        0,
    );
    let c = fixture_channel(true, SendOutcome::Confirmed("1".into()));
    let task = tokio::spawn(s.clone().run(c.clone()));
    s.receive(input("1", "first question?")).await.unwrap();
    for n in 1..=3 {
        eventually(|| m.requests.lock().unwrap().len() == n).await;
        s.receive(input(&(n + 1).to_string(), &format!("correction {n}?")))
            .await
            .unwrap();
        m.release.add_permits(1);
    }
    eventually(|| m.requests.lock().unwrap().len() == 4).await;
    m.release.add_permits(1);
    eventually(|| c.sent.lock().unwrap().len() == 2).await;
    let snapshot = k.channel_snapshot(s.id()).unwrap();
    assert_eq!(snapshot.batches.len(), 2);
    let first = k
        .actor_turn(snapshot.batches[0].actor_turn_id.unwrap())
        .unwrap();
    assert_eq!(first.model_requests, 3);
    assert_eq!(first.usage_tokens, 9);
    assert_eq!(first.private_chat.as_ref().unwrap().reconsiderations, 2);
    assert_eq!(
        first.private_chat.unwrap().decision,
        ChatDecision::Superseded
    );
    assert_eq!(snapshot.incoming[3].batch_id, Some(snapshot.batches[1].id));
    assert_eq!(c.sent.lock().unwrap()[1].text, "latest response");
    assert!(
        !c.sent
            .lock()
            .unwrap()
            .iter()
            .any(|d| d.text.starts_with("stale"))
    );
    k.shutdown().unwrap();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn cancelling_a_wait_stops_that_phase_and_a_new_message_can_start_a_turn() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.db"));
    let (s, m) = scripted(
        &k,
        vec![call("chat_wait", json!({"seconds":5})), text("fresh turn")],
        2,
    );
    let c = fixture_channel(true, SendOutcome::Confirmed("1".into()));
    let task = tokio::spawn(s.clone().run(c.clone()));
    s.receive(input("1", "稍等一下")).await.unwrap();
    eventually(|| {
        let snapshot = k.channel_snapshot(s.id()).unwrap();
        snapshot
            .batches
            .first()
            .and_then(|b| b.actor_turn_id)
            .is_some_and(|id| {
                k.actor_turn(id)
                    .unwrap()
                    .private_chat
                    .as_ref()
                    .is_some_and(|c| c.decision == ChatDecision::Waiting)
            })
    })
    .await;
    let id = turn(&k, &s).id;
    s.actor.cancel(id).await.unwrap();
    eventually(|| {
        k.actor_turn(id).unwrap().state == TurnState::Unknown && c.sent.lock().unwrap().len() == 1
    })
    .await;
    let saved = k.actor_turn(id).unwrap();
    assert_eq!(
        saved.private_chat.as_ref().unwrap().decision,
        ChatDecision::Interrupted
    );
    assert!(saved.private_chat.unwrap().wait_until_ms.is_none());
    assert_eq!(saved.actions[0].state, ActionState::Unknown);
    assert_eq!(m.requests.lock().unwrap().len(), 1);
    s.receive(input("2", "现在继续聊")).await.unwrap();
    eventually(|| c.sent.lock().unwrap().len() == 2).await;
    assert_eq!(m.requests.lock().unwrap().len(), 2);
    assert_eq!(c.sent.lock().unwrap()[1].text, "fresh turn");
    k.shutdown().unwrap();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn prewrite_filter_rechecks_credentials_in_a_previously_saved_draft() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.db"));
    let (s, m) = scripted(&k, vec![], 0);
    s.receive(input("1", "stored old reply")).await.unwrap();
    tokio::time::sleep(Duration::from_millis(780)).await;
    let batch = k.channel_next_batch(s.id()).unwrap().unwrap();
    let (turn, _) = k
        .actor_submit_scoped(
            batch.conversation_id,
            batch.id.0,
            batch.text.clone(),
            Some((s.id(), batch.grant_revision, batch.expires_at_ms)),
        )
        .unwrap();
    // Simulate a legacy draft whose old ordinary text is now a known credential.
    // No model or real credential is used, and Actor generation normally redacts it.
    let completed = k
        .actor_update(turn.id, |t| {
            t.message(MessageRole::Assistant, TOKEN.into())?;
            t.state = TurnState::Completed;
            t.private_chat.as_mut().unwrap().decision = ChatDecision::Reply;
            Ok(())
        })
        .unwrap();
    k.channel_link(batch.id, &completed).unwrap();
    k.channel_finish(batch.id, &completed).unwrap();
    let delivery = k.channel_claim_delivery(s.id()).unwrap().unwrap();
    assert!(!s.may_send(&delivery).await);
    k.channel_settle(delivery.id, SendOutcome::NotSent).unwrap();
    assert_eq!(
        k.channel_snapshot(s.id()).unwrap().deliveries[0].state,
        DeliveryState::NotSent
    );
    assert!(m.requests.lock().unwrap().is_empty());
    k.shutdown().unwrap();
}
