//! Source-backed, bounded terminal text. Screen decoration is never copy content.
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use std::{
    ops::Range,
    sync::{Arc, OnceLock},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Role {
    User,
    Assistant,
    System,
    Preview,
    Worker,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NoteAction {
    Fold(bool),
    Task(lain_wire::DeviceId, lain_wire::DeviceCommandId),
}
struct Fold {
    summary: String,
    details: String,
    expanded: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CopyKind {
    Plain,
    Markdown,
    Code(usize),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Anchor {
    pub id: Uuid,
    pub version: u64,
    pub offset: usize,
}
#[derive(Clone)]
struct Run {
    range: Range<usize>,
    style: Style,
}
#[derive(Clone)]
struct Decoration {
    offset: usize,
    label: String,
    code: usize,
}
#[derive(Default, Clone)]
struct Parsed {
    plain: String,
    runs: Vec<Run>,
    codes: Vec<String>,
    decorations: Vec<Decoration>,
    code_ranges: Vec<Range<usize>>,
    truncated: bool,
}
impl Parsed {
    fn text(&mut self, text: &str, style: Style) {
        if text.is_empty() {
            return;
        }
        let remaining = 65536_usize.saturating_sub(self.plain.len());
        let text = if text.len() > remaining {
            self.truncated = true;
            let mut end = remaining;
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            &text[..end]
        } else {
            text
        };
        if text.is_empty() {
            return;
        }
        let start = self.plain.len();
        self.plain.push_str(text);
        self.runs.push(Run {
            range: start..self.plain.len(),
            style,
        });
    }
    fn breaks(&mut self, n: usize) {
        if self.plain.is_empty() {
            return;
        }
        let current = self.plain.chars().rev().take_while(|c| *c == '\n').count();
        for _ in current..n {
            self.text("\n", Style::default());
        }
    }
}
pub(super) struct Entry {
    pub id: Uuid,
    pub version: u64,
    pub role: Role,
    pub raw: String,
    pub copyable: bool,
    parsed: Parsed,
    title: Option<String>,
    fold: Option<Fold>,
    link: Option<NoteAction>,
}
#[derive(Default)]
pub(super) struct Document {
    pub entries: Vec<Entry>,
    generation: u64,
    cache: Option<(u64, u16, Arc<Layout>)>,
}
#[derive(Clone)]
pub(super) struct Glyph {
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub column: u16,
    pub width: u16,
    pub style: Style,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RowKind {
    Heading,
    Code(usize),
    Body,
}
#[derive(Clone)]
pub(super) struct Row {
    pub id: Uuid,
    pub version: u64,
    pub glyphs: Vec<Glyph>,
    pub label: Option<String>,
    pub action: Option<CopyKind>,
    pub note: Option<NoteAction>,
    pub copyable: bool,
    pub kind: RowKind,
    /// Parsed source position, including empty lines; never inferred as zero.
    pub offset: usize,
}
#[derive(Default)]
pub(super) struct Layout {
    pub rows: Vec<Row>,
}
impl Layout {
    pub fn relocate(&self, old: &Row) -> Option<usize> {
        self.rows
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                r.id == old.id
                    && match (&r.kind, &old.kind) {
                        (RowKind::Heading, RowKind::Heading) => true,
                        (RowKind::Code(a), RowKind::Code(b)) => a == b,
                        (RowKind::Body, RowKind::Body) => r.offset <= old.offset,
                        _ => false,
                    }
            })
            .max_by_key(|(_, r)| r.offset)
            .map(|(i, _)| i)
    }
    pub fn hit(&self, row: usize, column: u16, trailing: bool) -> Option<Anchor> {
        let r = self.rows.get(row)?;
        if !r.copyable || r.glyphs.is_empty() {
            return None;
        }
        let g = r
            .glyphs
            .iter()
            .find(|g| column < g.column.saturating_add(g.width))
            .unwrap_or(r.glyphs.last()?);
        Some(Anchor {
            id: r.id,
            version: r.version,
            offset: if trailing { g.end } else { g.start },
        })
    }
}
impl Document {
    pub fn clear(&mut self) {
        self.entries.clear();
        self.changed();
    }
    fn changed(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.cache = None;
    }
    pub fn remove(&mut self, id: Uuid) {
        if self.entries.iter().any(|e| e.id == id) {
            self.entries.retain(|e| e.id != id);
            self.changed();
        }
    }
    pub fn place_after(&mut self, id: Uuid, after: Option<Uuid>) {
        let Some(index) = self.entries.iter().position(|e| e.id == id) else {
            return;
        };
        if (index == 0 && after.is_none())
            || (index > 0 && after == Some(self.entries[index - 1].id))
        {
            return;
        }
        let entry = self.entries.remove(index);
        let index = after
            .and_then(|a| self.entries.iter().position(|e| e.id == a).map(|i| i + 1))
            .unwrap_or(0);
        self.entries.insert(index, entry);
        self.changed();
    }
    pub fn note_positions(&self) -> Vec<(Uuid, Option<Uuid>)> {
        let mut after = None;
        let mut positions = vec![];
        for e in &self.entries {
            if e.link.is_some() || e.fold.is_some() {
                positions.push((e.id, after));
            }
            if matches!(e.role, Role::User | Role::Assistant) {
                after = Some(e.id);
            }
        }
        positions
    }
    pub fn upsert(&mut self, id: Uuid, role: Role, raw: &str, copyable: bool) {
        let raw = crate::tui::safe_text(&raw.replace("\r\n", "\n"));
        if raw.len() > 32768 {
            // Invalidate an older version/preview instead of keeping stale text
            // available for copy when the authoritative replacement is too large.
            self.upsert(id, Role::System, "消息超过本地正文显示上限（32 KiB）；完整消息仍保留在 Kernel 历史中。本条不提供截断复制。", false);
            return;
        }
        if let Some(e) = self.entries.iter().find(|e| e.id == id)
            && e.raw == raw
            && e.role == role
            && e.copyable == copyable
        {
            return;
        }
        let parsed = parse(&raw, role);
        if let Some(e) = self.entries.iter_mut().find(|e| e.id == id) {
            e.version = e.version.wrapping_add(1);
            e.raw = raw;
            e.role = role;
            e.copyable = copyable;
            e.parsed = parsed;
        } else {
            self.entries.push(Entry {
                id,
                version: 1,
                role,
                raw,
                copyable,
                parsed,
                title: None,
                fold: None,
                link: None,
            });
        }
        self.trim();
        self.changed();
    }
    fn trim(&mut self) {
        while self.entries.len() > 256
            || self
                .entries
                .iter()
                .map(|e| {
                    e.raw.len()
                        + e.title.as_ref().map_or(0, String::len)
                        + e.fold
                            .as_ref()
                            .map_or(0, |f| f.summary.len() + f.details.len())
                })
                .sum::<usize>()
                > 131072
            || self
                .entries
                .iter()
                .map(|e| e.parsed.plain.len())
                .sum::<usize>()
                > 262144
        {
            self.entries.remove(0);
        }
    }
    pub fn title(&mut self, id: Uuid, title: &str) {
        let title = crate::tui::safe_text(title)
            .chars()
            .take(100)
            .collect::<String>();
        if let Some(e) = self.entries.iter_mut().find(|e| e.id == id)
            && e.title.as_deref() != Some(&title)
        {
            e.title = Some(title);
            self.trim();
            self.changed();
        }
    }
    pub fn note(
        &mut self,
        id: Uuid,
        title: &str,
        summary: &str,
        details: Option<&str>,
        link: Option<NoteAction>,
    ) {
        let summary = crate::tui::safe_text(summary)
            .chars()
            .take(300)
            .collect::<String>();
        let details = details.map(|d| {
            let mut text = crate::tui::safe_text(d)
                .chars()
                .take(4000)
                .collect::<String>();
            if d.chars().count() > 4000 {
                text.push_str("\n详情显示有界；其余记录请查看 Kernel history。");
            }
            text
        });
        let expanded = self
            .entries
            .iter()
            .find(|e| e.id == id)
            .and_then(|e| e.fold.as_ref())
            .is_some_and(|f| f.expanded);
        let raw = if expanded {
            format!("{summary}\n\n{}", details.as_deref().unwrap_or_default())
        } else {
            summary.clone()
        };
        self.upsert(id, Role::System, &raw, false);
        if let Some(e) = self.entries.iter_mut().find(|e| e.id == id) {
            e.title = Some(crate::tui::safe_text(title).chars().take(100).collect());
            e.fold = details.map(|details| Fold {
                summary,
                details,
                expanded,
            });
            e.link = link;
        }
        self.trim();
        self.changed();
    }
    pub fn toggle(&mut self, id: Uuid) {
        let Some(e) = self.entries.iter_mut().find(|e| e.id == id) else {
            return;
        };
        let Some(f) = e.fold.as_mut() else { return };
        f.expanded = !f.expanded;
        let raw = if f.expanded {
            format!("{}\n\n{}", f.summary, f.details)
        } else {
            f.summary.clone()
        };
        self.upsert(id, Role::System, &raw, false);
    }
    pub fn notes(&self) -> Vec<(Uuid, u64, NoteAction, String)> {
        self.entries
            .iter()
            .rev()
            .filter_map(|e| {
                let action = e
                    .link
                    .or_else(|| e.fold.as_ref().map(|f| NoteAction::Fold(f.expanded)))?;
                let heading = e.title.clone().unwrap_or_else(|| "动态".into());
                let detail = e
                    .raw
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .chars()
                    .take(18)
                    .collect::<String>();
                let suffix = match action {
                    NoteAction::Task(_, command) => format!(
                        " · {}",
                        command.to_string().chars().take(8).collect::<String>()
                    ),
                    _ => String::new(),
                };
                Some((
                    e.id,
                    e.version,
                    action,
                    format!("{heading} · {detail}{suffix}"),
                ))
            })
            .take(40)
            .collect()
    }
    pub fn append(&mut self, text: &str) {
        if !text.trim().is_empty() {
            self.upsert(Uuid::new_v4(), Role::System, text, false);
        }
    }
    pub fn copy(&self, id: Uuid, kind: CopyKind) -> Option<String> {
        let e = self.entries.iter().find(|e| e.id == id && e.copyable)?;
        let text = match kind {
            CopyKind::Plain => e.parsed.plain.trim_end_matches('\n').into(),
            CopyKind::Markdown => e.raw.clone(),
            CopyKind::Code(n) => e.parsed.codes.get(n)?.clone(),
        };
        (!text.is_empty()).then_some(text)
    }
    pub fn latest(&self) -> Option<Uuid> {
        self.entries
            .iter()
            .rev()
            .find(|e| matches!(e.role, Role::Assistant | Role::Worker) && e.copyable)
            .map(|e| e.id)
    }
    pub fn version(&self, id: Uuid) -> Option<u64> {
        self.entries.iter().find(|e| e.id == id).map(|e| e.version)
    }
    pub fn options(&self) -> Vec<(Uuid, u64, CopyKind, String)> {
        let mut options = Vec::new();
        for e in self
            .entries
            .iter()
            .rev()
            .filter(|e| matches!(e.role, Role::Assistant | Role::Worker) && e.copyable)
            .take(20)
        {
            let title = e
                .parsed
                .plain
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(28)
                .collect::<String>();
            for (kind, label) in [
                (CopyKind::Plain, "纯文本"),
                (CopyKind::Markdown, "Markdown 原文"),
            ] {
                options.push((e.id, e.version, kind, format!("{label} · {title}")));
            }
            for n in 0..e.parsed.codes.len().min(32) {
                options.push((
                    e.id,
                    e.version,
                    CopyKind::Code(n),
                    format!("代码块 {} · {title}", n + 1),
                ));
            }
        }
        options.truncate(128);
        options
    }
    pub fn selected(&self, a: Anchor, b: Anchor) -> Option<String> {
        let ai = self
            .entries
            .iter()
            .position(|e| e.id == a.id && e.version == a.version && e.copyable)?;
        let bi = self
            .entries
            .iter()
            .position(|e| e.id == b.id && e.version == b.version && e.copyable)?;
        let (start, end, si, ei) = if (ai, a.offset) <= (bi, b.offset) {
            (a, b, ai, bi)
        } else {
            (b, a, bi, ai)
        };
        let mut out = String::new();
        for (i, e) in self.entries.iter().enumerate().take(ei + 1).skip(si) {
            if !e.copyable {
                continue;
            }
            let from = if i == si { start.offset } else { 0 };
            let to = if i == ei {
                end.offset
            } else {
                e.parsed.plain.len()
            };
            let text = e.parsed.plain.get(from..to)?;
            if !out.is_empty() && i > si {
                out.push_str("\n\n");
            }
            out.push_str(text);
        }
        (!out.is_empty()).then_some(out)
    }
    pub fn at_or_after(&self, a: Anchor, b: Anchor) -> Option<bool> {
        let index = |point: Anchor| {
            self.entries
                .iter()
                .position(|e| e.id == point.id && e.version == point.version && e.copyable)
        };
        Some((index(a)?, a.offset) >= (index(b)?, b.offset))
    }
    pub fn selected_glyph(
        &self,
        a: Anchor,
        b: Anchor,
        id: Uuid,
        version: u64,
        range: Range<usize>,
    ) -> bool {
        let index = |id| self.entries.iter().position(|e| e.id == id);
        let (Some(ai), Some(bi), Some(i)) = (index(a.id), index(b.id), index(id)) else {
            return false;
        };
        if self.entries[i].version != version
            || self.entries[ai].version != a.version
            || self.entries[bi].version != b.version
        {
            return false;
        }
        let ((si, start), (ei, end)) = if (ai, a.offset) <= (bi, b.offset) {
            ((ai, a.offset), (bi, b.offset))
        } else {
            ((bi, b.offset), (ai, a.offset))
        };
        i >= si && i <= ei && (i > si || range.end > start) && (i < ei || range.start < end)
    }
    pub fn layout(&mut self, width: u16) -> Arc<Layout> {
        let width = width.max(2);
        if let Some((g, w, l)) = &self.cache
            && *g == self.generation
            && *w == width
        {
            return l.clone();
        }
        let mut layout = Layout::default();
        for e in &self.entries {
            let label = match e.role {
                Role::User => "You",
                Role::Assistant => "Lain",
                Role::System => "System",
                Role::Preview => "Lain · 未完成预览",
                Role::Worker => "Codex · 设备回传结果",
            };
            let label = e.title.as_deref().unwrap_or(label);
            layout.rows.push(Row {
                id: e.id,
                version: e.version,
                glyphs: vec![],
                label: Some(if e.parsed.truncated {
                    format!("{label} · 显示有界截断，Markdown保留完整原文")
                } else {
                    label.into()
                }),
                action: (e.copyable && matches!(e.role, Role::Assistant | Role::Worker))
                    .then_some(CopyKind::Plain),
                note: e
                    .link
                    .or_else(|| e.fold.as_ref().map(|f| NoteAction::Fold(f.expanded))),
                copyable: false,
                kind: RowKind::Heading,
                offset: 0,
            });
            let blank = |offset| Row {
                id: e.id,
                version: e.version,
                glyphs: vec![],
                label: None,
                action: None,
                note: None,
                copyable: e.copyable,
                kind: RowKind::Body,
                offset,
            };
            let mut row = blank(0);
            let mut col = 0;
            let mut decor = e.parsed.decorations.iter().peekable();
            let mut run_index = 0;
            let mut words = e.parsed.plain.split_word_bound_indices().peekable();
            for (offset, g) in e.parsed.plain.grapheme_indices(true) {
                while decor.peek().is_some_and(|d| d.offset <= offset) {
                    if !row.glyphs.is_empty() {
                        layout.rows.push(row);
                        row = blank(offset);
                        col = 0;
                    }
                    let d = decor.next().unwrap();
                    layout.rows.push(Row {
                        id: e.id,
                        version: e.version,
                        glyphs: vec![],
                        label: Some(d.label.clone()),
                        action: e.copyable.then_some(CopyKind::Code(d.code)),
                        note: None,
                        copyable: false,
                        kind: RowKind::Code(d.code),
                        offset: d.offset,
                    });
                }
                if g == "\n" {
                    layout.rows.push(row);
                    row = blank(offset + g.len());
                    col = 0;
                    continue;
                }
                let w = if g == "\t" {
                    4 - col % 4
                } else {
                    UnicodeWidthStr::width(g).max(1).min(u16::MAX as usize) as u16
                };
                let in_code = e.parsed.code_ranges.iter().any(|r| r.contains(&offset));
                let line_width = if in_code { width } else { width.min(96) };
                while words
                    .peek()
                    .is_some_and(|(start, word)| start + word.len() <= offset)
                {
                    words.next();
                }
                let wrap_word = !in_code
                    && words.peek().is_some_and(|(start, word)| {
                        if *start != offset || word.chars().all(char::is_whitespace) {
                            return false;
                        }
                        let word_width = UnicodeWidthStr::width(*word);
                        word_width <= usize::from(line_width)
                            && usize::from(col) + word_width > usize::from(line_width)
                    });
                if (col + w > line_width || wrap_word) && col > 0 {
                    layout.rows.push(row);
                    row = blank(offset);
                    col = 0;
                }
                let w = if g == "\t" { 4 - col % 4 } else { w };
                while e
                    .parsed
                    .runs
                    .get(run_index)
                    .is_some_and(|r| r.range.end <= offset)
                {
                    run_index += 1;
                }
                let style = e
                    .parsed
                    .runs
                    .get(run_index)
                    .map_or(Style::default(), |r| r.style);
                row.glyphs.push(Glyph {
                    text: if g == "\t" {
                        " ".repeat(usize::from(w))
                    } else if UnicodeWidthStr::width(g) == 0 {
                        "�".into()
                    } else {
                        g.into()
                    },
                    start: offset,
                    end: offset + g.len(),
                    column: col,
                    width: w,
                    style,
                });
                col += w;
            }
            if !row.glyphs.is_empty() {
                layout.rows.push(row);
            }
            layout.rows.push(blank(e.parsed.plain.len()));
        }
        let result = Arc::new(layout);
        self.cache = Some((self.generation, width, result.clone()));
        result
    }
}
#[cfg(test)]
mod size_regression {
    use super::*;
    #[test]
    fn oversized_replacement_cannot_leave_old_text_available_to_copy() {
        let mut doc = Document::default();
        let id = Uuid::new_v4();
        doc.upsert(id, Role::Assistant, "old source", true);
        let previous = doc.version(id).unwrap();
        doc.upsert(id, Role::Assistant, &"x".repeat(32769), true);
        assert!(doc.version(id).unwrap() > previous);
        assert!(doc.copy(id, CopyKind::Markdown).is_none());
        assert!(doc.entries[0].raw.contains("上限"));
    }
    #[test]
    fn prose_wrap_keeps_short_english_words_together_and_retains_source() {
        let mut doc = Document::default();
        doc.upsert(Uuid::new_v4(), Role::Assistant, "one two", true);
        let layout = doc.layout(6);
        let rows = layout
            .rows
            .iter()
            .filter(|r| !r.glyphs.is_empty())
            .map(|r| r.glyphs.iter().map(|g| g.text.as_str()).collect::<String>())
            .collect::<Vec<_>>();
        assert_eq!(rows, ["one ", "two"]);
    }
    #[test]
    fn changed_anchor_does_not_highlight_the_replacement_body() {
        let mut doc = Document::default();
        let id = Uuid::new_v4();
        doc.upsert(id, Role::Assistant, "old", true);
        let a = Anchor {
            id,
            version: doc.version(id).unwrap(),
            offset: 0,
        };
        let b = Anchor { offset: 3, ..a };
        doc.upsert(id, Role::Assistant, "new", true);
        assert!(!doc.selected_glyph(a, b, id, doc.version(id).unwrap(), 0..1));
    }
}
fn parse(raw: &str, role: Role) -> Parsed {
    let mut out = Parsed::default();
    if !matches!(role, Role::Assistant | Role::Worker) {
        out.text(
            raw,
            Style::default().fg(if role == Role::User {
                Color::Rgb(205, 211, 220)
            } else {
                Color::Rgb(137, 145, 158)
            }),
        );
        return out;
    }
    let base = Style::default().fg(Color::Rgb(205, 211, 220));
    let accent = Style::default().fg(Color::Rgb(121, 185, 189));
    let mut styles = vec![base];
    let mut lists: Vec<Option<u64>> = vec![];
    let mut code: Option<(String, String)> = None;
    for event in Parser::new_ext(
        raw,
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS,
    ) {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                out.breaks(2);
                let lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(l) => l.into_string(),
                    _ => String::new(),
                };
                code = Some((lang, String::new()));
            }
            Event::Text(text) if code.is_some() => code.as_mut().unwrap().1.push_str(&text),
            Event::End(TagEnd::CodeBlock) => {
                if let Some((lang, text)) = code.take() {
                    out.decorations.push(Decoration {
                        offset: out.plain.len(),
                        label: if lang.is_empty() {
                            "code".into()
                        } else {
                            lang.clone()
                        },
                        code: out.codes.len(),
                    });
                    let start = out.plain.len();
                    highlight(&mut out, &text, &lang);
                    out.code_ranges.push(start..out.plain.len());
                    out.codes.push(text);
                    out.breaks(2);
                }
            }
            Event::Text(text) => out.text(&text, *styles.last().unwrap()),
            Event::Code(text) => out.text(&text, accent),
            Event::Start(Tag::Heading { .. }) => {
                out.breaks(2);
                styles.push(accent.add_modifier(Modifier::BOLD));
            }
            Event::End(TagEnd::Heading(_)) => {
                styles.pop();
                out.breaks(2);
            }
            Event::Start(Tag::Strong) => {
                styles.push(styles.last().copied().unwrap().add_modifier(Modifier::BOLD))
            }
            Event::Start(Tag::Emphasis) => styles.push(
                styles
                    .last()
                    .copied()
                    .unwrap()
                    .add_modifier(Modifier::ITALIC),
            ),
            Event::Start(Tag::Strikethrough) => styles.push(
                styles
                    .last()
                    .copied()
                    .unwrap()
                    .add_modifier(Modifier::CROSSED_OUT),
            ),
            Event::End(TagEnd::Strong | TagEnd::Emphasis | TagEnd::Strikethrough) => {
                if styles.len() > 1 {
                    styles.pop();
                }
            }
            Event::Start(Tag::List(start)) => {
                out.breaks(1);
                lists.push(start);
            }
            Event::End(TagEnd::List(_)) => {
                lists.pop();
                out.breaks(2);
            }
            Event::Start(Tag::Item) => {
                out.breaks(1);
                let prefix = match lists.last_mut() {
                    Some(Some(n)) => {
                        let p = format!("{n}. ");
                        *n += 1;
                        p
                    }
                    _ => "• ".into(),
                };
                out.text(
                    &format!(
                        "{}{prefix}",
                        "  ".repeat(lists.len().saturating_sub(1).min(8))
                    ),
                    base,
                );
            }
            Event::End(TagEnd::Item) => out.breaks(1),
            Event::End(TagEnd::Paragraph) => out.breaks(if lists.is_empty() { 2 } else { 1 }),
            Event::SoftBreak => out.text(" ", *styles.last().unwrap()),
            Event::HardBreak => out.text("\n", base),
            Event::Start(Tag::BlockQuote(_)) => {
                out.breaks(1);
                styles.push(base.add_modifier(Modifier::ITALIC));
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                if styles.len() > 1 {
                    styles.pop();
                }
                out.breaks(2);
            }
            Event::Start(Tag::Link { .. }) => {
                styles.push(accent.add_modifier(Modifier::UNDERLINED));
            }
            Event::End(TagEnd::Link) => {
                if styles.len() > 1 {
                    styles.pop();
                }
            }
            Event::TaskListMarker(checked) => {
                out.text(if checked { "[x] " } else { "[ ] " }, accent)
            }
            Event::Rule => out.breaks(2),
            Event::Html(text) | Event::InlineHtml(text) => out.text(&text, base),
            _ => {}
        }
    }
    // An incomplete stream/fence is plain preview; finalized malformed Markdown
    // still retains its literal code body rather than silently dropping text.
    if let Some((lang, text)) = code {
        highlight(&mut out, &text, &lang);
    }
    out
}
fn highlight(out: &mut Parsed, text: &str, lang: &str) {
    use syntect::{easy::HighlightLines, highlighting::ThemeSet, parsing::SyntaxSet};
    static SYNTAX: OnceLock<SyntaxSet> = OnceLock::new();
    static THEMES: OnceLock<ThemeSet> = OnceLock::new();
    let syntax = SYNTAX.get_or_init(SyntaxSet::load_defaults_newlines);
    let themes = THEMES.get_or_init(ThemeSet::load_defaults);
    let theme = &themes.themes["base16-ocean.dark"];
    let definition = syntax
        .find_syntax_by_token(lang.split_whitespace().next().unwrap_or(""))
        .unwrap_or_else(|| syntax.find_syntax_plain_text());
    let mut highlighter = HighlightLines::new(definition, theme);
    for line in text.split_inclusive('\n') {
        if line.len() > 4096 {
            out.text(line, Style::default().fg(Color::Rgb(121, 185, 189)));
            continue;
        }
        match highlighter.highlight_line(line, syntax) {
            Ok(parts) => {
                for (s, t) in parts {
                    out.text(
                        t,
                        Style::default().fg(Color::Rgb(
                            s.foreground.r,
                            s.foreground.g,
                            s.foreground.b,
                        )),
                    );
                }
            }
            Err(_) => out.text(line, Style::default().fg(Color::Rgb(121, 185, 189))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn text_regression_reference_links_cannot_amplify_display() {
        let raw = format!(
            "{}\n\n[r]: https://example.test/{}",
            "[x][r] ".repeat(1000),
            "a".repeat(8000)
        );
        let mut doc = Document::default();
        let id = Uuid::new_v4();
        doc.upsert(id, Role::Assistant, &raw, true);
        assert!(doc.copy(id, CopyKind::Plain).unwrap().len() <= 65536);
    }
    #[test]
    fn text_regression_zero_width_display_has_the_measured_cell_width() {
        let mut doc = Document::default();
        doc.upsert(Uuid::new_v4(), Role::Assistant, "a\u{200b}b", true);
        for row in &doc.layout(20).rows {
            for g in &row.glyphs {
                assert_eq!(
                    UnicodeWidthStr::width(g.text.as_str()),
                    usize::from(g.width)
                );
            }
        }
    }
    #[test]
    fn markdown_copy_and_code_preserve_content_without_ui_decoration() {
        let mut doc = Document::default();
        let id = uuid::Uuid::new_v4();
        let raw = "# 检查\n\n这是 **重点**，还有 `inline`。\n\n```python\n\tprint('│ and | stay')\n    x = 1\n```";
        doc.upsert(id, Role::Assistant, raw, true);
        assert_eq!(doc.copy(id, CopyKind::Markdown).unwrap(), raw);
        assert_eq!(
            doc.copy(id, CopyKind::Code(0)).unwrap(),
            "\tprint('│ and | stay')\n    x = 1\n"
        );
        let plain = doc.copy(id, CopyKind::Plain).unwrap();
        assert!(plain.contains("重点") && !plain.contains("**") && !plain.contains("```"));
        assert!(!plain.contains("Lain>") && !plain.contains("复制代码"));
    }
    #[test]
    fn wrapped_selection_joins_soft_lines_but_preserves_true_newlines_and_graphemes() {
        let mut doc = Document::default();
        let id = uuid::Uuid::new_v4();
        let raw = "中文段落会自动折行👩‍💻e\u{301}https://example.test/a-long-path";
        doc.upsert(id, Role::Assistant, raw, true);
        let layout = doc.layout(12);
        let anchors = layout
            .rows
            .iter()
            .flat_map(|r| r.glyphs.iter().map(move |g| (r, g)))
            .collect::<Vec<_>>();
        let (row, first) = anchors.first().unwrap();
        let (last_row, last) = anchors.last().unwrap();
        let start = Anchor {
            id: row.id,
            version: row.version,
            offset: first.start,
        };
        let end = Anchor {
            id: last_row.id,
            version: last_row.version,
            offset: last.end,
        };
        assert!(layout.rows.len() > 4);
        assert_eq!(doc.selected(start, end).unwrap(), raw);
        assert!(anchors.iter().any(|(_, g)| g.text == "👩‍💻"));
        assert!(anchors.iter().any(|(_, g)| g.text == "e\u{301}"));
    }
    #[test]
    fn old_selection_survives_other_messages_but_not_selected_revision_or_source_retirement() {
        let mut doc = Document::default();
        let id = uuid::Uuid::new_v4();
        doc.upsert(id, Role::Assistant, "saved", true);
        let version = doc.entries[0].version;
        let start = Anchor {
            id,
            version,
            offset: 0,
        };
        let end = Anchor {
            id,
            version,
            offset: 5,
        };
        doc.upsert(
            uuid::Uuid::new_v4(),
            Role::Assistant,
            "background reply",
            true,
        );
        assert_eq!(doc.selected(start, end).as_deref(), Some("saved"));
        doc.upsert(id, Role::Assistant, "changed", true);
        assert!(doc.selected(start, end).is_none());
        doc.clear();
        assert!(doc.copy(id, CopyKind::Markdown).is_none());
    }
    #[test]
    fn preview_is_visible_but_cannot_be_copied_as_a_committed_message() {
        let mut doc = Document::default();
        let id = uuid::Uuid::new_v4();
        doc.upsert(id, Role::Preview, "unfinished **text", false);
        assert!(doc.copy(id, CopyKind::Markdown).is_none());
        assert!(!doc.layout(40).rows.is_empty());
    }
}
