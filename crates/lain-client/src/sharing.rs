use crate::{api::Client, emit};
use anyhow::Result;
use clap::Subcommand;
use lain_wire::{
    ChannelId, DeliveryId, ExperienceId,
    sharing::{ShareQuery, ShareScope},
};
use serde_json::{Value, json};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum Commands {
    Recall {
        channel: ChannelId,
        #[arg(long)]
        from_ms: u64,
        #[arg(long)]
        to_ms: u64,
        #[arg(long)]
        text: Option<String>,
        #[arg(long, default_value_t = 3)]
        limit: u16,
    },
    Receipt {
        channel: ChannelId,
        delivery: DeliveryId,
    },
    Audit {
        channel: ChannelId,
    },
    Show {
        channel: ChannelId,
    },
    /// Independent contact grant. JSON requires every policy field; it has no defaults.
    Enable {
        channel: ChannelId,
        #[arg(long)]
        revision: u64,
        #[arg(long)]
        scope: PathBuf,
    },
    Revoke {
        channel: ChannelId,
        #[arg(long)]
        revision: u64,
    },
    /// Explicitly restrict ordinary sharing; replies, approvals and work remain separate.
    Dnd {
        channel: ChannelId,
        #[arg(long)]
        revision: u64,
        #[arg(long)]
        until_ms: Option<u64>,
    },
    Available {
        channel: ChannelId,
        #[arg(long)]
        revision: u64,
    },
    /// Consider an existing experience. It cannot replay a prior candidate/send.
    Consider {
        channel: ChannelId,
        experience: ExperienceId,
    },
}
pub async fn run(client: &Client, command: Commands, machine: bool) -> Result<()> {
    let value: Value = match command {
        Commands::Recall {
            channel,
            from_ms,
            to_ms,
            text,
            limit,
        } => {
            client
                .post(
                    &format!("v1/channels/{channel}/sharing/receipts/search"),
                    Some(serde_json::to_value(ShareQuery {
                        from_ms,
                        to_ms,
                        text,
                        limit,
                    })?),
                )
                .await?
        }
        Commands::Receipt { channel, delivery } => {
            client
                .get(&format!(
                    "v1/channels/{channel}/sharing/receipts/{delivery}"
                ))
                .await?
        }
        Commands::Audit { channel } => {
            client
                .get(&format!("v1/channels/{channel}/sharing/audit"))
                .await?
        }
        Commands::Show { channel } => {
            client
                .get(&format!("v1/channels/{channel}/sharing"))
                .await?
        }
        Commands::Enable {
            channel,
            revision,
            scope,
        } => {
            let scope: ShareScope = serde_json::from_slice(&std::fs::read(scope)?)?;
            client
                .post(
                    &format!("v1/channels/{channel}/sharing/grant"),
                    Some(json!({"expected_revision":revision,"scope":scope})),
                )
                .await?
        }
        Commands::Revoke { channel, revision } => {
            client
                .post(
                    &format!("v1/channels/{channel}/sharing/grant"),
                    Some(json!({"expected_revision":revision,"scope":null})),
                )
                .await?
        }
        Commands::Dnd {
            channel,
            revision,
            until_ms,
        } => client
            .post(
                &format!("v1/channels/{channel}/attention"),
                Some(
                    json!({"expected_revision":revision,"do_not_disturb":true,"until_ms":until_ms}),
                ),
            )
            .await?,
        Commands::Available { channel, revision } => client
            .post(
                &format!("v1/channels/{channel}/attention"),
                Some(json!({"expected_revision":revision,"do_not_disturb":false,"until_ms":null})),
            )
            .await?,
        Commands::Consider {
            channel,
            experience,
        } => {
            client
                .post(
                    &format!("v1/channels/{channel}/sharing/consider/{experience}"),
                    None,
                )
                .await?
        }
    };
    emit(&value, machine)
}
