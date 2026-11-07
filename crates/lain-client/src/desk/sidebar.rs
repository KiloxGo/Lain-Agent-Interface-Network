//! Read-only operational summaries. Local telemetry is never model context.
use super::*;
use crate::api::Update;
#[derive(Default)]
pub(super) struct Workers {
    pub connected: bool,
    cursor: u64,
    orders: BTreeMap<uuid::Uuid, u64>,
    pub sessions: BTreeMap<uuid::Uuid, Session>,
    pub truncated: bool,
}
pub(super) fn active(state: &str) -> bool {
    matches!(
        state,
        "created" | "starting" | "running" | "waiting_for_approval"
    )
}
impl Workers {
    pub fn apply(&mut self, update: Update) -> Result<()> {
        match update {
            Update::Snapshot(mut snapshot) => {
                self.cursor = snapshot.history_cursor;
                self.orders.clear();
                snapshot.sessions.sort_by_key(|s| (!active(&s.state), s.id));
                self.truncated = snapshot.sessions.len() > 512;
                self.sessions = snapshot
                    .sessions
                    .into_iter()
                    .take(512)
                    .map(|s| (s.id, s))
                    .collect();
                self.connected = true;
            }
            Update::Offline(_) => {
                self.connected = false;
                self.sessions.clear();
                self.orders.clear();
            }
            Update::Event(event) => {
                if !self.connected {
                    return Ok(());
                }
                let Some(order) = event.history_order else {
                    return Ok(());
                };
                if order
                    <= self
                        .cursor
                        .max(*self.orders.get(&event.work_session_id).unwrap_or(&0))
                {
                    return Ok(());
                }
                match event.kind.as_str() {
                    "session_created" => {
                        let session: Session =
                            serde_json::from_value(event.field("session").clone())?;
                        ensure!(
                            session.id == event.work_session_id,
                            "Worker event identity changed"
                        );
                        if self.sessions.len() < 512 {
                            self.sessions.insert(session.id, session);
                        } else {
                            self.truncated = true;
                        }
                    }
                    "session_state_changed" => {
                        if let Some(session) = self.sessions.get_mut(&event.work_session_id) {
                            session.state = event.field("to").as_str().unwrap_or("unknown").into();
                        }
                    }
                    _ => return Ok(()),
                }
                if self.sessions.contains_key(&event.work_session_id) {
                    self.orders.insert(event.work_session_id, order);
                }
            }
        }
        Ok(())
    }
}

pub(super) fn rows(app: &Model, width: u16, height: u16) -> Vec<String> {
    let info = &app.local_info;
    let mut rows = vec![
        "── 本机 ───────────────────".into(),
        safe_text(&info.system),
        format!(
            "CPU  {} 逻辑核",
            info.logical_cpus
                .map_or_else(|| "未知".into(), |n| n.to_string())
        ),
        format!(
            "RAM  {}",
            info.memory.map_or_else(|| "暂不可用".into(), |m| m.label())
        ),
    ];
    if height < 14 {
        let count = if app.worker_status.connected && app.remote_connected {
            task_rows(app).len().to_string()
        } else {
            "待同步".into()
        };
        rows.push(format!("任务 {count} · Tab 查看 Work"));
        return rows;
    }
    if height >= 19 {
        rows.extend([
            String::new(),
            "── 会话 ───────────────────".into(),
            format!("ID  {}", draw::short(app.conversation)),
        ]);
    }
    rows.push("本机目录".into());
    let path = info.directory.as_deref().unwrap_or("不可用");
    rows.extend(short_lines(path, width, 2));
    rows.push("任务工作区".into());
    let workspace = match app.surface {
        Surface::Work => app
            .sessions
            .get(app.selected)
            .map(|s| format!("Kernel: {}", s.workspace))
            .or_else(|| {
                app.selected
                    .checked_sub(app.sessions.len())
                    .and_then(|i| app.remote_heads().nth(i))
                    .map(|r| work_history::workspace(app, &r.command))
            }),
        Surface::Task(..) => app.task.as_ref().map(|t| work_history::workspace(app, t)),
        _ => None,
    }
    .or_else(|| app.workspace_context.clone())
    .unwrap_or_else(|| "未选择 · 纯聊天".into());
    rows.extend(short_lines(&workspace, width, 2));
    rows.extend([String::new(), "── 进行中的任务 ───────────".into()]);
    let tasks = task_rows(app);
    if tasks.is_empty() {
        rows.push(
            if app.worker_status.connected && app.remote_connected {
                "暂无进行中的任务"
            } else {
                "任务状态同步中"
            }
            .into(),
        );
    } else {
        let available = usize::from(height).saturating_sub(rows.len()).max(1);
        let count = tasks.len();
        let show = count.min(available.saturating_sub(usize::from(count > available)));
        for task in tasks.into_iter().take(show) {
            rows.extend(short_lines(&task, width, 1));
        }
        if show < count {
            rows.push(format!("另 {} 项 · Tab → Work", count - show));
        }
    }
    if !app.worker_status.connected || !app.remote_connected || app.worker_status.truncated {
        rows.push("部分状态待同步 / 列表有界".into());
    }
    rows
}
fn short_lines(text: &str, width: u16, max: usize) -> Vec<String> {
    let mut lines = super::draw::lines(&safe_text(text), width);
    if lines.len() > max {
        lines.truncate(max);
        if let Some(last) = lines.last_mut() {
            while unicode_width::UnicodeWidthStr::width(last.as_str()) + 1
                > usize::from(width.max(1))
            {
                last.pop();
            }
            last.push('…');
        }
    }
    lines
}
fn task_rows(app: &Model) -> Vec<String> {
    let mut tasks = Vec::new();
    if app.pending.as_ref().is_some_and(|p| p.uncertain) {
        tasks.push("◇ Lain · 发送结果待核对".into());
    } else if app
        .turn
        .as_ref()
        .is_some_and(|t| t.state == ActorTurnState::PendingConfirmation)
    {
        tasks.push("◇ Lain · 等待确认 F4".into());
    } else if app.active() || app.pending.is_some() {
        tasks.push("● Lain · 正在回复".into());
    }
    if app.worker_status.connected {
        tasks.extend(
            app.worker_status
                .sessions
                .values()
                .filter(|s| active(&s.state))
                .map(|s| {
                    format!(
                        "Kernel {} {} {}",
                        safe_text(&s.backend).chars().take(6).collect::<String>(),
                        &draw::short(s.id)[..4],
                        match s.state.as_str() {
                            "created" => "已创建",
                            "starting" => "启动中",
                            "running" => "运行中",
                            "waiting_for_approval" => "待审批",
                            _ => "状态未知",
                        }
                    )
                }),
        );
    }
    if app.remote_connected {
        tasks.extend(
            app.remote_heads()
                .filter(|r| !r.command.state.terminal())
                .map(|r| {
                    let c = &r.command;
                    format!("◇ 设备 {} · {:?}", draw::short(c.device_id), c.state)
                }),
        );
    }
    tasks
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pending_approval_is_waiting_not_an_active_reply() {
        let mut app = Model::new(ConversationId::new());
        let mut turn = super::super::tests::running(app.conversation);
        turn.state = ActorTurnState::PendingConfirmation;
        app.turn = Some(turn);
        let rows = task_rows(&app).join("\n");
        assert!(rows.contains("等待确认 F4") && !rows.contains("正在回复"));
    }
    #[test]
    fn offline_and_old_worker_events_cannot_claim_a_current_running_task() {
        let id = uuid::Uuid::new_v4();
        let mut state = Workers::default();
        let session = Session {
            id,
            backend: "codex".into(),
            runtime_profile_id: "codex/local".into(),
            workspace: "/fixture".into(),
            state: "idle".into(),
            runtime: "attached".into(),
        };
        state
            .apply(Update::Snapshot(crate::api::Snapshot {
                history_cursor: 10,
                sessions: vec![session],
                turns: vec![],
                approvals: vec![],
            }))
            .unwrap();
        let event = |order, to| {
            Update::Event(crate::api::Event {
                id: uuid::Uuid::new_v4(),
                history_order: Some(order),
                timestamp_ms: 1,
                work_session_id: id,
                turn_id: None,
                kind: "session_state_changed".into(),
                data: json!({"to":to}).as_object().unwrap().clone(),
                truncated: false,
            })
        };
        state.apply(event(12, "running")).unwrap();
        state.apply(event(11, "idle")).unwrap();
        assert_eq!(state.sessions[&id].state, "running");
        state.apply(Update::Offline("fixture".into())).unwrap();
        state.apply(event(13, "running")).unwrap();
        assert!(!state.connected && state.sessions.is_empty());
    }
}
