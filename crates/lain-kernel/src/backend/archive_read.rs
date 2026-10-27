//! Selected visible-history reads. No start/resume/fork/turn/approval surface.
use super::{
    Supervisor,
    codex::CodexLaunchSpec,
    rpc::{self, Rpc},
};
use crate::{archives, backend::ControlError, kernel::blocking};
use anyhow::{Context, Result, ensure};
use lain_wire::{ArchiveSourceId, RequestId, archives::*};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{path::Path, sync::Arc, time::Duration};

const VERSION: &str = "0.160.1";
#[derive(Debug, thiserror::Error)]
#[error("visible archive reader verified only against 0.160.1")]
struct UnsupportedVersion;
pub(crate) struct Batch {
    pub records: Vec<ImportArchiveRecord>,
    pub next: Option<String>,
    pub omitted: u16,
    pub waiting: bool,
    pub version: String,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    version: String,
    turns: Option<String>,
    turn_id: Option<String>,
    items: Option<String>,
}
fn opaque(v: &Value) -> Result<Option<String>> {
    if v.is_null() {
        return Ok(None);
    }
    let s = v.as_str().context("invalid archive pagination")?;
    ensure!(
        !s.is_empty() && s.len() <= 8192 && !s.chars().any(char::is_control),
        "invalid archive pagination"
    );
    Ok(Some(s.to_owned()))
}
fn identity(v: &Value) -> Result<&str> {
    let s = v.as_str().context("missing archive source identity")?;
    ensure!(
        !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control),
        "invalid archive source identity"
    );
    Ok(s)
}
fn metadata(value: &Value, thread: &str, workspace: &str) -> Result<()> {
    ensure!(
        value["thread"]["id"] == thread,
        "archive thread identity changed"
    );
    let returned = value["thread"]["cwd"]
        .as_str()
        .context("archive workspace missing")?;
    // Both are explicitly selected local paths, never a discovery scan.
    ensure!(
        Path::new(returned)
            .canonicalize()
            .ok()
            .zip(Path::new(workspace).canonicalize().ok())
            .is_some_and(|(a, b)| a == b),
        "archive workspace mismatch or unavailable"
    );
    ensure!(
        value["thread"]["historyMode"] == "paginated",
        "unsupported archive history mode"
    );
    Ok(())
}
fn normalize(entry: &Value, turn: &str) -> Result<Option<ImportArchiveRecord>> {
    ensure!(
        entry["turnId"] == turn,
        "archive item changed turn identity"
    );
    let item = &entry["item"];
    let (speaker, kind, text) = match item["type"].as_str() {
        Some("userMessage") => {
            let content = item["content"]
                .as_array()
                .context("invalid visible user content")?;
            if content.iter().any(|p| p["type"] != "text") {
                return Ok(None);
            }
            let text = content
                .iter()
                .map(|p| p["text"].as_str().context("invalid visible user text"))
                .collect::<Result<Vec<_>>>()?
                .join("\n");
            ("user", VisibleRecordKind::UserMessage, text)
        }
        Some("agentMessage") => {
            ensure!(
                item["phase"].is_null()
                    || matches!(item["phase"].as_str(), Some("commentary" | "final_answer")),
                "unsupported visible message phase"
            );
            (
                "worker",
                VisibleRecordKind::AssistantMessage,
                item["text"]
                    .as_str()
                    .context("invalid visible assistant text")?
                    .to_owned(),
            )
        }
        Some("commandExecution") => {
            if !matches!(
                item["status"].as_str(),
                Some("completed" | "failed" | "declined")
            ) {
                return Ok(None);
            }
            (
                "tool",
                VisibleRecordKind::ExecutionResult,
                serde_json::to_string(
                    &json!({"evidence":"worker_report_not_independent_verification","command":item["command"],"status":item["status"],"exit_code":item["exitCode"],"output":item["aggregatedOutput"]}),
                )?,
            )
        }
        Some("fileChange") => {
            if item["status"] != "completed" {
                return Ok(None);
            }
            let paths = item["changes"]
                .as_array()
                .context("invalid visible file changes")?
                .iter()
                .map(|c| c["path"].as_str().context("invalid visible file path"))
                .collect::<Result<Vec<_>>>()?;
            (
                "tool",
                VisibleRecordKind::FileChange,
                serde_json::to_string(
                    &json!({"evidence":"worker_report_not_independent_verification","paths":paths}),
                )?,
            )
        }
        // Never inspect reasoning/raw response bodies, prompts/hooks or unknown tools.
        _ => return Ok(None),
    };
    if archives::validate_text(&text).is_err() {
        return Ok(None);
    }
    let id = identity(&item["id"])?;
    let external_id = format!(
        "visible:{}",
        lain_wire::actor_configuration_revision(format!("{turn}\0{id}").as_bytes())
    );
    let position = format!("{turn}/{id}");
    let time = entry["completedAtMs"]
        .as_u64()
        .or_else(|| entry["startedAtMs"].as_u64())
        .filter(|n| *n <= i64::MAX as u64);
    Ok(Some(ImportArchiveRecord {
        external_id,
        source_position: (position.len() <= 256).then_some(position),
        occurred_at_ms: time,
        speaker: speaker.into(),
        kind,
        text,
    }))
}
async fn page(
    rpc: &Rpc,
    source: &ArchiveSourceKey,
    limit: u16,
    cursor: Option<&str>,
) -> Result<Batch> {
    let ArchiveSourceKey::CodexSession {
        installation,
        session_id,
        workspace,
    } = source
    else {
        anyhow::bail!("unsupported archive source")
    };
    ensure!(
        installation == "codex/local",
        "archive installation is not local"
    );
    let c: Cursor = if let Some(s) = cursor {
        serde_json::from_str(s).map_err(|_| anyhow::anyhow!("invalid saved archive cursor"))?
    } else {
        Cursor {
            version: VERSION.into(),
            ..Default::default()
        }
    };
    ensure!(c.version == VERSION, "archive reader version changed");
    metadata(
        &rpc.request(
            "thread/read",
            json!({"threadId":session_id,"includeTurns":false}),
        )
        .await?,
        session_id,
        workspace,
    )?;
    let response=rpc.request("thread/turns/list",json!({"threadId":session_id,"cursor":c.turns,"limit":1,"sortDirection":"asc","itemsView":"notLoaded"})).await?;
    let turns = response["data"]
        .as_array()
        .context("missing visible archive turns")?;
    ensure!(turns.len() <= 1, "archive turn page exceeds bound");
    let mut batch = Batch {
        records: vec![],
        next: None,
        omitted: 0,
        waiting: false,
        version: VERSION.into(),
    };
    let Some(turn) = turns.first() else {
        ensure!(c.turn_id.is_none(), "archive continuation turn disappeared");
        return Ok(batch);
    };
    let turn_id = identity(&turn["id"])?;
    ensure!(
        c.turn_id.as_deref().is_none_or(|id| id == turn_id),
        "archive continuation source changed"
    );
    if turn["status"] == "inProgress" {
        batch.waiting = true;
        batch.next = Some(serde_json::to_string(&c)?);
        return Ok(batch);
    }
    ensure!(
        matches!(
            turn["status"].as_str(),
            Some("completed" | "failed" | "interrupted")
        ),
        "unsupported archive turn status"
    );
    let response_items=rpc.request("thread/items/list",json!({"threadId":session_id,"turnId":turn_id,"cursor":c.items,"limit":limit,"sortDirection":"asc"})).await?;
    let items = response_items["data"]
        .as_array()
        .context("missing visible archive items")?;
    ensure!(
        items.len() <= usize::from(limit),
        "archive item page exceeds bound"
    );
    for entry in items {
        if let Some(r) = normalize(entry, turn_id)? {
            batch.records.push(r)
        } else {
            batch.omitted += 1
        }
    }
    batch.next = if let Some(items) = opaque(&response_items["nextCursor"])? {
        Some(serde_json::to_string(&Cursor {
            version: VERSION.into(),
            turns: c.turns,
            turn_id: Some(turn_id.into()),
            items: Some(items),
        })?)
    } else if let Some(turns) = opaque(&response["nextCursor"])? {
        Some(serde_json::to_string(&Cursor {
            version: VERSION.into(),
            turns: Some(turns),
            ..Default::default()
        })?)
    } else {
        None
    };
    metadata(
        &rpc.request(
            "thread/read",
            json!({"threadId":session_id,"includeTurns":false}),
        )
        .await?,
        session_id,
        workspace,
    )?;
    Ok(batch)
}
enum ReadResult {
    Page(Batch),
    Stopped(ArchiveReadFailure),
}
async fn read(
    executable: &Path,
    source: &ArchiveSourceKey,
    limit: u16,
    cursor: Option<&str>,
    cancel: impl std::future::Future<Output = ArchiveReadFailure>,
) -> Result<ReadResult> {
    tokio::pin!(cancel);
    tokio::select! {r=probe(executable)=>{r?;},reason=&mut cancel=>return Ok(ReadResult::Stopped(reason))}
    let mut rpc = Rpc::spawn(executable, &CodexLaunchSpec::observation()).await?;
    let work = tokio::time::timeout(Duration::from_secs(35), async {
        rpc.request("initialize",json!({"clientInfo":{"name":"lain_visible_archive","title":"Lain visible archive","version":env!("CARGO_PKG_VERSION")}})).await?;
        rpc.notify("initialized").await?;
        page(&rpc, source, limit, cursor).await
    });
    let result = tokio::select! {r=work=>r.unwrap_or_else(|_|Err(anyhow::anyhow!("archive read deadline exceeded"))).map(ReadResult::Page),reason=&mut cancel=>Ok(ReadResult::Stopped(reason))};
    rpc.shutdown().await?;
    result
}
pub(super) async fn probe(executable: &Path) -> Result<String> {
    let out = tokio::time::timeout(
        Duration::from_secs(10),
        rpc::command(executable).arg("--version").output(),
    )
    .await
    .context("archive version probe timeout")??;
    ensure!(
        out.status.success()
            && std::str::from_utf8(&out.stdout)
                .is_ok_and(|v| v.trim() == format!("codex-cli {VERSION}")),
        UnsupportedVersion
    );
    Ok(VERSION.into())
}
impl Supervisor {
    pub(crate) async fn read_archive(
        self: &Arc<Self>,
        source: ArchiveSourceId,
        q: ReadWorkerArchive,
    ) -> Result<ArchiveReadJob> {
        ensure!(!*self.archive_shutdown.borrow(), ControlError::Unavailable);
        let (job, fresh) = blocking(&self.kernel, move |k| k.archive_read_admit(source, q)).await?;
        if fresh {
            let this = self.clone();
            let id = job.request.request_id;
            let permit = match self.archive_slot.clone().try_acquire_owned() {
                Ok(p) => p,
                Err(_) => {
                    return blocking(&self.kernel, move |k| {
                        k.archive_read_fail(
                            id,
                            ArchiveReadState::Failed,
                            ArchiveReadFailure::Unavailable,
                            None,
                        )
                    })
                    .await;
                }
            };
            tokio::spawn(async move {
                let _permit = permit;
                if let Err(error) = this.run_archive_read(id).await {
                    let (reason, code) = if let Some(e) = error.downcast_ref::<rpc::Rejected>() {
                        (ArchiveReadFailure::Unsupported, Some(e.code))
                    } else if error.is::<UnsupportedVersion>() {
                        (ArchiveReadFailure::Unsupported, None)
                    } else if error
                        .downcast_ref::<ControlError>()
                        .is_some_and(|e| matches!(e, ControlError::InvalidInput))
                    {
                        (ArchiveReadFailure::InvalidContent, None)
                    } else if error.is::<ControlError>() {
                        (ArchiveReadFailure::ScopeChanged, None)
                    } else {
                        (ArchiveReadFailure::Unavailable, None)
                    };
                    let _ = blocking(&this.kernel, move |k| {
                        k.archive_read_fail(id, ArchiveReadState::Failed, reason, code)
                    })
                    .await;
                }
            });
        }
        Ok(job)
    }
    async fn run_archive_read(&self, id: RequestId) -> Result<()> {
        let mut changes = self.kernel.subscribe_archives();
        let (j, source, cursor) = blocking(&self.kernel, move |k| k.archive_read_claim(id)).await?;
        let path = self
            .info
            .executable
            .as_deref()
            .ok_or(ControlError::Unavailable)?;
        let mut shutdown = self.kernel.subscribe_shutdown();
        let mut archive_shutdown = self.archive_shutdown.subscribe();
        let invalidated = async {
            loop {
                let _ = changes.recv().await;
                if !blocking(&self.kernel, move |k| k.archive_read_current(id))
                    .await
                    .unwrap_or(false)
                {
                    break;
                }
            }
        };
        let cancelled = async {
            tokio::select! {_=async {let _=shutdown.wait_for(|v|*v).await;}=>ArchiveReadFailure::Interrupted,_=async{let _=archive_shutdown.wait_for(|v|*v).await;}=>ArchiveReadFailure::Interrupted,_=invalidated=>ArchiveReadFailure::ScopeChanged}
        };
        let result = match read(
            path,
            &source.key,
            j.request.limit,
            cursor.as_deref(),
            cancelled,
        )
        .await?
        {
            ReadResult::Page(batch) => batch,
            ReadResult::Stopped(reason) => {
                blocking(&self.kernel, move |k| {
                    k.archive_read_fail(id, ArchiveReadState::Cancelled, reason, None)
                })
                .await?;
                return Ok(());
            }
        };
        let redactor = self.redactor.for_runtime(None);
        blocking(&self.kernel, move |k| {
            k.archive_read_finish(id, result, redactor)
        })
        .await?;
        Ok(())
    }
}
