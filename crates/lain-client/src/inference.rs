use crate::{api::Client, emit};
use anyhow::Result;
use clap::Subcommand;
use lain_wire::InferenceBudgetId;
use serde_json::{Value, json};
#[derive(Subcommand)]
pub enum Commands {
    Show,
    Budgets,
    Requests {
        #[arg(long)]
        budget: Option<InferenceBudgetId>,
    },
    /// Add a finite Kernel-wide Lain ceiling, not permission to call a provider.
    Enable {
        #[arg(long)]
        revision: u64,
        #[arg(long)]
        id: InferenceBudgetId,
        #[arg(long)]
        max_requests: u32,
        #[arg(long)]
        expires_at_ms: u64,
    },
    /// Stop future Lain inference. Already reserved/attempted requests are never refunded.
    Stop {
        #[arg(long)]
        revision: u64,
    },
}
pub async fn run(client: &Client, command: Commands, machine: bool) -> Result<()> {
    let value:Value=match command {
        Commands::Show=>client.get("v1/inference/budget").await?,
        Commands::Budgets=>client.get("v1/inference/budgets").await?,
        Commands::Requests{budget}=>client.get(&budget.map_or("v1/inference/requests".into(),|id|format!("v1/inference/requests?budget={id}"))).await?,
        Commands::Enable{revision,id,max_requests,expires_at_ms}=>client.post("v1/inference/budget",Some(json!({"expected_revision":revision,"limit":{"id":id,"max_requests":max_requests,"expires_at_ms":expires_at_ms}}))).await?,
        Commands::Stop{revision}=>client.post("v1/inference/budget",Some(json!({"expected_revision":revision,"limit":null}))).await?,
    };
    emit(&value, machine)
}
