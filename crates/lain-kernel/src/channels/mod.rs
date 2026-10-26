//! Durable, bounded conversation ingress/egress; observation bridges are separate.
pub mod domain;
pub mod napcat;
#[cfg(test)]
mod tests;
use crate::{
    actor::{Actor, domain::TurnState},
    config::ApiToken,
    kernel::{Kernel, blocking},
    redaction::Redactor,
};
use anyhow::Result;
use domain::*;
use lain_wire::sharing::ShareDelivery;
use std::{future::Future, pin::Pin, sync::Arc, time::Duration};
use tokio::sync::watch;

pub(super) enum Outgoing {
    Reply(Delivery),
    Share(ShareDelivery),
}
impl Outgoing {
    fn id(&self) -> lain_wire::DeliveryId {
        match self {
            Self::Reply(d) => d.id,
            Self::Share(d) => d.id,
        }
    }
    fn text(&self) -> &str {
        match self {
            Self::Reply(d) => &d.text,
            Self::Share(d) => &d.text,
        }
    }
}

/// Implementations own all platform protocol/credentials. Never automatically retry send.
pub trait ConversationChannel: Send + Sync {
    fn connected(&self) -> watch::Receiver<bool>;
    fn send(&self, delivery: Delivery) -> Pin<Box<dyn Future<Output = SendOutcome> + Send + '_>>;
    fn send_share(
        &self,
        _delivery: ShareDelivery,
    ) -> Pin<Box<dyn Future<Output = SendOutcome> + Send + '_>> {
        Box::pin(async { SendOutcome::NotSent })
    }
}

pub struct ChannelService {
    kernel: Arc<Kernel>,
    actor: Arc<Actor>,
    binding: Binding,
    redactor: Redactor,
}
impl ChannelService {
    pub fn new(
        kernel: Arc<Kernel>,
        actor: Arc<Actor>,
        binding: Binding,
        token: &ApiToken,
    ) -> Arc<Self> {
        Arc::new(Self {
            kernel,
            actor,
            binding,
            redactor: Redactor::new(token.redaction_values()).for_runtime(None),
        })
    }
    pub fn id(&self) -> ChannelId {
        self.binding.id
    }
    pub(super) fn connection(&self, phase: ConnectionPhase, failure: Option<ConnectionFailure>) {
        if self
            .kernel
            .channel_connection(self.id(), phase, failure)
            .is_err()
        {
            tracing::warn!(channel = %self.id(), "channel connection observation unavailable");
        }
    }
    pub async fn receive(&self, input: Input) -> Result<Incoming> {
        self.actor.check_memory_input(&input)?;
        let encoded = serde_json::to_string(&input)?;
        anyhow::ensure!(
            self.redactor.text(&encoded) == encoded,
            "sensitive channel input rejected"
        );
        let id = self.id();
        blocking(&self.kernel, move |k| k.channel_receive(id, input)).await
    }
    /// A transport calls this immediately before writing, after queueing/identity checks.
    pub async fn may_send(&self, delivery: &Delivery) -> bool {
        let id = self.id();
        let delivery = delivery.clone();
        delivery.channel_id == id
            && self.actor.check_memory_input(&delivery).is_ok()
            && blocking(&self.kernel, move |k| k.channel_may_send(&delivery))
                .await
                .unwrap_or(false)
    }
    pub async fn may_send_share(&self, delivery: &ShareDelivery) -> bool {
        let delivery = delivery.clone();
        delivery.channel_id == self.id()
            && self.actor.check_memory_input(&delivery).is_ok()
            && blocking(&self.kernel, move |k| k.share_may_send(&delivery))
                .await
                .unwrap_or(false)
    }
    pub(super) async fn may_send_outgoing(&self, delivery: &Outgoing) -> bool {
        match delivery {
            Outgoing::Reply(d) => self.may_send(d).await,
            Outgoing::Share(d) => self.may_send_share(d).await,
        }
    }
    pub async fn run(self: Arc<Self>, channel: Arc<dyn ConversationChannel>) -> Result<()> {
        struct Guard(Arc<ChannelService>);
        impl Drop for Guard {
            fn drop(&mut self) {
                self.0.connection(ConnectionPhase::Stopped, None);
            }
        }
        let _guard = Guard(self.clone());
        let shutdown = self.kernel.subscribe_shutdown();
        let result = self.clone().run_loop(channel).await;
        // Kernel admission can close between a wake-up and a queued storage call.
        // Shutdown leaves committed work for the normal restart recovery policy.
        if *shutdown.borrow() { Ok(()) } else { result }
    }
    async fn run_loop(self: Arc<Self>, channel: Arc<dyn ConversationChannel>) -> Result<()> {
        let mut changes = self.kernel.subscribe_channels();
        let mut observations = self.actor.subscribe();
        let mut shutdown = self.kernel.subscribe_shutdown();
        let mut connection = channel.connected();
        let mut shares = tokio::task::JoinSet::<Result<()>>::new();
        loop {
            if *shutdown.borrow() {
                return Ok(());
            }
            // Connection is checked before claim; offline drafts stay Ready until expiry.
            if *connection.borrow() {
                let id = self.id();
                if let Some(delivery) =
                    blocking(&self.kernel, move |k| k.channel_claim_delivery(id)).await?
                {
                    let delivery_id = delivery.id;
                    let outcome = if self.may_send(&delivery).await {
                        tokio::time::timeout(Duration::from_secs(20), channel.send(delivery))
                            .await
                            .unwrap_or(SendOutcome::Unknown)
                    } else {
                        SendOutcome::NotSent
                    };
                    blocking(&self.kernel, move |k| {
                        k.channel_settle(delivery_id, outcome)
                    })
                    .await?;
                    continue;
                }
            }
            let id = self.id();
            if let Some(batch) = blocking(&self.kernel, move |k| k.channel_next_batch(id)).await? {
                let lookup = batch.clone();
                let existing =
                    blocking(&self.kernel, move |k| k.channel_actor_request(&lookup)).await?;
                let turn = match existing {
                    Some(turn) => Some(turn),
                    None => match self.actor.submit_channel(batch.clone()).await {
                        Ok(turn) => Some(turn),
                        Err(_) => {
                            tracing::warn!(channel = %id, "channel Actor submission unavailable; retained for inspection");
                            None
                        }
                    },
                };
                if let Some(turn) = turn {
                    let link = turn.clone();
                    let batch_id = batch.id;
                    blocking(&self.kernel, move |k| k.channel_link(batch_id, &link)).await?;
                    if turn.state != TurnState::Running {
                        blocking(&self.kernel, move |k| k.channel_finish(batch_id, &turn)).await?;
                        continue;
                    }
                }
            }
            // Ordinary shares are admitted only after current replies/input work.
            if *connection.borrow() {
                let id = self.id();
                if let Some(delivery) = blocking(&self.kernel, move |k| k.share_claim(id)).await? {
                    let service = self.clone();
                    let channel = channel.clone();
                    shares.spawn(async move {
                        let delivery_id = delivery.id;
                        let outcome = if service.may_send_share(&delivery).await {
                            tokio::time::timeout(
                                Duration::from_secs(20),
                                channel.send_share(delivery),
                            )
                            .await
                            .unwrap_or(SendOutcome::Unknown)
                        } else {
                            SendOutcome::NotSent
                        };
                        blocking(&service.kernel, move |k| {
                            k.share_settle(delivery_id, outcome)
                        })
                        .await
                    });
                    continue;
                }
                if let Some(candidate) =
                    blocking(&self.kernel, move |k| k.share_next_job(id)).await?
                {
                    if self.actor.draft_share(candidate).await.is_err() {
                        blocking(&self.kernel, move |k| k.share_fail_start(candidate)).await?;
                    }
                    continue;
                }
            } else {
                let id = self.id();
                blocking(&self.kernel, move |k| k.sharing_snapshot(id)).await?;
            }
            // Timers correspond to a queued batch's merge window or finite expiry,
            // never an idle model polling loop. Broadcast loss merely triggers a rescan.
            let id = self.id();
            let now = crate::domain::now_ms()?;
            let due = blocking(&self.kernel, move |k| {
                Ok(k.channel_deadline(id, now)?
                    .into_iter()
                    .chain(k.share_deadline(id, now)?)
                    .min())
            })
            .await?;
            let wait = async {
                if let Some(due) = due {
                    tokio::time::sleep(Duration::from_millis(due.saturating_sub(now).max(1))).await;
                } else {
                    std::future::pending::<()>().await;
                }
            };
            tokio::select! {
                result=shares.join_next(), if !shares.is_empty()=>{if let Some(result)=result {result??;}},
                _ = shutdown.changed() => {},
                _ = changes.recv() => {},
                _ = observations.recv() => {},
                result = connection.changed() => { if result.is_err() { return Ok(()); } },
                _ = wait => {},
            }
        }
    }
}
