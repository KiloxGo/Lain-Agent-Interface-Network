//! Client-owned wire views. No dependency on Kernel storage or worker adapters.
use anyhow::{Context, Result, bail, ensure};
use futures_util::StreamExt;
use reqwest::{
    Method, Url,
    header::{AUTHORIZATION, HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Session {
    pub id: Uuid,
    pub backend: String,
    pub runtime_profile_id: String,
    pub workspace: String,
    pub state: String,
    pub runtime: String,
}
impl Session {
    pub fn can_send(&self) -> bool {
        matches!(self.state.as_str(), "idle" | "ready")
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Turn {
    pub id: Uuid,
    pub work_session_id: Uuid,
    pub state: String,
    pub cancellation: String,
    #[serde(default)]
    pub created_at_ms: u64,
    #[serde(default)]
    pub failure: Option<Value>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Approval {
    pub id: Uuid,
    pub work_session_id: Uuid,
    pub kind: String,
    pub description: String,
    pub decisions: Vec<Decision>,
    pub status: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    ApproveOnce,
    Deny,
    Cancel,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Event {
    pub id: Uuid,
    pub history_order: Option<u64>,
    pub timestamp_ms: u64,
    pub work_session_id: Uuid,
    pub turn_id: Option<Uuid>,
    pub kind: String,
    #[serde(default)]
    pub truncated: bool,
    #[serde(flatten)]
    pub data: serde_json::Map<String, Value>,
}
impl Event {
    pub fn field(&self, key: &str) -> &Value {
        self.data.get(key).unwrap_or(&Value::Null)
    }
}
#[derive(Debug, Deserialize)]
pub struct Snapshot {
    pub history_cursor: u64,
    pub sessions: Vec<Session>,
    pub turns: Vec<Turn>,
    pub approvals: Vec<Approval>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct HistoryPage {
    pub events: Vec<Event>,
    pub next_cursor: u64,
    pub has_more: bool,
}

#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    base: Url,
}
impl Client {
    pub fn new(url: &str, token: &str) -> Result<Self> {
        let mut base =
            Url::parse(url).map_err(|_| anyhow::anyhow!("LAIN_URL must be an HTTP(S) URL"))?;
        ensure!(
            matches!(base.scheme(), "http" | "https")
                && base.host_str().is_some()
                && base.username().is_empty()
                && base.password().is_none()
                && base.query().is_none()
                && base.fragment().is_none(),
            "LAIN_URL must be HTTP(S), without credentials, query or fragment"
        );
        ensure!(
            (32..=256).contains(&token.len())
                && token
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
            "LAIN_API_TOKEN must be 32–256 ASCII letters, digits, '-' or '_'"
        );
        if !base.path().ends_with('/') {
            base.set_path(&format!("{}/", base.path()));
        }
        let mut auth = HeaderValue::from_str(&format!("Bearer {token}"))
            .context("invalid authentication header")?;
        auth.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, auth);
        let http = reqwest::Client::builder()
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .read_timeout(Duration::from_secs(45))
            .build()
            .map_err(|e| e.without_url())?;
        Ok(Self { http, base })
    }
    async fn response(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
        stream: bool,
    ) -> Result<reqwest::Response> {
        let url = self.base.join(path).context("invalid API path")?;
        let mut request = self.http.request(method, url);
        if !stream {
            request = request.timeout(Duration::from_secs(60));
        }
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request
            .send()
            .await
            .map_err(|e| e.without_url())
            .context("cannot reach Kernel")?;
        ensure!(
            response.status().is_success(),
            "Kernel returned HTTP {}",
            response.status().as_u16()
        );
        Ok(response)
    }
    pub async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<T> {
        let mut response = self.response(method, path, body, false).await?;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|e| e.without_url())? {
            ensure!(
                bytes.len() + chunk.len() <= 32 * 1024 * 1024,
                "API response exceeds 32 MiB"
            );
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| anyhow::anyhow!("invalid Kernel response"))
    }
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request(Method::GET, path, None).await
    }
    pub async fn post<T: DeserializeOwned>(&self, path: &str, body: Option<Value>) -> Result<T> {
        self.request(Method::POST, path, body).await
    }
    pub async fn history(
        &self,
        id: Uuid,
        after: u64,
        until: Option<u64>,
        limit: usize,
    ) -> Result<HistoryPage> {
        let path = format!(
            "v1/work-sessions/{id}/history?after={after}&limit={limit}{}",
            until.map(|v| format!("&until={v}")).unwrap_or_default()
        );
        self.get(&path).await
    }
    pub async fn decide(&self, session: Uuid, approval: Uuid, decision: Decision) -> Result<Value> {
        let approvals: Vec<Approval> = self
            .get(&format!("v1/work-sessions/{session}/approvals"))
            .await?;
        ensure!(
            approvals.iter().any(|a| a.id == approval
                && a.status == "pending"
                && a.decisions.contains(&decision)),
            "approval is no longer pending or decision is not offered"
        );
        self.post(
            &format!("v1/work-sessions/{session}/approvals/{approval}/decision"),
            Some(json!({"decision":decision})),
        )
        .await
    }
    pub async fn recent(&self, session: Uuid, cursor: u64) -> Result<HistoryPage> {
        self.get(&format!(
            "v1/work-sessions/{session}/history?before={}&until={cursor}&limit=200",
            cursor.saturating_add(1)
        ))
        .await
    }
    async fn stream_once(&self, tx: &mpsc::Sender<Update>) -> Result<()> {
        let response = self.response(Method::GET, "v1/events", None, true).await?;
        ensure!(
            response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|h| h.to_str().ok())
                .is_some_and(|v| v.starts_with("text/event-stream")),
            "Kernel did not return SSE"
        );
        let mut stream = response.bytes_stream();
        let mut decoder = SseDecoder::default();
        while let Some(chunk) = stream.next().await {
            for (name, data) in decoder.push(&chunk.map_err(|e| e.without_url())?)? {
                let update = match name.as_str() {
                    "snapshot" => Update::Snapshot(
                        serde_json::from_str(&data)
                            .map_err(|_| anyhow::anyhow!("invalid SSE snapshot"))?,
                    ),
                    "event" => Update::Event(
                        serde_json::from_str(&data)
                            .map_err(|_| anyhow::anyhow!("invalid SSE event"))?,
                    ),
                    "resync_required" => bail!("stream requires a new snapshot"),
                    _ => continue,
                };
                tx.send(update).await.context("observer closed")?;
            }
        }
        bail!("event stream ended")
    }
    pub async fn observe(self, tx: mpsc::Sender<Update>) {
        loop {
            let outcome =
                tokio::select! { _=tx.closed()=>return, result=self.stream_once(&tx)=>result };
            let message = outcome.err().map(|e| e.to_string()).unwrap_or_default();
            if tx.send(Update::Offline(message)).await.is_err() {
                return;
            }
            tokio::select! { _=tx.closed()=>return, _=tokio::time::sleep(Duration::from_secs(2))=>{} }
        }
    }
}

#[derive(Debug)]
pub enum Update {
    Snapshot(Snapshot),
    Event(Event),
    Offline(String),
}

/// Decode only at complete lines, so UTF-8 may span HTTP chunks. Bounded frames.
#[derive(Default)]
struct SseDecoder {
    pending: Vec<u8>,
    name: String,
    data: String,
    frame_bytes: usize,
}
impl SseDecoder {
    fn push(&mut self, bytes: &[u8]) -> Result<Vec<(String, String)>> {
        self.pending.extend_from_slice(bytes);
        let mut frames = Vec::new();
        while let Some(pos) = self.pending.iter().position(|b| *b == b'\n') {
            self.frame_bytes += pos + 1;
            ensure!(
                self.frame_bytes <= 32 * 1024 * 1024,
                "SSE frame exceeds 32 MiB"
            );
            let line: Vec<_> = self.pending.drain(..=pos).collect();
            let line = std::str::from_utf8(&line)
                .context("invalid SSE UTF-8")?
                .trim_end_matches(['\r', '\n']);
            if line.is_empty() {
                if !self.data.is_empty() {
                    frames.push((
                        std::mem::take(&mut self.name),
                        std::mem::take(&mut self.data),
                    ));
                }
                self.name.clear();
                self.frame_bytes = 0;
            } else if let Some(name) = line.strip_prefix("event:") {
                self.name = name.trim_start_matches(' ').to_owned();
            } else if let Some(data) = line.strip_prefix("data:") {
                if !self.data.is_empty() {
                    self.data.push('\n');
                }
                self.data.push_str(data.strip_prefix(' ').unwrap_or(data));
            }
        }
        ensure!(
            self.pending.len() + self.frame_bytes <= 32 * 1024 * 1024,
            "SSE frame exceeds 32 MiB"
        );
        Ok(frames)
    }
}

impl Client {
    pub async fn notice_observe(
        &self,
        conversation: lain_wire::ConversationId,
        after: &mut u64,
        tx: &mpsc::Sender<NoticeUpdate>,
    ) -> Result<()> {
        let response = self
            .response(
                Method::GET,
                &format!("v1/conversations/{conversation}/notices/events?after={after}"),
                None,
                true,
            )
            .await?;
        let mut stream = response.bytes_stream();
        let mut decoder = SseDecoder::default();
        while let Some(chunk) = stream.next().await {
            for (name, data) in decoder.push(&chunk.map_err(|e| e.without_url())?)? {
                if name != "notices" {
                    continue;
                }
                let page: lain_wire::notices::NoticePage = serde_json::from_str(&data)
                    .map_err(|_| anyhow::anyhow!("invalid notice page"))?;
                page.validate(conversation, *after)
                    .map_err(anyhow::Error::msg)?;
                let cursor = page.next_cursor;
                for notice in page.notices {
                    tx.send(NoticeUpdate::Notice(Box::new(notice)))
                        .await
                        .context("notice observer closed")?;
                }
                *after = cursor;
            }
        }
        bail!("conversation notice stream disconnected")
    }
    pub async fn actor_observe(
        &self,
        id: lain_wire::ActorTurnId,
        tx: &mpsc::Sender<ActorUpdate>,
    ) -> Result<()> {
        let response = self
            .response(
                Method::GET,
                &format!("v1/actor-turns/{id}/events"),
                None,
                true,
            )
            .await?;
        let mut stream = response.bytes_stream();
        let mut decoder = SseDecoder::default();
        let mut snapshot_seen = false;
        while let Some(chunk) = stream.next().await {
            for (name, data) in decoder.push(&chunk.map_err(|e| e.without_url())?)? {
                if name == "resync_required" {
                    bail!("Actor stream requires a fresh snapshot");
                }
                let (update, finished) = match name.as_str() {
                    "turn" => {
                        let value: lain_wire::ActorSnapshot = serde_json::from_str(&data)
                            .map_err(|_| anyhow::anyhow!("invalid Actor snapshot"))?;
                        value.validate().map_err(anyhow::Error::msg)?;
                        ensure!(value.id == id, "Actor snapshot belongs to another turn");
                        snapshot_seen = true;
                        let finished = value.state != lain_wire::ActorTurnState::Running;
                        (ActorUpdate::Snapshot(value), finished)
                    }
                    "preview" => {
                        ensure!(snapshot_seen, "Actor preview before snapshot");
                        let value: lain_wire::PreviewEvent = serde_json::from_str(&data)
                            .map_err(|_| anyhow::anyhow!("invalid Actor preview"))?;
                        ensure!(value.turn_id == id, "Actor preview belongs to another turn");
                        (ActorUpdate::Preview(value), false)
                    }
                    _ => continue,
                };
                tx.send(update).await.context("Actor observer closed")?;
                if finished {
                    return Ok(());
                }
            }
        }
        bail!("Actor stream disconnected; reconnect using the same turn ID")
    }
}

#[derive(Debug)]
pub enum ActorUpdate {
    Snapshot(lain_wire::ActorSnapshot),
    Preview(lain_wire::PreviewEvent),
}
pub enum NoticeUpdate {
    Notice(Box<lain_wire::notices::ConversationNotice>),
    Offline,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sse_chunks_unicode_multiline_comments_and_limits() {
        let wire = "event: event\r\ndata: 中\r\ndata: 文\r\n\r\n: heartbeat\n\nevent: resync_required\ndata: retry\n\n";
        let mut decoder = SseDecoder::default();
        let mut frames = Vec::new();
        for byte in wire.as_bytes() {
            frames.extend(decoder.push(&[*byte]).unwrap());
        }
        assert_eq!(
            frames,
            vec![
                ("event".into(), "中\n文".into()),
                ("resync_required".into(), "retry".into())
            ]
        );
        assert!(decoder.push(&vec![b'x'; 32 * 1024 * 1024 + 1]).is_err());
    }
    #[test]
    fn credentials_are_validated_without_echoing_values() {
        let token = "safe-owner-token-0123456789abcdef";
        for url in [
            "file:///private",
            "http://owner:secret@localhost",
            "http://localhost/?token=secret",
        ] {
            assert!(Client::new(url, token).is_err());
        }
        assert!(Client::new("https://example.com/lain", token).is_ok());
        assert!(
            !Client::new("http://localhost", "raw-secret")
                .err()
                .unwrap()
                .to_string()
                .contains("raw-secret")
        );
    }

    #[tokio::test]
    async fn observer_reconnects_with_fresh_snapshot_after_resync_and_stream_loss() {
        use axum::{
            Router,
            body::Body,
            extract::{Request, State},
            http::header,
            routing::get,
        };
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        async fn stream(
            State(count): State<Arc<AtomicUsize>>,
            request: Request,
        ) -> axum::response::Response {
            assert!(
                request.headers()["authorization"]
                    .to_str()
                    .unwrap()
                    .starts_with("Bearer ")
            );
            let n = count.fetch_add(1, Ordering::SeqCst);
            let wire = format!(
                "event: snapshot\ndata: {{\"history_cursor\":{},\"sessions\":[],\"turns\":[],\"approvals\":[]}}\n\n{}",
                n + 1,
                if n == 0 {
                    "event: resync_required\ndata: refresh\n\n"
                } else {
                    ""
                }
            );
            axum::response::Response::builder()
                .header(header::CONTENT_TYPE, "text/event-stream")
                .body(Body::from_stream(tokio_stream::iter(
                    wire.as_bytes()
                        .chunks(3)
                        .map(|chunk| Ok::<_, std::io::Error>(chunk.to_vec()))
                        .collect::<Vec<_>>(),
                )))
                .unwrap()
        }
        let count = Arc::new(AtomicUsize::new(0));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let router = Router::new()
            .route("/v1/events", get(stream))
            .with_state(count.clone());
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        let client = Client::new(
            &format!("http://127.0.0.1:{port}"),
            "fixture-owner-token-0123456789abcdef",
        )
        .unwrap();
        let (tx, mut rx) = mpsc::channel(8);
        let observer = tokio::spawn(client.observe(tx));
        tokio::time::timeout(Duration::from_secs(10), async {
            assert!(matches!(
                rx.recv().await,
                Some(Update::Snapshot(Snapshot {
                    history_cursor: 1,
                    ..
                }))
            ));
            assert!(matches!(rx.recv().await, Some(Update::Offline(_))));
            assert!(matches!(
                rx.recv().await,
                Some(Update::Snapshot(Snapshot {
                    history_cursor: 2,
                    ..
                }))
            ));
            assert!(matches!(rx.recv().await, Some(Update::Offline(_))));
        })
        .await
        .unwrap();
        drop(rx);
        tokio::time::timeout(Duration::from_secs(1), observer)
            .await
            .unwrap()
            .unwrap();
        server.abort();
    }
}
