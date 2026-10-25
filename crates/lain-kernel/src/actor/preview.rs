//! Ephemeral sanitized output. Never a tool instruction or persistence input.
use super::Observation;
use crate::redaction::{Redactor, StreamingRedactor};
use lain_wire::{
    ActorTurnId, DiscardReason, MessageId, ModelTiming, PreviewEvent, PreviewId, PreviewUpdate,
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, Weak},
    time::Instant,
};
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct PreviewSink(Arc<Mutex<State>>);
#[derive(Default)]
pub(super) struct Registry(Mutex<BTreeMap<ActorTurnId, Weak<Mutex<State>>>>);
impl Registry {
    pub fn checkpoint(&self, id: ActorTurnId) -> Option<PreviewEvent> {
        let state = self.0.lock().unwrap().get(&id)?.upgrade()?;
        let s = state.lock().unwrap();
        if s.closed {
            return None;
        }
        Some(PreviewEvent {
            turn_id: id,
            preview_id: s.preview_id,
            message_id: s.message_id,
            sequence: s.sequence.saturating_sub(1),
            update: PreviewUpdate::Started {
                text: s.visible.clone(),
            },
        })
    }
    pub fn forget(&self, id: ActorTurnId) {
        self.0.lock().unwrap().remove(&id);
    }
}
struct State {
    sender: broadcast::Sender<Observation>,
    turn_id: ActorTurnId,
    preview_id: PreviewId,
    message_id: MessageId,
    sequence: u64,
    sanitizer: StreamingRedactor,
    text: zeroize::Zeroizing<String>,
    visible: String,
    closed: bool,
    held: bool,
    started: Instant,
    timing: ModelTiming,
}
impl State {
    fn send(&mut self, update: PreviewUpdate) {
        let event = PreviewEvent {
            turn_id: self.turn_id,
            preview_id: self.preview_id,
            message_id: self.message_id,
            sequence: self.sequence,
            update,
        };
        self.sequence += 1;
        let _ = self.sender.send(Observation::Preview(event));
    }
    fn discard(&mut self, reason: DiscardReason) {
        if !self.closed {
            self.send(PreviewUpdate::Discarded { reason });
            self.closed = true;
        }
    }
    fn visible(&mut self, text: String) {
        if self.held {
            return;
        }
        if !text.is_empty() {
            self.visible.push_str(&text);
            self.timing
                .first_visible_ms
                .get_or_insert(elapsed(self.started));
            self.send(PreviewUpdate::Delta { text });
        }
    }
}
pub(super) fn elapsed(start: Instant) -> u64 {
    start.elapsed().as_millis().min(u64::MAX as u128) as u64
}
impl PreviewSink {
    pub(super) fn new(
        sender: broadcast::Sender<Observation>,
        redactor: Redactor,
        turn_id: ActorTurnId,
        message_id: MessageId,
        registry: &Registry,
    ) -> Self {
        let sink = Self(Arc::new(Mutex::new(State {
            sender,
            turn_id,
            message_id,
            preview_id: PreviewId::new(),
            sequence: 0,
            sanitizer: StreamingRedactor::new(redactor),
            text: zeroize::Zeroizing::new(String::new()),
            closed: false,
            held: false,
            visible: String::new(),
            started: Instant::now(),
            timing: ModelTiming::default(),
        })));
        registry
            .0
            .lock()
            .unwrap()
            .insert(turn_id, Arc::downgrade(&sink.0));
        sink.0.lock().unwrap().send(PreviewUpdate::Started {
            text: String::new(),
        });
        sink
    }
    pub(super) fn hold(&self) {
        self.0.lock().unwrap().held = true;
    }
    pub(super) fn interrupt(&self) {
        self.0.lock().unwrap().discard(DiscardReason::Interrupted);
    }
    pub(super) fn timing(&self) -> ModelTiming {
        self.0.lock().unwrap().timing.clone()
    }
    pub(crate) fn first_byte(&self) {
        let mut s = self.0.lock().unwrap();
        let elapsed = elapsed(s.started);
        s.timing.first_byte_ms.get_or_insert(elapsed);
    }
    pub(crate) fn provider_completed(&self) {
        let mut s = self.0.lock().unwrap();
        let elapsed = elapsed(s.started);
        s.timing.completed_ms.get_or_insert(elapsed);
    }
    pub(crate) fn delta(&self, delta: &str) {
        let mut s = self.0.lock().unwrap();
        if s.closed {
            return;
        }
        if !delta.is_empty() {
            let elapsed = elapsed(s.started);
            s.timing.first_text_ms.get_or_insert(elapsed);
        }
        if s.text.len().saturating_add(delta.len()) > 16384 {
            s.discard(DiscardReason::Limit);
            return;
        }
        s.text.push_str(delta);
        match s.sanitizer.push(delta) {
            Ok(safe) => s.visible(safe),
            Err(_) => s.discard(DiscardReason::Sensitive),
        }
    }
    pub(super) fn finish(&self, authoritative: &str) -> ModelTiming {
        let mut s = self.0.lock().unwrap();
        if !s.closed {
            if s.text.as_str() != authoritative {
                s.discard(DiscardReason::Inconsistent);
            } else {
                let tail = s.sanitizer.finish();
                s.visible(tail);
                s.send(PreviewUpdate::Finished);
                s.closed = true;
            }
        }
        s.timing.clone()
    }
}
impl Drop for State {
    fn drop(&mut self) {
        self.discard(DiscardReason::Interrupted);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_influenced_preview_is_held_and_can_be_discarded_without_text_disclosure() {
        let (sender, mut receiver) = broadcast::channel(16);
        let registry = Registry::default();
        let turn = ActorTurnId::new();
        let sink = PreviewSink::new(
            sender,
            Redactor::new(vec![]),
            turn,
            MessageId::new(),
            &registry,
        );
        sink.hold();
        sink.delta("historical private quote");
        assert!(
            matches!(registry.checkpoint(turn).unwrap().update,PreviewUpdate::Started {text} if text.is_empty())
        );
        sink.interrupt();
        sink.finish("");
        let mut discarded = 0;
        while let Ok(Observation::Preview(event)) = receiver.try_recv() {
            match event.update {
                PreviewUpdate::Delta { .. } | PreviewUpdate::Finished => {
                    panic!("held source text escaped before authorization check")
                }
                PreviewUpdate::Discarded { .. } => discarded += 1,
                _ => {}
            }
        }
        assert_eq!(discarded, 1);
    }
    #[test]
    fn slow_subscribers_are_bounded_and_recover_a_sanitized_checkpoint() {
        let (sender, mut receiver) = broadcast::channel(16);
        let registry = Registry::default();
        let turn = ActorTurnId::new();
        let message = MessageId::new();
        let sink = PreviewSink::new(
            sender,
            Redactor::new(vec!["credential-123".into()]),
            turn,
            message,
            &registry,
        );
        for _ in 0..100 {
            sink.delta("中");
        }
        assert!(matches!(
            receiver.try_recv(),
            Err(broadcast::error::TryRecvError::Lagged(_))
        ));
        let checkpoint = registry.checkpoint(turn).unwrap();
        assert_eq!(checkpoint.message_id, message);
        assert!(matches!(checkpoint.update,PreviewUpdate::Started{text} if text=="中".repeat(100)));
        sink.finish(&"中".repeat(100));
        assert!(registry.checkpoint(turn).is_none());
        sink.delta("late");
        let mut late = false;
        while let Ok(Observation::Preview(event)) = receiver.try_recv() {
            late |= matches!(event.update,PreviewUpdate::Delta{text} if text=="late");
        }
        assert!(!late);
    }
    #[test]
    fn last_clone_drop_discards_once_without_claiming_completion() {
        let (sender, mut receiver) = broadcast::channel(16);
        let registry = Registry::default();
        let turn = ActorTurnId::new();
        let sink = PreviewSink::new(
            sender,
            Redactor::new(vec![]),
            turn,
            MessageId::new(),
            &registry,
        );
        let other = sink.clone();
        sink.delta("partial");
        drop(sink);
        assert!(registry.checkpoint(turn).is_some());
        drop(other);
        assert!(registry.checkpoint(turn).is_none());
        let mut discarded = 0;
        while let Ok(Observation::Preview(event)) = receiver.try_recv() {
            assert!(!matches!(event.update, PreviewUpdate::Finished));
            discarded += usize::from(matches!(
                event.update,
                PreviewUpdate::Discarded {
                    reason: DiscardReason::Interrupted
                }
            ));
        }
        assert_eq!(discarded, 1);
    }
}
