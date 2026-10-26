use crate::{api::Client, emit};
use anyhow::Result;
use clap::Subcommand;
use lain_wire::{ReadingId, RequestId, reading::*};
use serde_json::{Value, json};

#[derive(Subcommand)]
pub enum Commands {
    Grant,
    Grants {
        #[arg(long, default_value_t = 0)]
        after: u64,
    },
    /// Explicit, independent authority; grants no memory or contact permission.
    Enable {
        #[arg(long)]
        revision: u64,
        #[arg(long, required = true)]
        url: Vec<String>,
        #[arg(long)]
        expires_at_ms: u64,
        #[arg(long)]
        retention_ms: u64,
        #[arg(long)]
        max_requests: u16,
        #[arg(long)]
        max_bytes: u32,
    },
    Revoke {
        #[arg(long)]
        revision: u64,
    },
    List,
    /// Persist a bounded acquisition. Reuse request-id to inspect an uncertain submit.
    Start {
        url: String,
        #[arg(long)]
        revision: u64,
        #[arg(long)]
        request_id: RequestId,
    },
    Show {
        id: ReadingId,
    },
    Cancel {
        id: ReadingId,
    },
}

pub async fn run(client: &Client, command: Commands, machine: bool) -> Result<()> {
    let result: Value = match command {
        Commands::Grant => client.get("v1/reading/grant").await?,
        Commands::Grants { after } => {
            client
                .get(&format!("v1/reading/grant/history?after={after}"))
                .await?
        }
        Commands::Enable {
            revision,
            url,
            expires_at_ms,
            retention_ms,
            max_requests,
            max_bytes,
        } => {
            let change = SetReadingGrant {
                expected_revision: revision,
                scope: Some(ReadingScope {
                    urls: url,
                    expires_at_ms,
                    retention_ms,
                    max_requests,
                    max_bytes,
                }),
            };
            client
                .post("v1/reading/grant", Some(serde_json::to_value(change)?))
                .await?
        }
        Commands::Revoke { revision } => {
            client
                .post(
                    "v1/reading/grant",
                    Some(json!({"expected_revision":revision,"scope":null})),
                )
                .await?
        }
        Commands::List => client.get("v1/reading/activities").await?,
        Commands::Start {
            url,
            revision,
            request_id,
        } => {
            client
                .post(
                    "v1/reading/activities",
                    Some(serde_json::to_value(StartReading {
                        request_id,
                        grant_revision: revision,
                        url,
                    })?),
                )
                .await?
        }
        Commands::Show { id } => client.get(&format!("v1/reading/activities/{id}")).await?,
        Commands::Cancel { id } => {
            client
                .post(&format!("v1/reading/activities/{id}/cancel"), None)
                .await?
        }
    };
    emit(&result, machine)
}
