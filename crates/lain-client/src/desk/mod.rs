//! Lain's conversational terminal surface. Kernel APIs own all execution.
mod activity;
mod cancel;
mod chat_status;
mod clipboard;
mod device_review;
mod devices_view;
mod dispatch;
mod draw;
mod followup;
mod live;
#[cfg(test)]
mod live_tests;
mod new_work;
mod paste;
mod portrait;
mod rich;
mod sidebar;
mod source_access;
mod source_setup;
mod sources_view;
mod telemetry;
#[cfg(test)]
mod tests;
mod text_view;
mod work_history;

use crate::{
    actor_render::Renderer,
    api::{ActorUpdate, Client, NoticeUpdate, Session},
    tui::{self, TerminalGuard, safe_text},
};
use anyhow::{Context, Result, ensure};
use crossterm::event::{
    Event as InputEvent, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent,
    MouseEventKind,
};
use futures_util::StreamExt;
use lain_wire::{
    ActionState, ActorSnapshot, ActorTurnState, ConversationId, DeviceCommandId, DeviceId,
    MessageId, MessageRole, RequestId,
    device_commands::{DeviceCommand, DeviceCommandSummary},
    device_work::{DeviceWorkPage, DeviceWorkRecord},
    devices::DeviceSnapshot,
};
use ratatui::{Terminal, backend::CrosstermBackend};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, IsTerminal},
    time::Duration,
};
use tokio::{sync::mpsc, task::JoinSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Surface {
    Chat,
    Work,
    Devices,
    Device(DeviceId),
    Sources,
    Source(lain_wire::ArchiveSourceId),
    Tasks(DeviceId),
    Task(DeviceId, DeviceCommandId),
}
#[derive(Default)]
struct Composer {
    text: String,
    cursor: usize,
}
impl Composer {
    fn insert(&mut self, text: &str) {
        let text: String = text
            .chars()
            .filter(|c| !c.is_control() || matches!(c, '\r' | '\n' | '\t'))
            .collect();
        if self.text.len() + text.len() <= 32768 {
            self.text.insert_str(self.cursor, &text);
            self.cursor += text.len();
        }
    }
    fn key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.insert("\n")
            }
            KeyCode::Left => {
                self.cursor = self.text[..self.cursor]
                    .char_indices()
                    .last()
                    .map_or(0, |(i, _)| i)
            }
            KeyCode::Right => {
                if let Some(c) = self.text[self.cursor..].chars().next() {
                    self.cursor += c.len_utf8();
                }
            }
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = self.text.len(),
            KeyCode::Backspace if self.cursor > 0 => {
                self.key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
                self.text.remove(self.cursor);
            }
            KeyCode::Delete if self.cursor < self.text.len() => {
                self.text.remove(self.cursor);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                *self = Self::default()
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.insert(&c.to_string())
            }
            _ => {}
        }
    }
}
#[derive(Clone)]
struct Pending {
    id: RequestId,
    text: String,
    uncertain: bool,
}
struct Review {
    turn: lain_wire::ActorTurnId,
    action: lain_wire::ActionId,
    revision: u64,
    text: String,
    scroll: u16,
    read: bool,
    seen_to: usize,
    width: u16,
    page: u16,
}
struct Model {
    live: Option<live::State>,
    conversation: ConversationId,
    surface: Surface,
    input: Composer,
    pending: Option<Pending>,
    turn: Option<ActorSnapshot>,
    renderer: Renderer<Vec<u8>>,
    users: BTreeSet<MessageId>,
    transcript: String,
    source_invalidated: bool,
    applying_history: bool,
    sync_cursor: Option<lain_wire::client_sync::SyncCursor>,
    status: String,
    sessions: Vec<Session>,
    devices: Vec<DeviceSnapshot>,
    device_read_at: Option<u64>,
    device_read_error: bool,
    device_page: devices_view::Page,
    detail_viewport: Option<(ratatui::layout::Rect, u16)>,
    sources: Vec<lain_wire::archives::ArchiveSourceStatus>,
    source_read_error: bool,
    source_access: source_access::State,
    source_setup: source_setup::State,
    tasks: Vec<DeviceCommandSummary>,
    task: Option<DeviceCommand>,
    task_from_work: bool,
    selected: usize,
    scroll: u16,
    follow: bool,
    review: Option<Review>,
    connected: bool,
    chat_status: chat_status::Availability,
    chat_checking: bool,
    request_busy: bool,
    notices: BTreeSet<lain_wire::NoticeId>,
    generation: u64,
    refreshing: bool,
    observing: BTreeSet<lain_wire::ActorTurnId>,
    portrait: Option<String>,
    positions: BTreeMap<Surface, (usize, u16, bool)>,
    remote_work: Vec<DeviceWorkRecord>,
    remote_versions: BTreeMap<(DeviceId, DeviceCommandId), (u64, String)>,
    remote_connected: bool,
    remote_review: Option<device_review::Review>,
    remote_pending: Option<device_review::Pending>,
    remote_deciding: bool,
    task_decisions: Option<Vec<lain_wire::device_execution::DeviceApprovalDecision>>,
    followup: Option<followup::Panel>,
    followup_pending: Option<followup::Pending>,
    followup_busy: bool,
    worker_drafts: BTreeMap<(DeviceId, DeviceCommandId), Composer>,
    cancel_review: Option<cancel::Review>,
    cancel_pending: Option<cancel::Pending>,
    cancel_notice: Option<cancel::Pending>,
    cancel_busy: bool,
    new_work: Option<new_work::Panel>,
    new_work_draft: Composer,
    new_work_preselect: Option<(DeviceId, lain_wire::DeviceWorkspaceId)>,
    new_work_pending: Option<new_work::Pending>,
    new_work_loading: bool,
    new_work_busy: bool,
    paste_pending: Option<paste::Pending>,
    local_info: telemetry::LocalInfo,
    worker_status: sidebar::Workers,
    workspace_context: Option<String>,
    chat_viewport: Option<ratatui::layout::Rect>,
    chat_bottom: u16,
    text_view: text_view::View,
    work_text: text_view::View,
    work_text_target: Option<(DeviceId, DeviceCommandId)>,
    work_diagnostics: bool,
    task_history: work_history::History,
    work_history_page: usize,
    activity: activity::Timeline,
}
impl Model {
    fn remote_heads(&self) -> impl Iterator<Item = &DeviceWorkRecord> {
        self.remote_work
            .iter()
            .filter(|r| r.command.continued_by.is_none())
    }
    fn new(conversation: ConversationId) -> Self {
        Self {
            live: None,
            conversation,
            surface: Surface::Chat,
            input: Composer::default(),
            pending: None,
            turn: None,
            renderer: Renderer::new(Vec::new()),
            users: BTreeSet::new(),
            transcript: String::new(),
            source_invalidated: false,
            applying_history: false,
            sync_cursor: None,
            status: "Ready. /help for commands".into(),
            sessions: vec![],
            devices: vec![],
            device_read_at: None,
            device_read_error: false,
            device_page: devices_view::Page::Overview,
            detail_viewport: None,
            sources: vec![],
            source_read_error: false,
            source_access: source_access::State::default(),
            source_setup: source_setup::State::default(),
            tasks: vec![],
            task: None,
            task_from_work: false,
            selected: 0,
            scroll: 0,
            follow: true,
            review: None,
            connected: true,
            chat_status: chat_status::Availability::Unknown,
            chat_checking: false,
            request_busy: false,
            notices: BTreeSet::new(),
            generation: 0,
            refreshing: false,
            observing: BTreeSet::new(),
            portrait: None,
            positions: BTreeMap::new(),
            remote_work: vec![],
            remote_versions: BTreeMap::new(),
            remote_connected: false,
            remote_review: None,
            remote_pending: None,
            remote_deciding: false,
            task_decisions: None,
            followup: None,
            followup_pending: None,
            followup_busy: false,
            worker_drafts: BTreeMap::new(),
            cancel_review: None,
            cancel_pending: None,
            cancel_notice: None,
            cancel_busy: false,
            new_work: None,
            new_work_draft: Composer::default(),
            new_work_preselect: None,
            new_work_pending: None,
            new_work_loading: false,
            new_work_busy: false,
            paste_pending: None,
            local_info: telemetry::LocalInfo::default(),
            worker_status: sidebar::Workers::default(),
            workspace_context: None,
            chat_viewport: None,
            chat_bottom: 0,
            text_view: text_view::View::default(),
            work_text: text_view::View::default(),
            work_text_target: None,
            work_diagnostics: false,
            task_history: work_history::History::default(),
            work_history_page: 0,
            activity: activity::Timeline::default(),
        }
    }
    fn back(&mut self) {
        self.review = None;
        self.navigate(Surface::Chat);
    }
    fn navigate(&mut self, surface: Surface) {
        if self.surface != surface {
            self.device_page = devices_view::Page::Overview;
        }
        if self.surface != surface {
            self.work_diagnostics = false;
        }
        new_work::close(self);
        source_access::close(self);
        source_setup::close(self);
        self.text_view.menu = None;
        followup::close(self);
        self.positions
            .insert(self.surface, (self.selected, self.scroll, self.follow));
        if self.positions.len() > 128 {
            // Only disposable viewport positions are evicted, never domain state.
            let stale = self
                .positions
                .keys()
                .copied()
                .find(|s| *s != Surface::Chat && *s != surface);
            if let Some(stale) = stale {
                self.positions.remove(&stale);
            }
        }
        if matches!(surface, Surface::Tasks(_)) && self.surface != surface {
            self.tasks.clear();
        }
        if matches!(surface, Surface::Task(..)) && !matches!(self.surface, Surface::Task(..)) {
            self.task_from_work = !matches!(
                self.surface,
                Surface::Devices | Surface::Device(_) | Surface::Tasks(_)
            );
        }
        self.surface = surface;
        (self.selected, self.scroll, self.follow) = self
            .positions
            .get(&surface)
            .copied()
            .unwrap_or((0, 0, true));
        self.task = None;
        self.task_history = work_history::History::default();
        self.work_history_page = 0;
        self.work_text.menu = None;
        self.task_decisions = None;
        self.remote_review = None;
        self.cancel_review = None;
        self.generation += 1;
        self.refreshing = false;
    }
    fn active(&self) -> bool {
        self.turn.as_ref().is_some_and(|t| {
            matches!(
                t.state,
                ActorTurnState::Running | ActorTurnState::PendingConfirmation
            )
        })
    }
    fn prepare(&mut self) -> Option<Pending> {
        if self.source_invalidated {
            self.status = "Source changed; refresh before sending. Draft retained.".into();
            return None;
        }
        if let Some(p) = &self.pending {
            self.status=if p.uncertain {
                "Delivery uncertain. /refresh inspects; /retry sends the same request. New draft retained."
            } else { "Sending the current message; next draft stays editable" }.into();
            return None;
        }
        if self.active() || self.request_busy {
            self.status =
                "Lain's current turn or action is active; your next draft stays editable".into();
            return None;
        }
        if self.input.text.trim().is_empty() {
            return None;
        }
        if let Some(hint) = self.chat_status.hint() {
            self.status = hint.into();
            return None;
        }
        let pending = Pending {
            id: RequestId::new(),
            text: std::mem::take(&mut self.input.text),
            uncertain: false,
        };
        self.input = Composer::default();
        self.pending = Some(pending.clone());
        self.status = "Sending; next draft stays editable".into();
        Some(pending)
    }
    fn submission_failed(&mut self) {
        self.connected = false;
        if let Some(p) = self.pending.as_mut() {
            p.uncertain = true;
        }
        self.status =
            "Delivery uncertain. /refresh inspects; /retry sends the same request. Draft retained."
                .into();
    }
    fn append(&mut self, text: &str) {
        self.text_view.append(text);
        self.append_legacy(text);
    }
    fn append_legacy(&mut self, text: &str) {
        self.transcript.push_str(text);
        if self.transcript.len() > 131072 {
            let mut n = self.transcript.len() - 98304;
            while !self.transcript.is_char_boundary(n) {
                n += 1;
            }
            self.transcript.drain(..n);
        }
    }
    fn snapshot(&mut self, turn: ActorSnapshot) -> Result<bool> {
        turn.validate().map_err(anyhow::Error::msg)?;
        ensure!(
            turn.conversation_id == self.conversation,
            "conversation identity changed"
        );
        if self.source_invalidated && !self.applying_history && turn.availability.is_none() {
            return Ok(false);
        }
        let withheld = turn.availability.is_some();
        if self.turn.as_ref().is_some_and(|old| {
            (turn.sequence < old.sequence && !withheld)
                || (turn.id == old.id
                    && (turn.revision < old.revision
                        || (turn.revision == old.revision
                            && (!withheld || old.availability.is_some()))))
        }) {
            return Ok(false);
        }
        if !withheld && !self.applying_history && self.refreshing && self.surface == Surface::Chat {
            // An in-flight history read may already contain the old turn. Retire
            // its view generation before applying this newer live observation.
            self.generation += 1;
            self.refreshing = false;
        }
        if withheld && !self.applying_history {
            // A revision counts audit writes, not current source visibility.
            // Retire all accumulated rendering until a checked page replaces it.
            self.transcript.clear();
            self.text_view.clear();
            self.work_text.clear();
            self.work_text_target = None;
            self.task_history = work_history::History::default();
            self.renderer = Renderer::new(Vec::new());
            self.users.clear();
            self.review = None;
            self.source_invalidated = true;
            self.connected = false;
            self.generation += 1;
            self.refreshing = false;
            if self
                .turn
                .as_ref()
                .is_some_and(|old| old.sequence > turn.sequence)
            {
                self.turn = None;
                self.status = "Source changed; refresh the conversation. Draft retained.".into();
                return Ok(true);
            }
        }
        for msg in &turn.messages {
            if msg.role == MessageRole::User && self.users.insert(msg.id) {
                self.append_legacy(&format!("\nYou> {}\n", safe_text(&msg.content)));
            }
        }
        self.renderer.snapshot(&turn)?;
        let text = self.renderer.drain_text();
        self.append_legacy(&text);
        self.text_view.snapshot(&turn)?;
        if self
            .pending
            .as_ref()
            .is_some_and(|p| p.id == turn.request_id)
        {
            self.pending = None;
        }
        if self
            .review
            .as_ref()
            .is_some_and(|r| r.turn != turn.id || r.revision != turn.revision)
        {
            self.review = None;
        }
        self.turn = Some(turn);
        Ok(true)
    }
    fn replace_history(&mut self, mut turns: Vec<ActorSnapshot>) -> Result<()> {
        ensure!(
            turns.len() <= 20
                && serde_json::to_vec(&turns)?.len() <= lain_wire::client_sync::MAX_BYTES,
            "history exceeds bound"
        );
        for turn in &turns {
            turn.validate().map_err(anyhow::Error::msg)?;
            ensure!(
                turn.conversation_id == self.conversation,
                "conversation identity changed"
            );
        }
        turns.sort_by_key(|t| t.sequence);
        ensure!(
            turns.windows(2).all(|w| w[0].sequence < w[1].sequence),
            "duplicate history sequence"
        );
        self.transcript.clear();
        self.text_view.clear();
        self.renderer = Renderer::new(Vec::new());
        self.users.clear();
        self.turn = None;
        self.review = None;
        self.applying_history = true;
        let result = (|| {
            for turn in &turns {
                self.snapshot(turn.clone())?;
            }
            // Each restricted snapshot retires accumulated bodies. Rebuild only
            // content-free save cards from this fully checked history afterwards,
            // so a later withheld read cannot erase an earlier actual write.
            for turn in &turns {
                text_view::reviewed_receipts(&mut self.text_view.doc, turn);
            }
            Ok(())
        })();
        self.applying_history = false;
        self.source_invalidated = result.is_err();
        result
    }
    fn bootstrap(&mut self, snapshot: lain_wire::client_sync::BootstrapSnapshot) -> Result<bool> {
        snapshot.validate().map_err(anyhow::Error::msg)?;
        ensure!(
            snapshot.conversation_id == Some(self.conversation),
            "bootstrap selection changed"
        );
        if let Some(previous) = self.sync_cursor {
            ensure!(
                snapshot.cursor.instance_id == previous.instance_id,
                "Kernel instance changed; inspect the connection before continuing"
            );
            if snapshot.cursor.revision < previous.revision {
                return Ok(false);
            }
        }
        let cursor = snapshot.cursor;
        activity::bootstrap_notices(self, &snapshot);
        self.workspace_context = snapshot
            .memory_context
            .as_ref()
            .and_then(|c| c.selection.as_ref())
            .map(|s| {
                format!(
                    "设备 {} / {}",
                    draw::short(s.device_id),
                    draw::short(s.workspace_id)
                )
            });
        self.replace_history(snapshot.turns)?;
        self.sync_cursor = Some(cursor);
        Ok(true)
    }
    fn open_review(&mut self) -> Result<()> {
        let turn = self
            .turn
            .as_ref()
            .context("No current Lain action to review")?;
        let action = turn
            .actions
            .iter()
            .find(|a| a.state == ActionState::Proposed)
            .context("No pending Lain action; Worker permissions are reviewed in its session")?;
        self.review = Some(Review {
            turn: turn.id,
            action: action.id,
            revision: turn.revision,
            text: dispatch::review_text(turn, action)?,
            scroll: 0,
            read: false,
            seen_to: 0,
            width: 0,
            page: 1,
        });
        Ok(())
    }
}
struct TaskView {
    command: DeviceCommand,
    decisions: Vec<lain_wire::device_execution::DeviceApprovalDecision>,
    history: work_history::History,
    device: Option<Box<DeviceSnapshot>>,
}
enum Complete {
    SourceSetupOpened(u64, source_setup::Kind, Box<Result<source_setup::Loaded>>),
    SourceSetupSent(
        Box<(
            source_setup::Pending,
            std::result::Result<lain_wire::ArchiveSourceId, source_setup::Failure>,
        )>,
    ),
    SourceAccessOpened(u64, Box<Result<source_access::Loaded>>),
    SourceAccessSent(
        Box<(
            source_access::Pending,
            std::result::Result<lain_wire::briefing::BriefingGrantReceipt, source_access::Failure>,
        )>,
    ),
    Paste(paste::Pending, Result<String>),
    NewWorkTargets(u64, Result<Vec<DeviceSnapshot>>),
    NewWorkSent(
        Box<(
            new_work::Pending,
            std::result::Result<Option<DeviceWorkRecord>, new_work::Failure>,
        )>,
    ),
    WorkerStatus(Box<crate::api::Update>),
    Live(u64, live::Update),
    Read(u64, Box<Complete>),
    Effect(u64, Box<Complete>),
    ChatStatus(Result<chat_status::Availability>),
    RemoteCancelled(
        Box<(
            cancel::Pending,
            std::result::Result<DeviceCommand, cancel::Failure>,
        )>,
    ),
    FollowupOpened(
        u64,
        DeviceId,
        DeviceCommandId,
        Box<Result<(DeviceWorkRecord, DeviceSnapshot)>>,
    ),
    FollowupSent(
        Box<(
            followup::Pending,
            std::result::Result<Option<DeviceCommand>, followup::Failure>,
        )>,
    ),
    Submit(RequestId, Result<ActorSnapshot>),
    Actor(ActorUpdate),
    StreamEnded(lain_wire::ActorTurnId, Result<()>),
    History(u64, Result<lain_wire::client_sync::BootstrapSnapshot>),
    Sessions(u64, Result<Vec<Session>>),
    Devices(u64, Result<Vec<DeviceSnapshot>>),
    Sources(u64, Result<Vec<lain_wire::archives::ArchiveSourceStatus>>),
    Tasks(u64, DeviceId, Result<Vec<DeviceCommandSummary>>),
    Task(u64, DeviceId, DeviceCommandId, Result<TaskView>),
    Mutation(Result<ActorSnapshot>),
    Notice(NoticeUpdate),
    NoticeEnded(Result<()>),
    Ack(Vec<lain_wire::NoticeId>),
    RemoteWork(Result<DeviceWorkPage>),
    RemoteDecision(
        device_review::Pending,
        Result<Option<lain_wire::device_execution::DeviceApprovalDecision>>,
    ),
}
fn observe(
    client: Client,
    id: lain_wire::ActorTurnId,
    app: &mut Model,
    tx: mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
) {
    if !app.observing.insert(id) {
        return;
    }
    let tx = live::read_sender(app, &tx, jobs);
    live::spawn_read(app, jobs, async move {
        let (a, mut r) = mpsc::channel(16);
        let future = async {
            let mut result = Ok(());
            for _ in 0..3 {
                result = client.actor_observe(id, &a).await;
                if result.is_ok() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(400)).await;
            }
            result
        };
        tokio::pin!(future);
        let result = loop {
            tokio::select! {Some(update)=r.recv()=>{if tx.send(Complete::Actor(update)).await.is_err(){return;}},result=&mut future=>break result}
        };
        while let Ok(update) = r.try_recv() {
            if tx.send(Complete::Actor(update)).await.is_err() {
                return;
            }
        }
        let _ = tx.send(Complete::StreamEnded(id, result)).await;
    });
}
fn submit(
    client: Client,
    conversation: ConversationId,
    pending: Pending,
    tag: Option<u64>,
    tx: mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
) {
    jobs.spawn(async move {
        let result = async {
            let turn: ActorSnapshot = client
                .post(
                    &format!("v1/conversations/{conversation}/turns"),
                    Some(json!({"request_id":pending.id,"text":pending.text})),
                )
                .await?;
            turn.validate().map_err(anyhow::Error::msg)?;
            ensure!(
                turn.conversation_id == conversation && turn.request_id == pending.id,
                "submission identity changed"
            );
            Ok(turn)
        }
        .await;
        let _ = tx
            .send(live::effect(tag, Complete::Submit(pending.id, result)))
            .await;
    });
}
fn refresh(client: &Client, app: &mut Model, tx: &mpsc::Sender<Complete>, jobs: &mut JoinSet<()>) {
    if app.live.is_some() && (app.surface == Surface::Chat || !live::fresh(app)) {
        live::refresh(client, app, tx, jobs);
        return;
    }
    if app.refreshing {
        return;
    }
    app.refreshing = true;
    if app.surface == Surface::Chat {
        chat_status::refresh(client, app, tx, jobs);
    }
    let c = client.clone();
    let tx = live::read_sender(app, tx, jobs);
    let surface = app.surface;
    let generation = app.generation;
    let conversation = app.conversation;
    let cached_work = app.remote_work.clone();
    live::spawn_read(app, jobs, async move {
        let result = match surface {
            Surface::Chat => Complete::History(generation, c.bootstrap(Some(conversation)).await),
            Surface::Work => {
                let (sessions, devices) =
                    tokio::join!(c.get("v1/work-sessions"), c.get("v1/devices"));
                let _ = tx.send(Complete::Devices(generation, devices)).await;
                Complete::Sessions(generation, sessions)
            }
            Surface::Devices | Surface::Device(_) => {
                Complete::Devices(generation, c.get("v1/devices").await)
            }
            Surface::Sources | Surface::Source(_) => {
                Complete::Sources(generation, c.get("v1/archives/status").await)
            }
            Surface::Tasks(id) => Complete::Tasks(
                generation,
                id,
                c.get(&format!("v1/devices/{id}/commands")).await,
            ),
            Surface::Task(id, cid) => Complete::Task(
                generation,
                id,
                cid,
                async {
                    let path = format!("v1/devices/{id}/commands/{cid}");
                    let receipts = format!("{path}/approvals");
                    let (command, decisions) =
                        tokio::try_join!(c.get::<DeviceCommand>(&path), c.get(&receipts))?;
                    let device_path = format!("v1/devices/{id}");
                    let (history, device) = tokio::join!(
                        work_history::load(&c, conversation, &command, &cached_work),
                        c.get::<DeviceSnapshot>(&device_path)
                    );
                    if let Err(e) = &device
                        && work_history::auth_error(e)
                    {
                        return Err(device.unwrap_err());
                    }
                    Ok(TaskView {
                        command,
                        decisions,
                        history: history?,
                        device: device
                            .ok()
                            .filter(|d| d.device.registration.id == id)
                            .map(Box::new),
                    })
                }
                .await,
            ),
        };
        let _ = tx.send(result).await;
    });
}

fn start_backgrounds(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
) {
    let conversation = app.conversation;
    let worker_client = client.clone();
    let worker_tx = live::read_sender(app, tx, jobs);
    live::spawn_read(app, jobs, async move {
        let (updates, mut incoming) = mpsc::channel(8);
        let watching = worker_client.observe(updates);
        tokio::pin!(watching);
        loop {
            tokio::select! {
                Some(update)=incoming.recv()=>if worker_tx.send(Complete::WorkerStatus(Box::new(update))).await.is_err(){return;},
                _=&mut watching=>return,
            }
        }
    });
    chat_status::refresh(client, app, tx, jobs);
    let remote_client = client.clone();
    let remote_tx = live::read_sender(app, tx, jobs);
    live::spawn_read(app, jobs, async move {
        loop {
            let (pages, mut incoming) = mpsc::channel(2);
            let watch = remote_client.device_work_observe(conversation, &pages);
            tokio::pin!(watch);
            let result = loop {
                tokio::select! {
                    Some(page)=incoming.recv()=>if remote_tx.send(Complete::RemoteWork(Ok(page))).await.is_err(){return;},
                    result=&mut watch=>break result,
                }
            };
            while let Ok(page) = incoming.try_recv() {
                if remote_tx
                    .send(Complete::RemoteWork(Ok(page)))
                    .await
                    .is_err()
                {
                    return;
                }
            }
            let error = result
                .err()
                .unwrap_or_else(|| anyhow::anyhow!("remote work observation ended"));
            if remote_tx
                .send(Complete::RemoteWork(Err(error)))
                .await
                .is_err()
            {
                return;
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
    if let Some(t) = &app.turn
        && t.state == ActorTurnState::Running
    {
        let id = t.id;
        observe(client.clone(), id, app, tx.clone(), jobs);
    }
    let notices_client = client.clone();
    let notices_tx = live::read_sender(app, tx, jobs);
    live::spawn_read(app, jobs, async move {
        let (n, mut r) = mpsc::channel(8);
        let mut after = 0;
        let observing = notices_client.notice_observe(conversation, &mut after, &n);
        tokio::pin!(observing);
        let result = loop {
            tokio::select! {Some(n)=r.recv()=>{if notices_tx.send(Complete::Notice(n)).await.is_err(){return;}},result=&mut observing=>break result}
        };
        while let Ok(n) = r.try_recv() {
            if notices_tx.send(Complete::Notice(n)).await.is_err() {
                return;
            }
        }
        let _ = notices_tx.send(Complete::NoticeEnded(result)).await;
    });
}

pub async fn run(
    client: Client,
    conversation: Option<ConversationId>,
    portrait_path: Option<std::path::PathBuf>,
) -> Result<()> {
    ensure!(
        io::stdin().is_terminal() && io::stdout().is_terminal(),
        "Lain desk requires a terminal; use --json ask/work/device commands for scripts"
    );
    let portrait = portrait::load(portrait_path)?;
    // Negotiate before creating/restoring a conversation. This does not run an Actor.
    let handshake = client.bootstrap(None).await?;
    let cache = crate::local::directory()
        .ok()
        .and_then(|root| crate::sync_cache::Cache::open(root).ok());
    let hint = cache.as_ref().and_then(|cache| {
        cache
            .load(client.endpoint(), handshake.cursor.instance_id)
            .ok()
            .flatten()
    });
    if hint
        .as_ref()
        .is_some_and(|record| !record.resource_versions_valid())
        && let Some(cache) = &cache
    {
        let _ = cache.expire_resources();
    }
    let conversation = match conversation {
        Some(id) => id,
        None => match hint {
            Some(record) => record.conversation_id,
            None => crate::chat::create(&client).await?,
        },
    };
    let page = client.bootstrap_lease(Some(conversation)).await?;
    let mut app = Model::new(conversation);
    app.portrait = portrait;
    app.local_info = telemetry::LocalInfo::collect();
    let (tx, mut rx) = mpsc::channel(64);
    let mut jobs = JoinSet::new();
    app.live = Some(live::State::new());
    app.live.as_mut().unwrap().cache = cache;
    live::install(&client, &mut app, page, &tx, &mut jobs)?;
    let mut guard = Some(TerminalGuard::enter_mouse()?);
    let mut terminal = Some(Terminal::new(CrosstermBackend::new(io::stdout()))?);
    terminal.as_mut().unwrap().clear()?;
    let mut keys = Some(EventStream::new());
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        live::expired(&client, &mut app, &tx, &mut jobs);
        if let Some(status) = app.text_view.poll() {
            app.status = status;
        }
        if let Some(status) = app.work_text.poll() {
            app.status = status;
        }
        while let Ok(result) = rx.try_recv() {
            handle_complete(&client, &mut app, &tx, &mut jobs, result)?;
        }
        terminal
            .as_mut()
            .unwrap()
            .draw(|f| draw::render(f, &mut app))?;
        let mut open_worker = None;
        let sync_deadline = live::deadline(&app);
        tokio::select! {
            _=tokio::time::sleep_until(sync_deadline.unwrap_or_else(||tokio::time::Instant::now()+Duration::from_secs(3600))),if sync_deadline.is_some()=>live::expired(&client,&mut app,&tx,&mut jobs),
            _=crate::signals::shutdown()?=>break,
            key=keys.as_mut().unwrap().next()=>match key {
                Some(Ok(InputEvent::Key(key))) if matches!(key.kind,KeyEventKind::Press|KeyEventKind::Repeat)=> {
                    match handle_key(&client,&mut app,&tx,&mut jobs,key)? {
                        Flow::Exit=>break,Flow::Continue=>{},Flow::Worker(id)=>open_worker=Some(id),
                    }
                },
                Some(Ok(InputEvent::Paste(text)))=>handle_paste(&mut app,&text),
                Some(Ok(InputEvent::Mouse(event)))=>{let previous=app.surface;handle_mouse(&mut app,event);if previous!=app.surface {refresh(&client,&mut app,&tx,&mut jobs);}},
                Some(Err(e))=>return Err(e.into()),None=>break,_=>{},
            },
            Some(result)=rx.recv()=> {
                handle_complete(&client,&mut app,&tx,&mut jobs,result)?;
            },
            _=tokio::time::sleep(Duration::from_millis(80)),if app.text_view.copying() || app.work_text.copying()=>{},
            _=interval.tick()=>{
                app.local_info.refresh();
                if app.surface!=Surface::Chat && app.review.is_none(){refresh(&client,&mut app,&tx,&mut jobs);}
            },
            _=jobs.join_next(),if !jobs.is_empty()=>{},
        }
        if let Some(id) = open_worker {
            drop(keys.take());
            drop(terminal.take());
            drop(guard.take());
            let outcome = tui::run(client.clone(), Some(id)).await;
            guard = Some(TerminalGuard::enter_mouse()?);
            terminal = Some(Terminal::new(CrosstermBackend::new(io::stdout()))?);
            terminal.as_mut().unwrap().clear()?;
            keys = Some(EventStream::new());
            app.status = match outcome {
                Ok(()) => "Returned to Lain; Worker lifetime unchanged".into(),
                Err(e) => format!("Worker view closed: {e}"),
            };
            app.back();
        }
    }
    Ok(()) // UI jobs/streams drop; no Actor/Worker/device cancellation is sent.
}

enum Flow {
    Continue,
    Exit,
    Worker(uuid::Uuid),
}
fn handle_paste(app: &mut Model, text: &str) {
    if app.source_setup.panel.is_some() || source_setup::waiting(app) {
        source_setup::paste(app, text);
    } else if app.source_access.panel.is_some() {
        source_access::paste(app, text);
    } else if source_access::waiting(app) {
        app.status = "权限范围正在重读，未粘贴到隐藏的聊天输入".into();
    } else if app.new_work.is_some() {
        if app.new_work_pending.is_none()
            && let Some(panel) = app
                .new_work
                .as_mut()
                .filter(|p| !p.choosing && p.preview.is_none())
        {
            panel.input.insert(text);
        }
    } else if app.followup.is_some() {
        if let Some(panel) = app.followup.as_mut().filter(|p| p.preview.is_none()) {
            panel.input.insert(text);
        }
    } else if app.surface == Surface::Chat
        && app.review.is_none()
        && !app.text_view.reader
        && app.text_view.menu.is_none()
    {
        app.input.insert(text);
    }
}
fn handle_mouse(app: &mut Model, event: MouseEvent) {
    if app.source_setup.panel.is_some() {
        source_setup::mouse(app, event);
        return;
    }
    if app.source_access.panel.is_some() {
        source_access::mouse(app, event);
        return;
    }
    if matches!(app.surface, Surface::Device(_) | Surface::Source(_))
        && let Some((area, bottom)) = app.detail_viewport
        && area.contains((event.column, event.row).into())
    {
        match event.kind {
            MouseEventKind::ScrollUp => app.scroll = app.scroll.saturating_sub(3),
            MouseEventKind::ScrollDown => app.scroll = app.scroll.saturating_add(3).min(bottom),
            _ => {}
        }
        return;
    }
    if text_view::mouse(app, event) {
        return;
    }
    if !(app.surface == Surface::Chat
        || (matches!(app.surface, Surface::Task(..)) && !app.work_diagnostics))
        || app.review.is_some()
        || app.remote_review.is_some()
        || app.cancel_review.is_some()
        || app.followup.is_some()
    {
        return;
    }
    let Some(area) = app.chat_viewport else {
        return;
    };
    if !area.contains((event.column, event.row).into()) {
        return;
    }
    match event.kind {
        MouseEventKind::ScrollUp => {
            app.follow = false;
            app.scroll = app.scroll.saturating_sub(3);
        }
        MouseEventKind::ScrollDown => {
            app.scroll = app.scroll.saturating_add(3).min(app.chat_bottom);
            app.follow = app.scroll == app.chat_bottom;
        }
        _ => {}
    }
}
fn handle_key(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
    key: KeyEvent,
) -> Result<Flow> {
    let conversation = app.conversation;
    let mut open_worker = None;
    let control = key.modifiers.contains(KeyModifiers::CONTROL);
    if control && key.code == KeyCode::Char('q') {
        return Ok(Flow::Exit);
    }
    live::expired(client, app, tx, jobs);
    if app.source_setup.panel.is_some() {
        if key.code == KeyCode::F(9) {
            if let Err(e) = paste::request(app, tx, jobs) {
                app.status = e.to_string();
            }
        } else if let Err(e) = source_setup::key(client, app, tx, jobs, key) {
            app.status = e.to_string();
        }
        return Ok(Flow::Continue);
    }
    if matches!(app.surface, Surface::Sources | Surface::Source(_))
        && !control
        && app.source_access.panel.is_none()
        && matches!(key.code, KeyCode::Char('n' | 'N' | 'e' | 'E' | 's' | 'S'))
    {
        let id = match app.surface {
            Surface::Source(id) => Some(id),
            _ => app.sources.get(app.selected).map(|s| s.view.source.id),
        };
        let kind = match key.code {
            KeyCode::Char('n' | 'N') => Some(source_setup::Kind::Register),
            KeyCode::Char('e' | 'E') => id.map(source_setup::Kind::Import),
            _ => id.map(source_setup::Kind::Sync),
        };
        if let Some(kind) = kind {
            if let Err(e) = source_setup::open(client, app, tx, jobs, kind) {
                app.status = e.to_string();
            }
        } else {
            app.status = "请先登记并选择一个来源".into();
        }
        return Ok(Flow::Continue);
    }
    if matches!(app.surface, Surface::Sources | Surface::Source(_))
        && !control
        && matches!(key.code, KeyCode::Char('g' | 'G'))
        && app.source_access.panel.is_none()
    {
        if let Err(e) = source_access::open(client, app, tx, jobs) {
            app.status = e.to_string();
        }
        return Ok(Flow::Continue);
    }
    if matches!(app.surface, Surface::Sources | Surface::Source(_)) && source_setup::waiting(app) {
        source_setup::waiting_key(app, key);
        return Ok(Flow::Continue);
    }
    if matches!(app.surface, Surface::Sources | Surface::Source(_))
        && source_access::waiting(app)
        && key.code != KeyCode::Esc
    {
        app.status = "正在重读权限范围；尚未修改授权，输入不会进入聊天草稿".into();
        return Ok(Flow::Continue);
    }
    if !live::fresh(app)
        && key.code == KeyCode::Esc
        && matches!(app.surface, Surface::Sources | Surface::Source(_))
    {
        app.back();
        return Ok(Flow::Continue);
    }
    if app.source_access.panel.is_some() {
        if key.code == KeyCode::F(9) {
            if let Err(e) = paste::request(app, tx, jobs) {
                app.status = e.to_string();
            }
        } else if let Err(e) = source_access::key(client, app, tx, jobs, key) {
            app.status = e.to_string();
        }
        return Ok(Flow::Continue);
    }
    if key.code == KeyCode::F(5)
        && matches!(app.surface, Surface::Task(..))
        && app.review.is_none()
        && app.remote_review.is_none()
        && app.cancel_review.is_none()
        && app.followup.is_none()
        && app.work_text.menu.is_none()
        && live::fresh(app)
    {
        work_history::page(app, key.modifiers.contains(KeyModifiers::SHIFT));
        return Ok(Flow::Continue);
    }
    if let Surface::Device(id) = app.surface {
        match key.code {
            KeyCode::Char('n' | 'N') if !control => {
                let target = devices_view::selected(app, id)
                    .and_then(|d| d.device.registration.workspaces.get(app.selected))
                    .map(|w| w.id);
                if let Some(workspace) = target {
                    if let Err(e) = new_work::open_for(client, app, tx, jobs, id, workspace) {
                        app.status = e.to_string();
                    }
                } else {
                    app.status = "选中电脑或目录不可用；C 刷新，不会改选其他电脑".into();
                }
                return Ok(Flow::Continue);
            }
            KeyCode::Char('c' | 'C') if !control => {
                app.status = "正在检查云端连接并重读设备信息；不代表执行服务在线".into();
                refresh(client, app, tx, jobs);
                return Ok(Flow::Continue);
            }
            KeyCode::Char('i' | 'I') if !control => {
                app.device_page = if app.device_page == devices_view::Page::Startup {
                    devices_view::Page::Overview
                } else {
                    devices_view::Page::Startup
                };
                app.scroll = 0;
                return Ok(Flow::Continue);
            }
            KeyCode::F(7) => {
                app.device_page = if app.device_page == devices_view::Page::Diagnostics {
                    devices_view::Page::Overview
                } else {
                    devices_view::Page::Diagnostics
                };
                app.scroll = 0;
                return Ok(Flow::Continue);
            }
            KeyCode::Char('t' | 'T') if !control => {
                app.navigate(Surface::Tasks(id));
                refresh(client, app, tx, jobs);
                return Ok(Flow::Continue);
            }
            KeyCode::Up | KeyCode::Down => {
                app.scroll = 0;
            }
            _ => {}
        }
    }
    if key.code == KeyCode::F(7)
        && matches!(app.surface, Surface::Task(..))
        && app.review.is_none()
        && app.remote_review.is_none()
        && app.cancel_review.is_none()
        && app.followup.is_none()
        && live::fresh(app)
    {
        app.work_diagnostics = !app.work_diagnostics;
        app.scroll = 0;
        app.follow = false;
        return Ok(Flow::Continue);
    }
    let previous_surface = app.surface;
    if text_view::key(app, key) {
        if app.surface != previous_surface {
            refresh(client, app, tx, jobs);
        }
        return Ok(Flow::Continue);
    }
    if app.live.is_some() && !live::fresh(app) {
        if control && key.code == KeyCode::Char('r') {
            live::resume(client, app, tx, jobs);
        } else if key.code == KeyCode::Enter
            || matches!(key.code, KeyCode::F(4) | KeyCode::F(6) | KeyCode::F(8))
        {
            app.status = "Not synchronized · draft retained; Ctrl-R reconnects.".into();
        } else {
            app.input.key(key);
        }
        return Ok(Flow::Continue);
    }
    if key.code == KeyCode::F(9) {
        if let Err(e) = paste::request(app, tx, jobs) {
            app.status = e.to_string();
        }
        return Ok(Flow::Continue);
    }
    if app.new_work.is_some() {
        if let Err(e) = new_work::key(client, app, tx, jobs, key) {
            app.status = e.to_string();
        }
        return Ok(Flow::Continue);
    }
    if matches!(app.surface, Surface::Sources | Surface::Source(_))
        && control
        && key.code == KeyCode::Char('r')
        && source_setup::pending(app)
    {
        source_setup::recover(client, app, tx, jobs);
        return Ok(Flow::Continue);
    }
    if matches!(app.surface, Surface::Sources | Surface::Source(_))
        && control
        && key.code == KeyCode::Char('r')
        && source_access::pending(app)
    {
        source_access::recover(client, app, tx, jobs);
        return Ok(Flow::Continue);
    }
    if control && key.code == KeyCode::Char('r') && app.new_work_pending.is_some() {
        new_work::recover(client, app, tx, jobs);
        return Ok(Flow::Continue);
    }
    if app.surface == Surface::Work && matches!(key.code, KeyCode::Char('n' | 'N')) && !control {
        if let Err(e) = new_work::open(client, app, tx, jobs) {
            app.status = e.to_string();
        }
        return Ok(Flow::Continue);
    }
    if control && key.code == KeyCode::Char('r') && app.cancel_pending.is_some() {
        cancel::recover(client, app, tx, jobs, false);
        refresh(client, app, tx, jobs);
        return Ok(Flow::Continue);
    }
    if app.cancel_review.is_some() {
        if control && key.code == KeyCode::Char('r') {
            app.cancel_review = None;
            refresh(client, app, tx, jobs);
        } else {
            cancel::key(client, app, tx, jobs, key);
        }
        return Ok(Flow::Continue);
    }
    if control && key.code == KeyCode::Char('r') && app.followup_pending.is_some() {
        followup::recover(client, app, tx, jobs, false);
        if app.remote_pending.is_some() {
            device_review::recover(client, app, tx, jobs, false);
        }
        refresh(client, app, tx, jobs);
        return Ok(Flow::Continue);
    }
    if app.followup.is_some() {
        if let Err(e) = followup::key(client, app, tx, jobs, key) {
            app.status = e.to_string();
        }
        return Ok(Flow::Continue);
    }
    if control && key.code == KeyCode::Char('c') {
        if app.input.text.is_empty() {
            return Ok(Flow::Exit);
        }
        app.input = Composer::default();
        return Ok(Flow::Continue);
    }
    if control && key.code == KeyCode::Char('r') && app.remote_pending.is_some() {
        device_review::recover(client, app, tx, jobs, false);
        refresh(client, app, tx, jobs);
        return Ok(Flow::Continue);
    }
    if app.remote_review.is_some() {
        device_review::key(client, app, tx, jobs, key);
        return Ok(Flow::Continue);
    }
    if key.code == KeyCode::F(6) && app.review.is_none() {
        if let Surface::Task(d, c) = app.surface {
            if let Err(e) = followup::open(client, app, tx, jobs, d, c) {
                app.status = e.to_string();
            }
        } else {
            app.status = "Select a remote Worker task before F6 follow-up".into();
        }
        return Ok(Flow::Continue);
    }
    if app.review.is_some() {
        match key.code {
            KeyCode::Esc => app.review = None,
            KeyCode::PageDown => {
                let r = app.review.as_mut().unwrap();
                r.scroll = r.scroll.saturating_add(r.page);
            }
            KeyCode::PageUp => {
                let r = app.review.as_mut().unwrap();
                r.scroll = r.scroll.saturating_sub(r.page);
            }
            KeyCode::Char('y' | 'd') if control && !app.request_busy => {
                let r = app.review.as_ref().unwrap();
                let valid = r.read
                    && app.turn.as_ref().is_some_and(|t| {
                        t.id == r.turn
                            && t.revision == r.revision
                            && t.actions.iter().any(|a| {
                                a.id == r.action
                                    && a.state == ActionState::Proposed
                                    && dispatch::review_text(t, a).is_ok_and(|text| text == r.text)
                            })
                    });
                if valid {
                    let c = client.clone();
                    let path = format!("v1/actor-turns/{}/actions/{}/decision", r.turn, r.action);
                    let out = live::effect_sender(app, tx, jobs);
                    let approve = key.code == KeyCode::Char('y');
                    app.request_busy = true;
                    jobs.spawn(async move {
                        let _ = out
                            .send(Complete::Mutation(
                                c.post(&path, Some(json!({"approve":approve}))).await,
                            ))
                            .await;
                    });
                } else {
                    app.status = "Read the full current scope before deciding".into();
                }
            }
            _ => {}
        }
    } else {
        match key.code {
            KeyCode::Tab => {
                let s = match app.surface {
                    Surface::Chat => Surface::Work,
                    Surface::Work => Surface::Devices,
                    Surface::Task(..) if app.task_from_work => Surface::Devices,
                    _ => Surface::Chat,
                };
                app.navigate(s);
                refresh(client, app, tx, jobs);
            }
            KeyCode::Esc => match app.surface {
                Surface::Device(_) => {
                    app.navigate(Surface::Devices);
                    refresh(client, app, tx, jobs);
                }
                Surface::Tasks(id) => {
                    app.navigate(Surface::Device(id));
                    refresh(client, app, tx, jobs);
                }
                Surface::Source(_) => {
                    app.navigate(Surface::Sources);
                    refresh(client, app, tx, jobs);
                }
                _ => app.back(),
            },
            KeyCode::F(4) => {
                let result = if matches!(app.surface, Surface::Task(..)) {
                    device_review::open(app)
                } else {
                    app.open_review()
                };
                if let Err(e) = result {
                    app.status = e.to_string();
                }
            }
            KeyCode::F(8) => {
                if let Err(e) = cancel::open(app) {
                    app.status = e.to_string();
                }
            }
            KeyCode::PageUp => {
                app.follow = false;
                let step = if matches!(app.surface, Surface::Source(_) | Surface::Device(_)) {
                    app.detail_viewport
                        .map_or(8, |(area, _)| area.height.max(1))
                } else {
                    8
                };
                app.scroll = app.scroll.saturating_sub(step);
            }
            KeyCode::PageDown => {
                app.follow = false;
                let step = if matches!(app.surface, Surface::Source(_) | Surface::Device(_)) {
                    app.detail_viewport
                        .map_or(8, |(area, _)| area.height.max(1))
                } else {
                    8
                };
                app.scroll = app.scroll.saturating_add(step);
            }
            KeyCode::End if control => {
                app.follow = true;
            }
            KeyCode::Home if control => {
                app.follow = false;
                app.scroll = 0;
            }
            KeyCode::Char('r') if control => refresh(client, app, tx, jobs),
            KeyCode::Up if app.surface != Surface::Chat => {
                app.selected = app.selected.saturating_sub(1)
            }
            KeyCode::Down if app.surface != Surface::Chat => {
                app.selected = app.selected.saturating_add(1)
            }
            KeyCode::Enter if app.surface != Surface::Chat => match app.surface {
                Surface::Work => {
                    open_worker = app.sessions.get(app.selected).map(|s| s.id);
                    let remote = app
                        .remote_heads()
                        .nth(app.selected.saturating_sub(app.sessions.len()))
                        .map(|r| (r.command.device_id, r.command.id));
                    if open_worker.is_none()
                        && let Some((d, c)) = remote
                    {
                        app.navigate(Surface::Task(d, c));
                        refresh(client, app, tx, jobs);
                    }
                }
                Surface::Devices => {
                    if let Some(d) = app.devices.get(app.selected) {
                        app.navigate(Surface::Device(d.device.registration.id));
                        refresh(client, app, tx, jobs);
                    }
                }
                Surface::Sources => {
                    if let Some(source) = app.sources.get(app.selected) {
                        app.navigate(Surface::Source(source.view.source.id));
                        refresh(client, app, tx, jobs);
                    }
                }
                Surface::Tasks(id) => {
                    if let Some(t) = app.tasks.get(app.selected) {
                        app.navigate(Surface::Task(id, t.id));
                        refresh(client, app, tx, jobs);
                    }
                }
                Surface::Task(d, c) => {
                    if let Some(next) = app
                        .task
                        .as_ref()
                        .filter(|task| task.id == c)
                        .and_then(|task| task.continued_by)
                    {
                        app.navigate(Surface::Task(d, next));
                        refresh(client, app, tx, jobs);
                    }
                }
                _ => {}
            },
            KeyCode::Enter
                if key.modifiers.intersects(
                    KeyModifiers::SHIFT | KeyModifiers::ALT | KeyModifiers::CONTROL,
                ) =>
            {
                app.input.insert("\n")
            }
            KeyCode::Enter => {
                let text = app.input.text.clone();
                let command = text.trim();
                let mut consumed = true;
                match command {
                                "/exit"=>return Ok(Flow::Exit),
                                "/work"|"/dashboard"=>{app.navigate(Surface::Work);refresh(client,app,tx,jobs);},
                                "/devices"=>{app.navigate(Surface::Devices);refresh(client,app,tx,jobs);},
                                "/sources"=>{app.navigate(Surface::Sources);refresh(client,app,tx,jobs);},
                                "/chat"=>app.back(),
                                "/copy"=>text_view::open_copy(app),
                                "/read"=>{text_view::key(app,KeyEvent::new(KeyCode::F(3),KeyModifiers::NONE));},
                                "/help"=>app.append("\nDesk> /chat /work /devices /refresh /review /retry /stop /seen /copy /read /exit\nF2 copy menu; Ctrl-O latest reply; drag + Ctrl-C selected text; F3 reader; Ctrl-Home top.\nF4 permission; F6 idle Worker follow-up; F8 inspect task cancellation.\n/continue <device UUID> <parent UUID> opens Worker input.\n/decision-retry /followup-retry /cancel-retry explicitly repeat only their pending exact request.\nTab switches surfaces; Esc returns to Lain. Worker work stays in the Kernel.\nPasted text retains line breaks. Ctrl-J inserts a new line. Modified Enter also works when forwarded by your terminal; Windows Terminal uses Alt-Enter for fullscreen.\n/seen marks the displayed work updates read; it never controls a Worker.\n"),
                                "/refresh"=>refresh(client,app,tx,jobs),
                                "/review"=>if let Err(e)=app.open_review(){app.status=e.to_string();},
                                "/decision-retry"=>device_review::recover(client,app,tx,jobs,true),
                                "/followup-retry"=>followup::recover(client,app,tx,jobs,true),
                                "/cancel-retry"=>cancel::recover(client,app,tx,jobs,true),
                                text if text.starts_with("/continue ")=>{
                                    let parts=text.split_whitespace().collect::<Vec<_>>();
                                    if parts.len()==3 {
                                        match (parts[1].parse(),parts[2].parse()){
                                            (Ok(d),Ok(p))=>if let Err(e)=followup::open(client,app,tx,jobs,d,p){app.status=e.to_string();},
                                            _=>app.status="Use /continue <device UUID> <parent command UUID>".into(),
                                        }
                                    }else{app.status="Use /continue <device UUID> <parent command UUID>".into();}
                                },
                                "/retry"=>if let Some(p)=app.pending.as_mut().filter(|p|p.uncertain){p.uncertain=false;let p=p.clone();submit(client.clone(),app.conversation,p,live::epoch(app),tx.clone(),jobs);app.status="Retrying identical request ID and text".into();},
                                "/stop"=>if let Some(t)=app.turn.as_ref().filter(|t|matches!(t.state,ActorTurnState::Running|ActorTurnState::PendingConfirmation)) && !app.request_busy {let c=client.clone();let id=t.id;let out=live::effect_sender(app,tx,jobs);app.request_busy=true;jobs.spawn(async move {let _=out.send(Complete::Mutation(c.post(&format!("v1/actor-turns/{id}/cancel"),None).await)).await;});},
                                "/seen"=>{let ids=app.notices.iter().copied().collect::<Vec<_>>();let c=client.clone();let out=tx.clone();jobs.spawn(async move {let mut acked=vec![];for id in ids {if c.post::<Value>(&format!("v1/conversations/{conversation}/notices/{id}/ack"),None).await.is_ok(){acked.push(id);}}let _=out.send(Complete::Ack(acked)).await;});},
                                _=>{consumed=false;if let Some(p)=app.prepare(){submit(client.clone(),conversation,p,live::epoch(app),tx.clone(),jobs);}},
                            }
                if consumed {
                    app.input = Composer::default();
                }
            }
            _ if app.surface == Surface::Chat => app.input.key(key),
            _ => {}
        }
    }
    Ok(open_worker.map_or(Flow::Continue, Flow::Worker))
}
fn handle_complete(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
    result: Complete,
) -> Result<()> {
    live::expired(client, app, tx, jobs);
    match result {
        Complete::Live(g, update) => return live::complete(client, app, tx, jobs, g, update),
        Complete::Read(g, result) => {
            if live::epoch(app) == Some(g) && live::fresh(app) {
                if let Some(status) = live::failure_status(&result) {
                    let failure = crate::api::HttpFailure(status);
                    return live::complete(
                        client,
                        app,
                        tx,
                        jobs,
                        g,
                        live::Update::Changed(Err(failure.into())),
                    );
                }
                return handle_complete(client, app, tx, jobs, *result);
            }
            return Ok(());
        }
        Complete::Effect(g, result) => {
            let stale = live::epoch(app) != Some(g) || !live::fresh(app);
            let failure = (!stale).then(|| live::failure_status(&result)).flatten();
            if stale && let Complete::Submit(id, ref response) = *result {
                if app.pending.as_ref().is_some_and(|p| p.id == id) {
                    match response {
                        Ok(t) => {
                            t.validate().map_err(anyhow::Error::msg)?;
                            ensure!(
                                t.request_id == id && t.conversation_id == app.conversation,
                                "submission receipt identity changed"
                            );
                            app.pending = None;
                        }
                        Err(_) => app.submission_failed(),
                    }
                }
                live::refresh(client, app, tx, jobs);
                return Ok(());
            }
            let outcome = handle_complete(client, app, tx, jobs, *result);
            if let Some(status) = failure {
                let current = live::epoch(app).context("live state missing")?;
                live::complete(
                    client,
                    app,
                    tx,
                    jobs,
                    current,
                    live::Update::Changed(Err(crate::api::HttpFailure(status).into())),
                )?;
            } else if stale {
                live::refresh(client, app, tx, jobs);
            }
            return outcome;
        }
        _ => {}
    }
    let conversation = app.conversation;
    match result {
        Complete::SourceSetupOpened(g, kind, result) => {
            if g == app.generation
                && matches!(app.surface, Surface::Sources | Surface::Source(_))
                && source_setup::accepts(app, kind)
                && let Err(e) = source_setup::opened(app, kind, *result)
            {
                app.status = format!("来源配置读取失败：{e}");
            }
        }
        Complete::SourceSetupSent(result) => {
            let (pending, result) = *result;
            if let Some(id) = source_setup::complete(app, pending, result) {
                app.navigate(Surface::Source(id));
                refresh(client, app, tx, jobs);
            }
        }
        Complete::SourceAccessOpened(g, result) => {
            if g == app.generation
                && matches!(app.surface, Surface::Sources | Surface::Source(_))
                && let Err(e) = source_access::opened(app, *result)
            {
                app.status = format!("来源权限读取失败：{e}");
            }
        }
        Complete::SourceAccessSent(result) => {
            let (pending, result) = *result;
            source_access::complete(app, pending, result);
        }
        Complete::Paste(p, result) => paste::complete(app, p, result),
        Complete::NewWorkTargets(g, result) => {
            if g == app.generation && app.surface == Surface::Work && app.new_work_loading {
                app.new_work_loading = false;
                match result {
                    Ok(devices) => new_work::opened(app, devices),
                    Err(e) => app.status = format!("无法读取执行位置：{e}；N 重新打开，未创建任务"),
                }
            }
        }
        Complete::NewWorkSent(result) => {
            let (p, result) = *result;
            new_work::complete(app, p, result)?;
            if matches!(app.surface, Surface::Task(..)) {
                refresh(client, app, tx, jobs);
            }
        }
        Complete::WorkerStatus(update) => {
            app.worker_status.apply(*update)?;
        }
        Complete::ChatStatus(result) => chat_status::apply(app, result),
        Complete::RemoteWork(Ok(mut page)) => {
            page.validate(conversation).map_err(anyhow::Error::msg)?;
            let baseline = !app.activity.warm || app.activity.reconnecting;
            let watermark = app.activity.watermark;
            app.remote_connected = true;
            let selected = if app.surface == Surface::Work && app.selected >= app.sessions.len() {
                app.remote_heads()
                    .nth(app.selected - app.sessions.len())
                    .map(|r| {
                        r.command
                            .continuation
                            .as_ref()
                            .map_or(r.command.id, |f| f.root_command_id)
                    })
            } else {
                None
            };
            for r in page.work.iter_mut().rev() {
                if let Some(old) = app.remote_work.iter().find(|old| {
                    old.command.id == r.command.id && old.command.revision > r.command.revision
                }) {
                    *r = old.clone();
                }
                let c = &r.command;
                let signature = serde_json::to_string(
                    &json!({"state":c.state,"cancel":c.cancel_requested,"execution":c.execution,"interruption":c.interruption}),
                )?;
                let key = (c.device_id, c.id);
                let old = app.remote_versions.get(&key);
                let prior_revision = old
                    .map(|(r, _)| *r)
                    .or_else(|| app.activity.seen.get(&key).copied());
                if old.is_none() && prior_revision.is_some_and(|r| r > c.revision) {
                    app.status = "任务回执落后于已知版本，等待更新；未显示旧正文".into();
                    return Ok(());
                }
                let known = prior_revision.is_some()
                    || app
                        .remote_work
                        .iter()
                        .any(|r| r.command.device_id == c.device_id && r.command.id == c.id);
                let fresh = prior_revision.is_none_or(|rev| c.revision > rev);
                let meaningful = old.is_none_or(|(_, s)| *s != signature);
                let restore = app.activity.visible(activity::task_id(c));
                if fresh || (old.is_none() && restore) {
                    app.remote_versions.insert(key, (c.revision, signature));
                    let has_card = app.text_view.doc.version(activity::task_id(c)).is_some();
                    let waiting = c
                        .execution
                        .as_ref()
                        .is_some_and(|e| !e.pending_approvals.is_empty());
                    if meaningful
                        && (waiting
                            || restore
                            || has_card
                            || (!baseline
                                && (known || c.created_at_ms > watermark)
                                && (c.state.terminal() || c.cancel_requested)))
                    {
                        activity::task(&mut app.text_view.doc, c);
                    }
                }
                app.activity.seen.insert(key, c.revision);
                app.activity.watermark = app.activity.watermark.max(c.created_at_ms);
                if app.surface == Surface::Task(c.device_id, c.id)
                    && app.task.as_ref().is_none_or(|t| t.revision < c.revision)
                {
                    app.task = Some(c.clone());
                    app.task_decisions = None;
                }
                cancel::observed(app, c);
            }
            app.remote_work = page.work;
            app.activity.warm = true;
            app.activity.reconnecting = false;
            activity::restore_positions(app);
            if app.activity.seen.len() > 512 {
                let visible = app.activity.cards();
                let current = app
                    .remote_work
                    .iter()
                    .map(|r| (r.command.device_id, r.command.id))
                    .collect::<BTreeSet<_>>();
                app.activity.seen.retain(|(d, c), _| {
                    current.contains(&(*d, *c))
                        || visible.contains(&activity::card_id(*d, *c))
                        || app
                            .text_view
                            .doc
                            .version(activity::card_id(*d, *c))
                            .is_some()
                });
            }
            if app.remote_versions.len() > 512 {
                let current = app
                    .remote_work
                    .iter()
                    .map(|r| (r.command.device_id, r.command.id))
                    .collect::<BTreeSet<_>>();
                app.remote_versions.retain(|(d, c), _| {
                    current.contains(&(*d, *c))
                        || app
                            .text_view
                            .doc
                            .version(activity::card_id(*d, *c))
                            .is_some()
                });
            }
            device_review::invalidate(app);
            let selection = selected.and_then(|id| {
                app.remote_heads().position(|r| {
                    r.command
                        .continuation
                        .as_ref()
                        .map_or(r.command.id, |f| f.root_command_id)
                        == id
                })
            });
            if let Some(index) = selection {
                app.selected = app.sessions.len() + index;
            }
            while app.remote_versions.len() > 128 {
                let stale = app
                    .remote_versions
                    .keys()
                    .copied()
                    .find(|(_, cid)| !app.remote_work.iter().any(|r| r.command.id == *cid));
                if let Some(stale) = stale {
                    app.remote_versions.remove(&stale);
                } else {
                    break;
                }
            }
        }
        Complete::RemoteWork(Err(_)) => {
            app.remote_connected = false;
            app.activity.reconnecting = true;
        }
        Complete::RemoteDecision(p, result) => device_review::complete(app, p, result)?,
        Complete::RemoteCancelled(result) => {
            let (pending, result) = *result;
            cancel::complete(app, pending, result)?;
        }
        Complete::FollowupOpened(g, d, p, result) => {
            if g == app.generation && app.surface == Surface::Task(d, p) {
                match *result {
                    Ok((r, s)) => {
                        if let Err(e) = followup::opened(app, r, s) {
                            app.status = e.to_string();
                        }
                    }
                    Err(e) => app.status = format!("Follow-up target unavailable: {e}"),
                }
            }
        }
        Complete::FollowupSent(payload) => {
            let (p, result) = *payload;
            followup::complete(app, p, result)?;
        }
        Complete::Submit(id, result) => {
            if app.pending.as_ref().is_some_and(|p| p.id == id) {
                match result {
                    Ok(turn) => {
                        let turn_id = turn.id;
                        let running = turn.state == ActorTurnState::Running;
                        app.snapshot(turn)?;
                        app.status =
                            "Received by Kernel; completed output follows its records".into();
                        if running {
                            observe(client.clone(), turn_id, app, tx.clone(), jobs);
                        }
                    }
                    Err(_) => app.submission_failed(),
                }
            }
        }
        Complete::Actor(update) => {
            app.connected = true;
            match update {
                ActorUpdate::Snapshot(t) => {
                    app.snapshot(*t)?;
                    if app.source_invalidated {
                        refresh(client, app, tx, jobs);
                    }
                }
                ActorUpdate::Preview(p) => {
                    if let Some(t) = &app.turn
                        && p.turn_id == t.id
                    {
                        let preview = p.clone();
                        app.renderer.preview(p, t)?;
                        let text = app.renderer.drain_text();
                        app.append_legacy(&text);
                        if app.renderer.preview_checkpoint()
                            == Some((preview.preview_id, preview.message_id, preview.sequence))
                        {
                            app.text_view.preview(&preview);
                        } else if app.renderer.preview_checkpoint().is_none() {
                            app.text_view.clear_preview();
                        }
                    }
                }
            }
        }
        Complete::StreamEnded(id, result) => {
            app.observing.remove(&id);
            if app.live.is_some() && result.is_err() {
                live::refresh(client, app, tx, jobs);
                return Ok(());
            }
            if app
                .turn
                .as_ref()
                .is_some_and(|t| t.id == id && t.state == ActorTurnState::Running)
                && result.is_err()
            {
                app.connected = false;
                app.renderer
                    .interrupted("connection lost; committed reply not confirmed")?;
                let text = app.renderer.drain_text();
                app.append(&text);
                app.status="Connection lost; /refresh reconstructs the same conversation. No request replay.".into();
            }
        }
        Complete::History(g, result) if g == app.generation && app.surface == Surface::Chat => {
            app.refreshing = false;
            match result {
                Ok(page) => {
                    if !app.bootstrap(page)? {
                        return Ok(());
                    }
                    if let Some(t) = &app.turn
                        && t.state == ActorTurnState::Running
                    {
                        observe(client.clone(), t.id, app, tx.clone(), jobs);
                    }
                    app.connected = true;
                }
                Err(e) => {
                    app.connected = false;
                    app.status = format!("History unavailable: {e}");
                }
            }
        }
        Complete::Sessions(g, result) if g == app.generation && app.surface == Surface::Work => {
            app.refreshing = false;
            match result {
                Ok(rows) => {
                    app.connected = true;
                    app.sessions = rows;
                }
                Err(e) => {
                    app.connected = false;
                    app.status = format!("Work list unavailable: {e}");
                }
            }
        }
        Complete::Sources(g, result) => {
            if g == app.generation && matches!(app.surface, Surface::Sources | Surface::Source(_)) {
                app.refreshing = false;
                match result {
                    Ok(rows) => {
                        if app.surface == Surface::Sources
                            && let Some(id) =
                                app.sources.get(app.selected).map(|s| s.view.source.id)
                        {
                            app.selected = rows
                                .iter()
                                .position(|s| s.view.source.id == id)
                                .unwrap_or(rows.len());
                        }
                        app.sources = rows;
                        app.source_read_error = false;
                        app.status = "来源状态已更新；没有启动采集或调用模型".into();
                    }
                    Err(e) => {
                        app.source_read_error = true;
                        app.status = format!("来源状态读取失败：{e}；未启用采集");
                    }
                }
            }
        }
        Complete::Devices(g, result)
            if g == app.generation
                && matches!(
                    app.surface,
                    Surface::Devices | Surface::Device(_) | Surface::Work
                ) =>
        {
            app.refreshing = false;
            match result {
                Ok(rows) => {
                    app.connected = true;
                    devices_view::retain_selection(app, &rows);
                    app.devices = rows;
                    app.device_read_at = Some(devices_view::now());
                    app.device_read_error = false;
                    if matches!(app.surface, Surface::Devices | Surface::Device(_)) {
                        app.status = "云端读取成功；最近回报不代表执行服务仍在运行".into();
                    }
                }
                Err(e) => {
                    app.connected = false;
                    app.device_read_error = true;
                    app.status = format!("Device list unavailable: {e}");
                }
            }
        }
        Complete::Tasks(g, id, result) => {
            if g == app.generation && app.surface == Surface::Tasks(id) {
                app.refreshing = false;
                match result {
                    Ok(rows) => {
                        app.connected = true;
                        app.tasks = rows;
                    }
                    Err(e) => {
                        app.connected = false;
                        app.status = format!("Device tasks unavailable: {e}");
                    }
                }
            }
        }
        Complete::Task(g, id, cid, result) => {
            if g == app.generation && app.surface == Surface::Task(id, cid) {
                app.refreshing = false;
                match result {
                    Ok(TaskView {
                        command: c,
                        decisions,
                        history,
                        device,
                    }) if c.id == cid && c.device_id == id => {
                        app.connected = true;
                        app.task_history = history;
                        if let Some(device) = device {
                            app.devices.retain(|d| d.device.registration.id != id);
                            app.devices.push(*device);
                        }
                        if app
                            .task
                            .as_ref()
                            .is_none_or(|old| c.revision >= old.revision)
                        {
                            cancel::observed(app, &c);
                            app.task = Some(c);
                            app.task_decisions = Some(decisions);
                            device_review::invalidate(app);
                            cancel::invalidate(app);
                        }
                    }
                    Ok(_) => app.status = "Device task identity mismatch".into(),
                    Err(e) => {
                        app.connected = false;
                        app.status = format!("Task unavailable: {e}");
                    }
                }
            }
        }
        Complete::Mutation(result) => {
            app.request_busy = false;
            app.review = None;
            match result {
                Ok(t) => {
                    let id = t.id;
                    let running = t.state == ActorTurnState::Running;
                    app.snapshot(t)?;
                    if running {
                        observe(client.clone(), id, app, tx.clone(), jobs);
                    }
                }
                Err(_) => {
                    app.status =
                        "Decision outcome uncertain; inspect /refresh before any retry".into();
                    refresh(client, app, tx, jobs);
                }
            }
        }
        Complete::Notice(NoticeUpdate::Notice(n)) => {
            if n.conversation_id == conversation
                && !n.acknowledged
                && !activity::quiet_notice(app, &n)
                && (app.notices.contains(&n.id) || app.notices.len() < 100)
                && app.notices.insert(n.id)
            {
                app.renderer.notice(&n)?;
                let text = app.renderer.drain_text();
                activity::notice(app, &n, &text);
            }
        }
        Complete::Notice(NoticeUpdate::Offline) | Complete::NoticeEnded(_) => {
            activity::notice_reconnect(app);
            if app.live.is_some() {
                live::refresh(client, app, tx, jobs);
            } else {
                live::retire(app);
            }
            if app.pending.is_none() {
                app.status = "Kernel updates unavailable; Ctrl-R refresh. Draft retained.".into();
            }
        }
        Complete::Ack(ids) => {
            for id in ids {
                app.notices.remove(&id);
            }
            app.status = "Displayed updates marked read".into();
        }
        Complete::History(..) | Complete::Sessions(..) | Complete::Devices(..) => {}
        Complete::Live(..) | Complete::Read(..) | Complete::Effect(..) => {
            unreachable!("handled lifecycle envelope")
        }
    }
    Ok(())
}
