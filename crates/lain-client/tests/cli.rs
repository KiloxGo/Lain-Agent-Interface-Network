use axum::{
    Json, Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    response::{IntoResponse, Response},
    routing::any,
};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

const TOKEN: &str = "fixture-owner-token-0123456789abcdef";
const ID: &str = "00000000-0000-0000-0000-000000000001";
const APPROVAL: &str = "00000000-0000-0000-0000-000000000002";
type Calls = Arc<Mutex<Vec<(String, String, Value)>>>;
async fn handler(State(calls): State<Calls>, request: Request<Body>) -> Response {
    assert_eq!(
        request.headers()["authorization"],
        format!("Bearer {TOKEN}")
    );
    let method = request.method().to_string();
    let path = request.uri().path().to_owned();
    let bytes = to_bytes(request.into_body(), 65536).await.unwrap();
    let input: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    calls
        .lock()
        .unwrap()
        .push((method.clone(), path.clone(), input.clone()));
    let session = json!({"id":ID,"backend":"codex","runtime_profile_id":"codex/local","workspace":"/remote/project","state":"idle","runtime":"detached","created_at_ms":1});
    let value = if path == "/health" {
        json!({"status":"ok"})
    } else if path == "/ready" {
        json!({"state":"ready"})
    } else if path == "/v1/backends" {
        json!([])
    } else if path.ends_with("/history") {
        json!({"events":[],"next_cursor":0,"has_more":false})
    } else if path.ends_with("/approvals") {
        json!([{"id":APPROVAL,"work_session_id":ID,"kind":"command","status":"pending","description":"echo safe","decisions":["deny"]}])
    } else if path == "/v1/work-sessions" && method == "GET" {
        json!([session])
    } else if path == "/v1/work-sessions" {
        assert_eq!(input["workspace"], "/server/exists-only-on-server");
        session
    } else if path.ends_with(ID) {
        session
    } else {
        json!({"accepted":true})
    };
    Json(value).into_response()
}
async fn cli(port: u16, args: &[&str]) -> std::process::Output {
    let args: Vec<_> = args.iter().map(|s| s.to_string()).collect();
    tokio::task::spawn_blocking(move || {
        std::process::Command::new(env!("CARGO_BIN_EXE_lain"))
            .args(args)
            .env("LAIN_URL", format!("http://127.0.0.1:{port}"))
            .env("LAIN_API_TOKEN", TOKEN)
            .output()
            .unwrap()
    })
    .await
    .unwrap()
}
#[tokio::test]
async fn commands_use_authenticated_http_preserve_server_paths_and_check_offered_decisions() {
    let calls: Calls = Arc::default();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let router = Router::new()
        .route("/{*path}", any(handler))
        .with_state(calls.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    for args in [
        vec!["status"],
        vec!["work", "list"],
        vec![
            "work",
            "start",
            "--workspace",
            "/server/exists-only-on-server",
        ],
        vec!["work", "show", ID],
        vec!["work", "send", ID, "continue"],
        vec!["work", "cancel", ID],
        vec!["work", "resume", ID],
        vec!["work", "history", ID],
        vec!["approval", "list", ID],
        vec!["approval", "deny", ID, APPROVAL],
    ] {
        let mut args = args;
        args.push("--json");
        let output = cli(port, &args).await;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let _: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(!String::from_utf8_lossy(&output.stdout).contains(TOKEN));
    }
    let output = cli(port, &["approval", "approve", ID, APPROVAL]).await;
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stderr).contains(TOKEN));
    let decisions: Vec<_> = calls
        .lock()
        .unwrap()
        .iter()
        .filter(|(_, p, _)| p.ends_with("/decision"))
        .cloned()
        .collect();
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].2["decision"], "deny");
    let output = cli(port, &["work", "attach", ID]).await;
    assert!(
        !output.status.success()
            && String::from_utf8_lossy(&output.stderr).contains("requires a terminal")
    );
    assert!(
        !calls
            .lock()
            .unwrap()
            .iter()
            .any(|(_, p, _)| p.ends_with("/resume") && p.contains("attach"))
    );
    server.abort();
}
