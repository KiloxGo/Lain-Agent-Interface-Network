use super::*;

struct RepeatedReadModel {
    requests: Mutex<Vec<Request>>,
    briefing: RequestId,
    kernel: Arc<Kernel>,
    conversation: ConversationId,
    scopes: Mutex<Vec<serde_json::Value>>,
}
impl ActorModel for RepeatedReadModel {
    fn respond(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = Result<Reply>> + Send + '_>> {
        let active = self
            .kernel
            .actor_recent(self.conversation)
            .unwrap()
            .into_iter()
            .find(|t| t.state.active())
            .unwrap();
        let mut scope = self.kernel.briefing_actor_scope(active.id).unwrap();
        scope.as_object_mut().unwrap().remove("reply_clock");
        self.scopes.lock().unwrap().push(scope);
        let mut requests = self.requests.lock().unwrap();
        requests.push(request);
        let n = requests.len();
        let calls = if n <= 3 {
            vec![Call {
                id: format!("repeated-read-{n}"),
                name: "briefing_read".into(),
                arguments: json!({"briefing_id":self.briefing,"after":0,"limit":4,
                    "query":match n {1=>Some("图书馆"),2=>Some("没有的词"),_=>None}})
                .to_string(),
            }]
        } else {
            vec![]
        };
        Box::pin(async move {
            Ok(Reply {
                text: if n > 3 {
                    json!({"briefing_answer":{"items":[{"briefing_id":self.briefing,"item_number":1}]}}).to_string()
                } else {
                    String::new()
                },
                usage_tokens: 7,
                continuation: vec![],
                calls,
            })
        })
    }
}

#[tokio::test]
async fn actual_foreground_repeated_reads_share_metadata_without_rewriting_receipts() {
    let (_dir, k, c, manifest, _) = fixture();
    let (processor, _) = run_model(&k, 1);
    let id = manifest.request.request_id;
    processor.start_briefing_run(c, id).await.unwrap();
    until(|| k.briefing_run_status(c, id).unwrap().run.phase.terminal()).await;
    let model = Arc::new(RepeatedReadModel {
        requests: Mutex::new(vec![]),
        briefing: id,
        kernel: k.clone(),
        conversation: c,
        scopes: Mutex::new(vec![]),
    });
    let actor = Actor::with_model(
        k.clone(),
        None,
        "fixture-context-growth-token-0123456789012345"
            .parse()
            .unwrap(),
        model.clone(),
    );
    let turn = actor
        .submit(c, uuid::Uuid::new_v4(), "原通知分别几点发的？只读。".into())
        .await
        .unwrap();
    until(|| !k.actor_turn(turn.id).unwrap().state.active()).await;
    let turn = k.actor_turn(turn.id).unwrap();
    assert_eq!(turn.state, TurnState::Completed);
    assert_eq!(turn.actions.len(), 3);
    assert_eq!(turn.model_requests, 4);
    let requests = model.requests.lock().unwrap();
    assert_eq!(requests.len(), 4);
    assert!(!requests[0].persona.contains("For this source-only turn"));
    assert!(
        requests[1..]
            .iter()
            .all(|r| r.persona.contains("For this source-only turn"))
    );
    let scopes = model.scopes.lock().unwrap();
    assert_eq!(scopes.len(), requests.len());
    assert!(
        serde_json::to_vec(&requests[0].tools).unwrap().len() <= 12_000,
        "duplicated schemas/descriptions crowd out source evidence: {}",
        serde_json::to_vec(&requests[0].tools).unwrap().len()
    );
    assert!(
        requests[0]
            .tools
            .iter()
            .any(|tool| tool["name"] == "memory_search")
    );
    assert!(
        !requests[0]
            .tools
            .iter()
            .any(|tool| tool["name"] == "memory_list"),
        "one offered query must support both literal filtering and null-text listing"
    );
    for tool in &requests[0].tools {
        eprintln!(
            "tool_definition {}",
            json!({"name":tool["name"],
            "bytes":serde_json::to_vec(tool).unwrap().len(),
            "description_bytes":tool["description"].as_str().map_or(0, str::len),
            "schema_bytes":serde_json::to_vec(&tool["parameters"]).unwrap().len()})
        );
    }
    for (index, (request, actual_scope)) in requests.iter().zip(scopes.iter()).enumerate() {
        eprintln!(
            "ordinary_source_request persona_bytes={} input_bytes={} tools_bytes={}",
            request.persona.len(),
            serde_json::to_vec(&request.input).unwrap().len(),
            serde_json::to_vec(&request.tools).unwrap().len()
        );
        assert!(
            request.persona.len() <= 11_500,
            "repeated ordinary source guidance crowds out evidence in a multi-request turn: {} bytes",
            request.persona.len()
        );
        let source_rules = request
            .persona
            .split_once("Daily-source conversation rules:\n")
            .expect("ordinary source guidance must be present in the actual request")
            .1
            .split_once("\nEnd daily-source conversation rules.")
            .unwrap()
            .0;
        assert!(
            source_rules.len() <= 4500,
            "source policy must leave room for actual current evidence: {}",
            source_rules.len()
        );
        assert!(source_rules.contains("target_resolution=not_attempted does not establish target existence or source membership"),
            "an unattempted reference must not become a negative existence claim");
        assert!(
            !request.persona.contains("Source interpretation:"),
            "ordinary requests must not append the old duplicate interpretation block"
        );
        let scope_prefix = "This conversation has independently scoped daily-source access: ";
        assert_eq!(request.persona.matches(scope_prefix).count(), 1);
        let scope_text = request.persona.split_once(scope_prefix).unwrap().1;
        let scope = serde_json::Deserializer::from_str(scope_text)
            .into_iter::<serde_json::Value>()
            .next()
            .unwrap()
            .unwrap();
        assert_eq!(
            &scope, actual_scope,
            "scope must remain complete, not summarized"
        );
        if index < 3 {
            for tool in [
                "briefing_yesterday",
                "briefing_read",
                "briefing_inspect",
                "briefing_remember",
                "device_list",
                "work_list",
            ] {
                assert!(
                    request.tools.iter().any(|t| t["name"] == tool),
                    "source guidance slimming must not remove offered capability {tool}"
                );
            }
        } else {
            assert_eq!(
                request.tools.len(),
                1,
                "existing tool allowance must still fence execution"
            );
            assert_eq!(request.tools[0]["name"], "briefing_answer");
        }
        let line = request.persona.lines().find_map(|line|
            line.strip_prefix("Kernel current reply clock: "))
            .expect("every model round needs its current reply frame, independently of old read receipts");
        let clock: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(clock["utc_offset_minutes"], 480);
        let observed = clock["observed_at_ms"].as_u64().unwrap();
        let date =
            time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(observed) * 1_000_000)
                .unwrap()
                .checked_add(time::Duration::hours(8))
                .unwrap()
                .date();
        assert_eq!(clock["today"], date.to_string());
        assert_eq!(clock["tomorrow"], date.next_day().unwrap().to_string());
    }
    let mut resolved = std::collections::BTreeMap::<String, serde_json::Value>::new();
    let mut references = 0;
    let mut intake_references = 0;
    let mut speaker_references = 0;
    for message in &requests[3].input {
        if message["type"] != "function_call_output" {
            continue;
        }
        let mut value: serde_json::Value =
            serde_json::from_str(message["output"].as_str().unwrap()).unwrap();
        if value["intake_semantics"]["same_field_value_as_call"].is_string() {
            intake_references += 1;
        }
        if value["speaker_attribution"]["same_field_value_as_call"].is_string() {
            speaker_references += 1;
        }
        for (key, field) in value.as_object_mut().unwrap() {
            if let Some(call) = field
                .get("same_field_value_as_call")
                .and_then(serde_json::Value::as_str)
            {
                *field = resolved
                    .get(call)
                    .expect("reference points outside the current request")[key]
                    .clone();
                references += 1;
            }
        }
        let call = message["call_id"].as_str().unwrap();
        let receipt = turn
            .actions
            .iter()
            .find(|a| a.call_id == call)
            .unwrap()
            .result
            .as_ref()
            .unwrap();
        assert_eq!(value["calendar"], receipt["calendar"]);
        assert_eq!(value["intake_semantics"], receipt["intake_semantics"]);
        assert_eq!(value["speaker_attribution"], receipt["speaker_attribution"]);
        assert_eq!(value["status"], receipt["status"]);
        assert_eq!(value["has_more"], receipt["has_more"]);
        assert_eq!(value["query"], receipt["query"]);
        let got = value["items"].as_array().unwrap();
        let full = receipt["items"].as_array().unwrap();
        assert_eq!(got.len(), full.len());
        for (a, b) in got.iter().zip(full) {
            assert_eq!(a["summary"], b["summary"]);
            assert_eq!(a["uncertainty"], b["uncertainty"]);
            assert_eq!(
                a["citations"][0]["quote_excerpt"],
                b["citations"][0]["quote_excerpt"]
            );
        }
        assert!(
            !receipt.to_string().contains("same_field_value_as_call"),
            "persisted receipt was replaced by references"
        );
        resolved.insert(call.into(), value);
    }
    assert!(
        references > 0,
        "the actual model request bypassed the projection"
    );
    assert_eq!(resolved.len(), 3);
    assert_eq!(
        intake_references, 2,
        "repeated intake explanation must not grow with each read"
    );
    assert_eq!(
        speaker_references, 2,
        "speaker attribution limits must be shared without changing full receipts"
    );
    assert!(
        resolved["repeated-read-2"]["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(!resolved["repeated-read-2"]["query_semantics"].is_null());
}
