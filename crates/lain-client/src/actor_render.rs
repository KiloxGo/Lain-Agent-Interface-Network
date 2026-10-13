//! One output coordinator for transient previews and authoritative Actor records.
use crate::tui::safe_text;
use anyhow::Result;
use lain_wire::{
    ActionId, ActionState, ActorSnapshot, ActorTurnState, MessageId, MessageRole, PreviewEvent,
    PreviewId, PreviewUpdate,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    time::Instant,
};

struct Preview {
    id: PreviewId,
    message: MessageId,
    sequence: u64,
    text: String,
    open: bool,
    finished: bool,
}
pub struct Renderer<W: Write> {
    output: W,
    messages: BTreeSet<MessageId>,
    actions: BTreeMap<ActionId, ActionState>,
    retired: BTreeSet<PreviewId>,
    current: Option<Preview>,
    error: Option<String>,
    pub first_visible: Option<Instant>,
}
impl<W: Write> Renderer<W> {
    pub fn new(output: W) -> Self {
        Self {
            output,
            messages: BTreeSet::new(),
            actions: BTreeMap::new(),
            retired: BTreeSet::new(),
            current: None,
            error: None,
            first_visible: None,
        }
    }
    fn text(&mut self, text: &str, start: bool) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        if start {
            write!(self.output, "Lain> ")?;
        }
        write!(self.output, "{}", safe_text(text).replace('\n', "\n      "))?;
        self.output.flush()?;
        self.first_visible.get_or_insert_with(Instant::now);
        Ok(())
    }
    fn close(&mut self, preview: &mut Preview) -> Result<()> {
        if preview.open {
            writeln!(self.output)?;
            preview.open = false;
        }
        Ok(())
    }
    pub fn interrupted(&mut self, reason: &str) -> Result<()> {
        if let Some(mut preview) = self.current.take() {
            self.close(&mut preview)?;
            if !preview.text.is_empty() {
                writeln!(
                    self.output,
                    "Preview> {}; completed message not confirmed.",
                    safe_text(reason)
                )?;
            }
            self.retired.insert(preview.id);
        }
        self.output.flush()?;
        Ok(())
    }
    pub fn snapshot(&mut self, turn: &ActorSnapshot) -> Result<()> {
        for message in &turn.messages {
            if message.role != MessageRole::Assistant || !self.messages.insert(message.id) {
                continue;
            }
            if self
                .current
                .as_ref()
                .is_some_and(|p| p.message == message.id)
            {
                let mut preview = self.current.take().unwrap();
                if let Some(tail) = message.content.strip_prefix(&preview.text) {
                    // A checkpoint/final snapshot can arrive before all live deltas.
                    self.text(tail, !preview.open)?;
                    if preview.open || !tail.is_empty() {
                        writeln!(self.output)?;
                    }
                } else {
                    self.close(&mut preview)?;
                    writeln!(self.output, "Preview> superseded by the completed message.")?;
                    self.text(&message.content, true)?;
                    writeln!(self.output)?;
                }
                self.retired.insert(preview.id);
            } else {
                self.interrupted("superseded")?;
                self.text(&message.content, true)?;
                writeln!(self.output)?;
            }
        }
        for action in &turn.actions {
            if self.actions.insert(action.id, action.state) != Some(action.state) {
                self.interrupted("tool state changed")?;
                writeln!(
                    self.output,
                    "Tool> {}",
                    safe_text(&serde_json::to_string_pretty(action)?)
                )?;
            }
        }
        if let Some(error) = &turn.error
            && self.error.as_ref() != Some(error)
        {
            self.interrupted("generation ended")?;
            writeln!(self.output, "Lain> {}", safe_text(error))?;
            self.error = Some(error.clone());
        }
        if turn.state != ActorTurnState::Running {
            self.interrupted("generation ended")?;
        }
        self.output.flush()?;
        Ok(())
    }
    pub fn preview(&mut self, event: PreviewEvent, turn: &ActorSnapshot) -> Result<()> {
        if turn.id != event.turn_id
            || turn.state != ActorTurnState::Running
            || self.messages.contains(&event.message_id)
            || self.retired.contains(&event.preview_id)
        {
            return Ok(());
        }
        if let PreviewUpdate::Started { text } = event.update {
            if text.len() > 16384 {
                return Ok(());
            }
            if self
                .current
                .as_ref()
                .is_some_and(|p| p.id == event.preview_id)
            {
                let mut p = self.current.take().unwrap();
                if p.message == event.message_id && event.sequence >= p.sequence && !p.finished {
                    if let Some(tail) = text.strip_prefix(&p.text) {
                        self.text(tail, !p.open)?;
                        p.open |= !tail.is_empty();
                        p.text = text;
                        p.sequence = event.sequence;
                    } else {
                        self.current = Some(p);
                        self.interrupted("inconsistent checkpoint")?;
                        return Ok(());
                    }
                }
                self.current = Some(p);
            } else {
                self.interrupted("new generation")?;
                self.text(&text, true)?;
                self.current = Some(Preview {
                    id: event.preview_id,
                    message: event.message_id,
                    sequence: event.sequence,
                    open: !text.is_empty(),
                    text,
                    finished: false,
                });
            }
            return Ok(());
        }
        let Some(mut p) = self.current.take() else {
            return Ok(());
        };
        if p.id != event.preview_id
            || p.message != event.message_id
            || event.sequence <= p.sequence
            || p.finished
        {
            self.current = Some(p);
            return Ok(());
        }
        if event.sequence != p.sequence + 1 {
            self.current = Some(p);
            self.interrupted("sequence gap; awaiting authoritative snapshot")?;
            return Ok(());
        }
        p.sequence = event.sequence;
        match event.update {
            PreviewUpdate::Delta { text } if p.text.len().saturating_add(text.len()) <= 16384 => {
                self.text(&text, !p.open)?;
                p.open |= !text.is_empty();
                p.text.push_str(&text);
            }
            PreviewUpdate::Finished => {
                self.close(&mut p)?;
                p.finished = true;
            }
            _ => {
                self.current = Some(p);
                self.interrupted("discarded")?;
                return Ok(());
            }
        }
        self.current = Some(p);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lain_wire::*;
    fn running() -> ActorSnapshot {
        ActorSnapshot {
            id: ActorTurnId::new(),
            conversation_id: ConversationId::new(),
            request_id: RequestId::new(),
            sequence: 1,
            revision: 1,
            state: ActorTurnState::Running,
            messages: vec![],
            actions: vec![],
            error: None,
            model_requests: 1,
            usage_tokens: 0,
            timing: ActorTiming::default(),
        }
    }
    fn event(
        turn: &ActorSnapshot,
        preview: PreviewId,
        message: MessageId,
        sequence: u64,
        update: PreviewUpdate,
    ) -> PreviewEvent {
        PreviewEvent {
            turn_id: turn.id,
            preview_id: preview,
            message_id: message,
            sequence,
            update,
        }
    }
    #[test]
    fn unicode_controls_checkpoint_overlap_and_final_are_rendered_once() {
        let mut turn = running();
        let preview = PreviewId::new();
        let message = MessageId::new();
        let mut r = Renderer::new(Vec::new());
        r.preview(
            event(
                &turn,
                preview,
                message,
                0,
                PreviewUpdate::Started {
                    text: String::new(),
                },
            ),
            &turn,
        )
        .unwrap();
        r.preview(
            event(
                &turn,
                preview,
                message,
                1,
                PreviewUpdate::Delta {
                    text: "你好\n".into(),
                },
            ),
            &turn,
        )
        .unwrap();
        r.preview(
            event(
                &turn,
                preview,
                message,
                1,
                PreviewUpdate::Delta {
                    text: "duplicate".into(),
                },
            ),
            &turn,
        )
        .unwrap();
        r.preview(
            event(
                &turn,
                preview,
                message,
                2,
                PreviewUpdate::Started {
                    text: "你好\n世界\u{1b}".into(),
                },
            ),
            &turn,
        )
        .unwrap();
        r.preview(
            event(&turn, preview, message, 3, PreviewUpdate::Finished),
            &turn,
        )
        .unwrap();
        turn.state = ActorTurnState::Completed;
        turn.messages.push(ActorMessage {
            id: message,
            conversation_id: turn.conversation_id,
            role: MessageRole::Assistant,
            content: "你好\n世界\u{1b}".into(),
            timestamp_ms: 0,
            status: ActorTurnState::Completed,
        });
        r.snapshot(&turn).unwrap();
        r.snapshot(&turn).unwrap();
        r.preview(
            event(
                &turn,
                PreviewId::new(),
                MessageId::new(),
                0,
                PreviewUpdate::Started {
                    text: "late".into(),
                },
            ),
            &turn,
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(r.output).unwrap(),
            "Lain> 你好\n      世界\\u{1b}\n"
        );
    }
    #[test]
    fn wrong_generation_gap_and_failure_never_become_completed_prose() {
        let mut turn = running();
        let preview = PreviewId::new();
        let message = MessageId::new();
        let mut r = Renderer::new(Vec::new());
        let other = running();
        r.preview(
            event(
                &other,
                preview,
                message,
                0,
                PreviewUpdate::Started {
                    text: "wrong".into(),
                },
            ),
            &turn,
        )
        .unwrap();
        r.preview(
            event(
                &turn,
                preview,
                message,
                0,
                PreviewUpdate::Started {
                    text: "partial".into(),
                },
            ),
            &turn,
        )
        .unwrap();
        r.preview(
            event(
                &turn,
                preview,
                message,
                2,
                PreviewUpdate::Delta { text: "gap".into() },
            ),
            &turn,
        )
        .unwrap();
        r.preview(
            event(
                &turn,
                preview,
                message,
                1,
                PreviewUpdate::Delta {
                    text: "stale".into(),
                },
            ),
            &turn,
        )
        .unwrap();
        turn.state = ActorTurnState::Unknown;
        r.snapshot(&turn).unwrap();
        let text = String::from_utf8(r.output).unwrap();
        assert!(text.contains("completed message not confirmed"));
        assert!(
            !text.contains("wrong")
                && !text.contains("Lain> partialgap")
                && !text.contains("stale")
        );
        assert!(r.messages.is_empty());
    }
}
