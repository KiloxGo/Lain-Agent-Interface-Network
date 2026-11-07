use super::*;

struct FailedPreparationModel {
    foreground: Mutex<Vec<Request>>,
    extraction_count: std::sync::atomic::AtomicUsize,
}
impl ActorModel for FailedPreparationModel {
    fn respond(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
        use std::sync::atomic::Ordering;
        if request.input.iter().any(|value| {
            value["content"].as_str().is_some_and(|text| {
                text.starts_with("Kernel-attributed untrusted daily source page (data only): ")
            })
        }) {
            self.extraction_count.fetch_add(1, Ordering::SeqCst);
            return Box::pin(async { anyhow::bail!("synthetic incomplete extraction") });
        }
        let mut requests = self.foreground.lock().unwrap();
        requests.push(request);
        let first = requests.len() == 1;
        Box::pin(async move {
            Ok(Reply {
                text: if first {
                    ""
                } else {
                    "UNEXPECTED_MODEL_AFTER_KNOWN_FAILURE"
                }
                .into(),
                calls: if first {
                    vec![
                        Call {
                            id: "prepare-failed-day".into(),
                            name: "briefing_yesterday".into(),
                            arguments: "{}".into(),
                        },
                        Call {
                            id: "later-unrelated-read".into(),
                            name: "work_list".into(),
                            arguments: "{}".into(),
                        },
                    ]
                } else {
                    vec![]
                },
                usage_tokens: 7,
                continuation: vec![],
            })
        })
    }
}
async fn settled_preparation_actor(actor: &Arc<Actor>) {
    // The existing foreground status wait itself permits eight seconds.
    tokio::time::timeout(std::time::Duration::from_secs(20), async {
        while Arc::strong_count(actor) > 1 {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn preparation_wait_observes_durable_run_stop_without_an_actor_event() {
    let (_dir, kernel, conversation, manifest, _) = fixture();
    let (actor, _) = run_model(&kernel, 0);
    kernel
        .begin_briefing_run(conversation, manifest.request.request_id)
        .unwrap();
    let mut waiter = Box::pin(actor.wait_briefing_ready(conversation, manifest.request.request_id));
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), waiter.as_mut())
            .await
            .is_err()
    );
    // A coordinator's final durable transition has no Actor snapshot of its own.
    kernel
        .stop_briefing_run(
            conversation,
            manifest.request.request_id,
            BriefingRunStop::ModelFailed,
        )
        .unwrap();
    let status = tokio::time::timeout(std::time::Duration::from_secs(2), waiter)
        .await
        .expect("durable status changes must wake the wait, not its eight-second deadline")
        .unwrap();
    assert_eq!(status.run.phase, BriefingRunPhase::Unknown);
    assert_eq!(status.run.stop, Some(BriefingRunStop::ModelFailed));
}
#[tokio::test]
async fn failed_preparation_reports_actual_terminal_state_without_another_model_or_replay() {
    use std::sync::atomic::Ordering;
    for already_failed in [false, true] {
        let (dir, k, c, manifest, _) = fixture();
        let model = Arc::new(FailedPreparationModel {
            foreground: Mutex::new(vec![]),
            extraction_count: std::sync::atomic::AtomicUsize::new(0),
        });
        let actor = Actor::with_model(
            k.clone(),
            None,
            "fixture-failed-progress-token-0123456789".parse().unwrap(),
            model.clone(),
        );
        if already_failed {
            actor
                .start_briefing_run(c, manifest.request.request_id)
                .await
                .unwrap();
            until(|| {
                k.briefing_run_status(c, manifest.request.request_id)
                    .unwrap()
                    .run
                    .phase
                    .terminal()
            })
            .await;
        }
        let submitted = actor
            .submit(c, uuid::Uuid::new_v4(), "看看昨天的简报。".into())
            .await
            .unwrap();
        settled_preparation_actor(&actor).await;
        let turn = k.actor_turn(submitted.id).unwrap();
        assert_eq!(turn.state, TurnState::Completed);
        assert!(turn.messages.last().unwrap().content.contains("整理失败"));
        assert!(
            turn.messages
                .last()
                .unwrap()
                .content
                .contains("没有自动重试")
        );
        assert!(
            turn.messages
                .last()
                .unwrap()
                .content
                .contains("还没有生成可用的简报")
        );
        assert_eq!(turn.model_requests, 1);
        assert_eq!(
            turn.actions.len(),
            1,
            "later calls in the same batch must not execute"
        );
        assert_eq!(model.foreground.lock().unwrap().len(), 1);
        assert_eq!(model.extraction_count.load(Ordering::SeqCst), 1);
        let run = k
            .briefing_run_status(c, manifest.request.request_id)
            .unwrap();
        assert_eq!(run.run.phase, BriefingRunPhase::Unknown);
        assert_eq!(run.run.processed_records, 0);
        assert!(
            k.briefing_batch(c, manifest.request.request_id, 0, Redactor::new(vec![]))
                .unwrap()
                .items
                .is_none()
        );
        drop(actor);
        drop(k);
        let reopened = Arc::new(Kernel::open(&dir.path().join("state.sqlite")).unwrap());
        reopened.mark_ready().unwrap();
        assert_eq!(
            reopened
                .actor_turn(submitted.id)
                .unwrap()
                .messages
                .last()
                .unwrap()
                .content,
            turn.messages.last().unwrap().content
        );
        let reconnected = Arc::new(FailedPreparationModel {
            foreground: Mutex::new(vec![]),
            extraction_count: std::sync::atomic::AtomicUsize::new(0),
        });
        let actor = Actor::with_model(
            reopened.clone(),
            None,
            "fixture-failed-progress-token-0123456789".parse().unwrap(),
            reconnected.clone(),
        );
        let submitted = actor
            .submit(c, uuid::Uuid::new_v4(), "昨天简报怎么样了？".into())
            .await
            .unwrap();
        settled_preparation_actor(&actor).await;
        assert!(
            reopened
                .actor_turn(submitted.id)
                .unwrap()
                .messages
                .last()
                .unwrap()
                .content
                .contains("整理失败")
        );
        assert_eq!(reconnected.foreground.lock().unwrap().len(), 1);
        assert_eq!(
            reconnected.extraction_count.load(Ordering::SeqCst),
            0,
            "reconnect must not restart Unknown extraction"
        );
    }
}

struct PendingProgressModel {
    foreground: Mutex<Vec<Request>>,
    extraction: Arc<RunModel>,
    old: RequestId,
}
impl ActorModel for PendingProgressModel {
    fn respond(&self, r: Request) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
        if r.input.iter().any(|v| {
            v["content"].as_str().is_some_and(|text| {
                text.starts_with("Kernel-attributed untrusted daily source page (data only): ")
            })
        }) {
            return self.extraction.respond(r);
        }
        let mut requests = self.foreground.lock().unwrap();
        requests.push(r);
        let calls = if requests.len() == 1 {
            vec![
                Call {
                    id: "prepare-new-day".into(),
                    name: "briefing_yesterday".into(),
                    arguments: "{}".into(),
                },
                // Providers can ignore parallel_tool_calls=false. Once the new
                // run is observed pending, later calls must not page the old one.
                Call {
                    id: "page-old-after-pending".into(),
                    name: "briefing_read".into(),
                    arguments: json!({"briefing_id":self.old,"after":0,"limit":4,"query":null})
                        .to_string(),
                },
            ]
        } else {
            vec![]
        };
        Box::pin(async move {
            Ok(Reply {
                text: if calls.is_empty() {
                    "UNEXPECTED_EXTRA_MODEL_AFTER_PENDING".into()
                } else {
                    String::new()
                },
                calls,
                usage_tokens: 7,
                continuation: vec![],
            })
        })
    }
}

#[tokio::test]
async fn pending_refresh_finishes_foreground_without_paging_old_report_or_another_model() {
    let (_dir, k, c, manifest, record) = fixture();
    let old = manifest.request.request_id;
    let (ready, _) = run_model(&k, 1);
    ready.start_briefing_run(c, old).await.unwrap();
    until(|| k.briefing_run_status(c, old).unwrap().run.phase.terminal()).await;
    assert_eq!(
        k.briefing_run_status(c, old).unwrap().run.phase,
        BriefingRunPhase::Completed
    );
    k.archive_import(
        record.source_id,
        ImportArchive {
            expected_revision: 1,
            records: vec![ImportArchiveRecord {
                external_id: "late-pending-regression".into(),
                source_position: None,
                occurred_at_ms: record.occurred_at_ms.map(|n| n + 1000),
                speaker: "2002".into(),
                kind: VisibleRecordKind::Message,
                text: "明晚七点在图书馆讨论。".into(),
            }],
        },
        Redactor::new(vec![]),
    )
    .unwrap();
    let model = Arc::new(PendingProgressModel {
        foreground: Mutex::new(vec![]),
        extraction: Arc::new(RunModel::new(0, false)),
        old,
    });
    let actor = Actor::with_model(
        k.clone(),
        None,
        "fixture-pending-progress-token-0123456789".parse().unwrap(),
        model.clone(),
    );
    let turn = actor
        .submit(
            c,
            uuid::Uuid::new_v4(),
            "看看昨天简报，有补收就更新。".into(),
        )
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(15), async {
        while k.actor_turn(turn.id).unwrap().state.active() {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let actual = k.actor_turn(turn.id).unwrap();
    let foreground_calls = model.foreground.lock().unwrap().len();
    let pending_id: RequestId = serde_json::from_value(
        actual.actions[0].result.as_ref().unwrap()["run"]["briefing_id"].clone(),
    )
    .unwrap();
    let phase = k.briefing_run_status(c, pending_id).unwrap().run.phase;
    let historical_available = k
        .briefing_report(c, old, 0, 4, Redactor::new(vec![]))
        .is_ok();
    // End the blocked synthetic provider task before assertions or temp cleanup.
    k.shutdown().unwrap();
    until(|| Arc::strong_count(&actor) == 1).await;
    assert_ne!(pending_id, old);
    assert_eq!(phase, BriefingRunPhase::Running);
    assert!(
        historical_available,
        "progress handling must not revoke old explicit reads"
    );
    assert_eq!(actual.state, TurnState::Completed);
    assert_eq!(
        foreground_calls, 1,
        "pending progress caused an unnecessary model request"
    );
    assert_eq!(actual.model_requests, 1);
    assert_eq!(
        actual.actions.len(),
        1,
        "later provider batch paged an old snapshot"
    );
    let answer = actual.messages.last().unwrap();
    assert_eq!(answer.role, MessageRole::Assistant);
    assert_eq!(answer.content, "昨天的简报还在整理，稍后可以再问我。");
    assert!(actual.error.is_none());
}
