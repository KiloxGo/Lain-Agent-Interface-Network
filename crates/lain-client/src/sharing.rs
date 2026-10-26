use crate::{api::Client, emit};
use anyhow::Result;
use clap::Subcommand;
use lain_wire::{ChannelId, ExperienceId, sharing::ShareScope};
use serde_json::{Value, json};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum Commands {
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
