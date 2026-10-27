use crate::{api::Client, emit};
use anyhow::{Result, ensure};
use clap::Subcommand;
use lain_wire::{ArchiveRecordId, ArchiveSourceId, RequestId};
use serde_json::{Value, json};
use std::{io::Read, path::PathBuf};

#[derive(Subcommand)]
pub enum Commands {
    KnowledgeSyncGrants {
        id: ArchiveSourceId,
    },
    KnowledgeSync {
        id: ArchiveSourceId,
    },
    /// File selects finite request/interval/expiry scope and current archive revision.
    KnowledgeSyncEnable {
        id: ArchiveSourceId,
        #[arg(long)]
        file: PathBuf,
    },
    KnowledgeSyncRevoke {
        id: ArchiveSourceId,
        #[arg(long)]
        revision: u64,
    },
    KnowledgeState {
        id: ArchiveSourceId,
    },
    KnowledgePull {
        id: ArchiveSourceId,
        #[arg(long)]
        request_id: RequestId,
        #[arg(long)]
        revision: u64,
        #[arg(long)]
        after: Option<RequestId>,
        #[arg(long, default_value_t = 16)]
        limit: u16,
    },
    RecallGrant {
        channel: lain_wire::ChannelId,
    },
    /// File contains expected_revision and a finite selected-source recall scope.
    RecallEnable {
        channel: lain_wire::ChannelId,
        #[arg(long)]
        file: PathBuf,
    },
    RecallRevoke {
        channel: lain_wire::ChannelId,
        #[arg(long)]
        revision: u64,
    },
    RecallReads {
        channel: lain_wire::ChannelId,
        #[arg(long, default_value_t = 0)]
        after: u64,
    },
    /// One explicit local Codex visible-history page; no background collection.
    Read {
        id: ArchiveSourceId,
        #[arg(long)]
        request_id: RequestId,
        #[arg(long)]
        revision: u64,
        #[arg(long)]
        after: Option<RequestId>,
        #[arg(long, default_value_t = 4)]
        limit: u16,
    },
    ReadJob {
        id: RequestId,
    },
    CancelRead {
        id: RequestId,
    },
    Sources,
    Grants {
        id: ArchiveSourceId,
    },
    Edits {
        id: ArchiveRecordId,
    },
    /// Register one explicit source key; grants remain disabled.
    Register {
        #[arg(long)]
        file: PathBuf,
    },
    Source {
        id: ArchiveSourceId,
    },
    Enable {
        id: ArchiveSourceId,
        #[arg(long)]
        revision: u64,
        #[arg(long)]
        expires_at_ms: u64,
        #[arg(long)]
        retention_ms: u64,
        #[arg(long)]
        max_records: u32,
    },
    Revoke {
        id: ArchiveSourceId,
        #[arg(long)]
        revision: u64,
    },
    /// Import an explicitly selected small normalized file, never scan a directory.
    Import {
        id: ArchiveSourceId,
        #[arg(long)]
        file: PathBuf,
    },
    /// File contains revision, bounded limit and optional saved cursor.
    Query {
        id: ArchiveSourceId,
        #[arg(long)]
        file: PathBuf,
    },
    Record {
        id: ArchiveRecordId,
    },
    Forget {
        id: ArchiveRecordId,
        #[arg(long)]
        revision: u64,
    },
    /// File contains expected_revision and replacement text; no history body copied.
    Correct {
        id: ArchiveRecordId,
        #[arg(long)]
        file: PathBuf,
    },
}
fn file(path: PathBuf) -> Result<Value> {
    let f = std::fs::File::open(path)?;
    ensure!(
        f.metadata()?.is_file(),
        "archive input must be an explicit ordinary JSON file"
    );
    let mut bytes = Vec::new();
    f.take(65537).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 65536, "archive JSON exceeds 64 KiB");
    serde_json::from_slice(&bytes).map_err(|_| anyhow::anyhow!("invalid archive JSON"))
}
pub async fn run(client: &Client, command: Commands, machine: bool) -> Result<()> {
    let value:Value=match command {
        Commands::KnowledgeSyncGrants{id}=>client.get(&format!("v1/archives/sources/{id}/knowledge/sync/grants")).await?,
        Commands::KnowledgeSync{id}=>client.get(&format!("v1/archives/sources/{id}/knowledge/sync")).await?,
        Commands::KnowledgeSyncEnable{id,file:p}=>client.post(&format!("v1/archives/sources/{id}/knowledge/sync"),Some(file(p)?)).await?,
        Commands::KnowledgeSyncRevoke{id,revision}=>client.post(&format!("v1/archives/sources/{id}/knowledge/sync"),Some(json!({"expected_revision":revision,"scope":null}))).await?,
        Commands::KnowledgeState{id}=>client.get(&format!("v1/archives/sources/{id}/knowledge")).await?,
        Commands::KnowledgePull{id,request_id,revision,after,limit}=>client.post(&format!("v1/archives/sources/{id}/knowledge/pull"),Some(json!({"request_id":request_id,"expected_revision":revision,"after":after,"limit":limit}))).await?,
        Commands::RecallGrant{channel}=>client.get(&format!("v1/archives/recall/{channel}")).await?,
        Commands::RecallEnable{channel,file:p}=>client.post(&format!("v1/archives/recall/{channel}"),Some(file(p)?)).await?,
        Commands::RecallRevoke{channel,revision}=>client.post(&format!("v1/archives/recall/{channel}"),Some(json!({"expected_revision":revision,"scope":null}))).await?,
        Commands::RecallReads{channel,after}=>client.get(&format!("v1/archives/recall/{channel}/reads?after={after}")).await?,
        Commands::Read{id,request_id,revision,after,limit}=>client.post(&format!("v1/archives/sources/{id}/read"),Some(json!({"request_id":request_id,"expected_revision":revision,"after":after,"limit":limit}))).await?,
        Commands::ReadJob{id}=>client.get(&format!("v1/archives/reads/{id}")).await?,
        Commands::CancelRead{id}=>client.post(&format!("v1/archives/reads/{id}/cancel"),None).await?,
        Commands::Sources=>client.get("v1/archives/sources").await?,
        Commands::Grants{id}=>client.get(&format!("v1/archives/sources/{id}/grants")).await?,
        Commands::Edits{id}=>client.get(&format!("v1/archives/records/{id}/edits")).await?,
        Commands::Register{file:p}=>client.post("v1/archives/sources",Some(file(p)?)).await?,
        Commands::Source{id}=>client.get(&format!("v1/archives/sources/{id}")).await?,
        Commands::Enable{id,revision,expires_at_ms,retention_ms,max_records}=>client.post(&format!("v1/archives/sources/{id}/grant"),Some(json!({"expected_revision":revision,"scope":{"expires_at_ms":expires_at_ms,"retention_ms":retention_ms,"max_records":max_records}}))).await?,
        Commands::Revoke{id,revision}=>client.post(&format!("v1/archives/sources/{id}/grant"),Some(json!({"expected_revision":revision,"scope":null}))).await?,
        Commands::Import{id,file:p}=>client.post(&format!("v1/archives/sources/{id}/import"),Some(file(p)?)).await?,
        Commands::Query{id,file:p}=>client.post(&format!("v1/archives/sources/{id}/query"),Some(file(p)?)).await?,
        Commands::Record{id}=>client.get(&format!("v1/archives/records/{id}")).await?,
        Commands::Forget{id,revision}=>client.post(&format!("v1/archives/records/{id}/forget"),Some(json!({"expected_revision":revision}))).await?,
        Commands::Correct{id,file:p}=>client.post(&format!("v1/archives/records/{id}/correct"),Some(file(p)?)).await?,
    };
    emit(&value, machine)
}
