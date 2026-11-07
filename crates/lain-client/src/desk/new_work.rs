//! Direct owner dispatch. Device execution stays behind the existing Kernel API.
use super::*;
use lain_wire::{
    Sandbox,
    device_commands::QueueDeviceWork,
    devices::{DeviceCapability, DeviceState, DeviceWorkspace},
};

#[derive(Clone)]
pub(super) struct Target {
    pub device: DeviceSnapshot,
    pub workspace: DeviceWorkspace,
}
#[derive(Clone)]
pub(super) struct Pending {
    pub conversation: ConversationId,
    pub target: Target,
    pub input: QueueDeviceWork,
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(u64::MAX, |d| d.as_millis().try_into().unwrap_or(u64::MAX))
}
fn eligible(d: &DeviceSnapshot) -> bool {
    d.device.state == DeviceState::Active
        && !d.expired
        && d.device.revision > 0
        && d.device.registration.valid()
        && d.device.registration.expires_at_ms > now()
        && d.device
            .registration
            .capabilities
            .contains(&DeviceCapability::CodexManagedWork)
}
fn pending(
    conversation: ConversationId,
    target: Target,
    text: &str,
    sandbox: Sandbox,
    minutes: u32,
) -> Result<Pending> {
    ensure!(
        eligible(&target.device)
            && target
                .device
                .device
                .registration
                .workspaces
                .contains(&target.workspace),
        "执行位置已失效，请重新选择"
    );
    ensure!(
        !text.trim().is_empty() && text.len() <= 8192 && [15, 60, 480].contains(&minutes),
        "任务需为 1–8192 字节，时限需重新选择"
    );
    crate::local::actor_config::non_secret(text, &Default::default())?;
    let input = QueueDeviceWork {
        request_id: RequestId::new(),
        expected_device_revision: target.device.device.revision,
        workspace_id: target.workspace.id,
        profile_id: "codex/local".into(),
        sandbox,
        task: text.into(),
        expires_at_ms: now()
            .saturating_add(u64::from(minutes) * 60000)
            .min(target.device.device.registration.expires_at_ms),
    };
    Ok(Pending {
        conversation,
        target,
        input,
    })
}
fn matches_receipt(p: &Pending, r: &DeviceWorkRecord) -> bool {
    r.origin.conversation_id == p.conversation
        && r.origin.actor_turn_id.is_none()
        && r.origin.action_id.is_none()
        && r.command.device_id == p.target.device.device.registration.id
        && r.command.input == p.input
        && !r.command.id.0.is_nil()
        && r.command.revision > 0
        && r.command.continuation.is_none()
}
pub(super) enum Failure {
    BeforePost(anyhow::Error),
    Uncertain(anyhow::Error),
}
pub(super) struct Preview {
    pub pending: Pending,
    pub text: String,
    pub scroll: u16,
    pub read: bool,
    pub seen_to: usize,
    pub width: u16,
    pub page: u16,
}
pub(super) struct Panel {
    pub targets: Vec<Target>,
    pub selected: usize,
    pub choosing: bool,
    pub input: Composer,
    pub sandbox: Sandbox,
    pub minutes: u32,
    pub preview: Option<Preview>,
}
pub(super) fn close(app: &mut Model) {
    app.new_work_loading = false;
    app.new_work_preselect = None;
    if let Some(p) = app.new_work.take() {
        app.new_work_draft = p.input;
    }
}
fn review(p: Pending) -> Result<Preview> {
    let text = format!(
        "新建 Codex 任务 · 原样发送 Unchanged\n电脑：{}\n项目目录：{}\n\n{}\n\n登记状态不是在线证明。确认后先排队，实际执行由这台电脑的服务和权限决定。\nEnter 不会提交；完整阅读后 Ctrl-Y 确认。",
        safe_text(&p.target.device.device.registration.label),
        safe_text(&p.target.workspace.path),
        serde_json::to_string_pretty(
            &json!({"conversation_id":p.conversation,"device_id":p.target.device.device.registration.id,"request":p.input})
        )?
    );
    Ok(Preview {
        pending: p,
        text,
        scroll: 0,
        read: false,
        seen_to: 0,
        width: 0,
        page: 1,
    })
}
pub(super) fn open(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
) -> Result<()> {
    ensure!(!app.new_work_loading, "正在读取执行位置");
    app.navigate(Surface::Work);
    if let Some(p) = app.new_work_pending.clone() {
        app.new_work = Some(Panel {
            targets: vec![p.target.clone()],
            selected: 0,
            choosing: false,
            input: Composer {
                cursor: p.input.task.len(),
                text: p.input.task.clone(),
            },
            sandbox: p.input.sandbox,
            minutes: 60,
            preview: Some(review(p)?),
        });
        app.status = "上次提交结果待核对；Ctrl-R 只读取原请求回执，不会重发".into();
        return Ok(());
    }
    app.new_work_loading = true;
    app.status = "正在读取可选电脑与项目；尚未创建任务".into();
    let c = client.clone();
    let out = live::read_sender(app, tx, jobs);
    let generation = app.generation;
    live::spawn_read(app, jobs, async move {
        let _ = out
            .send(Complete::NewWorkTargets(
                generation,
                c.get("v1/devices").await,
            ))
            .await;
    });
    Ok(())
}
pub(super) fn open_for(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
    device: DeviceId,
    workspace: lain_wire::DeviceWorkspaceId,
) -> Result<()> {
    open(client, app, tx, jobs)?;
    // An uncertain existing request takes precedence over a new destination.
    if app.new_work_pending.is_none() {
        app.new_work_preselect = Some((device, workspace));
    }
    Ok(())
}
pub(super) fn opened(app: &mut Model, devices: Vec<DeviceSnapshot>) {
    let mut targets: Vec<Target> = devices
        .into_iter()
        .filter(eligible)
        .flat_map(|d| {
            d.device
                .registration
                .workspaces
                .clone()
                .into_iter()
                .map(move |workspace| Target {
                    device: d.clone(),
                    workspace,
                })
        })
        .take(512)
        .collect();
    let preselect = app.new_work_preselect;
    let selected = preselect.and_then(|(device, workspace)| {
        targets
            .iter()
            .position(|t| t.device.device.registration.id == device && t.workspace.id == workspace)
    });
    let missing = preselect.is_some() && selected.is_none();
    if missing {
        // Never silently dispatch to a different computer after a stale detail view.
        targets.clear();
    }
    app.new_work_loading = false;
    app.new_work = Some(Panel {
        targets,
        selected: selected.unwrap_or(0),
        choosing: true,
        input: std::mem::take(&mut app.new_work_draft),
        sandbox: Sandbox::ReadOnly,
        minutes: 60,
        preview: None,
    });
    app.status = if missing {
        "原选电脑或目录已不可用；草稿保留，Esc 返回后重新选择。未改选其他电脑。"
    } else if preselect.is_some() {
        "已选中该电脑和项目；核对后 Enter 编写任务，尚未创建任务"
    } else {
        "选择执行电脑和项目；Chat 草稿独立保留"
    }
    .into();
}
pub(super) fn recover(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
) {
    let Some(p) = app.new_work_pending.clone().filter(|_| !app.new_work_busy) else {
        return;
    };
    app.new_work_busy = true;
    let c = client.clone();
    let out = live::effect_sender(app, tx, jobs);
    jobs.spawn(async move {
        let path = format!(
            "v1/conversations/{}/device-work/{}/requests/{}",
            p.conversation, p.target.device.device.registration.id, p.input.request_id
        );
        let result = c.get(&path).await.map_err(Failure::Uncertain);
        let _ = out.send(Complete::NewWorkSent(Box::new((p, result)))).await;
    });
}
pub(super) fn key(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
    key: KeyEvent,
) -> Result<()> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    if key.code == KeyCode::Esc || key.code == KeyCode::Tab {
        if app.new_work_pending.is_none()
            && key.code == KeyCode::Esc
            && let Some(panel) = app.new_work.as_mut()
        {
            if panel.preview.take().is_some() {
                return Ok(());
            }
            if !panel.choosing {
                panel.choosing = true;
                return Ok(());
            }
        }
        close(app);
        if key.code == KeyCode::Tab {
            app.back();
        }
        return Ok(());
    }
    if ctrl && key.code == KeyCode::Char('r') {
        if app.new_work_pending.is_some() {
            recover(client, app, tx, jobs);
        } else {
            let target = app.new_work_preselect;
            close(app);
            open(client, app, tx, jobs)?;
            app.new_work_preselect = target;
        }
        return Ok(());
    }
    if app.new_work_pending.is_some() {
        return Ok(());
    }
    let panel = app.new_work.as_mut().context("没有打开任务输入")?;
    if let Some(r) = panel.preview.as_mut() {
        match key.code {
            KeyCode::PageDown => r.scroll = r.scroll.saturating_add(r.page),
            KeyCode::PageUp => r.scroll = r.scroll.saturating_sub(r.page),
            KeyCode::Char('y') if ctrl && r.read && !app.new_work_busy => {
                let p = r.pending.clone();
                let c = client.clone();
                let out = live::effect_sender(app, tx, jobs);
                app.new_work_pending = Some(p.clone());
                app.new_work_busy = true;
                jobs.spawn(async move {
                    let result = async {
                        // Do not submit to an older Kernel that cannot recover a
                        // lost response by the exact original request identity.
                        let receipt_path = format!(
                            "v1/conversations/{}/device-work/{}/requests/{}",
                            p.conversation,
                            p.target.device.device.registration.id,
                            p.input.request_id
                        );
                        let prior: Option<DeviceWorkRecord> = c
                            .get(&receipt_path)
                            .await
                            .context("服务端需支持原请求回执查询；本次尚未提交任务")
                            .map_err(Failure::BeforePost)?;
                        if prior.is_some() {
                            return Ok(prior);
                        }
                        let d: DeviceSnapshot = c
                            .get(&format!(
                                "v1/devices/{}",
                                p.target.device.device.registration.id
                            ))
                            .await
                            .map_err(Failure::BeforePost)?;
                        if !eligible(&d)
                            || d.device != p.target.device.device
                            || p.input.expires_at_ms <= now()
                        {
                            return Err(Failure::BeforePost(anyhow::anyhow!(
                                "执行位置已变化或任务预览已到期，请刷新后重新确认"
                            )));
                        }
                        let command: DeviceCommand = c
                            .post(
                                &format!(
                                    "v1/conversations/{}/device-work/{}",
                                    p.conversation, p.target.device.device.registration.id
                                ),
                                Some(serde_json::to_value(&p.input).expect("typed work input")),
                            )
                            .await
                            .map_err(Failure::Uncertain)?;
                        Ok(Some(DeviceWorkRecord {
                            origin: lain_wire::device_work::DeviceWorkOrigin {
                                conversation_id: p.conversation,
                                actor_turn_id: None,
                                action_id: None,
                            },
                            command,
                        }))
                    }
                    .await;
                    let _ = out.send(Complete::NewWorkSent(Box::new((p, result)))).await;
                });
            }
            _ => {}
        }
    } else if panel.choosing {
        match key.code {
            KeyCode::Up => panel.selected = panel.selected.saturating_sub(1),
            KeyCode::Down => {
                panel.selected = (panel.selected + 1).min(panel.targets.len().saturating_sub(1))
            }
            KeyCode::Enter if !panel.targets.is_empty() => panel.choosing = false,
            _ => {}
        }
    } else {
        match key.code {
            KeyCode::F(5) => {
                panel.sandbox = match panel.sandbox {
                    Sandbox::ReadOnly => Sandbox::WorkspaceWrite,
                    Sandbox::WorkspaceWrite => Sandbox::ReadOnly,
                }
            }
            KeyCode::F(7) => {
                panel.minutes = match panel.minutes {
                    15 => 60,
                    60 => 480,
                    _ => 15,
                }
            }
            KeyCode::Enter
                if !key.modifiers.intersects(
                    KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT,
                ) =>
            {
                let target = panel
                    .targets
                    .get(panel.selected)
                    .cloned()
                    .context("请重新选择执行位置")?;
                panel.preview = Some(review(pending(
                    app.conversation,
                    target,
                    &panel.input.text,
                    panel.sandbox,
                    panel.minutes,
                )?)?);
            }
            _ => panel.input.key(key),
        }
    }
    Ok(())
}
pub(super) fn complete(
    app: &mut Model,
    p: Pending,
    result: std::result::Result<Option<DeviceWorkRecord>, Failure>,
) -> Result<()> {
    if app.new_work_pending.as_ref().is_none_or(|old| {
        old.input.request_id != p.input.request_id || old.conversation != p.conversation
    }) {
        return Ok(());
    }
    app.new_work_busy = false;
    match result {
        Ok(Some(r)) if matches_receipt(&p, &r) => {
            let open_task = app.new_work.is_some() && app.surface == Surface::Work;
            app.new_work_pending = None;
            close(app);
            if app.new_work_draft.text == p.input.task {
                app.new_work_draft = Composer::default();
            }
            app.status = "任务已排队；等待本机执行回执，可以继续与 Lain 聊天".into();
            if app.connected && app.conversation == p.conversation {
                let device = r.command.device_id;
                let id = r.command.id;
                if !app.remote_work.iter().any(|old| old.command.id == id) {
                    app.remote_work.push(r);
                }
                if open_task {
                    app.navigate(Surface::Task(device, id));
                }
            }
        }
        Err(Failure::BeforePost(e)) => {
            app.new_work_pending = None;
            if let Some(panel) = app.new_work.as_mut() {
                panel.preview = None;
            }
            app.status = format!("尚未提交任务：{e}");
        }
        Err(Failure::Uncertain(e)) => {
            app.status = format!("提交结果待核对：{e}；Ctrl-R 读取原请求回执，不会重发")
        }
        _ => app.status = "尚未找到匹配回执；保留原请求，Ctrl-R 再读，不创建替代任务".into(),
    }
    Ok(())
}
pub(super) fn draw(
    frame: &mut ratatui::Frame,
    app: &mut Model,
    area: ratatui::layout::Rect,
    input_area: ratatui::layout::Rect,
) {
    use ratatui::widgets::Paragraph;
    let panel = app.new_work.as_mut().unwrap();
    if let Some(r) = panel.preview.as_mut() {
        let end = draw::body(
            frame,
            area,
            &r.text,
            &mut r.scroll,
            false,
            " 新建任务 · 审阅原话与执行范围 · PgUp/PgDn ",
        );
        let width = area.width.saturating_sub(2);
        r.page = area.height.saturating_sub(2).max(1);
        if width != r.width {
            r.width = width;
            r.seen_to = 0;
            r.read = false;
        }
        if usize::from(r.scroll) <= r.seen_to {
            r.seen_to = r.seen_to.max(usize::from(r.scroll) + usize::from(r.page));
            r.read |= end;
        }
        let hint = if app.new_work_pending.is_some() {
            "提交结果待核对 · Ctrl-R 只读回执 · Esc 关闭"
        } else if r.read {
            "Ctrl-Y 确认创建 · Esc 返回修改 · Tab 回到 Lain"
        } else {
            "完整阅读后才能创建 · PgDn 翻页 · Esc 返回修改"
        };
        frame.render_widget(Paragraph::new(hint), input_area);
    } else if panel.choosing {
        let text = if panel.targets.is_empty() {
            "还没有可派工的项目。\n\n需要先登记这台电脑和项目目录，再显式启动本机执行服务。\nDevices 用于查看设备登记与任务；安装了 Codex 不等于已连接 Lain。\n\n连接后 Ctrl-R 重新读取。Esc 关闭，Tab 回到 Lain。".into()
        } else {
            format!(
                "选择电脑与项目 · 当前支持 Codex\n\n{}\n\n登记信息不保证在线；没有本机服务时任务会等待。\n最多展示 512 个位置；↑/↓ 选择，Enter 输入任务。",
                panel
                    .targets
                    .iter()
                    .enumerate()
                    .map(|(i, t)| format!(
                        "{} {} · {}",
                        if i == panel.selected { ">" } else { " " },
                        safe_text(&t.device.device.registration.label),
                        safe_text(&t.workspace.path)
                    ))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };
        let mut scroll = panel
            .selected
            .saturating_sub(usize::from(area.height.saturating_sub(8)))
            .min(u16::MAX as usize) as u16;
        draw::body(
            frame,
            area,
            &text,
            &mut scroll,
            false,
            " Work · 新建任务 · 1/2 执行位置 ",
        );
        frame.render_widget(
            Paragraph::new("↑/↓ 选择 · Enter 下一步 · Ctrl-R 刷新 · Esc 关闭 · Tab 回到 Lain"),
            input_area,
        );
    } else {
        let t = &panel.targets[panel.selected];
        let mut scroll = 0;
        let text = format!(
            "Agent：Codex\n电脑：{}\n项目：{}\n权限：{:?}（F5 切换；本机策略仍会检查）\n时限：{} 分钟（F7 切换 15/60/480）\n\n输入会原样发送给 Codex，不调用 Lain 改写。\nEnter 先预览，完整阅读后 Ctrl-Y 才创建。\nChat 草稿独立保留；Esc 返回选择位置。",
            safe_text(&t.device.device.registration.label),
            safe_text(&t.workspace.path),
            panel.sandbox,
            panel.minutes
        );
        draw::body(
            frame,
            area,
            &text,
            &mut scroll,
            false,
            " Work · 新建任务 · 2/2 任务内容 ",
        );
        draw::composer_with_title(
            frame,
            input_area,
            &panel.input,
            " 给 Codex · F9 粘贴 · Enter 预览 · Ctrl-J 换行 ",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    fn target() -> Target {
        let device: DeviceSnapshot = serde_json::from_value(json!({
            "device":{"registration":{"id":DeviceId::new(),"request_id":RequestId::new(),"label":"Test PC","platform":"windows","credential_fingerprint":"a".repeat(64),
                "workspaces":[{"id":lain_wire::DeviceWorkspaceId::new(),"path":"D:\\fixture"}],"capabilities":["codex_managed_work"],"expires_at_ms":now()+3600000},
                "state":"active","revision":1,"registered_at_ms":now(),"claim_until_ms":now()+1000,"claimed_at_ms":now(),"claim_request_id":RequestId::new(),"revoked_at_ms":null},
            "expired":false,"latest_report":null
        })).unwrap();
        Target {
            workspace: device.device.registration.workspaces[0].clone(),
            device,
        }
    }
    fn record(p: &Pending) -> DeviceWorkRecord {
        serde_json::from_value(json!({"origin":{"conversation_id":p.conversation,"actor_turn_id":null,"action_id":null},
            "command":{"id":DeviceCommandId::new(),"device_id":p.target.device.device.registration.id,"input":p.input,"created_at_ms":now(),"revision":1,"state":"queued","cancel_requested":false,"interruption":null,"last_event":null}})).unwrap()
    }
    #[test]
    fn work_review_and_connection_loss_preserve_chat_and_task_drafts_without_submission() {
        use ratatui::{Terminal, backend::TestBackend};
        let mut app = Model::new(ConversationId::new());
        app.input.insert("给 Lain 的草稿");
        app.navigate(Surface::Work);
        app.new_work_draft.insert("给 Codex 的任务");
        opened(&mut app, vec![target().device]);
        let panel = app.new_work.as_mut().unwrap();
        panel.choosing = false;
        panel.preview = Some(
            review(
                pending(
                    app.conversation,
                    panel.targets[0].clone(),
                    &panel.input.text,
                    Sandbox::ReadOnly,
                    60,
                )
                .unwrap(),
            )
            .unwrap(),
        );
        assert!(app.new_work_pending.is_none());
        assert!(
            !app.new_work
                .as_ref()
                .unwrap()
                .preview
                .as_ref()
                .unwrap()
                .read
        );
        let mut terminal = Terminal::new(TestBackend::new(130, 55)).unwrap();
        terminal
            .draw(|frame| draw::render(frame, &mut app))
            .unwrap();
        assert!(
            app.new_work
                .as_ref()
                .unwrap()
                .preview
                .as_ref()
                .unwrap()
                .read
        );
        let p = app
            .new_work
            .as_ref()
            .unwrap()
            .preview
            .as_ref()
            .unwrap()
            .pending
            .clone();
        app.new_work_pending = Some(p.clone());
        live::retire(&mut app);
        assert!(app.new_work.is_none());
        assert_eq!(app.new_work_draft.text, "给 Codex 的任务");
        assert_eq!(app.input.text, "给 Lain 的草稿");
        assert_eq!(app.new_work_pending.as_ref().unwrap().input, p.input);
    }
    #[test]
    fn late_or_unconfirmed_receipts_keep_chat_and_never_replace_the_request() {
        let mut app = Model::new(ConversationId::new());
        app.input.insert("chat draft");
        let p = pending(app.conversation, target(), "task", Sandbox::ReadOnly, 60).unwrap();
        app.new_work_pending = Some(p.clone());
        app.new_work_draft.insert("task");
        app.new_work_busy = true;
        complete(&mut app, p.clone(), Ok(None)).unwrap();
        assert_eq!(
            app.new_work_pending.as_ref().unwrap().input.request_id,
            p.input.request_id
        );
        assert!(!app.new_work_busy);
        complete(&mut app, p.clone(), Ok(Some(record(&p)))).unwrap();
        assert_eq!(app.surface, Surface::Chat);
        assert_eq!(app.input.text, "chat draft");
        assert!(app.new_work_pending.is_none());
        assert_eq!(app.remote_work.len(), 1);
    }
    #[test]
    fn event_paste_targets_only_editable_work_input_and_preserves_chat() {
        let mut app = Model::new(ConversationId::new());
        app.input.insert("chat draft");
        app.navigate(Surface::Work);
        opened(&mut app, vec![target().device]);
        super::super::handle_paste(&mut app, "ignored while choosing");
        assert!(app.new_work.as_ref().unwrap().input.text.is_empty());
        app.new_work.as_mut().unwrap().choosing = false;
        super::super::handle_paste(&mut app, "多行任务\n/不是命令");
        assert_eq!(
            app.new_work.as_ref().unwrap().input.text,
            "多行任务\n/不是命令"
        );
        let panel = app.new_work.as_mut().unwrap();
        panel.preview = Some(
            review(
                pending(
                    app.conversation,
                    panel.targets[0].clone(),
                    &panel.input.text,
                    Sandbox::ReadOnly,
                    60,
                )
                .unwrap(),
            )
            .unwrap(),
        );
        super::super::handle_paste(&mut app, "ignored in review");
        assert_eq!(
            app.new_work.as_ref().unwrap().input.text,
            "多行任务\n/不是命令"
        );
        assert_eq!(app.input.text, "chat draft");
    }
    struct Server {
        device: std::sync::Mutex<DeviceSnapshot>,
        receipt: std::sync::Mutex<Option<DeviceWorkRecord>>,
        posts: std::sync::atomic::AtomicUsize,
        supported: std::sync::atomic::AtomicBool,
    }
    async fn fixture() -> (Client, Arc<Server>, tokio::task::JoinHandle<()>) {
        use axum::{
            Json, Router,
            extract::{Path, State},
            http::StatusCode,
            routing::{get, post},
        };
        use std::sync::{
            Mutex,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        };
        let s = Arc::new(Server {
            device: Mutex::new(target().device),
            receipt: Mutex::new(None),
            posts: AtomicUsize::new(0),
            supported: AtomicBool::new(true),
        });
        let app = Router::new()
            .route(
                "/v1/devices/{device}",
                get(|State(s): State<Arc<Server>>| async move {
                    Json(s.device.lock().unwrap().clone())
                }),
            )
            .route(
                "/v1/conversations/{conversation}/device-work/{device}/requests/{request}",
                get(
                    |State(s): State<Arc<Server>>,
                     Path((conversation, device, request)): Path<(
                        ConversationId,
                        DeviceId,
                        RequestId,
                    )>| async move {
                        if !s.supported.load(Ordering::SeqCst) {
                            return Err(StatusCode::NOT_FOUND);
                        }
                        let r = s.receipt.lock().unwrap().clone().filter(|r| {
                            r.origin.conversation_id == conversation
                                && r.command.device_id == device
                                && r.command.input.request_id == request
                        });
                        Ok(Json(r))
                    },
                ),
            )
            .route(
                "/v1/conversations/{conversation}/device-work/{device}",
                post(
                    |State(s): State<Arc<Server>>,
                     Path((conversation, device)): Path<(ConversationId, DeviceId)>,
                     Json(input): Json<QueueDeviceWork>| async move {
                        s.posts.fetch_add(1, Ordering::SeqCst);
                        let snapshot = s.device.lock().unwrap().clone();
                        let t = Target {
                            workspace: snapshot.device.registration.workspaces[0].clone(),
                            device: snapshot,
                        };
                        let p = Pending {
                            conversation,
                            target: t,
                            input,
                        };
                        let r = record(&p);
                        assert_eq!(r.command.device_id, device);
                        *s.receipt.lock().unwrap() = Some(r);
                        (
                            StatusCode::BAD_GATEWAY,
                            Json(json!({"error":"synthetic lost response"})),
                        )
                    },
                ),
            )
            .with_state(s.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (
            Client::new(&format!("http://{address}"), &"a".repeat(40)).unwrap(),
            s,
            server,
        )
    }
    fn prepared(s: &Server) -> Model {
        use ratatui::{Terminal, backend::TestBackend};
        let mut app = Model::new(ConversationId::new());
        app.input.insert("Lain draft stays");
        app.navigate(Surface::Work);
        opened(&mut app, vec![s.device.lock().unwrap().clone()]);
        let panel = app.new_work.as_mut().unwrap();
        panel.choosing = false;
        panel.input.insert(" original\nCodex task ");
        panel.preview = Some(
            review(
                pending(
                    app.conversation,
                    panel.targets[0].clone(),
                    &panel.input.text,
                    Sandbox::ReadOnly,
                    60,
                )
                .unwrap(),
            )
            .unwrap(),
        );
        let mut terminal = Terminal::new(TestBackend::new(130, 60)).unwrap();
        terminal.draw(|f| draw::render(f, &mut app)).unwrap();
        assert!(
            app.new_work
                .as_ref()
                .unwrap()
                .preview
                .as_ref()
                .unwrap()
                .read
        );
        app
    }
    async fn completion(app: &mut Model, rx: &mut mpsc::Receiver<Complete>) {
        let result = tokio::time::timeout(Duration::from_secs(4), rx.recv())
            .await
            .unwrap()
            .unwrap();
        if let Complete::NewWorkSent(result) = result {
            let (p, r) = *result;
            complete(app, p, r).unwrap();
        } else {
            panic!("wrong completion");
        }
    }
    #[tokio::test]
    async fn lost_http_response_recovers_exact_request_without_post_or_chat_interruption() {
        use std::sync::atomic::Ordering;
        let (client, s, server) = fixture().await;
        let mut app = prepared(&s);
        let (tx, mut rx) = mpsc::channel(8);
        let mut jobs = JoinSet::new();
        app.new_work
            .as_mut()
            .unwrap()
            .preview
            .as_mut()
            .unwrap()
            .read = false;
        key(
            &client,
            &mut app,
            &tx,
            &mut jobs,
            KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
        )
        .unwrap();
        assert!(app.new_work_pending.is_none());
        assert_eq!(s.posts.load(Ordering::SeqCst), 0);
        app.new_work
            .as_mut()
            .unwrap()
            .preview
            .as_mut()
            .unwrap()
            .read = true;
        key(
            &client,
            &mut app,
            &tx,
            &mut jobs,
            KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
        )
        .unwrap();
        completion(&mut app, &mut rx).await;
        let p = app.new_work_pending.clone().unwrap();
        assert_eq!(s.posts.load(Ordering::SeqCst), 1);
        assert_eq!(p.input.task, " original\nCodex task ");
        app.back();
        recover(&client, &mut app, &tx, &mut jobs);
        completion(&mut app, &mut rx).await;
        assert!(app.new_work_pending.is_none());
        assert_eq!(s.posts.load(Ordering::SeqCst), 1);
        assert_eq!(app.surface, Surface::Chat);
        assert_eq!(app.input.text, "Lain draft stays");
        assert_eq!(app.remote_work.len(), 1);
        jobs.shutdown().await;
        server.abort();
    }
    #[tokio::test]
    async fn work_key_opens_the_form_without_dispatching_or_touching_chat() {
        use std::sync::atomic::Ordering;
        for letter in ['n', 'N'] {
            let (client, s, server) = fixture().await;
            let mut app = Model::new(ConversationId::new());
            app.input.insert("chat draft");
            app.navigate(Surface::Work);
            let (tx, _rx) = mpsc::channel(8);
            let mut jobs = JoinSet::new();
            super::super::handle_key(
                &client,
                &mut app,
                &tx,
                &mut jobs,
                KeyEvent::new(KeyCode::Char(letter), KeyModifiers::NONE),
            )
            .unwrap();
            assert!(app.new_work_loading);
            assert!(app.new_work_pending.is_none());
            assert_eq!(app.input.text, "chat draft");
            assert_eq!(s.posts.load(Ordering::SeqCst), 0);
            jobs.shutdown().await;
            server.abort();
        }
    }
    #[tokio::test]
    async fn stale_target_or_old_server_prevents_every_dispatch_post() {
        use std::sync::atomic::Ordering;
        for changed_target in [true, false] {
            let (client, s, server) = fixture().await;
            let mut app = prepared(&s);
            let (tx, mut rx) = mpsc::channel(8);
            let mut jobs = JoinSet::new();
            if changed_target {
                s.device.lock().unwrap().device.revision += 1;
            } else {
                s.supported.store(false, Ordering::SeqCst);
            }
            key(
                &client,
                &mut app,
                &tx,
                &mut jobs,
                KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
            )
            .unwrap();
            completion(&mut app, &mut rx).await;
            assert_eq!(s.posts.load(Ordering::SeqCst), 0);
            assert!(app.new_work_pending.is_none());
            assert!(app.new_work.as_ref().unwrap().preview.is_none());
            assert_eq!(
                app.new_work.as_ref().unwrap().input.text,
                " original\nCodex task "
            );
            jobs.shutdown().await;
            server.abort();
        }
    }
    #[test]
    fn task_details_keep_the_tab_of_the_entry_that_opened_them() {
        let mut app = Model::new(ConversationId::new());
        let device = DeviceId::new();
        app.navigate(Surface::Work);
        app.navigate(Surface::Task(device, DeviceCommandId::new()));
        assert!(app.task_from_work);
        app.navigate(Surface::Task(device, DeviceCommandId::new()));
        assert!(app.task_from_work);
        app.navigate(Surface::Tasks(device));
        app.navigate(Surface::Task(device, DeviceCommandId::new()));
        assert!(!app.task_from_work);
    }
    #[test]
    fn selection_requires_active_unexpired_declared_codex_work_but_not_a_false_online_claim() {
        let mut t = target();
        assert!(eligible(&t.device));
        t.device.expired = true;
        assert!(!eligible(&t.device));
        t.device.expired = false;
        t.device.device.state = DeviceState::Revoked;
        assert!(!eligible(&t.device));
        t.device.device.state = DeviceState::Active;
        t.device.device.registration.capabilities.clear();
        assert!(!eligible(&t.device));
    }
    #[test]
    fn preview_keeps_exact_task_scope_and_never_uses_the_lain_draft() {
        let conversation = ConversationId::new();
        let t = target();
        let p = pending(
            conversation,
            t.clone(),
            "  read README\n保留原话  ",
            Sandbox::ReadOnly,
            60,
        )
        .unwrap();
        assert_eq!(p.input.task, "  read README\n保留原话  ");
        assert_eq!(p.input.sandbox, Sandbox::ReadOnly);
        assert_eq!(p.input.expected_device_revision, t.device.device.revision);
        assert_eq!(p.input.workspace_id, t.workspace.id);
        assert!(p.input.expires_at_ms <= t.device.device.registration.expires_at_ms);
        assert!(pending(conversation, t.clone(), "   ", Sandbox::ReadOnly, 60).is_err());
        assert!(
            pending(
                conversation,
                t.clone(),
                &"x".repeat(8193),
                Sandbox::ReadOnly,
                60
            )
            .is_err()
        );
        let mut wrong = t.clone();
        wrong.workspace.id = lain_wire::DeviceWorkspaceId::new();
        assert!(pending(conversation, wrong, "task", Sandbox::ReadOnly, 60).is_err());
    }
    #[test]
    fn recovered_receipt_must_match_original_request_target_scope_and_conversation() {
        let p = pending(
            ConversationId::new(),
            target(),
            "task",
            Sandbox::ReadOnly,
            60,
        )
        .unwrap();
        let mut record:DeviceWorkRecord=serde_json::from_value(json!({"origin":{"conversation_id":p.conversation,"actor_turn_id":null,"action_id":null},
            "command":{"id":DeviceCommandId::new(),"device_id":p.target.device.device.registration.id,"input":p.input,"created_at_ms":now(),"revision":1,"state":"queued","cancel_requested":false,"interruption":null,"last_event":null}})).unwrap();
        assert!(matches_receipt(&p, &record));
        record.command.input.task.push_str("changed");
        assert!(!matches_receipt(&p, &record));
        record.command.input = p.input.clone();
        record.origin.conversation_id = ConversationId::new();
        assert!(!matches_receipt(&p, &record));
    }
}
