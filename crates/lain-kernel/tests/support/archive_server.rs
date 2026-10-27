//! Read-contract fixture. Any control operation is a test failure.
use serde_json::{Value, json};
use std::{
    io::{self, BufRead, Write},
    path::PathBuf,
    time::{Duration, Instant},
};
fn reply(id: &Value, value: Value) {
    println!("{}", json!({"id":id,"result":value}));
    io::stdout().flush().unwrap();
}
fn main() {
    if std::env::args().any(|a| a == "--version") {
        println!("codex-cli 0.160.1");
        return;
    }
    let mut ready = false;
    for line in io::stdin().lock().lines() {
        let q: Value = serde_json::from_str(&line.unwrap()).unwrap();
        let p = &q["params"];
        let id = &q["id"];
        match q["method"].as_str().unwrap() {
            "initialize" => {
                ready = true;
                reply(id, json!({"userAgent":"codex/0.160.1"}));
            }
            "initialized" => assert!(ready),
            method => {
                assert!(
                    ready
                        && matches!(
                            method,
                            "thread/read" | "thread/turns/list" | "thread/items/list"
                        )
                );
                let thread = p["threadId"].as_str().unwrap();
                let (mode, workspace) = thread.split_once('|').unwrap();
                let workspace = PathBuf::from(workspace);
                let mut calls = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(workspace.join("archive-fixture-calls"))
                    .unwrap();
                writeln!(calls, "{method}").unwrap();
                if mode == "hold" && method == "thread/read" {
                    std::fs::write(
                        workspace.join("archive-fixture-pid"),
                        std::process::id().to_string(),
                    )
                    .unwrap();
                    let end = Instant::now() + Duration::from_secs(15);
                    while !workspace.join("release-archive").exists() {
                        assert!(Instant::now() < end);
                        std::thread::sleep(Duration::from_millis(5));
                    }
                }
                match method {
                    "thread/read" => {
                        assert_eq!(p["includeTurns"], false);
                        reply(
                            id,
                            json!({"thread":{"id":if mode=="wrong" {"another-thread"}else{thread},"cwd":workspace,"historyMode":"paginated"}}),
                        );
                    }
                    "thread/turns/list" => {
                        assert_eq!(p["limit"], 1);
                        assert_eq!(p["itemsView"], "notLoaded");
                        assert_eq!(p["sortDirection"], "asc");
                        let status =
                            if mode == "running" && !workspace.join("archive-complete").exists() {
                                "inProgress"
                            } else {
                                "completed"
                            };
                        reply(
                            id,
                            json!({"data":[{"id":"turn-1","status":status,"items":[]}],"nextCursor":null}),
                        );
                    }
                    "thread/items/list" => {
                        assert_eq!(p["turnId"], "turn-1");
                        assert_eq!(p["sortDirection"], "asc");
                        let text = if mode == "secret" {
                            "fixture-owner-token-0123456789abcdef"
                        } else {
                            "synthetic visible user message"
                        };
                        let items = [
                            json!({"type":"userMessage","id":"user-1","content":[{"type":"text","text":text}]}),
                            json!({"type":"reasoning","id":"private-1","content":["must never enter the archive"],"summary":["also excluded"]}),
                            json!({"type":"agentMessage","id":"agent-1","text":"synthetic visible assistant result","phase":"final_answer"}),
                            json!({"type":"commandExecution","id":"command-1","status":"completed","command":"synthetic-command","aggregatedOutput":"synthetic-output","exitCode":0}),
                            json!({"type":"fileChange","id":"file-1","status":"completed","changes":[{"path":"synthetic.txt","diff":"private diff excluded"}]}),
                        ];
                        let after = p["cursor"]
                            .as_str()
                            .map_or(0, |s| s.parse::<usize>().unwrap());
                        let limit = p["limit"].as_u64().unwrap() as usize;
                        let end = (after + limit).min(items.len());
                        let data = items[after..end]
                            .iter()
                            .map(|v| json!({"turnId":"turn-1","item":v,"completedAtMs":1000}))
                            .collect::<Vec<_>>();
                        reply(
                            id,
                            json!({"data":data,"nextCursor":(end<items.len()).then(||end.to_string())}),
                        );
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}
