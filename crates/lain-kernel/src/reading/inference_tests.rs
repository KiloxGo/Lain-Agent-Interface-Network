use super::*;
use lain_wire::{InferenceBudgetId, inference::*};
use std::sync::atomic::{AtomicBool, Ordering};

struct Counted {
    requests: Mutex<Vec<Request>>,
    gate: Semaphore,
    fail: AtomicBool,
}
impl ActorModel for Counted {
    fn respond(&self, r: Request) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
        let sharing = r.persona.contains("This Kernel activity only considers");
        self.requests.lock().unwrap().push(r);
        Box::pin(async move {
            self.gate.acquire().await.unwrap().forget();
            ensure!(
                !self.fail.load(Ordering::SeqCst),
                "fixture provider failed after accepting request"
            );
            Ok(Reply {
                text: if sharing {
                    json!({"decision":"share","reason":"fixture useful point","text":"fixture source-linked share"}).to_string()
                } else {
                    "fixture response or source summary".into()
                },
                calls: vec![],
                usage_tokens: 5,
                continuation: vec![],
            })
        })
    }
}
fn counted(k: &Arc<Kernel>, permits: usize) -> (Arc<Actor>, Arc<Counted>) {
    let m = Arc::new(Counted {
        requests: Mutex::default(),
        gate: Semaphore::new(permits),
        fail: AtomicBool::new(false),
    });
    (
        Actor::with_model(k.clone(), None, TOKEN.parse().unwrap(), m.clone()),
        m,
    )
}
fn limit(max: u32) -> InferenceLimit {
    InferenceLimit {
        id: InferenceBudgetId::new(),
        max_requests: max,
        expires_at_ms: now_ms().unwrap() + 600000,
    }
}
fn enable(k: &Kernel, s: InferenceLimit) -> InferenceDecision {
    k.set_inference_budget(SetInferenceBudget {
        unlimited_private_channel: None,
        expected_revision: k.inference_snapshot().unwrap().decision.revision,
        limit: Some(s),
    })
    .unwrap()
}
fn stop(k: &Kernel) -> InferenceDecision {
    k.set_inference_budget(SetInferenceBudget {
        unlimited_private_channel: None,
        expected_revision: k.inference_snapshot().unwrap().decision.revision,
        limit: None,
    })
    .unwrap()
}

#[tokio::test]
async fn private_daily_is_persistent_scoped_and_preserves_spent_finite_budgets() {
    let (dir, k, b) = fixture(10);
    let old = limit(1);
    enable(&k, old.clone());
    let (a, m) = counted(&k, 10);
    let c = k.create_conversation().unwrap();
    let t = a
        .submit(c.id, uuid::Uuid::new_v4(), "finite fixture".into())
        .await
        .unwrap();
    assert_eq!(settled(&k, t.id).await.state, TurnState::Completed);
    k.set_inference_budget(SetInferenceBudget {
        expected_revision: 1,
        limit: None,
        unlimited_private_channel: Some(b.id),
    })
    .unwrap();
    assert_eq!(
        k.inference_snapshot().unwrap().admission,
        InferenceAdmission::PrivateConversationOnly
    );
    let t = private_turn(&k, &a, &b).await;
    let done = settled(&k, t.id).await;
    assert_eq!(done.state, TurnState::Completed);
    k.channel_finish(lain_wire::BatchId(done.request_id), &done)
        .unwrap();
    let c = k.create_conversation().unwrap();
    let t = a
        .submit(
            c.id,
            uuid::Uuid::new_v4(),
            "unbound call must be blocked".into(),
        )
        .await
        .unwrap();
    assert_eq!(settled(&k, t.id).await.model_requests, 0);
    let id = seed(&k, "reading remains outside daily private scope");
    let t = a.process_reading(id).await.unwrap();
    assert_eq!(settled(&k, t.id).await.model_requests, 0);
    let other = k
        .channel_bind(
            BindingSpec {
                name: "other".into(),
                adapter: "fixture".into(),
                account: "1001".into(),
                peer: "3003".into(),
            },
            true,
        )
        .unwrap();
    let t = private_turn(&k, &a, &other).await;
    assert_eq!(settled(&k, t.id).await.model_requests, 0);
    assert_eq!(m.requests.lock().unwrap().len(), 2);
    assert_eq!(
        k.inference_budgets()
            .unwrap()
            .iter()
            .find(|v| v.limit.id == old.id)
            .unwrap()
            .reserved_requests,
        1
    );
    until(|| Arc::strong_count(&a) == 1).await;
    drop(a);
    drop(k);
    let k = Kernel::open(&dir.path().join("state.db")).unwrap();
    k.mark_ready().unwrap();
    let s = k.inference_snapshot().unwrap();
    assert_eq!(s.decision.unlimited_private_channel, Some(b.id));
    assert!(s.decision.limit.is_none() && s.remaining_requests.is_none());
    assert_eq!(s.reserved_requests, 1);
    stop(&k);
    assert_eq!(
        k.inference_snapshot().unwrap().admission,
        InferenceAdmission::Stopped
    );
    assert!(
        k.set_inference_budget(SetInferenceBudget {
            expected_revision: 3,
            limit: Some(old.clone()),
            unlimited_private_channel: Some(b.id)
        })
        .is_err()
    );
    enable(&k, old);
    assert_eq!(
        k.inference_snapshot().unwrap().admission,
        InferenceAdmission::Exhausted
    );
}

#[tokio::test]
async fn private_daily_stop_between_reservation_and_entry_blocks_the_attempt() {
    let (_dir, k, b) = fixture(10);
    k.set_inference_budget(SetInferenceBudget {
        expected_revision: 0,
        limit: None,
        unlimited_private_channel: Some(b.id),
    })
    .unwrap();
    assert!(
        k.set_inference_budget(SetInferenceBudget {
            expected_revision: 1,
            limit: None,
            unlimited_private_channel: Some(lain_wire::ChannelId::new())
        })
        .is_err()
    );
    k.channel_receive(
        b.id,
        Input {
            segments: vec![],
            account: b.spec.account.clone(),
            peer: b.spec.peer.clone(),
            external_id: uuid::Uuid::new_v4().to_string(),
            text: "prepared fixture".into(),
        },
    )
    .unwrap();
    tokio::time::sleep(Duration::from_millis(780)).await;
    let batch = k.channel_next_batch(b.id).unwrap().unwrap();
    let (t, _) = k
        .actor_submit_scoped(
            b.conversation_id,
            batch.id.0,
            "prepared fixture".into(),
            Some((b.id, b.grant_revision, batch.expires_at_ms)),
        )
        .unwrap();
    k.actor_update(t.id, |t| {
        t.model_requests += 1;
        Ok(())
    })
    .unwrap();
    stop(&k);
    assert!(k.begin_inference(t.id, 1).is_err());
    k.actor_update(t.id, |t| {
        t.state = TurnState::Unknown;
        Ok(())
    })
    .unwrap();
    let s = k.inference_snapshot().unwrap();
    assert_eq!(s.reserved_requests, 1);
    assert_eq!(
        s.recent_requests[0].state,
        InferenceRequestState::NotAttempted
    );
    assert!(s.recent_requests[0].attempted_at_ms.is_none());
}

#[tokio::test]
async fn inference_one_budget_covers_reading_sharing_private_and_unbound_conversation() {
    let (_dir, k, b) = fixture(10);
    let budget = limit(3);
    enable(&k, budget.clone());
    let (a, m) = counted(&k, 10);
    let id = seed(&k, "source for shared budget");
    let t = a.process_reading(id).await.unwrap();
    assert_eq!(settled(&k, t.id).await.state, TurnState::Completed);
    let e = k
        .experiences(query(), Redactor::new(vec![]))
        .unwrap()
        .remove(0);
    let mut share = super::sharing::scope();
    share.max_model_requests = 10;
    k.set_share_grant(
        b.id,
        lain_wire::sharing::SetShareGrant {
            expected_revision: 0,
            scope: Some(share),
        },
    )
    .unwrap();
    k.set_attention(
        b.id,
        lain_wire::sharing::SetAttention {
            expected_revision: 0,
            do_not_disturb: false,
            until_ms: None,
        },
    )
    .unwrap();
    let c = k.share_consider(b.id, e.id).unwrap();
    let t = a.draft_share(c.id).await.unwrap();
    assert_eq!(settled(&k, t.id).await.state, TurnState::Completed);
    let t = private_turn(&k, &a, &b).await;
    assert_eq!(settled(&k, t.id).await.state, TurnState::Completed);
    let c = k.create_conversation().unwrap();
    let t = a
        .submit(
            c.id,
            uuid::Uuid::new_v4(),
            "must not call another model".into(),
        )
        .await
        .unwrap();
    let done = settled(&k, t.id).await;
    assert_eq!(done.model_requests, 0);
    assert_ne!(done.state, TurnState::Completed);
    assert_eq!(m.requests.lock().unwrap().len(), 3);
    let s = k.inference_snapshot().unwrap();
    assert_eq!(s.admission, InferenceAdmission::Exhausted);
    assert_eq!(s.reserved_requests, 3);
    assert_eq!(s.reported_usage_tokens_sum, 15);
    let records = k.inference_requests(Some(budget.id)).unwrap();
    assert_eq!(records.len(), 3);
    assert!(
        records
            .iter()
            .all(|r| r.state == InferenceRequestState::Returned)
    );
    for reason in [
        InferenceReason::Reading,
        InferenceReason::Sharing,
        InferenceReason::PrivateConversation,
    ] {
        assert!(records.iter().any(|r| r.reason == reason));
    }
    // Per-source grants and fresh Actor/conversation identities cannot replenish the ceiling.
    let g = k.reading_grant().unwrap();
    k.set_reading_grant(SetReadingGrant {
        expected_revision: 1,
        scope: g.scope,
    })
    .unwrap();
    let id = seed(&k, "another grant still shares the global ceiling");
    let t = a.process_reading(id).await.unwrap();
    settled(&k, t.id).await;
    assert_eq!(m.requests.lock().unwrap().len(), 3);
    assert_eq!(k.inference_snapshot().unwrap().reserved_requests, 3);
}

#[tokio::test]
async fn inference_concurrent_turns_atomically_compete_for_last_request() {
    let (_dir, k, _) = fixture(10);
    let budget = limit(1);
    enable(&k, budget);
    let (a, m) = counted(&k, 0);
    let c1 = k.create_conversation().unwrap();
    let c2 = k.create_conversation().unwrap();
    let (t1, t2) = tokio::join!(
        a.submit(c1.id, uuid::Uuid::new_v4(), "one".into()),
        a.submit(c2.id, uuid::Uuid::new_v4(), "two".into())
    );
    let (t1, t2) = (t1.unwrap(), t2.unwrap());
    until(|| m.requests.lock().unwrap().len() == 1).await;
    assert_eq!(k.inference_snapshot().unwrap().reserved_requests, 1);
    m.gate.add_permits(1);
    let (d1, d2) = (settled(&k, t1.id).await, settled(&k, t2.id).await);
    assert_eq!(
        [d1, d2]
            .iter()
            .filter(|t| t.state == TurnState::Completed)
            .count(),
        1
    );
    assert_eq!(m.requests.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn inference_cancel_failure_stop_and_reopen_never_refund_or_reset() {
    for cancel in [false, true] {
        let (dir, k, _) = fixture(10);
        let budget = limit(1);
        enable(&k, budget.clone());
        let (a, m) = counted(&k, if cancel { 0 } else { 1 });
        m.fail.store(!cancel, Ordering::SeqCst);
        let c = k.create_conversation().unwrap();
        let t = a
            .submit(c.id, uuid::Uuid::new_v4(), "fixture uncertain call".into())
            .await
            .unwrap();
        until(|| m.requests.lock().unwrap().len() == 1).await;
        if cancel {
            a.cancel(t.id).await.unwrap();
        }
        settled(&k, t.id).await;
        assert_eq!(
            k.inference_snapshot().unwrap().recent_requests[0].state,
            InferenceRequestState::Unknown
        );
        stop(&k);
        assert_eq!(k.inference_snapshot().unwrap().reserved_requests, 1);
        enable(&k, budget.clone());
        assert_eq!(
            k.inference_snapshot().unwrap().admission,
            InferenceAdmission::Exhausted
        );
        until(|| Arc::strong_count(&a) == 1).await;
        drop(a);
        drop(k);
        let k = Arc::new(Kernel::open(&dir.path().join("state.db")).unwrap());
        k.mark_ready().unwrap();
        assert_eq!(k.inference_snapshot().unwrap().reserved_requests, 1);
        assert_eq!(
            k.inference_snapshot().unwrap().admission,
            InferenceAdmission::Exhausted
        );
        let (a, m) = counted(&k, 1);
        let c = k.create_conversation().unwrap();
        let t = a
            .submit(c.id, uuid::Uuid::new_v4(), "restart cannot refill".into())
            .await
            .unwrap();
        settled(&k, t.id).await;
        assert!(m.requests.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn inference_stop_during_call_records_return_and_denies_new_scope_until_quiescent() {
    let (_dir, k, _) = fixture(10);
    let budget = limit(2);
    enable(&k, budget);
    let (a, m) = counted(&k, 0);
    let c = k.create_conversation().unwrap();
    let t = a
        .submit(c.id, uuid::Uuid::new_v4(), "in flight".into())
        .await
        .unwrap();
    until(|| m.requests.lock().unwrap().len() == 1).await;
    stop(&k);
    assert!(
        k.set_inference_budget(SetInferenceBudget {
            unlimited_private_channel: None,
            expected_revision: 2,
            limit: Some(limit(1))
        })
        .is_err()
    );
    m.gate.add_permits(1);
    settled(&k, t.id).await;
    let s = k.inference_snapshot().unwrap();
    assert_eq!(s.admission, InferenceAdmission::Stopped);
    assert_eq!(s.reported_usage_tokens_sum, 5);
    assert_eq!(s.recent_requests[0].state, InferenceRequestState::Returned);
    let c = k.create_conversation().unwrap();
    let t = a
        .submit(c.id, uuid::Uuid::new_v4(), "stopped".into())
        .await
        .unwrap();
    settled(&k, t.id).await;
    assert_eq!(m.requests.lock().unwrap().len(), 1);
    enable(&k, limit(1));
    assert_eq!(k.inference_snapshot().unwrap().remaining_requests, Some(1));
}

#[tokio::test]
async fn inference_prepared_stop_and_restart_do_not_replay_or_refund() {
    let (dir, k, _) = fixture(10);
    let budget = limit(2);
    enable(&k, budget.clone());
    let make = || {
        let c = k.create_conversation().unwrap();
        let (t, _) = k
            .actor_submit(
                c.id,
                uuid::Uuid::new_v4(),
                "manual reservation fixture".into(),
            )
            .unwrap();
        k.actor_update(t.id, |t| {
            t.model_requests += 1;
            Ok(())
        })
        .unwrap()
    };
    let first = make();
    let second = make();
    k.begin_inference(second.id, 1).unwrap();
    assert_eq!(k.inference_snapshot().unwrap().pending_actor_requests, 2);
    drop(k);
    let k = Kernel::open(&dir.path().join("state.db")).unwrap();
    k.mark_ready().unwrap();
    let requests = k.inference_requests(Some(budget.id)).unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests
            .iter()
            .find(|r| r.actor_turn_id == first.id)
            .unwrap()
            .state,
        InferenceRequestState::NotAttempted
    );
    assert_eq!(
        requests
            .iter()
            .find(|r| r.actor_turn_id == second.id)
            .unwrap()
            .state,
        InferenceRequestState::Unknown
    );
    assert_eq!(
        k.inference_snapshot().unwrap().admission,
        InferenceAdmission::Exhausted
    );
    assert_eq!(k.inference_snapshot().unwrap().pending_actor_requests, 0);
    assert!(k.begin_inference(first.id, 1).is_err());
}

#[tokio::test]
async fn inference_expiry_or_stop_between_reservation_and_adapter_blocks_attempt() {
    for expiry in [false, true] {
        let (_dir, k, _) = fixture(10);
        let mut budget = limit(1);
        if expiry {
            budget.expires_at_ms = now_ms().unwrap() + 2000;
        }
        enable(&k, budget.clone());
        let c = k.create_conversation().unwrap();
        let (t, _) = k
            .actor_submit(
                c.id,
                uuid::Uuid::new_v4(),
                "prepared but not attempted".into(),
            )
            .unwrap();
        k.actor_update(t.id, |t| {
            t.model_requests += 1;
            Ok(())
        })
        .unwrap();
        if expiry {
            tokio::time::sleep(Duration::from_millis(
                budget.expires_at_ms.saturating_sub(now_ms().unwrap()) + 10,
            ))
            .await;
        } else {
            stop(&k);
        }
        assert!(k.begin_inference(t.id, 1).is_err());
        k.actor_update(t.id, |t| {
            t.state = TurnState::Unknown;
            Ok(())
        })
        .unwrap();
        let s = k.inference_snapshot().unwrap();
        assert_eq!(s.reserved_requests, 1);
        assert_eq!(
            s.recent_requests[0].state,
            InferenceRequestState::NotAttempted
        );
        assert!(s.recent_requests[0].attempted_at_ms.is_none());
    }
}

#[tokio::test]
async fn inference_reservation_and_actor_counter_are_atomic_and_audit_has_no_prompt() {
    let (dir, k, _) = fixture(10);
    enable(&k, limit(1));
    let db = rusqlite::Connection::open(dir.path().join("state.db")).unwrap();
    db.execute_batch("CREATE TRIGGER refuse_inference BEFORE INSERT ON inference_requests BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    let (a, m) = counted(&k, 2);
    let c = k.create_conversation().unwrap();
    let t = a
        .submit(
            c.id,
            uuid::Uuid::new_v4(),
            "must-not-be-in-inference-ledger".into(),
        )
        .await
        .unwrap();
    let done = settled(&k, t.id).await;
    assert_eq!(done.model_requests, 0);
    assert!(m.requests.lock().unwrap().is_empty());
    assert_eq!(k.inference_snapshot().unwrap().reserved_requests, 0);
    db.execute_batch("DROP TRIGGER refuse_inference;").unwrap();
    let c = k.create_conversation().unwrap();
    let t = a
        .submit(
            c.id,
            uuid::Uuid::new_v4(),
            "must-not-be-in-inference-ledger".into(),
        )
        .await
        .unwrap();
    assert_eq!(settled(&k, t.id).await.state, TurnState::Completed);
    let records = k.inference_requests(None).unwrap();
    assert_eq!(records.len(), 1);
    assert!(
        !serde_json::to_string(&records)
            .unwrap()
            .contains("must-not-be-in-inference-ledger")
    );
}

#[tokio::test]
async fn inference_ids_limits_cas_and_legacy_inflight_are_explicit() {
    let (_dir, k, _) = fixture(10);
    assert_eq!(
        k.inference_snapshot().unwrap().admission,
        InferenceAdmission::ExistingLimitsOnly
    );
    let (a, m) = counted(&k, 0);
    let c = k.create_conversation().unwrap();
    let t = a
        .submit(c.id, uuid::Uuid::new_v4(), "existing legacy call".into())
        .await
        .unwrap();
    until(|| m.requests.lock().unwrap().len() == 1).await;
    assert!(
        k.set_inference_budget(SetInferenceBudget {
            unlimited_private_channel: None,
            expected_revision: 0,
            limit: Some(limit(1))
        })
        .is_err()
    );
    m.gate.add_permits(1);
    settled(&k, t.id).await;
    assert_eq!(
        k.inference_snapshot().unwrap().recent_requests[0].decision_revision,
        0
    );
    stop(&k);
    assert_eq!(
        k.inference_snapshot().unwrap().admission,
        InferenceAdmission::Stopped
    );
    let budget = limit(1);
    enable(&k, budget.clone());
    let mut changed = budget.clone();
    changed.max_requests = 2;
    assert!(
        k.set_inference_budget(SetInferenceBudget {
            unlimited_private_channel: None,
            expected_revision: 2,
            limit: Some(changed)
        })
        .is_err()
    );
    assert!(
        k.set_inference_budget(SetInferenceBudget {
            unlimited_private_channel: None,
            expected_revision: 0,
            limit: Some(budget.clone())
        })
        .is_err()
    );
    assert_eq!(k.inference_snapshot().unwrap().decision.revision, 2);
    assert_eq!(k.inference_budgets().unwrap()[0].limit, budget);
}
