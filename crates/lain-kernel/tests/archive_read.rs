use lain_kernel::{api, backend::Supervisor, kernel::Kernel};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc, time::Duration};
const TOKEN: &str = "fixture-owner-token-0123456789abcdef";
struct Harness {
    dir: tempfile::TempDir,
    kernel: Arc<Kernel>,
    workers: Arc<Supervisor>,
    url: String,
    close: tokio::sync::oneshot::Sender<()>,
    task: tokio::task::JoinHandle<anyhow::Result<()>>,
}
impl Harness {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let kernel = Arc::new(Kernel::open(&dir.path().join("state.db")).unwrap());
        kernel.mark_ready().unwrap();
        let workers = Supervisor::new(
            kernel.clone(),
            Some(PathBuf::from(env!(
                "CARGO_BIN_EXE_lain-test-archive-server"
            ))),
            TOKEN.parse().unwrap(),
            Duration::from_secs(60),
        )
        .await;
        assert!(!workers.info().available);
        assert_eq!(
            workers.info().archive_read_version.as_deref(),
            Some("0.160.1")
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (close, rx) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(api::serve_with_workers(
            listener,
            kernel.clone(),
            TOKEN.parse().unwrap(),
            Some(workers.clone()),
            async {
                let _ = rx.await;
            },
        ));
        Self {
            dir,
            kernel,
            workers,
            url,
            close,
            task,
        }
    }
    async fn call(&self, method: &str, path: &str, body: Option<Value>, status: u16) -> Value {
        let client = reqwest::Client::new();
        let mut q = client
            .request(method.parse().unwrap(), format!("{}{}", self.url, path))
            .bearer_auth(TOKEN);
        if let Some(body) = body {
            q = q.json(&body)
        }
        let r = q.send().await.unwrap();
        assert_eq!(r.status().as_u16(), status);
        assert_eq!(r.headers()["cache-control"], "no-store");
        r.json().await.unwrap()
    }
    async fn source(&self, mode: &str) -> String {
        let key = json!({"kind":"codex_session","installation":"codex/local","session_id":format!("{mode}|{}",self.dir.path().display()),"workspace":self.dir.path()});
        let s = self
            .call("POST", "/v1/archives/sources", Some(key), 200)
            .await;
        let id = s["source"]["id"].as_str().unwrap().to_owned();
        self.call("POST",&format!("/v1/archives/sources/{id}/grant"),Some(json!({"expected_revision":0,"scope":{"expires_at_ms":lain_kernel::domain::now_ms().unwrap()+60000,"retention_ms":30000,"max_records":100}})),200).await;
        id
    }
    async fn start(&self, source: &str, after: Option<&str>, limit: u16) -> Value {
        self.call("POST",&format!("/v1/archives/sources/{source}/read"),Some(json!({"request_id":uuid::Uuid::new_v4(),"expected_revision":1,"after":after,"limit":limit})),200).await
    }
    async fn settle(&self, job: &Value) -> Value {
        let id = job["request"]["request_id"].as_str().unwrap();
        let end = tokio::time::Instant::now() + Duration::from_secs(10);
        loop {
            let v = self
                .call("GET", &format!("/v1/archives/reads/{id}"), None, 200)
                .await;
            if !matches!(v["state"].as_str(), Some("prepared" | "reading")) {
                return v;
            }
            assert!(tokio::time::Instant::now() < end);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
    async fn stop(self) {
        self.kernel.shutdown().unwrap();
        self.workers.shutdown().await.unwrap();
        let _ = self.close.send(());
        self.task.await.unwrap().unwrap();
    }
}
#[tokio::test]
async fn visible_archive_pages_are_atomic_scoped_and_do_not_create_worker_or_model_turns() {
    let h = Harness::new().await;
    let source = h.source("normal").await;
    let first = h.start(&source, None, 2).await;
    let first = h.settle(&first).await;
    assert_eq!(first["state"], "completed");
    assert_eq!(first["new_records"], 1);
    assert_eq!(first["omitted_items"], 1);
    let replay = h
        .call(
            "POST",
            &format!("/v1/archives/sources/{source}/read"),
            Some(first["request"].clone()),
            200,
        )
        .await;
    assert_eq!(replay["record_ids"], first["record_ids"]);
    let second = h.start(&source, first["next"].as_str(), 2).await;
    let second = h.settle(&second).await;
    assert_eq!(second["new_records"], 2);
    let third = h.start(&source, second["next"].as_str(), 2).await;
    let third = h.settle(&third).await;
    assert_eq!(third["new_records"], 1);
    assert!(third["next"].is_null());
    let page = h
        .call(
            "POST",
            &format!("/v1/archives/sources/{source}/query"),
            Some(json!({"expected_revision":1,"cursor":null,"limit":20})),
            200,
        )
        .await;
    assert_eq!(page["records"].as_array().unwrap().len(), 4);
    assert!(!page.to_string().contains("must never enter"));
    assert!(!page.to_string().contains("private diff"));
    assert!(
        page["records"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["text_origin"] == "adapter_observation")
    );
    let again = h.start(&source, None, 2).await;
    assert_eq!(h.settle(&again).await["duplicate_records"], 1);
    let other = h.source("other").await;
    h.call("POST",&format!("/v1/archives/sources/{other}/read"),Some(json!({"request_id":uuid::Uuid::new_v4(),"expected_revision":1,"after":first["next"],"limit":2})),409).await;
    assert!(h.kernel.list_sessions().unwrap().is_empty());
    assert!(h.kernel.conversations().unwrap().is_empty());
    let calls = std::fs::read_to_string(h.dir.path().join("archive-fixture-calls")).unwrap();
    assert!(!calls.contains("resume") && !calls.contains("turn/start"));
    h.stop().await;
}
#[tokio::test]
async fn active_turn_is_revisited_and_sensitive_or_mismatched_sources_never_publish() {
    let h = Harness::new().await;
    let source = h.source("running").await;
    let first = h.start(&source, None, 2).await;
    let first = h.settle(&first).await;
    assert_eq!(first["waiting_for_source"], true);
    assert_eq!(first["new_records"], 0);
    std::fs::write(h.dir.path().join("archive-complete"), "fixture").unwrap();
    let next = h.start(&source, first["next"].as_str(), 2).await;
    assert_eq!(h.settle(&next).await["new_records"], 1);
    for mode in ["secret", "wrong"] {
        let source = h.source(mode).await;
        let j = h.start(&source, None, 2).await;
        let j = h.settle(&j).await;
        assert_eq!(j["state"], "failed");
        let source = h
            .call("GET", &format!("/v1/archives/sources/{source}"), None, 200)
            .await;
        assert_eq!(source["total_records"], 0);
        assert!(!j.to_string().contains(TOKEN));
    }
    h.stop().await;
}
#[tokio::test]
async fn cancellation_and_revoke_interrupt_read_process_without_publishing_late_history() {
    for action in ["cancel", "revoke", "shutdown"] {
        let h = Harness::new().await;
        let source = h.source("hold").await;
        let j = h.start(&source, None, 2).await;
        let end = tokio::time::Instant::now() + Duration::from_secs(5);
        while !h.dir.path().join("archive-fixture-pid").exists() {
            assert!(tokio::time::Instant::now() < end);
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        let pid: u32 = std::fs::read_to_string(h.dir.path().join("archive-fixture-pid"))
            .unwrap()
            .parse()
            .unwrap();
        if action == "revoke" {
            h.call(
                "POST",
                &format!("/v1/archives/sources/{source}/grant"),
                Some(json!({"expected_revision":1,"scope":null})),
                200,
            )
            .await;
        } else if action == "cancel" {
            h.call(
                "POST",
                &format!(
                    "/v1/archives/reads/{}/cancel",
                    j["request"]["request_id"].as_str().unwrap()
                ),
                None,
                200,
            )
            .await;
        } else {
            h.workers.shutdown().await.unwrap();
        }
        let done = h.settle(&j).await;
        assert_eq!(done["state"], "cancelled");
        let source = h
            .call("GET", &format!("/v1/archives/sources/{source}"), None, 200)
            .await;
        assert_eq!(source["total_records"], 0);
        h.stop().await;
        assert!(!process_alive(pid), "owned read process survived shutdown");
    }
}

#[cfg(windows)]
fn process_alive(pid: u32) -> bool {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
    };
    // SAFETY: query-only access to the PID emitted by this test's own child.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return false;
        }
        let mut code = 0;
        let ok = GetExitCodeProcess(handle, &mut code);
        CloseHandle(handle);
        ok != 0 && code == 259
    }
}
#[cfg(unix)]
fn process_alive(pid: u32) -> bool {
    // SAFETY: signal zero only checks existence of the fixture's own child.
    unsafe {
        libc::kill(pid as i32, 0) == 0
            || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }
}
