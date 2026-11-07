//! Source metadata only. This view starts no collector and discloses no message body.
use super::*;
use lain_wire::archives::{
    ArchiveCoverageKind, ArchiveImportState, ArchiveReadFailure, ArchiveReadState,
    ArchiveSourceKey, ArchiveSourceStatus, KnowledgeSyncStop,
};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

pub(super) fn name(source: &ArchiveSourceStatus) -> String {
    match &source.view.source.key {
        ArchiveSourceKey::QqPrivate {
            observer_account,
            peer_id,
        } => format!("QQ 私聊 {peer_id} · 观察账号 {observer_account}"),
        ArchiveSourceKey::QqGroup {
            observer_account,
            group_id,
        } => format!("QQ 群 {group_id} · 观察账号 {observer_account}"),
        ArchiveSourceKey::CodexSession { installation, .. } => {
            format!("Codex 历史 · {}", safe_text(installation))
        }
    }
}
fn state(s: &ArchiveSourceStatus) -> &'static str {
    match s.import_state {
        ArchiveImportState::Disabled => "未授权 / 已停止",
        ArchiveImportState::Enabled => "导入授权有效",
        ArchiveImportState::Expired => "授权已到期",
    }
}
fn time(at: Option<u64>) -> String {
    at.map_or_else(|| "尚无记录".into(), work_history::timestamp)
}
fn duration(ms: u64) -> String {
    if ms.is_multiple_of(86400000) {
        format!("{} 天", ms / 86400000)
    } else if ms.is_multiple_of(3600000) {
        format!("{} 小时", ms / 3600000)
    } else if ms.is_multiple_of(60000) {
        format!("{} 分钟", ms / 60000)
    } else {
        format!("{} 秒", ms / 1000)
    }
}
pub(super) fn list(frame: &mut Frame, app: &mut Model, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(
        if app.source_read_error && !app.sources.is_empty() {
            " Sources · 读取失败，显示上次状态 · Ctrl-R 重试 "
        } else {
            " Sources · 消息与档案来源 "
        },
    );
    if app.sources.is_empty() {
        let text = if app.refreshing {
            "正在读取来源状态…"
        } else if app.source_read_error {
            "来源状态暂时无法读取。Ctrl-R 重试，不会启动采集。"
        } else {
            "尚未接入任何档案来源。\n\nQQ 能回复你，不代表她已经获得阅读群消息或历史档案的权限。\n需要分别选择收集来源、允许哪个会话阅读、是否允许保存选中的记忆。\n\n本列表只读取登记和接收元数据。G 可显式管理本会话的简报阅读权限；收集须单独配置。\n配置与授权：docs/DAILY_BRIEFING.md"
        };
        frame.render_widget(
            Paragraph::new(text).wrap(Wrap { trim: false }).block(block),
            area,
        );
        return;
    }
    app.selected = app.selected.min(app.sources.len());
    let rows = app
        .sources
        .iter()
        .map(|s| {
            ListItem::new(Text::from(vec![
                Line::raw(name(s)),
                Line::raw(format!(
                    "  {} · 保留文本 {} 条",
                    state(s),
                    s.retained_body_count
                )),
                Line::raw(format!(
                    "  本次授权最近收到：{}",
                    time(s.latest_received_at_ms)
                )),
                Line::raw("  接收连接：尚无可确认的在线信息"),
                Line::raw(""),
            ]))
        })
        .collect::<Vec<_>>();
    let mut selected =
        ListState::default().with_selected((app.selected < rows.len()).then_some(app.selected));
    frame.render_stateful_widget(
        List::new(rows)
            .block(block)
            .highlight_symbol("› ")
            .highlight_style(Style::default().fg(Color::Rgb(121, 185, 189))),
        area,
        &mut selected,
    );
}
pub(super) fn text(s: &ArchiveSourceStatus) -> String {
    let grant = &s.view.grant;
    let limits = grant.scope.as_ref().map_or_else(
        || "关闭；不会接收新导入".into(),
        |g| {
            format!(
                "到期 {}\n文本保留 {} · 本次授权已导入 {}/{} 条\n许可建立 {}（不是 QQ 采集起点）",
                work_history::timestamp(g.expires_at_ms),
                duration(g.retention_ms),
                s.view.current_grant_records,
                g.max_records,
                work_history::timestamp(grant.changed_at_ms)
            )
        },
    );
    let read = s.latest_read.as_ref().map_or_else(
        || "尚未发起来源读取".into(),
        |r| {
            format!(
                "{} · 开始 {} · 结束 {}\n遗漏项 {} · 等待来源 {} · 失败类别 {}",
                match r.state {
                    ArchiveReadState::Prepared => "准备读取",
                    ArchiveReadState::Reading => "读取中",
                    ArchiveReadState::Completed => "读取已完成",
                    ArchiveReadState::Failed => "读取失败",
                    ArchiveReadState::Cancelled => "读取已取消",
                    ArchiveReadState::Unknown => "读取结果不确定",
                },
                time(r.started_at_ms),
                time(r.finished_at_ms),
                r.omitted_items,
                if r.waiting_for_source { "是" } else { "否" },
                match r.failure {
                    None => "未记录",
                    Some(ArchiveReadFailure::Unavailable) => "读取端不可用",
                    Some(ArchiveReadFailure::Unsupported) => "不支持此来源",
                    Some(ArchiveReadFailure::ScopeChanged) => "来源或授权已改变",
                    Some(ArchiveReadFailure::InvalidContent) => "返回内容不符合契约",
                    Some(ArchiveReadFailure::Interrupted) => "读取中断",
                }
            )
        },
    );
    let sync = if let Some(scope) = &s.synchronization.scope {
        let state = match s.synchronization.stopped {
            Some(KnowledgeSyncStop::Expired) => "授权已到期",
            // The planner checks the plan budget before the shared receipt cap.
            Some(KnowledgeSyncStop::Exhausted)
                if s.synchronization.used_requests < scope.max_requests =>
            {
                "因服务的历史读取记录容量已满而停止；单独增加本计划额度不会恢复，请先检查服务容量"
            }
            Some(KnowledgeSyncStop::Exhausted) => "本计划请求额度已用完",
            Some(KnowledgeSyncStop::ScopeChanged) => "来源保存版本已改变，计划已停止",
            Some(KnowledgeSyncStop::ReaderChanged) => "读取配置已改变，计划已停止",
            Some(KnowledgeSyncStop::ReadFailed) => "读取失败，计划已停止且不自动重试",
            Some(KnowledgeSyncStop::SourceUnavailable) => "来源不可用，计划已停止",
            None if scope.expires_at_ms <= s.observed_at_ms => "授权期限已到，不能开始新读取",
            None => "有限读取计划有效（不代表 QQ 在线）",
        };
        format!(
            "{}\n到期 {} · 已用请求 {}/{}\n下次计划 {} · 等待本次回执 {}",
            state,
            work_history::timestamp(scope.expires_at_ms),
            s.synchronization.used_requests,
            scope.max_requests,
            if s.synchronization.stopped.is_some() || scope.expires_at_ms <= s.observed_at_ms {
                "已停止".into()
            } else {
                work_history::timestamp(s.synchronization.next_at_ms)
            },
            if s.synchronization.pending {
                "是"
            } else {
                "否"
            }
        )
    } else if s.synchronization.revision > 0 {
        "已关闭定期同步；以前已用请求仍保留在历史中".into()
    } else {
        "未开启定期同步".into()
    };
    let coverage = s
        .upstream_snapshot
        .as_ref()
        .and_then(|u| u.coverage.as_ref())
        .map_or_else(
            || "尚无上游缺口报告；不能据此判断没有遗漏。".into(),
            |c| {
                let mut lines = vec![format!(
                    "上游历史报告截至 {}（最多 20 条；授权时期未追踪）",
                    work_history::timestamp(c.observed_at_ms)
                )];
                if !s.upstream_snapshot_fresh {
                    lines.push("这份快照已失效，仅供查看历史；不是当前接收状态。".into());
                }
                for e in &c.events {
                    lines.push(format!(
                        "{} · {}",
                        work_history::timestamp(e.recorded_at_ms),
                        match e.kind {
                            ArchiveCoverageKind::ConnectedNoBackfill =>
                                "收到连接报告；没有补回之前历史",
                            ArchiveCoverageKind::DisconnectedGap =>
                                "收到断连缺口报告；缺口起止未知",
                            ArchiveCoverageKind::MediaUnavailable => "部分媒体未能取得",
                            ArchiveCoverageKind::OutboxDeliveryFailed => "观察器消息投递失败",
                        }
                    ));
                }
                if c.events.is_empty() {
                    lines.push("有限历史中没有报告，不证明没有遗漏。".into());
                }
                lines.push("以上时间是服务收到报告的时间，不是实际断连或恢复时间。".into());
                lines.join("\n")
            },
        );
    format!(
        "{}\n\n来源状态更新：{}\n导入授权：{}\n{}\n\n本次授权最近收到：{}\n来源声称的最近发生时间：{}\n存有文本：{} 条（不等于当前会话有阅读权限）\n\n最近读取\n{}\n\n同步计划\n{}\n上游快照：{}\n\n覆盖与缺口\n只证明已保存的可见片段，不证明完整历史或持续在线。\n接收连接：尚无可确认的在线信息。\n{}\n数据空白不能解释成“昨天没有重要消息”。\n\n授权分开管理\n收集/导入、当前会话阅读、保存记忆相互独立。\n来源停止或到期后不能继续用旧文本回答。\n\nG 管理当前会话简报阅读权限（不启动收集）。\n配置、停止与撤销步骤：docs/DAILY_BRIEFING.md\n底层元数据：lain archive status {}",
        name(s),
        work_history::timestamp(s.observed_at_ms),
        state(s),
        limits,
        time(s.latest_received_at_ms),
        time(s.latest_reported_occurrence_at_ms),
        s.retained_body_count,
        read,
        sync,
        if s.upstream_snapshot_fresh {
            "最近同步的快照仍有效；不代表实时在线"
        } else {
            "未同步或已失效；不代表来源没有消息"
        },
        coverage,
        s.view.source.id
    )
}
pub(super) fn detail(
    frame: &mut Frame,
    app: &mut Model,
    area: Rect,
    id: lain_wire::ArchiveSourceId,
) {
    let mut text = app
        .sources
        .iter()
        .find(|s| s.view.source.id == id)
        .map_or_else(
            || "来源不在当前状态列表中。Ctrl-R 重读，Esc 返回。".into(),
            text,
        );
    if app.source_read_error {
        text = format!("最近读取失败；以下为上次读取的缓存状态，不能证明当前状态。\n\n{text}");
    }
    let (_, bottom) = draw::body_position(
        frame,
        area,
        &text,
        &mut app.scroll,
        false,
        " 来源详情 · 滚轮/PgUp/PgDn ",
    );
    app.detail_viewport = Some((area.inner(ratatui::layout::Margin::new(1, 1)), bottom));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_sync_distinguishes_plan_quota_from_service_receipt_capacity() {
        let mut row = fixture();
        row.synchronization.revision = 1;
        row.synchronization.scope = Some(lain_wire::archives::KnowledgeSyncScope {
            archive_revision: 1,
            expires_at_ms: 3000,
            max_requests: 2,
            interval_ms: 5000,
            limit: 16,
        });
        row.synchronization.stopped = Some(KnowledgeSyncStop::Exhausted);
        let body = text(&row);
        assert!(body.contains("服务的历史读取记录容量已满"), "{body}");
        assert!(body.contains("增加本计划额度不会恢复"), "{body}");
        assert!(!body.contains("本计划请求额度已用完"), "{body}");
        assert!(body.contains("已用请求 0/2"), "{body}");
        row.synchronization.used_requests = 2;
        let body = text(&row);
        assert!(body.contains("本计划请求额度已用完"), "{body}");
        assert!(!body.contains("历史读取记录容量已满"), "{body}");
    }
    #[test]
    fn coverage_report_is_historical_bounded_and_never_claims_live_connection() {
        let mut row = fixture();
        row.upstream_snapshot=Some(serde_json::from_value(json!({"source_id":row.view.source.id,"local_grant_revision":1,"reader_revision":"fixture","store_id":"fixture","remote_source_revision":1,"remote_source_enabled":true,"through":1,"last_read":lain_wire::RequestId::new(),"observed_at_ms":2000,"fresh_until_ms":3000,"coverage":{"observed_at_ms":2000,"bounded_history":true,"source_epochs_untracked":true,"events":[{"sequence":2,"kind":"disconnected_gap","recorded_at_ms":1800},{"sequence":1,"kind":"connected_no_backfill","recorded_at_ms":1000}]}})).unwrap());
        let body = text(&row);
        for phrase in [
            "快照已失效",
            "断连缺口",
            "没有补回之前历史",
            "授权时期未追踪",
            "服务收到报告的时间",
            "尚无可确认的在线信息",
        ] {
            assert!(body.contains(phrase), "{phrase}: {body}");
        }
        assert!(!body.contains("接收连接：在线"));
    }
    fn fixture() -> ArchiveSourceStatus {
        let id = lain_wire::ArchiveSourceId::new();
        serde_json::from_value(json!({"view":{"source":{"id":id,"key":{"kind":"qq_group","observer_account":"1001","group_id":"3003"},"created_at_ms":1000},"grant":{"source_id":id,"revision":0,"scope":null,"changed_at_ms":0},"total_records":0,"current_grant_records":0,"committed_through":0,"coverage":"selected_visible_slice"},"observed_at_ms":2000,"import_state":"disabled","latest_received_at_ms":null,"latest_reported_occurrence_at_ms":null,"retained_body_count":0,"receiving_now":null,"latest_read":null,"upstream_snapshot":null,"upstream_snapshot_fresh":false,"synchronization":{"source_id":id,"revision":0,"scope":null,"reader_revision":null,"used_requests":0,"next_at_ms":0,"last_read":null,"pending":false,"stopped":null,"changed_at_ms":0}})).unwrap()
    }
    #[tokio::test]
    async fn sources_navigation_stale_reads_and_retirement_never_enable_collection() {
        let mut app = Model::new(ConversationId::new());
        let client = Client::new(
            "http://127.0.0.1:1",
            "source-view-fixture-token-01234567890",
        )
        .unwrap();
        let (tx, _rx) = mpsc::channel(16);
        let mut jobs = JoinSet::new();
        app.input.insert("/sources");
        handle_key(
            &client,
            &mut app,
            &tx,
            &mut jobs,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        )
        .unwrap();
        assert_eq!(app.surface, Surface::Sources);
        let row = fixture();
        let id = row.view.source.id;
        let generation = app.generation;
        handle_complete(
            &client,
            &mut app,
            &tx,
            &mut jobs,
            Complete::Sources(generation, Ok(vec![row])),
        )
        .unwrap();
        handle_key(
            &client,
            &mut app,
            &tx,
            &mut jobs,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        )
        .unwrap();
        assert_eq!(app.surface, Surface::Source(id));
        assert!(text(&app.sources[0]).contains("不证明完整历史"));
        assert!(app.sources[0].view.grant.scope.is_none());
        handle_complete(
            &client,
            &mut app,
            &tx,
            &mut jobs,
            Complete::Sources(generation, Ok(vec![])),
        )
        .unwrap();
        assert_eq!(
            app.sources.len(),
            1,
            "old generation cannot replace the selected source view"
        );
        live::retire(&mut app);
        assert!(app.sources.is_empty());
        assert!(app.detail_viewport.is_none());
        jobs.abort_all();
    }
    #[test]
    fn source_text_reports_unknown_collection_and_empty_view_does_not_claim_no_news() {
        use ratatui::{Terminal, backend::TestBackend};
        let row = fixture();
        let body = text(&row);
        assert!(body.contains("尚无可确认的在线信息") && body.contains("未开启定期同步"));
        assert!(body.contains("1001") && body.contains("3003"));
        let mut app = Model::new(ConversationId::new());
        app.navigate(Surface::Sources);
        for (w, h) in [(160, 50), (80, 30), (40, 24)] {
            let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
            terminal.draw(|f| draw::render(f, &mut app)).unwrap();
            let screen = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|c| c.symbol())
                .collect::<String>()
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>();
            assert!(screen.contains("尚未接入任何档案来源"), "{w}x{h}: {screen}");
            assert!(!screen.contains("昨天没有重要消息"));
        }
    }
}
