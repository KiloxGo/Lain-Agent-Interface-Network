//! Chat text presentation and local selection, separate from Kernel authority.
use super::rich::{Anchor, CopyKind, Document, Role};
use super::*;
use crossterm::event::{MouseButton, MouseEventKind};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use uuid::Uuid;

struct Preview {
    id: lain_wire::PreviewId,
    message: MessageId,
    sequence: u64,
    text: String,
}
#[derive(Clone, Copy)]
struct Selection {
    start: Anchor,
    end: Anchor,
    dragging: bool,
    origin_start: Anchor,
    origin_end: Anchor,
}
pub(super) struct View {
    pub doc: Document,
    layout: Option<Arc<rich::Layout>>,
    rendered_scroll: u16,
    selection: Option<Selection>,
    preview: Option<Preview>,
    epoch: Arc<AtomicU64>,
    clipboard: Option<clipboard::Worker>,
    pub menu: Option<usize>,
    menu_options: Vec<(Uuid, u64, CopyKind, String)>,
    note_options: Vec<(Uuid, u64, rich::NoteAction, String)>,
    notes_mode: bool,
    pub reader: bool,
    saved_position: Option<(u16, bool)>,
}
impl Default for View {
    fn default() -> Self {
        Self {
            doc: Document::default(),
            layout: None,
            rendered_scroll: 0,
            selection: None,
            preview: None,
            epoch: Arc::new(AtomicU64::new(0)),
            clipboard: None,
            menu: None,
            menu_options: vec![],
            note_options: vec![],
            notes_mode: false,
            reader: false,
            saved_position: None,
        }
    }
}
impl View {
    pub fn clear(&mut self) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
        self.doc.clear();
        self.layout = None;
        self.rendered_scroll = 0;
        self.selection = None;
        self.preview = None;
        self.menu = None;
        self.menu_options.clear();
        self.note_options.clear();
        self.notes_mode = false;
        self.reader = false;
        self.saved_position = None;
    }
    pub fn append(&mut self, text: &str) {
        self.doc.append(text);
    }
    pub fn snapshot(&mut self, turn: &ActorSnapshot) -> Result<()> {
        if turn.availability.is_some() {
            self.clear();
            reviewed_receipts(&mut self.doc, turn);
            return Ok(());
        }
        reviewed_receipts(&mut self.doc, turn);
        let mut ids = BTreeSet::new();
        for message in &turn.messages {
            if matches!(message.role, MessageRole::User | MessageRole::Assistant) {
                let synthetic=message.role==MessageRole::Assistant && turn.actions.iter().filter(|a|matches!(a.state,ActionState::Executed|ActionState::Unknown)).any(|a|a.result.as_ref().is_some_and(|r|{
                    serde_json::to_string(&Some(r)).is_ok_and(|v|message.content==format!("Kernel action accepted: {v}") || message.content==format!("Kernel action did not complete reliably; do not replay automatically: {v}"))
                }));
                if synthetic {
                    continue;
                }
                ids.insert(message.id);
                self.doc.upsert(
                    message.id.0,
                    if message.role == MessageRole::User {
                        Role::User
                    } else {
                        Role::Assistant
                    },
                    &message.content,
                    true,
                );
            }
        }
        if self
            .preview
            .as_ref()
            .is_some_and(|p| ids.contains(&p.message) || turn.state != ActorTurnState::Running)
            && let Some(p) = self.preview.take()
            && !ids.contains(&p.message)
        {
            self.doc.remove(p.message.0);
        }
        let mut metadata = turn.clone();
        // Renderer emits only Assistant speech. Keep authenticated User records
        // available to the shared exact-quotation review validator without
        // rendering a second copy of the conversation.
        metadata.messages.retain(|m| m.role == MessageRole::User);
        let mut renderer = Renderer::new(Vec::new());
        renderer.snapshot(&metadata)?;
        let text = renderer.drain_text();
        if !text.trim().is_empty() {
            activity::metadata(&mut self.doc, turn, &text);
        }
        Ok(())
    }
    pub fn preview(&mut self, event: &lain_wire::PreviewEvent) {
        use lain_wire::PreviewUpdate;
        match &event.update {
            PreviewUpdate::Started { text } => {
                if self
                    .preview
                    .as_ref()
                    .is_some_and(|p| p.id == event.preview_id && p.sequence > event.sequence)
                {
                    return;
                }
                if let Some(old) = self.preview.take()
                    && old.message != event.message_id
                {
                    self.doc.remove(old.message.0);
                }
                self.preview = Some(Preview {
                    id: event.preview_id,
                    message: event.message_id,
                    sequence: event.sequence,
                    text: text.clone(),
                });
            }
            PreviewUpdate::Delta { text } => {
                if let Some(p) = self.preview.as_mut().filter(|p| {
                    p.id == event.preview_id
                        && p.message == event.message_id
                        && event.sequence == p.sequence + 1
                }) {
                    p.text.push_str(text);
                    p.sequence = event.sequence;
                } else {
                    return;
                }
            }
            PreviewUpdate::Discarded { .. } => {
                if let Some(p) = self.preview.take() {
                    self.doc.remove(p.message.0);
                }
                return;
            }
            PreviewUpdate::Finished => return,
        }
        if let Some(p) = &self.preview
            && p.text.len() <= 16384
        {
            self.doc.upsert(p.message.0, Role::Preview, &p.text, false);
        }
    }
    pub fn clear_preview(&mut self) {
        if let Some(p) = self.preview.take() {
            self.doc.remove(p.message.0);
        }
    }
    fn copy(&mut self, text: Option<String>) -> String {
        let Some(text) = text else {
            return "没有可复制的已保存正文".into();
        };
        if self.clipboard.is_none() {
            match clipboard::Worker::new() {
                Ok(worker) => self.clipboard = Some(worker),
                Err(error) => return error,
            }
        }
        let ticket = clipboard::Ticket {
            epoch: self.epoch.load(Ordering::SeqCst),
            source_epoch: self.epoch.clone(),
        };
        match self.clipboard.as_mut().unwrap().submit(text, ticket) {
            Ok(()) => "正在复制到本机剪贴板…".into(),
            Err(error) => error,
        }
    }
    fn copy_entry(&mut self, id: Uuid, kind: CopyKind) -> String {
        let text = self.doc.copy(id, kind);
        self.copy(text)
    }
    fn copy_selection(&mut self) -> String {
        let text = self
            .selection
            .and_then(|s| self.doc.selected(s.start, s.end));
        self.copy(text)
    }
    pub fn poll(&mut self) -> Option<String> {
        let (epoch, result) = self.clipboard.as_mut()?.poll()?;
        if epoch != self.epoch.load(Ordering::SeqCst) {
            return None;
        }
        Some(match result {
            Ok(()) => "已复制到本机剪贴板".into(),
            Err(error) => error,
        })
    }
    pub fn copying(&self) -> bool {
        self.clipboard.as_ref().is_some_and(|w| w.busy)
    }
}
impl Drop for View {
    fn drop(&mut self) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
    }
}
pub(super) fn available(app: &Model) -> bool {
    (app.surface == Surface::Chat
        || (matches!(app.surface, Surface::Task(..)) && !app.work_diagnostics))
        && app.review.is_none()
        && app.remote_review.is_none()
        && app.cancel_review.is_none()
        && app.followup.is_none()
        && !app.source_invalidated
        && live::fresh(app)
}
// One synchronous interaction/render reuses the same reader implementation.
// The two owned Views keep separate documents, menus, selections and clipboard
// epochs. No callback awaits or starts work; restore the original View even if
// navigation changes during the callback.
fn current<T>(app: &mut Model, action: impl FnOnce(&mut Model) -> T) -> T {
    let work = matches!(app.surface, Surface::Task(..)) && !app.work_diagnostics;
    if work {
        std::mem::swap(&mut app.text_view, &mut app.work_text);
    }
    let result = action(app);
    if work {
        std::mem::swap(&mut app.text_view, &mut app.work_text);
    }
    result
}
pub(super) fn key(app: &mut Model, key: KeyEvent) -> bool {
    current(app, |app| key_inner(app, key))
}
fn key_inner(app: &mut Model, key: KeyEvent) -> bool {
    if !available(app) {
        return false;
    }
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let copy_key = ctrl && matches!(key.code, KeyCode::Char('c' | 'C'));
    if copy_key && app.text_view.selection.is_some() {
        app.status = app.text_view.copy_selection();
        return true;
    }
    if app.text_view.menu.is_some() {
        let options = &app.text_view.menu_options;
        let notes = app.text_view.notes_mode;
        let count = if notes {
            app.text_view.note_options.len()
        } else {
            options.len()
        };
        let selected = app.text_view.menu.unwrap();
        match key.code {
            KeyCode::Esc | KeyCode::F(2) => app.text_view.menu = None,
            KeyCode::Up => app.text_view.menu = Some(selected.saturating_sub(1)),
            KeyCode::Down => app.text_view.menu = Some((selected + 1).min(count.saturating_sub(1))),
            KeyCode::Enter => {
                if notes {
                    let choice = app.text_view.note_options.get(selected).cloned();
                    app.text_view.menu = None;
                    if let Some((id, version, action, _)) = choice {
                        activity::activate(app, id, version, action);
                    }
                    return true;
                }
                if let Some((id, version, kind, _)) = options.get(selected).cloned() {
                    app.status = if app.text_view.doc.version(id) == Some(version) {
                        app.text_view.copy_entry(id, kind)
                    } else {
                        "消息已更新，请重新打开复制菜单".into()
                    };
                }
                app.text_view.menu = None;
            }
            _ => {}
        }
        return true;
    }
    if key.code == KeyCode::F(7) && app.surface == Surface::Chat {
        app.text_view.note_options = app.text_view.doc.notes();
        app.text_view.notes_mode = true;
        app.text_view.menu = Some(0);
        return true;
    }
    if key.code == KeyCode::F(2) {
        open_copy(app);
        return true;
    }
    if key.code == KeyCode::F(3) || (app.text_view.reader && key.code == KeyCode::Esc) {
        app.text_view.reader = !app.text_view.reader;
        app.text_view.selection = None;
        if app.text_view.reader {
            app.text_view.saved_position = Some((app.scroll, app.follow));
        } else if let Some((scroll, follow)) = app.text_view.saved_position.take() {
            app.scroll = scroll;
            app.follow = follow;
        }
        return true;
    }
    if key.code == KeyCode::Esc && app.text_view.selection.take().is_some() {
        return true;
    }
    if ctrl && key.code == KeyCode::Char('o') {
        app.status = match app.text_view.doc.latest() {
            Some(id) => app.text_view.copy_entry(id, CopyKind::Plain),
            None => "尚无已保存的回复可复制".into(),
        };
        return true;
    }
    if app.text_view.reader {
        match key.code {
            KeyCode::PageUp => {
                app.follow = false;
                app.scroll = app.scroll.saturating_sub(12);
            }
            KeyCode::PageDown => {
                app.follow = false;
                app.scroll = app.scroll.saturating_add(12);
            }
            KeyCode::Home if ctrl => {
                app.follow = false;
                app.scroll = 0;
            }
            KeyCode::End if ctrl => app.follow = true,
            _ => {}
        }
        return true;
    }
    if !matches!(
        key.code,
        KeyCode::PageUp | KeyCode::PageDown | KeyCode::End | KeyCode::Home
    ) {
        app.text_view.selection = None;
    }
    false
}
pub(super) fn open_copy(app: &mut Model) {
    app.text_view.notes_mode = false;
    app.text_view.menu_options = app.text_view.doc.options();
    app.text_view.menu = Some(0);
}
pub(super) fn mouse(app: &mut Model, event: MouseEvent) -> bool {
    current(app, |app| mouse_inner(app, event))
}
fn mouse_inner(app: &mut Model, event: MouseEvent) -> bool {
    if !available(app) || app.text_view.menu.is_some() {
        return false;
    }
    let Some(area) = app.chat_viewport else {
        return false;
    };
    let Some(layout) = &app.text_view.layout else {
        return false;
    };
    let inside = area.contains((event.column, event.row).into());
    // Mouse coordinates belong to the last frame actually presented. A queued
    // scroll/auto-scroll request has not moved that frame yet.
    let row = usize::from(app.text_view.rendered_scroll)
        + usize::from(
            event
                .row
                .saturating_sub(area.y)
                .min(area.height.saturating_sub(1)),
        );
    let col = event
        .column
        .saturating_sub(area.x)
        .min(area.width.saturating_sub(1));
    match event.kind {
        MouseEventKind::Down(MouseButton::Left) if !inside => {
            app.text_view.selection = None;
            false
        }
        MouseEventKind::Down(MouseButton::Left) if inside => {
            if let Some(r) = layout.rows.get(row)
                && let Some(action) = r.note
                && col >= area.width.saturating_sub(14)
            {
                let (id, version) = (r.id, r.version);
                app.text_view.selection = None;
                activity::activate(app, id, version, action);
                return true;
            }
            if let Some(r) = layout.rows.get(row)
                && r.action.is_some()
                && col >= area.width.saturating_sub(14)
            {
                let (id, kind) = (r.id, r.action.unwrap());
                app.text_view.selection = None;
                app.status = app.text_view.copy_entry(id, kind);
                return true;
            }
            app.text_view.selection = layout.hit(row, col, false).map(|start| Selection {
                start,
                end: start,
                dragging: true,
                origin_start: start,
                origin_end: layout.hit(row, col, true).unwrap_or(start),
            });
            app.scroll = app.text_view.rendered_scroll;
            app.follow = false;
            true
        }
        MouseEventKind::Drag(MouseButton::Left)
            if app.text_view.selection.is_some_and(|s| s.dragging) =>
        {
            if event.row < area.y {
                app.scroll = app.scroll.saturating_sub(1);
            } else if event.row >= area.bottom() {
                app.scroll = app.scroll.saturating_add(1).min(app.chat_bottom);
            }
            let s = app.text_view.selection.unwrap();
            if let Some(point) = layout.hit(row, col, false)
                && let Some(forward) = app.text_view.doc.at_or_after(point, s.origin_start)
                && let Some(end) = layout.hit(row, col, forward)
            {
                let selection = app.text_view.selection.as_mut().unwrap();
                selection.start = if forward {
                    s.origin_start
                } else {
                    s.origin_end
                };
                selection.end = end;
            }
            true
        }
        MouseEventKind::Up(MouseButton::Left) => {
            if let Some(s) = app.text_view.selection.as_mut() {
                s.dragging = false;
                if s.start == s.end {
                    app.text_view.selection = None;
                }
            }
            true
        }
        _ => false,
    }
}
pub(super) fn draw(frame: &mut Frame, app: &mut Model, area: Rect) {
    if matches!(app.surface, Surface::Task(..)) {
        activity::prepare_output(app);
    }
    current(app, |app| draw_inner(frame, app, area));
}
fn draw_inner(frame: &mut Frame, app: &mut Model, area: Rect) {
    use unicode_width::UnicodeWidthStr;
    let content = Rect::new(
        area.x + 1,
        area.y + 1,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    );
    app.chat_viewport = Some(content);
    let old_layout = app.text_view.layout.clone();
    let anchor = if !app.follow && app.scroll == app.text_view.rendered_scroll {
        app.text_view
            .layout
            .as_ref()
            .and_then(|l| l.rows.get(usize::from(app.text_view.rendered_scroll)))
            .cloned()
    } else {
        None
    };
    let layout = app.text_view.doc.layout(content.width);
    if let Some(anchor) = anchor
        && old_layout
            .as_ref()
            .is_some_and(|old| !Arc::ptr_eq(old, &layout))
        && let Some(index) = layout.relocate(&anchor)
    {
        app.scroll = index.min(u16::MAX as usize) as u16;
    }
    if app.text_view.selection.is_some_and(|s| {
        app.text_view
            .doc
            .at_or_after(s.origin_start, s.origin_start)
            .is_none()
            || (app.text_view.doc.selected(s.start, s.end).is_none() && s.start != s.end)
    }) {
        app.text_view.selection = None;
    }
    app.chat_bottom = layout
        .rows
        .len()
        .saturating_sub(usize::from(content.height))
        .min(u16::MAX as usize) as u16;
    app.scroll = if app.follow {
        app.chat_bottom
    } else {
        app.scroll.min(app.chat_bottom)
    };
    app.text_view.rendered_scroll = app.scroll;
    let mut lines = Vec::new();
    for row in layout
        .rows
        .iter()
        .skip(usize::from(app.scroll))
        .take(usize::from(content.height))
    {
        if let Some(label) = &row.label {
            let button = match (row.note, row.action) {
                (Some(rich::NoteAction::Task(..)), _) => "[查看结果]",
                (Some(rich::NoteAction::Fold(false)), _) => "[展开]",
                (Some(rich::NoteAction::Fold(true)), _) => "[收起]",
                (_, Some(CopyKind::Code(_))) => "[复制代码]",
                (_, Some(_)) => "[复制]",
                (_, None) => "",
            };
            let padding = usize::from(content.width).saturating_sub(
                UnicodeWidthStr::width(label.as_str()) + UnicodeWidthStr::width(button),
            );
            lines.push(Line::from(vec![
                Span::styled(
                    label.clone(),
                    Style::default()
                        .fg(Color::Rgb(121, 185, 189))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" ".repeat(padding)),
                Span::styled(button, Style::default().fg(Color::Rgb(137, 145, 158))),
            ]));
        } else {
            let spans = row
                .glyphs
                .iter()
                .map(|g| {
                    let selected = app.text_view.selection.is_some_and(|s| {
                        app.text_view.doc.selected_glyph(
                            s.start,
                            s.end,
                            row.id,
                            row.version,
                            g.start..g.end,
                        )
                    });
                    Span::styled(
                        g.text.clone(),
                        if selected {
                            g.style.bg(Color::Rgb(45, 76, 82)).fg(Color::White)
                        } else {
                            g.style
                        },
                    )
                })
                .collect::<Vec<_>>();
            lines.push(Line::from(spans));
        }
    }
    frame.render_widget(
        Paragraph::new(if app.follow && matches!(app.surface, Surface::Task(..)) {
            "任务时间线 · 滚轮查看前文 · F2 复制 · F5 更早轮次 · F7 诊断"
        } else if app.follow {
            "Conversation / result · wheel · F2 copy · F3 read · F7 动态/诊断"
        } else {
            "Reading history · Ctrl-End latest · F2 copy"
        })
        .style(Style::default().fg(Color::Rgb(137, 145, 158))),
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
    );
    frame.render_widget(Paragraph::new(lines), content);
    app.text_view.layout = Some(layout);
    if let Some(selected) = app.text_view.menu {
        let notes = app.text_view.notes_mode;
        let labels = if notes {
            app.text_view
                .note_options
                .iter()
                .map(|(_, _, _, s)| s.clone())
                .collect::<Vec<_>>()
        } else {
            app.text_view
                .menu_options
                .iter()
                .map(|(_, _, _, s)| s.clone())
                .collect::<Vec<_>>()
        };
        let skip = selected.saturating_sub(usize::from(content.height).saturating_sub(5));
        let mut lines = vec![
            Line::raw(if notes {
                "动态 · ↑/↓ 选择 · Enter 查看/展开 · Esc 返回"
            } else {
                "复制已保存的回复 · ↑/↓ 选择 · Enter 复制 · Esc 返回"
            }),
            Line::raw(""),
        ];
        if labels.is_empty() {
            lines.push(Line::raw(if notes {
                "暂无新动态；历史任务请在 Work 查看。"
            } else {
                "尚无已保存的回复。未完成的预览不能作为最终回复复制。"
            }));
        }
        lines.extend(labels.iter().enumerate().skip(skip).map(|(i, label)| {
            Line::styled(
                format!("{} {label}", if i == selected { "›" } else { " " }),
                if i == selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Rgb(121, 185, 189))
                } else {
                    Style::default()
                },
            )
        }));
        frame.render_widget(ratatui::widgets::Clear, area);
        frame.render_widget(
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(if notes {
                " 动态 "
            } else {
                " Copy · 复制内容 "
            })),
            area,
        );
        app.chat_viewport = None;
    }
}

pub(super) fn reviewed_receipts(doc: &mut Document, turn: &ActorSnapshot) {
    for r in &turn.memory_change_receipts {
        let title = if r.operation == lain_wire::memory::MemoryOperation::Correct {
            "已更正保存事项"
        } else {
            "已忘记来源事项"
        };
        let details = format!(
            "{}。\n时间：{}\n记忆：{}\n动作：{}\n替代记录：{}\n仅记录当时实际操作；不证明当前正文可用，不会自动重放。",
            title,
            work_history::timestamp(r.recorded_at_ms),
            r.memory_id,
            r.action_id,
            r.replacement_memory_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "无".into())
        );
        doc.note(
            Uuid::new_v5(
                &Uuid::NAMESPACE_OID,
                format!("lain/memory-change/{}", r.action_id).as_bytes(),
            ),
            title,
            "仅当前会话 · 历史操作回执",
            Some(&details),
            None,
        );
    }
    for r in &turn.reviewed_memory_receipts {
        let details = format!(
            "已保存来源事项。\n时间：{}\n记忆：{}\n动作：{}\n仅记录当时实际保存；不表示正文仍可用，不会自动重放。",
            work_history::timestamp(r.recorded_at_ms),
            r.memory_id,
            r.action_id
        );
        doc.note(
            Uuid::new_v5(
                &Uuid::NAMESPACE_OID,
                format!("lain/reviewed-memory/{}", r.action_id).as_bytes(),
            ),
            "已保存来源事项",
            "仅当前会话 · 历史操作回执",
            Some(&details),
            None,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn withdrawn_source_removes_old_text_but_retains_compact_save_receipt() {
        let mut view = View::default();
        view.doc
            .upsert(Uuid::new_v4(), Role::Assistant, "已撤回的通知正文", true);
        let mut turn = super::super::tests::running(ConversationId::new());
        turn.availability = Some(lain_wire::ActorAvailability::SourceUnavailable);
        turn.error = Some("Source unavailable; previous content withheld.".into());
        let action = lain_wire::ActionId::new();
        turn.reviewed_memory_receipts
            .push(lain_wire::memory::ReviewedMemoryReceipt {
                action_id: action,
                actor_turn_id: turn.id,
                conversation_id: turn.conversation_id,
                candidate_id: lain_wire::CandidateId(action.0),
                memory_id: lain_wire::MemoryId::new(),
                recorded_at_ms: 1234567,
            });
        view.snapshot(&turn).unwrap();
        assert!(
            view.doc
                .entries
                .iter()
                .all(|e| !e.raw.contains("已撤回的通知正文"))
        );
        assert!(
            view.doc
                .notes()
                .iter()
                .any(|e| e.3.contains("已保存来源事项"))
        );
        let id = view.doc.entries[0].id;
        view.doc.toggle(id);
        assert!(
            view.doc.entries[0]
                .raw
                .contains(&turn.reviewed_memory_receipts[0].memory_id.to_string())
        );
    }
    fn long_markdown(app: &mut Model) {
        let body = (0..30)
            .map(|i| {
                format!(
                    "Paragraph {i:02} 中文 line A\n\n```text\ncode-{i:02}\n```\n\nAfter {i:02}\n\n"
                )
            })
            .collect::<String>();
        app.text_view
            .doc
            .upsert(Uuid::new_v4(), Role::Assistant, &body, true);
    }
    #[test]
    fn scrolling_across_blank_and_code_rows_does_not_reanchor_an_unchanged_layout() {
        let mut app = Model::new(ConversationId::new());
        long_markdown(&mut app);
        app.follow = false;
        let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
        terminal.draw(|f| draw(f, &mut app, f.area())).unwrap();
        let bottom = app.chat_bottom;
        for requested in 0..=bottom {
            app.scroll = requested;
            terminal.draw(|f| draw(f, &mut app, f.area())).unwrap();
            assert_eq!(
                app.scroll, requested,
                "render must honor explicit scrolling, including blank rows and code captions"
            );
        }
    }
    #[test]
    fn dragging_while_a_blank_line_is_at_the_top_does_not_move_the_viewport() {
        let mut app = Model::new(ConversationId::new());
        long_markdown(&mut app);
        let layout = app.text_view.doc.layout(98);
        let top = layout
            .rows
            .iter()
            .enumerate()
            .rev()
            .find(|(i, r)| *i < layout.rows.len() - 8 && r.label.is_none() && r.glyphs.is_empty())
            .unwrap()
            .0;
        let height = (layout.rows.len() - top + 2) as u16;
        let mut terminal = Terminal::new(TestBackend::new(100, height)).unwrap();
        terminal.draw(|f| draw(f, &mut app, f.area())).unwrap();
        assert_eq!(usize::from(app.scroll), top);
        let area = app.chat_viewport.unwrap();
        let relative = layout
            .rows
            .iter()
            .skip(top)
            .position(|r| !r.glyphs.is_empty())
            .unwrap() as u16;
        let event = |kind, column| MouseEvent {
            kind,
            column,
            row: area.y + relative,
            modifiers: KeyModifiers::NONE,
        };
        mouse(
            &mut app,
            event(MouseEventKind::Down(MouseButton::Left), area.x),
        );
        terminal.draw(|f| draw(f, &mut app, f.area())).unwrap();
        assert_eq!(
            usize::from(app.scroll),
            top,
            "starting a selection must freeze the currently visible page"
        );
        mouse(
            &mut app,
            event(MouseEventKind::Drag(MouseButton::Left), area.x + 5),
        );
        terminal.draw(|f| draw(f, &mut app, f.area())).unwrap();
        assert_eq!(usize::from(app.scroll), top);
        let s = app.text_view.selection.unwrap();
        assert!(
            app.text_view.doc.selected(s.start, s.end).unwrap().len() <= 20,
            "a short drag must not select a distant paragraph"
        );
    }
    #[test]
    fn background_updates_keep_empty_lines_and_individual_code_headings_anchored() {
        let mut app = Model::new(ConversationId::new());
        long_markdown(&mut app);
        app.follow = false;
        let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
        terminal.draw(|f| draw(f, &mut app, f.area())).unwrap();
        for code in [false, true] {
            let layout = app.text_view.layout.clone().unwrap();
            let row = layout
                .rows
                .iter()
                .position(|r| {
                    if code {
                        r.kind == rich::RowKind::Code(4)
                    } else {
                        r.kind == rich::RowKind::Body && r.offset > 10 && r.glyphs.is_empty()
                    }
                })
                .unwrap();
            app.scroll = row as u16;
            terminal.draw(|f| draw(f, &mut app, f.area())).unwrap();
            let anchor = app.text_view.layout.as_ref().unwrap().rows[row].clone();
            let id = Uuid::new_v4();
            app.text_view
                .doc
                .upsert(id, Role::System, "new background activity\nline two", false);
            app.text_view.doc.place_after(id, None);
            terminal.draw(|f| draw(f, &mut app, f.area())).unwrap();
            let current = &app.text_view.layout.as_ref().unwrap().rows[usize::from(app.scroll)];
            assert_eq!(
                (current.id, current.kind, current.offset),
                (anchor.id, anchor.kind, anchor.offset)
            );
        }
    }
    #[test]
    fn dragging_inside_the_first_row_uses_the_presented_frame_without_autoscroll() {
        let mut app = Model::new(ConversationId::new());
        long_markdown(&mut app);
        app.follow = false;
        let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
        terminal.draw(|f| draw(f, &mut app, f.area())).unwrap();
        let row = app
            .text_view
            .layout
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .enumerate()
            .find(|(i, r)| *i > 20 && !r.glyphs.is_empty())
            .unwrap()
            .0;
        app.scroll = row as u16;
        terminal.draw(|f| draw(f, &mut app, f.area())).unwrap();
        let area = app.chat_viewport.unwrap();
        let point = app
            .text_view
            .layout
            .as_ref()
            .unwrap()
            .hit(row, 0, false)
            .unwrap();
        // A queued wheel request has not yet changed the pixels under the pointer.
        app.scroll -= 3;
        let event = |kind, column| MouseEvent {
            kind,
            column,
            row: area.y,
            modifiers: KeyModifiers::NONE,
        };
        mouse(
            &mut app,
            event(MouseEventKind::Down(MouseButton::Left), area.x),
        );
        assert_eq!(app.text_view.selection.unwrap().origin_start, point);
        mouse(
            &mut app,
            event(MouseEventKind::Drag(MouseButton::Left), area.x + 5),
        );
        terminal.draw(|f| draw(f, &mut app, f.area())).unwrap();
        assert_eq!(
            usize::from(app.scroll),
            row,
            "the first visible text row is inside the viewport"
        );
    }
    #[test]
    fn copy_menu_keeps_its_message_target_when_background_history_advances() {
        let mut app = Model::new(ConversationId::new());
        let old = Uuid::new_v4();
        app.text_view
            .doc
            .upsert(old, Role::Assistant, "original menu target", true);
        open_copy(&mut app);
        app.text_view.doc.upsert(
            Uuid::new_v4(),
            Role::Assistant,
            "new background reply",
            true,
        );
        assert_eq!(app.text_view.menu_options[0].0, old);
        app.text_view
            .doc
            .upsert(old, Role::Assistant, "changed body", true);
        key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.text_view.clipboard.is_none());
        assert!(app.status.contains("已更新"));
    }
    #[tokio::test]
    async fn text_regression_empty_started_accepts_normal_stream_deltas() {
        let mut app = Model::new(ConversationId::new());
        let turn = super::super::tests::running(app.conversation);
        let tid = turn.id;
        app.snapshot(turn).unwrap();
        let client = Client::new(
            "http://127.0.0.1:1",
            "fixture-stream-owner-12345678901234567890",
        )
        .unwrap();
        let (tx, _rx) = mpsc::channel(8);
        let mut jobs = JoinSet::new();
        let pid = lain_wire::PreviewId::new();
        let mid = MessageId::new();
        for (sequence, update) in [
            (
                0,
                lain_wire::PreviewUpdate::Started {
                    text: String::new(),
                },
            ),
            (
                1,
                lain_wire::PreviewUpdate::Delta {
                    text: "visible stream".into(),
                },
            ),
        ] {
            handle_complete(
                &client,
                &mut app,
                &tx,
                &mut jobs,
                Complete::Actor(ActorUpdate::Preview(lain_wire::PreviewEvent {
                    turn_id: tid,
                    preview_id: pid,
                    message_id: mid,
                    sequence,
                    update,
                })),
            )
            .unwrap();
        }
        assert!(
            app.text_view
                .doc
                .entries
                .iter()
                .any(|e| e.role == Role::Preview && e.raw == "visible stream")
        );
    }
    #[test]
    fn text_regression_reverse_drag_includes_the_origin_glyph_and_cannot_restart_outside() {
        let mut app = Model::new(ConversationId::new());
        let id = Uuid::new_v4();
        app.text_view.doc.upsert(id, Role::Assistant, "abcd", true);
        let mut terminal = Terminal::new(TestBackend::new(50, 24)).unwrap();
        terminal
            .draw(|f| super::super::draw::render(f, &mut app))
            .unwrap();
        let area = app.chat_viewport.unwrap();
        let row = app
            .text_view
            .layout
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .position(|r| !r.glyphs.is_empty())
            .unwrap() as u16;
        let event = |kind, x| MouseEvent {
            kind,
            column: x,
            row: area.y + row,
            modifiers: KeyModifiers::NONE,
        };
        mouse(
            &mut app,
            event(MouseEventKind::Down(MouseButton::Left), area.x + 3),
        );
        mouse(
            &mut app,
            event(MouseEventKind::Drag(MouseButton::Left), area.x),
        );
        mouse(
            &mut app,
            event(MouseEventKind::Up(MouseButton::Left), area.x),
        );
        let s = app.text_view.selection.unwrap();
        assert_eq!(app.text_view.doc.selected(s.start, s.end).unwrap(), "abcd");
        mouse(
            &mut app,
            event(MouseEventKind::Down(MouseButton::Left), area.x - 1),
        );
        let handled = mouse(
            &mut app,
            event(MouseEventKind::Drag(MouseButton::Left), area.x + 1),
        );
        assert!(!handled);
    }
    use ratatui::{Terminal, backend::TestBackend};
    #[test]
    fn source_retirement_removes_rich_text_copy_targets_and_pending_copy_authority() {
        let mut app = Model::new(ConversationId::new());
        let id = Uuid::new_v4();
        app.text_view
            .doc
            .upsert(id, Role::Assistant, "private fixture", true);
        assert!(key(
            &mut app,
            KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE)
        ));
        app.input.insert("unsent draft");
        let epoch = app.text_view.epoch.clone();
        let previous = epoch.load(Ordering::SeqCst);
        super::super::live::retire(&mut app);
        assert!(epoch.load(Ordering::SeqCst) > previous);
        assert!(app.text_view.doc.copy(id, CopyKind::Markdown).is_none());
        assert!(app.text_view.menu.is_none() && app.text_view.menu_options.is_empty());
        assert!(app.text_view.layout.is_none() && app.text_view.selection.is_none());
        assert_eq!(app.input.text, "unsent draft");
    }
    #[test]
    fn drag_uses_message_text_not_sidebar_or_border_and_reader_keeps_draft() {
        let mut app = Model::new(ConversationId::new());
        let mut turn = super::super::tests::running(app.conversation);
        turn.state = ActorTurnState::Completed;
        let id = MessageId::new();
        let text = "中文段落👩‍💻不会把装饰边框复制进去，这段文字会在小窗口中自然折行。";
        turn.messages.push(lain_wire::ActorMessage {
            id,
            conversation_id: app.conversation,
            role: MessageRole::Assistant,
            content: text.into(),
            timestamp_ms: 1,
            status: lain_wire::MessageStatus::Completed,
        });
        app.snapshot(turn).unwrap();
        app.input.insert("草稿不会丢失");
        let mut terminal = Terminal::new(TestBackend::new(50, 24)).unwrap();
        terminal
            .draw(|f| super::super::draw::render(f, &mut app))
            .unwrap();
        let area = app.chat_viewport.unwrap();
        let layout = app.text_view.layout.clone().unwrap();
        let first = layout
            .rows
            .iter()
            .position(|r| r.id == id.0 && !r.glyphs.is_empty())
            .unwrap();
        let last = layout
            .rows
            .iter()
            .rposition(|r| r.id == id.0 && !r.glyphs.is_empty())
            .unwrap();
        let event = |kind, column, row| MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        };
        assert!(mouse(
            &mut app,
            event(
                MouseEventKind::Down(MouseButton::Left),
                area.x,
                area.y + first as u16
            )
        ));
        assert!(mouse(
            &mut app,
            event(
                MouseEventKind::Drag(MouseButton::Left),
                area.right() - 1,
                area.y + last as u16
            )
        ));
        mouse(
            &mut app,
            event(
                MouseEventKind::Up(MouseButton::Left),
                area.right() - 1,
                area.y + last as u16,
            ),
        );
        let s = app.text_view.selection.unwrap();
        assert_eq!(app.text_view.doc.selected(s.start, s.end).unwrap(), text);
        terminal.resize(Rect::new(0, 0, 80, 24)).unwrap();
        terminal
            .draw(|f| super::super::draw::render(f, &mut app))
            .unwrap();
        assert_eq!(app.text_view.doc.selected(s.start, s.end).unwrap(), text);
        assert!(key(
            &mut app,
            KeyEvent::new(KeyCode::F(3), KeyModifiers::NONE)
        ));
        assert!(app.text_view.reader);
        assert!(key(
            &mut app,
            KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)
        ));
        assert!(key(
            &mut app,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)
        ));
        assert!(!app.text_view.reader);
        assert_eq!(app.input.text, "草稿不会丢失");
        app.text_view.clear();
        assert!(
            app.text_view.selection.is_none()
                && app.text_view.doc.copy(id.0, CopyKind::Markdown).is_none()
        );
    }
    #[tokio::test]
    async fn copy_menu_enter_cannot_submit_chat_or_worker_permissions() {
        let mut app = Model::new(ConversationId::new());
        app.input.insert("do not send");
        let client = Client::new(
            "http://127.0.0.1:1",
            "fixture-copy-menu-owner-12345678901234567890",
        )
        .unwrap();
        let (tx, _rx) = mpsc::channel(8);
        let mut jobs = JoinSet::new();
        handle_key(
            &client,
            &mut app,
            &tx,
            &mut jobs,
            KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE),
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
        assert!(app.pending.is_none() && jobs.is_empty());
        assert_eq!(app.input.text, "do not send");
    }
    #[test]
    fn preview_sequence_gaps_cannot_become_final_copy_content() {
        let mut view = View::default();
        let id = MessageId::new();
        let p = lain_wire::PreviewId::new();
        let turn = lain_wire::ActorTurnId::new();
        let event = |sequence, update| lain_wire::PreviewEvent {
            turn_id: turn,
            preview_id: p,
            message_id: id,
            sequence,
            update,
        };
        view.preview(&event(
            0,
            lain_wire::PreviewUpdate::Started {
                text: "known".into(),
            },
        ));
        view.preview(&event(
            2,
            lain_wire::PreviewUpdate::Delta {
                text: "not contiguous".into(),
            },
        ));
        assert!(view.doc.copy(id.0, CopyKind::Plain).is_none());
        assert!(!view.doc.entries[0].raw.contains("not contiguous"));
    }
}
