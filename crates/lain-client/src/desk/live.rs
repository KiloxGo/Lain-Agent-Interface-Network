//! Connection generation and read lifecycle. Never owns Worker cancellation.
use super::*;
use crate::api::{BootstrapLease, HttpFailure};
use lain_wire::client_sync::*;
use tokio::{task::AbortHandle, time::Instant};

pub(super) struct State {
    generation: u64,
    fresh: bool,
    fetching: bool,
    deadline: Option<Instant>,
    readers: Vec<AbortHandle>,
    watcher: Option<AbortHandle>,
    fetcher: Option<AbortHandle>,
    floor: Option<SyncCursor>,
    retry_seconds: u64,
    blocked: bool,
    pub cache: Option<crate::sync_cache::Cache>,
    pub metadata: Option<Metadata>,
}
pub(super) struct Metadata {
    context_current: bool,
    pending_permission: bool,
    unread_notice: bool,
}
pub(super) enum Update {
    Snapshot(Result<Box<BootstrapLease>>),
    Changed(Result<Invalidation>),
}
impl State {
    pub fn new() -> Self {
        Self {
            generation: 0,
            fresh: false,
            fetching: false,
            deadline: None,
            readers: vec![],
            watcher: None,
            fetcher: None,
            floor: None,
            retry_seconds: 2,
            blocked: false,
            cache: None,
            metadata: None,
        }
    }
}
pub(super) fn epoch(app: &Model) -> Option<u64> {
    app.live.as_ref().map(|s| s.generation)
}
pub(super) fn fresh(app: &Model) -> bool {
    app.live
        .as_ref()
        .is_none_or(|s| s.fresh && s.deadline.is_some_and(|d| Instant::now() < d))
}
pub(super) fn deadline(app: &Model) -> Option<Instant> {
    app.live.as_ref().and_then(|s| s.deadline)
}
pub(super) fn effect(tag: Option<u64>, result: Complete) -> Complete {
    match tag {
        Some(g) => Complete::Effect(g, Box::new(result)),
        None => result,
    }
}
pub(super) fn track(app: &mut Model, handle: AbortHandle) {
    if let Some(s) = app.live.as_mut() {
        s.readers.retain(|h| !h.is_finished());
        if s.readers.len() >= 64 {
            s.readers.remove(0).abort();
        }
        s.readers.push(handle);
    }
}
pub(super) fn spawn_read(
    app: &mut Model,
    jobs: &mut JoinSet<()>,
    future: impl std::future::Future<Output = ()> + Send + 'static,
) {
    let h = jobs.spawn(future);
    track(app, h);
}
fn sender(
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
    is_effect: bool,
) -> mpsc::Sender<Complete> {
    let Some(g) = epoch(app) else {
        return tx.clone();
    };
    let (out, mut incoming) = mpsc::channel(16);
    let target = tx.clone();
    let handle = jobs.spawn(async move {
        while let Some(result) = incoming.recv().await {
            let result = if is_effect {
                Complete::Effect(g, Box::new(result))
            } else {
                Complete::Read(g, Box::new(result))
            };
            if target.send(result).await.is_err() {
                break;
            }
        }
    });
    if !is_effect {
        track(app, handle);
    }
    out
}
pub(super) fn read_sender(
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
) -> mpsc::Sender<Complete> {
    sender(app, tx, jobs, false)
}
pub(super) fn effect_sender(
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
) -> mpsc::Sender<Complete> {
    sender(app, tx, jobs, true)
}

pub(super) fn retire(app: &mut Model) {
    new_work::close(app);
    source_access::invalidate(app);
    source_setup::invalidate(app);
    if let Some(s) = app.live.as_mut() {
        s.generation += 1;
        s.fresh = false;
        s.deadline = None;
        s.fetching = false;
        for h in s.readers.drain(..) {
            h.abort();
        }
        if let Some(h) = s.watcher.take() {
            h.abort();
        }
        if let Some(h) = s.fetcher.take() {
            h.abort();
        }
        s.metadata = None;
        if let Some(cache) = &s.cache {
            let _ = cache.expire_resources();
        }
    }
    app.generation += 1;
    app.refreshing = false;
    app.connected = false;
    app.remote_connected = false;
    app.transcript.clear();
    app.text_view.clear();
    app.work_text.clear();
    app.work_text_target = None;
    app.task_history = work_history::History::default();
    app.work_history_page = 0;
    app.renderer = Renderer::new(Vec::new());
    app.users.clear();
    app.turn = None;
    app.source_invalidated = true;
    app.review = None;
    app.remote_review = None;
    app.cancel_review = None;
    app.task_decisions = None;
    if let Some(p) = app.followup.take() {
        app.worker_drafts
            .insert((p.target.device_id, p.target.id), p.input);
    }
    app.sessions.clear();
    app.worker_status = sidebar::Workers::default();
    app.workspace_context = None;
    app.devices.clear();
    app.device_read_at = None;
    app.device_read_error = false;
    app.device_page = super::devices_view::Page::Overview;
    app.detail_viewport = None;
    app.sources.clear();
    app.source_read_error = false;
    app.tasks.clear();
    app.task = None;
    app.remote_work.clear();
    app.remote_versions.clear();
    app.activity = activity::Timeline::default();
    app.notices.clear();
    app.observing.clear();
    app.chat_checking = false;
    if let Some(p) = app.pending.as_mut() {
        p.uncertain = true;
    }
    app.status = if app.live.as_ref().is_some_and(|s| s.blocked) {
        "Authentication or protocol unavailable · Ctrl-R after configuration; draft retained."
    } else {
        "Synchronizing · draft retained; no operation replay."
    }
    .into();
}
pub(super) fn request(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
    delay: u64,
) {
    let Some(s) = app.live.as_mut() else {
        return;
    };
    if s.fetching || s.blocked {
        return;
    }
    s.fetching = true;
    let g = s.generation;
    let c = client.clone();
    let out = tx.clone();
    let conversation = app.conversation;
    s.fetcher = Some(jobs.spawn(async move {
        if delay > 0 {
            tokio::time::sleep(Duration::from_secs(delay)).await;
        }
        let result = c.bootstrap_lease(Some(conversation)).await.map(Box::new);
        let _ = out.send(Complete::Live(g, Update::Snapshot(result))).await;
    }));
}
pub(super) fn refresh(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
) {
    let timeline = activity::checkpoint(app);
    retire(app);
    app.activity = timeline;
    request(client, app, tx, jobs, 0);
}
pub(super) fn resume(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
) {
    if let Some(s) = app.live.as_mut() {
        s.blocked = false;
    }
    refresh(client, app, tx, jobs);
}

pub(super) fn failure_status(result: &Complete) -> Option<u16> {
    let error = match result {
        Complete::SourceSetupOpened(_, _, result) => result.as_ref().as_ref().err(),
        Complete::SourceSetupSent(result) => match &result.1 {
            Err(
                source_setup::Failure::NotSent(e)
                | source_setup::Failure::Rejected(e)
                | source_setup::Failure::Uncertain(e),
            ) => Some(e),
            _ => None,
        },
        Complete::SourceAccessOpened(_, result) => result.as_ref().as_ref().err(),
        Complete::SourceAccessSent(result) => match &result.1 {
            Err(
                source_access::Failure::BeforePost(e)
                | source_access::Failure::Rejected(e)
                | source_access::Failure::Uncertain(e),
            ) => Some(e),
            _ => None,
        },
        Complete::NewWorkTargets(_, Err(e)) => Some(e),
        Complete::NewWorkSent(result) => match &result.1 {
            Err(new_work::Failure::BeforePost(e) | new_work::Failure::Uncertain(e)) => Some(e),
            _ => None,
        },
        Complete::ChatStatus(Err(e))
        | Complete::StreamEnded(_, Err(e))
        | Complete::History(_, Err(e))
        | Complete::Sessions(_, Err(e))
        | Complete::Devices(_, Err(e))
        | Complete::Sources(_, Err(e))
        | Complete::Tasks(_, _, Err(e))
        | Complete::Task(_, _, _, Err(e))
        | Complete::RemoteWork(Err(e))
        | Complete::NoticeEnded(Err(e))
        | Complete::Submit(_, Err(e))
        | Complete::Mutation(Err(e))
        | Complete::RemoteDecision(_, Err(e)) => Some(e),
        Complete::FollowupOpened(_, _, _, result) => result.as_ref().as_ref().err(),
        Complete::RemoteCancelled(result) => match &result.1 {
            Err(cancel::Failure::BeforePost(e) | cancel::Failure::Uncertain(e)) => Some(e),
            _ => None,
        },
        Complete::FollowupSent(result) => match &result.1 {
            Err(followup::Failure::BeforePost(e) | followup::Failure::Uncertain(e)) => Some(e),
            _ => None,
        },
        _ => None,
    };
    error
        .and_then(|e| e.downcast_ref::<HttpFailure>())
        .filter(|e| matches!(e.0, 401 | 403 | 426))
        .map(|e| e.0)
}
fn fatal(e: &anyhow::Error) -> bool {
    e.downcast_ref::<HttpFailure>()
        .is_some_and(|e| matches!(e.0, 401 | 403 | 426))
}
pub(super) fn expired(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
) {
    if deadline(app).is_some_and(|d| Instant::now() >= d) {
        refresh(client, app, tx, jobs);
    }
}
pub(super) fn install(
    client: &Client,
    app: &mut Model,
    lease: BootstrapLease,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
) -> Result<()> {
    ensure!(
        Instant::now() < lease.deadline,
        "snapshot expired before display"
    );
    let snapshot = lease.snapshot;
    if let Some(floor) = app.live.as_ref().and_then(|s| s.floor) {
        ensure!(
            snapshot.cursor.instance_id == floor.instance_id
                && snapshot.cursor.revision >= floor.revision,
            "snapshot predates an observed invalidation"
        );
    }
    ensure!(
        app.bootstrap(snapshot.clone())?,
        "synchronization snapshot regressed"
    );
    let s = app.live.as_mut().context("live state missing")?;
    s.fresh = true;
    s.fetching = false;
    s.fetcher = None;
    s.floor = Some(snapshot.cursor);
    s.retry_seconds = 2;
    s.deadline = Some(lease.deadline);
    let context_current = snapshot
        .memory_context
        .as_ref()
        .and_then(|c| c.selection.as_ref())
        .is_none_or(|c| {
            snapshot.devices.iter().any(|d| {
                d.id == c.device_id
                    && d.revision == c.device_revision
                    && d.state == lain_wire::devices::DeviceState::Active
                    && d.expires_at_ms > snapshot.issued_at_ms
                    && d.workspace_ids.contains(&c.workspace_id)
            }) && c.project.as_ref().is_none_or(|p| {
                snapshot.projects.iter().any(|v| {
                    v.id == p.project_id
                        && v.revision == p.project_revision
                        && v.state == lain_wire::projects::ProjectState::Active
                }) && snapshot.mappings.iter().any(|m| {
                    m.device_id == c.device_id
                        && m.workspace_id == c.workspace_id
                        && m.device_revision == c.device_revision
                        && m.project_id == Some(p.project_id)
                        && m.revision == p.mapping_revision
                        && m.reviewed_project_revision == Some(p.project_revision)
                })
            })
        });
    s.metadata = Some(Metadata {
        context_current,
        pending_permission: snapshot
            .work
            .iter()
            .any(|w| !w.pending_approvals.is_empty()),
        unread_notice: snapshot.notices.iter().any(|n| !n.acknowledged),
    });
    let cache_ok = s.cache.as_ref().is_none_or(|cache| {
        cache
            .save(
                client.endpoint(),
                &snapshot,
                lease
                    .deadline
                    .saturating_duration_since(Instant::now())
                    .as_millis()
                    .try_into()
                    .unwrap_or(0),
            )
            .is_ok()
    });
    let g = s.generation;
    let c = client.clone();
    let out = tx.clone();
    let deadline = lease.deadline;
    s.watcher=Some(jobs.spawn(async move {
        let result=tokio::select!{r=c.sync_wait(&snapshot)=>r,_=tokio::time::sleep_until(deadline)=>Ok(Invalidation{cursor:snapshot.cursor,reason:InvalidationReason::Expired})};
        let _=out.send(Complete::Live(g,Update::Changed(result))).await;
    }));
    app.connected = true;
    app.status = if !cache_ok {
        "Connected · synchronized; local metadata cache unavailable."
    } else if s.metadata.as_ref().is_some_and(|m| !m.context_current) {
        "Connected · selected memory context changed; review its selection."
    } else if s.metadata.as_ref().is_some_and(|m| m.pending_permission) {
        "Connected · permission waiting in Work."
    } else if s.metadata.as_ref().is_some_and(|m| m.unread_notice) {
        "Connected · work update available."
    } else {
        "Connected · synchronized"
    }
    .into();
    super::start_backgrounds(client, app, tx, jobs);
    source_access::resume(client, app, tx, jobs);
    source_setup::resume(client, app, tx, jobs);
    Ok(())
}
pub(super) fn complete(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
    g: u64,
    update: Update,
) -> Result<()> {
    if epoch(app) != Some(g) {
        return Ok(());
    }
    match update {
        Update::Snapshot(Ok(lease)) => {
            if Instant::now() >= lease.deadline {
                refresh(client, app, tx, jobs);
                return Ok(());
            }
            if let Err(e) = install(client, app, *lease, tx, jobs) {
                retire(app);
                app.status = format!("Synchronization unavailable: {e}. Draft retained.");
            }
        }
        Update::Snapshot(Err(e)) | Update::Changed(Err(e)) => {
            let fatal = fatal(&e);
            let delay = app.live.as_ref().map_or(2, |s| s.retry_seconds);
            retire(app);
            if fatal && let Some(s) = app.live.as_mut() {
                s.blocked = true;
            }
            if fatal && let Some(cache) = app.live.as_ref().and_then(|s| s.cache.as_ref()) {
                let _ = cache.invalidate();
            }
            app.status=if fatal{"Authentication or protocol unavailable · Ctrl-R after configuration; draft retained."}else{"Disconnected · reconnecting; draft retained."}.into();
            if !fatal {
                if let Some(s) = app.live.as_mut() {
                    s.retry_seconds = (delay * 2).min(30);
                }
                request(client, app, tx, jobs, delay);
            }
        }
        Update::Changed(Ok(event)) => {
            if let Some(s) = app.live.as_mut() {
                s.floor = Some(event.cursor);
            }
            refresh(client, app, tx, jobs);
        }
    }
    Ok(())
}
