//! Text-only HTTP client. Reasoning, tools and authoritative state stay in laind.
use crate::{api::Client, emit, tui::safe_text};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    io::{self, IsTerminal, Write},
    time::Duration,
};
use tokio::sync::mpsc;
use uuid::Uuid;

fn id(value: &Value) -> Result<Uuid> {
    value
        .as_str()
        .context("Kernel identity missing")?
        .parse()
        .context("invalid Kernel identity")
}
pub async fn create(client: &Client) -> Result<Uuid> {
    let value: Value = client.post("v1/conversations", None).await?;
    id(&value["id"])
}
fn show(turn: &Value, seen: &mut BTreeSet<String>) -> Result<()> {
    if let Some(messages) = turn["messages"].as_array() {
        for message in messages {
            let key = message["id"].as_str().unwrap_or("").to_owned();
            if message["role"] == "assistant" && seen.insert(key) {
                println!(
                    "Lain> {}",
                    safe_text(message["content"].as_str().unwrap_or(""))
                );
            }
        }
    }
    if let Some(actions) = turn["actions"].as_array() {
        for action in actions {
            let key = format!("{}:{}", action["id"], action["state"]);
            if seen.insert(key) {
                println!(
                    "Tool> {}",
                    safe_text(&serde_json::to_string_pretty(action)?)
                );
            }
        }
    }
    if let Some(error) = turn["error"].as_str() {
        println!("Lain> {}", safe_text(error));
    }
    Ok(())
}
pub async fn wait(client: &Client, initial: Value, human: bool) -> Result<Value> {
    wait_seen(client, initial, human, &mut BTreeSet::new()).await
}
async fn wait_seen(
    client: &Client,
    initial: Value,
    human: bool,
    seen: &mut BTreeSet<String>,
) -> Result<Value> {
    let turn_id = id(&initial["id"])?;
    let (tx, mut rx) = mpsc::channel(16);
    let observer_client = client.clone();
    let observer = tokio::spawn(async move {
        for attempt in 0..3 {
            if observer_client.actor_observe(turn_id, &tx).await.is_ok() {
                return;
            }
            if attempt < 2 {
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
        }
    });
    let mut latest = initial;
    let result = loop {
        if human {
            show(&latest, seen)?;
        }
        if latest["state"] != "running" {
            break Ok(latest);
        }
        tokio::select! {
            _=tokio::signal::ctrl_c()=>{
                let _:Value=client.post(&format!("v1/actor-turns/{turn_id}/cancel"),None).await?;
                break Err(anyhow::anyhow!("Generation interruption requested. Resume conversation {} and inspect turn {turn_id}; workers remain Kernel-owned.",latest["conversation_id"]));
            },
            update=rx.recv()=>match update {
                Some(update)=>{if update["revision"].as_u64()>latest["revision"].as_u64() {latest=update;}},
                None=>break Err(anyhow::anyhow!("Observer disconnected. Turn {turn_id} remains Kernel-owned; resume the conversation to recover. Do not blindly resubmit a delegation.")),
            }
        }
    };
    observer.abort();
    result
}
async fn submit(client: &Client, conversation: Uuid, request: Uuid, text: String) -> Result<Value> {
    client.post(&format!("v1/conversations/{conversation}/turns"),Some(json!({"request_id":request,"text":text}))).await
        .with_context(||format!("Actor submission failed; inspect conversation {conversation} before retrying. Reuse request ID {request} for this same text. Configure the Actor with LAIN_ACTOR_CONFIG and its LAIN_SECRET variable if unavailable."))
}
pub async fn ask(
    client: &Client,
    text: String,
    conversation: Option<Uuid>,
    request: Option<Uuid>,
    machine: bool,
) -> Result<()> {
    let conversation = match conversation {
        Some(id) => id,
        None => create(client).await?,
    };
    let initial = submit(
        client,
        conversation,
        request.unwrap_or_else(Uuid::new_v4),
        text,
    )
    .await?;
    let turn = wait(client, initial, !machine).await?;
    if machine {
        emit(&turn, true)?;
    } else {
        println!("Conversation: {conversation}");
    }
    ensure!(
        !matches!(
            turn["state"].as_str(),
            Some("unknown" | "failed" | "cancelled")
        ),
        "Actor turn did not complete; inspect the conversation"
    );
    // Pending is a successful scripting result, explicitly not an executed action.
    Ok(())
}
struct Input {
    lines: mpsc::Receiver<Option<String>>,
}
impl Input {
    fn new() -> Result<Self> {
        let (sender, lines) = mpsc::channel(1);
        // A plain input thread may remain blocked at process exit. It is never joined;
        // unlike spawn_blocking, it cannot hold Tokio runtime shutdown waiting on stdin.
        std::thread::Builder::new()
            .name("lain-stdin".into())
            .spawn(move || {
                loop {
                    let mut text = String::new();
                    let line = match io::stdin().read_line(&mut text) {
                        Ok(0) | Err(_) => None,
                        Ok(_) => Some(text.trim().to_owned()),
                    };
                    let eof = line.is_none();
                    if sender.blocking_send(line).is_err() || eof {
                        break;
                    }
                }
            })?;
        Ok(Self { lines })
    }
    async fn line(&mut self, prompt: &str) -> Result<Option<String>> {
        print!("{prompt}");
        io::stdout().flush()?;
        Ok(self.lines.recv().await.flatten())
    }
}
async fn confirmation(client: &Client, mut turn: Value, input: &mut Input) -> Result<()> {
    let mut seen = BTreeSet::new();
    loop {
        turn = wait_seen(client, turn, true, &mut seen).await?;
        if turn["state"] != "pending_confirmation" {
            return Ok(());
        }
        let action = turn["actions"]
            .as_array()
            .context("missing actions")?
            .iter()
            .find(|a| a["state"] == "proposed")
            .context("missing pending proposal")?;
        let answer = tokio::select! {
            answer=input.line("Confirm this exact action? [y/N] ")=>answer?,
            _=tokio::signal::ctrl_c()=>bail!("Confirmation interrupted; the proposal remains pending for explicit review."),
        };
        let Some(answer) = answer else {
            return Ok(());
        };
        let approve = answer.eq_ignore_ascii_case("y");
        let turn_id = id(&turn["id"])?;
        let action_id = id(&action["id"])?;
        turn = client
            .post(
                &format!("v1/actor-turns/{turn_id}/actions/{action_id}/decision"),
                Some(json!({"approve":approve})),
            )
            .await?;
    }
}

pub async fn run(client: Client, conversation: Option<Uuid>) -> Result<()> {
    ensure!(
        io::stdin().is_terminal() && io::stdout().is_terminal(),
        "chat requires a normal terminal; use lain --json ask for scripts"
    );
    let diagnostics: Value = client.get("v1/diagnostics").await?;
    ensure!(
        diagnostics["actor"]["status"] == "configured",
        "Actor model unavailable: run lain doctor; set LAIN_ACTOR_CONFIG and its LAIN_SECRET variable on laind (Codex login is separate)"
    );
    let mut input = Input::new()?;
    let mut conversation = match conversation {
        Some(id) => id,
        None => create(&client).await?,
    };
    println!(
        "Conversation: {conversation}\nLain> You're here.\n/help for commands; /exit keeps this conversation."
    );
    let page: Value = client
        .get(&format!("v1/conversations/{conversation}?recent=true"))
        .await?;
    if let Some(turns) = page["turns"].as_array() {
        for turn in turns {
            if !matches!(
                turn["state"].as_str(),
                Some("running" | "pending_confirmation")
            ) {
                show(turn, &mut BTreeSet::new())?;
            }
        }
        if let Some(turn) = turns.last()
            && matches!(
                turn["state"].as_str(),
                Some("running" | "pending_confirmation")
            )
        {
            confirmation(&client, turn.clone(), &mut input).await?;
        }
        if page["has_more"] == true {
            println!(
                "Older conversation page loaded; use /history or the conversation API cursor for all records."
            );
        }
    }
    loop {
        let text = tokio::select! {
            text=input.line("You> ")=>text?,
            _=tokio::signal::ctrl_c()=>return Ok(()),
        };
        let Some(text) = text else {
            return Ok(());
        };
        if text.is_empty() {
            continue;
        }
        match text.as_str() {
            "/exit" => return Ok(()),
            "/help" => println!(
                "/help /status /work /conversations /history /new /exit\nOrdinary text goes to Lain. Each mutation requires explicit confirmation."
            ),
            "/status" => emit(&client.get::<Value>("v1/diagnostics").await?, false)?,
            "/work" => emit(&client.get::<Value>("v1/work-sessions").await?, false)?,
            "/conversations" => emit(&client.get::<Value>("v1/conversations").await?, false)?,
            "/history" => emit(
                &client
                    .get::<Value>(&format!("v1/conversations/{conversation}?limit=100"))
                    .await?,
                false,
            )?,
            "/new" => {
                conversation = create(&client).await?;
                println!("Conversation: {conversation}");
            }
            value if value.starts_with('/') => println!("Unknown command; /help lists commands."),
            _ => match submit(&client, conversation, Uuid::new_v4(), text).await {
                Ok(turn) => {
                    if let Err(error) = confirmation(&client, turn, &mut input).await {
                        eprintln!("{}", safe_text(&error.to_string()));
                    }
                }
                Err(error) => eprintln!("{}", safe_text(&error.to_string())),
            },
        }
    }
}
pub async fn work_watch(client: Client, session: Uuid, machine: bool) -> Result<()> {
    let (tx, mut rx) = mpsc::channel(32);
    let observer = tokio::spawn(client.observe(tx));
    loop {
        tokio::select! {
            _=tokio::signal::ctrl_c()=>{observer.abort();return Ok(());},
            update=rx.recv()=>match update {
                Some(crate::api::Update::Event(event)) if event.work_session_id==session=>{if machine {emit(&event,true)?;} else {println!("{}",crate::tui::event_text(&event));}},
                Some(crate::api::Update::Snapshot(snapshot))=>{if let Some(work)=snapshot.sessions.into_iter().find(|s|s.id==session) {emit(&work,machine)?;} else {observer.abort();bail!("work session not found");}},
                Some(crate::api::Update::Offline(message))=>eprintln!("{}",safe_text(&message)),
                Some(_)=>{},None=>bail!("observer stopped"),
            }
        }
    }
}
