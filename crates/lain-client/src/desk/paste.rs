//! Explicit local clipboard paste. Never reads in the background or contacts Kernel.
use super::*;

#[derive(Clone, PartialEq, Eq)]
enum Target {
    SourceSetup(RequestId, usize),
    SourceAccess(RequestId, usize),
    Chat,
    Work(DeviceId, lain_wire::DeviceWorkspaceId),
    Followup(DeviceId, DeviceCommandId),
}
#[derive(Clone)]
pub(super) struct Pending {
    id: uuid::Uuid,
    generation: u64,
    conversation: ConversationId,
    target: Target,
    before: String,
    cursor: usize,
}
fn target(app: &Model) -> Option<Target> {
    if !live::fresh(app)
        || app.review.is_some()
        || app.remote_review.is_some()
        || app.cancel_review.is_some()
    {
        return None;
    }
    if app.source_setup.panel.is_some() {
        return source_setup::paste_target(app).map(|(id, field)| Target::SourceSetup(id, field));
    }
    if app.source_access.panel.is_some() {
        return source_access::paste_target(app).map(|(id, field)| Target::SourceAccess(id, field));
    }
    if let Some(p) = app.new_work.as_ref() {
        if p.choosing || p.preview.is_some() || app.new_work_pending.is_some() {
            return None;
        }
        return p
            .targets
            .get(p.selected)
            .map(|t| Target::Work(t.device.device.registration.id, t.workspace.id));
    }
    if let Some(p) = app.followup.as_ref() {
        return (p.preview.is_none() && app.followup_pending.is_none())
            .then_some(Target::Followup(p.target.device_id, p.target.id));
    }
    (app.surface == Surface::Chat && !app.text_view.reader && app.text_view.menu.is_none())
        .then_some(Target::Chat)
}
fn input(app: &mut Model) -> &mut Composer {
    if app.source_setup.panel.is_some() {
        source_setup::paste_input(app)
    } else if app.source_access.panel.is_some() {
        source_access::paste_input(app)
    } else if let Some(panel) = &mut app.new_work {
        &mut panel.input
    } else if let Some(panel) = &mut app.followup {
        &mut panel.input
    } else {
        &mut app.input
    }
}
pub(super) fn begin(app: &mut Model) -> Result<Pending> {
    ensure!(app.paste_pending.is_none(), "剪贴板读取中，请稍候");
    let target = target(app).context("请先进入可编辑的输入框，再按 F9 粘贴")?;
    let generation = app.generation;
    let conversation = app.conversation;
    let draft = input(app);
    let pending = Pending {
        id: uuid::Uuid::new_v4(),
        generation,
        conversation,
        target,
        before: draft.text.clone(),
        cursor: draft.cursor,
    };
    app.paste_pending = Some(pending.clone());
    Ok(pending)
}
fn read() -> Result<String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|_| anyhow::anyhow!("系统剪贴板暂不可用"))?;
    clipboard
        .get_text()
        .map_err(|_| anyhow::anyhow!("剪贴板没有可读取的文字"))
}
pub(super) fn request(
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
) -> Result<()> {
    let pending = begin(app)?;
    let out = tx.clone();
    app.status = "正在从本机剪贴板粘贴；不会发送任务".into();
    jobs.spawn(async move {
        let result = tokio::task::spawn_blocking(read)
            .await
            .unwrap_or_else(|_| Err(anyhow::anyhow!("剪贴板读取未完成，请重试")));
        let _ = out.send(Complete::Paste(pending, result)).await;
    });
    Ok(())
}
pub(super) fn complete(app: &mut Model, p: Pending, result: Result<String>) {
    if app.paste_pending.as_ref().is_none_or(|old| old.id != p.id) {
        return;
    }
    app.paste_pending = None;
    if app.generation != p.generation
        || app.conversation != p.conversation
        || target(app).as_ref() != Some(&p.target)
    {
        app.status = "输入位置已变化，未粘贴；需要时重新按 F9".into();
        return;
    }
    let text = match result {
        Ok(text) => text,
        Err(e) => {
            app.status = e.to_string();
            return;
        }
    };
    let limit = if p.target == Target::Chat {
        32768
    } else {
        8192
    };
    let draft = input(app);
    if draft.text != p.before || draft.cursor != p.cursor {
        app.status = "草稿已变化，未插入迟到的剪贴板文字；可重新按 F9".into();
        return;
    }
    if text.is_empty()
        || draft.text.len().saturating_add(text.len()) > limit
        || text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\r' | '\n' | '\t'))
    {
        app.status = "剪贴板为空、超出输入上限或含不支持的控制字符；未修改草稿".into();
        return;
    }
    draft.insert(&text);
    app.status = "已粘贴，保留换行；Enter 仍需按当前流程确认".into();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_paste_preserves_multiline_and_does_not_overwrite_edits_or_navigation() {
        let mut app = Model::new(ConversationId::new());
        app.input.insert("prefix ");
        let p = begin(&mut app).unwrap();
        assert!(begin(&mut app).is_err());
        complete(&mut app, p, Ok("first\r\nsecond\n".into()));
        assert_eq!(app.input.text, "prefix first\r\nsecond\n");
        let p = begin(&mut app).unwrap();
        app.input.insert("typed meanwhile");
        let before = app.input.text.clone();
        complete(&mut app, p, Ok("must not arrive".into()));
        assert_eq!(app.input.text, before);
        let p = begin(&mut app).unwrap();
        app.navigate(Surface::Work);
        complete(&mut app, p, Ok("must not arrive".into()));
        assert_eq!(app.input.text, before);
    }
    #[test]
    fn reader_and_oversized_paste_do_not_read_or_partially_modify_a_draft() {
        let mut app = Model::new(ConversationId::new());
        app.text_view.reader = true;
        assert!(begin(&mut app).is_err());
        app.text_view.reader = false;
        app.input.insert("keep");
        let p = begin(&mut app).unwrap();
        complete(&mut app, p, Ok("x".repeat(32769)));
        assert_eq!(app.input.text, "keep");
    }
}
