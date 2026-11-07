use super::*;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
};
use unicode_width::UnicodeWidthStr;

const CYAN: Color = Color::Rgb(121, 185, 189);
const DIM: Color = Color::Rgb(137, 145, 158);

/// Shared wrapping keeps scrolling, review completion and cursor placement consistent.
pub(super) fn lines(text: &str, width: u16) -> Vec<String> {
    let mut rows = tui::wrapped(&text.replace('\t', "    "), usize::from(width.max(1)));
    if text.is_empty() || text.ends_with('\n') {
        rows.push(String::new());
    }
    rows
}

pub(super) fn body(
    frame: &mut Frame,
    area: Rect,
    text: &str,
    scroll: &mut u16,
    follow: bool,
    title: &str,
) -> bool {
    body_position(frame, area, text, scroll, follow, title).0
}
pub(super) fn body_position(
    frame: &mut Frame,
    area: Rect,
    text: &str,
    scroll: &mut u16,
    follow: bool,
    title: &str,
) -> (bool, u16) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(DIM));
    let inner = block.inner(area);
    let rows = lines(text, inner.width);
    let height = usize::from(inner.height);
    let end = rows.len().saturating_sub(height).min(u16::MAX as usize) as u16;
    *scroll = if follow { end } else { (*scroll).min(end) };
    let read = inner.width > 0 && height > 0 && usize::from(*scroll) + height >= rows.len();
    let visible = rows
        .into_iter()
        .skip(usize::from(*scroll))
        .take(height)
        .map(Line::raw)
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(visible).block(block), area);
    (read, end)
}

pub(super) fn render(frame: &mut Frame, app: &mut Model) {
    app.chat_viewport = None;
    app.detail_viewport = None;
    let area = frame.area();
    if area.width < 40 || area.height < 14 {
        frame.render_widget(Paragraph::new("lain // wired\nResize to at least 40 x 14.\nDraft retained. Ctrl-Q exits; work continues."),area);
        return;
    }
    // Preserve the selected portrait verbatim. Never crop it into a small face.
    let detailed =
        !app.text_view.reader && app.portrait.is_some() && area.width >= 110 && area.height >= 32;
    let welcome = !detailed
        && app.portrait.is_some()
        && app.surface == Surface::Chat
        && app.transcript.is_empty()
        && app.pending.is_none()
        && app.review.is_none()
        && area.height >= 24;
    if welcome {
        let parts = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(18),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(area);
        frame.render_widget(
            Paragraph::new(format!("lain // wired · Chat\n{}", app.conversation))
                .style(Style::default().fg(CYAN)),
            parts[0],
        );
        let face = Rect::new(
            parts[1].x + (parts[1].width - super::portrait::WIDTH) / 2,
            parts[1].y,
            super::portrait::WIDTH,
            super::portrait::HEIGHT,
        );
        frame.render_widget(
            Paragraph::new(app.portrait.as_deref().unwrap()).style(Style::default().fg(CYAN)),
            face,
        );
        composer(frame, parts[2], &app.input);
        frame.render_widget(
            Paragraph::new(if !app.connected {
                "Disconnected · Ctrl-R inspect · Ctrl-Q exit"
            } else {
                app.chat_status
                    .badge()
                    .unwrap_or("Connected · Tab Work/Devices · Ctrl-Q exit")
            })
            .style(Style::default().fg(DIM)),
            parts[3],
        );
        return;
    }
    let (main, face_area) = if detailed {
        let columns = Layout::horizontal([
            Constraint::Length(32),
            Constraint::Length(2),
            Constraint::Min(1),
        ])
        .split(area);
        (columns[2], Some(columns[0]))
    } else {
        (area, None)
    };
    let header = if main.width < 52 { 5 } else { 4 };
    let regions = Layout::vertical([
        Constraint::Length(header),
        Constraint::Min(3),
        Constraint::Length(4),
        Constraint::Length(2),
    ])
    .split(main);
    if let Some(side) = face_area {
        let face = Rect::new(
            side.x + 1,
            side.y + 1,
            super::portrait::WIDTH,
            super::portrait::HEIGHT,
        );
        frame.render_widget(
            Paragraph::new(app.portrait.as_deref().unwrap()).style(Style::default().fg(CYAN)),
            face,
        );
        frame.render_widget(
            Paragraph::new("LAIN\n\n在这里。").style(Style::default().fg(CYAN)),
            Rect::new(side.x + 1, side.y + 21, side.width - 1, 4),
        );
        let info = Rect::new(
            side.x + 1,
            side.y + 26,
            side.width.saturating_sub(2),
            side.height.saturating_sub(26),
        );
        let rows = sidebar::rows(app, info.width, info.height)
            .into_iter()
            .map(|row| Line::styled(row, Style::default().fg(DIM)))
            .collect::<Vec<_>>();
        frame.render_widget(Paragraph::new(rows), info);
    }
    let active = match app.surface {
        Surface::Chat => "[ Chat ]   Work    Devices",
        Surface::Work => "  Chat   [ Work ]  Devices",
        Surface::Task(..) if app.task_from_work => "  Chat   [ Work ]  Devices",
        Surface::Sources | Surface::Source(_) => "  Chat     Work    Devices  / Sources",
        _ => "  Chat     Work  [ Devices ]",
    };
    let state = if !app.connected {
        "disconnected · read /refresh"
    } else if app
        .turn
        .as_ref()
        .is_some_and(|t| t.state == ActorTurnState::PendingConfirmation)
    {
        "action awaits review · F4"
    } else if app.pending.as_ref().is_some_and(|p| p.uncertain) {
        "delivery uncertain · Ctrl-R inspect · /retry exact request"
    } else if app.active() || app.pending.is_some() {
        "responding · next draft is editable"
    } else {
        app.chat_status
            .badge()
            .unwrap_or("quiet · connected to Kernel")
    };
    let separator = if main.width < 52 { "\n" } else { " " };
    let heading = format!(
        "lain // wired\n{active}\n{state}\nconversation{separator}{}",
        app.conversation
    );
    frame.render_widget(
        Paragraph::new(heading).style(Style::default().fg(CYAN)),
        regions[0],
    );

    if app.source_setup.panel.is_some() {
        source_setup::draw(frame, app, regions[1], regions[2]);
    } else if app.source_access.panel.is_some() {
        source_access::draw(frame, app, regions[1], regions[2]);
    } else if let Some(r) = app.cancel_review.as_mut() {
        let end = body(
            frame,
            regions[1],
            &r.text,
            &mut r.scroll,
            false,
            " Cancel selected device task · PgUp/PgDn ",
        );
        let width = regions[1].width.saturating_sub(2);
        r.page = regions[1].height.saturating_sub(2).max(1);
        if r.width != width {
            r.width = width;
            r.seen_to = 0;
            r.read = false;
        }
        if usize::from(r.scroll) <= r.seen_to {
            r.seen_to = r.seen_to.max(usize::from(r.scroll) + usize::from(r.page));
            r.read |= end;
        }
        let hint = if app.cancel_pending.is_some() {
            "Cancellation receipt pending · Ctrl-R reads · Esc closes"
        } else if r.read {
            "Ctrl-K request cancellation · Esc closes without effect"
        } else {
            "Read the full target before requesting cancellation · PgDn · Esc closes"
        };
        frame.render_widget(
            Paragraph::new(hint).style(Style::default().fg(Color::Yellow)),
            regions[2],
        );
    } else if app.new_work.is_some() {
        new_work::draw(frame, app, regions[1], regions[2]);
    } else if let Some(panel) = app.followup.as_mut() {
        if let Some(r) = panel.preview.as_mut() {
            let end = body(
                frame,
                regions[1],
                &r.text,
                &mut r.scroll,
                false,
                " Review remote follow-up · PgUp/PgDn ",
            );
            let width = regions[1].width.saturating_sub(2);
            r.page = regions[1].height.saturating_sub(2).max(1);
            if r.width != width {
                r.width = width;
                r.seen_to = 0;
                r.read = false;
            }
            if usize::from(r.scroll) <= r.seen_to {
                r.seen_to = r.seen_to.max(usize::from(r.scroll) + usize::from(r.page));
                r.read |= end;
            }
            let hint = if app.followup_pending.is_some() {
                "Receipt pending · Ctrl-R reads · Esc keeps draft"
            } else if r.read {
                "Ctrl-Y send exact follow-up · Esc edits without sending"
            } else {
                "Read full target and text before sending · PgDn · Esc edits"
            };
            frame.render_widget(
                Paragraph::new(hint).style(Style::default().fg(Color::Yellow)),
                regions[2],
            );
        } else {
            let c = &panel.target;
            let mut scroll = 0;
            let text = format!(
                "Follow-up to same remote Worker\nDevice {}\nParent command {}\nSession {}\nWorkspace {} · profile {} · sandbox {:?}\n\nLast known completed/failed turn only; running steering is unsupported.\nEnter previews exact text/scope. Nothing is sent until explicit confirmation.\nEsc keeps this Worker draft; your Lain chat draft is separate.",
                c.device_id,
                c.id,
                c.execution
                    .as_ref()
                    .and_then(|e| e.session_id)
                    .map_or_else(|| "unavailable".into(), |id| id.to_string()),
                c.input.workspace_id,
                c.input.profile_id,
                c.input.sandbox
            );
            body(
                frame,
                regions[1],
                &text,
                &mut scroll,
                false,
                " Remote Worker input ",
            );
            composer_with_title(
                frame,
                regions[2],
                &panel.input,
                " to remote Codex · F9 paste · Enter preview · Ctrl-J newline ",
            );
        }
    } else if let Some(r) = &mut app.remote_review {
        let at_end = body(
            frame,
            regions[1],
            &r.text,
            &mut r.scroll,
            false,
            " Remote Worker permission · exact scope · PgUp/PgDn ",
        );
        let width = regions[1].width.saturating_sub(2);
        r.page = regions[1].height.saturating_sub(2).max(1);
        if r.width != width {
            r.width = width;
            r.seen_to = 0;
        }
        if usize::from(r.scroll) <= r.seen_to {
            r.seen_to = r.seen_to.max(usize::from(r.scroll) + usize::from(r.page));
            r.read |= at_end;
        }
        let hint = if r.read {
            format!(
                "{}\nCloud receipt does not prove Worker application · Esc close",
                r.choices_hint()
            )
        } else {
            "Read every page before deciding · PgDn · Esc close".into()
        };
        frame.render_widget(
            Paragraph::new(hint).style(Style::default().fg(Color::Yellow)),
            regions[2],
        );
    } else if let Some(r) = &mut app.review {
        // The immutable full payload must be reachable; there is no hidden truncation.
        let text = format!(
            "Lain action · exact scope\nWorker permissions belong to their own session.\n\n{}",
            r.text
        );
        let at_end = body(
            frame,
            regions[1],
            &text,
            &mut r.scroll,
            false,
            " Review action · PgUp/PgDn ",
        );
        let width = regions[1].width.saturating_sub(2);
        r.page = regions[1].height.saturating_sub(2).max(1);
        if r.width != width {
            r.width = width;
            r.seen_to = 0;
        }
        if usize::from(r.scroll) <= r.seen_to {
            r.seen_to = r.seen_to.max(usize::from(r.scroll) + usize::from(r.page));
            r.read |= at_end;
        }
        let hint = if r.read {
            "Ctrl-Y approve once · Ctrl-D deny · Esc close"
        } else {
            "Read to the end before deciding · PgDn · Esc close"
        };
        frame.render_widget(
            Paragraph::new(hint).style(Style::default().fg(Color::Yellow)),
            regions[2],
        );
    } else {
        match app.surface {
            Surface::Chat => {
                text_view::draw(frame, app, regions[1]);
            }
            Surface::Work => {
                work_history::draw(frame, app, regions[1]);
            }
            Surface::Devices => devices_view::list(frame, app, regions[1]),
            Surface::Device(id) => devices_view::detail(frame, app, regions[1], id),
            Surface::Sources => sources_view::list(frame, app, regions[1]),
            Surface::Source(id) => sources_view::detail(frame, app, regions[1], id),
            Surface::Tasks(id) => {
                let rows = app.tasks.iter().map(|t| {
                    Row::new(vec![
                        short(t.id),
                        safe_text(&t.profile_id),
                        format!("{:?}", t.state),
                        if t.cancel_requested {
                            "cancel requested".into()
                        } else {
                            short(t.workspace_id)
                        },
                    ])
                });
                table(
                    frame,
                    regions[1],
                    rows,
                    ["Task", "Profile", "Reported state", "Workspace"],
                    &mut app.selected,
                    app.tasks.len(),
                    &format!(" Device {} · Enter inspect · read only ", short(id)),
                );
            }
            Surface::Task(_, _) => {
                if !app.work_diagnostics {
                    text_view::draw(frame, app, regions[1]);
                } else {
                    let text = app
                        .task
                        .as_ref()
                        .map(|task| task_text(task, app.task_decisions.as_deref()))
                        .unwrap_or_else(|| "Loading selected task…".into());
                    body(
                        frame,
                        regions[1],
                        &text,
                        &mut app.scroll,
                        app.follow,
                        " Device task · F4 permission · F6 follow-up · F8 cancel ",
                    );
                }
            }
        }
        if app.surface == Surface::Chat {
            if app.text_view.reader {
                frame.render_widget(Paragraph::new("只读阅读 · 拖选正文 · Ctrl-C 复制选区 · F2 复制菜单\nPgUp/PgDn 滚动 · Ctrl-End 最新 · Esc/F3 返回聊天 · 草稿保留").style(Style::default().fg(DIM)),regions[2]);
            } else {
                composer(frame, regions[2], &app.input);
            }
        } else {
            let hint = match app.surface {
                Surface::Work => {
                    "Work：N 新建任务 · ↑/↓ 选择 · Enter 查看 · Esc 回到 Lain\n任务后台执行；在 Chat 中可以继续与 Lain 交流。"
                }
                Surface::Sources => {
                    "N 登记 · E 保存范围 · S 有限同步 · G 会话阅读\n↑/↓ 选择 · Enter 详情 · Ctrl-R 刷新 · Esc 聊天"
                }
                Surface::Source(_) => {
                    "N 登记 · E 保存范围 · S 有限同步 · G 会话阅读\n滚轮/PgUp/PgDn · Ctrl-R 刷新 · Esc 来源列表"
                }
                Surface::Devices => {
                    "Devices：管理执行电脑、目录和授权\n↑/↓ 选择 · Enter 电脑详情 · Ctrl-R 刷新 · Esc 回到 Lain"
                }
                Surface::Device(_) if main.width < 90 => {
                    "↑/↓ 项目 · N 新任务 · C 检查\nI 启动 · F7 诊断 · T 任务清单\n滚轮/PgUp/PgDn · Esc 返回"
                }
                Surface::Device(_) => {
                    "↑/↓ 选择项目 · N 在此创建任务 · C 检查云端连接\nI 启动说明 · F7 诊断 · T 电脑任务清单 · PgUp/PgDn 滚动 · Esc 返回电脑列表"
                }
                Surface::Tasks(_) => {
                    "这台电脑的任务清单（含其他聊天） · ↑/↓ 选择 · Enter 查看\nEsc 返回电脑详情；当前聊天的任务和结果到 Work 查看。"
                }
                _ => {
                    "F2 复制 · F5 更早轮次 / Shift-F5 较新轮次 · F7 时间线/诊断 · Esc 回到 Lain\nF4 权限 · F6 空闲会话追加 · F8 取消任务；切换页面不停止工作。"
                }
            };
            frame.render_widget(
                Paragraph::new(hint).style(Style::default().fg(DIM)),
                regions[2],
            );
        }
    }
    frame.render_widget(
        Paragraph::new(format!("{}\n{}", safe_text(&app.status),
            if matches!(app.surface, Surface::Devices | Surface::Device(_) | Surface::Tasks(_) | Surface::Sources | Surface::Source(_)) {
                "Tab 切换 Chat / Work / Devices · Ctrl-Q 退出"
            } else {
                "F2 copy · F3 read · F7 动态/诊断 · F9 paste · Tab Chat/Work/Devices · F4 review · Ctrl-Q exit"
            }))
        .style(Style::default().fg(DIM)),
        regions[3],
    );
}
pub(super) fn short(id: impl std::fmt::Display) -> String {
    id.to_string().chars().take(8).collect()
}
fn table<'a>(
    frame: &mut Frame,
    area: Rect,
    rows: impl Iterator<Item = Row<'a>>,
    headers: [&'a str; 4],
    selected: &mut usize,
    count: usize,
    title: &str,
) {
    *selected = (*selected).min(count.saturating_sub(1));
    let rows = rows.collect::<Vec<_>>();
    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Percentage(22),
            Constraint::Percentage(24),
            Constraint::Min(10),
        ],
    )
    .header(Row::new(headers).style(Style::default().fg(CYAN)))
    .row_highlight_style(Style::default().fg(Color::Black).bg(CYAN))
    .block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(DIM)),
    );
    frame.render_stateful_widget(
        table,
        area,
        &mut TableState::default().with_selected((count > 0).then_some(*selected)),
    );
    if count == 0 {
        let inner = Rect::new(
            area.x + 2,
            area.y + 2,
            area.width.saturating_sub(4),
            area.height.saturating_sub(3),
        );
        frame.render_widget(Paragraph::new("No entries in this view."), inner);
    }
}
fn composer(frame: &mut Frame, area: Rect, input: &Composer) {
    composer_with_title(
        frame,
        area,
        input,
        " to Lain · Enter send · Ctrl-J newline ",
    );
}
pub(super) fn composer_with_title(frame: &mut Frame, area: Rect, input: &Composer, title: &str) {
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CYAN));
    let inner = block.inner(area);
    let text = safe_text(&input.text);
    let prefix = safe_text(&input.text[..input.cursor]);
    let rows = lines(&text, inner.width);
    let before = lines(&prefix, inner.width);
    let row = before.len().saturating_sub(1);
    let col = UnicodeWidthStr::width(before.last().map_or("", String::as_str));
    let (row, col) = if col >= usize::from(inner.width.max(1)) {
        (row + 1, 0)
    } else {
        (row, col)
    };
    let start = row.saturating_sub(usize::from(inner.height.saturating_sub(1)));
    frame.render_widget(
        Paragraph::new(
            rows.into_iter()
                .skip(start)
                .map(Line::raw)
                .collect::<Vec<_>>(),
        )
        .block(block),
        area,
    );
    if inner.width > 0 && inner.height > 0 {
        frame.set_cursor_position((inner.x + col as u16, inner.y + (row - start) as u16));
    }
}
pub(super) fn task_text(
    task: &DeviceCommand,
    decisions: Option<&[lain_wire::device_execution::DeviceApprovalDecision]>,
) -> String {
    // Typed Lain DTO; values are still untrusted display data.
    let mut text = format!(
        "Task {}\nDevice {}\nState {:?} · revision {}\nCancel requested: {} · interruption: {:?}\nProfile: {} · workspace: {} · sandbox: {:?}\nDeadline (ms): {}\n\nOriginal request\n{}\n",
        task.id,
        task.device_id,
        task.state,
        task.revision,
        task.cancel_requested,
        task.interruption,
        task.input.profile_id,
        task.input.workspace_id,
        task.input.sandbox,
        task.input.expires_at_ms,
        task.input.task
    );
    if let Some(e) = &task.execution {
        text.push_str(&format!(
            "\nDevice evidence · {:?}\nExecution {}\nSession {:?} · turn {:?}\n",
            e.phase, e.execution_id, e.session_id, e.turn_id
        ));
        for a in &e.pending_approvals {
            text.push_str(&format!("\nPending approval {} · {}\n{}\nTruncated: {} · offered choices: {:?}\nUse `lain device command decide` for an explicit decision.\n",a.id,a.kind,a.description,a.truncated,a.choices));
        }
        if let Some(r) = &e.result {
            text.push_str(&format!("\nOutput tail (bounded)\n{}\nCommand failures: {} · history incomplete: {}\nReported paths (unverified references; no file reads):\n{}\nPaths withheld: {}\n",r.output_tail,r.command_failures,r.history_truncated,r.reported_paths.join("\n"),r.paths_withheld));
        }
    } else {
        text.push_str("\nNo execution evidence reported.\n");
    }
    if let Some(p) = &task.continuation {
        text.push_str(&format!("\nSame-session continuation\nRoot {}\nParent {}\nTarget session {} · previous turn {}\n",p.root_command_id,p.parent_command_id,p.session_id,p.parent_turn_id));
    }
    if let Some(next) = task.continued_by {
        text.push_str(&format!("\nNext command {next}\nEnter opens the recorded next turn; this parent cannot be continued again.\n"));
    }
    text.push_str(
        "\nCloud permission decisions (not device receipt or backend application proof)\n",
    );
    match decisions {
        None => text.push_str("Decision receipts unavailable; refresh before reviewing.\n"),
        Some([]) => text.push_str("No recorded decision in the fetched page.\n"),
        Some(rows) => {
            for row in rows {
                text.push_str(&format!(
                    "Approval {} · request {} · {:?} · recorded at {}\n",
                    row.approval_id,
                    row.decision.request_id,
                    row.decision.choice,
                    row.received_at_ms
                ));
                let report = task.execution.as_ref().and_then(|e| {
                    e.approval_reports
                        .iter()
                        .find(|r| r.approval_id == row.approval_id && r.decision == row.decision)
                });
                text.push_str(match report.map(|r| r.stage) {
                    None => "Cloud decision: recorded\nDevice receipt: unconfirmed\nWorker application: unconfirmed\n",
                    Some(lain_wire::device_execution::DeviceApprovalStage::Received) => "Cloud decision: recorded\nDevice receipt: reported received\nWorker application: unconfirmed\n",
                    Some(lain_wire::device_execution::DeviceApprovalStage::Applied) => "Cloud decision: recorded\nDevice receipt: reported received\nWorker application: reported confirmed (matching native request resolved; no proof of downstream work)\n",
                });
            }
        }
    }
    safe_text(&text)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn permission_stages_use_exact_evidence_instead_of_task_completion() {
        use lain_wire::device_execution::*;
        let mut task = super::super::tests::remote_record(ConversationId::new()).command;
        let aid = lain_wire::WorkerApprovalId::new();
        let decision = DeviceApprovalDecision {
            approval_id: aid,
            decision: DecideDeviceApproval {
                request_id: RequestId::new(),
                fingerprint: "a".repeat(64),
                choice: DeviceApprovalChoice::Deny,
            },
            received_at_ms: 1,
        };
        assert!(
            task_text(&task, Some(std::slice::from_ref(&decision)))
                .contains("Device receipt: unconfirmed")
        );
        task.execution = Some(ExecutionEvidence {
            execution_id: lain_wire::DeviceExecutionId::new(),
            session_id: Some(lain_wire::WorkSessionId::new()),
            turn_id: Some(lain_wire::WorkerTurnId::new()),
            phase: ExecutionPhase::Running,
            pending_approvals: vec![],
            result: None,
            approval_reports: vec![DeviceApprovalReport {
                approval_id: aid,
                decision: decision.decision.clone(),
                stage: DeviceApprovalStage::Received,
            }],
        });
        let text = task_text(&task, Some(std::slice::from_ref(&decision)));
        assert!(text.contains("Device receipt: reported received"));
        assert!(text.contains("Worker application: unconfirmed"));
        task.execution.as_mut().unwrap().approval_reports[0].stage = DeviceApprovalStage::Applied;
        assert!(
            task_text(&task, Some(std::slice::from_ref(&decision)))
                .contains("Worker application: reported confirmed")
        );
        task.execution.as_mut().unwrap().approval_reports[0]
            .decision
            .request_id = RequestId::new();
        assert!(
            task_text(&task, Some(std::slice::from_ref(&decision)))
                .contains("Device receipt: unconfirmed"),
            "another request is not proof for this decision"
        );
    }
    use ratatui::backend::TestBackend;
    #[test]
    fn all_sizes_render_and_small_review_does_not_enable_decision() {
        let mut app = Model::new(ConversationId::new());
        app.input.insert("草稿\nnext");
        app.review = Some(Review {
            turn: lain_wire::ActorTurnId::new(),
            action: lain_wire::ActionId::new(),
            revision: 1,
            text: "scope\n".repeat(100),
            scroll: 0,
            read: false,
            seen_to: 0,
            width: 0,
            page: 1,
        });
        for (w, h) in [(1, 1), (39, 13), (40, 14), (80, 24), (120, 40)] {
            let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
            terminal.draw(|f| render(f, &mut app)).unwrap();
            assert!(!app.review.as_ref().unwrap().read);
        }
        app.review.as_mut().unwrap().scroll = u16::MAX;
        Terminal::new(TestBackend::new(80, 24))
            .unwrap()
            .draw(|f| render(f, &mut app))
            .unwrap();
        assert!(
            !app.review.as_ref().unwrap().read,
            "jumping over unseen scope must not enable approval"
        );
        assert_eq!(app.input.text, "草稿\nnext");
        // Starting at the top and visiting each page enables a decision, including
        // one-line body windows where a fixed eight-line jump would skip scope.
        app.review.as_mut().unwrap().scroll = 0;
        app.review.as_mut().unwrap().seen_to = 0;
        let mut terminal = Terminal::new(TestBackend::new(40, 14)).unwrap();
        for _ in 0..500 {
            terminal.draw(|f| render(f, &mut app)).unwrap();
            let r = app.review.as_mut().unwrap();
            if r.read {
                break;
            }
            r.scroll = r.scroll.saturating_add(r.page);
        }
        assert!(app.review.as_ref().unwrap().read);
    }

    #[test]
    fn composer_cursor_tracks_trailing_newline_and_cjk_wrap() {
        let mut app = Model::new(ConversationId::new());
        app.input.insert("你好\n");
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let cursor =
            ratatui::backend::Backend::get_cursor_position(terminal.backend_mut()).unwrap();
        assert_eq!((cursor.x, cursor.y), (1, 20));
    }
}
