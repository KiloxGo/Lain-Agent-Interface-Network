//! Explicit owner input to an existing remote Worker. No model or automatic POST.
use super::*;
use lain_wire::{
    device_commands::CommandState, device_execution::ExecutionPhase,
    device_work::FollowupDeviceWork, devices::DeviceState,
};

#[derive(Clone)]
pub(super) struct Pending {
    pub device: DeviceId,
    pub parent: DeviceCommandId,
    pub input: FollowupDeviceWork,
    target: DeviceCommand,
}
pub(super) struct Preview {
    pub pending: Pending,
    pub text: String,
    pub scroll: u16,
    pub seen_to: usize,
    pub width: u16,
    pub page: u16,
    pub read: bool,
}
pub(super) struct Panel {
    pub target: DeviceCommand,
    device: DeviceSnapshot,
    pub input: Composer,
    pub preview: Option<Preview>,
}
pub(super) enum Failure {
    BeforePost(anyhow::Error),
    Uncertain(anyhow::Error),
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(u64::MAX, |d| d.as_millis().try_into().unwrap_or(u64::MAX))
}
fn idle(c: &DeviceCommand) -> bool {
    matches!(c.state, CommandState::Succeeded | CommandState::Failed)
        && c.continued_by.is_none()
        && !c.cancel_requested
        && c.interruption.is_none()
        && c.input.profile_id == "codex/local"
        && c.execution.as_ref().is_some_and(|e| {
            e.valid()
                && matches!(e.phase, ExecutionPhase::Completed | ExecutionPhase::Failed)
                && e.session_id.is_some()
                && e.turn_id.is_some()
                && e.pending_approvals.is_empty()
        })
}
fn ready(c: &DeviceCommand, d: &DeviceSnapshot) -> bool {
    idle(c)
        && d.device.registration.id == c.device_id
        && d.device.state == DeviceState::Active
        && !d.expired
        && d.device.registration.expires_at_ms > now()
        && d.device
            .registration
            .workspaces
            .iter()
            .any(|w| w.id == c.input.workspace_id)
}
fn base(conversation: ConversationId, p: &Pending) -> String {
    format!(
        "v1/conversations/{conversation}/device-work/{}/{}/followups",
        p.device, p.parent
    )
}
pub(super) fn close(app: &mut Model) {
    if let Some(panel) = app.followup.take() {
        app.worker_drafts
            .insert((panel.target.device_id, panel.target.id), panel.input);
        while app.worker_drafts.len() > 128 {
            if let Some(key) = app.worker_drafts.keys().copied().next() {
                app.worker_drafts.remove(&key);
            }
        }
    }
}
pub(super) fn opened(
    app: &mut Model,
    record: DeviceWorkRecord,
    device: DeviceSnapshot,
) -> Result<()> {
    ensure!(
        app.followup_pending.is_none(),
        "An earlier follow-up is unresolved; Ctrl-R inspects, /followup-retry explicitly retries the same input"
    );
    ensure!(
        record.origin.conversation_id == app.conversation && ready(&record.command, &device),
        "Follow-up requires the latest known completed/failed idle turn; running/unknown/cancelled or already-continued targets are unsupported"
    );
    close(app);
    let target = record.command;
    let input = app
        .worker_drafts
        .remove(&(target.device_id, target.id))
        .unwrap_or_default();
    app.review = None;
    app.remote_review = None;
    app.followup = Some(Panel {
        target,
        device,
        input,
        preview: None,
    });
    app.status =
        "Worker input is separate from Lain chat; Enter previews, Esc returns without sending"
            .into();
    Ok(())
}
pub(super) fn open(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
    device: DeviceId,
    parent: DeviceCommandId,
) -> Result<()> {
    ensure!(
        app.followup_pending.is_none(),
        "Resolve the earlier follow-up first; Ctrl-R reads, /followup-retry retries its exact request"
    );
    if app.surface != Surface::Task(device, parent) {
        app.navigate(Surface::Task(device, parent));
    }
    let generation = app.generation;
    let conversation = app.conversation;
    let c = client.clone();
    let out = live::read_sender(app, tx, jobs);
    app.status =
        "Reading the actual conversation-bound idle Worker target; no work submitted".into();
    live::spawn_read(app, jobs, async move {
        let record_path = format!("v1/conversations/{conversation}/device-work/{device}/{parent}");
        let device_path = format!("v1/devices/{device}");
        let result = tokio::try_join!(
            c.get::<DeviceWorkRecord>(&record_path),
            c.get::<DeviceSnapshot>(&device_path)
        );
        let _ = out
            .send(Complete::FollowupOpened(
                generation,
                device,
                parent,
                Box::new(result),
            ))
            .await;
    });
    Ok(())
}
pub(super) fn preview(app: &mut Model) -> Result<()> {
    let panel = app
        .followup
        .as_mut()
        .context("No selected remote Worker input")?;
    ensure!(
        ready(&panel.target, &panel.device),
        "Worker target is no longer eligible"
    );
    ensure!(
        !panel.input.text.trim().is_empty() && panel.input.text.len() <= 8192,
        "Worker follow-up must be 1..8192 UTF-8 bytes"
    );
    crate::local::actor_config::non_secret(&panel.input.text, &Default::default())?;
    let c = &panel.target;
    let input = FollowupDeviceWork {
        request_id: RequestId::new(),
        expected_device_revision: panel.device.device.revision,
        expected_parent_revision: c.revision,
        text: panel.input.text.clone(),
        expires_at_ms: now()
            .saturating_add(900000)
            .min(panel.device.device.registration.expires_at_ms),
    };
    let scope = json!({"device_id":c.device_id,"parent_command_id":c.id,"target_session_id":c.execution.as_ref().and_then(|e|e.session_id),
        "workspace_id":c.input.workspace_id,"profile_id":c.input.profile_id,"sandbox":c.input.sandbox,"request":input});
    let text = safe_text(&format!(
        "Remote Worker follow-up · Unchanged\nDirect owner input; no model rewriting, no new Worker session.\nOriginal and actual outbound task (JSON string):\n{}\n\nComplete target and request:\n{}",
        serde_json::to_string(&input.text)?,
        serde_json::to_string_pretty(&scope)?
    ));
    panel.preview = Some(Preview {
        pending: Pending {
            device: c.device_id,
            parent: c.id,
            input,
            target: c.clone(),
        },
        text,
        scroll: 0,
        seen_to: 0,
        width: 0,
        page: 1,
        read: false,
    });
    Ok(())
}
fn matches_receipt(p: &Pending, c: &DeviceCommand) -> bool {
    c.device_id == p.device
        && c.id != p.parent
        && c.input.request_id == p.input.request_id
        && c.input.task == p.input.text
        && c.input.expected_device_revision == p.input.expected_device_revision
        && c.input.expires_at_ms == p.input.expires_at_ms
        && c.input.workspace_id == p.target.input.workspace_id
        && c.input.profile_id == p.target.input.profile_id
        && c.input.sandbox == p.target.input.sandbox
        && c.continuation.as_ref().is_some_and(|f| {
            f.parent_command_id == p.parent
                && f.expected_parent_revision == p.input.expected_parent_revision
                && Some(f.session_id) == p.target.execution.as_ref().and_then(|e| e.session_id)
                && Some(f.parent_turn_id) == p.target.execution.as_ref().and_then(|e| e.turn_id)
                && f.root_command_id
                    == p.target
                        .continuation
                        .as_ref()
                        .map_or(p.parent, |f| f.root_command_id)
        })
}
pub(super) fn key(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
    key: KeyEvent,
) -> Result<()> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    if app.followup_pending.is_some() {
        if key.code == KeyCode::Esc {
            close(app);
        } else if key.code == KeyCode::Tab {
            close(app);
            app.back();
        }
        return Ok(());
    }
    if ctrl && key.code == KeyCode::Char('r') {
        let (device, parent) = app
            .followup
            .as_ref()
            .map(|p| (p.target.device_id, p.target.id))
            .context("No remote target to refresh")?;
        close(app);
        return open(client, app, tx, jobs, device, parent);
    }
    if key.code == KeyCode::Esc {
        if app.followup.as_ref().is_some_and(|p| p.preview.is_some()) {
            app.followup.as_mut().unwrap().preview = None;
        } else {
            close(app);
        }
        return Ok(());
    }
    if key.code == KeyCode::Tab {
        close(app);
        app.back();
        return Ok(());
    }
    let panel = app.followup.as_mut().unwrap();
    if let Some(r) = panel.preview.as_mut() {
        match key.code {
            KeyCode::PageDown => r.scroll = r.scroll.saturating_add(r.page),
            KeyCode::PageUp => r.scroll = r.scroll.saturating_sub(r.page),
            KeyCode::Char('y')
                if ctrl && r.read && !app.followup_busy && app.followup_pending.is_none() =>
            {
                let p = r.pending.clone();
                let c = client.clone();
                let out = live::effect_sender(app, tx, jobs);
                let conversation = app.conversation;
                app.followup_pending = Some(p.clone());
                app.followup_busy = true;
                app.status = "Checking current target before explicit follow-up submission".into();
                jobs.spawn(async move {
                    let record_path = format!(
                        "v1/conversations/{conversation}/device-work/{}/{}",
                        p.device, p.parent
                    );
                    let device_path = format!("v1/devices/{}", p.device);
                    let result = async {
                        let (record, d) = tokio::try_join!(
                            c.get::<DeviceWorkRecord>(&record_path),
                            c.get::<DeviceSnapshot>(&device_path)
                        )
                        .map_err(Failure::BeforePost)?;
                        if record.origin.conversation_id != conversation
                            || record.command.revision != p.input.expected_parent_revision
                            || record.command.input != p.target.input
                            || record.command.execution != p.target.execution
                            || !ready(&record.command, &d)
                            || d.device.revision != p.input.expected_device_revision
                            || d.device.registration.expires_at_ms < p.input.expires_at_ms
                        {
                            return Err(Failure::BeforePost(anyhow::anyhow!(
                                "Worker target changed; refresh and review again"
                            )));
                        }
                        c.post(
                            &base(conversation, &p),
                            Some(serde_json::to_value(&p.input).expect("typed request")),
                        )
                        .await
                        .map(Some)
                        .map_err(Failure::Uncertain)
                    }
                    .await;
                    let _ = out
                        .send(Complete::FollowupSent(Box::new((p, result))))
                        .await;
                });
            }
            _ => {}
        }
    } else {
        match key.code {
            KeyCode::Enter
                if key.modifiers.intersects(
                    KeyModifiers::SHIFT | KeyModifiers::ALT | KeyModifiers::CONTROL,
                ) =>
            {
                panel.input.insert("\n")
            }
            KeyCode::Enter => preview(app)?,
            KeyCode::Char('c') if ctrl => panel.input = Composer::default(),
            _ => panel.input.key(key),
        }
    }
    Ok(())
}
pub(super) fn recover(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
    retry: bool,
) {
    let Some(p) = app.followup_pending.clone().filter(|_| !app.followup_busy) else {
        return;
    };
    app.followup_busy = true;
    let c = client.clone();
    let out = live::effect_sender(app, tx, jobs);
    let conversation = app.conversation;
    jobs.spawn(async move {
        let result = async {
            let recorded: Option<DeviceCommand> = c
                .get(&base(conversation, &p))
                .await
                .map_err(Failure::Uncertain)?;
            if recorded.is_some() || !retry {
                return Ok(recorded);
            }
            // Explicit exact cloud request retry. The native host still owns its
            // original effect fences; this is never an automatic Worker replay.
            c.post(
                &base(conversation, &p),
                Some(serde_json::to_value(&p.input).expect("typed request")),
            )
            .await
            .map(Some)
            .map_err(Failure::Uncertain)
        }
        .await;
        let _ = out
            .send(Complete::FollowupSent(Box::new((p, result))))
            .await;
    });
}
pub(super) fn complete(
    app: &mut Model,
    p: Pending,
    result: std::result::Result<Option<DeviceCommand>, Failure>,
) -> Result<()> {
    if app
        .followup_pending
        .as_ref()
        .is_none_or(|old| old.input.request_id != p.input.request_id)
    {
        return Ok(());
    }
    app.followup_busy = false;
    match result {
        Ok(Some(mut c)) if matches_receipt(&p, &c) => {
            // A snapshot can arrive before the original POST response. Its
            // newer matching evidence must not regress to that historical ack.
            if let Some(latest) = app.remote_work.iter().find(|r| {
                r.origin.conversation_id == app.conversation
                    && r.command.id == c.id
                    && r.command.revision > c.revision
                    && matches_receipt(&p, &r.command)
            }) {
                c = latest.command.clone();
            }
            app.followup_pending = None;
            close(app);
            if app
                .worker_drafts
                .get(&(p.device, p.parent))
                .is_some_and(|draft| draft.text == p.input.text)
            {
                app.worker_drafts.remove(&(p.device, p.parent));
            }
            app.status = format!(
                "Follow-up queued as {}; same target session, execution not yet proven",
                c.id
            );
            if app.surface == Surface::Task(p.device, p.parent) {
                app.navigate(Surface::Task(p.device, c.id));
                app.task = Some(c);
            }
        }
        Ok(Some(c))
            if c.device_id == p.device
                && c.continuation
                    .as_ref()
                    .is_some_and(|f| f.parent_command_id == p.parent)
                && c.input.request_id != p.input.request_id =>
        {
            app.followup_pending = None;
            app.status = format!(
                "Another explicit follow-up {} occupies this parent; this request was not recorded",
                c.id
            );
        }
        Err(Failure::BeforePost(e)) => {
            app.followup_pending = None;
            app.status = format!("No new follow-up POST submitted: {e}");
        }
        Err(Failure::Uncertain(e)) => {
            app.status = format!(
                "Follow-up outcome uncertain: {e}; Ctrl-R reads; /followup-retry retries exact RequestId {}",
                p.input.request_id
            )
        }
        _ => {
            app.status = format!(
                "Follow-up receipt unconfirmed; Ctrl-R reads; /followup-retry retries exact RequestId {}",
                p.input.request_id
            )
        }
    }
    Ok(())
}
