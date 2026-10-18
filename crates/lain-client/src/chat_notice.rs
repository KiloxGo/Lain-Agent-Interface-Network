//! Observation and acknowledgement tasks never write to the terminal. The chat
//! input owner renders every update, then queues a best-effort display receipt.
use crate::{
    actor_render::Renderer,
    api::{Client, NoticeUpdate},
};
use anyhow::Result;
use lain_wire::{ConversationId, NoticeId};
use std::{io::Write, time::Duration};
use tokio::sync::mpsc;

pub(crate) struct NoticeInbox {
    receiver: mpsc::Receiver<NoticeUpdate>,
    acknowledgements: mpsc::Sender<NoticeId>,
    observer: tokio::task::JoinHandle<()>,
    acknowledger: tokio::task::JoinHandle<()>,
    displayed_cursor: u64,
    offline_shown: bool,
}
impl NoticeInbox {
    pub fn new(client: Client, conversation: ConversationId) -> Self {
        let (sender, receiver) = mpsc::channel(8);
        let (acknowledgements, mut ack_receiver) = mpsc::channel::<NoticeId>(8);
        let ack_client = client.clone();
        let acknowledger = tokio::spawn(async move {
            while let Some(id) = ack_receiver.recv().await {
                // Render-before-ack can replay after a crash. Failed/queued acks
                // remain recoverable; they never trigger a worker mutation.
                let _ = tokio::time::timeout(
                    Duration::from_secs(5),
                    ack_client.post::<serde_json::Value>(
                        &format!("v1/conversations/{conversation}/notices/{id}/ack"),
                        None,
                    ),
                )
                .await;
            }
        });
        let observer = tokio::spawn(async move {
            let mut cursor = 0;
            loop {
                tokio::select! {
                    _=sender.closed()=>break,
                    _=client.notice_observe(conversation, &mut cursor, &sender)=>{},
                }
                if sender.send(NoticeUpdate::Offline).await.is_err() {
                    break;
                }
                tokio::select! { _=sender.closed()=>break, _=tokio::time::sleep(Duration::from_secs(1))=>{} }
            }
        });
        Self {
            receiver,
            acknowledgements,
            observer,
            acknowledger,
            displayed_cursor: 0,
            offline_shown: false,
        }
    }
    pub async fn next(&mut self) -> NoticeUpdate {
        match self.receiver.recv().await {
            Some(update) => update,
            None => std::future::pending().await,
        }
    }
    pub fn visible(&self, update: &NoticeUpdate) -> bool {
        match update {
            NoticeUpdate::Notice(notice) => {
                notice.cursor > self.displayed_cursor && !notice.acknowledged
            }
            NoticeUpdate::Offline => !self.offline_shown,
        }
    }
    pub fn render<W: Write>(
        &mut self,
        update: NoticeUpdate,
        renderer: &mut Renderer<W>,
    ) -> Result<()> {
        match update {
            NoticeUpdate::Notice(notice) => {
                if notice.cursor <= self.displayed_cursor {
                    return Ok(());
                }
                if !notice.acknowledged {
                    renderer.notice(&notice)?;
                    let _ = self.acknowledgements.try_send(notice.id);
                }
                self.displayed_cursor = notice.cursor;
                self.offline_shown = false;
            }
            NoticeUpdate::Offline if !self.offline_shown => {
                renderer.notice_offline()?;
                self.offline_shown = true;
            }
            _ => {}
        }
        Ok(())
    }
    pub async fn flush(&mut self) {
        // Normal exit gives already queued display receipts a short drain;
        // network loss/crashes retain at-least-once replay semantics.
        let (closed, _) = mpsc::channel(1);
        drop(std::mem::replace(&mut self.acknowledgements, closed));
        let _ = tokio::time::timeout(Duration::from_secs(2), &mut self.acknowledger).await;
    }
}
impl Drop for NoticeInbox {
    fn drop(&mut self) {
        self.observer.abort();
        self.acknowledger.abort();
    }
}

pub(crate) async fn next(inbox: &mut Option<&mut NoticeInbox>) -> NoticeUpdate {
    match inbox {
        Some(inbox) => inbox.next().await,
        None => std::future::pending().await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn replay_deduplicates_visible_notices_before_acknowledgement() {
        let (_tx, receiver) = mpsc::channel(8);
        let (acknowledgements, mut acks) = mpsc::channel(8);
        let mut inbox = NoticeInbox {
            receiver,
            acknowledgements,
            observer: tokio::spawn(std::future::pending()),
            acknowledger: tokio::spawn(std::future::pending()),
            displayed_cursor: 0,
            offline_shown: false,
        };
        let notice: lain_wire::notices::ConversationNotice = serde_json::from_value(serde_json::json!({
            "id":NoticeId::new(),"cursor":5,"conversation_id":ConversationId::new(),"actor_turn_id":lain_wire::ActorTurnId::new(),"action_id":lain_wire::ActionId::new(),"work_session_id":lain_wire::WorkSessionId::new(),"worker_turn_id":lain_wire::WorkerTurnId::new(),"timestamp_ms":1,"source_history_order":10,
            "outcome":{"kind":"worker","state":"completed"},"evidence":{"final_assistant_message":"真实报告\u{1b}[31m", "failed_commands":[],"failed_command_count":0,"changed_files":[],"errors":[],"truncated":false},"acknowledged":false
        })).unwrap();
        let mut bytes = vec![];
        let mut renderer = Renderer::new(&mut bytes);
        for _ in 0..3 {
            inbox
                .render(
                    NoticeUpdate::Notice(Box::new(notice.clone())),
                    &mut renderer,
                )
                .unwrap();
        }
        drop(renderer);
        let text = String::from_utf8(bytes).unwrap();
        assert_eq!(text.matches("Worker> turn completed").count(), 1);
        assert!(text.contains("真实报告"));
        assert!(!text.contains('\u{1b}'));
        assert_eq!(acks.try_recv().unwrap(), notice.id);
        assert!(acks.try_recv().is_err());
    }
}
