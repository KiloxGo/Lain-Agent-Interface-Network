//! Explicit conversation disclosure; never starts intake or grants memory writes.
use super::*;
use lain_wire::{
    ArchiveSourceId,
    archives::{ArchiveImportState, ArchiveRecallSource, ArchiveSourceKey, ArchiveSourceStatus},
    briefing::{BriefingGrant, BriefingGrantReceipt, BriefingScope, SetBriefingGrant},
};

struct Editor {
    id: RequestId,
    sources: Vec<ArchiveSourceStatus>,
    previous: BriefingGrant,
    selected: BTreeSet<ArchiveSourceId>,
    fields: [Composer; 4],
    row: usize,
    focus: usize,
}
fn now() -> u64 {
    devices_view::now()
}
pub(super) fn offset(text: &str) -> Result<i16> {
    let b = text.as_bytes();
    ensure!(
        b.len() == 6
            && matches!(b[0], b'+' | b'-')
            && b[3] == b':'
            && [b[1], b[2], b[4], b[5]].iter().all(u8::is_ascii_digit),
        "时区格式需为 +08:00 或 -05:00"
    );
    let hours: i16 = text[1..3].parse()?;
    let minutes: i16 = text[4..6].parse()?;
    ensure!(
        hours <= 14 && minutes < 60 && (hours < 14 || minutes == 0),
        "时区需在 -14:00 至 +14:00"
    );
    Ok((hours * 60 + minutes) * if b[0] == b'-' { -1 } else { 1 })
}
pub(super) fn offset_text(minutes: i16) -> String {
    format!(
        "{}{:02}:{:02}",
        if minutes < 0 { '-' } else { '+' },
        minutes.unsigned_abs() / 60,
        minutes.unsigned_abs() % 60
    )
}
pub(super) fn date_text(ms: u64, offset: i16) -> Result<String> {
    let d = time::OffsetDateTime::from_unix_timestamp(i64::try_from(ms / 1000)?)?
        .to_offset(time::UtcOffset::from_whole_seconds(i32::from(offset) * 60)?);
    Ok(format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        d.year(),
        u8::from(d.month()),
        d.day(),
        d.hour(),
        d.minute(),
        d.second()
    ))
}
pub(super) fn date(text: &str, offset: i16) -> Result<u64> {
    let b = text.as_bytes();
    ensure!(
        (b.len() == 16 || b.len() == 19)
            && b.iter().enumerate().all(|(i, c)| match i {
                4 | 7 => *c == b'-',
                10 => *c == b' ',
                13 | 16 => *c == b':',
                _ => c.is_ascii_digit(),
            }),
        "日期格式需为 YYYY-MM-DD HH:MM（可带秒）"
    );
    let d = time::Date::from_calendar_date(
        text[..4].parse()?,
        time::Month::try_from(text[5..7].parse::<u8>()?)?,
        text[8..10].parse()?,
    )?;
    let t = time::Time::from_hms(
        text[11..13].parse()?,
        text[14..16].parse()?,
        if b.len() == 19 {
            text[17..].parse()?
        } else {
            0
        },
    )?;
    let seconds = d
        .with_time(t)
        .assume_offset(time::UtcOffset::from_whole_seconds(i32::from(offset) * 60)?)
        .unix_timestamp();
    u64::try_from(seconds)?
        .checked_mul(1000)
        .context("日期超出范围")
}
fn eligible(s: &ArchiveSourceStatus, now: u64) -> bool {
    s.view.source.id == s.view.grant.source_id
        && s.view.grant.revision > 0
        && matches!(s.import_state, ArchiveImportState::Enabled)
        && matches!(
            s.view.source.key,
            ArchiveSourceKey::QqGroup { .. } | ArchiveSourceKey::QqPrivate { .. }
        )
        && s.view
            .grant
            .scope
            .as_ref()
            .is_some_and(|g| g.expires_at_ms > now)
}
fn validate_scope(scope: &BriefingScope, sources: &[ArchiveSourceStatus], now: u64) -> Result<()> {
    ensure!(
        (1..=8).contains(&scope.sources.len()),
        "请选择 1–8 个来源；撤销请用 F8"
    );
    ensure!(
        (1..=4096).contains(&scope.max_records),
        "单次记录上限需为 1–4096"
    );
    ensure!(
        (-840..=840).contains(&scope.utc_offset_minutes)
            && scope.from_ms <= now
            && scope.from_ms < scope.expires_at_ms
            && scope.expires_at_ms > now
            && scope.expires_at_ms <= now.saturating_add(30 * 86400000),
        "起始时间不能在未来；到期时间需在未来 30 天内"
    );
    let mut seen = BTreeSet::new();
    for selected in &scope.sources {
        ensure!(seen.insert(selected.source_id), "来源重复");
        let s = sources
            .iter()
            .find(|s| s.view.source.id == selected.source_id)
            .context("所选来源已不在列表中，请重新读取")?;
        ensure!(
            eligible(s, now) && s.view.grant.revision == selected.grant_revision,
            "来源授权已改变或到期，请重新选择"
        );
        ensure!(
            scope.expires_at_ms <= s.view.grant.scope.as_ref().unwrap().expires_at_ms,
            "会话阅读到期不能晚于所选来源授权到期"
        );
    }
    Ok(())
}
fn draft(editor: &Editor, now: u64, revoke: bool) -> Result<SetBriefingGrant> {
    let scope = if revoke {
        None
    } else {
        let o = offset(&editor.fields[2].text)?;
        let mut sources = vec![];
        for id in &editor.selected {
            let s = editor
                .sources
                .iter()
                .find(|s| s.view.source.id == *id)
                .context("所选来源不存在")?;
            sources.push(ArchiveRecallSource {
                source_id: *id,
                grant_revision: s.view.grant.revision,
            });
        }
        let scope = BriefingScope {
            sources,
            from_ms: date(&editor.fields[0].text, o)?,
            expires_at_ms: date(&editor.fields[1].text, o)?,
            utc_offset_minutes: o,
            max_records: editor.fields[3].text.parse().context("记录上限需为数字")?,
        };
        validate_scope(&scope, &editor.sources, now)?;
        Some(scope)
    };
    Ok(SetBriefingGrant {
        request_id: RequestId::new(),
        expected_revision: editor.previous.revision,
        scope,
    })
}
fn matches_receipt(p: &SetBriefingGrant, c: ConversationId, r: &BriefingGrantReceipt) -> bool {
    r.request == *p
        && r.grant.conversation_id == c
        && p.expected_revision.checked_add(1) == Some(r.grant.revision)
        && r.grant.scope == p.scope
}

#[derive(Clone)]
pub(super) struct Pending {
    conversation: ConversationId,
    input: SetBriefingGrant,
    text: String,
}
struct Preview {
    pending: Pending,
    scroll: u16,
    read: bool,
    seen_to: usize,
    width: u16,
    page: u16,
}
impl Preview {
    fn new(pending: Pending) -> Self {
        Self {
            pending,
            scroll: 0,
            read: false,
            seen_to: 0,
            width: 0,
            page: 1,
        }
    }
}
pub(super) struct Panel {
    editor: Option<Editor>,
    preview: Option<Preview>,
}
#[derive(Default)]
pub(super) struct State {
    pub panel: Option<Panel>,
    loading: bool,
    pending: Option<Pending>,
    busy: bool,
    reopen: bool,
}
pub(super) enum Failure {
    BeforePost(anyhow::Error),
    Rejected(anyhow::Error),
    Uncertain(anyhow::Error),
}
fn post_failure(error: anyhow::Error) -> Failure {
    // These Kernel responses reject before applying a new grant. Timeouts and
    // server/transport errors retain the exact pending request for read-only recovery.
    if error
        .downcast_ref::<crate::api::HttpFailure>()
        .is_some_and(|e| matches!(e.0, 401 | 403 | 404 | 409 | 422))
    {
        Failure::Rejected(error)
    } else {
        Failure::Uncertain(error)
    }
}
async fn post(client: &Client, p: &Pending) -> std::result::Result<BriefingGrantReceipt, Failure> {
    match client
        .post(
            &format!("v1/briefing/{}/grant", p.conversation),
            Some(json!(p.input)),
        )
        .await
    {
        Ok(grant) => Ok(BriefingGrantReceipt {
            request: p.input.clone(),
            grant,
        }),
        Err(error)
            if error
                .downcast_ref::<crate::api::HttpFailure>()
                .is_some_and(|e| e.0 == 400) =>
        {
            // A response safety check can fail after commit. A completed 400 by
            // itself is insufficient proof of rejection: inspect the exact receipt.
            match client
                .get(&format!(
                    "v1/briefing/{}/grant/requests/{}",
                    p.conversation, p.input.request_id
                ))
                .await
            {
                Ok(receipt) => Ok(receipt),
                Err(read)
                    if read
                        .downcast_ref::<crate::api::HttpFailure>()
                        .is_some_and(|e| e.0 == 404) =>
                {
                    Err(Failure::Rejected(error))
                }
                Err(_) => Err(Failure::Uncertain(error)),
            }
        }
        Err(error) => Err(post_failure(error)),
    }
}
pub(super) type Loaded = (Vec<ArchiveSourceStatus>, BriefingGrant);
pub(super) fn pending(app: &Model) -> bool {
    app.source_access.pending.is_some()
}
pub(super) fn waiting(app: &Model) -> bool {
    app.source_access.panel.is_none() && (app.source_access.loading || app.source_access.reopen)
}
pub(super) fn close(app: &mut Model) {
    app.source_access.panel = None;
    app.source_access.loading = false;
    app.source_access.reopen = false;
}
pub(super) fn invalidate(app: &mut Model) {
    let reopen =
        app.source_access.reopen || app.source_access.loading || app.source_access.panel.is_some();
    close(app);
    // Only re-open a requested metadata editor. Never retain its old selection,
    // review gate or a permission to send after synchronization changes.
    app.source_access.reopen = reopen && app.source_access.pending.is_none();
}
pub(super) fn resume(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
) {
    if app.source_access.reopen
        && live::fresh(app)
        && matches!(app.surface, Surface::Sources | Surface::Source(_))
        && let Err(e) = open(client, app, tx, jobs)
    {
        app.status = format!("重新读取来源权限失败：{e}");
    }
}
pub(super) fn open(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
) -> Result<()> {
    source_setup::close(app);
    if !live::fresh(app) {
        app.source_access.reopen = true;
        app.status = "正在同步；完成后重新读取来源权限，尚未修改授权".into();
        return Ok(());
    }
    ensure!(!app.source_access.loading, "正在读取会话来源权限");
    app.generation += 1;
    app.refreshing = false;
    if let Some(p) = app.source_access.pending.clone() {
        app.source_access.panel = Some(Panel {
            editor: None,
            preview: Some(Preview::new(p)),
        });
        app.status = "上次授权变更结果待核对；Ctrl-R 只读取原回执，不会重复提交".into();
        return Ok(());
    }
    app.source_access.loading = true;
    app.source_access.reopen = false;
    let generation = app.generation;
    let conversation = app.conversation;
    let c = client.clone();
    let out = live::read_sender(app, tx, jobs);
    live::spawn_read(app, jobs, async move {
        let result = async {
            let grant = c.get(&format!("v1/briefing/{conversation}/grant")).await?;
            Ok((c.get("v1/archives/status").await?, grant))
        }
        .await;
        let _ = out
            .send(Complete::SourceAccessOpened(generation, Box::new(result)))
            .await;
    });
    app.status = "正在读取来源和当前会话权限；尚未修改授权".into();
    Ok(())
}
pub(super) fn opened(app: &mut Model, result: Result<Loaded>) -> Result<()> {
    app.source_access.loading = false;
    let (sources, previous) = result?;
    ensure!(
        previous.conversation_id == app.conversation,
        "会话权限身份不匹配"
    );
    ensure!(
        sources.len() <= 64,
        "来源超过状态页 64 个上限；请使用命令行明确配置，不截断当前选择"
    );
    let mut identities = BTreeSet::new();
    ensure!(
        sources.iter().all(|s| identities.insert(s.view.source.id)),
        "来源身份重复，请重新读取"
    );
    let o = previous
        .scope
        .as_ref()
        .map_or(480, |s| s.utc_offset_minutes);
    let selected = previous.scope.as_ref().map_or_else(BTreeSet::new, |p| {
        p.sources
            .iter()
            .filter(|v| {
                sources.iter().any(|s| {
                    eligible(s, now())
                        && s.view.source.id == v.source_id
                        && s.view.grant.revision == v.grant_revision
                })
            })
            .map(|v| v.source_id)
            .collect()
    });
    let from = previous.scope.as_ref().map_or_else(
        || {
            let shifted = i128::from(now()) + i128::from(o) * 60000;
            u64::try_from(
                shifted.div_euclid(86400000) * 86400000 - 86400000 - i128::from(o) * 60000,
            )
            .unwrap_or(0)
        },
        |p| p.from_ms,
    );
    let until = previous
        .scope
        .as_ref()
        .map_or(now().saturating_add(86400000), |p| p.expires_at_ms);
    let limit = previous.scope.as_ref().map_or(256, |p| p.max_records);
    let fields = [
        date_text(from, o)?,
        date_text(until, o)?,
        offset_text(o),
        limit.to_string(),
    ]
    .map(|text| Composer {
        cursor: text.len(),
        text,
    });
    app.source_access.panel = Some(Panel {
        editor: Some(Editor {
            id: RequestId::new(),
            sources,
            previous,
            selected,
            fields,
            row: 0,
            focus: 0,
        }),
        preview: None,
    });
    app.status = "编辑仅当前会话的阅读范围；Enter 审阅后 Ctrl-Y 才生效".into();
    Ok(())
}
fn scope_text(scope: Option<&BriefingScope>, sources: &[ArchiveSourceStatus]) -> Result<String> {
    let Some(s) = scope else {
        return Ok("关闭：此会话不能用这些档案生成简报".into());
    };
    let names = s
        .sources
        .iter()
        .map(|v| {
            let name = sources
                .iter()
                .find(|s| s.view.source.id == v.source_id)
                .map_or_else(
                    || format!("来源 {}（当前列表缺失）", v.source_id),
                    sources_view::name,
                );
            format!("- {} · 来源版本 {}", safe_text(&name), v.grant_revision)
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(format!(
        "{names}\n起始：{} {}\n到期：{} {}\n单次最多 {} 条记录（不是完整历史保证）",
        date_text(s.from_ms, s.utc_offset_minutes)?,
        offset_text(s.utc_offset_minutes),
        date_text(s.expires_at_ms, s.utc_offset_minutes)?,
        offset_text(s.utc_offset_minutes),
        s.max_records
    ))
}
fn preview(editor: &Editor, revoke: bool, conversation: ConversationId) -> Result<Preview> {
    let input = draft(editor, now(), revoke)?;
    let text = format!(
        "{}\n\n作用会话：{}\n这次变更只影响此会话；QQ 私聊是另一个会话，不会一并共享。\n\n当前配置（版本 {}）\n{}\n\n确认后替换为\n{}\n\n此操作允许按需读取原消息并交给已配置模型处理。\n不启动收集、不补拉历史、不开放其他会话、不授权自动保存记忆。\n撤销会使依赖旧权限的简报和回答正文不可用；原档案及独立记忆按各自规则处理。\n来源读取失败、消息缺口或版本变化时仍可能无法生成简报。\n\n尚未提交。Ctrl-Y 仅发送这一份确切范围；结果不确定时只查询回执。\n操作编号：{}",
        if revoke {
            "撤销当前会话的简报阅读权限"
        } else {
            "审阅当前会话的简报阅读权限"
        },
        conversation,
        editor.previous.revision,
        scope_text(editor.previous.scope.as_ref(), &editor.sources)?,
        scope_text(input.scope.as_ref(), &editor.sources)?,
        input.request_id
    );
    Ok(Preview::new(Pending {
        conversation,
        input,
        text,
    }))
}
pub(super) fn recover(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
) {
    let Some(p) = app
        .source_access
        .pending
        .clone()
        .filter(|_| !app.source_access.busy)
    else {
        return;
    };
    app.source_access.busy = true;
    let c = client.clone();
    let out = live::effect_sender(app, tx, jobs);
    jobs.spawn(async move {
        let result = c
            .get(&format!(
                "v1/briefing/{}/grant/requests/{}",
                p.conversation, p.input.request_id
            ))
            .await
            .map_err(Failure::Uncertain);
        let _ = out
            .send(Complete::SourceAccessSent(Box::new((p, result))))
            .await;
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
    if ctrl && key.code == KeyCode::Char('r') {
        if app.source_access.pending.is_some() {
            recover(client, app, tx, jobs);
        } else {
            close(app);
            open(client, app, tx, jobs)?;
        }
        return Ok(());
    }
    if key.code == KeyCode::Esc {
        if app.source_access.pending.is_none()
            && let Some(panel) = app.source_access.panel.as_mut()
            && panel.editor.is_some()
            && panel.preview.take().is_some()
        {
            return Ok(());
        }
        close(app);
        return Ok(());
    }
    if app.source_access.pending.is_some() {
        return Ok(());
    }
    let panel = app
        .source_access
        .panel
        .as_mut()
        .context("来源权限页已关闭")?;
    if let Some(r) = panel.preview.as_mut() {
        match key.code {
            KeyCode::PageUp => r.scroll = r.scroll.saturating_sub(r.page),
            KeyCode::PageDown => r.scroll = r.scroll.saturating_add(r.page),
            KeyCode::Char('y') if ctrl && r.read => {
                ensure!(live::fresh(app), "连接状态已过期，请重新同步并审阅");
                let p = app
                    .source_access
                    .panel
                    .as_ref()
                    .unwrap()
                    .preview
                    .as_ref()
                    .unwrap()
                    .pending
                    .clone();
                app.source_access.pending = Some(p.clone());
                app.source_access.busy = true;
                let c = client.clone();
                let out = live::effect_sender(app, tx, jobs);
                jobs.spawn(async move {
                    let check: Result<()> = async {
                        let current: BriefingGrant = c
                            .get(&format!("v1/briefing/{}/grant", p.conversation))
                            .await?;
                        ensure!(
                            current.conversation_id == p.conversation
                                && current.revision == p.input.expected_revision,
                            "会话权限已改变，请重新读取并审阅"
                        );
                        if let Some(scope) = &p.input.scope {
                            let sources: Vec<ArchiveSourceStatus> =
                                c.get("v1/archives/status").await?;
                            validate_scope(scope, &sources, now())?;
                        }
                        Ok(())
                    }
                    .await;
                    let result = match check {
                        Err(e) => Err(Failure::BeforePost(e)),
                        Ok(()) => post(&c, &p).await,
                    };
                    let _ = out
                        .send(Complete::SourceAccessSent(Box::new((p, result))))
                        .await;
                });
                app.status = "正在提交确切阅读范围；可以返回聊天，结果不确定时不会重发".into();
            }
            _ => {}
        }
    } else if let Some(e) = panel.editor.as_mut() {
        match key.code {
            KeyCode::Tab => e.focus = (e.focus + 1) % 5,
            KeyCode::BackTab => e.focus = (e.focus + 4) % 5,
            KeyCode::Up if e.focus == 0 => e.row = e.row.saturating_sub(1),
            KeyCode::Down if e.focus == 0 => {
                e.row = (e.row + 1).min(e.sources.len().saturating_sub(1))
            }
            KeyCode::Char(' ') if e.focus == 0 => {
                let s = e
                    .sources
                    .get(e.row)
                    .context("没有可选择的来源；需先登记并明确授权导入")?;
                if !e.selected.remove(&s.view.source.id) {
                    ensure!(
                        eligible(s, now()),
                        "此来源未授权、已到期或不是 QQ 来源；本页不会自动启用采集"
                    );
                    ensure!(e.selected.len() < 8, "最多选择 8 个来源");
                    e.selected.insert(s.view.source.id);
                }
            }
            KeyCode::Enter => {
                panel.preview = Some(preview(e, false, app.conversation)?);
                app.status = "范围尚未提交；请完整审阅新旧配置后决定".into();
            }
            KeyCode::F(8) => {
                panel.preview = Some(preview(e, true, app.conversation)?);
                app.status = "尚未撤销；请完整审阅影响后决定".into();
            }
            _ if e.focus > 0 => e.fields[e.focus - 1].key(key),
            _ => {}
        }
    }
    Ok(())
}
pub(super) fn complete(
    app: &mut Model,
    p: Pending,
    result: std::result::Result<BriefingGrantReceipt, Failure>,
) {
    if app
        .source_access
        .pending
        .as_ref()
        .is_none_or(|old| old.conversation != p.conversation || old.input != p.input)
    {
        return;
    }
    app.source_access.busy = false;
    match result {
        Ok(r) if matches_receipt(&p.input, p.conversation, &r) => {
            app.source_access.pending = None;
            close(app);
            app.status =
                "该次阅读授权变更已记录；以 G 重新读取的当前状态为准。未启用采集或自动记忆。"
                    .into();
        }
        Err(Failure::BeforePost(e)) => {
            app.source_access.pending = None;
            close(app);
            app.status = format!("尚未提交阅读授权：{e}；G 重新读取");
        }
        Err(Failure::Rejected(e)) => {
            app.source_access.pending = None;
            close(app);
            app.status = format!("阅读授权请求被拒绝，未应用变更：{e}；G 重新读取并审阅");
        }
        Err(Failure::Uncertain(e)) => {
            app.status = format!("授权结果待核对：{e}；Ctrl-R 只读原回执，不重复提交")
        }
        _ => app.status = "回执不匹配；保留原请求，Ctrl-R 重新核对，不重复提交".into(),
    }
}
pub(super) fn paste(app: &mut Model, text: &str) {
    if app.source_access.pending.is_none()
        && let Some(e) = app
            .source_access
            .panel
            .as_mut()
            .filter(|p| p.preview.is_none())
            .and_then(|p| p.editor.as_mut())
            .filter(|e| e.focus > 0)
    {
        e.fields[e.focus - 1].insert(text);
    }
}
pub(super) fn paste_target(app: &Model) -> Option<(RequestId, usize)> {
    app.source_access
        .panel
        .as_ref()
        .filter(|p| p.preview.is_none() && app.source_access.pending.is_none())
        .and_then(|p| p.editor.as_ref())
        .filter(|e| e.focus > 0)
        .map(|e| (e.id, e.focus))
}
pub(super) fn paste_input(app: &mut Model) -> &mut Composer {
    let e = app
        .source_access
        .panel
        .as_mut()
        .unwrap()
        .editor
        .as_mut()
        .unwrap();
    &mut e.fields[e.focus - 1]
}
pub(super) fn mouse(app: &mut Model, event: MouseEvent) {
    if let Some(r) = app
        .source_access
        .panel
        .as_mut()
        .and_then(|p| p.preview.as_mut())
    {
        match event.kind {
            MouseEventKind::ScrollUp => r.scroll = r.scroll.saturating_sub(3),
            MouseEventKind::ScrollDown => r.scroll = r.scroll.saturating_add(3),
            _ => {}
        }
    }
}
pub(super) fn draw(
    frame: &mut ratatui::Frame,
    app: &mut Model,
    area: ratatui::layout::Rect,
    hint: ratatui::layout::Rect,
) {
    use ratatui::{
        layout::{Constraint, Layout},
        style::{Color, Style},
        widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    };
    let panel = app.source_access.panel.as_mut().unwrap();
    if let Some(r) = panel.preview.as_mut() {
        let text = if app.source_access.pending.is_some() {
            format!(
                "该次变更已发起，结果待核对；下文是当时审阅的范围，不会再次发送。\n\n{}",
                r.pending.text.replace(
                    "尚未提交。Ctrl-Y 仅发送这一份确切范围；结果不确定时只查询回执。",
                    "本次请求不会重发；Ctrl-R 只查询原回执。"
                )
            )
        } else {
            r.pending.text.clone()
        };
        let end = draw::body(
            frame,
            area,
            &text,
            &mut r.scroll,
            false,
            " 审阅来源阅读权限 · PgUp/PgDn ",
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
        frame.render_widget(
            Paragraph::new(if app.source_access.pending.is_some() {
                "结果待核对 · Ctrl-R 只读原回执 · Esc 返回"
            } else if r.read {
                "Ctrl-Y 确认确切范围 · Esc 返回修改"
            } else {
                "完整阅读后才能确认 · PgDn 翻页 · Esc 返回修改"
            }),
            hint,
        );
    } else if let Some(e) = panel.editor.as_mut() {
        let a = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(2),
            Constraint::Length(12),
        ])
        .split(area);
        frame.render_widget(Paragraph::new("当前会话 · 简报阅读权限\nSpace 选择来源；接收消息、当前会话阅读、记忆保存分别授权。"),a[0]);
        let items: Vec<_> = e
            .sources
            .iter()
            .map(|s| {
                ListItem::new(format!(
                    "[{}] {} · {}",
                    if e.selected.contains(&s.view.source.id) {
                        'x'
                    } else {
                        ' '
                    },
                    safe_text(&sources_view::name(s)),
                    if eligible(s, now()) {
                        "可选"
                    } else {
                        "未授权 / 过期 / 不适用"
                    }
                ))
            })
            .collect();
        let mut state = ListState::default().with_selected((!items.is_empty()).then_some(e.row));
        let title = if items.is_empty() {
            " 没有登记来源；此页仍可撤销旧权限 "
        } else {
            " 已登记来源 · ↑/↓ · Space "
        };
        frame.render_stateful_widget(
            List::new(items)
                .block(Block::default().borders(Borders::ALL).title(title))
                .highlight_style(Style::default().fg(Color::Rgb(121, 185, 189))),
            a[1],
            &mut state,
        );
        let fields = Layout::vertical([Constraint::Length(3); 4]).split(a[2]);
        for (i, label) in [
            "起始时间 YYYY-MM-DD HH:MM[:SS]",
            "到期时间 YYYY-MM-DD HH:MM[:SS]",
            "时区偏移，例如 +08:00",
            "单次记录上限 1–4096",
        ]
        .iter()
        .enumerate()
        {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(*label)
                .border_style(Style::default().fg(if e.focus == i + 1 {
                    Color::Rgb(121, 185, 189)
                } else {
                    Color::DarkGray
                }));
            let inner = fields[i].inner(ratatui::layout::Margin::new(1, 1));
            let cursor = unicode_width::UnicodeWidthStr::width(
                safe_text(e.fields[i].text.get(..e.fields[i].cursor).unwrap_or("")).as_str(),
            );
            let left = if e.focus == i + 1 {
                cursor.saturating_sub(usize::from(inner.width.saturating_sub(1)))
            } else {
                0
            }
            .min(usize::from(u16::MAX)) as u16;
            frame.render_widget(
                Paragraph::new(safe_text(&e.fields[i].text))
                    .scroll((0, left))
                    .block(block),
                fields[i],
            );
            if e.focus == i + 1 && inner.width > 0 && inner.height > 0 {
                frame.set_cursor_position((
                    inner.x
                        + (cursor.saturating_sub(usize::from(left)))
                            .min(usize::from(inner.width - 1)) as u16,
                    inner.y,
                ));
            }
        }
        frame.render_widget(Paragraph::new("Tab 切换 · Ctrl-U 清空 · F9 粘贴 · Enter 审阅\nF8 审阅撤销 · Ctrl-R 重读 · Esc 关闭；不启用收集"),hint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const NOW: u64 = 1_793_880_000_000;
    fn source() -> ArchiveSourceStatus {
        let id = ArchiveSourceId::new();
        serde_json::from_value(json!({"view":{"source":{"id":id,"key":{"kind":"qq_group","observer_account":"1001","group_id":"3003"},"created_at_ms":1000},"grant":{"source_id":id,"revision":2,"scope":{"expires_at_ms":NOW+86400000,"retention_ms":86400000,"max_records":1000},"changed_at_ms":NOW},"total_records":0,"current_grant_records":0,"committed_through":0,"coverage":"selected_visible_slice"},"observed_at_ms":NOW,"import_state":"enabled","latest_received_at_ms":null,"latest_reported_occurrence_at_ms":null,"retained_body_count":0,"receiving_now":null,"latest_read":null,"upstream_snapshot":null,"upstream_snapshot_fresh":false,"synchronization":{"source_id":id,"revision":0,"scope":null,"reader_revision":null,"used_requests":0,"next_at_ms":0,"last_read":null,"pending":false,"stopped":null,"changed_at_ms":0}})).unwrap()
    }
    fn editor() -> Editor {
        let s = source();
        let selected = BTreeSet::from([s.view.source.id]);
        Editor {
            id: RequestId::new(),
            sources: vec![s],
            previous: BriefingGrant {
                conversation_id: ConversationId::new(),
                revision: 7,
                scope: None,
                changed_at_ms: NOW,
            },
            selected,
            fields: ["2026-11-05 00:00", "2026-11-06 19:00", "+08:00", "256"].map(|s| Composer {
                text: s.into(),
                cursor: s.len(),
            }),
            row: 0,
            focus: 0,
        }
    }
    fn live_editor() -> Editor {
        let mut e = editor();
        e.sources[0]
            .view
            .grant
            .scope
            .as_mut()
            .unwrap()
            .expires_at_ms = now() + 3600000;
        e.fields[0].text = date_text(now() - 86400000, 480).unwrap();
        e.fields[1].text = date_text(now() + 600000, 480).unwrap();
        e
    }
    #[tokio::test]
    async fn source_access_g_during_sync_is_not_lost_or_inserted_into_chat_draft() {
        let mut app = Model::new(ConversationId::new());
        app.navigate(Surface::Sources);
        app.input.insert("chat draft");
        app.live = Some(live::State::new());
        let client = Client::new(
            "http://127.0.0.1:1",
            "fixture-source-access-sync-credential",
        )
        .unwrap();
        let (tx, _rx) = mpsc::channel(8);
        let mut jobs = JoinSet::new();
        handle_key(
            &client,
            &mut app,
            &tx,
            &mut jobs,
            KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
        )
        .unwrap();
        assert_eq!(app.input.text, "chat draft");
        assert!(
            app.source_access.reopen,
            "owner requested metadata editor must resume after sync"
        );
        assert!(app.source_access.pending.is_none());
        assert!(jobs.is_empty());
        // A completed sync resumes only GETs. No old selection or approval survives.
        app.live = None;
        resume(&client, &mut app, &tx, &mut jobs);
        assert!(app.source_access.loading);
        assert!(app.source_access.pending.is_none());
        assert_eq!(jobs.len(), 1);
        app.navigate(Surface::Chat);
        assert!(!app.source_access.reopen && !app.source_access.loading);
    }
    #[test]
    fn source_access_sync_withdraws_old_review_and_navigation_cancels_reopen() {
        let e = live_editor();
        let mut r = preview(&e, false, e.previous.conversation_id).unwrap();
        r.read = true;
        let mut app = Model::new(e.previous.conversation_id);
        app.navigate(Surface::Sources);
        app.source_access.panel = Some(Panel {
            editor: Some(e),
            preview: Some(r),
        });
        live::retire(&mut app);
        assert!(app.source_access.panel.is_none());
        assert!(app.source_access.reopen);
        assert!(app.source_access.pending.is_none());
        app.back();
        assert!(!app.source_access.reopen);
    }
    #[tokio::test]
    async fn source_access_http_rejection_can_be_edited_and_lost_ack_recovers_without_post() {
        use axum::{
            Json, Router,
            extract::{Path, State as AxumState},
            http::StatusCode,
            routing::get,
        };
        use std::sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        };
        struct Server {
            grant: BriefingGrant,
            sources: Vec<ArchiveSourceStatus>,
            mode: AtomicUsize,
            posts: AtomicUsize,
            receipt: Mutex<Option<BriefingGrantReceipt>>,
        }
        let e = live_editor();
        let conversation = e.previous.conversation_id;
        let server = Arc::new(Server {
            grant: e.previous.clone(),
            sources: e.sources.clone(),
            mode: AtomicUsize::new(0),
            posts: AtomicUsize::new(0),
            receipt: Mutex::new(None),
        });
        let router=Router::new().route("/v1/archives/status",get(|AxumState(s):AxumState<Arc<Server>>|async move {Json(s.sources.clone())}))
            .route("/v1/briefing/{conversation}/grant",get(|AxumState(s):AxumState<Arc<Server>>|async move {Json(s.grant.clone())})
                .post(|AxumState(s):AxumState<Arc<Server>>,Json(q):Json<SetBriefingGrant>|async move {
                    s.posts.fetch_add(1,Ordering::SeqCst);
                    let mode=s.mode.load(Ordering::SeqCst);
                    if mode==0 {return (StatusCode::CONFLICT,Json(json!({"error":"state_conflict"})));}
                    if mode==3 {return (StatusCode::BAD_REQUEST,Json(json!({"error":"invalid_input"})));}
                    *s.receipt.lock().unwrap()=Some(BriefingGrantReceipt {request:q.clone(),grant:BriefingGrant {conversation_id:s.grant.conversation_id,revision:q.expected_revision+1,scope:q.scope,changed_at_ms:now()}});
                    // Operation committed, response could not be returned. This must stay uncertain.
                    (if mode==2 {StatusCode::BAD_REQUEST} else {StatusCode::INTERNAL_SERVER_ERROR},Json(json!({"error":"fixture_lost_ack"})))
                }))
            .route("/v1/briefing/{conversation}/grant/requests/{request}",get(|AxumState(s):AxumState<Arc<Server>>,Path((c,id)):Path<(ConversationId,RequestId)>|async move {
                s.receipt.lock().unwrap().clone().filter(|r|r.grant.conversation_id==c && r.request.request_id==id).map(Json).ok_or(StatusCode::NOT_FOUND)
            })).with_state(server.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let client = Client::new(
            &format!("http://{}", listener.local_addr().unwrap()),
            "fixture-source-access-http-credential",
        )
        .unwrap();
        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        let mut app = Model::new(conversation);
        app.navigate(Surface::Sources);
        let (tx, mut rx) = mpsc::channel(8);
        let mut jobs = JoinSet::new();
        for mode in 0..=3 {
            server.mode.store(mode, Ordering::SeqCst);
            let mut review = preview(&e, false, conversation).unwrap();
            review.read = true;
            app.source_access.panel = Some(Panel {
                editor: None,
                preview: Some(review),
            });
            key(
                &client,
                &mut app,
                &tx,
                &mut jobs,
                KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
            )
            .unwrap();
            let result = tokio::time::timeout(Duration::from_secs(5), rx.recv())
                .await
                .unwrap()
                .unwrap();
            let Complete::SourceAccessSent(result) = result else {
                panic!("unexpected completion");
            };
            let (p, result) = *result;
            complete(&mut app, p, result);
            if mode == 0 || mode == 3 {
                assert!(
                    !pending(&app),
                    "definite 409 rejection must allow a fresh review, not permanent pending/404"
                );
                assert!(app.status.contains("请求被拒绝"));
            } else if mode == 2 {
                assert!(
                    !pending(&app),
                    "400 after commit is recovered from its actual receipt"
                );
            } else {
                assert!(
                    pending(&app),
                    "500 after commit must not be treated as rejection"
                );
                app.navigate(Surface::Chat);
                recover(&client, &mut app, &tx, &mut jobs);
                let result = tokio::time::timeout(Duration::from_secs(5), rx.recv())
                    .await
                    .unwrap()
                    .unwrap();
                let Complete::SourceAccessSent(result) = result else {
                    panic!("unexpected recovery");
                };
                let (p, result) = *result;
                complete(&mut app, p, result);
                assert!(!pending(&app));
                assert_eq!(app.surface, Surface::Chat);
            }
        }
        assert_eq!(
            server.posts.load(Ordering::SeqCst),
            4,
            "recovery must be GET-only"
        );
        task.abort();
    }
    #[tokio::test]
    async fn source_access_review_requires_full_read_and_resize_invalidates_read_gate() {
        use ratatui::{Terminal, backend::TestBackend};
        let mut app = Model::new(ConversationId::new());
        app.navigate(Surface::Sources);
        let e = live_editor();
        let r = preview(&e, false, app.conversation).unwrap();
        assert!(r.pending.text.contains("不启动收集"));
        assert!(r.pending.text.contains("QQ 私聊是另一个会话"));
        app.source_access.panel = Some(Panel {
            editor: Some(e),
            preview: Some(r),
        });
        let c = Client::new(
            "http://127.0.0.1:1",
            "fixture-source-access-credential-123456",
        )
        .unwrap();
        let (tx, _rx) = mpsc::channel(8);
        let mut jobs = JoinSet::new();
        key(
            &c,
            &mut app,
            &tx,
            &mut jobs,
            KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
        )
        .unwrap();
        assert!(app.source_access.pending.is_none());
        assert!(jobs.is_empty());
        let mut terminal = Terminal::new(TestBackend::new(130, 65)).unwrap();
        terminal
            .draw(|f| {
                draw(
                    f,
                    &mut app,
                    ratatui::layout::Rect::new(0, 0, 130, 60),
                    ratatui::layout::Rect::new(0, 60, 130, 4),
                )
            })
            .unwrap();
        assert!(
            app.source_access
                .panel
                .as_ref()
                .unwrap()
                .preview
                .as_ref()
                .unwrap()
                .read
        );
        terminal
            .draw(|f| {
                draw(
                    f,
                    &mut app,
                    ratatui::layout::Rect::new(0, 0, 55, 12),
                    ratatui::layout::Rect::new(0, 12, 55, 4),
                )
            })
            .unwrap();
        assert!(
            !app.source_access
                .panel
                .as_ref()
                .unwrap()
                .preview
                .as_ref()
                .unwrap()
                .read
        );
        key(
            &c,
            &mut app,
            &tx,
            &mut jobs,
            KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
        )
        .unwrap();
        assert!(app.source_access.pending.is_none());
        assert!(jobs.is_empty());
    }
    #[test]
    fn source_access_uncertain_write_survives_navigation_and_never_accepts_forged_receipt() {
        let e = live_editor();
        let p = preview(&e, false, e.previous.conversation_id)
            .unwrap()
            .pending;
        let mut app = Model::new(p.conversation);
        app.source_access.pending = Some(p.clone());
        app.source_access.busy = true;
        complete(
            &mut app,
            p.clone(),
            Err(Failure::Uncertain(anyhow::anyhow!("lost response"))),
        );
        assert!(pending(&app));
        assert!(!app.source_access.busy);
        app.navigate(Surface::Chat);
        assert!(pending(&app));
        let mut receipt = BriefingGrantReceipt {
            request: p.input.clone(),
            grant: BriefingGrant {
                conversation_id: p.conversation,
                revision: p.input.expected_revision + 1,
                scope: p.input.scope.clone(),
                changed_at_ms: now(),
            },
        };
        receipt.grant.conversation_id = ConversationId::new();
        complete(&mut app, p.clone(), Ok(receipt.clone()));
        assert!(pending(&app));
        receipt.grant.conversation_id = p.conversation;
        complete(&mut app, p, Ok(receipt));
        assert!(!pending(&app));
        assert!(app.status.contains("该次阅读授权变更已记录"));
        assert!(app.turn.is_none());
    }
    #[test]
    fn source_access_clipboard_is_bound_to_exact_editor_and_field() {
        let mut app = Model::new(ConversationId::new());
        app.navigate(Surface::Sources);
        let mut e = live_editor();
        e.focus = 1;
        e.fields[0] = Composer::default();
        app.source_access.panel = Some(Panel {
            editor: Some(e),
            preview: None,
        });
        let p = paste::begin(&mut app).unwrap();
        app.source_access
            .panel
            .as_mut()
            .unwrap()
            .editor
            .as_mut()
            .unwrap()
            .focus = 2;
        let before = app
            .source_access
            .panel
            .as_ref()
            .unwrap()
            .editor
            .as_ref()
            .unwrap()
            .fields[1]
            .text
            .clone();
        paste::complete(&mut app, p, Ok("unexpected".into()));
        assert_eq!(
            app.source_access
                .panel
                .as_ref()
                .unwrap()
                .editor
                .as_ref()
                .unwrap()
                .fields[1]
                .text,
            before
        );
        app.source_access
            .panel
            .as_mut()
            .unwrap()
            .editor
            .as_mut()
            .unwrap()
            .focus = 1;
        let p = paste::begin(&mut app).unwrap();
        paste::complete(&mut app, p, Ok("2026-11-05 00:00".into()));
        assert_eq!(
            app.source_access
                .panel
                .as_ref()
                .unwrap()
                .editor
                .as_ref()
                .unwrap()
                .fields[0]
                .text,
            "2026-11-05 00:00"
        );
        let p = paste::begin(&mut app).unwrap();
        app.source_access
            .panel
            .as_mut()
            .unwrap()
            .editor
            .as_mut()
            .unwrap()
            .id = RequestId::new();
        paste::complete(&mut app, p, Ok("late".into()));
        assert!(
            !app.source_access
                .panel
                .as_ref()
                .unwrap()
                .editor
                .as_ref()
                .unwrap()
                .fields[0]
                .text
                .contains("late")
        );
        assert!(app.input.text.is_empty());
    }
    #[test]
    fn source_access_exact_scope_and_revoke_have_no_implicit_permissions() {
        let e = editor();
        let q = draft(&e, NOW, false).unwrap();
        assert_eq!(q.expected_revision, 7);
        let scope = q.scope.unwrap();
        assert_eq!(
            scope.sources,
            vec![ArchiveRecallSource {
                source_id: e.sources[0].view.source.id,
                grant_revision: 2
            }]
        );
        assert_eq!(scope.utc_offset_minutes, 480);
        assert_eq!(scope.max_records, 256);
        assert_eq!(scope.from_ms, 1_793_808_000_000);
        assert!(scope.expires_at_ms <= NOW + 86400000);
        assert!(draft(&e, NOW, true).unwrap().scope.is_none());
    }
    #[test]
    fn source_access_rejects_stale_missing_expired_and_broadened_targets() {
        let mut e = editor();
        e.selected.insert(ArchiveSourceId::new());
        assert!(draft(&e, NOW, false).is_err());
        e.selected = BTreeSet::from([e.sources[0].view.source.id]);
        e.sources[0].import_state = ArchiveImportState::Disabled;
        assert!(draft(&e, NOW, false).is_err());
        e.sources[0].import_state = ArchiveImportState::Enabled;
        e.sources[0]
            .view
            .grant
            .scope
            .as_mut()
            .unwrap()
            .expires_at_ms = NOW;
        assert!(draft(&e, NOW, false).is_err());
        e.sources[0]
            .view
            .grant
            .scope
            .as_mut()
            .unwrap()
            .expires_at_ms = NOW + 60000;
        assert!(draft(&e, NOW, false).is_err());
        assert!(
            draft(&e, NOW, true).is_ok(),
            "revocation must not depend on live sources"
        );
    }
    #[test]
    fn source_access_validates_dates_offsets_and_record_bounds() {
        for (field, bad) in [
            (0, "2026-02-30 12:00"),
            (0, "2027-01-01 12:00"),
            (1, "2026-11-05 12:00"),
            (2, "+14:01"),
            (2, "-00:99"),
            (3, "0"),
            (3, "4097"),
        ] {
            let mut e = editor();
            e.fields[field].text = bad.into();
            assert!(draft(&e, NOW, false).is_err(), "{bad}");
        }
        let mut e = editor();
        e.selected.clear();
        assert!(draft(&e, NOW, false).is_err());
    }
    #[test]
    fn source_access_recovery_requires_exact_request_conversation_revision_and_scope() {
        let e = editor();
        let q = draft(&e, NOW, false).unwrap();
        let mut r = BriefingGrantReceipt {
            request: q.clone(),
            grant: BriefingGrant {
                conversation_id: e.previous.conversation_id,
                revision: 8,
                scope: q.scope.clone(),
                changed_at_ms: NOW,
            },
        };
        assert!(matches_receipt(&q, e.previous.conversation_id, &r));
        r.request.request_id = RequestId::new();
        assert!(!matches_receipt(&q, e.previous.conversation_id, &r));
        r.request = q.clone();
        r.grant.conversation_id = ConversationId::new();
        assert!(!matches_receipt(&q, e.previous.conversation_id, &r));
        r.grant.conversation_id = e.previous.conversation_id;
        r.grant.scope = None;
        assert!(!matches_receipt(&q, e.previous.conversation_id, &r));
    }
}
