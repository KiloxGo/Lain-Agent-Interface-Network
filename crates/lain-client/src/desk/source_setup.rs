//! Owner-operated registration, retained import and finite GET synchronization.
//! Each review applies one existing API decision, never a combined permission grant.
use super::*;
use lain_wire::{ArchiveSourceId, archives::*};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Kind {
    Register,
    Import(ArchiveSourceId),
    Sync(ArchiveSourceId),
}
#[derive(Clone, PartialEq, Eq, Debug)]
enum Change {
    Register(ArchiveSourceKey),
    Import(ArchiveSourceKey, ArchiveSourceId, SetArchiveGrant),
    Sync(ArchiveSourceKey, ArchiveSourceId, SetKnowledgeSync),
}
struct Editor {
    id: RequestId,
    kind: Kind,
    source: Option<ArchiveSourceStatus>,
    reader: Option<KnowledgeReaderStatus>,
    group: bool,
    fields: Vec<Composer>,
    focus: usize,
}
fn now() -> u64 {
    devices_view::now()
}
impl Kind {
    fn id(self) -> Option<ArchiveSourceId> {
        match self {
            Self::Register => None,
            Self::Import(id) | Self::Sync(id) => Some(id),
        }
    }
    fn title(self) -> &'static str {
        match self {
            Self::Register => "登记 QQ 来源",
            Self::Import(_) => "Lain 保存范围",
            Self::Sync(_) => "有限来源同步",
        }
    }
    fn labels(self) -> &'static [&'static str] {
        match self {
            Self::Register => &["观察账号 QQ 号", "群号 / 对方 QQ 号"],
            Self::Import(_) => &[
                "到期时间 YYYY-MM-DD HH:MM[:SS]",
                "时区，例如 +08:00",
                "文本保留分钟 1–44640",
                "本次授权最多导入记录 1–10000",
            ],
            Self::Sync(_) => &[
                "到期时间 YYYY-MM-DD HH:MM[:SS]",
                "时区，例如 +08:00",
                "读取间隔秒 5–86400",
                "最多 HTTP 读取请求 1–10000",
                "每次变更条数 1–32",
            ],
        }
    }
}
fn numeric(text: &str) -> Result<String> {
    let n: u64 = text.parse().context("QQ 号 / 群号需为正整数")?;
    ensure!(
        n > 0 && n.to_string() == text,
        "号码不能包含空格、正负号或前导零"
    );
    Ok(text.into())
}
fn draft(e: &Editor, now: u64, revoke: bool) -> Result<Change> {
    ensure!(e.fields.len() == e.kind.labels().len(), "字段不完整");
    if e.kind == Kind::Register {
        ensure!(!revoke, "登记不包含撤销操作");
        let account = numeric(&e.fields[0].text)?;
        let peer = numeric(&e.fields[1].text)?;
        return Ok(Change::Register(if e.group {
            ArchiveSourceKey::QqGroup {
                observer_account: account,
                group_id: peer,
            }
        } else {
            ArchiveSourceKey::QqPrivate {
                observer_account: account,
                peer_id: peer,
            }
        }));
    }
    let s = e.source.as_ref().context("来源尚未读取")?;
    let id = e.kind.id().unwrap();
    ensure!(
        s.view.source.id == id
            && s.view.grant.source_id == id
            && s.synchronization.source_id == id
            && matches!(
                s.view.source.key,
                ArchiveSourceKey::QqGroup { .. } | ArchiveSourceKey::QqPrivate { .. }
            ),
        "来源身份不匹配或不是 QQ 来源"
    );
    let expiry = if revoke {
        0
    } else {
        let offset = source_access::offset(&e.fields[1].text)?;
        let expiry = source_access::date(&e.fields[0].text, offset)?;
        ensure!(
            expiry > now && expiry <= now.saturating_add(31 * 86400000),
            "到期需在未来 31 天内"
        );
        expiry
    };
    match e.kind {
        Kind::Import(_) => {
            let scope = if revoke {
                None
            } else {
                let minutes: u64 = e.fields[2].text.parse().context("保留分钟需为数字")?;
                let max_records: u32 = e.fields[3].text.parse().context("记录上限需为数字")?;
                ensure!(
                    (1..=44640).contains(&minutes) && (1..=10000).contains(&max_records),
                    "保留分钟或记录上限超出范围"
                );
                Some(ArchiveScope {
                    expires_at_ms: expiry,
                    retention_ms: minutes * 60000,
                    max_records,
                })
            };
            Ok(Change::Import(
                s.view.source.key.clone(),
                id,
                SetArchiveGrant {
                    expected_revision: s.view.grant.revision,
                    scope,
                },
            ))
        }
        Kind::Sync(_) => {
            let scope = if revoke {
                None
            } else {
                let a = s
                    .view
                    .grant
                    .scope
                    .as_ref()
                    .context("需先明确设置 Lain 保存范围")?;
                ensure!(
                    matches!(s.import_state, ArchiveImportState::Enabled)
                        && s.view.grant.revision > 0
                        && a.expires_at_ms > now
                        && expiry <= a.expires_at_ms,
                    "同步到期不能晚于有效保存授权到期"
                );
                let seconds: u64 = e.fields[2].text.parse().context("间隔秒需为数字")?;
                let max_requests: u32 = e.fields[3].text.parse().context("读取请求数需为数字")?;
                let limit: u16 = e.fields[4].text.parse().context("变更条数需为数字")?;
                ensure!(
                    (5..=86400).contains(&seconds)
                        && (1..=10000).contains(&max_requests)
                        && (1..=32).contains(&limit),
                    "同步间隔、请求数或单次条数超出范围"
                );
                Some(KnowledgeSyncScope {
                    archive_revision: s.view.grant.revision,
                    expires_at_ms: expiry,
                    interval_ms: seconds * 1000,
                    max_requests,
                    limit,
                })
            };
            let reader = if revoke {
                None
            } else {
                let r = e
                    .reader
                    .as_ref()
                    .filter(|r| r.configured && r.credentials_available)
                    .context("服务器尚无可用的只读知识服务配置；请先配置读取端")?;
                Some(
                    r.revision
                        .as_ref()
                        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
                        .context("读取配置版本无效")?
                        .clone(),
                )
            };
            Ok(Change::Sync(
                s.view.source.key.clone(),
                id,
                SetKnowledgeSync {
                    expected_revision: s.synchronization.revision,
                    expected_reader_revision: reader,
                    scope,
                },
            ))
        }
        Kind::Register => unreachable!(),
    }
}

fn source_name(key: &ArchiveSourceKey) -> String {
    match key {
        ArchiveSourceKey::QqGroup {
            observer_account,
            group_id,
        } => format!("QQ 群 {group_id} · 观察账号 {observer_account}"),
        ArchiveSourceKey::QqPrivate {
            observer_account,
            peer_id,
        } => format!("QQ 私聊 {peer_id} · 观察账号 {observer_account}"),
        _ => "不支持的来源".into(),
    }
}
fn import_text(scope: Option<&ArchiveScope>) -> String {
    scope.map_or_else(
        || "关闭；停止接受新导入，旧授权版本的来源读取失效".into(),
        |s| {
            format!(
                "到期 {}\n文本保留 {} 分钟 · 本次授权最多 {} 条",
                work_history::timestamp(s.expires_at_ms),
                s.retention_ms / 60000,
                s.max_records
            )
        },
    )
}
fn sync_text(scope: Option<&KnowledgeSyncScope>) -> String {
    scope.map_or_else(
        || "关闭有限读取计划".into(),
        |s| {
            format!(
                "来源保存版本 {} · 到期 {}\n每 {} 秒最多 {} 条变更 · 总计最多 {} 次 HTTP 读取",
                s.archive_revision,
                work_history::timestamp(s.expires_at_ms),
                s.interval_ms / 1000,
                s.limit,
                s.max_requests
            )
        },
    )
}
#[derive(Clone)]
pub(super) struct Pending {
    id: RequestId,
    change: Change,
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
struct DraftFields {
    kind: Kind,
    key: Option<ArchiveSourceKey>,
    fields: Vec<(String, usize)>,
    focus: usize,
    group: bool,
}
#[derive(Default)]
pub(super) struct State {
    pub panel: Option<Panel>,
    intent: Option<Kind>,
    loading: bool,
    pending: Option<Pending>,
    busy: bool,
    draft: Option<DraftFields>,
    follow_result: bool,
}
pub(super) enum Failure {
    NotSent(anyhow::Error),
    Rejected(anyhow::Error),
    Uncertain(anyhow::Error),
}
pub(super) type Loaded = (ArchiveSourceStatus, Option<KnowledgeReaderStatus>);
fn review(e: &Editor, revoke: bool) -> Result<Preview> {
    let change = draft(e, now(), revoke)?;
    let text = match &change {
        Change::Register(key) => format!(
            "登记来源\n{}\n\n仅在 Kernel 保存这个来源身份。\n不会开启 QQ 收集、导入、同步、会话阅读或记忆。\n如果同一来源已登记，返回现有来源，原有权限保持原状。\n观察器与独立知识服务仍需分别选择相同账号和群 / 私聊。",
            source_name(key)
        ),
        Change::Import(key, id, q) => format!(
            "{}\n来源 {}\n\n原保存配置（版本 {}）\n{}\n\n确认后替换为\n{}\n\n此操作只决定 Lain 接受导入的期限、文本保留和记录上限。\n每次替换都会产生新的来源授权版本；旧版本的简报和依赖可能不可用，有限同步需重新选择当前版本。\n不会启动观察器或知识服务，不会启用定期同步、会话阅读或自动记忆。\n关闭后旧版本读取立即失效，正文按维护策略清理；不删除 QQ 平台消息。",
            source_name(key),
            id,
            q.expected_revision,
            import_text(e.source.as_ref().unwrap().view.grant.scope.as_ref()),
            import_text(q.scope.as_ref())
        ),
        Change::Sync(key, id, q) => format!(
            "{}\n来源 {}\n\n原同步配置（版本 {}，已用 {} 次）\n{}\n\n确认后替换为\n{}\n读取配置版本：{}\n\n启用后会从服务器已配置的只读知识服务复制该来源当前保留的片段及后续变更。\n不是只从此刻开始的采集，不按这张表单过滤历史时间；不需要历史片段时不要启用。\n已有游标按原版本/配置规则续接，缺少游标时从服务端保留的变更起点开始。\n新增计划有独立有限额度，不清除以前已用请求；失败、不确定、到期和用完后停止，不自动续期。\n只执行有界 HTTP GET，不调用模型，不启用上游 QQ 收集、其他会话阅读或自动记忆。\n停止同步不是删除已保存的消息；其来源新鲜度和独立保留规则仍生效。",
            source_name(key),
            id,
            q.expected_revision,
            e.source.as_ref().unwrap().synchronization.used_requests,
            sync_text(e.source.as_ref().unwrap().synchronization.scope.as_ref()),
            sync_text(q.scope.as_ref()),
            q.expected_reader_revision
                .as_deref()
                .unwrap_or("撤销不要求读取配置")
        ),
    };
    Ok(Preview::new(Pending {
        id: RequestId::new(),
        change,
        text: format!(
            "{text}\n\n请完整审阅。Ctrl-Y 只提交这一项变更；未知结果仅核对服务器版本记录，不重复提交。"
        ),
    }))
}
pub(super) fn close(app: &mut Model) {
    app.source_setup.panel = None;
    app.source_setup.intent = None;
    app.source_setup.loading = false;
    app.source_setup.draft = None;
    app.source_setup.follow_result = false;
}
pub(super) fn invalidate(app: &mut Model) {
    let follow_result = app.source_setup.follow_result || app.source_setup.panel.is_some();
    let draft = app
        .source_setup
        .panel
        .as_ref()
        .and_then(|p| p.editor.as_ref())
        .map(|e| DraftFields {
            kind: e.kind,
            key: e.source.as_ref().map(|s| s.view.source.key.clone()),
            fields: e
                .fields
                .iter()
                .map(|f| (f.text.clone(), f.cursor))
                .collect(),
            focus: e.focus,
            group: e.group,
        })
        .or_else(|| app.source_setup.draft.take());
    let intent = app.source_setup.intent.or_else(|| {
        app.source_setup
            .panel
            .as_ref()
            .and_then(|p| p.editor.as_ref())
            .map(|e| e.kind)
    });
    close(app);
    if app.source_setup.pending.is_none() {
        app.source_setup.intent = intent;
        app.source_setup.draft = draft;
    } else {
        app.source_setup.follow_result = follow_result;
    }
}
fn restore_draft(app: &mut Model, e: &mut Editor) -> bool {
    let Some(d) = app.source_setup.draft.take() else {
        return false;
    };
    if d.kind != e.kind
        || d.key != e.source.as_ref().map(|s| s.view.source.key.clone())
        || d.fields.len() != e.fields.len()
    {
        return false;
    }
    e.fields = d
        .fields
        .into_iter()
        .map(|(text, cursor)| Composer {
            cursor: if text.is_char_boundary(cursor) {
                cursor
            } else {
                text.len()
            },
            text,
        })
        .collect();
    e.focus = d.focus.min(e.fields.len() - 1);
    e.group = d.group;
    true
}
pub(super) fn pending(app: &Model) -> bool {
    app.source_setup.pending.is_some()
}
pub(super) fn accepts(app: &Model, kind: Kind) -> bool {
    app.source_setup.loading && app.source_setup.intent == Some(kind)
}
pub(super) fn waiting(app: &Model) -> bool {
    app.source_setup.panel.is_none()
        && (app.source_setup.loading || app.source_setup.intent.is_some())
}
pub(super) fn waiting_key(app: &mut Model, key: KeyEvent) {
    if key.code == KeyCode::Esc {
        close(app);
        app.status = "已关闭来源配置；尚未提交新的变更".into();
        return;
    }
    if let Some(d) = app.source_setup.draft.as_mut() {
        match key.code {
            KeyCode::Tab => d.focus = (d.focus + 1) % d.fields.len(),
            KeyCode::BackTab => d.focus = (d.focus + d.fields.len() - 1) % d.fields.len(),
            KeyCode::F(5) if d.kind == Kind::Register => d.group = !d.group,
            KeyCode::Enter => {}
            _ => {
                let (text, cursor) = std::mem::take(&mut d.fields[d.focus]);
                let mut field = Composer { text, cursor };
                field.key(key);
                d.fields[d.focus] = (field.text, field.cursor);
            }
        }
    }
    app.status = "正在重读最新来源配置；输入草稿保留，恢复后需重新审阅".into();
}
fn editor(
    kind: Kind,
    source: Option<ArchiveSourceStatus>,
    reader: Option<KnowledgeReaderStatus>,
) -> Result<Editor> {
    let o = 480;
    let fields = match kind {
        Kind::Register => vec![String::new(), String::new()],
        Kind::Import(_) => {
            let scope = source.as_ref().and_then(|s| s.view.grant.scope.as_ref());
            vec![
                source_access::date_text(scope.map_or(now() + 86400000, |s| s.expires_at_ms), o)?,
                source_access::offset_text(o),
                scope
                    .map_or(10080, |s| s.retention_ms / 60000)
                    .max(1)
                    .to_string(),
                scope.map_or(4096, |s| s.max_records).to_string(),
            ]
        }
        Kind::Sync(_) => {
            let s = source.as_ref().context("缺少来源状态")?;
            let scope = s.synchronization.scope.as_ref();
            let end = scope.map_or_else(
                || {
                    s.view
                        .grant
                        .scope
                        .as_ref()
                        .map_or(now() + 3600000, |a| a.expires_at_ms.min(now() + 86400000))
                },
                |s| s.expires_at_ms,
            );
            vec![
                source_access::date_text(end, o)?,
                source_access::offset_text(o),
                scope.map_or(20, |s| s.interval_ms / 1000).to_string(),
                scope.map_or(5000, |s| s.max_requests).to_string(),
                scope.map_or(16, |s| s.limit).to_string(),
            ]
        }
    }
    .into_iter()
    .map(|text| Composer {
        cursor: text.len(),
        text,
    })
    .collect();
    Ok(Editor {
        id: RequestId::new(),
        kind,
        source,
        reader,
        group: true,
        fields,
        focus: 0,
    })
}
pub(super) fn open(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
    kind: Kind,
) -> Result<()> {
    source_access::close(app);
    if !live::fresh(app) {
        app.source_setup.intent = Some(kind);
        app.status = "正在同步；恢复后重读来源配置，尚未修改授权".into();
        return Ok(());
    }
    ensure!(!app.source_setup.loading, "正在读取来源配置");
    app.generation += 1;
    app.refreshing = false;
    app.source_setup.intent = None;
    if app
        .source_setup
        .draft
        .as_ref()
        .is_some_and(|d| d.kind != kind)
    {
        app.source_setup.draft = None;
    }
    if let Some(p) = app.source_setup.pending.clone() {
        app.source_setup.panel = Some(Panel {
            editor: None,
            preview: Some(Preview::new(p)),
        });
        app.status = "上次来源变更结果待核对；Ctrl-R 只读服务器记录".into();
        return Ok(());
    }
    if kind == Kind::Register {
        let mut editor = editor(kind, None, None)?;
        restore_draft(app, &mut editor);
        app.source_setup.panel = Some(Panel {
            editor: Some(editor),
            preview: None,
        });
        app.status = "填写准确账号与群号 / 对方号码；登记不会启用接收".into();
        return Ok(());
    }
    app.source_setup.loading = true;
    app.source_setup.intent = Some(kind);
    let c = client.clone();
    let generation = app.generation;
    let out = live::read_sender(app, tx, jobs);
    live::spawn_read(app, jobs, async move {
        let result = async {
            let s = c
                .get(&format!(
                    "v1/archives/sources/{}/status",
                    kind.id().unwrap()
                ))
                .await?;
            let reader = if matches!(kind, Kind::Sync(_)) {
                Some(c.get("v1/archives/knowledge-reader").await?)
            } else {
                None
            };
            Ok((s, reader))
        }
        .await;
        let _ = out
            .send(Complete::SourceSetupOpened(
                generation,
                kind,
                Box::new(result),
            ))
            .await;
    });
    app.status = "正在读取该来源配置；尚未修改授权".into();
    Ok(())
}
pub(super) fn resume(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
) {
    if !app.source_setup.loading
        && live::fresh(app)
        && matches!(app.surface, Surface::Sources | Surface::Source(_))
        && let Some(kind) = app.source_setup.intent
        && let Err(e) = open(client, app, tx, jobs, kind)
    {
        app.status = e.to_string();
    }
}
pub(super) fn opened(app: &mut Model, kind: Kind, result: Result<Loaded>) -> Result<()> {
    app.source_setup.loading = false;
    app.source_setup.intent = None;
    let (source, reader) = result?;
    ensure!(Some(source.view.source.id) == kind.id(), "来源身份不匹配");
    let mut editor = editor(kind, Some(source), reader)?;
    let restored = restore_draft(app, &mut editor);
    app.source_setup.panel = Some(Panel {
        editor: Some(editor),
        preview: None,
    });
    app.status = if restored {
        "最新配置已重读，未发送字段草稿保留；需重新审阅后确认"
    } else {
        "修改这一项配置，Enter 审阅后才可确认"
    }
    .into();
    Ok(())
}

enum Observation {
    Recorded(ArchiveSourceId),
    NotRecorded,
    Unknown,
}
fn key_matches(s: &ArchiveSourceView, key: &ArchiveSourceKey, id: ArchiveSourceId) -> bool {
    s.source.id == id && s.grant.source_id == id && s.source.key == *key && !id.0.is_nil()
}
async fn observation(c: &Client, change: &Change) -> Result<Observation> {
    match change {
        Change::Register(key) => {
            let sources: Vec<ArchiveSourceView> = c.get("v1/archives/sources").await?;
            ensure!(sources.len() <= 64, "来源列表超出协议上限");
            let mut found = sources.iter().filter(|s| s.source.key == *key);
            let result = found.next();
            ensure!(found.next().is_none(), "来源身份重复");
            if let Some(s) = result {
                ensure!(key_matches(s, key, s.source.id), "来源回执身份不匹配");
                Ok(Observation::Recorded(s.source.id))
            } else {
                Ok(Observation::NotRecorded)
            }
        }
        Change::Import(key, id, q) => {
            let source: ArchiveSourceView = c.get(&format!("v1/archives/sources/{id}")).await?;
            ensure!(key_matches(&source, key, *id), "来源身份已改变");
            let rows: Vec<ArchiveGrant> =
                c.get(&format!("v1/archives/sources/{id}/grants")).await?;
            ensure!(
                rows.len() <= 100 && rows.iter().all(|g| g.source_id == *id),
                "保存记录身份无效"
            );
            if let Some(g) = rows
                .iter()
                .find(|g| Some(g.revision) == q.expected_revision.checked_add(1))
            {
                Ok(if g.scope == q.scope {
                    Observation::Recorded(*id)
                } else {
                    Observation::NotRecorded
                })
            } else {
                Ok(if source.grant.revision == q.expected_revision {
                    Observation::NotRecorded
                } else {
                    Observation::Unknown
                })
            }
        }
        Change::Sync(key, id, q) => {
            let source: ArchiveSourceView = c.get(&format!("v1/archives/sources/{id}")).await?;
            ensure!(key_matches(&source, key, *id), "来源身份已改变");
            let current: KnowledgeSyncGrant = c
                .get(&format!("v1/archives/sources/{id}/knowledge/sync"))
                .await?;
            let rows: Vec<KnowledgeSyncGrant> = c
                .get(&format!("v1/archives/sources/{id}/knowledge/sync/grants"))
                .await?;
            ensure!(
                current.source_id == *id
                    && rows.len() <= 100
                    && rows.iter().all(|g| g.source_id == *id),
                "同步记录身份无效"
            );
            if let Some(g) = rows
                .iter()
                .find(|g| Some(g.revision) == q.expected_revision.checked_add(1))
            {
                Ok(
                    if g.scope == q.scope
                        && q.expected_reader_revision
                            .as_ref()
                            .is_none_or(|r| g.reader_revision.as_ref() == Some(r))
                    {
                        Observation::Recorded(*id)
                    } else {
                        Observation::NotRecorded
                    },
                )
            } else {
                Ok(if current.revision == q.expected_revision {
                    Observation::NotRecorded
                } else {
                    Observation::Unknown
                })
            }
        }
    }
}
async fn preflight(c: &Client, change: &Change) -> Result<()> {
    match change {
        Change::Register(_) => Ok(()),
        Change::Import(key, id, q) => {
            let s: ArchiveSourceStatus = c.get(&format!("v1/archives/sources/{id}/status")).await?;
            ensure!(
                key_matches(&s.view, key, *id) && s.view.grant.revision == q.expected_revision,
                "来源保存配置已改变，请重新读取审阅"
            );
            if let Some(a) = &q.scope {
                ensure!(
                    a.expires_at_ms > now()
                        && a.expires_at_ms <= now().saturating_add(31 * 86400000),
                    "保存期限已失效"
                );
            }
            Ok(())
        }
        Change::Sync(key, id, q) => {
            let s: ArchiveSourceStatus = c.get(&format!("v1/archives/sources/{id}/status")).await?;
            ensure!(
                key_matches(&s.view, key, *id)
                    && s.synchronization.source_id == *id
                    && s.synchronization.revision == q.expected_revision,
                "同步配置已改变，请重新读取审阅"
            );
            if let Some(scope) = &q.scope {
                let a = s.view.grant.scope.as_ref().context("保存范围已关闭")?;
                ensure!(
                    matches!(s.import_state, ArchiveImportState::Enabled)
                        && scope.archive_revision == s.view.grant.revision
                        && scope.expires_at_ms > now()
                        && scope.expires_at_ms <= a.expires_at_ms,
                    "来源保存版本或期限已改变"
                );
                let r: KnowledgeReaderStatus = c.get("v1/archives/knowledge-reader").await?;
                ensure!(
                    r.configured
                        && r.credentials_available
                        && r.revision == q.expected_reader_revision,
                    "服务器只读配置或凭据已改变，请重新审阅"
                );
            }
            Ok(())
        }
    }
}
async fn post(c: &Client, change: &Change) -> std::result::Result<ArchiveSourceId, Failure> {
    preflight(c, change).await.map_err(Failure::NotSent)?;
    let response: Result<Value> = match change {
        Change::Register(key) => c.post("v1/archives/sources", Some(json!(key))).await,
        Change::Import(_, id, q) => {
            c.post(&format!("v1/archives/sources/{id}/grant"), Some(json!(q)))
                .await
        }
        Change::Sync(_, id, q) => {
            c.post(
                &format!("v1/archives/sources/{id}/knowledge/sync"),
                Some(json!(q)),
            )
            .await
        }
    };
    match response {
        Ok(value) => {
            let matched: Result<ArchiveSourceId> = (|| match change {
                Change::Register(key) => {
                    let s: ArchiveSourceView = serde_json::from_value(value)?;
                    ensure!(key_matches(&s, key, s.source.id), "登记回执不匹配");
                    Ok(s.source.id)
                }
                Change::Import(_, id, q) => {
                    let g: ArchiveGrant = serde_json::from_value(value)?;
                    ensure!(
                        g.source_id == *id
                            && Some(g.revision) == q.expected_revision.checked_add(1)
                            && g.scope == q.scope,
                        "保存回执不匹配"
                    );
                    Ok(*id)
                }
                Change::Sync(_, id, q) => {
                    let g: KnowledgeSyncGrant = serde_json::from_value(value)?;
                    ensure!(
                        g.source_id == *id
                            && Some(g.revision) == q.expected_revision.checked_add(1)
                            && g.scope == q.scope
                            && q.expected_reader_revision
                                .as_ref()
                                .is_none_or(|r| g.reader_revision.as_ref() == Some(r)),
                        "同步回执不匹配"
                    );
                    Ok(*id)
                }
            })();
            matched.map_err(Failure::Uncertain)
        }
        Err(e) => match e.downcast_ref::<crate::api::HttpFailure>().map(|e| e.0) {
            Some(401 | 403 | 404 | 409 | 422) => Err(Failure::Rejected(e)),
            Some(400) => match observation(c, change).await {
                Ok(Observation::Recorded(id)) => Ok(id),
                Ok(Observation::NotRecorded) => Err(Failure::Rejected(e)),
                _ => Err(Failure::Uncertain(e)),
            },
            _ => Err(Failure::Uncertain(e)),
        },
    }
}
pub(super) fn recover(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
) {
    let Some(p) = app
        .source_setup
        .pending
        .clone()
        .filter(|_| !app.source_setup.busy)
    else {
        return;
    };
    app.source_setup.busy = true;
    let c = client.clone();
    let out = live::effect_sender(app, tx, jobs);
    jobs.spawn(async move {
        let result = match observation(&c, &p.change).await {
            Ok(Observation::Recorded(id)) => Ok(id),
            Ok(Observation::NotRecorded) => Err(Failure::Uncertain(anyhow::anyhow!(
                "当前尚未找到同一配置，不证明在途请求不会稍后生效"
            ))),
            Ok(Observation::Unknown) => Err(Failure::Uncertain(anyhow::anyhow!(
                "历史范围不足，尚不能核对原变更"
            ))),
            Err(e) => Err(Failure::Uncertain(e)),
        };
        let _ = out
            .send(Complete::SourceSetupSent(Box::new((p, result))))
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
    if key.code == KeyCode::Esc {
        if app.source_setup.pending.is_none()
            && let Some(p) = app.source_setup.panel.as_mut()
            && p.editor.is_some()
            && p.preview.take().is_some()
        {
            return Ok(());
        }
        close(app);
        return Ok(());
    }
    if ctrl && key.code == KeyCode::Char('r') {
        if pending(app) {
            recover(client, app, tx, jobs);
        } else {
            let kind = app
                .source_setup
                .panel
                .as_ref()
                .and_then(|p| p.editor.as_ref())
                .context("没有可重读的配置")?
                .kind;
            close(app);
            open(client, app, tx, jobs, kind)?;
        }
        return Ok(());
    }
    if pending(app) {
        if let Some(r) = app
            .source_setup
            .panel
            .as_mut()
            .and_then(|p| p.preview.as_mut())
        {
            match key.code {
                KeyCode::PageUp => r.scroll = r.scroll.saturating_sub(r.page),
                KeyCode::PageDown => r.scroll = r.scroll.saturating_add(r.page),
                _ => {}
            }
        }
        if ctrl
            && key.code == KeyCode::Char('d')
            && !app.source_setup.busy
            && app
                .source_setup
                .panel
                .as_ref()
                .and_then(|p| p.preview.as_ref())
                .is_some_and(|r| r.read)
        {
            app.source_setup.pending = None;
            close(app);
            app.status =
                "已停止本机跟踪，原请求仍可能生效；这不是撤销。重新读取当前配置后再决定。".into();
        }
        return Ok(());
    }
    let p = app.source_setup.panel.as_mut().context("配置页已关闭")?;
    if let Some(r) = p.preview.as_mut() {
        match key.code {
            KeyCode::PageUp => r.scroll = r.scroll.saturating_sub(r.page),
            KeyCode::PageDown => r.scroll = r.scroll.saturating_add(r.page),
            KeyCode::Char('y') if ctrl && r.read => {
                ensure!(live::fresh(app), "连接已失效，需重新读取审阅");
                let pending = app
                    .source_setup
                    .panel
                    .as_ref()
                    .unwrap()
                    .preview
                    .as_ref()
                    .unwrap()
                    .pending
                    .clone();
                app.source_setup.pending = Some(pending.clone());
                app.source_setup.busy = true;
                app.source_setup.follow_result = true;
                if let Some(r) = app
                    .source_setup
                    .panel
                    .as_mut()
                    .and_then(|p| p.preview.as_mut())
                {
                    r.read = false;
                    r.seen_to = 0;
                    r.scroll = 0;
                    r.width = 0;
                }
                let c = client.clone();
                let out = live::effect_sender(app, tx, jobs);
                jobs.spawn(async move {
                    let result = post(&c, &pending.change).await;
                    let _ = out
                        .send(Complete::SourceSetupSent(Box::new((pending, result))))
                        .await;
                });
                app.status = "正在提交这一项来源变更；未知结果不会重发".into();
            }
            _ => {}
        }
    } else if let Some(e) = p.editor.as_mut() {
        match key.code {
            KeyCode::Tab => e.focus = (e.focus + 1) % e.fields.len(),
            KeyCode::BackTab => e.focus = (e.focus + e.fields.len() - 1) % e.fields.len(),
            KeyCode::F(5) if e.kind == Kind::Register => e.group = !e.group,
            KeyCode::Enter => {
                p.preview = Some(review(e, false)?);
                app.status = "尚未提交；请完整审阅实际范围".into();
            }
            KeyCode::F(8) if e.kind != Kind::Register => {
                p.preview = Some(review(e, true)?);
                app.status = "尚未停止；请审阅影响后决定".into();
            }
            _ => e.fields[e.focus].key(key),
        }
    }
    Ok(())
}
pub(super) fn complete(
    app: &mut Model,
    p: Pending,
    result: std::result::Result<ArchiveSourceId, Failure>,
) -> Option<ArchiveSourceId> {
    if app
        .source_setup
        .pending
        .as_ref()
        .is_none_or(|old| old.id != p.id || old.change != p.change)
    {
        return None;
    }
    app.source_setup.busy = false;
    match result {
        Ok(id) => {
            let reopen = (app.source_setup.follow_result || app.source_setup.panel.is_some())
                && matches!(app.surface, Surface::Sources | Surface::Source(_));
            app.source_setup.pending = None;
            close(app);
            app.status="服务器已找到同一配置的记录；当前状态以刷新为准。接收、保存、同步与会话阅读分别管理。".into();
            reopen.then_some(id)
        }
        Err(Failure::NotSent(e)) => {
            app.source_setup.pending = None;
            close(app);
            app.status = format!("尚未提交来源变更：{e}");
            None
        }
        Err(Failure::Rejected(e)) => {
            app.source_setup.pending = None;
            close(app);
            app.status = format!("该配置未记录或请求被拒绝：{e}；重新读取后可审阅");
            None
        }
        Err(Failure::Uncertain(e)) => {
            app.status = format!("来源变更结果待核对：{e}；Ctrl-R 只读版本记录，不重复提交");
            None
        }
    }
}
pub(super) fn paste_target(app: &Model) -> Option<(RequestId, usize)> {
    app.source_setup
        .panel
        .as_ref()
        .filter(|p| p.preview.is_none() && !pending(app))
        .and_then(|p| p.editor.as_ref())
        .map(|e| (e.id, e.focus))
}
pub(super) fn paste_input(app: &mut Model) -> &mut Composer {
    let e = app
        .source_setup
        .panel
        .as_mut()
        .unwrap()
        .editor
        .as_mut()
        .unwrap();
    &mut e.fields[e.focus]
}
pub(super) fn paste(app: &mut Model, text: &str) {
    if paste_target(app).is_some() {
        paste_input(app).insert(text);
    } else if let Some(d) = app.source_setup.draft.as_mut() {
        let (before, cursor) = std::mem::take(&mut d.fields[d.focus]);
        let mut field = Composer {
            text: before,
            cursor,
        };
        field.insert(text);
        d.fields[d.focus] = (field.text, field.cursor);
    }
}
pub(super) fn mouse(app: &mut Model, event: MouseEvent) {
    if let Some(r) = app
        .source_setup
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
        widgets::Paragraph,
    };
    let p = app.source_setup.panel.as_mut().unwrap();
    if let Some(r) = p.preview.as_mut() {
        let text = if app.source_setup.pending.is_some() {
            format!(
                "该次变更已发起，结果待核对；下文仅为已审阅范围，不会重新发送。\n\n{}\n\n若仍无法核对，Ctrl-D 只停止本机跟踪，不撤销原请求；在途请求可能稍后生效。\n继续操作前需重新读取当前配置，不得把没有找到记录当作从未提交。",
                r.pending.text
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
            " 审阅来源配置 · PgUp/PgDn ",
        );
        let width = area.width.saturating_sub(2);
        r.page = area.height.saturating_sub(2).max(1);
        if r.width != width {
            r.width = width;
            r.read = false;
            r.seen_to = 0;
        }
        if usize::from(r.scroll) <= r.seen_to {
            r.seen_to = r.seen_to.max(usize::from(r.scroll) + usize::from(r.page));
            r.read |= end;
        }
        frame.render_widget(
            Paragraph::new(if app.source_setup.pending.is_some() {
                if r.read && !app.source_setup.busy {
                    "Ctrl-R 只读记录 · Ctrl-D 停止跟踪（不撤销） · Esc 关闭"
                } else {
                    "Ctrl-R 只读版本记录 · PgDn 阅读 · Esc 关闭；不会重发"
                }
            } else if r.read {
                "Ctrl-Y 确认这一项变更 · Esc 返回修改"
            } else {
                "请完整阅读 · PgDn 翻页 · Esc 返回修改"
            }),
            hint,
        );
    } else if let Some(e) = p.editor.as_mut() {
        let regions = Layout::vertical([Constraint::Length(4), Constraint::Min(0)]).split(area);
        let subtitle = if e.kind == Kind::Register {
            format!(
                "类型：{} · F5 切换\n只登记身份，不启用接收、同步、阅读或记忆。",
                if e.group { "QQ 群" } else { "QQ 私聊" }
            )
        } else if matches!(e.kind, Kind::Sync(_)) {
            let state = match e.reader.as_ref() {
                Some(r) if r.configured && r.credentials_available => "配置与凭据可用",
                Some(r) if r.configured => "未找到读取凭据",
                _ => "未配置或配置无效",
            };
            format!(
                "{}\n读取端：{}（尚未检查连接）",
                source_name(&e.source.as_ref().unwrap().view.source.key),
                state
            )
        } else {
            format!(
                "{}\n只修改这一层配置；其他权限独立。",
                source_name(&e.source.as_ref().unwrap().view.source.key)
            )
        };
        frame.render_widget(
            Paragraph::new(format!("{}\n{}", e.kind.title(), safe_text(&subtitle))),
            regions[0],
        );
        let rows = Layout::vertical(vec![Constraint::Length(2); e.fields.len()]).split(regions[1]);
        for (i, label) in e.kind.labels().iter().enumerate() {
            let r = rows[i];
            frame.render_widget(
                Paragraph::new(format!("{}{label}", if e.focus == i { "› " } else { "  " })).style(
                    Style::default().fg(if e.focus == i {
                        Color::Rgb(121, 185, 189)
                    } else {
                        Color::DarkGray
                    }),
                ),
                ratatui::layout::Rect {
                    height: r.height.min(1),
                    ..r
                },
            );
            if r.height < 2 || r.width == 0 {
                continue;
            }
            let input = ratatui::layout::Rect {
                y: r.y + 1,
                height: 1,
                ..r
            };
            let prefix = safe_text(e.fields[i].text.get(..e.fields[i].cursor).unwrap_or(""));
            let cursor = unicode_width::UnicodeWidthStr::width(prefix.as_str());
            let left = cursor
                .saturating_sub(usize::from(input.width.saturating_sub(1)))
                .min(usize::from(u16::MAX)) as u16;
            frame.render_widget(
                Paragraph::new(safe_text(&e.fields[i].text)).scroll((0, left)),
                input,
            );
            if e.focus == i {
                frame.set_cursor_position((
                    input.x
                        + (cursor.saturating_sub(usize::from(left)))
                            .min(usize::from(input.width - 1)) as u16,
                    input.y,
                ));
            }
        }
        frame.render_widget(Paragraph::new("Tab 切换 · Ctrl-U 清空 · F9 粘贴 · Enter 审阅\nF8 审阅停止（登记除外） · Ctrl-R 重读 · Esc 关闭"),hint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const NOW: u64 = 1_793_880_000_000;
    fn source() -> ArchiveSourceStatus {
        let id = ArchiveSourceId::new();
        serde_json::from_value(json!({"view":{"source":{"id":id,"key":{"kind":"qq_group","observer_account":"1001","group_id":"3003"},"created_at_ms":1000},"grant":{"source_id":id,"revision":2,"scope":{"expires_at_ms":NOW+86400000,"retention_ms":600000,"max_records":1000},"changed_at_ms":NOW},"total_records":0,"current_grant_records":0,"committed_through":0,"coverage":"selected_visible_slice"},"observed_at_ms":NOW,"import_state":"enabled","latest_received_at_ms":null,"latest_reported_occurrence_at_ms":null,"retained_body_count":0,"receiving_now":null,"latest_read":null,"upstream_snapshot":null,"upstream_snapshot_fresh":false,"synchronization":{"source_id":id,"revision":4,"scope":null,"reader_revision":null,"used_requests":0,"next_at_ms":0,"last_read":null,"pending":false,"stopped":null,"changed_at_ms":NOW}})).unwrap()
    }
    fn editor(kind: Kind, fields: &[&str]) -> Editor {
        Editor {
            id: RequestId::new(),
            kind,
            source: None,
            reader: Some(KnowledgeReaderStatus {
                configured: true,
                credentials_available: true,
                revision: Some("a".repeat(64)),
            }),
            group: true,
            fields: fields
                .iter()
                .map(|s| Composer {
                    text: (*s).into(),
                    cursor: s.len(),
                })
                .collect(),
            focus: 0,
        }
    }
    fn live_editor(kind: Kind, source: ArchiveSourceStatus) -> Editor {
        let mut e = super::editor(
            kind,
            Some(source),
            Some(KnowledgeReaderStatus {
                configured: true,
                credentials_available: true,
                revision: Some("a".repeat(64)),
            }),
        )
        .unwrap();
        e.fields[0].text = source_access::date_text(now() + 60000, 480).unwrap();
        e
    }
    #[test]
    fn source_setup_result_follows_sync_but_never_hijacks_after_explicit_close() {
        for leave in [false, true] {
            let mut row = source();
            row.view.grant.scope.as_mut().unwrap().expires_at_ms = now() + 3600000;
            let e = live_editor(Kind::Import(row.view.source.id), row.clone());
            let r = review(&e, false).unwrap();
            let p = r.pending.clone();
            let mut app = Model::new(ConversationId::new());
            app.navigate(Surface::Sources);
            app.source_setup.panel = Some(Panel {
                editor: Some(e),
                preview: Some(r),
            });
            app.source_setup.pending = Some(p.clone());
            live::retire(&mut app);
            if leave {
                close(&mut app);
            }
            assert_eq!(
                complete(&mut app, p, Ok(row.view.source.id)),
                if leave {
                    None
                } else {
                    Some(row.view.source.id)
                },
                "sync must not lose the requested result, explicit close must cancel navigation"
            );
            assert!(!pending(&app));
        }
    }
    #[test]
    fn source_setup_draft_survives_metadata_sync_without_restoring_old_approval() {
        let mut row = source();
        row.view.grant.scope.as_mut().unwrap().expires_at_ms = now() + 3600000;
        let kind = Kind::Sync(row.view.source.id);
        let mut e = live_editor(kind, row.clone());
        e.fields[3].text = "2500".into();
        let mut r = review(&e, false).unwrap();
        r.read = true;
        let mut app = Model::new(ConversationId::new());
        app.navigate(Surface::Sources);
        app.source_setup.panel = Some(Panel {
            editor: Some(e),
            preview: Some(r),
        });
        live::retire(&mut app);
        row.synchronization.used_requests = 10;
        opened(
            &mut app,
            kind,
            Ok((
                row,
                Some(KnowledgeReaderStatus {
                    configured: true,
                    credentials_available: true,
                    revision: Some("b".repeat(64)),
                }),
            )),
        )
        .unwrap();
        let panel = app.source_setup.panel.as_ref().unwrap();
        assert_eq!(
            panel.editor.as_ref().unwrap().fields[3].text,
            "2500",
            "background source updates must not erase the user's unsent configuration"
        );
        assert!(panel.preview.is_none());
        assert!(!pending(&app));
        assert_eq!(
            panel
                .editor
                .as_ref()
                .unwrap()
                .reader
                .as_ref()
                .unwrap()
                .revision,
            Some("b".repeat(64))
        );
    }
    #[test]
    fn source_setup_waiting_keys_and_paste_stay_in_unsent_field_not_chat() {
        let mut row = source();
        row.view.grant.scope.as_mut().unwrap().expires_at_ms = now() + 3600000;
        let kind = Kind::Sync(row.view.source.id);
        let mut e = live_editor(kind, row.clone());
        e.focus = 3;
        e.fields[3] = Composer {
            text: "2500".into(),
            cursor: 4,
        };
        let mut app = Model::new(ConversationId::new());
        app.navigate(Surface::Sources);
        app.input.insert("chat draft");
        app.source_setup.panel = Some(Panel {
            editor: Some(e),
            preview: None,
        });
        live::retire(&mut app);
        let c = Client::new(
            "http://127.0.0.1:1",
            "fixture-source-waiting-credential-123456",
        )
        .unwrap();
        let (tx, _rx) = mpsc::channel(8);
        let mut jobs = JoinSet::new();
        handle_key(
            &c,
            &mut app,
            &tx,
            &mut jobs,
            KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE),
        )
        .unwrap();
        handle_paste(&mut app, "2");
        assert_eq!(app.input.text, "chat draft");
        assert!(!pending(&app));
        assert!(jobs.is_empty());
        opened(
            &mut app,
            kind,
            Ok((
                row,
                Some(KnowledgeReaderStatus {
                    configured: true,
                    credentials_available: true,
                    revision: Some("a".repeat(64)),
                }),
            )),
        )
        .unwrap();
        assert_eq!(
            app.source_setup
                .panel
                .as_ref()
                .unwrap()
                .editor
                .as_ref()
                .unwrap()
                .fields[3]
                .text,
            "250012"
        );
        assert!(app.source_setup.panel.as_ref().unwrap().preview.is_none());
    }
    struct Server {
        source: std::sync::Mutex<ArchiveSourceStatus>,
        reader: std::sync::Mutex<KnowledgeReaderStatus>,
        history: std::sync::Mutex<Vec<ArchiveGrant>>,
        mode: std::sync::atomic::AtomicUsize,
        posts: std::sync::atomic::AtomicUsize,
    }
    async fn server() -> (Client, std::sync::Arc<Server>, tokio::task::JoinHandle<()>) {
        use axum::{Json, Router, extract::State as AxumState, http::StatusCode, routing::get};
        use std::sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        };
        let mut row = source();
        row.view.grant.scope.as_mut().unwrap().expires_at_ms = now() + 3600000;
        let state = Arc::new(Server {
            source: Mutex::new(row),
            reader: Mutex::new(KnowledgeReaderStatus {
                configured: true,
                credentials_available: true,
                revision: Some("a".repeat(64)),
            }),
            history: Mutex::new(vec![]),
            mode: AtomicUsize::new(0),
            posts: AtomicUsize::new(0),
        });
        let router=Router::new()
            .route("/v1/archives/sources/{id}/status",get(|AxumState(s):AxumState<Arc<Server>>|async move{Json(s.source.lock().unwrap().clone())}))
            .route("/v1/archives/sources/{id}",get(|AxumState(s):AxumState<Arc<Server>>|async move{Json(s.source.lock().unwrap().view.clone())}))
            .route("/v1/archives/knowledge-reader",get(|AxumState(s):AxumState<Arc<Server>>|async move{Json(s.reader.lock().unwrap().clone())}))
            .route("/v1/archives/sources/{id}/grants",get(|AxumState(s):AxumState<Arc<Server>>|async move{Json(s.history.lock().unwrap().clone())}))
            .route("/v1/archives/sources/{id}/grant",axum::routing::post(|AxumState(s):AxumState<Arc<Server>>,Json(q):Json<SetArchiveGrant>|async move {
                s.posts.fetch_add(1,Ordering::SeqCst);let mode=s.mode.load(Ordering::SeqCst);
                if mode==2 {return (StatusCode::CONFLICT,Json(json!({"error":"state_conflict"})));}
                if mode==3 {return (StatusCode::BAD_REQUEST,Json(json!({"error":"invalid_input"})));}
                if mode==7 {return (StatusCode::INTERNAL_SERVER_ERROR,Json(json!({"error":"unknown"})));}
                let mut source=s.source.lock().unwrap();let g=ArchiveGrant {source_id:source.view.source.id,revision:q.expected_revision+1,scope:q.scope,changed_at_ms:now()};
                source.view.grant=g.clone();s.history.lock().unwrap().push(g.clone());
                match mode {1=>(StatusCode::INTERNAL_SERVER_ERROR,Json(json!({"error":"lost_ack"}))),4=>(StatusCode::BAD_REQUEST,Json(json!({"error":"response_unavailable"}))),_=>(StatusCode::OK,Json(json!(g)))}
            })).with_state(state.clone());
        let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let client = Client::new(
            &format!("http://{}", socket.local_addr().unwrap()),
            "fixture-source-setup-http-credential",
        )
        .unwrap();
        let task = tokio::spawn(async move {
            axum::serve(socket, router).await.unwrap();
        });
        (client, state, task)
    }
    #[tokio::test]
    async fn source_setup_http_commit_and_recovery_are_version_bound_without_repost() {
        use std::sync::atomic::Ordering;
        for mode in [0, 1, 2, 3, 4, 7] {
            let (client, s, task) = server().await;
            s.mode.store(mode, Ordering::SeqCst);
            let row = s.source.lock().unwrap().clone();
            let id = row.view.source.id;
            let e = live_editor(Kind::Import(id), row);
            let p = review(&e, false).unwrap().pending;
            let result = post(&client, &p.change).await;
            if mode == 0 || mode == 4 {
                assert_eq!(result.ok(), Some(id));
            } else if mode == 2 || mode == 3 {
                assert!(matches!(result, Err(Failure::Rejected(_))));
            } else {
                assert!(matches!(result, Err(Failure::Uncertain(_))));
                let mut app = Model::new(ConversationId::new());
                app.source_setup.pending = Some(p.clone());
                let (tx, mut rx) = mpsc::channel(8);
                let mut jobs = JoinSet::new();
                recover(&client, &mut app, &tx, &mut jobs);
                let completed = tokio::time::timeout(Duration::from_secs(5), rx.recv())
                    .await
                    .unwrap()
                    .unwrap();
                let Complete::SourceSetupSent(done) = completed else {
                    panic!()
                };
                let (p, result) = *done;
                complete(&mut app, p, result);
                assert_eq!(
                    pending(&app),
                    mode == 7,
                    "absence after an unknown POST is not proof of rejection"
                );
            }
            assert_eq!(
                s.posts.load(Ordering::SeqCst),
                1,
                "GET recovery must not re-post"
            );
            task.abort();
        }
    }
    #[tokio::test]
    async fn source_setup_preflight_blocks_stale_epoch_and_changed_reader_without_post() {
        use std::sync::atomic::Ordering;
        let (client, s, task) = server().await;
        let row = s.source.lock().unwrap().clone();
        let id = row.view.source.id;
        let e = live_editor(Kind::Import(id), row.clone());
        let p = review(&e, false).unwrap().pending;
        s.source.lock().unwrap().view.grant.revision += 1;
        assert!(matches!(
            post(&client, &p.change).await,
            Err(Failure::NotSent(_))
        ));
        s.source.lock().unwrap().view.grant.revision -= 1;
        let e = live_editor(Kind::Sync(id), row);
        let p = review(&e, false).unwrap().pending;
        s.reader.lock().unwrap().revision = Some("b".repeat(64));
        assert!(matches!(
            post(&client, &p.change).await,
            Err(Failure::NotSent(_))
        ));
        assert_eq!(s.posts.load(Ordering::SeqCst), 0);
        task.abort();
    }
    #[tokio::test]
    async fn source_setup_escape_during_get_discards_late_completion() {
        let (client, s, task) = server().await;
        let id = s.source.lock().unwrap().view.source.id;
        let mut app = Model::new(ConversationId::new());
        app.navigate(Surface::Sources);
        let (tx, mut rx) = mpsc::channel(8);
        let mut jobs = JoinSet::new();
        open(&client, &mut app, &tx, &mut jobs, Kind::Import(id)).unwrap();
        handle_key(
            &client,
            &mut app,
            &tx,
            &mut jobs,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        )
        .unwrap();
        assert!(app.source_setup.panel.is_none());
        let result = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .unwrap()
            .unwrap();
        handle_complete(&client, &mut app, &tx, &mut jobs, result).unwrap();
        assert!(
            app.source_setup.panel.is_none(),
            "a late metadata GET must not reopen the editor after explicit Esc"
        );
        task.abort();
    }
    #[tokio::test]
    async fn source_setup_unknown_tracking_can_only_be_abandoned_after_reading_warning() {
        use ratatui::{Terminal, backend::TestBackend};
        let mut row = source();
        row.view.grant.scope.as_mut().unwrap().expires_at_ms = now() + 3600000;
        let e = live_editor(Kind::Import(row.view.source.id), row);
        let r = review(&e, false).unwrap();
        let p = r.pending.clone();
        let mut app = Model::new(ConversationId::new());
        app.source_setup.panel = Some(Panel {
            editor: None,
            preview: Some(r),
        });
        app.source_setup.pending = Some(p);
        let c = Client::new(
            "http://127.0.0.1:1",
            "fixture-source-setup-tracking-credential",
        )
        .unwrap();
        let (tx, _rx) = mpsc::channel(8);
        let mut jobs = JoinSet::new();
        key(
            &c,
            &mut app,
            &tx,
            &mut jobs,
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
        )
        .unwrap();
        assert!(pending(&app));
        let mut terminal = Terminal::new(TestBackend::new(80, 16)).unwrap();
        for _ in 0..20 {
            terminal
                .draw(|f| {
                    draw(
                        f,
                        &mut app,
                        ratatui::layout::Rect::new(0, 0, 80, 12),
                        ratatui::layout::Rect::new(0, 12, 80, 4),
                    )
                })
                .unwrap();
            if app
                .source_setup
                .panel
                .as_ref()
                .unwrap()
                .preview
                .as_ref()
                .unwrap()
                .read
            {
                break;
            }
            key(
                &c,
                &mut app,
                &tx,
                &mut jobs,
                KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE),
            )
            .unwrap();
        }
        assert!(
            app.source_setup
                .panel
                .as_ref()
                .unwrap()
                .preview
                .as_ref()
                .unwrap()
                .read,
            "keyboard paging must reach the pending warning before abandoning tracking"
        );
        key(
            &c,
            &mut app,
            &tx,
            &mut jobs,
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
        )
        .unwrap();
        assert!(!pending(&app));
        assert!(app.status.contains("原请求仍可能生效"));
        assert!(jobs.is_empty());
    }
    #[tokio::test]
    async fn source_setup_review_clipboard_and_sync_do_not_apply_or_restore_old_scope() {
        use ratatui::{Terminal, backend::TestBackend};
        let mut app = Model::new(ConversationId::new());
        app.navigate(Surface::Sources);
        app.input.insert("chat draft");
        let mut row = source();
        row.view.grant.scope.as_mut().unwrap().expires_at_ms = now() + 3600000;
        let e = live_editor(Kind::Import(row.view.source.id), row);
        let r = review(&e, false).unwrap();
        app.source_setup.panel = Some(Panel {
            editor: Some(e),
            preview: None,
        });
        let p = paste::begin(&mut app).unwrap();
        app.source_setup
            .panel
            .as_mut()
            .unwrap()
            .editor
            .as_mut()
            .unwrap()
            .focus = 1;
        let before = app
            .source_setup
            .panel
            .as_ref()
            .unwrap()
            .editor
            .as_ref()
            .unwrap()
            .fields[1]
            .text
            .clone();
        paste::complete(&mut app, p, Ok("wrong field".into()));
        assert_eq!(
            app.source_setup
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
        app.source_setup.panel.as_mut().unwrap().preview = Some(r);
        let c = Client::new(
            "http://127.0.0.1:1",
            "fixture-source-setup-review-credential",
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
        assert!(!pending(&app));
        assert!(jobs.is_empty());
        let mut t = Terminal::new(TestBackend::new(135, 65)).unwrap();
        t.draw(|f| {
            draw(
                f,
                &mut app,
                ratatui::layout::Rect::new(0, 0, 135, 60),
                ratatui::layout::Rect::new(0, 60, 135, 4),
            )
        })
        .unwrap();
        assert!(
            app.source_setup
                .panel
                .as_ref()
                .unwrap()
                .preview
                .as_ref()
                .unwrap()
                .read
        );
        t.draw(|f| {
            draw(
                f,
                &mut app,
                ratatui::layout::Rect::new(0, 0, 55, 12),
                ratatui::layout::Rect::new(0, 12, 55, 4),
            )
        })
        .unwrap();
        assert!(
            !app.source_setup
                .panel
                .as_ref()
                .unwrap()
                .preview
                .as_ref()
                .unwrap()
                .read
        );
        live::retire(&mut app);
        assert!(app.source_setup.panel.is_none());
        assert!(app.source_setup.intent.is_some());
        app.navigate(Surface::Chat);
        assert!(app.source_setup.intent.is_none());
        assert_eq!(app.input.text, "chat draft");
    }
    #[test]
    fn source_setup_registration_has_exact_numeric_identity_and_no_grant() {
        let mut e = editor(Kind::Register, &["1001", "3003"]);
        assert_eq!(
            draft(&e, NOW, false).unwrap(),
            Change::Register(ArchiveSourceKey::QqGroup {
                observer_account: "1001".into(),
                group_id: "3003".into()
            })
        );
        e.group = false;
        assert!(matches!(
            draft(&e, NOW, false).unwrap(),
            Change::Register(ArchiveSourceKey::QqPrivate { .. })
        ));
        for bad in [
            "0",
            "03003",
            "+3003",
            " 3003",
            "3003\n",
            "18446744073709551616",
        ] {
            e.fields[1].text = bad.into();
            assert!(draft(&e, NOW, false).is_err(), "{bad}");
        }
        assert!(draft(&e, NOW, true).is_err());
    }
    #[test]
    fn source_setup_import_and_sync_preserve_versions_and_independent_finite_limits() {
        let s = source();
        let id = s.view.source.id;
        let mut e = editor(
            Kind::Import(id),
            &["2026-11-06 19:00", "+08:00", "10", "1000"],
        );
        e.source = Some(s.clone());
        let Change::Import(_, got, q) = draft(&e, NOW, false).unwrap() else {
            panic!()
        };
        assert_eq!(got, id);
        assert_eq!(q.expected_revision, 2);
        assert_eq!(q.scope.as_ref().unwrap().retention_ms, 600000);
        let Change::Import(_, _, q) = draft(&e, NOW, true).unwrap() else {
            panic!()
        };
        assert!(q.scope.is_none());
        let mut e = editor(
            Kind::Sync(id),
            &["2026-11-06 19:00", "+08:00", "20", "100", "16"],
        );
        e.source = Some(s);
        let Change::Sync(_, got, q) = draft(&e, NOW, false).unwrap() else {
            panic!()
        };
        assert_eq!(got, id);
        assert_eq!(q.expected_revision, 4);
        let scope = q.scope.unwrap();
        assert_eq!(scope.archive_revision, 2);
        assert_eq!(scope.interval_ms, 20000);
        assert_eq!(scope.max_requests, 100);
        assert_eq!(scope.limit, 16);
        let Change::Sync(_, _, q) = draft(&e, NOW, true).unwrap() else {
            panic!()
        };
        assert!(q.scope.is_none());
    }
    #[test]
    fn source_setup_rejects_mismatched_source_expiry_and_unbounded_sync() {
        let s = source();
        let mut e = editor(
            Kind::Sync(s.view.source.id),
            &["2026-11-06 19:00", "+08:00", "20", "100", "16"],
        );
        e.source = Some(s);
        for (field, bad) in [
            (0, "2026-11-07 20:00"),
            (2, "0"),
            (2, "4"),
            (2, "86401"),
            (3, "0"),
            (3, "10001"),
            (4, "0"),
            (4, "33"),
        ] {
            let previous = e.fields[field].text.clone();
            e.fields[field].text = bad.into();
            assert!(draft(&e, NOW, false).is_err(), "{bad}");
            e.fields[field].text = previous;
        }
        e.source.as_mut().unwrap().view.grant.scope = None;
        assert!(draft(&e, NOW, false).is_err());
        assert!(draft(&e, NOW, true).is_ok());
        e.kind = Kind::Sync(ArchiveSourceId::new());
        assert!(draft(&e, NOW, true).is_err());
    }
}
