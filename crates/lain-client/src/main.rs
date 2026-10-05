mod api;
mod tui;

use anyhow::{Result, ensure};
use api::{Client, Decision, Session};
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::{Value, json};
use std::io::{self, Write};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "lain", version, about = "Lain Kernel terminal client")]
struct Args {
    #[arg(long, global = true, help = "Print JSON for non-interactive commands")]
    json: bool,
    #[command(subcommand)]
    command: Option<Command>,
}
#[derive(Subcommand)]
enum Command {
    Status,
    Work {
        #[command(subcommand)]
        command: Work,
    },
    Approval {
        #[command(subcommand)]
        command: Approvals,
    },
}
#[derive(Subcommand)]
enum Work {
    List,
    Start {
        #[arg(long)]
        workspace: String,
        #[arg(long)]
        model: Option<String>,
        #[arg(long, value_enum, default_value = "read-only")]
        sandbox: Sandbox,
    },
    Show {
        session: Uuid,
    },
    Attach {
        session: Uuid,
    },
    Send {
        session: Uuid,
        text: String,
    },
    Cancel {
        session: Uuid,
    },
    Resume {
        session: Uuid,
    },
    History {
        session: Uuid,
        #[arg(long, default_value_t = 0)]
        after: u64,
        #[arg(long)]
        until: Option<u64>,
        #[arg(long, default_value_t=100, value_parser=clap::value_parser!(u16).range(1..=200))]
        limit: u16,
    },
}
#[derive(Clone, Copy, ValueEnum)]
enum Sandbox {
    ReadOnly,
    WorkspaceWrite,
}
#[derive(Subcommand)]
enum Approvals {
    List { session: Uuid },
    Approve { session: Uuid, approval: Uuid },
    Deny { session: Uuid, approval: Uuid },
    Cancel { session: Uuid, approval: Uuid },
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let result = run(args).await;
    if let Err(error) = result {
        eprintln!("lain: {}", tui::safe_text(&error.to_string()));
        std::process::exit(1);
    }
}
fn emit(value: &impl serde::Serialize, machine: bool) -> Result<()> {
    let encoded = serde_json::to_string_pretty(value)?;
    let mut stdout = io::stdout().lock();
    writeln!(
        stdout,
        "{}",
        if machine {
            encoded
        } else {
            tui::safe_text(&encoded)
        }
    )?;
    Ok(())
}
async fn run(args: Args) -> Result<()> {
    let token = std::env::var("LAIN_API_TOKEN")
        .map_err(|_| anyhow::anyhow!("set LAIN_API_TOKEN to the Kernel authentication token"))?;
    let client = Client::new(
        &std::env::var("LAIN_URL").unwrap_or_else(|_| "http://127.0.0.1:8787".into()),
        &token,
    )?;
    let Some(command) = args.command else {
        ensure!(!args.json, "--json requires a non-interactive command");
        return tui::run(client, None).await;
    };
    let json_output = args.json;
    match command {
        Command::Status => {
            let health: Value = client.get("health").await?;
            let ready: Value = client.get("ready").await?;
            let backends: Value = client.get("v1/backends").await?;
            emit(
                &json!({"health":health,"kernel":ready,"backends":backends}),
                json_output,
            )
        }
        Command::Work { command } => match command {
            Work::List => {
                let value: Value = client.get("v1/work-sessions").await?;
                if json_output {
                    return emit(&value, true);
                }
                let sessions: Vec<Session> = serde_json::from_value(value)?;
                let mut out = io::stdout().lock();
                writeln!(
                    out,
                    "SESSION                               BACKEND  STATE                 RUNTIME    WORKSPACE"
                )?;
                for s in sessions {
                    writeln!(
                        out,
                        "{}  {:8} {:21} {:10} {}",
                        s.id,
                        tui::safe_text(&s.backend),
                        tui::safe_text(&s.state),
                        tui::safe_text(&s.runtime),
                        tui::safe_text(&s.workspace)
                    )?;
                }
                Ok(())
            }
            Work::Start {
                workspace,
                model,
                sandbox,
            } => {
                // The path belongs to the server. Do not resolve/canonicalize it locally.
                let value: Value = client.post("v1/work-sessions", Some(json!({"workspace":workspace,"options":{"model":model,"sandbox":match sandbox { Sandbox::ReadOnly=>"read_only", Sandbox::WorkspaceWrite=>"workspace_write" }}}))).await?;
                emit(&value, json_output)
            }
            Work::Show { session } => emit(
                &client
                    .get::<Value>(&format!("v1/work-sessions/{session}"))
                    .await?,
                json_output,
            ),
            Work::Attach { session } => {
                ensure!(
                    !json_output,
                    "attach requires an interactive terminal; use show/history --json"
                );
                tui::run(client, Some(session)).await
            }
            Work::Send { session, text } => emit(
                &client
                    .post::<Value>(
                        &format!("v1/work-sessions/{session}/turns"),
                        Some(json!({"text":text})),
                    )
                    .await?,
                json_output,
            ),
            Work::Cancel { session } => emit(
                &client
                    .post::<Value>(&format!("v1/work-sessions/{session}/cancel"), None)
                    .await?,
                json_output,
            ),
            Work::Resume { session } => emit(
                &client
                    .post::<Value>(&format!("v1/work-sessions/{session}/resume"), None)
                    .await?,
                json_output,
            ),
            Work::History {
                session,
                after,
                until,
                limit,
            } => {
                let page = client.history(session, after, until, limit.into()).await?;
                if json_output {
                    return emit(&page, true);
                }
                let mut out = io::stdout().lock();
                for event in &page.events {
                    writeln!(out, "{}", tui::event_text(event))?;
                }
                if page.has_more {
                    writeln!(out, "More history: --after {}", page.next_cursor)?;
                }
                Ok(())
            }
        },
        Command::Approval { command } => match command {
            Approvals::List { session } => emit(
                &client
                    .get::<Value>(&format!("v1/work-sessions/{session}/approvals"))
                    .await?,
                json_output,
            ),
            Approvals::Approve { session, approval } => emit(
                &client
                    .decide(session, approval, Decision::ApproveOnce)
                    .await?,
                json_output,
            ),
            Approvals::Deny { session, approval } => emit(
                &client.decide(session, approval, Decision::Deny).await?,
                json_output,
            ),
            Approvals::Cancel { session, approval } => emit(
                &client.decide(session, approval, Decision::Cancel).await?,
                json_output,
            ),
        },
    }
}
