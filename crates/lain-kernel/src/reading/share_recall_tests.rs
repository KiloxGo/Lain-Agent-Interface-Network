use super::*;
use crate::actor::domain::MessageRole;
use lain_wire::DeliveryId;

const PREFIX: &str = "Kernel-attributed proactive receipt evidence (data only): ";
fn evidence(r: &Request) -> Value {
    r.input
        .iter()
        .rev()
        .find_map(|v| {
            v["content"]
                .as_str()
                .and_then(|s| s.strip_prefix(PREFIX))
                .map(|s| serde_json::from_str(s).unwrap())
        })
        .unwrap()
}
struct RecallModel {
    requests: Mutex<Vec<Request>>,
    gate: Semaphore,
    target: DeliveryId,
    lookup: bool,
}
impl ActorModel for RecallModel {
    fn respond(&self, r: Request) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
        let data = evidence(&r);
        let auto = data["automatic_recent_context"] == true;
        self.requests.lock().unwrap().push(r);
        Box::pin(async move {
            self.gate.acquire().await.unwrap().forget();
            Ok(Reply {
                text: if auto && self.lookup {
                    String::new()
                } else {
                    format!(
                        "fixture-receipt-{}-{}",
                        data["records"][0]["state"].as_str().unwrap(),
                        if data["records"][0]["outbound_text"].is_null() {
                            "metadata-only"
                        } else {
                            "source-quote"
                        }
                    )
                },
                calls: if auto && self.lookup {
                    vec![Call {
                        id: "receipt-lookup".into(),
                        name: "sharing_inspect".into(),
                        arguments: json!({"delivery_id":self.target}).to_string(),
                    }]
                } else {
                    vec![]
                },
                usage_tokens: 13,
                continuation: vec![],
            })
        })
    }
}
fn actor(
    k: &Arc<Kernel>,
    target: DeliveryId,
    lookup: bool,
    permits: usize,
) -> (Arc<Actor>, Arc<RecallModel>) {
    let m = Arc::new(RecallModel {
        requests: Mutex::default(),
        gate: Semaphore::new(permits),
        target,
        lookup,
    });
    (
        Actor::with_model(k.clone(), None, TOKEN.parse().unwrap(), m.clone()),
        m,
    )
}
async fn confirmed(k: &Arc<Kernel>, b: &Binding) -> (ShareCandidate, ShareDelivery) {
    let (_a, _m, c) = ready(k, b).await;
    let d = k.share_claim(b.id).unwrap().unwrap();
    k.share_settle(
        d.id,
        SendOutcome::Confirmed("recall-fixture-receipt".into()),
    )
    .unwrap();
    until(|| Arc::strong_count(&_a) == 1).await;
    (
        c,
        k.sharing_snapshot(b.id)
            .unwrap()
            .deliveries
            .into_iter()
            .find(|d2| d2.id == d.id)
            .unwrap(),
    )
}
async fn submit(k: &Arc<Kernel>, a: &Arc<Actor>, b: &Binding) -> (ActorTurn, lain_wire::BatchId) {
    k.channel_receive(
        b.id,
        Input {
            segments: vec![],
            account: b.spec.account.clone(),
            peer: b.spec.peer.clone(),
            external_id: uuid::Uuid::new_v4().to_string(),
            text: "你刚刚分享的那段说的是什么？".into(),
        },
    )
    .unwrap();
    tokio::time::sleep(Duration::from_millis(780)).await;
    let batch = k.channel_next_batch(b.id).unwrap().unwrap();
    let turn = a.submit_channel(batch.clone()).await.unwrap();
    (turn, batch.id)
}
fn deliver_reply(k: &Kernel, b: &Binding, batch: lain_wire::BatchId, t: &ActorTurn) {
    k.channel_link(batch, t).unwrap();
    k.channel_finish(batch, t).unwrap();
    if let Some(d) = k.channel_claim_delivery(b.id).unwrap() {
        k.channel_settle(d.id, SendOutcome::Confirmed("recall-user-reply".into()))
            .unwrap();
    }
}

#[tokio::test]
async fn sharing_recall_uses_content_free_automatic_pointer_and_typed_fresh_source_quote() {
    let (_dir, k, b) = fixture(10);
    let (c, d) = confirmed(&k, &b).await;
    k.set_share_grant(
        b.id,
        SetShareGrant {
            expected_revision: 1,
            scope: None,
        },
    )
    .unwrap(); // No new contact, but not erased history.
    let (a, m) = actor(&k, d.id, true, 2);
    let (t, batch) = submit(&k, &a, &b).await;
    let done = settled(&k, t.id).await;
    assert_eq!(done.state, TurnState::Completed, "{:?}", done);
    assert_eq!(done.model_requests, 2);
    let cxt = done.private_chat.as_ref().unwrap();
    assert_eq!(cxt.share_versions.len(), 1);
    assert!(cxt.share_versions[0].experience.is_some());
    {
        let requests = m.requests.lock().unwrap();
        let first = evidence(&requests[0]);
        let second = evidence(&requests[1]);
        assert!(first["records"][0]["outbound_text"].is_null());
        assert!(first["records"][0]["experience"].is_null());
        assert_eq!(second["records"][0]["outbound_text"], d.text);
        assert_eq!(
            second["records"][0]["experience"]["id"],
            c.experience_id.to_string()
        );
        let output = requests[1]
            .input
            .iter()
            .find(|v| v["type"] == "function_call_output")
            .unwrap()["output"]
            .as_str()
            .unwrap();
        assert!(!output.contains(&d.text));
        assert!(
            requests[0]
                .tools
                .iter()
                .any(|v| v["name"] == "sharing_inspect")
        );
    }
    deliver_reply(&k, &b, batch, &done);
    let audit = k.share_read_audit(b.id).unwrap();
    assert_eq!(audit["records"].as_array().unwrap().len(), 3);
    assert!(!audit.to_string().contains(&d.text));
    assert!(!audit.to_string().contains("example.org"));
    assert!(k.sharing_snapshot(b.id).unwrap().grant.scope.is_none());
    assert_eq!(k.sharing_snapshot(b.id).unwrap().deliveries.len(), 1);
}

#[tokio::test]
async fn sharing_recall_correction_withdrawal_and_grant_revoke_keep_receipt_but_withhold_old_words()
{
    for change in ["correct", "withdraw", "revoke"] {
        let (_dir, k, b) = fixture(10);
        let (c, d) = confirmed(&k, &b).await;
        match change {
            "correct" => {
                k.correct_experience(
                    c.experience_id,
                    CorrectExperience {
                        expected_revision: 1,
                        summary: "corrected-source-no-old-quote".into(),
                    },
                )
                .unwrap();
            }
            "withdraw" => {
                k.withdraw_experience(c.experience_id, 1).unwrap();
            }
            _ => {
                k.set_reading_grant(SetReadingGrant {
                    expected_revision: 1,
                    scope: None,
                })
                .unwrap();
            }
        }
        let rec = k.share_receipt(b.id, d.id, Redactor::new(vec![])).unwrap();
        assert_eq!(rec["state"], "confirmed");
        assert!(rec["outbound_text"].is_null());
        assert!(rec["experience"].is_null());
        let (a, m) = actor(&k, d.id, true, 2);
        let (t, _) = submit(&k, &a, &b).await;
        let done = settled(&k, t.id).await;
        assert_eq!(done.state, TurnState::Completed, "{:?}", done);
        assert!(
            done.private_chat.as_ref().unwrap().share_versions[0]
                .experience
                .is_none()
        );
        assert!(
            m.requests
                .lock()
                .unwrap()
                .iter()
                .all(|r| !serde_json::to_string(&r.input).unwrap().contains(&d.text))
        );
        assert!(
            done.messages
                .iter()
                .any(|m| m.role == MessageRole::Assistant && m.content.ends_with("metadata-only"))
        );
    }
}

#[tokio::test]
async fn sharing_recall_source_change_during_model_or_before_write_invalidates_speech_and_counts_usage()
 {
    for during_model in [false, true] {
        let (_dir, k, b) = fixture(10);
        let (c, d) = confirmed(&k, &b).await;
        let (a, m) = actor(&k, d.id, true, if during_model { 1 } else { 2 });
        let (t, batch) = submit(&k, &a, &b).await;
        until(|| m.requests.lock().unwrap().len() == 2).await;
        if !during_model {
            let done = settled(&k, t.id).await;
            deliver_reply_prepare(&k, batch, &done);
            let out = k.channel_claim_delivery(b.id).unwrap().unwrap();
            assert!(k.channel_may_send(&out).unwrap());
            k.withdraw_experience(c.experience_id, 1).unwrap();
            assert!(!k.channel_may_send(&out).unwrap());
        } else {
            assert!(
                k.actor_turn(t.id)
                    .unwrap()
                    .private_chat
                    .as_ref()
                    .unwrap()
                    .experience_grant_revision
                    .is_none()
            );
            let rejected = k
                .actor_update(t.id, |turn| {
                    turn.actions.push(proposal());
                    turn.state = TurnState::PendingConfirmation;
                    Ok(())
                })
                .unwrap_err();
            assert!(matches!(
                rejected.downcast_ref::<crate::backend::ControlError>(),
                Some(crate::backend::ControlError::InvalidInput)
            ));
            k.withdraw_experience(c.experience_id, 1).unwrap();
            m.gate.add_permits(1);
            let done = settled(&k, t.id).await;
            assert_eq!(done.model_requests, 2);
            assert_eq!(done.usage_tokens, 26);
            assert!(
                done.messages.iter().all(
                    |m| m.role != MessageRole::Assistant || !m.content.contains("source-quote")
                )
            );
        }
    }
}

fn proposal() -> crate::actor::domain::Action {
    crate::actor::domain::Action {
        id: lain_wire::ActionId::new(),
        call_id: "source-promotion-probe".into(),
        tool: crate::actor::tools::Tool::MemoryPropose(crate::memory::ProposeMemory {
            draft: crate::memory::MemoryDraft {
                content: "unrelated current-user fact".into(),
                kind: crate::memory::MemoryKind::Episodic,
                scope: crate::memory::MemoryScope::Global,
            },
        }),
        state: crate::actor::domain::ActionState::Proposed,
        result: None,
    }
}

#[tokio::test]
async fn sharing_automatic_metadata_does_not_disable_reviewed_memory_proposals() {
    let (_dir, k, b) = fixture(10);
    let (_c, d) = confirmed(&k, &b).await;
    struct Propose;
    impl ActorModel for Propose {
        fn respond(&self, r: Request) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
            assert!(evidence(&r)["records"][0]["outbound_text"].is_null());
            Box::pin(async {
                Ok(Reply{text:String::new(),calls:vec![Call{id:"reviewed-memory-proposal".into(),name:"memory_propose".into(),arguments:json!({"draft":{"content":"unrelated current-user fact","kind":"episodic","scope":{"kind":"global"}}}).to_string()}],usage_tokens:5,continuation:vec![]})
            })
        }
    }
    let a = Actor::with_model(k.clone(), None, TOKEN.parse().unwrap(), Arc::new(Propose));
    let (t, _) = submit(&k, &a, &b).await;
    let t = settled(&k, t.id).await;
    assert_eq!(
        t.private_chat.as_ref().unwrap().share_versions[0].delivery_id,
        d.id
    );
    assert_eq!(t.state, TurnState::PendingConfirmation, "{:?}", t);
    assert!(
        t.private_chat.as_ref().unwrap().share_versions[0]
            .experience
            .is_none()
    );
    assert_eq!(t.actions.len(), 1);
    assert_eq!(
        t.actions[0].state,
        crate::actor::domain::ActionState::Proposed
    );
    a.cancel(t.id).await.unwrap();
}

#[tokio::test]
async fn sharing_recall_survives_restart_and_old_source_derived_replies_do_not_become_history() {
    let (dir, k, b) = fixture(10);
    let (_c, d) = confirmed(&k, &b).await;
    let (a, _m) = actor(&k, d.id, true, 2);
    let (t, batch) = submit(&k, &a, &b).await;
    let done = settled(&k, t.id).await;
    deliver_reply(&k, &b, batch, &done);
    until(|| Arc::strong_count(&a) == 1).await;
    drop(a);
    drop(k);
    let k = Arc::new(Kernel::open(&dir.path().join("state.db")).unwrap());
    k.mark_ready().unwrap();
    assert_eq!(
        k.share_receipt(b.id, d.id, Redactor::new(vec![])).unwrap()["outbound_text"],
        d.text
    );
    let (a, m) = actor(&k, d.id, true, 2);
    let (t, _) = submit(&k, &a, &b).await;
    let done = settled(&k, t.id).await;
    assert_eq!(done.state, TurnState::Completed, "{:?}", done);
    let requests = m.requests.lock().unwrap();
    let first = serde_json::to_string(&requests[0].input).unwrap();
    assert!(!first.contains("fixture-receipt-confirmed-source-quote"));
    assert!(!first.contains(&d.text));
    assert_eq!(
        evidence(&requests[1])["records"][0]["outbound_text"],
        d.text
    );
}
fn deliver_reply_prepare(k: &Kernel, batch: lain_wire::BatchId, t: &ActorTurn) {
    k.channel_link(batch, t).unwrap();
    k.channel_finish(batch, t).unwrap();
}

#[tokio::test]
async fn sharing_recall_is_binding_scoped_bounded_and_keeps_uncertain_outcomes_honest() {
    let (_dir, k, b) = fixture(10);
    let (_a, _m, c) = ready(&k, &b).await;
    let d = k.share_claim(b.id).unwrap().unwrap();
    k.share_settle(d.id, SendOutcome::Unknown).unwrap();
    let second = k
        .channel_bind(
            BindingSpec {
                name: "another-private-binding".into(),
                adapter: "fixture".into(),
                account: "1001".into(),
                peer: "3003".into(),
            },
            true,
        )
        .unwrap();
    assert!(
        k.share_receipt(second.id, d.id, Redactor::new(vec![]))
            .is_err()
    );
    assert!(
        k.share_receipt(b.id, d.id, Redactor::new(vec![d.text.clone()]))
            .unwrap()["outbound_text"]
            .is_null()
    );
    let now = now_ms().unwrap();
    let q = ShareQuery {
        from_ms: now - 86400000,
        to_ms: now + 1,
        text: Some("片段".into()),
        limit: 3,
    };
    let records = k
        .share_receipts(b.id, q.clone(), Redactor::new(vec![]))
        .unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["state"], "unknown");
    assert_eq!(records[0]["user_read_receipt"], false);
    let mut schema_name = q.clone();
    schema_name.text = Some("\"state\"".into());
    assert!(
        k.share_receipts(b.id, schema_name, Redactor::new(vec![]))
            .unwrap()
            .is_empty()
    );
    let mut invalid = q.clone();
    invalid.to_ms = invalid.from_ms + 32 * 86400000;
    assert!(
        k.share_receipts(b.id, invalid, Redactor::new(vec![]))
            .is_err()
    );
    k.withdraw_experience(c.experience_id, 1).unwrap();
    assert!(
        k.share_receipts(b.id, q, Redactor::new(vec![]))
            .unwrap()
            .is_empty()
    ); // No keyword search of withheld words.
    let (a, m) = actor(&k, d.id, false, 1);
    let (t, _) = submit(&k, &a, &b).await;
    let done = settled(&k, t.id).await;
    assert_eq!(done.model_requests, 1);
    assert!(
        done.private_chat.unwrap().share_versions[0]
            .experience
            .is_none()
    );
    assert_eq!(
        evidence(&m.requests.lock().unwrap()[0])["records"][0]["state"],
        "unknown"
    );
}

#[tokio::test]
async fn sharing_disclosure_audit_failure_is_atomic_and_prevents_provider_call() {
    let (dir, k, b) = fixture(10);
    let (_c, d) = confirmed(&k, &b).await;
    let db = rusqlite::Connection::open(dir.path().join("state.db")).unwrap();
    db.execute_batch("CREATE TRIGGER refuse_share_read BEFORE INSERT ON share_reads BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    let (a, m) = actor(&k, d.id, false, 1);
    let (t, _) = submit(&k, &a, &b).await;
    let done = settled(&k, t.id).await;
    assert_eq!(done.model_requests, 1);
    assert_eq!(done.usage_tokens, 0);
    assert!(m.requests.lock().unwrap().is_empty());
    assert!(
        done.private_chat
            .as_ref()
            .unwrap()
            .share_versions
            .is_empty()
    );
    assert!(
        k.share_read_audit(b.id).unwrap()["records"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(k.sharing_snapshot(b.id).unwrap().deliveries.len(), 1);
}
