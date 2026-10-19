use crate::{api::Client, emit};
use anyhow::Result;
use clap::{Args, Subcommand, ValueEnum};
use lain_wire::{
    CandidateId, MemoryId, RequestId, SummaryId, WorkSessionId, WorkerTurnId, memory::*,
};
use serde_json::{Value, json};

#[derive(Clone, Copy, ValueEnum)]
pub enum Kind {
    Episodic,
    ProjectKnowledge,
    Preference,
    Reflection,
}
impl From<Kind> for MemoryKind {
    fn from(k: Kind) -> Self {
        match k {
            Kind::Episodic => Self::Episodic,
            Kind::ProjectKnowledge => Self::ProjectKnowledge,
            Kind::Preference => Self::Preference,
            Kind::Reflection => Self::Reflection,
        }
    }
}
#[derive(Clone, Copy, ValueEnum)]
pub enum Status {
    Active,
    Disabled,
    Superseded,
}
impl From<Status> for MemoryStatus {
    fn from(s: Status) -> Self {
        match s {
            Status::Active => Self::Active,
            Status::Disabled => Self::Disabled,
            Status::Superseded => Self::Superseded,
        }
    }
}
#[derive(Args)]
pub struct Scope {
    /// Absolute workspace directory on the daemon host. No local path resolution.
    #[arg(long,conflicts_with_all=["global","internal"])]
    workspace: Option<String>,
    #[arg(long, conflicts_with = "internal")]
    global: bool,
    #[arg(long)]
    internal: bool,
}
impl Scope {
    fn value(self) -> MemoryScope {
        if let Some(path) = self.workspace {
            MemoryScope::Workspace { path }
        } else if self.internal {
            MemoryScope::Internal
        } else {
            MemoryScope::Global
        }
    }
}
#[derive(Args)]
pub struct Filter {
    #[command(flatten)]
    scope: Scope,
    #[arg(long, value_enum)]
    kind: Option<Kind>,
    #[arg(long, value_enum, default_value = "active")]
    status: Status,
    #[arg(long, default_value_t = 0)]
    after: u64,
    #[arg(long,default_value_t=20,value_parser=clap::value_parser!(u16).range(1..=50))]
    limit: u16,
}
impl Filter {
    fn query(self, text: Option<String>) -> MemoryQuery {
        MemoryQuery {
            scope: self.scope.value(),
            kind: self.kind.map(Into::into),
            status: Some(self.status.into()),
            text,
            after: self.after,
            limit: self.limit,
        }
    }
}
#[derive(Subcommand)]
pub enum Commands {
    List {
        #[command(flatten)]
        filter: Filter,
    },
    Search {
        text: String,
        #[command(flatten)]
        filter: Filter,
    },
    Show {
        id: MemoryId,
    },
    /// Direct owner-authored statement; this command itself authorizes the write.
    Add {
        content: String,
        #[arg(long, value_enum)]
        kind: Kind,
        #[command(flatten)]
        scope: Scope,
        #[arg(long)]
        request_id: Option<RequestId>,
    },
    Disable {
        id: MemoryId,
        #[arg(long)]
        revision: u64,
    },
    Enable {
        id: MemoryId,
        #[arg(long)]
        revision: u64,
    },
    Supersede {
        id: MemoryId,
        #[arg(long)]
        with: MemoryId,
        #[arg(long)]
        revision: u64,
    },
    Delete {
        id: MemoryId,
        #[arg(long)]
        revision: u64,
    },
    Candidates {
        #[arg(long, default_value_t = 0)]
        after: u64,
    },
    Candidate {
        id: CandidateId,
    },
    Accept {
        id: CandidateId,
    },
    Reject {
        id: CandidateId,
    },
    /// Select project knowledge from an inspected SessionSummary for review.
    FromSummary {
        summary: SummaryId,
        content: String,
        #[arg(long)]
        workspace: String,
        #[arg(long)]
        request_id: Option<RequestId>,
    },
}
#[derive(Subcommand)]
pub enum Summaries {
    List {
        session: WorkSessionId,
    },
    Show {
        id: SummaryId,
    },
    Create {
        session: WorkSessionId,
        #[arg(long)]
        turn: WorkerTurnId,
    },
}
pub async fn run(client: &Client, command: Commands, machine: bool) -> Result<()> {
    let value: Value = match command {
        Commands::List { filter } => {
            let page = client
                .post::<MemoryPage>("v1/memory/query", Some(json!(filter.query(None))))
                .await?;
            serde_json::to_value(page)?
        }
        Commands::Search { text, filter } => {
            let page = client
                .post::<MemoryPage>("v1/memory/query", Some(json!(filter.query(Some(text)))))
                .await?;
            serde_json::to_value(page)?
        }
        Commands::Show { id } => serde_json::to_value(
            client
                .get::<MemoryRecord>(&format!("v1/memory/{id}"))
                .await?,
        )?,
        Commands::Add {
            content,
            kind,
            scope,
            request_id,
        } => {
            let input = AddMemory {
                request_id: request_id.unwrap_or_default(),
                draft: MemoryDraft {
                    content,
                    kind: kind.into(),
                    scope: scope.value(),
                },
            };
            client.post("v1/memory", Some(json!(input))).await?
        }
        Commands::Disable { id, revision } | Commands::Enable { id, revision } => {
            let status = if matches!(command, Commands::Enable { .. }) {
                MemoryStatus::Active
            } else {
                MemoryStatus::Disabled
            };
            let change = ChangeMemory {
                expected_revision: revision,
                status,
                superseded_by: None,
            };
            client
                .post(&format!("v1/memory/{id}/status"), Some(json!(change)))
                .await?
        }
        Commands::Supersede { id, with, revision } => {
            let change = ChangeMemory {
                expected_revision: revision,
                status: MemoryStatus::Superseded,
                superseded_by: Some(with),
            };
            client
                .post(&format!("v1/memory/{id}/status"), Some(json!(change)))
                .await?
        }
        Commands::Delete { id, revision } => {
            client
                .post(
                    &format!("v1/memory/{id}/delete"),
                    Some(json!({"expected_revision": revision})),
                )
                .await?
        }
        Commands::Candidates { after } => {
            client
                .get(&format!("v1/memory-candidates?after={after}&limit=50"))
                .await?
        }
        Commands::Candidate { id } => client.get(&format!("v1/memory-candidates/{id}")).await?,
        Commands::Accept { id } | Commands::Reject { id } => {
            let accept = matches!(command, Commands::Accept { .. });
            client
                .post(
                    &format!("v1/memory-candidates/{id}/decision"),
                    Some(json!({"accept": accept})),
                )
                .await?
        }
        Commands::FromSummary {
            summary,
            content,
            workspace,
            request_id,
        } => {
            let draft = MemoryDraft {
                content,
                kind: MemoryKind::ProjectKnowledge,
                scope: MemoryScope::Workspace { path: workspace },
            };
            client.post("v1/memory-candidates", Some(json!({
                "request_id": request_id.unwrap_or_default(), "summary_id": summary, "draft": draft,
            }))).await?
        }
    };
    if !machine && value.get("records").is_some() {
        let page: MemoryPage = serde_json::from_value(value)?;
        for record in page.records {
            println!(
                "{}  {:?}  {:?}  {}\n  {}",
                record.id,
                record.status,
                record.draft.kind,
                crate::tui::safe_text(&serde_json::to_string(&record.draft.scope)?),
                crate::tui::safe_text(&record.draft.content)
            );
        }
        if page.has_more {
            println!("More: --after {}", page.next_cursor);
        }
        Ok(())
    } else {
        emit(&value, machine)
    }
}
pub async fn summaries(client: &Client, command: Summaries, machine: bool) -> Result<()> {
    let value: Value = match command {
        Summaries::List { session } => {
            client
                .get(&format!("v1/work-sessions/{session}/summaries"))
                .await?
        }
        Summaries::Show { id } => client.get(&format!("v1/session-summaries/{id}")).await?,
        Summaries::Create { session, turn } => {
            client
                .post(
                    &format!("v1/work-sessions/{session}/summaries"),
                    Some(json!({"turn_id":turn})),
                )
                .await?
        }
    };
    emit(&value, machine)
}
