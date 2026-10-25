//! Serialized text client. Inference and authoritative state stay in laind.
use crate::{
    actor_render::Renderer,
    api::{ActorUpdate, Client},
    chat_notice::NoticeInbox,
    emit,
    tui::safe_text,
};
use anyhow::{Context, Result, bail, ensure};
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers},
    terminal,
};
use futures_util::StreamExt;
use lain_wire::{ActionState, ActorSnapshot, ActorTurnState, ConversationId, RequestId};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    io::{self, IsTerminal, Write},
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use uuid::Uuid;

pub async fn create(client: &Client) -> Result<ConversationId> {
    #[derive(Deserialize)]
    struct Created {
        id: ConversationId,
    }
    Ok(client.post::<Created>("v1/conversations", None).await?.id)
}
fn validate(turn: &ActorSnapshot) -> Result<()> {
    turn.validate().map_err(anyhow::Error::msg)
}

pub async fn wait(client: &Client, initial: ActorSnapshot, human: bool) -> Result<ActorSnapshot> {
    let mut renderer = Renderer::new(io::stdout());
    wait_seen(client, initial, human, &mut renderer, None).await
}
async fn wait_seen(
    client: &Client,
    initial: ActorSnapshot,
    human: bool,
    renderer: &mut Renderer<io::Stdout>,
    mut notices: Option<&mut NoticeInbox>,
) -> Result<ActorSnapshot> {
    validate(&initial)?;
    if human {
        renderer.snapshot(&initial)?;
    }
    if initial.state != ActorTurnState::Running {
        return Ok(initial);
    }
    let turn_id = initial.id;
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
    // Keep all exits inside this future so observer cancellation also happens on
    // malformed data, rendering failure and failed interruption requests.
    let result = async {
        loop {
            tokio::select! {
                update=crate::chat_notice::next(&mut notices)=>{
                    if let Some(inbox) = notices.as_deref_mut() { inbox.render(update, renderer)?; }
                },
                _=crate::signals::shutdown()?=>{
                    let _:ActorSnapshot=client.post(&format!("v1/actor-turns/{turn_id}/cancel"),None).await?;
                    bail!("Generation interruption requested. Resume conversation {} and inspect turn {turn_id}; workers remain Kernel-owned.", latest.conversation_id);
                },
                update=rx.recv()=>match update {
                    Some(ActorUpdate::Snapshot(update))=>{
                        ensure!(update.id == latest.id && update.conversation_id == latest.conversation_id,
                            "Actor snapshot identity changed");
                        if update.revision <= latest.revision { continue; }
                        latest = *update;
                        if human { renderer.snapshot(&latest)?; }
                        if latest.state != ActorTurnState::Running { return Ok(latest); }
                    },
                    Some(ActorUpdate::Preview(update))=>{
                        if human { renderer.preview(update, &latest)?; }
                    },
                    None=>bail!("Observer disconnected. Turn {turn_id} remains Kernel-owned; resume the conversation to recover. Do not blindly resubmit a delegation."),
                }
            }
        }
    }.await;
    observer.abort();
    if result.is_err() && human {
        renderer.interrupted("observation interrupted")?;
    }
    result
}
async fn submit(
    client: &Client,
    conversation: ConversationId,
    request: RequestId,
    text: String,
) -> Result<ActorSnapshot> {
    let turn: ActorSnapshot = client.post(&format!("v1/conversations/{conversation}/turns"),Some(json!({"request_id":request,"text":text}))).await
        .with_context(||format!("Actor submission failed; inspect conversation {conversation} before retrying. Reuse request ID {request} for this same text. Configure the Actor on laind if unavailable."))?;
    validate(&turn)?;
    ensure!(
        turn.conversation_id == conversation && turn.request_id == request,
        "Actor submission identity mismatch"
    );
    Ok(turn)
}
fn millis(duration: Duration) -> u64 {
    duration.as_millis().min(u64::MAX as u128) as u64
}
struct Latency {
    started: Instant,
    submission_ms: u64,
}
impl Latency {
    fn report(&self, turn: &ActorSnapshot, renderer: &Renderer<io::Stdout>) -> Result<()> {
        // stderr only, including JSON mode. No text, credentials or provider frames.
        let visible = renderer
            .first_visible
            .and_then(|at| at.checked_duration_since(self.started))
            .map(millis);
        eprintln!(
            "Timing> {}",
            serde_json::to_string(&json!({
                "cli_submission_ms":self.submission_ms, "cli_first_visible_ms":visible,
                "cli_total_ms":millis(self.started.elapsed()), "kernel":turn.timing
            }))?
        );
        Ok(())
    }
}
pub async fn ask(
    client: &Client,
    text: String,
    conversation: Option<ConversationId>,
    request: Option<RequestId>,
    machine: bool,
    timing: bool,
) -> Result<()> {
    let conversation = match conversation {
        Some(id) => id,
        None => create(client).await?,
    };
    let started = Instant::now();
    let initial = submit(client, conversation, request.unwrap_or_default(), text).await?;
    let latency = Latency {
        started,
        submission_ms: millis(started.elapsed()),
    };
    let mut renderer = Renderer::new(io::stdout());
    let turn = wait_seen(client, initial, !machine, &mut renderer, None).await?;
    if timing {
        latency.report(&turn, &renderer)?;
    }
    if machine {
        emit(&turn, true)?;
    } else {
        println!("Conversation: {conversation}");
    }
    ensure!(
        !matches!(
            turn.state,
            ActorTurnState::Unknown | ActorTurnState::Failed | ActorTurnState::Cancelled
        ),
        "Actor turn did not complete; inspect the conversation"
    );
    Ok(())
}

// Raw mode is scoped to the one active line read. There is no background stdin
// reader during submission/rendering; one task owns all prompt and response output.
struct RawLine;
impl RawLine {
    fn new() -> Result<Self> {
        terminal::enable_raw_mode()?;
        Ok(Self)
    }
}
impl Drop for RawLine {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}
enum InputResult {
    Line(String),
    Exit,
}
#[derive(PartialEq, Eq)]
enum InputState {
    Ready,
    Reading,
    Closed,
}
fn unknown_command(text: &str) -> bool {
    // A single slash word is ambiguous with a root directory. Reserve it for
    // typo feedback; paths containing separators, dots or spaces are ordinary
    // input. This is syntax classification, never local/remote path resolution.
    text.strip_prefix('/').is_some_and(|word| {
        !word.is_empty()
            && word
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
    })
}
fn input_rows(prompt: &str, text: &str, columns: u16) -> u16 {
    let columns = usize::from(columns.max(1));
    let mut column = 0;
    let mut rows = 0usize;
    let visible_text = safe_text(text);
    for c in prompt.chars().chain(visible_text.chars()) {
        let width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if width == 0 {
            continue;
        }
        if column + width > columns {
            rows += 1;
            column = 0;
        }
        column += width;
    }
    rows.min(u16::MAX as usize) as u16
}
fn clear_input(prompt: &str, text: &str) -> Result<()> {
    let (columns, height) = terminal::size()?;
    let rows = input_rows(prompt, text, columns).min(height.saturating_sub(1));
    crossterm::execute!(io::stdout(), crossterm::cursor::MoveToColumn(0))?;
    if rows != 0 {
        crossterm::execute!(io::stdout(), crossterm::cursor::MoveUp(rows))?;
    }
    crossterm::execute!(
        io::stdout(),
        terminal::Clear(terminal::ClearType::FromCursorDown)
    )?;
    Ok(())
}
struct Input {
    state: InputState,
    events: EventStream,
    swallow_lf: bool,
}
impl Input {
    fn new() -> Self {
        Self {
            state: InputState::Ready,
            events: EventStream::new(),
            swallow_lf: false,
        }
    }
    async fn line(
        &mut self,
        prompt: &str,
        allow_empty: bool,
        inbox: &mut NoticeInbox,
        renderer: &mut Renderer<io::Stdout>,
    ) -> Result<InputResult> {
        ensure!(
            self.state == InputState::Ready,
            "interactive input already owned or closed"
        );
        self.state = InputState::Reading;
        let _raw = RawLine::new()?;
        print!("{prompt}");
        io::stdout().flush()?;
        let mut text = String::new();
        loop {
            let event = tokio::select! {
                update=inbox.next()=>{
                    if inbox.visible(&update) {
                        clear_input(prompt, &text)?;
                        // Normal output owns LF handling while raw input is
                        // briefly suspended. No async work or stdin read occurs.
                        terminal::disable_raw_mode()?;
                        let rendered = inbox.render(update, renderer);
                        terminal::enable_raw_mode()?;
                        print!("{prompt}{}", safe_text(&text));
                        io::stdout().flush()?;
                        rendered?;
                    } else { inbox.render(update, renderer)?; }
                    continue;
                },
                _=crate::signals::shutdown()?=>None,
                event=self.events.next()=>event.transpose()?,
            };
            let Some(event) = event else {
                self.state = InputState::Closed;
                print!("\r\n");
                io::stdout().flush()?;
                return Ok(InputResult::Exit);
            };
            let Event::Key(key) = event else {
                continue;
            };
            if key.kind == KeyEventKind::Release {
                continue;
            }
            let lf =
                key.code == KeyCode::Char('j') && key.modifiers.contains(KeyModifiers::CONTROL);
            if lf && self.swallow_lf {
                self.swallow_lf = false;
                continue;
            }
            self.swallow_lf = false;
            // Crossterm reports LF as Ctrl-J in raw mode; CR is Enter. Treat
            // both as line endings, while an empty second CRLF event stays put.
            let code = if key.code == KeyCode::Char('j')
                && key.modifiers.contains(KeyModifiers::CONTROL)
            {
                KeyCode::Enter
            } else {
                key.code
            };
            match code {
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.state = InputState::Closed;
                    print!("^C\r\n");
                    io::stdout().flush()?;
                    return Ok(InputResult::Exit);
                }
                KeyCode::Char('d')
                    if key.modifiers.contains(KeyModifiers::CONTROL) && text.is_empty() =>
                {
                    self.state = InputState::Closed;
                    print!("\r\n");
                    io::stdout().flush()?;
                    return Ok(InputResult::Exit);
                }
                // Empty Enter keeps the existing prompt active, including the
                // second Enter produced by CRLF. It never prints another You>.
                KeyCode::Enter if text.trim().is_empty() && !allow_empty => {
                    self.swallow_lf = !lf;
                    for _ in 0..unicode_width::UnicodeWidthStr::width(text.as_str()) {
                        print!("\x08 \x08");
                    }
                    text.clear();
                }
                KeyCode::Enter => {
                    self.swallow_lf = !lf;
                    self.state = InputState::Ready;
                    print!("\r\n");
                    io::stdout().flush()?;
                    return Ok(InputResult::Line(text.trim().to_owned()));
                }
                KeyCode::Backspace => {
                    if let Some(c) = text.pop() {
                        let width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
                        for _ in 0..width {
                            print!("\x08 \x08");
                        }
                    }
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    text.clear();
                    print!("\r\x1b[2K{prompt}");
                }
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                        && !c.is_control()
                        && text.len() + c.len_utf8() <= 8192 =>
                {
                    text.push(c);
                    print!("{}", safe_text(&c.to_string()));
                }
                _ => {}
            }
            io::stdout().flush()?;
        }
    }
}
async fn confirmation(
    client: &Client,
    mut turn: ActorSnapshot,
    input: &mut Input,
    timing: Option<Latency>,
    renderer: &mut Renderer<io::Stdout>,
    inbox: &mut NoticeInbox,
) -> Result<bool> {
    let mut timing = timing;
    loop {
        turn = wait_seen(client, turn, true, renderer, Some(inbox)).await?;
        if let Some(latency) = timing.take() {
            latency.report(&turn, renderer)?;
        }
        if turn.state != ActorTurnState::PendingConfirmation {
            return Ok(true);
        }
        let action = turn
            .actions
            .iter()
            .find(|a| a.state == ActionState::Proposed)
            .context("missing pending proposal")?;
        println!(
            "Approval executes only the exact Tool arguments above. Work submission does not mean worker completion."
        );
        let answer = match input
            .line("Confirm this exact action? [y/N] ", true, inbox, renderer)
            .await?
        {
            InputResult::Line(answer) => answer,
            InputResult::Exit => {
                println!("Proposal remains pending for explicit review.");
                return Ok(false);
            }
        };
        let approve = answer.eq_ignore_ascii_case("y");
        turn = client
            .post(
                &format!("v1/actor-turns/{}/actions/{}/decision", turn.id, action.id),
                Some(json!({"approve":approve})),
            )
            .await?;
        validate(&turn)?;
    }
}
#[derive(Deserialize)]
struct ConversationPage {
    turns: Vec<ActorSnapshot>,
    has_more: bool,
}
pub async fn run(client: Client, conversation: Option<ConversationId>, timing: bool) -> Result<()> {
    ensure!(
        io::stdin().is_terminal() && io::stdout().is_terminal(),
        "chat requires a normal terminal; use lain --json ask for scripts"
    );
    let diagnostics: Value = client.get("v1/diagnostics").await?;
    ensure!(
        diagnostics["actor"]["status"] == "configured",
        "Actor model unavailable: run lain doctor; configure Actor on laind (Codex login is separate)"
    );
    let mut input = Input::new();
    let mut conversation = match conversation {
        Some(id) => id,
        None => create(&client).await?,
    };
    println!(
        "Conversation: {conversation}\nLain> You're here.\n/help for commands; /exit keeps this conversation."
    );
    let page: ConversationPage = client
        .get(&format!("v1/conversations/{conversation}?recent=true"))
        .await?;
    let mut history = Renderer::new(io::stdout());
    let mut inbox = NoticeInbox::new(client.clone(), conversation);
    for turn in &page.turns {
        validate(turn)?;
        ensure!(
            turn.conversation_id == conversation,
            "conversation history identity mismatch"
        );
        if !matches!(
            turn.state,
            ActorTurnState::Running | ActorTurnState::PendingConfirmation
        ) {
            history.snapshot(turn)?;
        }
    }
    if let Some(turn) = page.turns.last()
        && matches!(
            turn.state,
            ActorTurnState::Running | ActorTurnState::PendingConfirmation
        )
        && !confirmation(
            &client,
            turn.clone(),
            &mut input,
            None,
            &mut history,
            &mut inbox,
        )
        .await?
    {
        inbox.flush().await;
        return Ok(());
    }
    if page.has_more {
        println!(
            "Older conversation page loaded; use /history or the conversation API cursor for all records."
        );
    }
    loop {
        let InputResult::Line(text) = input.line("You> ", false, &mut inbox, &mut history).await?
        else {
            inbox.flush().await;
            return Ok(());
        };
        match text.as_str() {
            "/exit" => {
                inbox.flush().await;
                return Ok(());
            }
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
                inbox.flush().await;
                inbox = NoticeInbox::new(client.clone(), conversation);
                history = Renderer::new(io::stdout());
                println!("Conversation: {conversation}");
            }
            value if unknown_command(value) => println!(
                "Unknown command; /help lists commands. For an ambiguous one-component path, write `path: /name` or add a trailing slash."
            ),
            _ => {
                let started = Instant::now();
                match submit(&client, conversation, RequestId::new(), text).await {
                    Ok(turn) => {
                        let latency = timing.then(|| Latency {
                            started,
                            submission_ms: millis(started.elapsed()),
                        });
                        match confirmation(
                            &client,
                            turn,
                            &mut input,
                            latency,
                            &mut history,
                            &mut inbox,
                        )
                        .await
                        {
                            Ok(false) => {
                                inbox.flush().await;
                                return Ok(());
                            }
                            Ok(true) => {}
                            Err(error) => eprintln!("{}", safe_text(&error.to_string())),
                        }
                    }
                    Err(error) => eprintln!("{}", safe_text(&error.to_string())),
                }
            }
        }
    }
}
fn emit_json_line(value: &impl serde::Serialize) -> Result<()> {
    let mut output = io::stdout().lock();
    serde_json::to_writer(&mut output, value)?;
    writeln!(output)?;
    output.flush()?;
    Ok(())
}
pub async fn work_watch(client: Client, session: Uuid, machine: bool) -> Result<()> {
    let (tx, mut rx) = mpsc::channel(32);
    let observer = tokio::spawn(client.observe(tx));
    let result = async {
        loop {
            tokio::select! {
                _=crate::signals::shutdown()?=>return Ok(()),
                update=rx.recv()=>match update {
                    Some(crate::api::Update::Event(event)) if event.work_session_id==session=>{
                        if machine {emit_json_line(&event)?;} else {println!("{}",crate::tui::event_text(&event));}
                    },
                    Some(crate::api::Update::Snapshot(snapshot))=>{
                        if let Some(work)=snapshot.sessions.into_iter().find(|s|s.id==session) {
                            if machine {emit_json_line(&work)?;} else {emit(&work,false)?;}
                        } else {bail!("work session not found");}
                    },
                    Some(crate::api::Update::Offline(message))=>eprintln!("{}",safe_text(&message)),
                    Some(_)=>{},None=>bail!("observer stopped"),
                }
            }
        }
    }.await;
    observer.abort();
    result
}

#[cfg(test)]
mod input_tests {
    use super::*;
    #[test]
    fn paths_and_plain_text_do_not_become_slash_commands() {
        for input in [
            "/Users/example/project",
            "/tmp/",
            "/",
            "/directory with spaces",
            "/project.txt",
            r"C:\Users\example\project",
            r"D:\directory with spaces\project",
            r"\\server\share\project",
            "你好",
            "检查 /some/path",
        ] {
            assert!(!unknown_command(input), "{input}");
        }
        for input in ["/conversation", "/hlep", "/unknown-command"] {
            assert!(unknown_command(input));
        }
        // Known commands are matched explicitly before the unknown-word guard.
        assert!(unknown_command("/help"));
    }
    #[test]
    fn input_redraw_accounts_for_wrapping_and_wide_characters() {
        assert_eq!(input_rows("You> ", "abc", 8), 0);
        assert_eq!(input_rows("You> ", "abcd", 8), 1);
        assert_eq!(input_rows("You> ", "你好", 8), 1);
        assert_eq!(input_rows("You> ", "a\u{301}b", 8), 0);
    }
}
