use super::*;
use crate::actor::{domain::Action, tools::Tool};
#[path = "briefing_context_tests.rs"]
mod context_growth;

fn read(
    k: &Arc<Kernel>,
    c: ConversationId,
    args: serde_json::Value,
) -> (ActorTurnId, serde_json::Value) {
    lookup(k, c, "briefing_read", args)
}
fn read_original(
    k: &Arc<Kernel>,
    c: ConversationId,
    id: RequestId,
    item: u16,
    citation: u16,
) -> (ActorTurnId, serde_json::Value) {
    lookup(
        k,
        c,
        "briefing_inspect",
        json!({"briefing_id":id,"item_number":item,"citation_index":citation}),
    )
}
fn lookup(
    k: &Arc<Kernel>,
    c: ConversationId,
    name: &str,
    args: serde_json::Value,
) -> (ActorTurnId, serde_json::Value) {
    // Parse the actual model-call boundary, rather than directly constructing a
    // Rust value that can bypass its old/new JSON compatibility contract.
    let tool = Tool::parse(name, &args.to_string()).unwrap();
    tool.validate().unwrap();
    let (turn, _) = k
        .actor_submit(c, uuid::Uuid::new_v4(), "查这条通知原文".into())
        .unwrap();
    let action = ActionId::new();
    k.actor_update(turn.id, |t| {
        t.actions.push(Action {
            id: action,
            call_id: "lookup-short-phrase".into(),
            tool,
            state: ActionState::Executing,
            result: None,
        });
        Ok(())
    })
    .unwrap();
    (
        turn.id,
        k.briefing_actor_lookup(turn.id, action, Redactor::new(vec![]))
            .unwrap(),
    )
}

struct BriefingProjectionModel {
    requests: Mutex<Vec<Request>>,
    briefing: RequestId,
    final_text: Option<String>,
    final_calls: Vec<(String, String, String)>,
}

#[tokio::test]
async fn short_checked_quote_keeps_same_source_conditions_in_default_reply() {
    let (_dir, k, c, _, record) = fixture();
    let selected = "周五17:00可凭学生证到校园书店领取免费笔记本";
    let paragraph = format!("福利通知：{selected}，限前50人，每人一本。");
    let corrected = k
        .archive_correct(
            record.id,
            CorrectArchiveRecord {
                expected_revision: record.revision,
                text: format!("不相关的前段。\n\n{paragraph}\n\n不相关的后段。"),
            },
            Redactor::new(vec![]),
        )
        .unwrap();
    let manifest = k
        .briefing_start(
            c,
            StartBriefing {
                request_id: RequestId::new(),
                grant_revision: 1,
                day: BriefingDay::Yesterday,
            },
        )
        .unwrap();
    let id = manifest.request.request_id;
    let processor = Arc::new(Model {
        requests: Mutex::new(vec![]),
        gate: Semaphore::new(1),
        tool: false,
        text: json!({"important_items_omitted":false,"items":[{"title":"Free notebooks",
            "summary":"Free notebooks at the bookstore, first50students and one per person.",
            "category":"resource","time_text":"周五17:00","uncertainty":"Unverified source notice.",
            "citations":[{"record_id":corrected.id,"quote":selected}]}]})
        .to_string(),
    });
    let extractor = Actor::with_model(
        k.clone(),
        None,
        "fixture-condition-context-extractor-123456"
            .parse()
            .unwrap(),
        processor,
    );
    extractor.start_briefing_run(c, id).await.unwrap();
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    assert_eq!(
        k.briefing_run_status(c, id).unwrap().run.phase,
        BriefingRunPhase::Completed
    );
    let model = Arc::new(BriefingProjectionModel {
        requests: Mutex::new(vec![]),
        briefing: id,
        final_text: None,
        final_calls: vec![],
    });
    let actor = Actor::with_model(
        k.clone(),
        None,
        "fixture-condition-context-front-123456789".parse().unwrap(),
        model.clone(),
    );
    let t = actor
        .submit(c, uuid::Uuid::new_v4(), "昨天有什么福利？".into())
        .await
        .unwrap();
    until(|| !k.actor_turn(t.id).unwrap().state.active()).await;
    let t = k.actor_turn(t.id).unwrap();
    assert_eq!(t.state, TurnState::Completed);
    let body = t
        .messages
        .iter()
        .filter(|m| m.role == MessageRole::Assistant)
        .map(|m| m.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        body.contains("限前50人，每人一本"),
        "default must not lose source conditions merely because checked model quote is shorter"
    );
    assert!(!body.contains("不相关的前段") && !body.contains("不相关的后段"));
    assert!(body.contains("其他段落未展示"));
    let citation = &t.actions[0].result.as_ref().unwrap()["items"][0]["citations"][0];
    assert_eq!(
        citation["quote_excerpt"], selected,
        "keep exact original model quotation separately"
    );
    assert_eq!(
        citation["source_context"]["text"].as_str().unwrap().trim(),
        paragraph
    );
    assert_eq!(citation["source_context"]["omitted_before"], true);
    assert_eq!(citation["source_context"]["omitted_after"], true);
    assert_eq!(t.model_requests, 2);
    assert_eq!(
        t.actions.len(),
        1,
        "context is within the same authorized read, not a hidden inspect"
    );
    let requests = model.requests.lock().unwrap();
    assert!(requests[1].input.iter().any(|m| {
        m["type"] == "function_call_output"
            && m["output"]
                .as_str()
                .is_some_and(|s| s.contains("限前50人，每人一本"))
    }));
    k.archive_correct(
        corrected.id,
        CorrectArchiveRecord {
            expected_revision: corrected.revision,
            text: "通知撤销。".into(),
        },
        Redactor::new(vec![]),
    )
    .unwrap();
    assert!(
        k.actor_view(t.id).unwrap().messages.is_empty(),
        "source context must withdraw with the original record"
    );
}

#[tokio::test]
async fn explicit_original_request_cannot_finish_with_only_report_excerpts() {
    let (_dir, k, c, manifest, record) = fixture();
    let id = manifest.request.request_id;
    let (extractor, _) = model(&k, &record, 1, false, false);
    extractor.start_briefing_run(c, id).await.unwrap();
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    let model = Arc::new(BriefingProjectionModel {
        requests: Mutex::new(vec![]),
        briefing: id,
        final_text: None,
        final_calls: vec![(
            "wrong-final".into(),
            "briefing_answer".into(),
            json!({"items":[{"briefing_id":id,"item_number":1}]}).to_string(),
        )],
    });
    let actor = Actor::with_model(
        k.clone(),
        None,
        "fixture-original-request-contract-123456789"
            .parse()
            .unwrap(),
        model.clone(),
    );
    let t = actor
        .submit(
            c,
            uuid::Uuid::new_v4(),
            "课程讨论的地点更正那条，原文怎么说？请先给原文。".into(),
        )
        .await
        .unwrap();
    until(|| !k.actor_turn(t.id).unwrap().state.active()).await;
    let t = k.actor_turn(t.id).unwrap();
    assert_eq!(
        t.state,
        TurnState::Failed,
        "a report-only final must not complete an explicit original request"
    );
    assert_eq!(t.model_requests, 2);
    assert_eq!(t.usage_tokens, 14);
    assert_eq!(
        t.actions.len(),
        1,
        "request guard adds no source read or replay"
    );
    assert!(t.error.as_deref().unwrap().contains("original_required"));
    assert!(
        t.messages
            .iter()
            .filter(|m| m.role == MessageRole::Assistant)
            .all(|m| !m.content.contains("来源简报：")
                && !m.content.contains("明晚七点在图书馆讨论"))
    );
    let requests = model.requests.lock().unwrap();
    assert!(
        requests[1]
            .persona
            .contains("Kernel requested source view: original")
    );
    let catalog_line = requests[1]
        .persona
        .lines()
        .find_map(|line| line.strip_prefix("Kernel available final evidence selections: "))
        .unwrap();
    let catalog: serde_json::Value = serde_json::from_str(catalog_line).unwrap();
    assert_eq!(catalog["items"], json!([]));
    assert_eq!(catalog["originals"], json!([]));
}

#[tokio::test]
async fn structured_briefing_selection_finishes_without_extra_request_or_action() {
    let (_dir, k, c, manifest, record) = fixture();
    let id = manifest.request.request_id;
    let (extractor, _) = model(&k, &record, 1, false, false);
    extractor.start_briefing_run(c, id).await.unwrap();
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    let model = Arc::new(BriefingProjectionModel {
        requests: Mutex::new(vec![]),
        briefing: id,
        final_text: Some("UNACCEPTED_FINAL_DRAFT".into()),
        final_calls: vec![(
            "selection-final".into(),
            "briefing_answer".into(),
            json!({"items":[{"briefing_id":id,"item_number":1}]}).to_string(),
        )],
    });
    let actor = Actor::with_model(
        k.clone(),
        None,
        "fixture-structured-source-answer-token-12345"
            .parse()
            .unwrap(),
        model.clone(),
    );
    let t = actor
        .submit(c, uuid::Uuid::new_v4(), "昨天的重要消息？".into())
        .await
        .unwrap();
    until(|| !k.actor_turn(t.id).unwrap().state.active()).await;
    let t = k.actor_turn(t.id).unwrap();
    assert_eq!(
        t.state,
        TurnState::Completed,
        "a checked final-selection call is formatting, not an unknown executable tool"
    );
    assert_eq!(t.model_requests, 2);
    assert_eq!(t.usage_tokens, 14);
    assert_eq!(
        t.actions.len(),
        1,
        "final formatting must add no Kernel action"
    );
    let body = t
        .messages
        .iter()
        .filter(|m| m.role == MessageRole::Assistant)
        .map(|m| m.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(body.contains("明晚七点在图书馆讨论"));
    assert!(!body.contains("briefing_answer") && !body.contains("UNACCEPTED_FINAL_DRAFT"));
    let requests = model.requests.lock().unwrap();
    assert!(
        !requests[0]
            .tools
            .iter()
            .any(|t| t["name"] == "briefing_answer")
    );
    assert!(
        requests[1]
            .tools
            .iter()
            .any(|t| t["name"] == "briefing_answer")
    );
}

#[tokio::test]
async fn structured_final_selection_rejects_bad_refs_and_mixed_calls_without_execution() {
    let (_dir, k, c, manifest, record) = fixture();
    let id = manifest.request.request_id;
    let (extractor, _) = model(&k, &record, 1, false, false);
    extractor.start_briefing_run(c, id).await.unwrap();
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    let valid = json!({"items":[{"briefing_id":id,"item_number":1}]}).to_string();
    for (call_id, args, mixed) in [
        (
            "answer-final",
            json!({"items":[{"briefing_id":id,"item_number":999}]}).to_string(),
            false,
        ),
        (
            "answer-final",
            json!({"items":[{"briefing_id":id,"item_number":1}],"explanation":"unaccepted fact"})
                .to_string(),
            false,
        ),
        (
            "answer-final",
            json!({"originals":[{"briefing_id":id,"item_number":1,"citation_index":0}]})
                .to_string(),
            false,
        ),
        ("answer-final", "not JSON".into(), false),
        ("projected-read", valid.clone(), false),
        ("answer-final", valid.clone(), true),
    ] {
        let mut calls = vec![(call_id.into(), "briefing_answer".into(), args)];
        if mixed {
            calls.push(("must-not-run".into(), "device_list".into(), "{}".into()));
        }
        let model = Arc::new(BriefingProjectionModel {
            requests: Mutex::new(vec![]),
            briefing: id,
            final_text: Some("UNACCEPTED_FINAL_DRAFT".into()),
            final_calls: calls,
        });
        let actor = Actor::with_model(
            k.clone(),
            None,
            "fixture-rejected-selection-token-123456789"
                .parse()
                .unwrap(),
            model,
        );
        let t = actor
            .submit(c, uuid::Uuid::new_v4(), "只读昨天消息".into())
            .await
            .unwrap();
        until(|| !k.actor_turn(t.id).unwrap().state.active()).await;
        let t = k.actor_turn(t.id).unwrap();
        assert_eq!(t.state, TurnState::Failed);
        assert_eq!(
            t.model_requests, 2,
            "a bad final selection must not trigger another model request"
        );
        assert_eq!(t.usage_tokens, 14);
        assert_eq!(
            t.actions.len(),
            1,
            "neither final formatting nor mixed calls may add an action"
        );
        assert!(
            t.messages.iter().all(|m| m.role != MessageRole::Assistant
                || !m.content.contains("UNACCEPTED_FINAL_DRAFT"))
        );
    }
}

#[tokio::test]
async fn briefing_final_answer_requires_actual_evidence_selection_and_preserves_accounting() {
    let (_dir, k, c, manifest, record) = fixture();
    let id = manifest.request.request_id;
    let (extractor, _) = model(&k, &record, 1, false, false);
    extractor.start_briefing_run(c, id).await.unwrap();
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    for (text, accepted) in [
        ("发送当天是周日，这些是群的转发通知。".to_owned(), true),
        (json!({"briefing_answer":{"items":[{"briefing_id":id,"item_number":999}]}}).to_string(), false),
        (json!({"briefing_answer":{"items":[{"briefing_id":RequestId::new(),"item_number":1}]}}).to_string(), false),
        (json!({"briefing_answer":{"items":[{"briefing_id":id,"item_number":1},{"briefing_id":id,"item_number":1}]}}).to_string(), false),
        (json!({"briefing_answer":{"items":[{"briefing_id":id,"item_number":1}],"explanation":"发送当天是周日"}}).to_string(), false),
        (json!({"briefing_answer":{"originals":[{"briefing_id":id,"item_number":1,"citation_index":0}]}}).to_string(), false),
        (json!({"briefing_answer":{"empty":{"briefing_id":id,"query":null}}}).to_string(), false),
        (json!({"briefing_answer":{"items":[{"briefing_id":id,"item_number":1}]}}).to_string(), true),
        (json!({"briefing_answer":{"detailed":true,"items":[{"briefing_id":id,"item_number":1}]}}).to_string(), true),
        (json!({"briefing_answer":{"detailed":"true","items":[{"briefing_id":id,"item_number":1}]}}).to_string(), false),
        (format!("发送当天是周日，这些是群的转发通知。\n```json\n{}\n```",json!({"briefing_answer":{"items":[{"briefing_id":id,"item_number":1}]}})), true),
    ] {
        let detailed = text.contains("\"detailed\":true");
        let model = Arc::new(BriefingProjectionModel {
            requests: Mutex::new(vec![]), briefing:id, final_text:Some(text), final_calls:vec![],
        });
        let actor = Actor::with_model(k.clone(), None, "fixture-presentation-token-0123456789".parse().unwrap(), model);
        let mut observations=actor.subscribe();
        let turn = actor.submit(c, uuid::Uuid::new_v4(), "昨天有什么重要消息？".into()).await.unwrap();
        until(|| !k.actor_turn(turn.id).unwrap().state.active()).await;
        let turn = k.actor_turn(turn.id).unwrap();
        assert_eq!(turn.model_requests, 2);
        assert_eq!(turn.usage_tokens, 14, "a rejected answer still consumed the returned usage");
        let body = turn.messages.iter().filter(|m|m.role==MessageRole::Assistant).map(|m|m.content.as_str()).collect::<Vec<_>>().join("\n");
        let mut preview_count = 0;
        while let Ok(observation) = observations.try_recv() {
            if let crate::actor::Observation::Preview(event) = observation {
                preview_count += 1;
                if let lain_wire::PreviewUpdate::Delta { text }
                | lain_wire::PreviewUpdate::Started { text } = event.update {
                    assert!(!text.contains("briefing_answer") && !text.contains("发送当天是周日") && !text.contains("转发通知"), "unvalidated source answer escaped through preview");
                }
            }
        }
        assert!(preview_count >= 2, "test must inspect both request previews, not stop at a snapshot");
        if accepted {
            assert_eq!(turn.state, TurnState::Completed);
            assert!(body.contains("明晚七点在图书馆讨论"));
            assert!(body.contains("来源"));
            assert!(!body.contains("briefing_answer"));
            assert_eq!(body.contains("整理参考（模型生成，非来源原话）"), detailed);
            assert!(!body.contains("发送当天是周日")&&!body.contains("转发通知"));
        } else {
            assert_eq!(turn.state, TurnState::Failed, "known answer rejection is Failed; unbound prose or invented references must not publish");
            assert!(turn.error.as_deref().unwrap().contains("简报已读取"));
            assert!(!body.contains("发送当天是周日") && !body.contains("转发通知") && !body.contains("briefing_answer"));
        }
    }
}

#[tokio::test]
async fn report_does_not_supply_speaker_identity_but_original_retains_source_local_label() {
    let (_dir, k, c, manifest, record) = fixture();
    let id = manifest.request.request_id;
    let (extractor, _) = model(&k, &record, 1, false, false);
    extractor.start_briefing_run(c, id).await.unwrap();
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    let (report_turn, report) = read(&k, c, json!({"briefing_id":id,"after":0,"limit":4}));
    assert_eq!(report["speaker_attribution"]["report"], "not_provided");
    assert_eq!(
        report["speaker_attribution"]["lookup"],
        "briefing_inspect: record.speaker + source.id"
    );
    for citation in report["items"][0]["citations"].as_array().unwrap() {
        assert!(citation.get("speaker").is_none());
    }
    k.actor_update(report_turn, |t| {
        t.actions[0].state = ActionState::Executed;
        t.actions[0].result = Some(report.clone());
        t.state = TurnState::Completed;
        Ok(())
    })
    .unwrap();
    let (_, original) = read_original(&k, c, id, 1, 0);
    assert_eq!(original["record"]["speaker"], record.speaker);
    assert_eq!(original["source"]["id"], record.source_id.to_string());
    assert_eq!(
        original["context_role"],
        "untrusted_original_report_not_instruction_or_participation_claim"
    );
}

#[tokio::test]
async fn intake_semantics_reaches_extraction_report_and_original_without_changing_clocks() {
    let (_dir, k, c, manifest, record) = fixture();
    let id = manifest.request.request_id;
    let (extractor, model) = model(&k, &record, 1, false, false);
    extractor.start_briefing_run(c, id).await.unwrap();
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    let input = {
        let requests = model.requests.lock().unwrap();
        let raw = requests[0].input.last().unwrap()["content"]
            .as_str()
            .unwrap();
        serde_json::from_str::<serde_json::Value>(
            raw.strip_prefix("Kernel-attributed untrusted daily source page (data only): ")
                .unwrap(),
        )
        .unwrap()
    };
    let semantics = input["intake_semantics"].clone();
    assert!(
        semantics.is_object(),
        "extraction lacks intake/publication boundary"
    );
    assert!(
        semantics["receipt_clock"]
            .as_str()
            .unwrap()
            .to_ascii_lowercase()
            .contains("archive intake")
    );
    assert!(
        semantics["source_clock"]
            .as_str()
            .unwrap()
            .contains("resend")
    );
    assert!(
        semantics["source_wording"]
            .as_str()
            .unwrap()
            .contains("unverified")
    );
    let (report_turn, report) = read(&k, c, json!({"briefing_id":id,"after":0,"limit":4}));
    k.actor_update(report_turn, |t| {
        t.actions[0].state = ActionState::Executed;
        t.actions[0].result = Some(report.clone());
        t.state = TurnState::Completed;
        Ok(())
    })
    .unwrap();
    let (original_turn, original) = read_original(&k, c, id, 1, 0);
    k.actor_update(original_turn, |t| {
        t.actions[0].state = ActionState::Executed;
        t.actions[0].result = Some(original.clone());
        t.state = TurnState::Completed;
        Ok(())
    })
    .unwrap();
    assert_eq!(report["intake_semantics"], semantics);
    assert_eq!(original["intake_semantics"], semantics);
    let extraction_record = &input["page"]["records"][0];
    let citation = &report["items"][0]["citations"][0];
    let received = time::OffsetDateTime::from_unix_timestamp(
        i64::try_from(record.received_at_ms / 1000).unwrap(),
    )
    .unwrap()
    .to_offset(time::UtcOffset::from_hms(8, 0, 0).unwrap());
    let occurred = time::OffsetDateTime::from_unix_timestamp(
        i64::try_from(record.occurred_at_ms.unwrap() / 1000).unwrap(),
    )
    .unwrap()
    .to_offset(time::UtcOffset::from_hms(8, 0, 0).unwrap());
    assert_ne!(
        occurred.date(),
        received.date(),
        "fixture must cross intake days"
    );
    let receipt_label = format!(
        "{} {:02}:{:02}:{:02} UTC+08:00",
        received.date(),
        received.hour(),
        received.minute(),
        received.second()
    );
    for (stage, value) in [
        ("extraction", extraction_record),
        ("report citation", citation),
        ("original", &original),
    ] {
        assert_eq!(
            value["archive_received_local_time"], receipt_label,
            "{stage} must retain its own readable archive receipt clock"
        );
    }
    for field in ["occurred_at_ms", "received_at_ms"] {
        let expected = serde_json::to_value(&record).unwrap()[field].clone();
        assert_eq!(extraction_record[field], expected);
        assert_eq!(citation[field], expected);
        assert_eq!(original["record"][field], expected);
    }
    assert_eq!(extraction_record["text"], json!(record.text));
    assert_eq!(original["record"]["text"], json!(record.text));
}

#[tokio::test]
async fn briefing_followup_keeps_exact_read_pointers_without_replaying_old_evidence() {
    let (_dir, k, c, m, record) = fixture();
    let (extractor, _) = model(&k, &record, 1, false, false);
    extractor
        .start_briefing_run(c, m.request.request_id)
        .await
        .unwrap();
    until(|| {
        k.briefing_run_status(c, m.request.request_id)
            .unwrap()
            .run
            .phase
            .terminal()
    })
    .await;
    let (past, value) = read(
        &k,
        c,
        json!({"briefing_id":m.request.request_id,"after":0,"limit":4}),
    );
    let numbers = value["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["item_number"].clone())
        .collect::<Vec<_>>();
    k.actor_update(past, |t| {
        t.actions[0].state = ActionState::Executed;
        t.actions[0].result = Some(value);
        t.message(
            MessageRole::Assistant,
            "PREVIOUS_SOURCE_ANSWER_MUST_NOT_REPLAY".into(),
        )?;
        t.state = TurnState::Completed;
        Ok(())
    })
    .unwrap();
    let recording = Arc::new(Model {
        requests: Mutex::new(vec![]),
        gate: Semaphore::new(4),
        text: "recorded follow-up".into(),
        tool: false,
    });
    let actor = Actor::with_model(
        k.clone(),
        None,
        "fixture-followup-owner-1234567890".parse().unwrap(),
        recording.clone(),
    );
    let turn = actor
        .submit(c, uuid::Uuid::new_v4(), "这条回复原文怎么说？".into())
        .await
        .unwrap();
    until(|| !k.actor_turn(turn.id).unwrap().state.active()).await;
    {
        let requests = recording.requests.lock().unwrap();
        let input = &requests[0].input;
        let pointer = input
            .iter()
            .filter_map(|v| v["content"].as_str())
            .filter_map(|text| serde_json::from_str::<serde_json::Value>(text).ok())
            .find(|v| v["historical_briefing_reads"].is_array())
            .expect("source-history slimming lost the follow-up's exact read pointers");
        assert_eq!(
            pointer["historical_briefing_reads"],
            json!([{"tool":"briefing_read","briefing_id":m.request.request_id,"item_numbers":numbers}])
        );
        let text = serde_json::to_string(input).unwrap();
        assert!(
            !text.contains("PREVIOUS_SOURCE_ANSWER_MUST_NOT_REPLAY")
                && !text.contains(&record.text.clone().unwrap())
        );
    }
    let (inspected, value) = read_original(&k, c, m.request.request_id, 1, 0);
    k.actor_update(inspected, |t| {
        t.actions[0].state = ActionState::Executed;
        t.actions[0].result = Some(value);
        t.message(
            MessageRole::Assistant,
            "PREVIOUS_ORIGINAL_MUST_NOT_REPLAY".into(),
        )?;
        t.state = TurnState::Completed;
        Ok(())
    })
    .unwrap();
    let next = actor
        .submit(c, uuid::Uuid::new_v4(), "刚才那条原文再看一下".into())
        .await
        .unwrap();
    until(|| !k.actor_turn(next.id).unwrap().state.active()).await;
    {
        let requests = recording.requests.lock().unwrap();
        let input = &requests.last().unwrap().input;
        let pointers = input
            .iter()
            .filter_map(|v| v["content"].as_str())
            .filter_map(|text| serde_json::from_str::<serde_json::Value>(text).ok())
            .filter_map(|v| v["historical_briefing_reads"].as_array().cloned())
            .flatten()
            .collect::<Vec<_>>();
        assert!(pointers.contains(&json!({"tool":"briefing_inspect","briefing_id":m.request.request_id,"item_number":1,"citation_index":0})));
        let text = serde_json::to_string(input).unwrap();
        assert!(
            !text.contains("PREVIOUS_ORIGINAL_MUST_NOT_REPLAY")
                && !text.contains(&record.text.clone().unwrap())
        );
    }
    // Current authority remains enforced by the normal lookup, not the pointer.
    k.archive_set_grant(
        record.source_id,
        SetArchiveGrant {
            expected_revision: 1,
            scope: None,
        },
    )
    .unwrap();
    let denied = actor
        .submit(c, uuid::Uuid::new_v4(), "原文呢？".into())
        .await
        .unwrap();
    until(|| !k.actor_turn(denied.id).unwrap().state.active()).await;
    let requests = recording.requests.lock().unwrap();
    let request = requests.last().unwrap();
    assert!(
        !request
            .tools
            .iter()
            .any(|v| v["name"] == "briefing_inspect")
    );
    let text = serde_json::to_string(&request.input).unwrap();
    assert!(
        text.matches("not current evidence, delivery or permission")
            .count()
            <= 1,
        "the same historical navigation explanation was loaded for every old turn"
    );
    assert!(
        !text.contains("PREVIOUS_SOURCE_ANSWER_MUST_NOT_REPLAY")
            && !text.contains(&record.text.unwrap())
    );
}
impl ActorModel for BriefingProjectionModel {
    fn respond_stream(
        &self,
        request: Request,
        preview: crate::actor::preview::PreviewSink,
    ) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
        Box::pin(async move {
            let reply = self.respond(request).await?;
            preview.delta(&reply.text);
            Ok(reply)
        })
    }
    fn respond(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
        let first = {
            let mut requests = self.requests.lock().unwrap();
            let first = requests.is_empty();
            requests.push(request);
            first
        };
        let args =
            json!({"briefing_id":self.briefing,"after":0,"limit":4,"query":null}).to_string();
        Box::pin(async move {
            Ok(Reply {
                text: if first {
                    String::new()
                } else {
                    self.final_text.clone().unwrap_or_else(||
                    json!({"briefing_answer":{"items":[{"briefing_id":self.briefing,"item_number":1}]}}).to_string())
                },
                calls: if first {
                    vec![crate::actor::model::Call {
                        id: "projected-read".into(),
                        name: "briefing_read".into(),
                        arguments: args.clone(),
                    }]
                } else {
                    self.final_calls
                        .iter()
                        .map(|(id, name, arguments)| crate::actor::model::Call {
                            id: id.clone(),
                            name: name.clone(),
                            arguments: arguments.clone(),
                        })
                        .collect()
                },
                usage_tokens: 7,
                continuation: if first {
                    vec![
                        json!({"type":"function_call","call_id":"projected-read","name":"briefing_read","arguments":args}),
                    ]
                } else {
                    vec![]
                },
            })
        })
    }
}
#[tokio::test]
async fn briefing_model_projection_keeps_facts_and_inspection_handles_with_full_audit() {
    let (_dir, k, c, m) = many();
    let id = m.request.request_id;
    let (extractor, _) = run_model(&k, 5);
    extractor.start_briefing_run(c, id).await.unwrap();
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    let full = k
        .briefing_report(c, id, 0, 4, Redactor::new(vec![]))
        .unwrap();
    let model = Arc::new(BriefingProjectionModel {
        requests: Mutex::new(vec![]),
        briefing: id,
        final_text: None,
        final_calls: vec![],
    });
    let actor = Actor::with_model(
        k.clone(),
        None,
        "fixture-projection-owner-1234567890".parse().unwrap(),
        model.clone(),
    );
    let t = actor
        .submit(c, uuid::Uuid::new_v4(), "昨天有什么重要消息？".into())
        .await
        .unwrap();
    until(|| !k.actor_turn(t.id).unwrap().state.active()).await;
    let stored = k.actor_turn(t.id).unwrap();
    assert_eq!(stored.state, TurnState::Completed);
    assert_eq!(stored.actions.len(), 1);
    assert_eq!(
        stored.actions[0].result.as_ref().unwrap()["items"],
        full["items"],
        "audit/API must retain complete identities and evidence"
    );
    let requests = model.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    let catalog: serde_json::Value = serde_json::from_str(
        requests[1]
            .persona
            .lines()
            .find_map(|line| line.strip_prefix("Kernel available final evidence selections: "))
            .expect("report citations must not advertise unread originals as selectable"),
    )
    .unwrap();
    assert_eq!(catalog["originals"], json!([]));
    assert!(
        catalog["items"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty())
    );
    let outputs = requests[1]
        .input
        .iter()
        .filter(|v| v["type"] == "function_call_output")
        .collect::<Vec<_>>();
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0]["call_id"], "projected-read");
    let view: serde_json::Value =
        serde_json::from_str(outputs[0]["output"].as_str().unwrap()).unwrap();
    assert!(
        view["model_projection"].is_string(),
        "model needs a compact view rather than a repeated owner report"
    );
    for key in [
        "briefing_id",
        "calendar",
        "coverage",
        "status",
        "has_more",
        "next_after",
        "reconciliation",
        "speaker_attribution",
        "omission_flag_semantics",
        "total_generated_items",
        "total_display_groups",
    ] {
        assert_eq!(view[key], full[key], "changed {key}");
    }
    let rows = full["items"].as_array().unwrap();
    assert_eq!(view["items"].as_array().unwrap().len(), rows.len());
    for (index, row) in rows.iter().enumerate() {
        let projected = &view["items"][index];
        for key in [
            "item_number",
            "title",
            "summary",
            "category",
            "time_text",
            "uncertainty",
            "summary_origin",
            "group_number",
            "members",
            "relation",
            "relation_parent_number",
            "reported_correction_item_number",
            "related_statement_numbers",
        ] {
            assert_eq!(projected[key], row[key], "changed {key}");
        }
        assert_eq!(view["uncertainty_scope"], row["uncertainty_scope"]);
        assert!(projected.get("item_id").is_none() && projected.get("uncertainty_scope").is_none());
        for (citation_index, citation) in row["citations"].as_array().unwrap().iter().enumerate() {
            let target = &projected["citations"][citation_index];
            assert_eq!(target["citation_index"], citation_index);
            for key in [
                "record_id",
                "source_label",
                "source_local_time",
                "time_basis",
                "text_origin",
                "quote_excerpt",
                "quote_excerpt_truncated",
            ] {
                assert_eq!(target[key], citation[key], "changed {key}");
            }
            for key in ["source_id", "revision", "occurred_at_ms", "received_at_ms"] {
                assert!(target.get(key).is_none());
                assert!(citation.get(key).is_some());
            }
        }
    }
    eprintln!(
        "identity_projection_bytes full={} model={}",
        serde_json::to_vec(&full).unwrap().len(),
        serde_json::to_vec(&view).unwrap().len()
    );
    assert!(
        serde_json::to_vec(&view).unwrap().len() + 1000 < serde_json::to_vec(&full).unwrap().len(),
        "remove repeated metadata without clipping facts"
    );
    eprintln!(
        "projection_bytes full={} model={} initial_instructions={} initial_input={} initial_tools={}",
        serde_json::to_vec(&full).unwrap().len(),
        serde_json::to_vec(&view).unwrap().len(),
        requests[0].persona.len(),
        serde_json::to_vec(&requests[0].input).unwrap().len(),
        serde_json::to_vec(&requests[0].tools).unwrap().len()
    );
}

#[tokio::test]
async fn briefing_phrase_lookup_reaches_late_citation_without_reading_every_page_and_captures_source()
 {
    let (_dir, k, c, m) = many();
    let late = &m.records[16];
    let text = "定位测试：补发校园帆布袋通知，明天下午14:00凭学生证在书店领取。";
    k.archive_correct(
        late.record_id,
        CorrectArchiveRecord {
            expected_revision: late.revision,
            text: text.into(),
        },
        Redactor::new(vec![]),
    )
    .unwrap();
    let manifest = k
        .briefing_start(
            c,
            StartBriefing {
                request_id: RequestId::new(),
                grant_revision: 1,
                day: BriefingDay::Yesterday,
            },
        )
        .unwrap();
    let id = manifest.request.request_id;
    let (a, model) = run_model(&k, 5);
    a.start_briefing_run(c, id).await.unwrap();
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    assert_eq!(
        k.briefing_report(c, id, 0, 4, Redactor::new(vec![]))
            .unwrap()["items"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    let before = model.requests.lock().unwrap().len();
    // The distinguishing phrase is present only in a checked citation, not its
    // generated title/summary. The fifth item is beyond the ordinary first page.
    let (turn, value) = read(
        &k,
        c,
        json!({"briefing_id":id,"after":0,"limit":4,"query":"帆布袋"}),
    );
    assert_eq!(value["items"].as_array().unwrap().len(), 1);
    assert_eq!(value["items"][0]["item_number"], 5);
    assert_eq!(value["items"][0]["matching_citation_indices"], json!([0]));
    assert_eq!(value["items"][0]["citations"][0]["quote_excerpt"], text);
    assert_eq!(
        value["items"][0]["citations"][0]["quote_excerpt_truncated"],
        false
    );
    assert_eq!(value["total_matching_items"], 1);
    assert_eq!(value["query"], "帆布袋");
    assert_eq!(value["has_more"], false);
    assert!(serde_json::to_vec(&value).unwrap().len() <= 12 * 1024);
    assert!(!k.actor_memory_proposal_available(turn).unwrap());
    let inspect = ActionId::new();
    k.actor_update(turn, |t| {
        t.actions[0].state = ActionState::Executed;
        t.actions[0].result = Some(value);
        t.actions.push(Action {
            id: inspect,
            call_id: "exact-late-original".into(),
            tool: Tool::BriefingInspect(BriefingInspect {
                briefing_id: Some(id),
                item_number: 5,
                citation_index: 0,
            }),
            state: ActionState::Executing,
            result: None,
        });
        Ok(())
    })
    .unwrap();
    let original = k
        .briefing_actor_lookup(turn, inspect, Redactor::new(vec![]))
        .unwrap();
    assert_eq!(original["record"]["id"], late.record_id.to_string());
    assert_eq!(original["record"]["text"], text);
    assert_eq!(
        model.requests.lock().unwrap().len(),
        before,
        "lookup starts no model processing"
    );
    k.archive_set_grant(
        late.source_id,
        SetArchiveGrant {
            expected_revision: 1,
            scope: None,
        },
    )
    .unwrap();
    assert!(k.check_actor_memory(turn).is_err());
    assert!(k.actor_view(turn).unwrap().messages.is_empty());
    assert!(
        k.briefing_actor_lookup(turn, inspect, Redactor::new(vec![]))
            .is_err()
    );
}

#[tokio::test]
async fn briefing_phrase_lookup_keeps_duplicate_original_numbers_and_bounded_cursor_pages() {
    let (_dir, k, c, m) = many();
    let id = m.request.request_id;
    let mut fake = RunModel::new(5, true);
    fake.link = true;
    let a = Actor::with_model(
        k.clone(),
        None,
        "fixture-lookup-token-0123456789012345".parse().unwrap(),
        Arc::new(fake),
    );
    a.start_briefing_run(c, id).await.unwrap();
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    assert_eq!(
        k.briefing_report(c, id, 0, 4, Redactor::new(vec![]))
            .unwrap()["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let mut after = 0;
    let mut found = vec![];
    loop {
        let (turn, page) = read(
            &k,
            c,
            json!({"briefing_id":id,"after":after,"limit":4,"query":"图书馆"}),
        );
        assert_eq!(page["total_matching_items"], 5);
        assert_eq!(
            page["total_display_rows"], 1,
            "matching count must not replace ordinary folded display counts"
        );
        assert!(serde_json::to_vec(&page).unwrap().len() <= 12 * 1024);
        for item in page["items"].as_array().unwrap() {
            found.push(item["item_number"].as_u64().unwrap());
        }
        k.actor_update(turn, |t| {
            t.actions[0].state = ActionState::Executed;
            t.actions[0].result = Some(page.clone());
            t.state = TurnState::Completed;
            Ok(())
        })
        .unwrap();
        if page["has_more"] == false {
            break;
        }
        let next = page["next_after"].as_u64().unwrap();
        assert!(next > after);
        after = next;
    }
    assert_eq!(found, vec![1, 2, 3, 4, 5]);
    let (_, empty) = read(
        &k,
        c,
        json!({"briefing_id":id,"after":0,"limit":4,"query":"%_不存在"}),
    );
    assert!(empty["items"].as_array().unwrap().is_empty());
    assert_eq!(empty["total_matching_items"], 0);
    assert_eq!(empty["has_more"], false);
    assert!(
        empty["query_semantics"]
            .as_str()
            .unwrap()
            .contains("not_evidence_of_absence")
    );
}

#[test]
fn briefing_phrase_lookup_validates_literal_bounds_and_accepts_legacy_calls() {
    let legacy = Tool::parse(
        "briefing_read",
        r#"{"briefing_id":null,"after":0,"limit":4}"#,
    )
    .unwrap();
    legacy.validate().unwrap();
    for query in [
        json!(null),
        json!(" 图书馆 "),
        json!("CyBeR"),
        json!("%_[]"),
    ] {
        Tool::parse(
            "briefing_read",
            &json!({"briefing_id":null,"after":0,"limit":4,"query":query}).to_string(),
        )
        .unwrap()
        .validate()
        .unwrap();
    }
    for query in [
        json!(""),
        json!("  "),
        json!("字".repeat(43)),
        json!("two\nlines"),
        json!("\u{0000}"),
    ] {
        let tool = Tool::parse(
            "briefing_read",
            &json!({"briefing_id":null,"after":0,"limit":4,"query":query}).to_string(),
        );
        assert!(tool.is_err() || tool.unwrap().validate().is_err());
    }
}

#[tokio::test]
async fn briefing_quote_excerpts_are_exact_utf8_bounded_and_do_not_truncate_report_items() {
    let (_dir, k, c, m) = many();
    let text = format!("定位测试：{}🙂🙂尾部事实。", "中".repeat(150));
    assert!(text.len() > 128 && text.len() <= 512);
    for original in m.records.iter().step_by(4) {
        k.archive_correct(
            original.record_id,
            CorrectArchiveRecord {
                expected_revision: original.revision,
                text: text.clone(),
            },
            Redactor::new(vec![]),
        )
        .unwrap();
    }
    let fresh = k
        .briefing_start(
            c,
            StartBriefing {
                request_id: RequestId::new(),
                grant_revision: 1,
                day: BriefingDay::Yesterday,
            },
        )
        .unwrap();
    let id = fresh.request.request_id;
    let (actor, _) = run_model_detail(&k, 5, true);
    actor.start_briefing_run(c, id).await.unwrap();
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    assert_eq!(
        k.briefing_run_status(c, id).unwrap().run.phase,
        BriefingRunPhase::Completed
    );
    let mut after = 0;
    let mut numbers = vec![];
    loop {
        let value = k
            .briefing_report(c, id, after, 4, Redactor::new(vec![]))
            .unwrap();
        assert!(serde_json::to_vec(&value).unwrap().len() <= 12 * 1024);
        for row in value["items"].as_array().unwrap() {
            numbers.push(row["item_number"].as_u64().unwrap());
            assert_eq!(row["summary"], "s".repeat(768));
            assert_eq!(row["uncertainty"], "u".repeat(512));
            assert_eq!(row["citations"].as_array().unwrap().len(), 8);
            for citation in row["citations"].as_array().unwrap() {
                let quote = citation["quote_excerpt"]
                    .as_str()
                    .expect("foreground needs checked source words, not only a generated summary");
                assert!(!quote.is_empty() && quote.len() <= 128 && text.starts_with(quote));
                assert_eq!(citation["quote_excerpt_truncated"], true);
                assert!(
                    citation["source_local_time"]
                        .as_str()
                        .unwrap()
                        .ends_with("UTC+08:00")
                );
            }
        }
        assert!(
            value["citation_semantics"]
                .as_str()
                .unwrap()
                .contains("not_the_complete_original")
        );
        if value["has_more"] == false {
            break;
        }
        let next = value["next_after"].as_u64().unwrap() as u16;
        assert!(next > after);
        after = next;
    }
    assert_eq!(numbers, vec![1, 2, 3, 4, 5]);
}

#[tokio::test]
async fn briefing_calendar_distinguishes_source_tomorrow_from_reply_tomorrow() {
    let (_dir, k, c, m) = many();
    let id = m.request.request_id;
    let (actor, _) = run_model(&k, 5);
    actor.start_briefing_run(c, id).await.unwrap();
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    let report = k
        .briefing_report(c, id, 0, 4, Redactor::new(vec![]))
        .unwrap();
    let calendar = &report["calendar"];
    assert!(
        calendar.is_object(),
        "the read needs separate source and answer date frames"
    );
    assert_eq!(
        calendar["source_today"].as_str(),
        report["day_from_local"]
            .as_str()
            .and_then(|s| s.split(' ').next())
    );
    assert_eq!(calendar["source_tomorrow"], calendar["reply_today"]);
    assert_ne!(calendar["source_tomorrow"], calendar["reply_tomorrow"]);
    assert_eq!(calendar["source_anchor_requires_occurrence"], true);
    let (turn, value) = read(
        &k,
        c,
        json!({"briefing_id":id,"after":0,"limit":4,"query":null}),
    );
    assert_eq!(value["calendar"], report["calendar"]);
    k.actor_update(turn, |t| {
        t.actions[0].state = ActionState::Executed;
        t.actions[0].result = Some(value);
        t.state = TurnState::Completed;
        Ok(())
    })
    .unwrap();
    let (_, original) = read_original(&k, c, id, 1, 0);
    assert_eq!(original["calendar"], report["calendar"]);
}

#[tokio::test]
async fn briefing_quote_excerpts_apply_the_current_secret_filter_before_foreground_disclosure() {
    for padding in [0, 100] {
        let (_dir, k, c, m) = many();
        let secret = "later-config-private-value";
        let text = format!(
            "定位测试：{}{secret} 是一段合成资料引用，未授予任何执行权限。",
            "x".repeat(padding)
        );
        if padding > 0 {
            let start = text.find(secret).unwrap();
            assert!(
                start < 128 && start + secret.len() > 128,
                "exercise a secret crossing the excerpt boundary"
            );
        }
        let original = &m.records[0];
        k.archive_correct(
            original.record_id,
            CorrectArchiveRecord {
                expected_revision: original.revision,
                text,
            },
            Redactor::new(vec![]),
        )
        .unwrap();
        let fresh = k
            .briefing_start(
                c,
                StartBriefing {
                    request_id: RequestId::new(),
                    grant_revision: 1,
                    day: BriefingDay::Yesterday,
                },
            )
            .unwrap();
        let id = fresh.request.request_id;
        let (actor, _) = run_model(&k, 5);
        actor.start_briefing_run(c, id).await.unwrap();
        until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
        assert!(
            k.briefing_report(c, id, 0, 4, Redactor::new(vec![secret.into()]))
                .is_err(),
            "owner reports must check the complete quotation before clipping too"
        );
        let (turn, _) = k
            .actor_submit(c, uuid::Uuid::new_v4(), "查看简报".into())
            .unwrap();
        let action = ActionId::new();
        k.actor_update(turn.id, |t| {
            t.actions.push(Action {
                id: action,
                call_id: "new-secret-filter".into(),
                tool: Tool::parse(
                    "briefing_read",
                    &json!({"briefing_id":id,"after":0,"limit":4,"query":null}).to_string(),
                )
                .unwrap(),
                state: ActionState::Executing,
                result: None,
            });
            Ok(())
        })
        .unwrap();
        assert!(
            k.briefing_actor_lookup(turn.id, action, Redactor::new(vec![secret.into()]))
                .is_err(),
            "a key registered after extraction must not escape in a new quote projection"
        );
    }
}

#[tokio::test]
async fn briefing_quote_excerpts_keep_maximum_conflict_group_readable_with_escaped_text() {
    let (_dir, k, c, _, record) = fixture();
    let now = now_ms().unwrap();
    k.archive_set_grant(
        record.source_id,
        SetArchiveGrant {
            expected_revision: 1,
            scope: Some(ArchiveScope {
                expires_at_ms: now + 600000,
                retention_ms: 86400000,
                max_records: 300,
            }),
        },
    )
    .unwrap();
    for chunk in (0..256).collect::<Vec<_>>().chunks(32) {
        let records = chunk
            .iter()
            .map(|i| ImportArchiveRecord {
                external_id: if *i == 0 {
                    "synthetic-notice-max".into()
                } else {
                    format!("max-group-{i}")
                },
                source_position: None,
                occurred_at_ms: record.occurred_at_ms,
                speaker: "2002".into(),
                kind: VisibleRecordKind::Message,
                text: if *i == 0 {
                    format!("定位测试：{} 明晚七点在图书馆讨论。", "\\".repeat(160))
                } else {
                    "明晚七点在图书馆讨论。".into()
                },
            })
            .collect();
        k.archive_import(
            record.source_id,
            ImportArchive {
                expected_revision: 2,
                records,
            },
            Redactor::new(vec![]),
        )
        .unwrap();
    }
    k.briefing_set(
        c,
        SetBriefingGrant {
            request_id: RequestId::new(),
            expected_revision: 1,
            scope: Some(BriefingScope {
                sources: vec![ArchiveRecallSource {
                    source_id: record.source_id,
                    grant_revision: 2,
                }],
                from_ms: 0,
                expires_at_ms: now + 300000,
                utc_offset_minutes: 480,
                max_records: 256,
            }),
        },
    )
    .unwrap();
    let id = k
        .briefing_start(
            c,
            StartBriefing {
                request_id: RequestId::new(),
                grant_revision: 2,
                day: BriefingDay::Yesterday,
            },
        )
        .unwrap()
        .request
        .request_id;
    let mut model = RunModel::new(64, false);
    model.max_group = true;
    model.link = true;
    model.pair_relation = crate::persistence::briefing_relations::Relation::Conflict;
    let actor = Actor::with_model(
        k.clone(),
        None,
        "fixture-max-group-token-0123456789".parse().unwrap(),
        Arc::new(model),
    );
    actor.start_briefing_run(c, id).await.unwrap();
    // 256 items take about 110s alone in a debug build; allow for parallel tests.
    tokio::time::timeout(std::time::Duration::from_secs(300), async {
        while !k.briefing_run_status(c, id).unwrap().run.phase.terminal() {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    let status = k.briefing_run_status(c, id).unwrap();
    assert_eq!(status.run.phase, BriefingRunPhase::Completed);
    let report = k
        .briefing_report(c, id, 0, 4, Redactor::new(vec![]))
        .expect("a valid maximum group must remain readable");
    assert!(serde_json::to_vec(&report).unwrap().len() <= 12 * 1024);
    let row = &report["items"][0];
    assert_eq!(row["members"].as_array().unwrap().len(), 256);
    assert_eq!(
        row["related_statement_numbers"].as_array().unwrap().len(),
        256
    );
    assert_eq!(row["summary"], "\\".repeat(768));
    assert_eq!(row["uncertainty"], "\\".repeat(512));
    assert_eq!(report["next_after"], 1);
    assert!(
        report["intake_semantics"]["receipt_clock"]
            .as_str()
            .unwrap()
            .contains("label omitted")
    );
    for citation in row["citations"].as_array().unwrap() {
        assert!(citation.get("archive_received_local_time").is_none());
        assert!(citation["received_at_ms"].is_u64());
        let quote = citation["quote_excerpt"].as_str().unwrap();
        assert!(!quote.is_empty() && quote.len() < 128 && "\\".repeat(160).starts_with(quote));
        assert_eq!(citation["quote_excerpt_truncated"], true);
    }
    let first_receipt = row["citations"][0]["received_at_ms"].clone();
    let (_, original) = read_original(&k, c, id, 1, 0);
    assert_eq!(original["record"]["received_at_ms"], first_receipt);
    assert!(original["archive_received_local_time"].is_string());
    let next = k
        .briefing_report(c, id, 1, 4, Redactor::new(vec![]))
        .unwrap();
    assert_eq!(next["items"][0]["item_number"], 2);
}
