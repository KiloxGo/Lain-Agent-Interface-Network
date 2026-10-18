//! Deterministic subprocess fixture: no model, network, or real tools.
use serde_json::{Value, json};
use std::{
    io::{self, BufRead, Write},
    path::PathBuf,
};

fn emit(value: Value) {
    println!("{value}");
    io::stdout().flush().unwrap();
}
fn result(id: &Value, value: Value) {
    emit(json!({"id":id,"result":value}));
}
fn note(method: &str, params: Value) {
    emit(json!({"method":method,"params":params}));
}

fn main() {
    if std::env::args().any(|arg| arg == "--version") {
        println!("codex-cli 0.160.0");
        return;
    }
    let mut initialized = false;
    let mut ready = false;
    let mut thread = String::new();
    let mut workspace = PathBuf::new();
    let mut turns = Vec::<Value>::new();
    let mut approval = Value::Null;
    let mut mode = String::new();
    let args: Vec<String> = std::env::args().collect();
    let config: Vec<String> = args
        .windows(2)
        .filter(|p| p[0] == "-c")
        .map(|p| p[1].clone())
        .collect();
    for line in io::stdin().lock().lines() {
        let request: Value = serde_json::from_str(&line.unwrap()).unwrap();
        let id = &request["id"];
        let params = &request["params"];
        match request["method"].as_str() {
            Some("initialize") => {
                assert!(!initialized);
                initialized = true;
                result(
                    id,
                    json!({"userAgent":"codex/0.160.0","platformFamily":"test","platformOs":"test","codexHome":"test"}),
                );
            }
            Some("initialized") => {
                assert!(initialized);
                ready = true;
            }
            Some("thread/start") | Some("thread/resume") => {
                assert!(ready);
                assert!(matches!(
                    params["approvalPolicy"].as_str(),
                    Some("on-request" | "untrusted")
                ));
                assert_eq!(params["approvalsReviewer"], "user");
                workspace = PathBuf::from(params["cwd"].as_str().unwrap());
                assert!(std::env::var_os("LAIN_API_TOKEN").is_none());
                assert!(
                    std::env::vars_os()
                        .all(|(k, _)| !k.to_string_lossy().starts_with("LAIN_SECRET_"))
                );
                if params["modelProvider"] == "lain_runtime" {
                    let credential = std::env::var("LAIN_PROFILE_CREDENTIAL").unwrap();
                    match params["model"].as_str() {
                        Some("model-a") => assert!(credential == "fixture-private-alpha-value"),
                        Some("model-b") => assert!(credential == "fixture-private-beta-value"),
                        _ => {}
                    }
                }
                std::fs::write(workspace.join("fixture-config.json"), json!({"args":config,"model":params["model"],"provider":params["modelProvider"],"sandbox":params["sandbox"],"approval_policy":params["approvalPolicy"],"config":params["config"]}).to_string()).unwrap();
                if request["method"] == "thread/resume" {
                    let saved: Value = serde_json::from_str(
                        &std::fs::read_to_string(workspace.join("fixture-history.json")).unwrap(),
                    )
                    .unwrap();
                    thread = saved["id"].as_str().unwrap().into();
                    assert_eq!(params["threadId"], thread);
                    turns = saved["turns"].as_array().unwrap().clone();
                    for turn in &mut turns {
                        if turn["status"] == "inProgress" {
                            turn["status"] = json!("interrupted");
                        }
                    }
                } else {
                    thread = uuid::Uuid::new_v4().to_string();
                }
                result(
                    id,
                    json!({"model":params["model"].as_str().unwrap_or("fixture-default"),"reasoningEffort":params["config"]["model_reasoning_effort"].as_str().unwrap_or("high"),"thread":{"id":thread,"turns":turns,"status":{"type":"idle"}}}),
                );
            }
            Some("turn/start") => {
                assert_eq!(params["threadId"], thread);
                mode = params["input"][0]["text"].as_str().unwrap().into();
                if mode == "reject" {
                    emit(json!({"id":id,"error":{"code":-32600,"message":"secret=do-not-expose"}}));
                    continue;
                }
                let turn = json!({"id":uuid::Uuid::new_v4().to_string(),"status":"inProgress","items":[],"error":null});
                turns.push(turn.clone());
                persist(&workspace, &thread, &turns);
                // Deliberately notify before replying; the driver must not lose identity/order.
                note("turn/started", json!({"threadId":thread,"turn":turn}));
                if mode == "lost-prompt-ack" {
                    std::process::exit(17); // accepted upstream, RPC receipt lost
                }
                result(id, json!({"turn":turn}));
                let turn_id = turn["id"].clone();
                match mode.as_str() {
                    "notice-gate" => {
                        // A fixture-owned file gate lets the native PTY test
                        // type before completion; no real work or provider runs.
                        std::fs::write(workspace.join("worker-waiting"), "ready").unwrap();
                        let deadline =
                            std::time::Instant::now() + std::time::Duration::from_secs(15);
                        while !workspace.join("release-worker").exists() {
                            assert!(
                                std::time::Instant::now() < deadline,
                                "native fixture gate timed out"
                            );
                            std::thread::sleep(std::time::Duration::from_millis(5));
                        }
                        note(
                            "item/completed",
                            json!({"threadId":thread,"turnId":turn_id,"item":{"type":"agentMessage","id":"notice-report","text":"NOTICE-GATED RESULT 你好"}}),
                        );
                        finish(&workspace, &thread, &mut turns, "completed");
                    }
                    "profile-echo" => {
                        let credential = std::env::var("LAIN_PROFILE_CREDENTIAL").unwrap();
                        note(
                            "item/completed",
                            json!({"threadId":thread,"turnId":turn_id,"item":{"type":"agentMessage","text":credential}}),
                        );
                        note(
                            "item/started",
                            json!({"threadId":thread,"turnId":turn_id,"item":{"type":"commandExecution","command":credential}}),
                        );
                        note(
                            "item/completed",
                            json!({"threadId":thread,"turnId":turn_id,"item":{"type":"commandExecution","command":credential,"aggregatedOutput":credential,"exitCode":0}}),
                        );
                        emit(
                            json!({"id":42,"method":"item/commandExecution/requestApproval","params":{"threadId":thread,"turnId":turn_id,"command":credential,"cwd":workspace}}),
                        );
                        approval = json!(42);
                    }
                    "retry-success" => {
                        note(
                            "error",
                            json!({"threadId":thread,"turnId":turn_id,"willRetry":true,"error":{"message":"secret=hidden","codexErrorInfo":{"httpConnectionFailed":{"httpStatusCode":503}}}}),
                        );
                        finish(&workspace, &thread, &mut turns, "completed");
                    }
                    "auth-failure" => {
                        turns.last_mut().unwrap()["error"] =
                            json!({"message":"secret=hidden","codexErrorInfo":"unauthorized"});
                        finish(&workspace, &thread, &mut turns, "failed");
                    }
                    "credential-generation" => {
                        let credential = std::env::var("LAIN_PROFILE_CREDENTIAL").unwrap();
                        let generation = match credential.as_str() {
                            "fixture-rotation-old" => "old",
                            "fixture-rotation-new" => "new",
                            _ => panic!("unexpected fixture credential"),
                        };
                        note(
                            "item/completed",
                            json!({"threadId":thread,"turnId":turn_id,"item":{"type":"agentMessage","text":format!("credential generation {generation}")}}),
                        );
                        finish(&workspace, &thread, &mut turns, "completed");
                    }
                    "idle-crash" => {
                        finish(&workspace, &thread, &mut turns, "completed");
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        std::process::exit(17);
                    }
                    "wait" | "cancel-race" | "reject-cancel" | "crash" | "bad-frame"
                    | "overflow" => {}
                    "unsupported" => {
                        approval = json!(42);
                        emit(
                            json!({"id":approval,"method":"item/unknown/request","params":{"threadId":thread,"turnId":turn_id}}),
                        );
                    }
                    "approve" | "approve-string" | "permissions" | "file-approval" => {
                        approval = if mode == "approve-string" {
                            json!("server-approval")
                        } else {
                            json!(42)
                        };
                        let method = if mode == "permissions" {
                            "item/permissions/requestApproval"
                        } else if mode == "file-approval" {
                            "item/fileChange/requestApproval"
                        } else {
                            "item/commandExecution/requestApproval"
                        };
                        emit(
                            json!({"id":approval,"method":method,"params":{"threadId":thread,"turnId":turn_id,"itemId":"command-1","command":"echo fixture","cwd":workspace,"permissions":{"network":{"enabled":true}},"startedAtMs":1}}),
                        );
                    }
                    _ => {
                        if mode == "result-evidence" {
                            for number in 0..12 {
                                note(
                                    "item/completed",
                                    json!({"threadId":thread,"turnId":turn_id,"item":{"type":"commandExecution","command":format!("fixture-check-{number}"),"aggregatedOutput":"fixture failure","exitCode":2}}),
                                );
                            }
                        }
                        if mode == "history-output" {
                            note(
                                "item/started",
                                json!({"threadId":thread,"turnId":turn_id,"item":{"type":"commandExecution","command":"echo safe"}}),
                            );
                            note(
                                "item/completed",
                                json!({"threadId":thread,"turnId":turn_id,"item":{"type":"commandExecution","command":"echo safe","aggregatedOutput":"文".repeat(20_000),"exitCode":0}}),
                            );
                            note(
                                "item/completed",
                                json!({"threadId":thread,"turnId":turn_id,"item":{"type":"commandExecution","command":"echo redacted","aggregatedOutput":format!("{}sk-secret-after-output-limit", "a".repeat(40_000)),"exitCode":0}}),
                            );
                            note(
                                "item/completed",
                                json!({"threadId":thread,"turnId":turn_id,"item":{"type":"fileChange","status":"completed","changes":(0..32).map(|n|json!({"path":format!("file-{n}.txt")})).collect::<Vec<_>>()}}),
                            );
                            note(
                                "item/completed",
                                json!({"threadId":thread,"turnId":turn_id,"item":{"type":"mcpToolCall","tool":"fixture-tool"}}),
                            );
                        }
                        note(
                            "item/agentMessage/delta",
                            json!({"threadId":thread,"turnId":turn_id,"delta":"sk-secret-"}),
                        );
                        note(
                            "item/agentMessage/delta",
                            json!({"threadId":thread,"turnId":turn_id,"delta":"split-across-chunks"}),
                        );
                        note(
                            "item/completed",
                            json!({"threadId":"unrelated-session","turnId":turn_id,"item":{"type":"agentMessage","text":"WRONG SESSION"}}),
                        );
                        note(
                            "item/completed",
                            json!({"threadId":thread,"turnId":turn_id,"item":{"type":"agentMessage","id":"message-1","text":if mode == "history-output" {"中".repeat(40_000)} else {mode.clone()}}}),
                        );
                        finish(&workspace, &thread, &mut turns, "completed");
                    }
                }
            }
            Some("thread/fork") => {
                assert_eq!(params["threadId"], thread);
                assert_eq!(params["ephemeral"], false);
                thread = uuid::Uuid::new_v4().to_string();
                persist(&workspace, &thread, &turns);
                result(
                    id,
                    json!({"model":params["model"].as_str().unwrap_or("fixture-default"),"reasoningEffort":params["config"]["model_reasoning_effort"].as_str().unwrap_or("high"),"thread":{"id":thread,"turns":turns,"status":{"type":"idle"}}}),
                );
            }
            Some("thread/read") => {
                result(
                    id,
                    json!({"thread":{"id":thread,"turns":turns,"status":{"type":"idle"}}}),
                );
            }
            Some("turn/interrupt") => {
                assert_eq!(params["threadId"], thread);
                assert_eq!(params["turnId"], turns.last().unwrap()["id"]);
                if mode == "crash" {
                    std::process::exit(17);
                }
                if mode == "bad-frame" {
                    println!("not-json");
                    io::stdout().flush().unwrap();
                    continue;
                }
                if mode == "overflow" {
                    for _ in 0..2000 {
                        note(
                            "item/agentMessage/delta",
                            json!({"threadId":thread,"turnId":turns.last().unwrap()["id"],"delta":"a"}),
                        );
                    }
                    continue;
                }
                if mode == "reject-cancel" {
                    mode = "wait".into();
                    emit(json!({"id":id,"error":{"code":-32600,"message":"rejected"}}));
                    continue;
                }
                result(id, json!({}));
                if !approval.is_null() {
                    note(
                        "serverRequest/resolved",
                        json!({"threadId":thread,"requestId":approval}),
                    );
                    approval = Value::Null;
                }
                finish(
                    &workspace,
                    &thread,
                    &mut turns,
                    if mode == "cancel-race" {
                        "completed"
                    } else {
                        "interrupted"
                    },
                );
            }
            None => {
                assert_eq!(id, &approval);
                if mode == "unsupported" {
                    assert_eq!(request["error"]["code"], -32601);
                    finish(&workspace, &thread, &mut turns, "completed");
                    continue;
                }
                assert!(
                    request["result"]["decision"].is_string()
                        || request["result"]["permissions"].is_object()
                );
                note(
                    "serverRequest/resolved",
                    json!({"threadId":thread,"requestId":approval}),
                );
                approval = Value::Null;
                finish(&workspace, &thread, &mut turns, "completed");
            }
            _ => panic!("unsupported fixture command"),
        }
    }
}

fn persist(workspace: &std::path::Path, thread: &str, turns: &[Value]) {
    std::fs::write(
        workspace.join("fixture-history.json"),
        json!({"id":thread,"turns":turns}).to_string(),
    )
    .unwrap();
}
fn finish(workspace: &std::path::Path, thread: &str, turns: &mut [Value], status: &str) {
    turns.last_mut().unwrap()["status"] = json!(status);
    persist(workspace, thread, turns);
    note(
        "turn/completed",
        json!({"threadId":thread,"turn":turns.last().unwrap()}),
    );
}
