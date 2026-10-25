mod actor_render;
mod api;
mod channel;
mod chat;
mod chat_notice;
mod local;
mod memory;
mod signals;
mod tui;

use anyhow::{Result, ensure};
use api::{Client, Decision, Session};
use clap::{Parser, Subcommand, ValueEnum};
use lain_wire::{ActionId, ActorTurnId, ConversationId, RequestId};
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
    Channel {
        #[command(subcommand)]
        command: channel::Commands,
    },
    Memory {
        #[command(subcommand)]
        command: memory::Commands,
    },
    Summary {
        #[command(subcommand)]
        command: memory::Summaries,
    },
    Setup {
        #[arg(long, required = true)]
        local: bool,
    },
    Up {
        #[arg(long)]
        env_file: Option<std::path::PathBuf>,
    },
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    Doctor {
        #[arg(long)]
        env_file: Option<std::path::PathBuf>,
    },
    Chat {
        #[arg(long)]
        conversation: Option<ConversationId>,
        #[arg(long)]
        timing: bool,
    },
    Ask {
        text: String,
        #[arg(long)]
        conversation: Option<ConversationId>,
        #[arg(long)]
        request_id: Option<RequestId>,
        #[arg(long)]
        timing: bool,
    },
    Conversations,
    Notices {
        conversation: ConversationId,
        #[arg(long, default_value_t = 0)]
        after: u64,
    },
    Delegations {
        conversation: ConversationId,
    },
    Conversation {
        conversation: ConversationId,
        #[arg(long, default_value_t = 0)]
        after: u64,
    },
    Actor {
        #[command(subcommand)]
        command: ActorCommands,
    },
    Status,
    Profile {
        #[command(subcommand)]
        command: Profiles,
    },
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
enum ConfigCommands {
    Actor {
        #[command(subcommand)]
        command: Option<ActorConfigurationCommands>,
    },
}
#[derive(Subcommand)]
enum ActorConfigurationCommands {
    Set {
        #[arg(long)]
        file: std::path::PathBuf,
    },
    EnvFile {
        #[arg(long)]
        file: std::path::PathBuf,
    },
    Show,
    Clear,
}
#[derive(Subcommand)]
enum ActorCommands {
    Show { turn: ActorTurnId },
    Cancel { turn: ActorTurnId },
    Approve { turn: ActorTurnId, action: ActionId },
    Deny { turn: ActorTurnId, action: ActionId },
}
#[derive(Subcommand)]
enum Work {
    List,
    Start {
        #[arg(long)]
        workspace: String,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        profile: Option<String>,
        #[arg(long, value_enum)]
        sandbox: Option<Sandbox>,
    },
    Show {
        session: Uuid,
    },
    /// Inspect actual worker turns, independently of submission receipts.
    Turns {
        session: Uuid,
    },
    Attach {
        session: Uuid,
    },
    Watch {
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
#[derive(Subcommand)]
enum Profiles {
    List,
    Show { id: String },
    Validate { id: String },
}
fn profile_id(value: &str) -> Result<&str> {
    ensure!(
        !value.is_empty()
            && value.len() <= 128
            && value.split('/').all(|s| !s.is_empty()
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))),
        "invalid profile ID"
    );
    Ok(value)
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let machine = args.json;
    let result = run(args).await;
    if let Err(error) = result {
        if machine {
            eprintln!("{}", serde_json::json!({"error":error.to_string()}));
        } else {
            eprintln!("lain: {}", tui::safe_text(&error.to_string()));
        }
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
    match &args.command {
        Some(Command::Setup { local: true }) => {
            return emit(
                &local::setup_at(&local::directory()?, "http://127.0.0.1:8787")?,
                args.json,
            );
        }
        Some(Command::Up { env_file }) => return local::up(args.json, env_file.as_deref()).await,
        Some(Command::Config {
            command: ConfigCommands::Actor { command },
        }) => {
            let value = match command {
                Some(ActorConfigurationCommands::Set { file }) => {
                    local::actor_config::set_actor(file)?
                }
                Some(ActorConfigurationCommands::EnvFile { file }) => {
                    local::actor_config::set_env_file(file)?
                }
                Some(ActorConfigurationCommands::Clear) => local::actor_config::clear()?,
                Some(ActorConfigurationCommands::Show) | None => local::actor_config::show()?,
            };
            return emit(&value, args.json);
        }
        Some(Command::Doctor { env_file }) => {
            return local::doctor(args.json, env_file.as_deref()).await;
        }
        _ => {}
    }
    let connection = local::connection()?;
    let client = Client::new(&connection.url, &connection.token)?;
    let Some(command) = args.command else {
        ensure!(!args.json, "--json requires a non-interactive command");
        return tui::run(client, None).await;
    };
    let json_output = args.json;
    match command {
        Command::Channel { command } => channel::run(&client, command, json_output).await,
        Command::Memory { command } => memory::run(&client, command, json_output).await,
        Command::Summary { command } => memory::summaries(&client, command, json_output).await,
        Command::Setup { .. }
        | Command::Up { .. }
        | Command::Config { .. }
        | Command::Doctor { .. } => {
            unreachable!("handled before client configuration")
        }
        Command::Chat {
            conversation,
            timing,
        } => {
            ensure!(!json_output, "chat is interactive; use --json ask");
            chat::run(client, conversation, timing).await
        }
        Command::Ask {
            text,
            conversation,
            request_id,
            timing,
        } => chat::ask(&client, text, conversation, request_id, json_output, timing).await,
        Command::Conversations => {
            emit(&client.get::<Value>("v1/conversations").await?, json_output)
        }
        Command::Notices {
            conversation,
            after,
        } => emit(
            &client
                .get::<lain_wire::notices::NoticePage>(&format!(
                    "v1/conversations/{conversation}/notices?after={after}&limit=100"
                ))
                .await?,
            json_output,
        ),
        Command::Delegations { conversation } => emit(
            &client
                .get::<Value>(&format!("v1/conversations/{conversation}/delegations"))
                .await?,
            json_output,
        ),
        Command::Conversation {
            conversation,
            after,
        } => emit(
            &client
                .get::<Value>(&format!(
                    "v1/conversations/{conversation}?after={after}&limit=100"
                ))
                .await?,
            json_output,
        ),
        Command::Actor { command } => match command {
            ActorCommands::Show { turn } => emit(
                &client
                    .get::<Value>(&format!("v1/actor-turns/{turn}"))
                    .await?,
                json_output,
            ),
            ActorCommands::Cancel { turn } => emit(
                &client
                    .post::<Value>(&format!("v1/actor-turns/{turn}/cancel"), None)
                    .await?,
                json_output,
            ),
            ActorCommands::Approve { turn, action } | ActorCommands::Deny { turn, action } => {
                let approve = matches!(command, ActorCommands::Approve { .. });
                let initial = client
                    .post::<lain_wire::ActorSnapshot>(
                        &format!("v1/actor-turns/{turn}/actions/{action}/decision"),
                        Some(json!({"approve":approve})),
                    )
                    .await?;
                let result = chat::wait(&client, initial, !json_output).await?;
                if json_output {
                    emit(&result, true)?;
                }
                ensure!(
                    result.state == lain_wire::ActorTurnState::Completed,
                    "Actor action outcome is uncertain; inspect its recorded partial result"
                );
                Ok(())
            }
        },
        Command::Profile { command } => match command {
            Profiles::List => emit(
                &client.get::<Value>("v1/runtime-profiles").await?,
                json_output,
            ),
            Profiles::Show { id } => emit(
                &client
                    .get::<Value>(&format!(
                        "v1/runtime-profiles/inspect?id={}",
                        profile_id(&id)?
                    ))
                    .await?,
                json_output,
            ),
            Profiles::Validate { id } => emit(
                &client
                    .post::<Value>(
                        "v1/runtime-profiles/validate",
                        Some(json!({"id":profile_id(&id)?})),
                    )
                    .await?,
                json_output,
            ),
        },
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
                profile,
                sandbox,
            } => {
                // The path belongs to the server. Do not resolve/canonicalize it locally.
                if let Some(id) = &profile {
                    profile_id(id)?;
                }
                // Resolve defaults in the Kernel, including workspace policy; no local profile list.
                let value: Value = client.post("v1/work-sessions", Some(json!({"workspace":workspace,"profile_id":profile,"options":{"model":model,"sandbox":sandbox.map(|s| match s { Sandbox::ReadOnly=>"read_only", Sandbox::WorkspaceWrite=>"workspace_write" })}}))).await?;
                emit(&value, json_output)
            }
            Work::Show { session } => emit(
                &client
                    .get::<Value>(&format!("v1/work-sessions/{session}"))
                    .await?,
                json_output,
            ),
            Work::Watch { session } => chat::work_watch(client, session, json_output).await,
            Work::Turns { session } => emit(
                &client
                    .get::<Vec<api::Turn>>(&format!("v1/work-sessions/{session}/turns"))
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
