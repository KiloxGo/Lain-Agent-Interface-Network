//! Selected read-only pulls and independently granted finite source synchronization.
use crate::{
    archives,
    backend::ControlError,
    kernel::{Kernel, blocking},
    profiles::{SecretRef, SecretValue, resolve_secret},
    redaction::Redactor,
};
use anyhow::{Result, ensure};
use lain_wire::{ArchiveSourceId, RequestId, archives::*};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{io::Read, sync::Arc, time::Duration};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
    pub endpoint: String,
    pub store_id: String,
    pub secret_ref: SecretRef,
}
impl Config {
    pub(crate) fn load() -> Result<Self> {
        let path = std::env::var_os("LAIN_KNOWLEDGE_CONFIG").ok_or(ControlError::Unavailable)?;
        let f = std::fs::File::open(path).map_err(|_| ControlError::Unavailable)?;
        ensure!(
            f.metadata()
                .map_err(|_| ControlError::Unavailable)?
                .is_file(),
            ControlError::InvalidInput
        );
        let mut bytes = vec![];
        f.take(16385)
            .read_to_end(&mut bytes)
            .map_err(|_| ControlError::Unavailable)?;
        ensure!(bytes.len() <= 16384, ControlError::InvalidInput);
        let c: Self = serde_json::from_slice(&bytes).map_err(|_| ControlError::InvalidInput)?;
        c.validate()?;
        Ok(c)
    }
    fn validate(&self) -> Result<()> {
        archives::safe(&self.endpoint, &Redactor::new(vec![]).for_runtime(None))?;
        ensure!(
            uuid::Uuid::parse_str(&self.store_id).is_ok(),
            ControlError::InvalidInput
        );
        let u = reqwest::Url::parse(&self.endpoint).map_err(|_| ControlError::InvalidInput)?;
        let local = u.host_str().is_some_and(|h| {
            h.parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
        });
        ensure!(
            (u.scheme() == "https" || (u.scheme() == "http" && local))
                && u.username().is_empty()
                && u.password().is_none()
                && u.query().is_none()
                && u.fragment().is_none()
                && !u.cannot_be_a_base()
                && self.endpoint.len() <= 2048,
            ControlError::InvalidInput
        );
        Ok(())
    }
    pub(crate) fn revision(&self) -> String {
        let mut bytes = b"qq-knowledge/scoped-v1\0".to_vec();
        bytes.extend(serde_json::to_vec(self).expect("knowledge config"));
        lain_wire::actor_configuration_revision(&bytes)
    }
}
pub(crate) fn source_key(key: &ArchiveSourceKey) -> Result<String> {
    Ok(match key {
        ArchiveSourceKey::QqPrivate {
            observer_account,
            peer_id,
        } => format!("qq:{observer_account}:private:{peer_id}"),
        ArchiveSourceKey::QqGroup {
            observer_account,
            group_id,
        } => format!("qq:{observer_account}:group:{group_id}"),
        _ => anyhow::bail!(ControlError::InvalidInput),
    })
}
#[derive(Deserialize)]
pub(crate) struct Source {
    pub id: String,
    pub account: String,
    pub kind: String,
    pub peer: String,
    pub revision: u64,
    pub enabled: bool,
}
#[derive(Deserialize)]
pub(crate) struct Page {
    pub schema_version: u32,
    pub store_id: String,
    pub source: Source,
    pub from_after: u64,
    pub through: u64,
    pub next_after: u64,
    pub has_more: bool,
    pub items: Vec<Change>,
}
#[derive(Deserialize)]
pub(crate) struct Change {
    pub sequence: u64,
    pub message: Message,
}
#[derive(Deserialize, Serialize)]
pub(crate) struct Message {
    pub id: u64,
    pub source_id: String,
    pub external_id: String,
    pub sender: String,
    pub occurred_at_ms: u64,
    pub received_at_ms: u64,
    pub expires_at_ms: u64,
    pub state: String,
    pub revision: u64,
    pub text: Option<String>,
    pub images: Vec<Value>,
}
impl Page {
    pub(crate) fn validate(
        &self,
        c: &Config,
        key: &ArchiveSourceKey,
        after: u64,
        limit: u16,
    ) -> Result<()> {
        let id = source_key(key)?;
        let (account, kind, peer) = match key {
            ArchiveSourceKey::QqPrivate {
                observer_account,
                peer_id,
            } => (observer_account, "private", peer_id),
            ArchiveSourceKey::QqGroup {
                observer_account,
                group_id,
            } => (observer_account, "group", group_id),
            _ => anyhow::bail!(ControlError::InvalidInput),
        };
        ensure!(
            self.schema_version == 1
                && self.store_id == c.store_id
                && self.source.id == id
                && self.source.account == *account
                && self.source.kind == kind
                && self.source.peer == *peer
                && self.source.revision > 0
                && self.source.revision <= i64::MAX as u64
                && self.from_after == after
                && after <= self.next_after
                && self.next_after <= self.through
                && self.through <= i64::MAX as u64
                && self.items.len() <= usize::from(limit),
            ControlError::InvalidInput
        );
        let mut previous = after;
        for item in &self.items {
            ensure!(
                item.sequence > previous && item.sequence <= self.through,
                ControlError::InvalidInput
            );
            previous = item.sequence;
            let m = &item.message;
            ensure!(
                m.id > 0
                    && m.id <= i64::MAX as u64
                    && m.source_id == id
                    && m.external_id.len() <= 256
                    && m.sender.len() <= 256
                    && [&m.external_id, &m.sender]
                        .iter()
                        .all(|s| !s.trim().is_empty() && !s.chars().any(char::is_control))
                    && m.text.as_ref().is_none_or(|s| s.len() <= 65536)
                    && m.revision > 0
                    && m.revision <= i64::MAX as u64
                    && [m.occurred_at_ms, m.received_at_ms, m.expires_at_ms]
                        .into_iter()
                        .all(|n| n <= i64::MAX as u64)
                    && m.images.len() <= 8
                    && matches!(
                        m.state.as_str(),
                        "active" | "recalled" | "forgotten" | "revoked" | "expired"
                    ),
                ControlError::InvalidInput
            );
            if m.state != "active" {
                ensure!(
                    m.text.is_none() && m.images.is_empty(),
                    ControlError::InvalidInput
                );
            }
            for image in &m.images {
                ensure!(
                    image["id"].as_str().is_some_and(|s| !s.is_empty()
                        && s.len() <= 128
                        && !s.chars().any(char::is_control))
                        && image["state"].as_str().is_some_and(|s| s.len() <= 64)
                        && [&image["model"], &image["prompt_revision"]]
                            .iter()
                            .all(|v| v.is_null()
                                || v.as_str().is_some_and(
                                    |s| s.len() <= 128 && !s.chars().any(char::is_control)
                                ))
                        && (image["description"].is_null()
                            || image["description"]
                                .as_str()
                                .is_some_and(|s| s.len() <= 65536)),
                    ControlError::InvalidInput
                );
            }
        }
        ensure!(
            self.next_after == previous
                && (self.has_more || self.next_after == self.through)
                && (!self.has_more || !self.items.is_empty()),
            ControlError::InvalidInput
        );
        Ok(())
    }
}
fn excerpt(text: &str, max: usize) -> Value {
    let mut n = text.len().min(max);
    while !text.is_char_boundary(n) {
        n -= 1;
    }
    json!({"text":&text[..n],"original_bytes":text.len(),"truncated":n<text.len()})
}
pub(crate) fn projection(m: &Message) -> Result<String> {
    let images=m.images.iter().take(2).map(|i|json!({"asset_id":i["id"],"state":i["state"],"description_origin":i["description"].as_str().map(|_|"external_model_inference_not_sender_words"),"description":i["description"].as_str().map(|s|excerpt(s,512)),"model":i["model"],"prompt_revision":i["prompt_revision"],"processing_policy":"first_frame_resized_at_most_1280x1280_not_full_animation_or_verified_ocr"})).collect::<Vec<_>>();
    let text = serde_json::to_string(
        &json!({"report_kind":"qq_observed_projection","participation":"observation_only","native_message_id":m.external_id,"upstream_record_id":m.id,"source_id":m.source_id,"images":images,"omitted_images":m.images.len().saturating_sub(2),"original_message_text":m.text.as_deref().map(|s|excerpt(s,2048)),"image_bytes":"retained_only_in_independent_source_service"}),
    )?;
    archives::validate_text(&text)?;
    Ok(text)
}
async fn fetch(
    c: &Config,
    key: &ArchiveSourceKey,
    after: u64,
    limit: u16,
    r: &Redactor,
    credential: &SecretValue,
) -> Result<Page> {
    let mut url = reqwest::Url::parse(&format!(
        "{}/v1/sources/{}/changes",
        c.endpoint.trim_end_matches('/'),
        source_key(key)?
    ))
    .map_err(|_| ControlError::InvalidInput)?;
    url.query_pairs_mut()
        .append_pair("source_id", &source_key(key)?)
        .append_pair("after", &after.to_string())
        .append_pair("limit", &limit.to_string());
    let client = reqwest::Client::builder()
        .retry(reqwest::retry::never())
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|_| ControlError::Unavailable)?;
    let mut response = client
        .get(url)
        .bearer_auth(credential.0.as_str())
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|_| ControlError::Unavailable)?;
    if response.status() != reqwest::StatusCode::OK {
        anyhow::bail!(HttpRejected(response.status().as_u16()));
    }
    ensure!(
        response
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .is_some_and(|v| v.split(';').next() == Some("application/json"))
            && response.content_length().is_none_or(|n| n <= 1024 * 1024),
        ControlError::Unavailable
    );
    let mut bytes = vec![];
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| ControlError::Unavailable)?
    {
        ensure!(
            bytes.len() + chunk.len() <= 1024 * 1024,
            ControlError::InvalidInput
        );
        bytes.extend_from_slice(&chunk);
    }
    let p: Page = serde_json::from_slice(&bytes).map_err(|_| ControlError::InvalidInput)?;
    p.validate(c, key, after, limit)?;
    archives::safe(&p.source.id, r)?;
    for item in &p.items {
        archives::safe(&item.message, &r.for_runtime(Some(credential)))?;
    }
    Ok(p)
}
#[derive(Debug, thiserror::Error)]
#[error("knowledge HTTP response rejected")]
struct HttpRejected(u16);
pub(crate) async fn start(
    k: Arc<Kernel>,
    source: ArchiveSourceId,
    q: ReadArchive,
    r: Redactor,
) -> Result<ArchiveReadJob> {
    start_config(k, source, q, r, Config::load()?).await
}
pub(crate) async fn start_config(
    k: Arc<Kernel>,
    source: ArchiveSourceId,
    q: ReadArchive,
    r: Redactor,
    c: Config,
) -> Result<ArchiveReadJob> {
    c.validate()?;
    let credential = resolve_secret(&c.secret_ref).map_err(|_| ControlError::Unavailable)?;
    start_resolved(k, source, q, r, c, credential).await
}
async fn start_resolved(
    k: Arc<Kernel>,
    source: ArchiveSourceId,
    q: ReadArchive,
    r: Redactor,
    c: Config,
    credential: SecretValue,
) -> Result<ArchiveReadJob> {
    c.validate()?;
    let revision = c.revision();
    let (job, fresh) = blocking(&k, move |k| k.knowledge_admit(source, q, revision)).await?;
    if fresh {
        launch(k, job.clone(), r, c, credential).await;
    }
    Ok(job)
}
async fn launch(
    k: Arc<Kernel>,
    job: ArchiveReadJob,
    r: Redactor,
    c: Config,
    credential: SecretValue,
) {
    let permit = k.knowledge_slot.clone().try_acquire_owned();
    let id = job.request.request_id;
    let Ok(permit) = permit else {
        let _ = blocking(&k, move |k| {
            k.archive_read_fail(
                id,
                ArchiveReadState::Failed,
                ArchiveReadFailure::Unavailable,
                None,
            )
        })
        .await;
        return;
    };
    launch_owned(k, job, r, c, credential, permit);
}
fn launch_owned(
    k: Arc<Kernel>,
    job: ArchiveReadJob,
    r: Redactor,
    c: Config,
    credential: SecretValue,
    permit: tokio::sync::OwnedSemaphorePermit,
) {
    let id = job.request.request_id;
    tokio::spawn(async move {
        let _permit = permit;
        let result = run(&k, id, &c, &r, &credential).await;
        if let Err(error) = result {
            let reason = match error.downcast_ref::<ControlError>() {
                Some(ControlError::InvalidInput) => ArchiveReadFailure::InvalidContent,
                Some(ControlError::Conflict) => ArchiveReadFailure::ScopeChanged,
                _ => ArchiveReadFailure::Unavailable,
            };
            let code = error.downcast_ref::<HttpRejected>().map(|e| i64::from(e.0));
            let _ = blocking(&k, move |k| {
                k.archive_read_fail(id, ArchiveReadState::Failed, reason, code)
            })
            .await;
        }
    });
}
async fn run(
    k: &Arc<Kernel>,
    id: RequestId,
    c: &Config,
    r: &Redactor,
    credential: &SecretValue,
) -> Result<()> {
    let mut changes = k.subscribe_archives();
    let mut shutdown = k.subscribe_shutdown();
    let (job, source, cursor) = blocking(k, move |k| k.archive_read_claim(id)).await?;
    ensure!(
        job.reader == ArchiveReaderKind::Knowledge
            && job.reader_revision.as_deref() == Some(c.revision().as_str()),
        ControlError::Conflict
    );
    let after = cursor
        .as_deref()
        .map(str::parse::<u64>)
        .transpose()
        .map_err(|_| ControlError::Conflict)?
        .unwrap_or(0);
    let expiry = if job.sync_revision.is_some() {
        blocking(k, move |k| k.knowledge_sync_grant(job.source_id))
            .await?
            .scope
            .map(|s| s.expires_at_ms)
    } else {
        None
    };
    let invalid = async {
        loop {
            let timer = async {
                if let Some(expiry) = expiry {
                    tokio::time::sleep(Duration::from_millis(
                        expiry.saturating_sub(crate::domain::now_ms().unwrap_or(u64::MAX)),
                    ))
                    .await;
                } else {
                    std::future::pending::<()>().await;
                }
            };
            tokio::select! { _=changes.recv()=>{}, _=timer=>{} }
            if !blocking(k, move |k| k.archive_read_current(id))
                .await
                .unwrap_or(false)
            {
                break;
            }
        }
    };
    let work = tokio::time::timeout(
        Duration::from_secs(15),
        fetch(c, &source.key, after, job.request.limit, r, credential),
    );
    let page = tokio::select! {p=work=>p.map_err(|_|ControlError::Unavailable)??,_=invalid=>{blocking(k,move|k|k.archive_read_fail(id,ArchiveReadState::Cancelled,ArchiveReadFailure::ScopeChanged,None)).await?;return Ok(())},_=async {let _=shutdown.wait_for(|v|*v).await;}=>{blocking(k,move|k|k.archive_read_fail(id,ArchiveReadState::Cancelled,ArchiveReadFailure::Interrupted,None)).await?;return Ok(())}};
    let redactor = r.clone();
    blocking(k, move |k| k.knowledge_finish(id, page, redactor)).await?;
    Ok(())
}
/// One timer for persisted source grants. No generic scheduler or model polling.
pub async fn maintain(k: Arc<Kernel>, actor: Arc<crate::actor::Actor>) {
    let r = actor.source_redactor();
    let mut changes = k.subscribe_archives();
    let mut shutdown = k.subscribe_shutdown();
    loop {
        if *shutdown.borrow() {
            return;
        }
        // Admission must not spend a request while a just-completed read still
        // owns its runtime slot. Wait for ownership, not a polling retry.
        let permit = tokio::select! {
            p=k.knowledge_slot.clone().acquire_owned()=>match p {Ok(p)=>p,Err(_)=>return},
            _=shutdown.changed()=>{return;}
        };
        // Configuration changes stop the selected scope instead of silently moving it.
        let selected = Config::load()
            .and_then(|c| {
                let key = resolve_secret(&c.secret_ref).map_err(|_| ControlError::Unavailable)?;
                archives::safe(&c.endpoint, &r.for_runtime(Some(&key)))?;
                Ok((c, key))
            })
            .ok();
        let revision = selected.as_ref().map(|(c, _)| c.revision());
        let (job, deadline) = match blocking(&k, move |k| k.knowledge_sync_plan(revision)).await {
            Ok(plan) => plan,
            Err(_) => {
                tracing::error!("knowledge synchronization stopped; inspect durable source state");
                return;
            }
        };
        if let Some(job) = job {
            let Some((c, key)) = selected else { return };
            launch_owned(k.clone(), job, r.clone(), c, key, permit);
        } else {
            drop(permit);
        }
        let timer = async {
            if let Some(deadline) = deadline {
                let now = crate::domain::now_ms().unwrap_or(0);
                tokio::time::sleep(Duration::from_millis(deadline.saturating_sub(now))).await;
            } else {
                std::future::pending::<()>().await;
            }
        };
        tokio::select! {_=timer=>{},_=changes.recv()=>{},_=shutdown.changed()=>{}}
    }
}

pub async fn drain(k: &Arc<Kernel>) -> Result<()> {
    let _ = tokio::time::timeout(Duration::from_secs(16), k.knowledge_slot.acquire())
        .await
        .map_err(|_| ControlError::Unavailable)?;
    Ok(())
}

#[cfg(test)]
mod tests;
