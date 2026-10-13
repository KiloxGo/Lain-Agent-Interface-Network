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
    pub(crate) fn first_byte(&self) {
        let mut s = self.0.lock().unwrap();
        let elapsed = elapsed(s.started);
        s.timing.first_byte_ms.get_or_insert(elapsed);
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
        s.timing.completed_ms = Some(elapsed(s.started));
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
impl Drop for PreviewSink {
    fn drop(&mut self) {
        if Arc::strong_count(&self.0) == 1 {
            self.0.lock().unwrap().discard(DiscardReason::Interrupted);
        }
    }
}
