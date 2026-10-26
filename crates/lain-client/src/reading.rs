use crate::{api::Client, emit};
use anyhow::Result;
use clap::Subcommand;
use lain_wire::{ChannelId, ExperienceId, ReadingId, RequestId, reading::*};
use serde_json::{Value, json};

#[derive(Subcommand)]
pub enum Commands {
    Budget,
    Audit,
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
        #[arg(long, requires_all=["model_requests", "utc_offset_minutes"])]
        model_channel: Option<ChannelId>,
        #[arg(long, requires = "model_channel")]
        model_requests: Option<u16>,
        #[arg(long, requires = "model_channel", allow_hyphen_values = true)]
        utc_offset_minutes: Option<i16>,
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
    Process {
        id: ReadingId,
    },
    Suppressed,
    RestoreSource {
        url: String,
    },
}

pub async fn run(client: &Client, command: Commands, machine: bool) -> Result<()> {
    let result: Value = match command {
        Commands::Budget => client.get("v1/reading/budget").await?,
        Commands::Audit => client.get("v1/reading/audit").await?,
        Commands::Suppressed => client.get("v1/reading/suppressed").await?,
        Commands::RestoreSource { url } => {
            client
                .post(
                    "v1/reading/restore-source",
                    Some(serde_json::to_value(RestoreReadingSource { url })?),
                )
                .await?
        }
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
            model_channel,
            model_requests,
            utc_offset_minutes,
        } => {
            let change = SetReadingGrant {
                expected_revision: revision,
                scope: Some(ReadingScope {
                    urls: url,
                    expires_at_ms,
                    retention_ms,
                    max_requests,
                    max_bytes,
                    processing: model_channel.map(|channel_id| ReadingModelScope {
                        channel_id,
                        max_requests: model_requests.expect("required by clap"),
                        utc_offset_minutes: utc_offset_minutes.expect("required by clap"),
                    }),
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
        Commands::Process { id } => {
            client
                .post(&format!("v1/reading/activities/{id}/process"), None)
                .await?
        }
    };
    emit(&result, machine)
}

#[derive(Subcommand)]
pub enum Experiences {
    History {
        id: ExperienceId,
    },
    Correct {
        id: ExperienceId,
        summary: String,
        #[arg(long)]
        revision: u64,
    },
    Search {
        #[arg(long)]
        from_ms: u64,
        #[arg(long)]
        to_ms: u64,
        #[arg(long)]
        text: Option<String>,
        #[arg(long, default_value_t = 3)]
        limit: u16,
    },
    Show {
        id: ExperienceId,
    },
    /// Withdraws the derived summary and source body; retains operation metadata.
    Withdraw {
        id: ExperienceId,
        #[arg(long)]
        revision: u64,
    },
}
pub async fn experiences(client: &Client, command: Experiences, machine: bool) -> Result<()> {
    let result: Value = match command {
        Experiences::History { id } => client.get(&format!("v1/experiences/{id}/history")).await?,
        Experiences::Correct {
            id,
            summary,
            revision,
        } => {
            client
                .post(
                    &format!("v1/experiences/{id}/correct"),
                    Some(serde_json::to_value(CorrectExperience {
                        expected_revision: revision,
                        summary,
                    })?),
                )
                .await?
        }
        Experiences::Search {
            from_ms,
            to_ms,
            text,
            limit,
        } => {
            client
                .post(
                    "v1/experiences/search",
                    Some(serde_json::to_value(ExperienceQuery {
                        from_ms,
                        to_ms,
                        text,
                        limit,
                    })?),
                )
                .await?
        }
        Experiences::Show { id } => client.get(&format!("v1/experiences/{id}")).await?,
        Experiences::Withdraw { id, revision } => {
            client
                .post(
                    &format!("v1/experiences/{id}/withdraw"),
                    Some(serde_json::to_value(WithdrawExperience {
                        expected_revision: revision,
                    })?),
                )
                .await?
        }
    };
    emit(&result, machine)
}
