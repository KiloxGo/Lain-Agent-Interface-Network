//! Read-only task-family presentation. Original requests and per-turn receipts stay separate.
use super::*;
use lain_wire::device_commands::CommandState;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Row, Table, TableState, Wrap},
};

const PAGE: usize = 8; // 8 * (8 KiB request + 2 KiB receipt) fits the text document budget.
const LIMIT: usize = 64;
// Conservative one-column display budget, including tab expansion/Markdown decoration.
// The shared reader uses u16 row positions. Split before a valid multi-line request
// can push an otherwise byte-bounded page beyond that range.
fn windows(commands: &[&DeviceCommand]) -> Vec<(usize, usize)> {
    let mut windows = vec![];
    let mut end = commands.len();
    while end > 0 {
        let mut start = end;
        let mut cells = 0;
        while start > 0 && end - start < PAGE {
            let c = commands[start - 1];
            let request = safe_text(&c.input.task)
                .chars()
                .map(|c| {
                    if c == '\t' {
                        4
                    } else {
                        unicode_width::UnicodeWidthChar::width(c)
                            .unwrap_or(1)
                            .max(1)
                    }
                })
                .sum::<usize>();
            let output = c
                .execution
                .as_ref()
                .and_then(|e| e.result.as_ref())
                .map_or(0, |r| r.output_tail.len() * 4);
            let weight = request + output + 512;
            if start < end && cells + weight > 45000 {
                break;
            }
            cells += weight;
            start -= 1;
        }
        windows.push((start, end));
        end = start;
    }
    windows
}
#[derive(Default)]
pub(super) struct History {
    pub commands: Vec<DeviceCommand>,
    pub incomplete: bool,
}
pub(super) fn root(c: &DeviceCommand) -> DeviceCommandId {
    c.continuation.as_ref().map_or(c.id, |p| p.root_command_id)
}
fn valid_parent(child: &DeviceCommand, parent: &DeviceCommand) -> bool {
    child.continuation.as_ref().is_some_and(|p| {
        parent.id == p.parent_command_id
            && parent.device_id == child.device_id
            && root(parent) == p.root_command_id
            && parent.continued_by == Some(child.id)
            && parent.input.workspace_id == child.input.workspace_id
            && parent.input.profile_id == child.input.profile_id
            && parent.input.sandbox == child.input.sandbox
            && parent.execution.as_ref().and_then(|e| e.session_id) == Some(p.session_id)
    })
}
/// Ancestors outside the recent-32 snapshot use the existing conversation-scoped GET.
/// A partial read is labelled; it never becomes a command retry or synthetic result.
pub(super) async fn load(
    client: &Client,
    conversation: ConversationId,
    task: &DeviceCommand,
    cached: &[DeviceWorkRecord],
) -> Result<History> {
    let mut history = History::default();
    let mut child = task.clone();
    let mut seen = BTreeSet::from([child.id]);
    while let Some(link) = child.continuation.as_ref() {
        if history.commands.len() + 1 >= LIMIT || !seen.insert(link.parent_command_id) {
            history.incomplete = true;
            break;
        }
        let record = if let Some(r) = cached.iter().find(|r| {
            r.origin.conversation_id == conversation
                && r.command.device_id == task.device_id
                && r.command.id == link.parent_command_id
                && valid_parent(&child, &r.command)
        }) {
            Ok(r.clone())
        } else {
            client
                .get::<DeviceWorkRecord>(&format!(
                    "v1/conversations/{conversation}/device-work/{}/{}",
                    task.device_id, link.parent_command_id
                ))
                .await
        };
        let record = match record {
            Ok(record) => record,
            Err(e) if auth_error(&e) => return Err(e),
            Err(_) => {
                history.incomplete = true;
                break;
            }
        };
        let page = DeviceWorkPage {
            work: vec![record.clone()],
            truncated: false,
        };
        if page.validate(conversation).is_err() || !valid_parent(&child, &record.command) {
            history.incomplete = true;
            break;
        }
        child = record.command;
        history.commands.push(child.clone());
    }
    Ok(history)
}
pub(super) fn auth_error(e: &anyhow::Error) -> bool {
    e.downcast_ref::<crate::api::HttpFailure>()
        .is_some_and(|e| matches!(e.0, 401 | 403 | 426))
}
fn chain<'a>(app: &'a Model, task: &'a DeviceCommand) -> (Vec<&'a DeviceCommand>, bool) {
    let mut commands = vec![task];
    let mut seen = BTreeSet::from([task.id]);
    while let Some(link) = commands.last().unwrap().continuation.as_ref() {
        let current = *commands.last().unwrap();
        let next = app
            .remote_work
            .iter()
            .map(|r| &r.command)
            .chain(app.task_history.commands.iter())
            .filter(|c| c.device_id == task.device_id && c.id == link.parent_command_id)
            .max_by_key(|c| c.revision);
        let Some(parent) = next else {
            return (commands.into_iter().rev().collect(), true);
        };
        if commands.len() >= LIMIT || !seen.insert(parent.id) || !valid_parent(current, parent) {
            return (commands.into_iter().rev().collect(), true);
        }
        commands.push(parent);
    }
    commands.reverse();
    (commands, false)
}
pub(super) fn workspace(app: &Model, c: &DeviceCommand) -> String {
    app.devices
        .iter()
        .find(|d| d.device.registration.id == c.device_id)
        .and_then(|d| {
            d.device
                .registration
                .workspaces
                .iter()
                .find(|w| w.id == c.input.workspace_id)
        })
        .map(|w| safe_text(&w.path))
        .unwrap_or_else(|| format!("目录待同步 · {}", draw::short(c.input.workspace_id)))
}
fn device(app: &Model, c: &DeviceCommand) -> String {
    app.devices
        .iter()
        .find(|d| d.device.registration.id == c.device_id)
        .map(|d| safe_text(&d.device.registration.label))
        .unwrap_or_else(|| draw::short(c.device_id))
}
fn summary(text: &str) -> String {
    safe_text(text)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(180)
        .collect()
}
pub(super) fn timestamp(ms: u64) -> String {
    let Ok(seconds) = i64::try_from(ms / 1000) else {
        return "时间未知".into();
    };
    let Ok(date) = time::OffsetDateTime::from_unix_timestamp(seconds) else {
        return "时间未知".into();
    };
    let date =
        date.to_offset(time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC));
    let offset = date.offset().whole_seconds();
    format!(
        "{}-{:02}-{:02} {:02}:{:02} {}{:02}:{:02}",
        date.year(),
        u8::from(date.month()),
        date.day(),
        date.hour(),
        date.minute(),
        if offset < 0 { '-' } else { '+' },
        offset.unsigned_abs() / 3600,
        offset.unsigned_abs() / 60 % 60,
    )
}
fn state(c: &DeviceCommand) -> &'static str {
    match c.state {
        CommandState::Queued => "排队中",
        CommandState::Accepted => "已接收",
        CommandState::Running => "执行中",
        CommandState::Succeeded => "已返回结果",
        CommandState::Failed => "执行失败",
        CommandState::Cancelled => "已取消",
        CommandState::Rejected => "已拒绝",
        CommandState::Unknown => "结果待核实",
        CommandState::Expired => "已到期",
        CommandState::Revoked => "已撤销",
    }
}
fn updated(c: &DeviceCommand) -> u64 {
    c.last_event
        .as_ref()
        .map_or(c.created_at_ms, |e| e.received_at_ms)
}
pub(super) fn page(app: &mut Model, newer: bool) {
    let count = app
        .task
        .as_ref()
        .map_or(0, |t| windows(&chain(app, t).0).len());
    app.work_history_page = if newer {
        app.work_history_page.saturating_sub(1)
    } else {
        (app.work_history_page + 1).min(count.saturating_sub(1))
    };
    app.work_text.clear();
    app.scroll = 0;
    app.follow = false;
    app.work_diagnostics = false;
}
fn entry_id(c: &DeviceCommand, purpose: &str) -> uuid::Uuid {
    uuid::Uuid::new_v5(&c.id.0, purpose.as_bytes())
}
pub(super) fn prepare(app: &mut Model) {
    if !live::fresh(app) || app.source_invalidated {
        app.work_text.clear();
        return;
    }
    let Some(task) = app.task.as_ref() else {
        app.work_text.clear();
        return;
    };
    let target = (task.device_id, root(task));
    if app.work_text_target != Some(target) {
        app.work_text.clear();
        app.work_text_target = Some(target);
        app.work_history_page = 0;
    }
    let (all, incomplete) = chain(app, task);
    let count = all.len();
    let windows = windows(&all);
    let history_page = app.work_history_page.min(windows.len().saturating_sub(1));
    let (start, end) = windows[history_page];
    // Clone only the selected window so the source cache and rendered document stay bounded.
    let window = all[start..end]
        .iter()
        .map(|c| (*c).clone())
        .collect::<Vec<_>>();
    let first = all.first().unwrap();
    let incomplete = incomplete || app.task_history.incomplete;
    let header = format!(
        "工作目录（{} 登记）\n{}\n{} {} · 最近回执 {}\n当前载入第 {}–{} 轮，共已载入 {} 轮{}\n{}",
        device(app, task),
        workspace(app, task),
        if incomplete {
            "最早已载入"
        } else {
            "创建"
        },
        timestamp(first.created_at_ms),
        timestamp(updated(task)),
        start + 1,
        end,
        count,
        if incomplete {
            " · 更早历史未完整载入，Ctrl-R 重试"
        } else {
            ""
        },
        if windows.len() > 1 {
            "F5 更早一页 · Shift-F5 较新一页；权限/追加/取消仍针对所选任务，不随翻页改变。"
        } else {
            "滚轮 / PgUp 查看前文 · F7 查看所选轮次的诊断。"
        }
    );
    let header_id = entry_id(task, "family-header");
    app.work_history_page = history_page;
    let mut keep = BTreeSet::from([header_id]);
    for c in &window {
        keep.insert(entry_id(c, "request"));
        keep.insert(c.id.0);
    }
    let stale = app
        .work_text
        .doc
        .entries
        .iter()
        .filter(|e| !keep.contains(&e.id))
        .map(|e| e.id)
        .collect::<Vec<_>>();
    for id in stale {
        app.work_text.doc.remove(id);
    }
    app.work_text
        .doc
        .upsert(header_id, rich::Role::System, &header, false);
    app.work_text
        .doc
        .title(header_id, "任务时间线 · 设备回传；未独立验收结果");
    app.work_text.doc.place_after(header_id, None);
    let mut after = header_id;
    for (i, c) in window.iter().enumerate() {
        let request = entry_id(c, "request");
        app.work_text
            .doc
            .upsert(request, rich::Role::User, &c.input.task, true);
        app.work_text.doc.title(
            request,
            &format!(
                "You · 第 {} 轮{}{} · {}",
                start + i + 1,
                if incomplete { "（载入顺序）" } else { "" },
                if c.continuation.is_some() {
                    "追加"
                } else {
                    "原始任务"
                },
                timestamp(c.created_at_ms)
            ),
        );
        app.work_text.doc.place_after(request, Some(after));
        let result = c.execution.as_ref().and_then(|e| e.result.as_ref());
        let text = result
            .map(|r| r.output_tail.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("尚未收到正文；状态和权限请查看 F7 诊断或 F4 权限。");
        app.work_text.doc.upsert(
            c.id.0,
            rich::Role::Worker,
            text,
            result.is_some_and(|r| !r.output_tail.is_empty()),
        );
        app.work_text.doc.title(
            c.id.0,
            &format!(
                "Codex · {} · {} · {}",
                state(c),
                if result.is_some_and(|r| r.history_truncated) {
                    "回传节选"
                } else {
                    "回传结果"
                },
                timestamp(updated(c))
            ),
        );
        app.work_text.doc.place_after(c.id.0, Some(request));
        after = c.id.0;
    }
}

pub(super) fn draw(frame: &mut Frame, app: &mut Model, area: Rect) {
    let color = Color::Rgb(121, 185, 189);
    let count = app.sessions.len() + app.remote_heads().count();
    app.selected = app.selected.min(count.saturating_sub(1));
    let heads = app.remote_heads().collect::<Vec<_>>();
    let areas = Layout::vertical([
        Constraint::Min(5),
        Constraint::Length(if area.height >= 17 { 9 } else { 0 }),
    ])
    .split(area);
    let rows = app
        .sessions
        .iter()
        .map(|s| {
            Row::new(vec![
                format!("Kernel Worker {}", draw::short(s.id)),
                s.state.clone(),
                format!("{}\n{}", safe_text(&s.backend), safe_text(&s.workspace)),
            ])
            .height(2)
        })
        .chain(heads.iter().map(|r| {
            let c = &r.command;
            let (family, missing) = chain(app, c);
            let first = family.first().unwrap();
            Row::new(vec![
                if missing {
                    format!("原始任务待载入 · {}", summary(&c.input.task))
                } else {
                    summary(&first.input.task)
                },
                format!(
                    "{}\n{} · {}{}轮",
                    timestamp(updated(c)),
                    state(c),
                    if missing { "≥" } else { "" },
                    family.len()
                ),
                format!("Codex · {}\n{}", device(app, c), workspace(app, c)),
            ])
            .height(2)
        }))
        .collect::<Vec<_>>();
    let widget = Table::new(
        rows,
        [
            Constraint::Percentage(42),
            Constraint::Length(33),
            Constraint::Min(16),
        ],
    )
    .header(
        Row::new([
            "任务 / 原始请求",
            "最近回执（本地时区） / 状态",
            "执行设备 / 工作目录",
        ])
        .style(Style::default().fg(color)),
    )
    .row_highlight_style(Style::default().fg(Color::Black).bg(color))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Work · N 新建 · Enter 时间线 · Chat 与 Lain 交流 "),
    );
    frame.render_stateful_widget(
        widget,
        areas[0],
        &mut TableState::default().with_selected((count > 0).then_some(app.selected)),
    );
    if count == 0 {
        frame.render_widget(
            Paragraph::new("暂无任务 · 按 N 选择电脑和目录，创建 Codex 任务"),
            Rect::new(area.x + 2, area.y + 2, area.width.saturating_sub(4), 1),
        );
    }
    if let Some(r) = app
        .selected
        .checked_sub(app.sessions.len())
        .and_then(|i| heads.get(i))
    {
        let c = &r.command;
        let (family, missing) = chain(app, c);
        let first = family.first().unwrap();
        let text = format!(
            "{} {} · 最近回执 {}{}\n原始任务：{}\n最近指令：{}\n工作目录（{} 登记）：{}\nEnter 查看每轮指令与返回结果；在详情中按 F6 追加。",
            if missing { "最早已载入" } else { "创建" },
            timestamp(first.created_at_ms),
            timestamp(updated(c)),
            if missing {
                " · 更早轮次在详情中加载"
            } else {
                ""
            },
            if missing {
                "未载入 · Enter 读取前文".into()
            } else {
                summary(&first.input.task)
            },
            summary(&c.input.task),
            device(app, c),
            workspace(app, c),
        );
        frame.render_widget(
            Paragraph::new(text)
                .wrap(Wrap { trim: false })
                .block(Block::default().borders(Borders::TOP).title(" 所选任务 ")),
            areas[1],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn screen(terminal: &ratatui::Terminal<ratatui::backend::TestBackend>) -> String {
        let b = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..b.area.height {
            let mut x = 0;
            while x < b.area.width {
                let symbol = b[(x, y)].symbol();
                text.push_str(symbol);
                x += unicode_width::UnicodeWidthStr::width(symbol).max(1) as u16;
            }
            text.push('\n');
        }
        text
    }
    fn family(conversation: ConversationId, rounds: usize) -> Vec<DeviceWorkRecord> {
        let (first, _) = super::super::tests::idle_remote(conversation);
        let mut rows = vec![first];
        for n in 1..rounds {
            let previous = rows.last_mut().unwrap();
            let mut child = previous.clone();
            child.command.id = DeviceCommandId::new();
            child.command.input.task = format!("request-{n}");
            child.command.created_at_ms += 60000;
            let e = previous.command.execution.as_ref().unwrap();
            child.command.continuation = Some(lain_wire::device_work::DeviceContinuation {
                root_command_id: root(&previous.command),
                parent_command_id: previous.command.id,
                expected_parent_revision: previous.command.revision,
                session_id: e.session_id.unwrap(),
                parent_turn_id: e.turn_id.unwrap(),
            });
            let execution = child.command.execution.as_mut().unwrap();
            execution.turn_id = Some(lain_wire::WorkerTurnId::new());
            execution.result.as_mut().unwrap().output_tail = format!("result-{n}");
            previous.command.continued_by = Some(child.command.id);
            rows.push(child);
        }
        rows
    }
    #[test]
    fn history_pages_keep_exact_sources_and_controls_and_never_evict_a_partial_turn() {
        let mut app = Model::new(ConversationId::new());
        let mut rows = family(app.conversation, 9);
        for r in &mut rows {
            r.command.input.task = "中".repeat(2730); // just under the 8192-byte request cap
            r.command
                .execution
                .as_mut()
                .unwrap()
                .result
                .as_mut()
                .unwrap()
                .output_tail = "r".repeat(2048);
        }
        let head = rows.last().unwrap().command.clone();
        let first = rows.first().unwrap().command.clone();
        app.surface = Surface::Task(head.device_id, head.id);
        app.task = Some(head.clone());
        app.remote_work = rows;
        prepare(&mut app);
        let latest_entries = app.work_text.doc.entries.len();
        assert!((3..=17).contains(&latest_entries) && latest_entries % 2 == 1);
        assert!(
            app.work_text
                .doc
                .copy(first.id.0, rich::CopyKind::Plain)
                .is_none()
        );
        for _ in 0..9 {
            page(&mut app, false);
            prepare(&mut app);
        }
        assert_eq!(
            app.work_text
                .doc
                .copy(first.id.0, rich::CopyKind::Plain)
                .unwrap(),
            "r".repeat(2048)
        );
        assert_eq!(app.task.as_ref().unwrap().id, head.id);
        for _ in 0..9 {
            page(&mut app, true);
            prepare(&mut app);
        }
        assert_eq!(app.work_text.doc.entries.len(), latest_entries);
        assert_eq!(
            app.work_text
                .doc
                .copy(head.id.0, rich::CopyKind::Markdown)
                .unwrap(),
            "r".repeat(2048)
        );
        live::retire(&mut app);
        assert!(app.task_history.commands.is_empty() && app.work_text.doc.entries.is_empty());
    }
    #[test]
    fn multiline_pages_keep_the_last_result_within_the_readers_row_range() {
        let mut app = Model::new(ConversationId::new());
        let mut rows = family(app.conversation, 8);
        for r in &mut rows {
            r.command.input.task = format!("x{}", "\n".repeat(8191));
            r.command
                .execution
                .as_mut()
                .unwrap()
                .result
                .as_mut()
                .unwrap()
                .output_tail = format!("```text\n{}\n```", "x\n".repeat(1000));
        }
        let head = rows.last().unwrap().command.clone();
        app.surface = Surface::Task(head.device_id, head.id);
        app.task = Some(head);
        app.remote_work = rows;
        prepare(&mut app);
        let (commands, _) = chain(&app, app.task.as_ref().unwrap());
        assert!(
            windows(&commands).len() > 1,
            "a byte-bounded page can still exceed the row limit"
        );
        for _ in 0..8 {
            assert!(app.work_text.doc.layout(1).rows.len() < usize::from(u16::MAX));
            page(&mut app, false);
            prepare(&mut app);
        }
        let first = &app.remote_work[0].command;
        assert!(
            app.work_text
                .doc
                .copy(first.id.0, rich::CopyKind::Markdown)
                .is_some()
        );
    }
    #[tokio::test]
    async fn ancestor_read_recovers_a_stale_or_missing_recent_snapshot_and_rejects_wrong_scope() {
        use axum::{Json, Router, routing::get};
        let conversation = ConversationId::new();
        let rows = family(conversation, 2);
        let parent = rows[0].clone();
        let child = rows[1].command.clone();
        let route = format!(
            "/v1/conversations/{conversation}/device-work/{}/{}",
            parent.command.device_id, parent.command.id
        );
        let response = parent.clone();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new().route(
                    &route,
                    get(move || {
                        let r = response.clone();
                        async move { Json(r) }
                    }),
                ),
            )
            .await
            .unwrap();
        });
        let client = Client::new(
            &format!("http://{address}"),
            "history-fixture-only-credential-123456",
        )
        .unwrap();
        let mut stale = parent.clone();
        stale.command.continued_by = None;
        let history = load(&client, conversation, &child, &[stale]).await.unwrap();
        assert!(!history.incomplete);
        assert_eq!(history.commands[0].id, parent.command.id);
        let history = load(&client, conversation, &child, &[]).await.unwrap();
        assert_eq!(history.commands[0].input.task, parent.command.input.task);
        // Neither cache nor the exact GET can substitute another workspace/session family.
        let mut wrong = child.clone();
        wrong.input.workspace_id = lain_wire::DeviceWorkspaceId::new();
        let history = load(&client, conversation, &wrong, &[parent])
            .await
            .unwrap();
        assert!(history.incomplete && history.commands.is_empty());
        server.abort();
    }
    #[tokio::test]
    async fn ancestor_auth_failure_is_not_hidden_as_an_empty_history() {
        use axum::{Router, http::StatusCode, routing::get};
        let conversation = ConversationId::new();
        let rows = family(conversation, 2);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new().fallback(get(|| async { StatusCode::UNAUTHORIZED })),
            )
            .await
            .unwrap();
        });
        let client = Client::new(
            &format!("http://{address}"),
            "history-fixture-only-credential-123456",
        )
        .unwrap();
        let error = load(&client, conversation, &rows[1].command, &[])
            .await
            .err()
            .unwrap();
        assert!(auth_error(&error));
        server.abort();
    }
    #[test]
    fn task_list_shows_human_context_and_labels_unavailable_roots_without_guessing() {
        use ratatui::{Terminal, backend::TestBackend};
        let mut app = Model::new(ConversationId::new());
        let (record, device) = super::super::tests::idle_remote(app.conversation);
        app.devices = vec![device];
        app.remote_work = vec![record.clone()];
        let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
        terminal.draw(|f| draw(f, &mut app, f.area())).unwrap();
        let text = screen(&terminal);
        assert!(
            text.contains("original remote task")
                && text.contains("C:\\fixture")
                && text.contains(&timestamp(1))
        );
        let mut rows = family(app.conversation, 2);
        rows.remove(0);
        app.remote_work = rows;
        terminal.draw(|f| draw(f, &mut app, f.area())).unwrap();
        let text = screen(&terminal);
        assert!(text.contains("原始任务待载入") && text.contains("目录待同步"));
        assert_eq!(timestamp(u64::MAX), "时间未知");
    }
}
