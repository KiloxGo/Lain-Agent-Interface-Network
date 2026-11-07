//! Disposable presentation only. Cards never submit work or decide permissions.
use super::*;
use rich::{Document, NoteAction};
use uuid::Uuid;

#[derive(Default, Clone)]
pub(super) struct Timeline {
    pub warm: bool,
    pub watermark: u64,
    pub reconnecting: bool,
    pub seen: BTreeMap<(DeviceId, DeviceCommandId), u64>,
    positions: Vec<(Uuid, Option<Uuid>)>,
    notices_initialized: bool,
    baseline_notices: BTreeSet<lain_wire::NoticeId>,
}
impl Timeline {
    pub fn visible(&self, id: Uuid) -> bool {
        self.positions.iter().any(|(card, _)| *card == id)
    }
    pub fn cards(&self) -> BTreeSet<Uuid> {
        self.positions.iter().map(|(id, _)| *id).collect()
    }
}
// No labels, prompts, output, signatures or credentials survive retirement.
// A checked read must supply the body again before any card can be restored.
pub(super) fn checkpoint(app: &Model) -> Timeline {
    let mut state = app.activity.clone();
    if !app.text_view.doc.entries.is_empty() {
        for (id, after) in app.text_view.doc.note_positions() {
            if let Some(old) = state.positions.iter_mut().find(|(card, _)| *card == id) {
                *old = (id, after);
            } else {
                state.positions.push((id, after));
            }
        }
        if state.positions.len() > 256 {
            state.positions.drain(..state.positions.len() - 256);
        }
    }
    state
}
pub(super) fn restore_positions(app: &mut Model) {
    for (id, after) in app.activity.positions.iter().rev() {
        app.text_view.doc.place_after(*id, *after);
    }
}
pub(super) fn bootstrap_notices(
    app: &mut Model,
    snapshot: &lain_wire::client_sync::BootstrapSnapshot,
) {
    if !app.activity.notices_initialized {
        app.activity.baseline_notices = snapshot.notices.iter().map(|n| n.id).collect();
        app.activity.notices_initialized = true;
    }
}
pub(super) fn notice_reconnect(app: &mut Model) {
    app.activity.notices_initialized = false;
}
fn notice_id(n: &lain_wire::notices::ConversationNotice) -> Uuid {
    Uuid::new_v5(
        &Uuid::NAMESPACE_OID,
        format!("lain/kernel-work/{}", n.action_id).as_bytes(),
    )
}
pub(super) fn quiet_notice(app: &Model, n: &lain_wire::notices::ConversationNotice) -> bool {
    matches!(
        n.outcome,
        lain_wire::notices::NoticeOutcome::Worker {
            state: lain_wire::notices::WorkerOutcome::Completed
                | lain_wire::notices::WorkerOutcome::Cancelled
        }
    ) && app.activity.baseline_notices.contains(&n.id)
        && !app.activity.visible(notice_id(n))
}
pub(super) fn notice(app: &mut Model, n: &lain_wire::notices::ConversationNotice, details: &str) {
    use lain_wire::notices::{NoticeOutcome, WorkerOutcome};
    let quiet = matches!(
        n.outcome,
        NoticeOutcome::Worker {
            state: WorkerOutcome::Completed | WorkerOutcome::Cancelled
        }
    );
    if quiet_notice(app, n) {
        return;
    }
    let title = match n.outcome {
        NoticeOutcome::Worker {
            state: WorkerOutcome::Completed,
        } => "工作 · 已返回结果",
        NoticeOutcome::Worker {
            state: WorkerOutcome::Cancelled,
        } => "工作 · 已取消",
        NoticeOutcome::Worker {
            state: WorkerOutcome::Failed,
        } => "工作 · 执行失败",
        _ => "工作 · 状态待核对",
    };
    let summary = if quiet {
        "Kernel Worker 回执；展开查看结果。"
    } else {
        "展开查看原回执；不会自动重试。"
    };
    app.text_view
        .doc
        .note(notice_id(n), title, summary, Some(details), None);
    restore_positions(app);
}

pub(super) fn task_id(c: &DeviceCommand) -> Uuid {
    card_id(c.device_id, c.id)
}
pub(super) fn card_id(device: DeviceId, command: DeviceCommandId) -> Uuid {
    Uuid::new_v5(
        &Uuid::NAMESPACE_OID,
        format!("lain/device-card/{device}/{command}").as_bytes(),
    )
}
pub(super) fn task(doc: &mut Document, c: &DeviceCommand) {
    use lain_wire::device_commands::CommandState::*;
    let waiting = c
        .execution
        .as_ref()
        .is_some_and(|e| !e.pending_approvals.is_empty());
    let state = if waiting {
        "需要处理权限"
    } else if c.cancel_requested && !c.state.terminal() {
        "正在请求取消"
    } else {
        match c.state {
            Succeeded if c.execution.as_ref().is_some_and(|e| e.result.is_some()) => "已返回结果",
            Succeeded => "设备报告已结束",
            Failed => "执行失败",
            Cancelled => "已取消",
            Rejected => "派工被拒绝",
            Unknown => "状态待核对",
            Expired => "任务已过期",
            Revoked => "设备授权已撤销",
            Queued => "等待执行",
            Accepted => "设备已接收",
            Running => "执行中",
        }
    };
    let title = c
        .input
        .task
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(76)
        .collect::<String>();
    let mut summary = format!(
        "{title}\n{}",
        match c.input.sandbox {
            lain_wire::Sandbox::ReadOnly => "只读",
            lain_wire::Sandbox::WorkspaceWrite => "工作区写入",
        }
    );
    if let Some(result) = c.execution.as_ref().and_then(|e| e.result.as_ref()) {
        if result.history_truncated {
            summary.push_str(" · 回传含节选");
        }
        if result.command_failures > 0 {
            summary.push_str(&format!(" · {} 条命令错误记录", result.command_failures));
        }
    }
    if c.state == Unknown {
        summary.push_str(" · 不会自动重发");
    }
    doc.note(
        task_id(c),
        &format!("Codex · {state}"),
        &summary,
        None,
        Some(NoteAction::Task(c.device_id, c.id)),
    );
}
pub(super) fn metadata(doc: &mut Document, turn: &ActorSnapshot, details: &str) {
    if details.trim().is_empty() {
        return;
    }
    let count = |state| turn.actions.iter().filter(|a| a.state == state).count();
    let missing = turn
        .actions
        .iter()
        .filter(|a| crate::actor_render::missing_memory_read(a))
        .count();
    let failed = count(ActionState::Failed) - missing;
    let (title, summary) = if let Some(error) = &turn.error {
        ("对话异常", safe_text(error).chars().take(180).collect())
    } else if count(ActionState::Unknown) > 0 {
        ("操作结果待核对", "请查看回执；不会自动重试。".into())
    } else if count(ActionState::Proposed) > 0 {
        ("需要确认", "F4 审阅具体操作；此提示不会执行操作。".into())
    } else if failed > 0 {
        (
            "工具执行异常",
            format!(
                "{failed} 项失败{}，展开查看。",
                if missing > 0 {
                    format!(" · {missing} 项未找到或不可见")
                } else {
                    String::new()
                }
            ),
        )
    } else if missing > 0 {
        (
            "记忆查询 · 未找到或不可见",
            format!(
                "{missing} 项未找到或不可见 · {} 项已处理{}；展开查看原回执。",
                count(ActionState::Executed) + count(ActionState::Rejected),
                if count(ActionState::Executing) > 0 {
                    " · 仍有操作处理中"
                } else {
                    ""
                }
            ),
        )
    } else {
        (
            "工具动态",
            format!(
                "{} 项已处理{}",
                count(ActionState::Executed) + count(ActionState::Rejected),
                if count(ActionState::Executing) > 0 {
                    " · 仍有操作处理中"
                } else {
                    ""
                }
            ),
        )
    };
    doc.note(
        Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            format!("lain/actor-tools/{}", turn.id).as_bytes(),
        ),
        title,
        &summary,
        Some(details),
        None,
    );
}
pub(super) fn activate(app: &mut Model, id: Uuid, version: u64, action: NoteAction) {
    if app.text_view.doc.version(id) != Some(version) {
        app.status = "动态已更新，请重新选择".into();
        return;
    }
    match action {
        NoteAction::Fold(_) => app.text_view.doc.toggle(id),
        NoteAction::Task(device, command) => {
            let record = app
                .remote_work
                .iter()
                .find(|r| r.command.device_id == device && r.command.id == command)
                .cloned();
            if let Some(record) = record {
                app.navigate(Surface::Task(device, command));
                app.task = Some(record.command);
                app.task_decisions = None;
            } else {
                app.status = "该任务不在当前快照中；请到 Work 刷新查看".into();
            }
        }
    }
}
pub(super) fn prepare_output(app: &mut Model) {
    work_history::prepare(app);
}
