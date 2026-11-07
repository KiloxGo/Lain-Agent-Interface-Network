//! Explicit cancellation of one inspected device command, never page lifetime.
use super::*;
use lain_wire::device_commands::{CancelDeviceCommand, CommandState};

#[derive(Clone)]
pub(super) struct Pending {
    pub target: DeviceCommand,
}
pub(super) struct Review {
    pub pending: Pending,
    pub text: String,
    pub scroll: u16,
    pub seen_to: usize,
    pub width: u16,
    pub page: u16,
    pub read: bool,
}
pub(super) enum Failure {
    BeforePost(anyhow::Error),
    Uncertain(anyhow::Error),
}

fn eligible(c: &DeviceCommand) -> bool {
    (!c.state.terminal() || c.state == CommandState::Unknown) && !c.cancel_requested
}
fn same(p: &Pending, c: &DeviceCommand) -> bool {
    c.id == p.target.id && c.device_id == p.target.device_id && c.input == p.target.input
}
pub(super) fn open(app: &mut Model) -> Result<()> {
    ensure!(
        app.cancel_pending.is_none(),
        "A cancellation outcome is uncertain; Ctrl-R inspects it before another request"
    );
    let c = app
        .task
        .as_ref()
        .context("Inspect the actual device task before F8 cancel")?;
    ensure!(
        app.surface == Surface::Task(c.device_id, c.id) && eligible(c),
        "Task is terminal or cancellation is already requested; refresh current facts"
    );
    let text = safe_text(&format!(
        "Request cancellation of this device task\n{}\n\nThis requests a stop; it does not undo previous effects.\nA cloud acknowledgement does not prove native termination.\nClosing this page or the frontend does not cancel work.",
        serde_json::to_string_pretty(&json!({"device_id":c.device_id,"command_id":c.id,
            "expected_revision":c.revision,"state":c.state,"workspace_id":c.input.workspace_id,
            "profile_id":c.input.profile_id,"sandbox":c.input.sandbox,"original_task":c.input.task,
            "execution_id":c.execution.as_ref().map(|e|e.execution_id),
            "session_id":c.execution.as_ref().and_then(|e|e.session_id),
            "turn_id":c.execution.as_ref().and_then(|e|e.turn_id)}))?
    ));
    app.review = None;
    app.remote_review = None;
    app.cancel_review = Some(Review {
        pending: Pending { target: c.clone() },
        text,
        scroll: 0,
        seen_to: 0,
        width: 0,
        page: 1,
        read: false,
    });
    Ok(())
}
pub(super) fn invalidate(app: &mut Model) {
    if app.cancel_review.as_ref().is_some_and(|r| {
        !app.task.as_ref().is_some_and(|c| {
            same(&r.pending, c) && c.revision == r.pending.target.revision && eligible(c)
        })
    }) {
        app.cancel_review = None;
        app.status =
            "Cancellation target changed; read the current task before requesting a stop".into();
    }
}
fn path(p: &Pending) -> String {
    format!("v1/devices/{}/commands/{}", p.target.device_id, p.target.id)
}
fn send(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
    p: Pending,
) {
    app.cancel_pending = Some(p.clone());
    app.cancel_busy = true;
    let c = client.clone();
    let out = live::effect_sender(app, tx, jobs);
    jobs.spawn(async move {
        let result = async {
            let fresh: DeviceCommand = c.get(&path(&p)).await.map_err(Failure::BeforePost)?;
            if !same(&p, &fresh) || fresh.revision != p.target.revision || !eligible(&fresh) {
                return Err(Failure::BeforePost(anyhow::anyhow!(
                    "Cancellation scope changed; no POST sent"
                )));
            }
            c.post(
                &format!("{}/cancel", path(&p)),
                Some(
                    serde_json::to_value(CancelDeviceCommand {
                        expected_revision: p.target.revision,
                    })
                    .expect("typed cancel"),
                ),
            )
            .await
            .map_err(Failure::Uncertain)
        }
        .await;
        let _ = out
            .send(Complete::RemoteCancelled(Box::new((p, result))))
            .await;
    });
}
pub(super) fn key(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
    key: KeyEvent,
) {
    match key.code {
        KeyCode::Esc => app.cancel_review = None,
        KeyCode::Tab => {
            app.cancel_review = None;
            app.back();
        }
        KeyCode::PageDown => {
            let r = app.cancel_review.as_mut().unwrap();
            r.scroll = r.scroll.saturating_add(r.page);
        }
        KeyCode::PageUp => {
            let r = app.cancel_review.as_mut().unwrap();
            r.scroll = r.scroll.saturating_sub(r.page);
        }
        KeyCode::Char('k')
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && !app.cancel_busy
                && app.cancel_pending.is_none() =>
        {
            invalidate(app);
            if let Some(r) = app.cancel_review.as_ref().filter(|r| r.read) {
                send(client, app, tx, jobs, r.pending.clone());
            } else {
                app.status = "Read the full current cancellation target first".into();
            }
        }
        _ => {}
    }
}
pub(super) fn recover(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
    retry: bool,
) {
    if app.cancel_busy {
        return;
    }
    let Some(p) = app.cancel_pending.clone() else {
        return;
    };
    // Retrying retains the original reviewed revision; never adopt a new scope.
    if retry {
        send(client, app, tx, jobs, p);
        return;
    }
    app.cancel_busy = true;
    let c = client.clone();
    let out = live::effect_sender(app, tx, jobs);
    jobs.spawn(async move {
        let result = c.get(&path(&p)).await.map_err(Failure::Uncertain);
        let _ = out
            .send(Complete::RemoteCancelled(Box::new((p, result))))
            .await;
    });
}
pub(super) fn complete(
    app: &mut Model,
    p: Pending,
    result: std::result::Result<DeviceCommand, Failure>,
) -> Result<()> {
    if !app
        .cancel_pending
        .as_ref()
        .is_some_and(|old| same(old, &p.target) && old.target.revision == p.target.revision)
    {
        return Ok(());
    }
    app.cancel_busy = false;
    app.cancel_review = None;
    match result {
        Ok(c) => {
            ensure!(
                same(&p, &c) && c.revision >= p.target.revision,
                "Cancellation response target changed"
            );
            app.cancel_notice = c.cancel_requested.then(|| p.clone());
            app.status = if c.state == CommandState::Cancelled {
                if c.execution.as_ref().is_some_and(|e| {
                    e.valid() && e.phase == lain_wire::device_execution::ExecutionPhase::Cancelled
                }) {
                    "Device reports cancellation; previous effects are not undone".into()
                } else if p.target.state == CommandState::Queued && c.execution.is_none() {
                    "Queued work cancelled in cloud; no device execution reported".into()
                } else {
                    "Cloud state Cancelled; native termination unconfirmed".into()
                }
            } else if c.state.terminal() && c.state != CommandState::Unknown {
                "Task completed or ended before cancellation was recorded; inspect its actual result".into()
            } else if c.cancel_requested {
                "Cancellation requested in cloud; native termination unconfirmed (this fact does not identify the requesting client)".into()
            } else if c.revision > p.target.revision {
                "Task changed without a cancellation record; the old revision cannot authorize a later stop. Review again if needed".into()
            } else {
                "Cancellation outcome uncertain; Ctrl-R inspects, /cancel-retry explicitly retries the original revision".into()
            };
            if c.cancel_requested
                || c.revision > p.target.revision
                || (c.state.terminal() && c.state != CommandState::Unknown)
            {
                app.cancel_pending = None;
            }
            if app.surface == Surface::Task(c.device_id, c.id)
                && app
                    .task
                    .as_ref()
                    .is_none_or(|old| old.revision <= c.revision)
            {
                app.task = Some(c);
            }
            // The acknowledgement may arrive after a newer SSE/detail snapshot.
            // Use only this exact command's newest already-known facts.
            let latest = app
                .task
                .iter()
                .chain(app.remote_work.iter().map(|r| &r.command))
                .filter(|c| same(&p, c))
                .max_by_key(|c| c.revision)
                .cloned();
            if let Some(latest) = latest {
                observed(app, &latest);
            }
        }
        Err(Failure::BeforePost(e)) => {
            app.cancel_pending = None;
            app.status = format!("No cancellation submitted: {e}");
        }
        Err(Failure::Uncertain(_e)) => {
            app.status="Cancellation outcome uncertain; Ctrl-R reads current facts. No automatic retry or Worker replay".into();
        }
    }
    Ok(())
}

pub(super) fn observed(app: &mut Model, c: &DeviceCommand) {
    let Some(notice) = app.cancel_notice.as_ref() else {
        return;
    };
    if !same(notice, c)
        || c.revision <= notice.target.revision
        || !c.state.terminal()
        || c.state == CommandState::Unknown
    {
        return;
    }
    app.status = match c.state {
        CommandState::Cancelled
            if c.execution.as_ref().is_some_and(|e| {
                e.valid() && e.phase == lain_wire::device_execution::ExecutionPhase::Cancelled
            }) =>
        {
            format!(
                "Device reports cancellation for task {}; previous effects are not undone",
                c.id
            )
        }
        CommandState::Cancelled
            if notice.target.state == CommandState::Queued && c.execution.is_none() =>
        {
            format!(
                "Queued task {} cancelled in cloud; no device execution reported",
                c.id
            )
        }
        CommandState::Cancelled => format!(
            "Cloud reports task {} cancelled; native termination unconfirmed",
            c.id
        ),
        CommandState::Succeeded => format!(
            "Task {} completed despite cancellation request; inspect its actual result",
            c.id
        ),
        CommandState::Failed => format!(
            "Task {} failed after cancellation request; inspect its actual result",
            c.id
        ),
        state => format!(
            "Task {} is {state:?}; native termination is not established",
            c.id
        ),
    };
    app.cancel_notice = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> Model {
        let mut app = Model::new(ConversationId::new());
        let mut c = super::super::tests::remote_record(app.conversation).command;
        c.state = CommandState::Running;
        app.surface = Surface::Task(c.device_id, c.id);
        app.task = Some(c);
        app.input.insert("Lain draft stays here");
        app
    }
    #[test]
    fn cancellation_scope_is_explicit_current_and_navigation_has_no_effect() {
        let mut app = app();
        open(&mut app).unwrap();
        let review = app.cancel_review.as_ref().unwrap();
        assert!(
            review
                .text
                .contains(&app.task.as_ref().unwrap().id.to_string())
        );
        assert!(!review.read);
        assert!(app.cancel_pending.is_none());
        app.navigate(Surface::Chat);
        assert!(app.cancel_review.is_none());
        assert!(app.cancel_pending.is_none());
        assert_eq!(app.input.text, "Lain draft stays here");
    }
    #[test]
    fn loading_task_cannot_offer_cancel_and_fresh_read_can_be_reviewed() {
        let mut app = app();
        let target = app.task.take();
        assert!(open(&mut app).is_err());
        assert!(app.cancel_review.is_none() && app.cancel_pending.is_none());
        app.task = target;
        open(&mut app).unwrap();
        assert!(!app.cancel_review.as_ref().unwrap().read);
    }
    #[test]
    fn changed_or_terminal_task_requires_new_review() {
        let mut app = app();
        open(&mut app).unwrap();
        app.task.as_mut().unwrap().revision += 1;
        invalidate(&mut app);
        assert!(app.cancel_review.is_none());
        app.task.as_mut().unwrap().state = CommandState::Succeeded;
        assert!(open(&mut app).is_err());
        app.task.as_mut().unwrap().state = CommandState::Unknown;
        open(&mut app).unwrap();
        app.task.as_mut().unwrap().cancel_requested = true;
        invalidate(&mut app);
        assert!(app.cancel_review.is_none());
        assert!(open(&mut app).is_err());
    }
    #[tokio::test]
    async fn unread_or_stale_review_never_starts_a_cancel_request() {
        let mut app = app();
        open(&mut app).unwrap();
        let client =
            Client::new("http://127.0.0.1:1", "fixture-cancel-test-only-credential").unwrap();
        let (tx, _) = mpsc::channel(2);
        let mut jobs = JoinSet::new();
        key(
            &client,
            &mut app,
            &tx,
            &mut jobs,
            KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL),
        );
        assert!(jobs.is_empty() && app.cancel_pending.is_none());
        app.cancel_review.as_mut().unwrap().read = true;
        app.task.as_mut().unwrap().revision += 1;
        key(
            &client,
            &mut app,
            &tx,
            &mut jobs,
            KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL),
        );
        assert!(jobs.is_empty() && app.cancel_pending.is_none() && app.cancel_review.is_none());
    }
    #[test]
    fn recovery_distinguishes_recorded_request_termination_and_another_client() {
        let mut app = app();
        let p = Pending {
            target: app.task.clone().unwrap(),
        };
        app.cancel_pending = Some(p.clone());
        complete(&mut app, p.clone(), Ok(p.target.clone())).unwrap();
        assert!(
            app.cancel_pending.is_some(),
            "unchanged GET cannot prove an earlier POST failed"
        );
        assert!(app.status.contains("uncertain"));
        let mut recorded = p.target.clone();
        recorded.revision += 1;
        recorded.cancel_requested = true;
        complete(&mut app, p.clone(), Ok(recorded.clone())).unwrap();
        assert!(app.cancel_pending.is_none());
        assert!(app.status.contains("requested") && app.status.contains("unconfirmed"));
        // A concurrent client's receipt proves only the current task fact.
        assert!(!app.status.contains("your request"));
        app.cancel_pending = Some(p.clone());
        recorded.state = CommandState::Cancelled;
        complete(&mut app, p.clone(), Ok(recorded)).unwrap();
        assert!(app.status.contains("native termination unconfirmed"));
        app.cancel_pending = Some(p.clone());
        let mut completed = p.target.clone();
        completed.revision += 1;
        completed.state = CommandState::Succeeded;
        complete(&mut app, p, Ok(completed)).unwrap();
        assert!(app.cancel_pending.is_none());
        assert!(app.status.contains("completed"));
    }

    #[tokio::test]
    async fn later_device_cancellation_updates_the_cloud_only_status_without_replay() {
        use lain_wire::device_execution::*;
        let mut app = app();
        let p = Pending {
            target: app.task.clone().unwrap(),
        };
        app.cancel_pending = Some(p.clone());
        let mut requested = p.target.clone();
        requested.cancel_requested = true;
        requested.revision += 1;
        complete(&mut app, p, Ok(requested.clone())).unwrap();
        assert!(app.status.contains("native termination unconfirmed"));
        requested.revision += 1;
        requested.state = CommandState::Cancelled;
        requested.execution = Some(ExecutionEvidence {
            execution_id: lain_wire::DeviceExecutionId::new(),
            session_id: Some(lain_wire::WorkSessionId::new()),
            turn_id: Some(lain_wire::WorkerTurnId::new()),
            phase: ExecutionPhase::Cancelled,
            pending_approvals: vec![],
            approval_reports: vec![],
            result: None,
        });
        let mut record = super::super::tests::remote_record(app.conversation);
        record.command = requested;
        let client =
            Client::new("http://127.0.0.1:1", "fixture-status-test-only-credential").unwrap();
        let (tx, _) = mpsc::channel(8);
        let mut jobs = JoinSet::new();
        handle_complete(
            &client,
            &mut app,
            &tx,
            &mut jobs,
            Complete::RemoteWork(Ok(DeviceWorkPage {
                work: vec![record],
                truncated: false,
            })),
        )
        .unwrap();
        assert!(
            app.status.contains("Device reports cancellation"),
            "stale cloud-only status: {}",
            app.status
        );
        assert!(jobs.is_empty() && app.cancel_pending.is_none());
        assert_eq!(app.input.text, "Lain draft stays here");
    }

    #[test]
    fn late_cancel_ack_does_not_hide_an_already_known_terminal_fact() {
        let mut app = app();
        let p = Pending {
            target: app.task.clone().unwrap(),
        };
        let mut ack = p.target.clone();
        ack.revision += 1;
        ack.cancel_requested = true;
        let mut ended = ack.clone();
        ended.revision += 1;
        ended.state = CommandState::Succeeded;
        app.task = Some(ended);
        app.cancel_pending = Some(p.clone());
        complete(&mut app, p, Ok(ack)).unwrap();
        assert!(
            app.status.contains("completed"),
            "late acknowledgement hid completion: {}",
            app.status
        );
        assert!(!app.status.contains("unconfirmed"));
        assert_eq!(app.task.as_ref().unwrap().state, CommandState::Succeeded);
    }
}
