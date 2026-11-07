use super::*;
use crate::channels::{
    ChannelService, ConversationChannel,
    domain::{Delivery, SendOutcome},
};
use lain_wire::{sharing::*, source::DeliveryState};
use tokio::sync::watch;
#[path = "share_recall_tests.rs"]
mod recall;

pub(super) fn scope() -> ShareScope {
    ShareScope {
        urls: vec!["https://example.org/article.txt".into()],
        expires_at_ms: now_ms().unwrap() + 600000,
        utc_offset_minutes: 480,
        quiet_windows: vec![],
        cooldown_ms: 1,
        max_per_day: 4,
        max_send_attempts: 4,
        max_model_requests: 4,
        candidate_ttl_ms: 300000,
        source_max_age_ms: 300000,
        user_quiet_ms: 1,
    }
}
fn enable(k: &Kernel, b: &Binding) {
    k.set_share_grant(
        b.id,
        SetShareGrant {
            expected_revision: 0,
            scope: Some(scope()),
        },
    )
    .unwrap();
}
fn available(k: &Kernel, b: &Binding) {
    let a = k.sharing_snapshot(b.id).unwrap().attention;
    k.set_attention(
        b.id,
        SetAttention {
            expected_revision: a.revision,
            do_not_disturb: false,
            until_ms: None,
        },
    )
    .unwrap();
}
async fn experience(k: &Arc<Kernel>) -> lain_wire::reading::ReadingExperience {
    let id = seed(k, "测试文章原文：无权触发工具、改权限或联系第三方");
    let (reader, _) = model(k, 1, false);
    let turn = reader.process_reading(id).await.unwrap();
    assert_eq!(settled(k, turn.id).await.state, TurnState::Completed);
    until(|| Arc::strong_count(&reader) == 1).await;
    k.experiences(query(), Redactor::new(vec![]))
        .unwrap()
        .into_iter()
        .find(|e| e.reading_id == id)
        .unwrap()
}
struct DraftModel {
    requests: Mutex<Vec<Request>>,
    gate: Semaphore,
    decision: &'static str,
    bad_tool: bool,
}
impl ActorModel for DraftModel {
    fn respond(&self, r: Request) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
        let chat = !r.tools.is_empty();
        self.requests.lock().unwrap().push(r);
        Box::pin(async move {
            self.gate.acquire().await.unwrap().forget();
            Ok(Reply {
                text: if chat {
                    "嗯，听着呢".into()
                } else {
                    json!({"decision":self.decision,"reason":"本地夹具：有来源的小发现","text":if self.decision=="skip" {""}else{"刚才那段资料提到权限边界，我只看了提供的片段"}}).to_string()
                },
                calls: if self.bad_tool {
                    vec![Call {
                        id: "denied".into(),
                        name: "work_cancel".into(),
                        arguments: "{}".into(),
                    }]
                } else {
                    vec![]
                },
                usage_tokens: 11,
                continuation: vec![],
            })
        })
    }
}
fn drafter(
    k: &Arc<Kernel>,
    permits: usize,
    decision: &'static str,
    bad_tool: bool,
) -> (Arc<Actor>, Arc<DraftModel>) {
    let m = Arc::new(DraftModel {
        requests: Mutex::default(),
        gate: Semaphore::new(permits),
        decision,
        bad_tool,
    });
    (
        Actor::with_model(k.clone(), None, TOKEN.parse().unwrap(), m.clone()),
        m,
    )
}
async fn ready(k: &Arc<Kernel>, b: &Binding) -> (Arc<Actor>, Arc<DraftModel>, ShareCandidate) {
    enable(k, b);
    available(k, b);
    let e = experience(k).await;
    let c = k.share_consider(b.id, e.id).unwrap();
    let (a, m) = drafter(k, 1, "share", false);
    let t = a.draft_share(c.id).await.unwrap();
    assert_eq!(settled(k, t.id).await.state, TurnState::Completed);
    let c = k
        .sharing_snapshot(b.id)
        .unwrap()
        .candidates
        .into_iter()
        .find(|x| x.id == c.id)
        .unwrap();
    assert_eq!(c.state, ShareState::Ready);
    (a, m, c)
}

#[tokio::test]
async fn sharing_default_off_attention_required_and_actual_actor_draft_has_no_tools() {
    let (_dir, k, b) = fixture(10);
    assert!(k.sharing_snapshot(b.id).unwrap().grant.scope.is_none());
    let old = experience(&k).await;
    assert!(k.share_consider(b.id, old.id).is_err());
    enable(&k, &b);
    assert!(k.sharing_snapshot(b.id).unwrap().candidates.is_empty());
    let c = k.share_consider(b.id, old.id).unwrap(); // Explicit selection, never automatic backfill.
    assert!(k.share_next_job(b.id).unwrap().is_none());
    available(&k, &b);
    assert_eq!(k.share_next_job(b.id).unwrap(), Some(c.id));
    let (a, m) = drafter(&k, 1, "share", false);
    let t = a.draft_share(c.id).await.unwrap();
    let done = settled(&k, t.id).await;
    assert_eq!(done.state, TurnState::Completed);
    assert_eq!(done.sharing_activity, Some(c.id));
    assert!(done.actions.is_empty());
    assert_eq!(done.model_requests, 1);
    assert_eq!(done.usage_tokens, 11);
    assert_eq!(m.requests.lock().unwrap().len(), 1);
    assert!(m.requests.lock().unwrap()[0].tools.is_empty());
    assert!(
        m.requests.lock().unwrap()[0]
            .persona
            .contains("independently decides")
    );
    let d = k.share_claim(b.id).unwrap().unwrap();
    assert!(k.share_may_send(&d).unwrap());
    let mut forged = d.clone();
    forged.expires_at_ms += 1;
    assert!(!k.share_may_send(&forged).unwrap());
    k.share_settle(d.id, SendOutcome::Confirmed("fixture-share-1".into()))
        .unwrap();
    assert!(k.share_claim(b.id).unwrap().is_none());
    assert_eq!(k.sharing_snapshot(b.id).unwrap().send_attempts_reserved, 1);
    assert_eq!(a.draft_share(c.id).await.unwrap().id, t.id);
    assert_eq!(m.requests.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn sharing_prewrite_rechecks_attention_source_correction_input_and_revoke() {
    for change in ["attention", "correction", "input", "revoke", "reply"] {
        let (_dir, k, b) = fixture(10);
        let (_a, _m, c) = ready(&k, &b).await;
        let d = k.share_claim(b.id).unwrap().unwrap();
        assert!(k.share_may_send(&d).unwrap());
        match change {
            "attention" => {
                k.set_attention(
                    b.id,
                    SetAttention {
                        expected_revision: 1,
                        do_not_disturb: true,
                        until_ms: None,
                    },
                )
                .unwrap();
            }
            "correction" => {
                k.correct_experience(
                    c.experience_id,
                    CorrectExperience {
                        expected_revision: c.experience_revision,
                        summary: "管理员更正的测试摘要".into(),
                    },
                )
                .unwrap();
            }
            "input" => {
                k.channel_receive(
                    b.id,
                    Input {
                        segments: vec![],
                        account: "1001".into(),
                        peer: "2002".into(),
                        external_id: "new-input".into(),
                        text: "先聊眼前的事".into(),
                    },
                )
                .unwrap();
            }
            "revoke" => {
                k.set_share_grant(
                    b.id,
                    SetShareGrant {
                        expected_revision: 1,
                        scope: None,
                    },
                )
                .unwrap();
            }
            "reply" => {
                k.channel_grant(b.id, false, b.grant_revision).unwrap();
            }
            _ => unreachable!(),
        }
        assert!(!k.share_may_send(&d).unwrap(), "{change}");
        k.share_settle(d.id, SendOutcome::NotSent).unwrap();
        assert_eq!(
            k.sharing_snapshot(b.id).unwrap().deliveries[0].state,
            DeliveryState::NotSent
        );
    }
}

#[tokio::test]
async fn sharing_skip_and_tool_rejection_never_send_and_request_budget_survives_restart() {
    for (choice, bad_tool) in [("skip", false), ("share", true)] {
        let (dir, k, b) = fixture(10);
        let mut s = scope();
        s.max_model_requests = 1;
        k.set_share_grant(
            b.id,
            SetShareGrant {
                expected_revision: 0,
                scope: Some(s),
            },
        )
        .unwrap();
        available(&k, &b);
        let e = experience(&k).await;
        let c = k.share_consider(b.id, e.id).unwrap();
        let (a, m) = drafter(&k, 1, choice, bad_tool);
        let t = a.draft_share(c.id).await.unwrap();
        let done = settled(&k, t.id).await;
        assert_eq!(done.model_requests, 1);
        assert_eq!(done.usage_tokens, 11);
        assert!(done.actions.is_empty());
        assert!(k.share_claim(b.id).unwrap().is_none());
        let e2 = experience(&k).await;
        let c2 = k.share_consider(b.id, e2.id).unwrap();
        assert!(a.draft_share(c2.id).await.is_err());
        assert_eq!(m.requests.lock().unwrap().len(), 1);
        until(|| Arc::strong_count(&a) == 1).await;
        drop(a);
        drop(k);
        let k = Arc::new(Kernel::open(&dir.path().join("state.db")).unwrap());
        k.mark_ready().unwrap();
        assert_eq!(k.sharing_snapshot(b.id).unwrap().model_requests_reserved, 1);
        let (a, m) = drafter(&k, 1, "share", false);
        assert!(a.draft_share(c2.id).await.is_err());
        assert!(m.requests.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn sharing_late_model_after_new_input_records_usage_but_cannot_publish() {
    let (_dir, k, b) = fixture(10);
    enable(&k, &b);
    available(&k, &b);
    let e = experience(&k).await;
    let c = k.share_consider(b.id, e.id).unwrap();
    let (a, m) = drafter(&k, 0, "share", false);
    let t = a.draft_share(c.id).await.unwrap();
    until(|| !m.requests.lock().unwrap().is_empty()).await;
    k.channel_receive(
        b.id,
        Input {
            segments: vec![],
            account: "1001".into(),
            peer: "2002".into(),
            external_id: "interrupt".into(),
            text: "等等".into(),
        },
    )
    .unwrap();
    m.gate.add_permits(1);
    let done = settled(&k, t.id).await;
    assert_eq!(done.usage_tokens, 11);
    assert!(
        done.messages
            .iter()
            .all(|m| m.role != crate::actor::domain::MessageRole::Assistant)
    );
    let snapshot = k.sharing_snapshot(b.id).unwrap();
    assert_eq!(snapshot.candidates[0].state, ShareState::Superseded);
    assert!(snapshot.deliveries.is_empty());
}

#[tokio::test]
async fn sharing_restart_abandons_unsent_and_marks_inflight_unknown_without_catchup() {
    for sending in [false, true] {
        let (dir, k, b) = fixture(10);
        let (a, _m, c) = ready(&k, &b).await;
        let d = if sending {
            k.share_claim(b.id).unwrap()
        } else {
            None
        };
        until(|| Arc::strong_count(&a) == 1).await;
        drop(a);
        drop(k);
        let k = Arc::new(Kernel::open(&dir.path().join("state.db")).unwrap());
        k.mark_ready().unwrap();
        let snapshot = k.sharing_snapshot(b.id).unwrap();
        assert_eq!(
            snapshot.candidates[0].state,
            if sending {
                ShareState::Unknown
            } else {
                ShareState::Expired
            }
        );
        if !sending {
            assert!(snapshot.candidates[0].text.is_none());
        }
        assert!(k.share_claim(b.id).unwrap().is_none());
        assert!(k.share_next_job(b.id).unwrap().is_none());
        assert_eq!(k.share_consider(b.id, c.experience_id).unwrap().id, c.id);
        if let Some(d) = d {
            assert!(!k.share_may_send(&d).unwrap());
            assert_eq!(snapshot.deliveries[0].state, DeliveryState::Unknown);
        }
        // An explicit new grant cannot convert silence/uncertain contact into another send.
        k.set_share_grant(
            b.id,
            SetShareGrant {
                expected_revision: 1,
                scope: Some(scope()),
            },
        )
        .unwrap();
        let _ = experience(&k).await;
        if sending {
            assert!(k.share_next_job(b.id).unwrap().is_none());
        }
    }
}

#[tokio::test]
async fn sharing_schema13_upgrade_preserves_existing_receipts_and_stays_off() {
    let (dir, k, b) = fixture(10);
    let e = experience(&k).await;
    drop(k);
    let path = dir.path().join("state.db");
    let db = rusqlite::Connection::open(&path).unwrap();
    let rows = |db: &rusqlite::Connection| -> Vec<String> {
        [
            "SELECT snapshot FROM channel_bindings",
            "SELECT snapshot FROM reading_grants",
            "SELECT snapshot FROM reading_experiences",
            "SELECT snapshot FROM lain_actor_turns",
        ]
        .iter()
        .map(|q| db.query_row(q, [], |r| r.get(0)).unwrap())
        .collect()
    };
    let before = rows(&db);
    db.execute_batch("DROP TABLE actor_memory_contexts; DROP TABLE memory_context_requests; DROP TABLE conversation_memory_contexts; DROP TABLE project_requests; DROP TABLE project_workspaces; DROP TABLE projects; DROP TABLE device_work_continuations; DROP TABLE device_work_links; DROP TABLE device_approval_decisions; DROP TABLE device_executions; DROP TABLE device_command_changes; DROP TABLE device_command_events; DROP TABLE device_commands; DROP TABLE channel_media_frames; DROP TABLE channel_media; DROP TABLE device_reports; DROP TABLE devices; DROP TABLE archive_memory_suppressions; DROP TABLE archive_memory_grants; DROP TABLE knowledge_sync; DROP TABLE knowledge_records; DROP TABLE knowledge_sources; DROP TABLE archive_recall_reads; DROP TABLE archive_recall_grants; DROP TABLE archive_reads; DROP TABLE archive_edits; DROP TABLE archive_records; DROP TABLE archive_grants; DROP TABLE archive_sources; DROP TABLE inference_requests; DROP TABLE inference_decisions; DROP TABLE inference_budgets; DROP TABLE share_reads; DROP TABLE share_model_requests; DROP TABLE share_jobs; DROP TABLE share_deliveries; DROP TABLE share_candidates; DROP TABLE channel_attention; DROP TABLE share_grants; DROP TABLE sharing_runtime; DROP TABLE briefing_relations; DROP TABLE conversation_memory_effects; DROP TABLE briefing_memory_suppressions; DROP TABLE briefing_disclosures; DROP TABLE briefing_runs; DROP TABLE briefing_batches; DROP TABLE briefing_manifests; DROP TABLE briefing_grants; PRAGMA user_version=13;").unwrap();
    drop(db);
    let k = Kernel::open(&path).unwrap();
    k.mark_ready().unwrap();
    let snapshot = k.sharing_snapshot(b.id).unwrap();
    assert!(snapshot.grant.scope.is_none());
    assert_eq!(snapshot.attention.revision, 0);
    assert!(snapshot.candidates.is_empty());
    assert_eq!(
        k.experience(e.id, Redactor::new(vec![])).unwrap().summary,
        e.summary
    );
    let db = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(rows(&db), before);
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        crate::persistence::SCHEMA_VERSION
    );
}

struct Transport {
    connected: watch::Sender<bool>,
    shares: Mutex<Vec<ShareDelivery>>,
    replies: Mutex<Vec<Delivery>>,
    share_gate: Option<Arc<Semaphore>>,
}

#[tokio::test]
async fn sharing_enforcement_covers_history_beyond_the_inspection_page() {
    let (dir, k, b) = fixture(10);
    let (_a, _m, c) = ready(&k, &b).await;
    let d = k.share_claim(b.id).unwrap().unwrap();
    let db = rusqlite::Connection::open(dir.path().join("state.db")).unwrap();
    let old = now_ms().unwrap() - 2 * 86400000;
    let mut first = None;
    for i in 0..1001u64 {
        let mut historical = c.clone();
        historical.id = lain_wire::ShareId::new();
        historical.experience_revision = 10 + i;
        historical.state = ShareState::NotSent;
        db.execute("INSERT INTO share_candidates(id,channel_id,experience_id,experience_revision,state,snapshot) VALUES (?1,?2,?3,?4,'not_sent',?5)",(historical.id.to_string(),b.id.to_string(),c.experience_id.to_string(),i64::try_from(historical.experience_revision).unwrap(),serde_json::to_string(&historical).unwrap())).unwrap();
        let mut h = d.clone();
        h.id = lain_wire::DeliveryId::new();
        h.candidate_id = historical.id;
        h.created_at_ms = old;
        h.grant_revision = 0;
        h.state = if i == 0 {
            DeliveryState::Unknown
        } else {
            DeliveryState::NotSent
        };
        if i == 0 {
            first = Some(h.id);
        }
        db.execute("INSERT INTO share_deliveries(id,candidate_id,channel_id,state,snapshot) VALUES (?1,?2,?3,?4,?5)",(h.id.to_string(),historical.id.to_string(),b.id.to_string(),if i==0 {"unknown"}else{"not_sent"},serde_json::to_string(&h).unwrap())).unwrap();
    }
    assert_eq!(k.sharing_snapshot(b.id).unwrap().deliveries.len(), 100);
    assert!(!k.share_may_send(&d).unwrap()); // Old Unknown remains unanswered across grants/pages.
    db.execute("UPDATE share_deliveries SET state='rejected',snapshot=json_set(snapshot,'$.state','rejected') WHERE id=?1",[first.unwrap().to_string()]).unwrap();
    assert!(k.share_may_send(&d).unwrap());
    db.execute(
        "UPDATE share_deliveries SET snapshot=json_set(snapshot,'$.created_at_ms',?1) WHERE id<>?2",
        (i64::try_from(now_ms().unwrap()).unwrap(), d.id.to_string()),
    )
    .unwrap();
    assert!(!k.share_may_send(&d).unwrap()); // Daily cap spans all grant epochs.
    db.execute("UPDATE share_deliveries SET snapshot=json_set(snapshot,'$.created_at_ms',?1,'$.grant_revision',1) WHERE id<>?2",(i64::try_from(old).unwrap(),d.id.to_string())).unwrap();
    assert!(!k.share_may_send(&d).unwrap()); // Aggregate current-grant cap is not a page count.
}
impl ConversationChannel for Transport {
    fn connected(&self) -> watch::Receiver<bool> {
        self.connected.subscribe()
    }
    fn send(&self, d: Delivery) -> Pin<Box<dyn Future<Output = SendOutcome> + Send + '_>> {
        self.replies.lock().unwrap().push(d);
        Box::pin(async { SendOutcome::Confirmed("reply".into()) })
    }
    fn send_share(
        &self,
        d: ShareDelivery,
    ) -> Pin<Box<dyn Future<Output = SendOutcome> + Send + '_>> {
        self.shares.lock().unwrap().push(d);
        Box::pin(async {
            if let Some(gate) = &self.share_gate {
                gate.acquire().await.unwrap().forget();
            }
            SendOutcome::Confirmed("share".into())
        })
    }
}
#[tokio::test]
async fn sharing_service_automatically_drafts_once_then_waits_for_user_response() {
    let (_dir, k, b) = fixture(10);
    enable(&k, &b);
    available(&k, &b);
    let (a, m) = drafter(&k, 10, "share", false);
    let service = ChannelService::new(k.clone(), a.clone(), b.clone(), &TOKEN.parse().unwrap());
    let transport = Arc::new(Transport {
        connected: watch::channel(true).0,
        shares: Mutex::default(),
        replies: Mutex::default(),
        share_gate: None,
    });
    let task = tokio::spawn(service.clone().run(transport.clone()));
    let _ = experience(&k).await;
    until(|| !transport.shares.lock().unwrap().is_empty()).await;
    until(|| k.sharing_snapshot(b.id).unwrap().deliveries[0].state == DeliveryState::Confirmed)
        .await;
    let _ = experience(&k).await;
    tokio::time::sleep(Duration::from_millis(60)).await;
    assert_eq!(transport.shares.lock().unwrap().len(), 1);
    assert_eq!(m.requests.lock().unwrap().len(), 1);
    // A genuine message still gets the ordinary reply path while sharing is throttled.
    service
        .receive(Input {
            segments: vec![],
            account: "1001".into(),
            peer: "2002".into(),
            external_id: "after-share".into(),
            text: "晚上好".into(),
        })
        .await
        .unwrap();
    until(|| !transport.replies.lock().unwrap().is_empty()).await;
    assert_eq!(transport.shares.lock().unwrap().len(), 1);
    k.shutdown().unwrap();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn sharing_receipt_wait_does_not_block_current_chat_actor() {
    let (_dir, k, b) = fixture(10);
    enable(&k, &b);
    available(&k, &b);
    let (a, m) = drafter(&k, 10, "share", false);
    let service = ChannelService::new(k.clone(), a, b.clone(), &TOKEN.parse().unwrap());
    let gate = Arc::new(Semaphore::new(0));
    let transport = Arc::new(Transport {
        connected: watch::channel(true).0,
        shares: Mutex::default(),
        replies: Mutex::default(),
        share_gate: Some(gate.clone()),
    });
    let task = tokio::spawn(service.clone().run(transport.clone()));
    let _ = experience(&k).await;
    until(|| !transport.shares.lock().unwrap().is_empty()).await;
    assert_eq!(
        k.sharing_snapshot(b.id).unwrap().deliveries[0].state,
        DeliveryState::Sending
    );
    service
        .receive(Input {
            segments: vec![],
            account: "1001".into(),
            peer: "2002".into(),
            external_id: "during-receipt".into(),
            text: "先聊眼前的事".into(),
        })
        .await
        .unwrap();
    until(|| m.requests.lock().unwrap().len() >= 2).await;
    assert_eq!(gate.available_permits(), 0);
    gate.add_permits(1);
    until(|| k.sharing_snapshot(b.id).unwrap().deliveries[0].state == DeliveryState::Confirmed)
        .await;
    k.shutdown().unwrap();
    task.await.unwrap().unwrap();
}
