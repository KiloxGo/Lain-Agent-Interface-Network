//! Private App Server framing and process transport. Never exposes JSON to the Kernel.
use std::{
    collections::BTreeMap,
    path::Path,
    process::Stdio,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{ChildStdin, Command},
    sync::{mpsc, oneshot, watch},
};

use super::process::{self, ProcessTree};

const RPC_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_FRAME: u64 = 8 * 1024 * 1024;
type Pending = Arc<Mutex<BTreeMap<u64, oneshot::Sender<Result<Value>>>>>;

#[derive(Debug, thiserror::Error)]
#[error("worker rejected {method} (code {code})")]
pub(super) struct Rejected {
    pub method: &'static str,
    pub code: i64,
}

pub(super) struct Rpc {
    stdin: tokio::sync::Mutex<Option<ChildStdin>>,
    pending: Pending,
    next_id: AtomicU64,
    stop: watch::Sender<bool>,
    closed: watch::Receiver<bool>,
    pub messages: mpsc::Receiver<Value>,
}

pub(super) fn command(executable: &Path) -> Command {
    let mut command = Command::new(executable);
    command.env_clear();
    // No daemon token, arbitrary credentials, or whole-environment inheritance.
    for key in [
        "PATH",
        "HOME",
        "USERPROFILE",
        "SYSTEMROOT",
        "WINDIR",
        "APPDATA",
        "LOCALAPPDATA",
        "TEMP",
        "TMP",
        "TMPDIR",
        "COMSPEC",
        "PATHEXT",
        "PROGRAMFILES",
        "PROGRAMFILES(X86)",
        "PROGRAMDATA",
        "LANG",
        "LC_ALL",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "NO_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
        "no_proxy",
        "CODEX_HOME",
    ] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    process::configure(&mut command);
    command
}

impl Rpc {
    pub async fn spawn(executable: &Path) -> Result<Self> {
        let mut child = command(executable)
            .args(["app-server", "--listen", "stdio://"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("launch Codex App Server")?;
        let tree = ProcessTree::attach(&child)?;
        let stdin = child.stdin.take().context("worker stdin missing")?;
        let stdout = child.stdout.take().context("worker stdout missing")?;
        let stderr = child.stderr.take().context("worker stderr missing")?;
        let pending: Pending = Arc::new(Mutex::new(BTreeMap::new()));
        let (messages, receiver) = mpsc::channel(512);
        let (stop, mut stop_rx) = watch::channel(false);
        let (closed_tx, closed) = watch::channel(false);
        let read_pending = pending.clone();
        let read_stop = stop.clone();
        let reader = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut frame = Vec::new();
                // Bound each JSON frame. Cancel-safe reading is not required in this dedicated task.
                let read = (&mut reader)
                    .take(MAX_FRAME + 1)
                    .read_until(b'\n', &mut frame)
                    .await;
                if !matches!(read, Ok(n) if n > 0 && n as u64 <= MAX_FRAME) {
                    break;
                }
                let Ok(value) = serde_json::from_slice::<Value>(&frame) else {
                    break;
                };
                if value.get("method").is_none() {
                    if let Some(id) = value.get("id").and_then(Value::as_u64) {
                        let sender = read_pending.lock().expect("RPC pending lock").remove(&id);
                        if let Some(sender) = sender {
                            let _ = sender.send(Ok(value));
                        }
                    }
                } else if messages.try_send(value).is_err() {
                    // Never wait for storage while reading protocol. Overflow loses certainty:
                    // stop this owned runtime; recovery must reconcile rather than skip lifecycle events.
                    break;
                }
            }
            let _ = read_stop.send(true);
        });
        let stderr_reader = tokio::spawn(async move {
            // Drain bytes without retaining or logging potentially credential-bearing diagnostics.
            let _ = tokio::io::copy(&mut BufReader::new(stderr), &mut tokio::io::sink()).await;
        });
        let exit_pending = pending.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = child.wait() => {},
                _ = async { let _ = stop_rx.wait_for(|stop| *stop).await; } => {
                    if tokio::time::timeout(Duration::from_secs(2), child.wait()).await.is_err() {
                        let _ = child.kill().await;
                    }
                }
            }
            drop(tree);
            reader.abort();
            stderr_reader.abort();
            for (_, sender) in std::mem::take(&mut *exit_pending.lock().expect("RPC pending lock"))
            {
                let _ = sender.send(Err(anyhow::anyhow!("worker transport closed")));
            }
            let _ = closed_tx.send(true);
        });
        Ok(Self {
            stdin: tokio::sync::Mutex::new(Some(stdin)),
            pending,
            next_id: AtomicU64::new(1),
            stop,
            closed,
            messages: receiver,
        })
    }

    async fn send(&self, value: &Value) -> Result<()> {
        ensure!(!*self.closed.borrow(), "worker transport closed");
        let mut bytes = serde_json::to_vec(value)?;
        bytes.push(b'\n');
        let mut stdin = self.stdin.lock().await;
        let stdin = stdin.as_mut().context("worker input closed")?;
        tokio::time::timeout(RPC_TIMEOUT, async {
            stdin.write_all(&bytes).await?;
            stdin.flush().await
        })
        .await
        .context("worker write timeout")??;
        Ok(())
    }

    pub async fn request(&self, method: &'static str, params: Value) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (send, receive) = oneshot::channel();
        self.pending
            .lock()
            .expect("RPC pending lock")
            .insert(id, send);
        let outcome = async {
            self.send(&json!({"id":id,"method":method,"params":params}))
                .await?;
            let response = tokio::time::timeout(RPC_TIMEOUT, receive)
                .await
                .context("worker response timeout")?
                .context("worker response channel closed")??;
            if let Some(error) = response.get("error") {
                return Err(Rejected {
                    method,
                    code: error["code"].as_i64().unwrap_or(-32603),
                }
                .into());
            }
            response
                .get("result")
                .cloned()
                .context("worker response missing result")
        }
        .await;
        self.pending.lock().expect("RPC pending lock").remove(&id);
        if outcome
            .as_ref()
            .is_err_and(|error: &anyhow::Error| error.downcast_ref::<Rejected>().is_none())
        {
            let _ = self.stop.send(true);
        }
        outcome
    }

    pub async fn notify(&self, method: &str) -> Result<()> {
        self.send(&json!({"method":method,"params":{}})).await
    }
    pub async fn reply(&self, id: Value, result: Value) -> Result<()> {
        self.send(&json!({"id":id,"result":result})).await
    }
    pub async fn unsupported(&self, id: Value) -> Result<()> {
        self.send(&json!({"id":id,"error":{"code":-32601,"message":"unsupported client request"}}))
            .await
    }
    pub async fn next(&mut self) -> Result<Value> {
        tokio::select! {
            biased;
            _ = self.closed.wait_for(|closed| *closed) => bail!("worker disconnected"),
            value = self.messages.recv() => value.context("worker protocol stream closed"),
        }
    }
    pub async fn shutdown(&mut self) -> Result<()> {
        self.stdin.lock().await.take();
        let _ = self.stop.send(true);
        tokio::time::timeout(
            Duration::from_secs(5),
            self.closed.wait_for(|closed| *closed),
        )
        .await
        .context("worker shutdown timed out")?
        .context("worker shutdown channel closed")?;
        Ok(())
    }
}

impl Drop for Rpc {
    fn drop(&mut self) {
        let _ = self.stop.send(true);
    }
}
