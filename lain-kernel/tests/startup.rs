use std::{
    net::{SocketAddr, TcpListener},
    process::Stdio,
    time::Duration,
};

use tempfile::TempDir;
use tokio::{process::Command, time::sleep};

fn unused_local_address() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("ephemeral port should bind");
    listener.local_addr().expect("listener has local address")
}

#[tokio::test]
async fn health_starts_without_provider_key_while_readiness_fails() {
    let fixture = TempDir::new().expect("temporary fixture");
    let prompt_path = fixture.path().join("lain_system.md");
    std::fs::write(&prompt_path, "Synthetic Lain system prompt.").expect("write prompt fixture");
    let data_dir = fixture.path().join("data");
    let address = unused_local_address();

    let mut child = Command::new(env!("CARGO_BIN_EXE_lain-kernel"))
        .env("LAIN_BIND_ADDR", address.to_string())
        .env("LAIN_SYSTEM_PROMPT", &prompt_path)
        .env("LAIN_DATA_DIR", &data_dir)
        .env("DEEPSEEK_API_KEY", "")
        .env("RUST_LOG", "error")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .expect("kernel process should spawn");

    let client = reqwest::Client::new();
    let health_url = format!("http://{address}/api/health");
    let mut health_response = None;
    for _ in 0..50 {
        match client.get(&health_url).send().await {
            Ok(response) => {
                health_response = Some(response);
                break;
            }
            Err(_) => sleep(Duration::from_millis(100)).await,
        }
    }

    let health_response = health_response.expect("health endpoint should become available");
    assert_eq!(health_response.status(), reqwest::StatusCode::OK);
    assert!(health_response.headers().contains_key("x-request-id"));

    let ready_response = client
        .get(format!("http://{address}/api/ready"))
        .send()
        .await
        .expect("readiness request");
    assert_eq!(
        ready_response.status(),
        reqwest::StatusCode::SERVICE_UNAVAILABLE
    );
    let body: serde_json::Value = ready_response.json().await.expect("readiness JSON");
    assert_eq!(body["status"], "not_ready");
    let checks = body["checks"].as_array().expect("readiness checks");
    let provider = checks
        .iter()
        .find(|check| check["name"] == "provider")
        .expect("provider readiness check");
    let storage = checks
        .iter()
        .find(|check| check["name"] == "storage")
        .expect("storage readiness check");
    assert_eq!(provider["ready"], false);
    assert_eq!(storage["ready"], true);

    child.kill().await.expect("kernel process should stop");
    child.wait().await.expect("kernel process should be reaped");
}
