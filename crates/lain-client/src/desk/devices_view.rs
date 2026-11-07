//! Read-only computer inventory. Registration and inventory reports are not heartbeats.
use super::*;
use lain_wire::devices::{DeviceCapability, DevicePlatform, DeviceState};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Text},
    widgets::{Block, Borders, List, ListItem, ListState},
};

#[derive(Default, PartialEq, Eq)]
pub(super) enum Page {
    #[default]
    Overview,
    Startup,
    Diagnostics,
}
pub(super) fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis().try_into().unwrap_or(u64::MAX))
}
fn platform(d: &DeviceSnapshot) -> &'static str {
    match d.device.registration.platform {
        DevicePlatform::Windows => "Windows",
        DevicePlatform::Macos => "macOS",
        DevicePlatform::Linux => "Linux",
    }
}
fn authority(d: &DeviceSnapshot, now: u64) -> &'static str {
    match d.device.state {
        DeviceState::Revoked => "授权已撤销",
        _ if d.expired || d.device.registration.expires_at_ms <= now => "授权已到期",
        DeviceState::Pending => "等待电脑配对",
        DeviceState::Active => "登记有效",
    }
}
fn ago(at: u64, now: u64) -> String {
    if at > now {
        return "时间异常，不能判断新鲜度".into();
    }
    let seconds = (now - at) / 1000;
    match seconds {
        0..=59 => format!("{seconds} 秒前"),
        60..=3599 => format!("{} 分钟前", seconds / 60),
        3600..=86399 => format!("{} 小时前", seconds / 3600),
        _ => format!("{} 天前", seconds / 86400),
    }
}
fn report(d: &DeviceSnapshot, now: u64) -> String {
    d.latest_report.as_ref().map_or_else(
        || "尚无设备回报".into(),
        |r| {
            format!(
                "{} · {}",
                work_history::timestamp(r.received_at_ms),
                ago(r.received_at_ms, now)
            )
        },
    )
}
fn project(path: &str) -> String {
    safe_text(
        path.trim_end_matches(['/', '\\'])
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(path),
    )
}
pub(super) fn selected(app: &Model, id: DeviceId) -> Option<&DeviceSnapshot> {
    app.devices.iter().find(|d| d.device.registration.id == id)
}
pub(super) fn retain_selection(app: &mut Model, rows: &[DeviceSnapshot]) {
    match app.surface {
        Surface::Devices => {
            if let Some(id) = app
                .devices
                .get(app.selected)
                .map(|d| d.device.registration.id)
            {
                app.selected = rows
                    .iter()
                    .position(|d| d.device.registration.id == id)
                    .unwrap_or(rows.len());
            }
        }
        Surface::Device(id) => {
            if let Some(old) = selected(app, id) {
                let workspace = old
                    .device
                    .registration
                    .workspaces
                    .get(app.selected)
                    .map(|w| w.id);
                if let Some(new) = rows.iter().find(|d| d.device.registration.id == id) {
                    app.selected = workspace
                        .and_then(|id| {
                            new.device
                                .registration
                                .workspaces
                                .iter()
                                .position(|w| w.id == id)
                        })
                        .unwrap_or(new.device.registration.workspaces.len());
                }
            }
        }
        _ => {}
    }
}
pub(super) fn list(frame: &mut Frame, app: &mut Model, area: Rect) {
    let now = now();
    let items = app
        .devices
        .iter()
        .map(|d| {
            let r = &d.device.registration;
            ListItem::new(Text::from(vec![
                Line::styled(
                    format!(
                        "{} · {} · {}",
                        safe_text(&r.label),
                        platform(d),
                        authority(d, now)
                    ),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Line::raw(format!(
                    "  {} 个项目目录 · {}",
                    r.workspaces.len(),
                    r.workspaces
                        .first()
                        .map_or_else(|| "无目录".into(), |w| project(&w.path))
                )),
                Line::raw(format!("  最近回报：{}", report(d, now))),
                Line::raw("  执行服务：待确认；Enter 查看详情"),
                Line::raw(""),
            ]))
        })
        .collect::<Vec<_>>();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Devices · 执行电脑 ");
    if items.is_empty() {
        let text = if app.refreshing {
            "正在读取电脑列表…"
        } else if app.device_read_error {
            "无法读取电脑列表。Ctrl-R 重连；不会自动重新配对。"
        } else {
            "还没有登记执行电脑。\n在需要执行 Codex 的电脑上完成配对并启动执行服务。\n这里只管理执行位置；Chat 与云端 Lain 交流，Work 查看和创建任务。\n详见 docs/DEVICE_HOST.md 的配对与启动步骤。"
        };
        frame.render_widget(
            ratatui::widgets::Paragraph::new(text)
                .wrap(ratatui::widgets::Wrap { trim: false })
                .block(block),
            area,
        );
        return;
    }
    app.selected = app.selected.min(items.len());
    let mut state =
        ListState::default().with_selected((app.selected < items.len()).then_some(app.selected));
    frame.render_stateful_widget(
        List::new(items)
            .block(block)
            .highlight_symbol("› ")
            .highlight_style(Style::default().fg(Color::Rgb(121, 185, 189))),
        area,
        &mut state,
    );
}
fn startup(d: &DeviceSnapshot) -> String {
    let command = match d.device.registration.platform {
        DevicePlatform::Windows => {
            "若已使用 Lain-Cloud 安装包，在这台执行电脑的 PowerShell 运行：\n& \"$env:LOCALAPPDATA\\Lain-Cloud\\Start-Codex-Host.cmd\"\n\n此入口仅适用于已安装该启动器的电脑；不是远程启动命令。"
        }
        _ => {
            "在这台执行电脑的终端运行：\nlain-device-agent --directory <设备配置目录>\n\n将占位符替换为配对时的私有配置目录；不要在云服务器上代替本地电脑运行。"
        }
    };
    format!(
        "启动 / 恢复执行服务\n\n{command}\n\n1. 若旧执行服务还在运行，先在它的终端 Ctrl-C，等待退出。\n2. 启动后保持该终端运行；关闭 Lain 界面不会停止它。\n3. 回到 Work 创建一条明确的任务，查看实际回报。\n\n任务一直排队：检查电脑是否开机、网络和执行服务。\n任务中断或 Unknown：先检查原任务回报，不要盲目重发。\n授权到期或已撤销：重新核对配对范围，刷新不能延长授权。\n\n本页只显示说明，不启动、安装或停止任何服务。\nCodex 登录与本机权限需在执行电脑检查；云端不会保存登录凭据。\n更多说明：docs/DEVICE_HOST.md / docs/WORK_DESK.md"
    )
}
pub(super) fn text(app: &Model, id: DeviceId, now: u64) -> String {
    let Some(d) = selected(app, id) else {
        return "这台电脑不在当前列表中，可能已移除或信息尚未读到。\nC 重新读取；Esc 返回电脑列表。不会改选另一台电脑。".into();
    };
    let r = &d.device.registration;
    let header = format!("{} · {}\n\n", safe_text(&r.label), platform(d));
    if app.device_page == Page::Startup {
        return header + &startup(d);
    }
    if app.device_page == Page::Diagnostics {
        return header
            + &format!(
                "设备编号：{}\n登记版本：{}\n登记时间：{}\n状态：{:?}\n\n最近设备信息回报：{}\n回报序号：{}\n回报绑定登记版本：{}\n\n项目目录（登记声明）：\n{}\n\nT 查看这台电脑的任务清单（可能包含其他聊天的任务）。\n该清单不自动成为当前 Chat 的上下文。\n凭据及其指纹不在本页显示。",
                r.id,
                d.device.revision,
                work_history::timestamp(d.device.registered_at_ms),
                d.device.state,
                report(d, now),
                d.latest_report
                    .as_ref()
                    .map_or_else(|| "无".into(), |x| x.report.sequence.to_string()),
                d.latest_report
                    .as_ref()
                    .map_or_else(|| "无".into(), |x| x.device_revision.to_string()),
                r.workspaces
                    .iter()
                    .map(|w| format!("{}\n{}", w.id, safe_text(&w.path)))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            );
    }
    let checked = app
        .device_read_at
        .map_or_else(|| "尚未读取".into(), work_history::timestamp);
    let declared = r.capabilities.contains(&DeviceCapability::CodexManagedWork);
    let version = d
        .latest_report
        .as_ref()
        .and_then(|x| {
            x.report
                .capabilities
                .iter()
                .find(|c| c.capability == DeviceCapability::CodexManagedWork)
        })
        .map_or_else(
            || "尚无版本回报".into(),
            |c| format!("设备曾回报 {}", safe_text(&c.version)),
        );
    let workspace = r.workspaces.get(app.selected);
    let projects = r
        .workspaces
        .iter()
        .map(|w| {
            format!(
                "{} {}",
                if Some(w) == workspace { "›" } else { " " },
                project(&w.path)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let location = workspace.map_or_else(
        || "原选目录已不可用；↑/↓ 重新选择，尚未选中其他目录".into(),
        |w| {
            let availability = match &d.latest_report {
                None => "尚无目录可用性回报",
                Some(report) if report.report.available_workspaces.contains(&w.id) => {
                    "最近设备信息回报曾列为可用；不代表当前可用"
                }
                Some(_) => "最近设备信息回报未列为可用",
            };
            format!("{}\n{}", safe_text(&w.path), availability)
        },
    );
    header
        + &format!(
            "云端信息：{} · 更新于 {}\n执行服务：待确认（尚无当前运行状态）\n最近设备回报：{}\n\n授权：{} · 到期 {}\nCodex：{}；{}\n本机权限：尚未收到本机权限信息，不能据此认定可写文件。\n\n项目目录 · ↑/↓ 选择，N 在此创建任务\n{}\n\n选中目录（电脑登记的路径）\n{}\n\nC 检查云端连接并重读设备信息，不会探测 Codex 登录或启动服务。\n排队不等于执行；实际执行状态与输出到 Work 查看。",
            if app.device_read_error {
                "读取失败，以下为旧快照"
            } else if app.device_read_at.is_none() {
                "尚未读取，以下为缓存"
            } else {
                "最近读取成功"
            },
            checked,
            report(d, now),
            authority(d, now),
            work_history::timestamp(r.expires_at_ms),
            if declared {
                "已登记 Codex 执行能力"
            } else {
                "未声明 Codex 执行能力"
            },
            version,
            projects,
            location
        )
}
pub(super) fn detail(frame: &mut Frame, app: &mut Model, area: Rect, id: DeviceId) {
    if let Some(d) = selected(app, id) {
        app.selected = app.selected.min(d.device.registration.workspaces.len());
    }
    let text = text(app, id, now());
    let (_, bottom) = draw::body_position(
        frame,
        area,
        &text,
        &mut app.scroll,
        false,
        " 电脑详情 · I 启动说明 · F7 诊断 ",
    );
    app.detail_viewport = Some((area.inner(ratatui::layout::Margin::new(1, 1)), bottom));
}

#[cfg(test)]
mod tests {
    use super::*;
    use lain_wire::{DeviceWorkspaceId, devices::DeviceState};
    fn fixture() -> DeviceSnapshot {
        serde_json::from_value(json!({
            "device":{"registration":{"id":DeviceId::new(),"request_id":RequestId::new(),"label":"工作电脑","platform":"windows","credential_fingerprint":"a".repeat(64),
            "workspaces":[{"id":DeviceWorkspaceId::new(),"path":"D:\\projects\\项目一"},{"id":DeviceWorkspaceId::new(),"path":"D:\\projects\\项目二"}],"capabilities":["codex_managed_work"],"expires_at_ms":now()+3600000},
            "state":"active","revision":2,"registered_at_ms":now(),"claim_until_ms":now()+1000,"claimed_at_ms":now(),"claim_request_id":RequestId::new(),"revoked_at_ms":null},
            "expired":false,"latest_report":null
        })).unwrap()
    }
    fn model(d: DeviceSnapshot) -> Model {
        let mut app = Model::new(ConversationId::new());
        app.devices.push(d);
        app.device_read_at = Some(now());
        app
    }
    #[test]
    fn registration_and_old_inventory_never_claim_service_presence_or_write_access() {
        let mut d = fixture();
        let id = d.device.registration.id;
        assert_eq!(authority(&d, now()), "登记有效");
        d.latest_report = Some(serde_json::from_value(json!({"device_id":id,"device_revision":2,"received_at_ms":now()-86400000,
            "report":{"request_id":RequestId::new(),"sequence":1,"reported_at_ms":now()-86400000,"available_workspaces":[d.device.registration.workspaces[0].id],"capabilities":[{"capability":"codex_managed_work","version":"fixture-1"}]}})).unwrap());
        let app = model(d.clone());
        let t = text(&app, id, now());
        for expected in [
            "执行服务：待确认",
            "1 天前",
            "设备曾回报 fixture-1",
            "不能据此认定可写",
            "D:\\projects\\项目一",
        ] {
            assert!(t.contains(expected), "{expected}");
        }
        assert!(!t.contains(&"a".repeat(64)));
        d.device.state = DeviceState::Revoked;
        assert_eq!(authority(&d, now()), "授权已撤销");
        d.device.state = DeviceState::Active;
        d.device.registration.expires_at_ms = 1;
        assert_eq!(authority(&d, now()), "授权已到期");
        assert!(ago(now() + 1000, now()).contains("时间异常"));
    }
    #[test]
    fn refreshed_inventory_preserves_ids_and_does_not_select_a_replacement() {
        let d = fixture();
        let id = d.device.registration.id;
        let mut app = model(d.clone());
        app.navigate(Surface::Device(id));
        let mut changed = d.clone();
        changed.device.registration.workspaces.reverse();
        retain_selection(&mut app, &[changed.clone()]);
        assert_eq!(app.selected, 1);
        app.devices = vec![changed.clone()];
        changed.device.registration.workspaces.remove(1);
        retain_selection(&mut app, &[changed.clone()]);
        app.devices = vec![changed];
        assert_eq!(app.selected, 1);
        assert!(text(&app, id, now()).contains("尚未选中其他目录"));
        app.navigate(Surface::Devices);
        retain_selection(&mut app, &[fixture()]);
        assert_eq!(
            app.selected, 1,
            "removed computer must leave no replacement selected"
        );
    }
    #[tokio::test]
    async fn computer_navigation_and_targeted_draft_keep_chat_and_exact_destination() {
        let d = fixture();
        let id = d.device.registration.id;
        let workspace = d.device.registration.workspaces[1].id;
        let mut app = model(d.clone());
        app.input.insert("Chat 草稿");
        app.new_work_draft.insert("任务草稿");
        app.navigate(Surface::Devices);
        let client =
            Client::new("http://127.0.0.1:1", "fixture-device-token-0123456789-abcd").unwrap();
        let (tx, _rx) = mpsc::channel(16);
        let mut jobs = JoinSet::new();
        let mut key = |app: &mut Model, code| {
            handle_key(
                &client,
                app,
                &tx,
                &mut jobs,
                KeyEvent::new(code, KeyModifiers::NONE),
            )
            .unwrap()
        };
        key(&mut app, KeyCode::Enter);
        assert_eq!(app.surface, Surface::Device(id));
        key(&mut app, KeyCode::Char('i'));
        assert!(text(&app, id, now()).contains("Start-Codex-Host.cmd"));
        key(&mut app, KeyCode::Char('i'));
        key(&mut app, KeyCode::Down);
        key(&mut app, KeyCode::Char('n'));
        assert_eq!(app.new_work_preselect, Some((id, workspace)));
        assert_eq!(app.surface, Surface::Work);
        new_work::opened(&mut app, vec![fixture(), d]);
        let panel = app.new_work.as_ref().unwrap();
        assert_eq!(panel.targets[panel.selected].workspace.id, workspace);
        assert_eq!(panel.input.text, "任务草稿");
        assert_eq!(app.input.text, "Chat 草稿");
        assert!(app.new_work_pending.is_none());
        jobs.abort_all();
    }
    #[test]
    fn removed_target_retains_draft_and_cannot_fall_back_to_another_computer() {
        let d = fixture();
        let mut app = model(d.clone());
        app.navigate(Surface::Work);
        app.new_work_draft.insert("继续保留");
        app.new_work_preselect = Some((
            d.device.registration.id,
            d.device.registration.workspaces[0].id,
        ));
        new_work::opened(&mut app, vec![fixture()]);
        let panel = app.new_work.as_ref().unwrap();
        assert!(panel.targets.is_empty());
        assert_eq!(panel.input.text, "继续保留");
        assert!(app.status.contains("未改选其他电脑"));
        new_work::close(&mut app);
        assert_eq!(app.new_work_draft.text, "继续保留");
        assert!(app.new_work_preselect.is_none());
    }
    #[test]
    fn details_render_on_wide_and_narrow_terminals_and_retire_clears_inventory() {
        use ratatui::{Terminal, backend::TestBackend};
        let d = fixture();
        let id = d.device.registration.id;
        let mut app = model(d);
        for surface in [Surface::Devices, Surface::Device(id)] {
            app.navigate(surface);
            for (w, h) in [(160, 50), (80, 40), (40, 24)] {
                Terminal::new(TestBackend::new(w, h))
                    .unwrap()
                    .draw(|f| draw::render(f, &mut app))
                    .unwrap();
            }
        }
        let before = app.scroll;
        let (area, bottom) = app.detail_viewport.unwrap();
        assert!(bottom > 0);
        handle_mouse(
            &mut app,
            MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: area.x,
                row: area.y,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert!(app.scroll > before);
        let inside = app.scroll;
        handle_mouse(
            &mut app,
            MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: 0,
                row: 0,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert_eq!(app.scroll, inside);
        app.device_page = Page::Diagnostics;
        assert!(text(&app, id, now()).contains(&id.to_string()));
        assert!(!text(&app, id, now()).contains(&"a".repeat(64)));
        live::retire(&mut app);
        assert!(app.devices.is_empty());
        assert!(app.device_read_at.is_none());
        assert!(app.device_page == Page::Overview);
    }
}
