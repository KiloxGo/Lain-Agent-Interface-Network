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
                assert_eq!(params["approvalPolicy"], "on-request");
                assert_eq!(params["approvalsReviewer"], "user");
                workspace = PathBuf::from(params["cwd"].as_str().unwrap());
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
                    json!({"thread":{"id":thread,"turns":turns,"status":{"type":"idle"}}}),
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
                result(id, json!({"turn":turn}));
                let turn_id = turn["id"].clone();
                match mode.as_str() {
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
                            json!({"threadId":thread,"turnId":turn_id,"item":{"type":"agentMessage","id":"message-1","text":mode}}),
                        );
                        finish(&workspace, &thread, &mut turns, "completed");
                    }
                }
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
