use crate::api::{Approval, Client, Decision, Event, HistoryPage, Session, Snapshot, Turn, Update};
use anyhow::{Result, ensure};
use crossterm::{
    cursor::Show,
    event::{
        DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event as TerminalEvent, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures_util::StreamExt;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{self, IsTerminal},
    sync::atomic::{AtomicBool, Ordering},
};
use tokio::{sync::mpsc, task::JoinSet};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
use uuid::Uuid;

/// Worker text is data, never terminal instructions (including C1 OSC/CSI).
pub fn safe_text(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            if (c.is_control() && !matches!(c, '\n' | '\t'))
                || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
            {
                format!("\\u{{{:x}}}", c as u32).chars().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}
pub fn event_text(event: &Event) -> String {
    let field = |name: &str| event.data.get(name).and_then(Value::as_str).unwrap_or("");
    let detail = match event.kind.as_str() {
        "assistant_message_completed" => field("text").to_owned(),
        "command_started" => field("command").to_owned(),
        "command_completed" => format!(
            "{}\nexit {}\n{}",
            field("command"),
            event.data.get("exit_code").unwrap_or(&Value::Null),
            field("output")
        ),
        "file_changed" => event
            .data
            .get("paths")
            .and_then(Value::as_array)
            .map(|paths| {
                paths
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default(),
        "turn_started" | "turn_completed" | "turn_state_changed" => event
            .data
            .get("turn")
            .map(|turn| {
                format!(
                    "{} {}{}",
                    turn["id"].as_str().unwrap_or(""),
                    turn["state"].as_str().unwrap_or(""),
                    turn["failure"]["message"]
                        .as_str()
                        .map(|s| format!("\n{s}"))
                        .unwrap_or_default()
                )
            })
            .unwrap_or_default(),
        "approval_requested" | "approval_resolved" | "approval_state_changed" => event
            .data
            .get("approval")
            .map(|a| {
                format!(
                    "{} {}\n{}",
                    a["kind"].as_str().unwrap_or(""),
                    a["status"].as_str().unwrap_or(""),
                    a["description"].as_str().unwrap_or("")
                )
            })
            .unwrap_or_default(),
        "session_state_changed" | "runtime_attachment_changed" => {
            format!("{} -> {}", field("from"), field("to"))
        }
        "backend_disconnected" | "error_observed" => event
            .data
            .get("failure")
            .and_then(|f| f["message"].as_str())
            .unwrap_or("")
            .to_owned(),
        "tool_activity" => format!(
            "{} {}",
            field("name"),
            if event.data.get("completed").and_then(Value::as_bool) == Some(true) {
                "completed"
            } else {
                "started"
            }
        ),
        "cancellation_changed" => field("state").to_owned(),
        _ => String::new(),
    };
    let seconds = event.timestamp_ms / 1000;
    safe_text(&format!(
        "{:02}:{:02}:{:02} UTC #{} {}{}\n{}",
        seconds / 3600 % 24,
        seconds / 60 % 60,
        seconds % 60,
        event.history_order.unwrap_or(0),
        event.kind,
        if event.truncated { " [truncated]" } else { "" },
        detail
    ))
}

static TERMINAL_ACTIVE: AtomicBool = AtomicBool::new(false);
static MOUSE_ACTIVE: AtomicBool = AtomicBool::new(false);
pub(crate) struct TerminalGuard;
fn restore_commands(writer: &mut impl io::Write) -> io::Result<()> {
    execute!(writer, DisableBracketedPaste, LeaveAlternateScreen, Show)
}
fn restore() {
    if TERMINAL_ACTIVE.swap(false, Ordering::SeqCst) {
        // Win32 DisableMouseCapture restores the modes captured after raw mode
        // was enabled. Restore mouse first, then the original cooked input mode.
        if MOUSE_ACTIVE.swap(false, Ordering::SeqCst) {
            let _ = execute!(io::stdout(), DisableMouseCapture);
        }
        let _ = disable_raw_mode();
        let _ = restore_commands(&mut io::stdout());
    }
}
impl TerminalGuard {
    pub(crate) fn enter_mouse() -> Result<Self> {
        let guard = Self::enter()?;
        execute!(io::stdout(), EnableMouseCapture)?;
        MOUSE_ACTIVE.store(true, Ordering::SeqCst);
        Ok(guard)
    }
    pub(crate) fn enter() -> Result<Self> {
        ensure!(
            io::stdin().is_terminal() && io::stdout().is_terminal(),
            "TUI requires a terminal; use work list/show/history --json for pipes"
        );
        static HOOK: std::sync::Once = std::sync::Once::new();
        HOOK.call_once(|| {
            let old_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                restore();
                old_hook(info);
            }));
        });
        TERMINAL_ACTIVE.store(true, Ordering::SeqCst);
        let guard = Self;
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, EnableBracketedPaste)?;
        Ok(guard)
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore();
    }
}

#[derive(Default)]
struct Input {
    text: String,
    cursor: usize,
}
impl Input {
    fn insert(&mut self, value: &str) {
        let value = safe_text(value).replace(['\r', '\n', '\t'], " ");
        if self.text.len() + value.len() <= 32 * 1024 {
            self.text.insert_str(self.cursor, &value);
            self.cursor += value.len();
        }
    }
    fn key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Left => {
                self.cursor = self.text[..self.cursor]
                    .char_indices()
                    .last()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
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
                self.text.clear();
                self.cursor = 0;
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

#[derive(Default)]
struct View {
    sessions: BTreeMap<Uuid, Session>,
    turns: BTreeMap<Uuid, Turn>,
    approvals: BTreeMap<Uuid, Approval>,
    selected: Option<Uuid>,
    table: TableState,
    timeline: BTreeMap<u64, Event>,
    snapshot_cursor: u64,
    connected: bool,
    message: String,
    progress: String,
    input: Input,
    scroll: usize,
    follow: bool,
    approval_index: usize,
    busy: bool,
    older: bool,
    history_loading: bool,
    history_generation: u64,
    projected_orders: BTreeMap<Uuid, u64>,
    submitted_prompt: Option<(Uuid, String)>,
    approval_full: bool,
    approval_scroll: usize,
    timeline_revision: u64,
    cached_revision: u64,
    cached_width: u16,
    cached_lines: Vec<String>,
}
impl View {
    fn snapshot(&mut self, snapshot: Snapshot) {
        self.sessions = snapshot.sessions.into_iter().map(|s| (s.id, s)).collect();
        self.turns = snapshot.turns.into_iter().map(|t| (t.id, t)).collect();
        self.approvals = snapshot.approvals.into_iter().map(|a| (a.id, a)).collect();
        self.snapshot_cursor = snapshot.history_cursor;
        self.projected_orders.clear();
        self.history_loading = false;
        self.history_generation += 1;
        self.connected = true;
        self.message = "Connected".into();
        self.table.select(Some(
            self.table
                .selected()
                .unwrap_or(0)
                .min(self.sessions.len().saturating_sub(1)),
        ));
    }
    fn merge(&mut self, events: impl IntoIterator<Item = Event>) {
        for event in events {
            if Some(event.work_session_id) != self.selected {
                continue;
            }
            if let Some(order) = event.history_order
                && !self.timeline.values().any(|e| e.id == event.id)
            {
                self.timeline.insert(order, event);
                self.timeline_revision += 1;
            }
        }
        // ponytail: recent window is capped at 1000 records; CLI pagination exports older history.
        while self.timeline.len() > 1000 {
            self.timeline.pop_first();
        }
    }
    fn live(&mut self, event: Event) -> Result<()> {
        if event.history_order.is_none() {
            ensure!(
                event.kind == "output_progress",
                "durable event has no history order"
            );
            if Some(event.work_session_id) == self.selected {
                self.progress = format!(
                    "{}: {} characters",
                    event
                        .data
                        .get("category")
                        .and_then(Value::as_str)
                        .unwrap_or("output"),
                    event.data.get("characters").unwrap_or(&Value::Null)
                );
            }
            return Ok(());
        }
        if event.history_order.is_some_and(|order| {
            order
                > self.snapshot_cursor.max(
                    self.projected_orders
                        .get(&event.work_session_id)
                        .copied()
                        .unwrap_or(0),
                )
        }) {
            match event.kind.as_str() {
                "session_created" => {
                    let session: Session = serde_json::from_value(event.field("session").clone())?;
                    self.sessions.insert(session.id, session);
                }
                "session_state_changed" => {
                    if let Some(s) = self.sessions.get_mut(&event.work_session_id) {
                        s.state = event.field("to").as_str().unwrap_or("unknown").into();
                    }
                }
                "runtime_attachment_changed" => {
                    if let Some(s) = self.sessions.get_mut(&event.work_session_id) {
                        s.runtime = event.field("to").as_str().unwrap_or("lost").into();
                    }
                }
                "turn_started" | "turn_completed" | "turn_state_changed" => {
                    let turn: Turn = serde_json::from_value(event.field("turn").clone())?;
                    self.turns.insert(turn.id, turn);
                }
                "approval_requested" | "approval_resolved" | "approval_state_changed" => {
                    let approval: Approval =
                        serde_json::from_value(event.field("approval").clone())?;
                    self.approvals.insert(approval.id, approval);
                }
                "approval_decision_submitted" => {
                    if let Some(a) = event
                        .field("approval_id")
                        .as_str()
                        .and_then(|id| id.parse().ok())
                        .and_then(|id| self.approvals.get_mut(&id))
                    {
                        a.status = "responding".into();
                    }
                }
                "cancellation_changed" => {
                    if let Some(turn) = event.turn_id.and_then(|id| self.turns.get_mut(&id)) {
                        turn.cancellation =
                            event.field("state").as_str().unwrap_or("unknown").into();
                    }
                }
                _ => {}
            }
            self.projected_orders
                .insert(event.work_session_id, event.history_order.unwrap());
        }
        self.merge([event]);
        Ok(())
    }
    fn pending(&self) -> Vec<&Approval> {
        self.approvals
            .values()
            .filter(|a| Some(a.work_session_id) == self.selected && a.status == "pending")
            .collect()
    }
    fn open(&mut self, id: Uuid) {
        self.history_loading = false;
        self.history_generation += 1;
        self.selected = Some(id);
        self.timeline.clear();
        self.timeline_revision += 1;
        self.scroll = 0;
        self.follow = true;
        self.input = Input::default();
        self.approval_index = 0;
        self.older = false;
        self.approval_full = false;
        self.approval_scroll = 0;
    }
}

enum Completion {
    History(Uuid, u64, Result<HistoryPage>, bool),
    Action(Result<Value>),
}
fn load_history(
    view: &mut View,
    client: &Client,
    jobs: &mut JoinSet<()>,
    tx: &mpsc::Sender<Completion>,
    older: bool,
) {
    let Some(id) = view.selected else {
        return;
    };
    if view.history_loading {
        return;
    }
    view.history_loading = true;
    view.history_generation += 1;
    let generation = view.history_generation;
    let client = client.clone();
    let tx = tx.clone();
    let last = view.timeline.last_key_value().map(|(order, _)| *order);
    let cursor = if older {
        view.timeline
            .first_key_value()
            .map(|(order, _)| order.saturating_sub(1))
            .unwrap_or(view.snapshot_cursor)
    } else {
        view.snapshot_cursor
    };
    jobs.spawn(async move {
        if !older && let Some(mut after) = last {
            loop {
                let page = client.history(id, after, Some(cursor), 200).await;
                let more = page.as_ref().is_ok_and(|p| p.has_more);
                if let Ok(p) = &page {
                    after = p.next_cursor;
                }
                if tx
                    .send(Completion::History(id, generation, page, more))
                    .await
                    .is_err()
                    || !more
                {
                    break;
                }
            }
        } else {
            let page = client.recent(id, cursor).await;
            let _ = tx
                .send(Completion::History(id, generation, page, false))
                .await;
        }
    });
}
fn action(
    view: &mut View,
    jobs: &mut JoinSet<()>,
    tx: &mpsc::Sender<Completion>,
    task: impl std::future::Future<Output = Result<Value>> + Send + 'static,
) {
    view.busy = true;
    view.message = "Request in progress".into();
    let tx = tx.clone();
    jobs.spawn(async move {
        let _ = tx.send(Completion::Action(task.await)).await;
    });
}

pub async fn run(client: Client, selected: Option<Uuid>) -> Result<()> {
    // Inspect through HTTP first: observing an idle session never resumes its worker.
    if let Some(id) = selected {
        let _: Session = client.get(&format!("v1/work-sessions/{id}")).await?;
    }
    let _guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    // Re-entering Windows' alternate buffer can retain the previous surface.
    terminal.clear()?;
    let mut keys = EventStream::new();
    let (updates_tx, mut updates) = mpsc::channel(64);
    let (completed_tx, mut completed) = mpsc::channel(64);
    let mut jobs = JoinSet::new();
    jobs.spawn(client.clone().observe(updates_tx));
    let mut view = View::default();
    if let Some(id) = selected {
        view.open(id);
    }
    loop {
        terminal.draw(|frame| draw(frame, &mut view))?;
        tokio::select! {
            _=tokio::signal::ctrl_c()=>break,
            key=keys.next()=>match key {
                Some(Ok(TerminalEvent::Key(key))) if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat => {
                    if key.code == KeyCode::Char('q') && (key.modifiers.contains(KeyModifiers::CONTROL) || view.selected.is_none()) || key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) { break; }
                    if view.selected.is_none() {
                        let current = view.table.selected().unwrap_or(0);
                        match key.code {
                            KeyCode::Down => view.table.select(Some((current + 1).min(view.sessions.len().saturating_sub(1)))),
                            KeyCode::Up => view.table.select(Some(current.saturating_sub(1))),
                            KeyCode::Enter => if let Some(id) = view.sessions.keys().nth(current).copied() { view.open(id); load_history(&mut view, &client, &mut jobs, &completed_tx, false); },
                            _=>{}
                        }
                    } else {
                        let id = view.selected.unwrap();
                        match key.code {
                            KeyCode::Esc if view.approval_full => view.approval_full = false,
                            KeyCode::Esc => { view.selected = None; view.timeline.clear(); }
                            KeyCode::F(2) => { view.approval_full = !view.approval_full; view.approval_scroll = 0; }
                            KeyCode::PageUp if view.approval_full => view.approval_scroll = view.approval_scroll.saturating_sub(10),
                            KeyCode::PageDown if view.approval_full => view.approval_scroll = view.approval_scroll.saturating_add(10),
                            KeyCode::PageUp => { view.follow = false; view.scroll = view.scroll.saturating_sub(10); }
                            KeyCode::PageDown => { view.follow = false; view.scroll = view.scroll.saturating_add(10); }
                            KeyCode::End if key.modifiers.contains(KeyModifiers::CONTROL) => view.follow = true,
                            KeyCode::Tab => { view.approval_index = (view.approval_index + 1) % view.pending().len().max(1); view.approval_scroll = 0; }
                            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                if view.timeline.len() < 1000 && view.older { load_history(&mut view, &client, &mut jobs, &completed_tx, true); }
                                else { view.message = "History window: use lain work history --after for older/exported records".into(); }
                            }
                            KeyCode::Char('a' | 'd' | 'x') if key.modifiers.contains(KeyModifiers::CONTROL) && view.connected && !view.busy => {
                                let decision = match key.code { KeyCode::Char('a')=>Decision::ApproveOnce, KeyCode::Char('d')=>Decision::Deny, _=>Decision::Cancel };
                                if let Some(approval) = view.pending().get(view.approval_index % view.pending().len().max(1)).copied().filter(|a| a.decisions.contains(&decision)) {
                                    let approval = approval.id; let client = client.clone(); action(&mut view, &mut jobs, &completed_tx, async move { client.decide(id, approval, decision).await });
                                } else { view.message = "That approval decision is not offered".into(); }
                            }
                            KeyCode::Char('k' | 'r') if key.modifiers.contains(KeyModifiers::CONTROL) && view.connected && !view.busy => {
                                let endpoint = if key.code == KeyCode::Char('k') { "cancel" } else { "resume" }; let client = client.clone();
                                action(&mut view, &mut jobs, &completed_tx, async move { client.post(&format!("v1/work-sessions/{id}/{endpoint}"), None).await });
                            }
                            KeyCode::Enter if view.connected && !view.busy && view.sessions.get(&id).is_some_and(Session::can_send) && !view.input.text.trim().is_empty() => {
                                let text = view.input.text.clone(); view.submitted_prompt = Some((id, text.clone())); let client = client.clone();
                                action(&mut view, &mut jobs, &completed_tx, async move { client.post(&format!("v1/work-sessions/{id}/turns"), Some(json!({"text":text}))).await });
                            }
                            _ => view.input.key(key),
                        }
                    }
                }
                Some(Ok(TerminalEvent::Paste(text))) if view.selected.is_some() => view.input.insert(&text),
                Some(Ok(_))=>{}, Some(Err(error))=>return Err(error.into()), None=>break,
            },
            Some(update)=updates.recv()=>match update {
                Update::Snapshot(snapshot)=>{ view.snapshot(snapshot); load_history(&mut view, &client, &mut jobs, &completed_tx, false); }
                Update::Event(event)=>{
                    if let Err(error) = view.live(event) { view.message = safe_text(&format!("Invalid event: {error}")); }
                }
                Update::Offline(message)=>{ view.connected = false; view.message = format!("Reconnecting: {}", safe_text(&message)); }
            },
            Some(result)=completed.recv()=>match result {
                Completion::History(id, generation, result, more)=>{
                    if view.selected == Some(id) && generation == view.history_generation {
                        view.history_loading = more;
                        match result { Ok(page)=>{ if view.timeline.is_empty() || page.events.first().is_some_and(|e| e.history_order < view.timeline.first_key_value().map(|(o, _)| Some(*o)).unwrap_or(None)) { view.older = page.has_more; } view.merge(page.events); }
                            Err(error)=>{view.message = format!("History unavailable: {}", safe_text(&error.to_string()));}
                        }
                    }
                }
                Completion::Action(result)=>{ view.busy = false;
                    let submitted = view.submitted_prompt.take();
                    match result { Ok(_)=>{view.message = "Request accepted; completion follows backend events".into();
                        if submitted.is_some_and(|(id, text)| view.selected == Some(id) && view.input.text == text) { view.input = Input::default(); }
                    }, Err(error)=>view.message = safe_text(&error.to_string()) }
                }
            },
            _=jobs.join_next(), if !jobs.is_empty()=>{},
        }
    }
    Ok(()) // JoinSet cancels observation/HTTP tasks; guard restores even on errors/panics.
}

pub(crate) fn wrapped(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut result = Vec::new();
    for raw in text.lines() {
        let mut line = String::new();
        let mut used = 0;
        for c in raw.chars() {
            let n = c.width().unwrap_or(0);
            if used + n > width && !line.is_empty() {
                result.push(std::mem::take(&mut line));
                used = 0;
            }
            line.push(c);
            used += n;
        }
        result.push(line);
    }
    result
}
fn approval_choices(approval: &Approval) -> String {
    approval
        .decisions
        .iter()
        .map(|d| match d {
            Decision::ApproveOnce => "Ctrl-A approve once",
            Decision::Deny => "Ctrl-D deny",
            Decision::Cancel => "Ctrl-X cancel",
        })
        .collect::<Vec<_>>()
        .join(" · ")
}
fn draw(frame: &mut ratatui::Frame, view: &mut View) {
    let area = frame.area();
    if view.selected.is_none() {
        let chunks = Layout::vertical([Constraint::Min(3), Constraint::Length(2)]).split(area);
        let rows = view.sessions.values().map(|s| {
            let pending = view
                .approvals
                .values()
                .filter(|a| a.work_session_id == s.id && a.status == "pending")
                .count();
            Row::new(vec![
                s.id.to_string()[..8].to_owned(),
                safe_text(&s.backend),
                safe_text(&s.state),
                safe_text(&s.runtime),
                format!("{pending}"),
                safe_text(&s.workspace),
            ])
        });
        let table = Table::new(
            rows,
            [
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Length(20),
                Constraint::Length(9),
                Constraint::Length(3),
                Constraint::Min(8),
            ],
        )
        .header(
            Row::new(["SESSION", "BACKEND", "STATE", "RUNTIME", "ASK", "WORKSPACE"])
                .style(Style::default().fg(Color::Cyan)),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Lain · work sessions"),
        )
        .row_highlight_style(Style::default().bg(Color::DarkGray));
        frame.render_stateful_widget(table, chunks[0], &mut view.table);
        frame.render_widget(
            Paragraph::new(format!(
                "↑/↓ select · Enter attach · q quit\n{}",
                safe_text(&view.message)
            )),
            chunks[1],
        );
        return;
    }
    let id = view.selected.unwrap();
    let pending = view.pending();
    let approval = pending
        .get(view.approval_index % pending.len().max(1))
        .copied();
    let approval_text = approval.map(|a| {
        format!(
            "{} [{}] {}\nOffered: {}\n{}",
            a.id,
            a.kind,
            a.status,
            approval_choices(a),
            a.description
        )
    });
    let choices = approval.map(approval_choices).unwrap_or_default();
    if view.approval_full
        && let Some(text) = &approval_text
    {
        let chunks = Layout::vertical([Constraint::Min(2), Constraint::Length(3)]).split(area);
        let lines = wrapped(&safe_text(text), chunks[0].width.saturating_sub(2).into());
        view.approval_scroll = view.approval_scroll.min(
            lines
                .len()
                .saturating_sub(chunks[0].height.saturating_sub(2).into()),
        );
        frame.render_widget(
            Paragraph::new(
                lines
                    .into_iter()
                    .skip(view.approval_scroll)
                    .map(Line::from)
                    .collect::<Vec<_>>(),
            )
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Approval details · PgUp/PgDn scroll"),
            ),
            chunks[0],
        );
        frame.render_widget(
            Paragraph::new(format!(
                "F2 / Esc back · Tab next approval\n{choices}\nCtrl-Q quit"
            )),
            chunks[1],
        );
        return;
    }
    let approval_height = if approval.is_some() {
        (area.height / 3).clamp(3, 12)
    } else {
        0
    };
    let chunks = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(approval_height),
        Constraint::Min(2),
        Constraint::Length(3),
        Constraint::Length(2),
    ])
    .split(area);
    let header = view
        .sessions
        .get(&id)
        .map(|s| {
            format!(
                "{} · {}\n{}\nstate: {}   runtime: {}   turn: {}",
                id,
                s.runtime_profile_id,
                s.workspace,
                s.state,
                s.runtime,
                view.turns
                    .values()
                    .filter(|t| t.work_session_id == id)
                    .max_by_key(|t| t.created_at_ms)
                    .map(|t| format!(
                        "{} / {}{}",
                        t.state,
                        t.cancellation,
                        t.failure
                            .as_ref()
                            .and_then(|f| f["message"].as_str())
                            .map(|s| format!(" · {s}"))
                            .unwrap_or_default()
                    ))
                    .unwrap_or_else(|| "none".into())
            )
        })
        .unwrap_or_else(|| format!("{id}\nWaiting for Kernel snapshot"));
    frame.render_widget(
        Paragraph::new(safe_text(&header)).block(Block::default().borders(Borders::BOTTOM)),
        chunks[0],
    );
    if let Some(text) = approval_text {
        frame.render_widget(
            Paragraph::new(safe_text(&text))
                .wrap(ratatui::widgets::Wrap { trim: false })
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL).title(format!(
                    "APPROVAL {}/{} · Tab next · F2 details",
                    view.approval_index % pending.len() + 1,
                    pending.len()
                ))),
            chunks[1],
        );
    }
    if view.cached_revision != view.timeline_revision || view.cached_width != chunks[2].width {
        view.cached_lines = view
            .timeline
            .values()
            .flat_map(|e| wrapped(&event_text(e), chunks[2].width.saturating_sub(2).into()))
            .collect();
        view.cached_revision = view.timeline_revision;
        view.cached_width = chunks[2].width;
    }
    let max_scroll = view
        .cached_lines
        .len()
        .saturating_sub(chunks[2].height.saturating_sub(2).into());
    view.scroll = if view.follow {
        max_scroll
    } else {
        view.scroll.min(max_scroll)
    };
    frame.render_widget(
        Paragraph::new(
            view.cached_lines
                .iter()
                .skip(view.scroll)
                .take(chunks[2].height.saturating_sub(2).into())
                .map(|s| Line::from(s.as_str()))
                .collect::<Vec<_>>(),
        )
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Timeline · {} records · PgUp/PgDn · Ctrl-P older · Ctrl-End follow",
            view.timeline.len()
        ))),
        chunks[2],
    );
    let enabled =
        view.connected && !view.busy && view.sessions.get(&id).is_some_and(Session::can_send);
    // Horizontal viewport keeps editing/cursor valid even for long UTF-8 input.
    let width = chunks[3].width.saturating_sub(4) as usize;
    let mut start = 0;
    while view.input.text[start..view.input.cursor].width() > width.saturating_sub(1) {
        start += view.input.text[start..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or(1);
    }
    frame.render_widget(
        Paragraph::new(format!("> {}", &view.input.text[start..])).block(
            Block::default().borders(Borders::ALL).title(if enabled {
                "Enter send · Ctrl-U clear"
            } else {
                "Prompt waits for Idle / connection / current request"
            }),
        ),
        chunks[3],
    );
    if chunks[3].width > 4 && chunks[3].height > 2 {
        frame.set_cursor_position((
            chunks[3].x + 3 + view.input.text[start..view.input.cursor].width() as u16,
            chunks[3].y + 1,
        ));
    }
    frame.render_widget(
        Paragraph::new(format!(
            "Esc list · Ctrl-K cancel turn · Ctrl-R resume · Ctrl-Q quit\n{} · {}",
            safe_text(&view.message),
            safe_text(&view.progress)
        )),
        chunks[4],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a native interactive terminal; optional ConPTY smoke"]
    fn terminal_guard_restores_on_error_and_panic() {
        assert!(!crossterm::terminal::is_raw_mode_enabled().unwrap());
        let error: Result<()> = (|| {
            let _guard = TerminalGuard::enter()?;
            assert!(crossterm::terminal::is_raw_mode_enabled()?);
            anyhow::bail!("expected terminal error");
        })();
        assert!(error.is_err());
        assert!(!crossterm::terminal::is_raw_mode_enabled().unwrap());
        let panic = std::panic::catch_unwind(|| {
            let _guard = TerminalGuard::enter().unwrap();
            assert!(crossterm::terminal::is_raw_mode_enabled().unwrap());
            panic!("expected terminal panic");
        });
        assert!(panic.is_err());
        assert!(!crossterm::terminal::is_raw_mode_enabled().unwrap());
    }
    fn event(order: u64) -> Event {
        serde_json::from_value(json!({"id":format!("00000000-0000-0000-0000-{order:012}"),"history_order":order,"timestamp_ms":0,"work_session_id":"00000000-0000-0000-0000-000000000001","turn_id":null,"kind":"assistant_message_completed","text":"safe\u{001b}]52;c;clipboard\u{0007}中"})).unwrap()
    }
    #[test]
    fn history_live_overlap_dedup_order_and_snapshot_prevents_state_rollback() {
        let mut view = View::default();
        view.open(event(1).work_session_id);
        view.merge([event(3), event(1)]);
        view.live(event(3)).unwrap();
        view.merge([event(2)]);
        assert_eq!(
            view.timeline.keys().copied().collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        let id = view.selected.unwrap();
        view.snapshot(Snapshot {
            history_cursor: 10,
            sessions: vec![Session {
                id,
                backend: "codex".into(),
                runtime_profile_id: "codex/local".into(),
                workspace: "/server/project".into(),
                state: "idle".into(),
                runtime: "detached".into(),
            }],
            turns: vec![],
            approvals: vec![],
        });
        let mut old = event(4);
        old.kind = "session_state_changed".into();
        old.data.insert("to".into(), json!("running"));
        view.live(old).unwrap();
        assert_eq!(view.sessions[&id].state, "idle");
        let mut new = event(11);
        new.kind = "runtime_attachment_changed".into();
        new.data.insert("to".into(), json!("attached"));
        view.live(new).unwrap();
        assert_eq!(view.sessions[&id].runtime, "attached");
        let mut latest = event(12);
        latest.kind = "runtime_attachment_changed".into();
        latest.data.insert("to".into(), json!("detached"));
        view.live(latest).unwrap();
        let mut duplicate = event(11);
        duplicate.kind = "runtime_attachment_changed".into();
        duplicate.data.insert("to".into(), json!("attached"));
        view.live(duplicate).unwrap();
        assert_eq!(view.sessions[&id].runtime, "detached");
        let mut malformed = event(13);
        malformed.kind = "turn_completed".into();
        assert!(view.live(malformed).is_err());
        assert!(!event_text(&event(1)).contains('\u{001b}'));
    }
    #[test]
    fn unicode_input_editing_and_terminal_restoration_commands() {
        let mut input = Input::default();
        input.insert("a中b");
        input.key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        input.key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(input.text, "ab");
        input.insert("文");
        assert_eq!(input.text, "a文b");
        assert_eq!(
            safe_text("\u{009b}31m\u{001b}]52;c;x\u{0007}"),
            "\\u{9b}31m\\u{1b}]52;c;x\\u{7}"
        );
        let mut output = Vec::new();
        restore_commands(&mut output).unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("?1049l") && output.contains("?2004l") && output.contains("?25h"));
    }
    #[test]
    fn detail_render_resize_approval_choices_and_detached_input() {
        use ratatui::backend::TestBackend;
        let mut view = View::default();
        let id = event(1).work_session_id;
        view.open(id);
        view.sessions.insert(
            id,
            Session {
                id,
                backend: "codex".into(),
                runtime_profile_id: "codex/local".into(),
                workspace: "/remote/project".into(),
                state: "idle".into(),
                runtime: "detached".into(),
            },
        );
        view.connected = true;
        view.merge([event(1)]);
        for (width, height) in [(120, 35), (80, 24), (30, 10), (1, 1)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|f| draw(f, &mut view)).unwrap();
            let rendered: String = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect();
            if width == 120 {
                assert!(rendered.contains("runtime: detached") && rendered.contains("Enter send"));
            }
        }
        let failure = json!({"kind":"authentication_failed","message":"worker authentication failed","code":401,"retryable":false});
        let turn: Turn = serde_json::from_value(json!({"id":id,"work_session_id":id,"state":"failed","cancellation":"none","created_at_ms":1,"failure":failure})).unwrap();
        view.snapshot(Snapshot {
            history_cursor: 1,
            sessions: view.sessions.values().cloned().collect(),
            turns: vec![turn.clone()],
            approvals: vec![],
        });
        let mut terminal = Terminal::new(TestBackend::new(120, 35)).unwrap();
        terminal.draw(|f| draw(f, &mut view)).unwrap();
        let rendered: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(
            rendered.contains("worker authentication failed") && rendered.contains("codex/local")
        );
        let mut failed = event(2);
        failed.kind = "turn_completed".into();
        failed
            .data
            .insert("turn".into(), serde_json::to_value(turn).unwrap());
        assert!(event_text(&failed).contains("worker authentication failed"));
        view.approvals.insert(
            id,
            Approval {
                id,
                work_session_id: id,
                kind: "permissions".into(),
                description: "network requested".into(),
                decisions: vec![Decision::Deny],
                status: "pending".into(),
            },
        );
        let mut terminal = Terminal::new(TestBackend::new(120, 35)).unwrap();
        terminal.draw(|f| draw(f, &mut view)).unwrap();
        let rendered: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(rendered.contains("Ctrl-D deny") && !rendered.contains("Ctrl-A approve once"));
    }
}
