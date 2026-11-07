use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

struct ReturnedFormatModel {
    requests: AtomicUsize,
    reads_before: usize,
    repeat: bool,
    usage: u64,
    ordinary_timeout: bool,
}
impl ActorModel for ReturnedFormatModel {
    fn respond(
        &self,
        request: Request,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<model::Reply>> + Send + '_>>
    {
        let number = self.requests.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            if self.ordinary_timeout {
                return Err(model::ProviderFailure::Timeout.into());
            }
            if number < self.reads_before {
                let id = format!("format-read-{number}");
                return Ok(model::Reply {
                    text: String::new(),
                    calls: vec![model::Call {
                        id: id.clone(),
                        name: "work_list".into(),
                        arguments: "{}".into(),
                    }],
                    usage_tokens: 5,
                    continuation: vec![
                        json!({"type":"function_call","call_id":id,"name":"work_list","arguments":"{}"}),
                    ],
                });
            }
            if number == self.reads_before || self.repeat {
                return Err(model::UnstructuredToolInvocation {
                    usage_tokens: self.usage,
                }
                .into());
            }
            assert!(request.persona.contains("invalid tool-call format"));
            assert_eq!(
                request
                    .input
                    .iter()
                    .filter(|v| v["type"] == "function_call_output")
                    .count(),
                self.reads_before,
                "format repair must retain existing receipts without repeating calls"
            );
            Ok(model::Reply {
                text: "fixture source answer".into(),
                calls: vec![],
                usage_tokens: 11,
                continuation: vec![],
            })
        })
    }
}
async fn settled_format_turn(k: &Arc<Kernel>, id: ActorTurnId) -> ActorTurn {
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let t = k.actor_turn(id).unwrap();
            if t.state != TurnState::Running {
                return t;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap()
}
fn format_fixture(
    reads_before: usize,
    repeat: bool,
    usage: u64,
    ordinary_timeout: bool,
) -> (
    tempfile::TempDir,
    Arc<Kernel>,
    Arc<Actor>,
    Arc<ReturnedFormatModel>,
) {
    let dir = tempfile::tempdir().unwrap();
    let k = Arc::new(Kernel::open(&dir.path().join("format.sqlite3")).unwrap());
    k.mark_ready().unwrap();
    let model = Arc::new(ReturnedFormatModel {
        requests: AtomicUsize::new(0),
        reads_before,
        repeat,
        usage,
        ordinary_timeout,
    });
    let actor = Actor::with_model(
        k.clone(),
        None,
        "fixture-format-owner-12345678901234567890".parse().unwrap(),
        model.clone(),
    );
    (dir, k, actor, model)
}
#[tokio::test]
async fn returned_format_error_gets_one_accounted_repair_without_replaying_reads() {
    for reads in [0, 1] {
        let (_dir, k, actor, model) = format_fixture(reads, false, 7, false);
        let c = k.create_conversation().unwrap().id;
        let first = actor
            .submit(c, Uuid::new_v4(), "fixture original question".into())
            .await
            .unwrap();
        let t = settled_format_turn(&k, first.id).await;
        assert_eq!(t.state, TurnState::Completed);
        assert_eq!(t.model_requests, reads as u32 + 2);
        assert_eq!(t.usage_tokens, reads as u64 * 5 + 18);
        assert_eq!(t.actions.len(), reads);
        assert!(t.actions.iter().all(|a| a.state == ActionState::Executed));
        assert_eq!(model.requests.load(Ordering::SeqCst), reads + 2);
        assert_eq!(
            t.messages
                .iter()
                .filter(|m| m.role == MessageRole::Tool && m.content.contains("格式修复"))
                .count(),
            1,
            "the repair must be observable"
        );
        let ledger = k.inference_snapshot().unwrap();
        assert_eq!(ledger.reserved_requests, reads as u64 + 2);
        assert_eq!(ledger.reported_usage_tokens_sum, t.usage_tokens);
    }
}
#[tokio::test]
async fn returned_format_error_stops_after_one_repair_and_preserves_both_usages() {
    let (_dir, k, actor, model) = format_fixture(0, true, 7, false);
    let c = k.create_conversation().unwrap().id;
    let first = actor
        .submit(c, Uuid::new_v4(), "fixture original question".into())
        .await
        .unwrap();
    let t = settled_format_turn(&k, first.id).await;
    assert_eq!(t.state, TurnState::Unknown);
    assert_eq!(model.requests.load(Ordering::SeqCst), 2);
    assert_eq!(t.usage_tokens, 14);
    assert!(t.error.as_deref().unwrap().contains("一次格式修复"));
    assert!(t.actions.is_empty());
    assert_eq!(
        k.inference_snapshot().unwrap().reported_usage_tokens_sum,
        14
    );
}
#[tokio::test]
async fn returned_format_error_obeys_token_request_and_normal_failure_limits() {
    for (reads, usage, cap, timeout, expected) in [
        (0, 1200, 1024, false, TurnState::Failed),
        (3, 7, 32768, false, TurnState::Unknown),
        (0, 7, 32768, true, TurnState::Unknown),
    ] {
        let (_dir, k, mut actor, model) = format_fixture(reads, false, usage, timeout);
        Arc::get_mut(&mut actor).unwrap().max_tokens = cap;
        let c = k.create_conversation().unwrap().id;
        let first = actor
            .submit(c, Uuid::new_v4(), "fixture original question".into())
            .await
            .unwrap();
        let t = settled_format_turn(&k, first.id).await;
        assert_eq!(t.state, expected);
        assert_eq!(
            model.requests.load(Ordering::SeqCst),
            reads + 1,
            "no extra request beyond the remaining budget or after a timeout"
        );
        if !timeout {
            assert_eq!(t.usage_tokens, reads as u64 * 5 + usage);
        }
        assert!(!t.messages.iter().any(|m| m.content.contains("格式修复")));
    }
}
#[tokio::test]
async fn returned_format_error_is_not_repaired_after_effectful_or_uncertain_receipts() {
    use tools::{Empty, SessionArg, Tool};
    for (tool, state) in [
        (
            Tool::WorkCancel(SessionArg {
                session_id: crate::domain::WorkSessionId::new(),
            }),
            ActionState::Executed,
        ),
        (Tool::BriefingYesterday(Empty {}), ActionState::Executed),
        (Tool::WorkList(Empty {}), ActionState::Unknown),
        (Tool::WorkList(Empty {}), ActionState::Executing),
    ] {
        let (_dir, k, actor, model) = format_fixture(0, false, 7, false);
        let c = k.create_conversation().unwrap().id;
        let (t, _) = k
            .actor_submit(c, Uuid::new_v4(), "fixture receipt guard".into())
            .unwrap();
        k.actor_update(t.id, |t| {
            t.actions.push(domain::Action {
                id: lain_wire::ActionId::new(),
                call_id: "preceding-receipt".into(),
                tool,
                state,
                result: (state == ActionState::Executed).then(|| json!({"fixture_receipt":true})),
            });
            Ok(())
        })
        .unwrap();
        actor.launch(t.id, None).await;
        let t = settled_format_turn(&k, t.id).await;
        assert_eq!(t.state, TurnState::Unknown);
        assert_eq!(model.requests.load(Ordering::SeqCst), 1);
        assert_eq!(t.usage_tokens, 7);
        assert_eq!(t.actions.len(), 1);
    }
}

struct BatchModel {
    requests: AtomicUsize,
    count: usize,
    mutation_at: Option<usize>,
    second_batch: bool,
    workspace: String,
}
impl ActorModel for BatchModel {
    fn respond(
        &self,
        request: Request,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<model::Reply>> + Send + '_>>
    {
        let number = self.requests.fetch_add(1, Ordering::SeqCst);
        let first = number == 0;
        Box::pin(async move {
            if !first {
                if self.count == MAX_TOOLS && self.mutation_at.is_none() && !self.second_batch {
                    assert!(
                        request.tools.is_empty(),
                        "the final prose request has no remaining tool allowance"
                    );
                    assert!(
                        request
                            .persona
                            .contains("Remaining tool calls in this turn: 0.")
                    );
                }
                ensure!(
                    request
                        .input
                        .iter()
                        .filter(|i| i["type"] == "function_call_output")
                        .count()
                        == self.count,
                    "every call needs its own result"
                );
                let ids = |kind| {
                    request
                        .input
                        .iter()
                        .filter(|i| i["type"] == kind)
                        .map(|i| i["call_id"].as_str().unwrap())
                        .collect::<std::collections::BTreeSet<_>>()
                };
                assert_eq!(
                    ids("function_call"),
                    ids("function_call_output"),
                    "the next Responses input must pair every original call identity"
                );
            }
            let calls: Vec<model::Call> = if first || self.second_batch {
                (0..self.count).map(|i| model::Call {
                id:format!("batch-{number}-{i}"),
                name: if self.mutation_at==Some(i) { "work_create" } else if i%2==0 { "work_list" } else { "device_work_list" }.into(),
                arguments: if self.mutation_at==Some(i) { json!({"workspace":self.workspace,"profile_id":"codex/local","sandbox":"read_only","task":"fixture task"}).to_string() } else { "{}".into() },
            }).collect()
            } else {
                vec![]
            };
            let continuation = calls.iter().map(|c| json!({"type":"function_call","call_id":c.id,"name":c.name,"arguments":c.arguments})).collect();
            Ok(model::Reply {
                text: if first { "" } else { "batch query complete" }.into(),
                calls,
                usage_tokens: 10,
                continuation,
            })
        })
    }
}
#[tokio::test]
async fn parallel_read_batch_is_serialized_and_mutation_stops_for_exact_confirmation() {
    for (count, mutation_at, second_batch) in [
        (2, None, false),
        (4, None, false),
        (2, Some(0), false),
        (3, Some(1), false),
        (5, None, false),
        (3, None, true),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let kernel = Arc::new(Kernel::open(&dir.path().join("batch.sqlite3")).unwrap());
        kernel.mark_ready().unwrap();
        let model = Arc::new(BatchModel {
            requests: AtomicUsize::new(0),
            count,
            mutation_at,
            second_batch,
            workspace: dir.path().to_str().unwrap().into(),
        });
        let actor = Actor::with_model(
            kernel.clone(),
            None,
            "fixture-batch-owner-12345678901234567890".parse().unwrap(),
            model.clone(),
        );
        let conversation = kernel.create_conversation().unwrap();
        let submitted = actor
            .submit(
                conversation.id,
                Uuid::new_v4(),
                "list selected worker facts".into(),
            )
            .await
            .unwrap();
        let turn = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let t = kernel.actor_turn(submitted.id).unwrap();
                if t.state != TurnState::Running {
                    break t;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        if count > MAX_TOOLS {
            assert_eq!(turn.state, TurnState::Unknown);
            assert!(turn.actions.is_empty());
            assert!(
                turn.error
                    .as_deref()
                    .unwrap()
                    .contains("工具调用数量超过本轮剩余额度")
            );
        } else if let Some(index) = mutation_at {
            assert_eq!(turn.state, TurnState::PendingConfirmation);
            assert_eq!(turn.actions.len(), index + 1);
            assert!(
                turn.actions[..index]
                    .iter()
                    .all(|a| a.state == ActionState::Executed)
            );
            assert_eq!(turn.actions[index].state, ActionState::Proposed);
            assert_eq!(model.requests.load(Ordering::SeqCst), 1);
        } else if second_batch {
            assert_eq!(turn.state, TurnState::Unknown);
            assert!(
                turn.error
                    .as_deref()
                    .unwrap()
                    .contains("工具调用数量超过本轮剩余额度")
            );
            assert_eq!(
                turn.actions.len(),
                count,
                "an oversized second batch cannot consume remaining slots partially"
            );
            assert_eq!(model.requests.load(Ordering::SeqCst), 2);
        } else {
            assert_eq!(turn.state, TurnState::Completed);
            assert_eq!(turn.actions.len(), count);
            assert!(
                turn.actions
                    .iter()
                    .all(|a| a.state == ActionState::Executed)
            );
            assert_eq!(model.requests.load(Ordering::SeqCst), 2);
        }
    }
}

struct TwoStepModel(AtomicUsize);
impl ActorModel for TwoStepModel {
    fn respond(
        &self,
        _: Request,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<model::Reply>> + Send + '_>>
    {
        let first = self.0.fetch_add(1, Ordering::SeqCst) == 0;
        Box::pin(async move {
            Ok(model::Reply {
                text: if first { "" } else { "finished-fixture-query" }.into(),
                calls: if first {
                    vec![model::Call {
                    id: "fixture-read".into(), name: "memory_search".into(),
                    arguments: json!({"scope":{"kind":"global"},"kind":"preference","status":"active","text":"name preferred fixture","after":0,"limit":10}).to_string(),
                }]
                } else {
                    vec![]
                },
                continuation: vec![],
                usage_tokens: if first { 5085 } else { 5261 },
            })
        })
    }
}

#[tokio::test]
async fn multi_step_budget_records_all_returned_usage_and_keeps_read_receipt() {
    for (cap, expected) in [(8192, TurnState::Failed), (32768, TurnState::Completed)] {
        let dir = tempfile::tempdir().unwrap();
        let kernel = Arc::new(Kernel::open(&dir.path().join("fixture.sqlite3")).unwrap());
        kernel.mark_ready().unwrap();
        let model = Arc::new(TwoStepModel(AtomicUsize::new(0)));
        let mut actor = Actor::with_model(
            kernel.clone(),
            None,
            "fixture-budget-owner-12345678901234567890".parse().unwrap(),
            model.clone(),
        );
        Arc::get_mut(&mut actor).unwrap().max_tokens = cap;
        let conversation = kernel.create_conversation().unwrap();
        let submitted = actor
            .submit(
                conversation.id,
                Uuid::new_v4(),
                "look up the fixture name".into(),
            )
            .await
            .unwrap();
        let turn = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let turn = kernel.actor_turn(submitted.id).unwrap();
                if turn.state != TurnState::Running {
                    break turn;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            turn.usage_tokens, 10346,
            "returned usage must survive a cap failure"
        );
        assert_eq!(turn.state, expected);
        assert_eq!(turn.model_requests, 2);
        assert_eq!(model.0.load(Ordering::SeqCst), 2);
        assert_eq!(turn.actions.len(), 1);
        assert_eq!(turn.actions[0].state, ActionState::Executed);
        assert_eq!(
            turn.messages
                .iter()
                .any(|m| m.content == "finished-fixture-query"),
            cap == 32768
        );
        if cap == 8192 {
            assert!(turn.error.as_deref().unwrap().contains("token"));
        }
    }
}

#[tokio::test]
async fn memory_query_receipt_identifies_literal_filter_and_limited_coverage() {
    let dir = tempfile::tempdir().unwrap();
    let kernel = Arc::new(Kernel::open(&dir.path().join("fixture.sqlite3")).unwrap());
    kernel.mark_ready().unwrap();
    let conversation = kernel.create_conversation().unwrap();
    use crate::memory::{AddMemory, MemoryDraft, MemoryKind, MemoryScope};
    let saved = kernel
        .add_memory(AddMemory {
            request_id: lain_wire::RequestId::new(),
            draft: MemoryDraft {
                content: "Preferred name: Mira".into(),
                kind: MemoryKind::Preference,
                scope: MemoryScope::Global,
            },
        })
        .unwrap();
    let tool = tools::Tool::parse("memory_search", &json!({"scope":{"kind":"global"},"kind":"preference","status":"active","text":"name preferred fixture","after":0,"limit":10}).to_string()).unwrap();
    let submitted = kernel
        .actor_submit(conversation.id, Uuid::new_v4(), "fixture".into())
        .unwrap();
    let result = tools::read(&kernel, None, submitted.0.id, &tool)
        .await
        .unwrap();
    assert_eq!(result["query_semantics"]["text_match"], "literal_substring");
    assert_eq!(result["query_semantics"]["coverage"], "filtered_page_only");
    assert_eq!(result["query"]["text"], "name preferred fixture");
    assert_eq!(result["page"]["records"], json!([]));
    let short = tools::Tool::parse("memory_search", &json!({"scope":{"kind":"global"},"kind":"preference","status":"active","text":"Mira","after":0,"limit":10}).to_string()).unwrap();
    let matched = tools::read(&kernel, None, submitted.0.id, &short)
        .await
        .unwrap();
    assert_eq!(matched["page"]["records"][0]["id"], json!(saved.id));
    // The former model-facing list name remains a typed, persisted compatibility
    // alias. Null-text listing and literal filtering must use the same current
    // Kernel checks and return the same complete evidence, never broader scope.
    for text in [None, Some("Mira"), Some("not present")] {
        let args = json!({"scope":{"kind":"global"},"kind":"preference",
            "status":"active","text":text,"after":0,"limit":10})
        .to_string();
        let canonical = tools::Tool::parse("memory_search", &args).unwrap();
        let legacy = tools::Tool::parse("memory_list", &args).unwrap();
        let canonical_value = tools::read(&kernel, None, submitted.0.id, &canonical)
            .await
            .unwrap();
        let legacy_value = tools::read(&kernel, None, submitted.0.id, &legacy)
            .await
            .unwrap();
        assert_eq!(canonical_value, legacy_value);
        let count = canonical_value["page"]["records"].as_array().unwrap().len();
        assert_eq!(count, usize::from(text != Some("not present")));
        let encoded = serde_json::to_string(&legacy).unwrap();
        let restored: tools::Tool = serde_json::from_str(&encoded).unwrap();
        assert!(
            matches!(restored, tools::Tool::MemoryList(_)),
            "old action identity must survive restore"
        );
    }
    let foreign = kernel.create_conversation().unwrap();
    for name in ["memory_search", "memory_list"] {
        let denied = tools::Tool::parse(
            name,
            &json!({"scope":{"kind":"conversation",
            "conversation_id":foreign.id},"kind":null,"status":"active",
            "text":null,"after":0,"limit":10})
            .to_string(),
        )
        .unwrap();
        assert!(
            tools::read(&kernel, None, submitted.0.id, &denied)
                .await
                .is_err(),
            "canonicalization must not permit another conversation's scope"
        );
    }
}

struct BriefingContextModel {
    requests: std::sync::Mutex<Vec<Request>>,
    read_scopes_first: Option<ConversationId>,
    answer: &'static str,
}
impl ActorModel for BriefingContextModel {
    fn respond(
        &self,
        request: Request,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<model::Reply>> + Send + '_>>
    {
        let first = {
            let mut requests = self.requests.lock().unwrap();
            let first = requests.is_empty();
            requests.push(request);
            first
        };
        Box::pin(async move {
            let calls: Vec<model::Call> = if first {
                self.read_scopes_first
                    .map(|conversation_id| {
                        [
                            json!({"kind":"conversation","conversation_id":conversation_id}),
                            json!({"kind":"global"}),
                            json!({"kind":"internal"}),
                        ]
                        .into_iter()
                        .enumerate()
                        .map(|(index, scope)| model::Call {
                            id: format!("scoped-memory-{index}"),
                            name: "memory_list".into(),
                            arguments: json!({"scope":scope,"kind":null,"status":"active",
                                "text":null,"after":0,"limit":10})
                            .to_string(),
                        })
                        .collect()
                    })
                    .unwrap_or_default()
            } else {
                vec![]
            };
            let continuation = calls
                .iter()
                .map(|call| {
                    json!({"type":"function_call","call_id":call.id,
                        "name":call.name,"arguments":call.arguments})
                })
                .collect();
            Ok(model::Reply {
                text: if calls.is_empty() { self.answer } else { "" }.into(),
                calls,
                continuation,
                usage_tokens: 10,
            })
        })
    }
}

async fn wait_for_context_turn(kernel: &Kernel, id: ActorTurnId) -> domain::ActorTurn {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let turn = kernel.actor_turn(id).unwrap();
            if turn.state != TurnState::Running {
                break turn;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn ordinary_yesterday_query_without_grant_has_explicit_unavailable_context() {
    let dir = tempfile::tempdir().unwrap();
    let kernel = Arc::new(Kernel::open(&dir.path().join("no-briefing.sqlite3")).unwrap());
    kernel.mark_ready().unwrap();
    let conversation = kernel.create_conversation().unwrap();
    let answer = "当前会话没有可用的昨日消息来源，我无法据此判断昨天有哪些重要消息。";
    let model = Arc::new(BriefingContextModel {
        requests: std::sync::Mutex::new(vec![]),
        read_scopes_first: None,
        answer,
    });
    let actor = Actor::with_model(
        kernel.clone(),
        None,
        "fixture-context-owner-12345678901234567890"
            .parse()
            .unwrap(),
        model.clone(),
    );
    let submitted = actor
        .submit(
            conversation.id,
            Uuid::new_v4(),
            "昨天有什么重要消息？".into(),
        )
        .await
        .unwrap();
    let turn = wait_for_context_turn(&kernel, submitted.id).await;
    assert_eq!(turn.state, TurnState::Completed);
    assert!(turn.error.is_none());
    assert!(turn.actions.is_empty());
    assert!(
        turn.messages
            .iter()
            .any(|message| { message.role == MessageRole::Assistant && message.content == answer })
    );
    let requests = model.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert!(request.persona.contains(
        "Kernel daily-briefing access: unavailable for this conversation at this request."
    ));
    assert!(
        !request
            .persona
            .contains("This conversation has independently scoped daily-source access:")
    );
    assert!(
        request
            .tools
            .iter()
            .all(|tool| { !tool["name"].as_str().unwrap().starts_with("briefing_") })
    );
    assert!(
        request
            .input
            .iter()
            .any(|item| item["content"] == "昨天有什么重要消息？")
    );
}

#[tokio::test]
async fn three_memory_reads_update_remaining_allowances_and_preserve_every_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let kernel = Arc::new(Kernel::open(&dir.path().join("remaining-budget.sqlite3")).unwrap());
    kernel.mark_ready().unwrap();
    let conversation = kernel.create_conversation().unwrap();
    let answer = "这些记忆页为空；这不能说明昨天没有重要消息，当前会话也没有可用的简报来源。";
    let model = Arc::new(BriefingContextModel {
        requests: std::sync::Mutex::new(vec![]),
        read_scopes_first: Some(conversation.id),
        answer,
    });
    let actor = Actor::with_model(
        kernel.clone(),
        None,
        "fixture-context-owner-12345678901234567890"
            .parse()
            .unwrap(),
        model.clone(),
    );
    let submitted = actor
        .submit(
            conversation.id,
            Uuid::new_v4(),
            "昨天有什么重要消息？".into(),
        )
        .await
        .unwrap();
    let turn = wait_for_context_turn(&kernel, submitted.id).await;
    assert_eq!(turn.state, TurnState::Completed);
    assert_eq!(turn.model_requests, 2);
    assert!(turn.error.is_none());
    assert_eq!(turn.actions.len(), 3);
    assert!(
        turn.messages
            .iter()
            .any(|message| { message.role == MessageRole::Assistant && message.content == answer })
    );
    let requests = model.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(
        requests[0]
            .persona
            .contains("Remaining tool calls in this turn: 4.")
    );
    assert!(
        requests[0]
            .persona
            .contains("Remaining model requests after this request: 3.")
    );
    assert!(
        requests[1]
            .persona
            .contains("Remaining tool calls in this turn: 1.")
    );
    assert!(
        requests[1]
            .persona
            .contains("Remaining model requests after this request: 2.")
    );
    for (index, action) in turn.actions.iter().enumerate() {
        assert_eq!(action.call_id, format!("scoped-memory-{index}"));
        assert!(matches!(action.tool, tools::Tool::MemoryList(_)));
        assert_eq!(action.state, ActionState::Executed);
        let receipt = action.result.as_ref().unwrap();
        assert_eq!(receipt["context_role"], "untrusted_memory_evidence");
        assert_eq!(receipt["page"]["records"], json!([]));
        let output = requests[1]
            .input
            .iter()
            .find(|item| {
                item["type"] == "function_call_output" && item["call_id"] == action.call_id
            })
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(output["output"].as_str().unwrap()).unwrap(),
            *receipt
        );
        assert!(
            requests[1].input.iter().any(|item| {
                item["type"] == "function_call" && item["call_id"] == action.call_id
            })
        );
    }
}

#[tokio::test]
async fn ordinary_conversation_with_current_briefing_grant_keeps_positive_access_context() {
    use lain_wire::{archives::*, briefing::*};
    let dir = tempfile::tempdir().unwrap();
    let kernel = Arc::new(Kernel::open(&dir.path().join("granted-briefing.sqlite3")).unwrap());
    kernel.mark_ready().unwrap();
    let conversation = kernel.create_conversation().unwrap();
    let now = crate::domain::now_ms().unwrap();
    let source = kernel
        .archive_register(
            ArchiveSourceKey::QqGroup {
                observer_account: "1001".into(),
                group_id: "3003".into(),
            },
            crate::redaction::Redactor::new(vec![]),
        )
        .unwrap()
        .source
        .id;
    kernel
        .archive_set_grant(
            source,
            SetArchiveGrant {
                expected_revision: 0,
                scope: Some(ArchiveScope {
                    expires_at_ms: now + 600000,
                    retention_ms: 86400000,
                    max_records: 100,
                }),
            },
        )
        .unwrap();
    kernel
        .briefing_set(
            conversation.id,
            SetBriefingGrant {
                request_id: lain_wire::RequestId::new(),
                expected_revision: 0,
                scope: Some(BriefingScope {
                    sources: vec![ArchiveRecallSource {
                        source_id: source,
                        grant_revision: 1,
                    }],
                    from_ms: 0,
                    expires_at_ms: now + 300000,
                    utc_offset_minutes: 480,
                    max_records: 100,
                }),
            },
        )
        .unwrap();
    let model = Arc::new(BriefingContextModel {
        requests: std::sync::Mutex::new(vec![]),
        read_scopes_first: None,
        answer: "可以从当前授权的来源读取昨日消息。",
    });
    let actor = Actor::with_model(
        kernel.clone(),
        None,
        "fixture-context-owner-12345678901234567890"
            .parse()
            .unwrap(),
        model.clone(),
    );
    let submitted = actor
        .submit(conversation.id, Uuid::new_v4(), "能读哪些消息？".into())
        .await
        .unwrap();
    let turn = wait_for_context_turn(&kernel, submitted.id).await;
    assert_eq!(turn.state, TurnState::Completed);
    let requests = model.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert!(
        requests[0]
            .persona
            .contains("This conversation has independently scoped daily-source access:")
    );
    assert!(!requests[0].persona.contains(
        "Kernel daily-briefing access: unavailable for this conversation at this request."
    ));
    for name in ["briefing_yesterday", "briefing_read", "briefing_inspect"] {
        assert!(requests[0].tools.iter().any(|tool| tool["name"] == name));
    }
}
