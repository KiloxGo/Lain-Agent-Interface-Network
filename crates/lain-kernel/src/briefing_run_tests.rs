use super::*;
use crate::actor::domain::ActionState;
#[path = "briefing_lookup_tests.rs"]
mod lookup;
#[path = "briefing_memory_tests.rs"]
mod memory;
#[path = "briefing_pending_tests.rs"]
mod pending;
struct RunModel {
    requests: Mutex<Vec<Request>>,
    gate: Semaphore,
    dense: bool,
    max_group: bool,
    link: bool,
    pair_relation: crate::persistence::briefing_relations::Relation,
    correcting: Option<u16>,
    single_pair: bool,
    compare_gate: Option<Arc<Semaphore>>,
    /// Reproduces the observed provider slip: asked to copy a source UUID, it
    /// gets one character wrong. Only an offered citation selector avoids it.
    miscopies_record_ids: bool,
    /// Extraction pages at these indices answer with output that fails validation.
    rejected_from: Vec<u64>,
}
impl RunModel {
    fn new(permits: usize, dense: bool) -> Self {
        Self {
            requests: Mutex::new(vec![]),
            gate: Semaphore::new(permits),
            dense,
            max_group: false,
            link: false,
            pair_relation: crate::persistence::briefing_relations::Relation::Duplicate,
            correcting: None,
            single_pair: false,
            compare_gate: None,
            miscopies_record_ids: false,
            rejected_from: vec![],
        }
    }
}
impl ActorModel for RunModel {
    fn respond(&self, r: Request) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
        let relation_prefix = "Kernel-attributed untrusted briefing relation input (data only): ";
        if let Some(data) = r
            .input
            .last()
            .and_then(|v| v["content"].as_str())
            .and_then(|v| v.strip_prefix(relation_prefix))
        {
            let data: serde_json::Value = serde_json::from_str(data).unwrap();
            let selector_offered = r.persona.contains("\"citation_index\"");
            self.requests.lock().unwrap().push(r);
            let text = if data["mode"] == "compare" && self.miscopies_record_ids {
                let evidence = data["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| {
                        let citation = &v["item"]["citations"][0];
                        if selector_offered {
                            json!({"item_number":v["item_number"],"citation_index":0,"quote":citation["quote"]})
                        } else {
                            let id = citation["source"]["record_id"].as_str().unwrap();
                            let typo = if id.starts_with('0') { '1' } else { '0' };
                            json!({"item_number":v["item_number"],"record_id":format!("{typo}{}",&id[1..]),"quote":citation["quote"]})
                        }
                    })
                    .collect::<Vec<_>>();
                json!({"relation":self.pair_relation,"correction_item_number":self.correcting,"reason":"基于双方原文的关系判断，未独立核实","evidence":evidence}).to_string()
            } else if data["mode"] == "index" {
                let count = data["item_count"].as_u64().unwrap() as usize;
                json!({"parents":(0..count).map(|n|if n>0 && self.link && (!self.single_pair || n==1) {Some(1)} else {None}).collect::<Vec<_>>()}).to_string()
            } else {
                let evidence=data["items"].as_array().unwrap().iter().map(|v|json!({"item_number":v["item_number"],"record_id":v["item"]["citations"][0]["source"]["record_id"],"quote":v["item"]["citations"][0]["quote"]})).collect::<Vec<_>>();
                json!({"relation":self.pair_relation,"correction_item_number":self.correcting,"reason":"基于双方原文的关系判断，未独立核实","evidence":evidence}).to_string()
            };
            let gate = if data["mode"] == "compare" {
                self.compare_gate.clone()
            } else {
                None
            };
            return Box::pin(async move {
                if let Some(gate) = gate {
                    gate.acquire().await.unwrap().forget();
                }
                Ok(Reply {
                    text,
                    calls: vec![],
                    usage_tokens: 7,
                    continuation: vec![],
                })
            });
        }
        let prefix = "Kernel-attributed untrusted daily source page (data only): ";
        let data: serde_json::Value = serde_json::from_str(
            r.input.last().unwrap()["content"]
                .as_str()
                .unwrap()
                .strip_prefix(prefix)
                .unwrap(),
        )
        .unwrap();
        self.requests.lock().unwrap().push(r);
        let rejected = data["page"]["from_index"]
            .as_u64()
            .is_some_and(|index| self.rejected_from.contains(&index));
        Box::pin(async move {
            self.gate.acquire().await.unwrap().forget();
            if rejected {
                return Ok(Reply {
                    text: "not the required JSON".into(),
                    usage_tokens: 7,
                    calls: vec![],
                    continuation: vec![],
                });
            }
            let record = &data["page"]["records"][0];
            let mut output = json!({"important_items_omitted":false,"items":[{"title":"图书馆讨论","summary":"有人通知讨论，未确认用户参加。","category":"schedule","time_text":"明晚七点","uncertainty":"未独立核实","citations":[{"record_id":record["id"],"quote":"明晚七点在图书馆讨论。"}]}]});
            if let Some(raw) = record["text"].as_str()
                && (raw.starts_with("更正：") || raw.starts_with("冲突："))
            {
                output["items"][0]["summary"] = raw.into();
                output["items"][0]["citations"][0]["quote"] = raw.into();
            }
            if let Some(raw) = record["text"].as_str()
                && raw.starts_with("定位测试：")
            {
                output["items"][0]["title"] = "一条补发通知".into();
                output["items"][0]["summary"] = "另有一条领取安排，详情见实际引文。".into();
                output["items"][0]["time_text"] = serde_json::Value::Null;
                output["items"][0]["citations"][0]["quote"] = raw.into();
            }
            if self.dense {
                let item = &mut output["items"][0];
                item["title"] = "t".repeat(160).into();
                item["summary"] = "s".repeat(768).into();
                item["uncertainty"] = "u".repeat(512).into();
                item["citations"] = vec![item["citations"][0].clone(); 8].into();
            }
            if self.max_group {
                let small = json!({"title":"小项","summary":"通知","category":"other","time_text":null,"uncertainty":"待核实","citations":[{"record_id":record["id"],"quote":"明晚"}]});
                output["items"] = vec![small; 4].into();
                if record["external_id"] == "synthetic-notice-max" {
                    let item = &mut output["items"][0];
                    item["title"] = "\\".repeat(160).into();
                    item["summary"] = "\\".repeat(768).into();
                    item["uncertainty"] = "\\".repeat(512).into();
                    item["time_text"] = "\\".repeat(160).into();
                    item["citations"] =
                        vec![json!({"record_id":record["id"],"quote":"\\".repeat(160)}); 8].into();
                }
            }
            let text = output.to_string();
            assert!(
                text.len() <= 8192,
                "fixture must pass the actual extraction output bound: {} bytes",
                text.len()
            );
            Ok(Reply {
                text,
                usage_tokens: 7,
                calls: vec![],
                continuation: vec![],
            })
        })
    }
}
fn run_model(k: &Arc<Kernel>, permits: usize) -> (Arc<Actor>, Arc<RunModel>) {
    run_model_detail(k, permits, false)
}
fn run_model_detail(k: &Arc<Kernel>, permits: usize, dense: bool) -> (Arc<Actor>, Arc<RunModel>) {
    let m = Arc::new(RunModel::new(permits, dense));
    (
        Actor::with_model(
            k.clone(),
            None,
            "fixture-briefing-run-token-0123456789".parse().unwrap(),
            m.clone(),
        ),
        m,
    )
}

#[tokio::test]
async fn one_rejected_batch_is_skipped_and_reported_without_a_retry() {
    let (_dir, k, c, m) = many();
    let id = m.request.request_id;
    let mut model = RunModel::new(5, false);
    model.rejected_from = vec![4];
    let model = Arc::new(model);
    let actor = Actor::with_model(
        k.clone(),
        None,
        "fixture-briefing-run-token-0123456789".parse().unwrap(),
        model.clone(),
    );
    actor.start_briefing_run(c, id).await.unwrap();
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    let run = k.briefing_run_status(c, id).unwrap().run;
    assert_eq!(run.phase, BriefingRunPhase::Completed);
    assert_eq!((run.skipped_records, run.skipped_batches), (4, 1));
    assert_eq!(
        run.processed_records, run.selected_records,
        "the rest still ran"
    );
    let extraction = model
        .requests
        .lock()
        .unwrap()
        .iter()
        .filter(|r| {
            r.input.last().unwrap()["content"]
                .as_str()
                .is_some_and(|t| t.contains("from_index\":4,"))
        })
        .count();
    assert_eq!(extraction, 1, "a rejected batch is not retried");
    let report = k
        .briefing_report(c, id, 0, 4, Redactor::new(vec![]))
        .unwrap();
    assert_eq!(report["status"]["run"]["skipped_records"], 4);
}

#[tokio::test]
async fn too_many_rejected_batches_stop_the_run() {
    let (_dir, k, c, m) = many();
    let id = m.request.request_id;
    let mut model = RunModel::new(9, false);
    model.rejected_from = (0..64).map(|n| n * 4).collect();
    let actor = Actor::with_model(
        k.clone(),
        None,
        "fixture-briefing-run-token-0123456789".parse().unwrap(),
        Arc::new(model),
    );
    actor.start_briefing_run(c, id).await.unwrap();
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    let run = k.briefing_run_status(c, id).unwrap().run;
    assert_eq!(run.phase, BriefingRunPhase::Unknown);
    assert_eq!(run.stop, Some(BriefingRunStop::ModelFailed));
    assert_eq!(run.completed_batches, 0);
}

#[tokio::test]
async fn comparison_instructions_let_a_model_cite_without_transcribing_source_uuids() {
    let (_dir, k, c, m) = many();
    let id = m.request.request_id;
    let mut model = RunModel::new(5, false);
    model.link = true;
    model.single_pair = true;
    model.miscopies_record_ids = true;
    let actor = Actor::with_model(
        k.clone(),
        None,
        "fixture-relation-token-0123456789012345".parse().unwrap(),
        Arc::new(model),
    );
    actor.start_briefing_run(c, id).await.unwrap();
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    let status = k.briefing_run_status(c, id).unwrap().run;
    assert_eq!(status.phase, BriefingRunPhase::Completed);
    let report = k
        .briefing_report(c, id, 0, 4, Redactor::new(vec![]))
        .unwrap();
    assert_eq!(report["reconciliation"]["applied"], true);
    assert_eq!(report["total_display_groups"], 4);
    assert_eq!(report["items"][0]["members"], json!([1, 2]));
}

#[tokio::test]
async fn cross_batch_duplicates_keep_one_stable_group_and_every_original_member() {
    let (_dir, k, c, m) = many();
    let id = m.request.request_id;
    let mut model = RunModel::new(5, false);
    model.link = true;
    let model = Arc::new(model);
    let actor = Actor::with_model(
        k.clone(),
        None,
        "fixture-briefing-run-token-0123456789".parse().unwrap(),
        model,
    );
    actor.start_briefing_run(c, id).await.unwrap();
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    let report = k
        .briefing_report(c, id, 0, 4, Redactor::new(vec![]))
        .unwrap();
    assert_eq!(
        report["total_display_groups"], 1,
        "repeated notices across five extraction batches must be grouped"
    );
    let rows = report["items"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["item_number"], 1);
    assert_eq!(rows[0]["members"].as_array().unwrap().len(), 5);
    assert_eq!(rows[0]["members"][4], 5);
    assert_eq!(
        report["total_generated_items"], 5,
        "original identities are preserved"
    );
    let (save, _) = k
        .actor_submit(
            c,
            uuid::Uuid::new_v4(),
            "整组讨论通知记一下，不代表参加。".into(),
        )
        .unwrap();
    let action = ActionId::new();
    k.actor_update(save.id, |t| {
        t.actions.push(crate::actor::domain::Action {
            id: action,
            call_id: "remember-group".into(),
            tool: crate::actor::tools::Tool::BriefingRemember(RememberBriefing {
                briefing_id: id,
                item_number: 1,
                include_related: true,
            }),
            state: ActionState::Proposed,
            result: None,
        });
        t.state = TurnState::PendingConfirmation;
        Ok(())
    })
    .unwrap();
    let preview = k.actor_view(save.id).unwrap();
    let candidate = preview
        .actions
        .iter()
        .find_map(|a| a.memory_review.as_ref())
        .unwrap();
    let lain_wire::memory::MemorySource::BriefingSelection { evidence: e } =
        &candidate.provenance.source
    else {
        panic!()
    };
    assert_eq!(
        e.reconciliation
            .as_ref()
            .expect("group saves need complete reconciliation lineage")
            .member_numbers,
        vec![1, 2, 3, 4, 5]
    );
    assert!(candidate.draft.content.contains("所选来源引文"));
    for member in 1..=5 {
        assert!(
            candidate
                .draft
                .content
                .contains(&format!("原编号 {member} ·")),
            "a fitting group addition must include every selected member"
        );
    }
    actor.decide(save.id, action, true).await.unwrap();
    until(|| !k.actor_turn(save.id).unwrap().state.active()).await;
    let saved = k
        .memory_candidate(lain_wire::CandidateId(action.0))
        .unwrap()
        .memory_id
        .unwrap();
    let (recall, _) = k
        .actor_submit(c, uuid::Uuid::new_v4(), "我让你记住的那组通知呢？".into())
        .unwrap();
    let inspect = crate::actor::tools::Tool::MemoryInspect(crate::actor::tools::MemoryArg {
        memory_id: saved,
        scope: lain_wire::memory::MemoryScope::Conversation { conversation_id: c },
    });
    let answer = crate::actor::tools::read(&k, None, recall.id, &inspect)
        .await
        .unwrap();
    assert_eq!(answer["record"]["id"], json!(saved));
    assert_eq!(
        answer["briefing_recall_semantics"]["grouped_memory_ids"],
        json!([saved]),
        "a grouped recall must distinguish historical batch limitations from current availability"
    );
    let listing = crate::actor::tools::Tool::MemoryList(lain_wire::memory::MemoryQuery {
        scope: lain_wire::memory::MemoryScope::Conversation { conversation_id: c },
        kind: None,
        status: None,
        text: None,
        after: 0,
        limit: 10,
    });
    let page = crate::actor::tools::read(&k, None, recall.id, &listing)
        .await
        .unwrap();
    assert_eq!(
        page["briefing_recall_semantics"],
        answer["briefing_recall_semantics"]
    );
    assert_eq!(
        k.memory(saved).unwrap().draft.content,
        answer["record"]["content"],
        "interpretation metadata must not rewrite the reviewed memory"
    );
    let foreign = k.create_conversation().unwrap().id;
    let (other, _) = k
        .actor_submit(foreign, uuid::Uuid::new_v4(), "读另一会话".into())
        .unwrap();
    assert!(
        crate::actor::tools::read(&k, None, other.id, &inspect)
            .await
            .is_err()
    );
    k.delete_memory(saved, 1).unwrap();
    assert!(
        crate::actor::tools::read(&k, None, recall.id, &inspect)
            .await
            .is_err(),
        "annotations cannot restore a forgotten source selection"
    );
    let db = rusqlite::Connection::open(_dir.path().join("state.sqlite")).unwrap();
    let suppressed: u32 = db
        .query_row(
            "SELECT count(*) FROM briefing_memory_suppressions WHERE conversation_id=?1",
            [c.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        suppressed, 17,
        "forget must suppress inputs from every grouped extraction, not just the primary batch"
    );
}

#[tokio::test]
async fn correction_direction_uses_the_actual_source_not_original_number_order() {
    use crate::persistence::briefing_relations::Relation;
    let (_dir, k, c, m) = many();
    k.archive_correct(
        m.records[0].record_id,
        CorrectArchiveRecord {
            expected_revision: m.records[0].revision,
            text: "更正：明晚七点在图书馆讨论。地点改到教学楼101室，时间不变。".into(),
        },
        Redactor::new(vec![]),
    )
    .unwrap();
    let expiry = now_ms().unwrap() + 60000;
    let db = rusqlite::Connection::open(_dir.path().join("state.sqlite")).unwrap();
    db.execute("UPDATE archive_records SET retain_until_ms=?2,snapshot=json_set(snapshot,'$.retain_until_ms',?2) WHERE id=?1",(m.records[16].record_id.to_string(),i64::try_from(expiry).unwrap())).unwrap();
    let m = k
        .briefing_start(
            c,
            StartBriefing {
                request_id: RequestId::new(),
                grant_revision: 1,
                day: BriefingDay::Yesterday,
            },
        )
        .unwrap();
    let id = m.request.request_id;
    let mut model = RunModel::new(5, false);
    model.link = true;
    model.single_pair = true;
    model.pair_relation = Relation::ReportedCorrection;
    model.correcting = Some(1);
    let actor = Actor::with_model(
        k.clone(),
        None,
        "fixture-relation-token-0123456789012345".parse().unwrap(),
        Arc::new(model),
    );
    actor.start_briefing_run(c, id).await.unwrap();
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    let report = k
        .briefing_report(c, id, 0, 4, Redactor::new(vec![]))
        .unwrap();
    assert_eq!(report["total_display_groups"], 4);
    assert_eq!(report["items"][0]["relation"], "reported_correction");
    assert_eq!(report["items"][1]["relation"], "related_original");
    assert_eq!(report["items"][1]["reported_correction_item_number"], 1);
    assert_eq!(
        report["items"][0]["related_statement_numbers"],
        json!([1, 2])
    );
    assert_eq!(report["items"][0]["members"], json!([1, 2]));
    assert_eq!(
        k.briefing_report(c, id, 4, 4, Redactor::new(vec![]))
            .unwrap()["items"][0]["item_number"],
        5
    );
    let (save, _) = k
        .actor_submit(
            c,
            uuid::Uuid::new_v4(),
            "把整组更正和相关原通知记下来，不代表参加。".into(),
        )
        .unwrap();
    let action = ActionId::new();
    k.actor_update(save.id, |t| {
        t.actions.push(crate::actor::domain::Action {
            id: action,
            call_id: "remember-correction-group".into(),
            tool: crate::actor::tools::Tool::BriefingRemember(RememberBriefing {
                briefing_id: id,
                item_number: 1,
                include_related: true,
            }),
            state: ActionState::Proposed,
            result: None,
        });
        t.state = TurnState::PendingConfirmation;
        Ok(())
    })
    .unwrap();
    let view = k.actor_view(save.id).unwrap();
    let draft = view
        .actions
        .iter()
        .find_map(|a| a.memory_review.as_ref())
        .unwrap();
    let lain_wire::memory::MemorySource::BriefingSelection { evidence: e } =
        &draft.provenance.source
    else {
        panic!()
    };
    assert_eq!(
        e.expires_at_ms, expiry,
        "even uncited discovery inputs bound the grouped memory lifetime"
    );
    assert!(
        draft.draft.content.contains("报告更正")
            && draft.draft.content.contains("教学楼101")
            && draft.draft.content.contains("相关原通知")
    );
    actor.decide(save.id, action, true).await.unwrap();
    until(|| !k.actor_turn(save.id).unwrap().state.active()).await;
    let saved = k
        .memory_candidate(lain_wire::CandidateId(action.0))
        .unwrap()
        .memory_id
        .unwrap();
    assert!(k.memory(saved).is_ok());
    k.archive_correct(
        m.records[16].record_id,
        CorrectArchiveRecord {
            expected_revision: m.records[16].revision,
            text: "与所选两条不相同的发现输入被更正".into(),
        },
        Redactor::new(vec![]),
    )
    .unwrap();
    assert!(
        k.memory(saved).is_err(),
        "an outside-group discovery input change must invalidate its influenced relation result"
    );
    k.delete_memory(saved, 1).unwrap();
    let count: u32 = db
        .query_row(
            "SELECT count(*) FROM briefing_memory_suppressions WHERE conversation_id=?1",
            [c.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        count, 8,
        "forget suppresses actual member extraction inputs, not unrelated raw archives"
    );
    assert!(
        k.archive_record(m.records[16].record_id)
            .unwrap()
            .text
            .is_some()
    );
}

#[tokio::test]
async fn source_withdrawal_during_pair_judgment_blocks_late_merge_and_never_replays() {
    let (_dir, k, c, m) = many();
    let id = m.request.request_id;
    let gate = Arc::new(Semaphore::new(0));
    let mut model = RunModel::new(5, false);
    model.link = true;
    model.single_pair = true;
    model.compare_gate = Some(gate.clone());
    let model = Arc::new(model);
    let actor = Actor::with_model(
        k.clone(),
        None,
        "fixture-relation-token-0123456789012345".parse().unwrap(),
        model.clone(),
    );
    actor.start_briefing_run(c, id).await.unwrap();
    until(|| model.requests.lock().unwrap().len() == 7).await;
    assert_eq!(
        k.briefing_run_status(c, id).unwrap().run.phase,
        BriefingRunPhase::Reconciling
    );
    k.briefing_set(
        c,
        SetBriefingGrant {
            request_id: RequestId::new(),
            expected_revision: 1,
            scope: None,
        },
    )
    .unwrap();
    gate.add_permits(1);
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    assert_eq!(
        k.briefing_run_status(c, id).unwrap().run.phase,
        BriefingRunPhase::SourceUnavailable
    );
    assert!(
        k.briefing_report(c, id, 0, 4, Redactor::new(vec![]))
            .is_err()
    );
    actor.start_briefing_run(c, id).await.unwrap();
    assert_eq!(
        model.requests.lock().unwrap().len(),
        7,
        "revoked/unknown jobs cannot replay on exact start"
    );
}

#[tokio::test]
async fn restart_during_pair_judgment_preserves_unknown_without_replaying_relations() {
    let (dir, k, c, m) = many();
    let id = m.request.request_id;
    let mut model = RunModel::new(5, false);
    model.link = true;
    model.single_pair = true;
    model.compare_gate = Some(Arc::new(Semaphore::new(0)));
    let model = Arc::new(model);
    let actor = Actor::with_model(
        k.clone(),
        None,
        "fixture-relation-token-0123456789012345".parse().unwrap(),
        model.clone(),
    );
    actor.start_briefing_run(c, id).await.unwrap();
    until(|| model.requests.lock().unwrap().len() == 7).await;
    k.shutdown().unwrap();
    until(|| Arc::strong_count(&actor) == 1).await;
    drop(actor);
    drop(k);
    let k = Arc::new(Kernel::open(&dir.path().join("state.sqlite")).unwrap());
    k.mark_ready().unwrap();
    let (actor, unused) = run_model(&k, 5);
    assert_eq!(
        actor.start_briefing_run(c, id).await.unwrap().run.phase,
        BriefingRunPhase::Unknown
    );
    assert!(unused.requests.lock().unwrap().is_empty());
    let report = k
        .briefing_report(c, id, 0, 4, Redactor::new(vec![]))
        .unwrap();
    assert_eq!(report["reconciliation"]["applied"], false);
    assert_eq!(
        report["total_display_groups"], 5,
        "unfinished judgments must not hide original notices"
    );
    assert_eq!(k.inference_snapshot().unwrap().reserved_requests, 7);
}

struct Saver {
    briefing: RequestId,
    calls: Mutex<u32>,
}
impl ActorModel for Saver {
    fn respond(&self, _r: Request) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
        let mut calls = self.calls.lock().unwrap();
        let step = *calls;
        *calls += 1;
        assert!(
            step < 2,
            "an unavailable group must not trigger silent model narrowing/retry"
        );
        let (name, args) = if step == 0 {
            (
                "briefing_read",
                json!({"briefing_id":self.briefing,"after":0,"limit":4}),
            )
        } else {
            (
                "briefing_remember",
                json!({"briefing_id":self.briefing,"item_number":1,"include_related":true}),
            )
        };
        Box::pin(async move {
            Ok(Reply {
                text: "".into(),
                calls: vec![Call {
                    id: format!("group-save-{step}"),
                    name: name.into(),
                    arguments: args.to_string(),
                }],
                usage_tokens: 7,
                continuation: vec![],
            })
        })
    }
}

#[tokio::test]
async fn oversized_group_returns_a_clear_no_write_notice_without_silently_narrowing() {
    use crate::persistence::briefing_relations::Relation;
    let (_dir, k, c, m) = many();
    let id = m.request.request_id;
    let mut model = RunModel::new(5, true);
    model.link = true;
    model.pair_relation = Relation::Conflict;
    let extractor = Actor::with_model(
        k.clone(),
        None,
        "fixture-group-token-0123456789012345".parse().unwrap(),
        Arc::new(model),
    );
    extractor.start_briefing_run(c, id).await.unwrap();
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    let model = Arc::new(Saver {
        briefing: id,
        calls: Mutex::new(0),
    });
    let actor = Actor::with_model(
        k.clone(),
        None,
        "fixture-group-token-0123456789012345".parse().unwrap(),
        model.clone(),
    );
    let t = actor
        .submit(c, uuid::Uuid::new_v4(), "把第一组事项完整记下来。".into())
        .await
        .unwrap();
    until(|| !k.actor_turn(t.id).unwrap().state.active()).await;
    let t = k.actor_view(t.id).unwrap();
    assert_eq!(t.state, lain_wire::ActorTurnState::Completed);
    assert!(t.messages.iter().any(|m| m.role == MessageRole::Assistant
        && m.content.contains("单条记忆长度上限")
        && m.content.contains("尚未保存")));
    assert!(t.reviewed_memory_receipts.is_empty());
    assert_eq!(*model.calls.lock().unwrap(), 2);
}

#[tokio::test]
async fn pending_group_returns_not_saved_instead_of_an_unknown_mutation() {
    let (_dir, k, c, m) = many();
    let id = m.request.request_id;
    let gate = Arc::new(Semaphore::new(0));
    let mut pending = RunModel::new(5, false);
    pending.link = true;
    pending.single_pair = true;
    pending.compare_gate = Some(gate.clone());
    let pending = Arc::new(pending);
    let extractor = Actor::with_model(
        k.clone(),
        None,
        "fixture-group-token-0123456789012345".parse().unwrap(),
        pending.clone(),
    );
    extractor.start_briefing_run(c, id).await.unwrap();
    until(|| pending.requests.lock().unwrap().len() == 7).await;
    let model = Arc::new(Saver {
        briefing: id,
        calls: Mutex::new(0),
    });
    let actor = Actor::with_model(
        k.clone(),
        None,
        "fixture-group-token-0123456789012345".parse().unwrap(),
        model.clone(),
    );
    let t = actor
        .submit(c, uuid::Uuid::new_v4(), "把第一组完整记下来。".into())
        .await
        .unwrap();
    until(|| !k.actor_turn(t.id).unwrap().state.active()).await;
    let view = k.actor_view(t.id).unwrap();
    gate.add_permits(1);
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    until(|| Arc::strong_count(&extractor) == 1).await;
    assert_eq!(view.state, lain_wire::ActorTurnState::Completed);
    assert!(
        view.messages
            .iter()
            .any(|m| m.role == MessageRole::Assistant
                && m.content.contains("整组关联尚未完成")
                && m.content.contains("尚未保存"))
    );
    assert!(view.reviewed_memory_receipts.is_empty());
    assert_eq!(*model.calls.lock().unwrap(), 2);
}

#[tokio::test]
async fn report_has_readable_attribution_and_byte_bounded_pages_without_skipping_items() {
    let (_dir, k, c, m) = many();
    let id = m.request.request_id;
    let (a, _) = run_model_detail(&k, 5, true);
    a.start_briefing_run(c, id).await.unwrap();
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    let mut after = 0;
    let mut numbers = vec![];
    let mut limited = false;
    loop {
        let report = k
            .briefing_report(c, id, after, 4, Redactor::new(vec![]))
            .unwrap();
        assert!(serde_json::to_vec(&report).unwrap().len() <= 12 * 1024);
        let rows = report["items"].as_array().unwrap();
        assert!(!rows.is_empty());
        for row in rows {
            assert_eq!(row["citations"][0]["source_label"], "QQ群 3003");
            assert!(
                row["citations"][0]["source_local_time"]
                    .as_str()
                    .is_some_and(|s| s.ends_with("00:00:10 UTC+08:00"))
            );
            assert_eq!(
                row["citations"][0]["time_basis"],
                "source_reported_occurrence"
            );
            numbers.push(row["item_number"].as_u64().unwrap());
        }
        limited |= report["page_limited_by_bytes"] == true;
        if !report["has_more"].as_bool().unwrap() {
            break;
        }
        let next = report["next_after"].as_u64().unwrap() as u16;
        assert!(next > after);
        after = next;
    }
    assert!(limited);
    assert_eq!(numbers, vec![1, 2, 3, 4, 5]);
}
fn many() -> (
    tempfile::TempDir,
    Arc<Kernel>,
    ConversationId,
    BriefingManifest,
) {
    let (dir, k, c, _, record) = fixture();
    let records = (0..16)
        .map(|i| ImportArchiveRecord {
            external_id: format!("synthetic-extra-{i}"),
            source_position: None,
            occurred_at_ms: record.occurred_at_ms,
            speaker: "2002".into(),
            kind: VisibleRecordKind::Message,
            text: "明晚七点在图书馆讨论。".into(),
        })
        .collect();
    k.archive_import(
        record.source_id,
        ImportArchive {
            expected_revision: 1,
            records,
        },
        Redactor::new(vec![]),
    )
    .unwrap();
    let m = k
        .briefing_start(
            c,
            StartBriefing {
                request_id: RequestId::new(),
                grant_revision: 1,
                day: BriefingDay::Yesterday,
            },
        )
        .unwrap();
    (dir, k, c, m)
}
#[tokio::test]
async fn coordinator_processes_entire_manifest_once_and_exact_start_is_not_another_runner() {
    let (_dir, k, c, manifest) = many();
    let (a, m) = run_model(&k, 5);
    let id = manifest.request.request_id;
    a.start_briefing_run(c, id).await.unwrap();
    a.start_briefing_run(c, id).await.unwrap();
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    let status = k.briefing_run_status(c, id).unwrap();
    assert_eq!(status.run.phase, BriefingRunPhase::Completed);
    assert_eq!(status.run.selected_records, 17);
    assert_eq!(status.run.processed_records, 17);
    assert_eq!(status.run.completed_batches, 5);
    assert!(status.source_current && !status.complete_upstream_history);
    let encoded = serde_json::to_value(&status).unwrap();
    assert!(
        encoded["day_from_local"]
            .as_str()
            .is_some_and(|s| s.ends_with("00:00:00 UTC+08:00"))
    );
    assert!(
        encoded["day_to_local_exclusive"]
            .as_str()
            .is_some_and(|s| s.ends_with("00:00:00 UTC+08:00"))
    );
    assert_eq!(m.requests.lock().unwrap().len(), 6);
    a.start_briefing_run(c, id).await.unwrap();
    assert_eq!(m.requests.lock().unwrap().len(), 6);
}
#[tokio::test]
async fn restart_does_not_relaunch_unknown_coordinator_or_provider_attempt() {
    let (dir, k, c, manifest) = many();
    let (a, m) = run_model(&k, 0);
    let id = manifest.request.request_id;
    a.start_briefing_run(c, id).await.unwrap();
    until(|| !m.requests.lock().unwrap().is_empty()).await;
    k.shutdown().unwrap();
    until(|| Arc::strong_count(&a) == 1).await;
    drop(a);
    drop(k);
    let k = Arc::new(Kernel::open(&dir.path().join("state.sqlite")).unwrap());
    k.mark_ready().unwrap();
    let (a, m) = run_model(&k, 5);
    let status = a.start_briefing_run(c, id).await.unwrap();
    assert_eq!(status.run.phase, BriefingRunPhase::Unknown);
    assert!(m.requests.lock().unwrap().is_empty());
    assert_eq!(k.inference_snapshot().unwrap().reserved_requests, 1);
}
#[tokio::test]
async fn source_revoke_during_first_batch_stops_the_rest_of_day() {
    let (_dir, k, c, manifest) = many();
    let (a, m) = run_model(&k, 0);
    let id = manifest.request.request_id;
    a.start_briefing_run(c, id).await.unwrap();
    until(|| !m.requests.lock().unwrap().is_empty()).await;
    k.briefing_set(
        c,
        SetBriefingGrant {
            request_id: RequestId::new(),
            expected_revision: 1,
            scope: None,
        },
    )
    .unwrap();
    m.gate.add_permits(5);
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    let status = k.briefing_run_status(c, id).unwrap();
    assert!(!status.source_current);
    assert_eq!(status.run.phase, BriefingRunPhase::SourceUnavailable);
    assert_eq!(m.requests.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn ordinary_briefing_read_and_original_lookup_capture_sources_block_generic_memory_and_withdraw()
 {
    use crate::actor::tools::{Empty, Tool};
    let (_dir, k, c, m) = many();
    let (a, _) = run_model(&k, 5);
    let id = m.request.request_id;
    a.start_briefing_run(c, id).await.unwrap();
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    let (turn, _) = k
        .actor_submit(c, uuid::Uuid::new_v4(), "昨天有什么重要消息？".into())
        .unwrap();
    let read = ActionId::new();
    k.actor_update(turn.id, move |t| {
        t.actions.push(Action {
            id: read,
            call_id: "brief-read".into(),
            tool: Tool::BriefingRead(BriefingRead {
                briefing_id: Some(id),
                after: 0,
                limit: 4,
                query: None,
            }),
            state: ActionState::Executing,
            result: None,
        });
        Ok(())
    })
    .unwrap();
    let value = k
        .briefing_actor_lookup(turn.id, read, Redactor::new(vec![]))
        .unwrap();
    assert_eq!(value["items"].as_array().unwrap().len(), 4);
    assert_eq!(value["items"][0]["item_number"], 1);
    assert!(value["has_more"].as_bool().unwrap());
    assert!(value["next_after"].as_u64().unwrap() > 0);
    assert!(!k.actor_memory_proposal_available(turn.id).unwrap());
    let inspect = ActionId::new();
    k.actor_update(turn.id, move |t| {
        let recorded = t.actions.iter_mut().find(|a| a.id == read).unwrap();
        recorded.state = ActionState::Executed;
        recorded.result = Some(value);
        t.actions.push(Action {
            id: inspect,
            call_id: "brief-original".into(),
            tool: Tool::BriefingInspect(BriefingInspect {
                briefing_id: Some(id),
                item_number: 2,
                citation_index: 0,
            }),
            state: ActionState::Executing,
            result: None,
        });
        Ok(())
    })
    .unwrap();
    let original = k
        .briefing_actor_lookup(turn.id, inspect, Redactor::new(vec![]))
        .unwrap();
    assert!(
        original["record"]["text"]
            .as_str()
            .unwrap()
            .contains("图书馆讨论")
    );
    k.actor_update(turn.id, move |t| {
        let a = t.actions.iter_mut().find(|a| a.id == inspect).unwrap();
        a.state = ActionState::Executed;
        a.result = Some(original);
        Ok(())
    })
    .unwrap();
    let (other, _) = k
        .actor_submit(
            k.create_conversation().unwrap().id,
            uuid::Uuid::new_v4(),
            "偷看另一会话".into(),
        )
        .unwrap();
    let other_read = ActionId::new();
    k.actor_update(other.id, move |t| {
        t.actions.push(Action {
            id: other_read,
            call_id: "denied".into(),
            tool: Tool::BriefingRead(BriefingRead {
                briefing_id: Some(id),
                after: 0,
                limit: 4,
                query: None,
            }),
            state: ActionState::Executing,
            result: None,
        });
        Ok(())
    })
    .unwrap();
    assert!(
        k.briefing_actor_lookup(other.id, other_read, Redactor::new(vec![]))
            .is_err()
    );
    // Metadata-only yesterday start does not itself disclose source content.
    assert!(!Tool::BriefingYesterday(Empty {}).source_read());
    let v = &m.records.last().unwrap();
    k.archive_correct(
        v.record_id,
        CorrectArchiveRecord {
            expected_revision: v.revision,
            text: "最后一条已经更正".into(),
        },
        Redactor::new(vec![]),
    )
    .unwrap();
    assert!(k.check_actor_memory(turn.id).is_err());
    assert!(k.actor_view(turn.id).unwrap().messages.is_empty());
    assert!(
        k.actor_update(turn.id, |t| {
            t.model_requests += 1;
            Ok(())
        })
        .is_err()
    );
    let notice = k.actor_source_notice(turn.id, None).unwrap();
    assert_eq!(notice.state, TurnState::Completed);
    assert!(
        k.check_actor_memory(turn.id).is_err(),
        "operational notice cannot restore disclosure authority"
    );
}

#[tokio::test]
async fn stopped_day_does_not_accept_a_late_batch_or_change_final_report() {
    let (_dir, k, c, manifest) = many();
    let (a, m) = run_model(&k, 0);
    let id = manifest.request.request_id;
    a.start_briefing_run(c, id).await.unwrap();
    until(|| !m.requests.lock().unwrap().is_empty()).await;
    k.stop_briefing_run(c, id, BriefingRunStop::Deadline)
        .unwrap();
    m.gate.add_permits(1);
    until(|| Arc::strong_count(&a) == 1).await;
    let status = k.briefing_run_status(c, id).unwrap();
    assert_eq!(status.run.phase, BriefingRunPhase::Partial);
    assert_eq!(status.run.processed_records, 0);
    assert!(
        k.briefing_batch(c, id, 0, Redactor::new(vec![]))
            .unwrap()
            .items
            .is_none()
    );
    assert!(
        k.briefing_report(c, id, 0, 4, Redactor::new(vec![]))
            .unwrap()["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(k.inference_snapshot().unwrap().reported_usage_tokens_sum, 7);
}

#[test]
fn stop_between_preparation_and_actual_provider_admission_never_starts_the_request() {
    let (_dir, k, c, m, _) = fixture();
    let id = m.request.request_id;
    k.begin_briefing_run(c, id).unwrap();
    let (t, _) = k
        .create_briefing_job(
            BriefingActivity {
                stage: lain_wire::briefing::BriefingStage::Extract,
                conversation_id: c,
                briefing_id: id,
                from_index: 0,
            },
            Redactor::new(vec![]),
        )
        .unwrap();
    k.actor_update(t.id, |t| {
        t.model_requests = 1;
        Ok(())
    })
    .unwrap();
    k.briefing_actor_input(t.id, Redactor::new(vec![])).unwrap();
    k.stop_briefing_run(c, id, BriefingRunStop::Deadline)
        .unwrap();
    assert!(k.begin_inference(t.id, 1).is_err());
    let entry = k
        .inference_snapshot()
        .unwrap()
        .recent_requests
        .into_iter()
        .find(|r| r.actor_turn_id == t.id)
        .unwrap();
    assert_eq!(
        entry.state,
        lain_wire::inference::InferenceRequestState::Prepared
    );
}
