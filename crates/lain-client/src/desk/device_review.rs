use super::*;
use lain_wire::{WorkerApprovalId, device_commands::CommandState, device_execution::*};

#[derive(Clone)]
pub(super) struct Pending {
    pub device: DeviceId,
    pub command: DeviceCommandId,
    pub approval: WorkerApprovalId,
    pub input: DecideDeviceApproval,
}
#[derive(Clone)]
pub(super) struct Review {
    command: DeviceCommand,
    approval: DeviceApproval,
    pub text: String,
    pub scroll: u16,
    pub seen_to: usize,
    pub width: u16,
    pub page: u16,
    pub read: bool,
}
impl Review {
    pub(super) fn choices_hint(&self) -> String {
        self.approval
            .choices
            .iter()
            .map(|c| match c {
                DeviceApprovalChoice::ApproveOnce => "Ctrl-Y allow once",
                DeviceApprovalChoice::Deny => "Ctrl-D deny",
                DeviceApprovalChoice::Cancel => "Ctrl-K cancel",
            })
            .collect::<Vec<_>>()
            .join(" · ")
    }
    fn current(&self, c: &DeviceCommand) -> bool {
        c.id == self.command.id
            && c.device_id == self.command.device_id
            && c.revision == self.command.revision
            && c.input == self.command.input
            && c.state == CommandState::Running
            && !c.cancel_requested
            && c.interruption.is_none()
            && c.input.expires_at_ms > now()
            && c.execution
                .as_ref()
                .is_some_and(|e| e.valid() && Some(e) == self.command.execution.as_ref())
    }
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(u64::MAX, |d| {
            u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
        })
}
pub(super) fn open(app: &mut Model) -> Result<()> {
    ensure!(
        app.remote_pending.is_none(),
        "A decision outcome is unresolved; Ctrl-R reads its receipt, /decision-retry retries the exact request"
    );
    let c = app
        .task
        .as_ref()
        .context("Select the actual device task first")?;
    let decisions = app
        .task_decisions
        .as_ref()
        .context("Refresh task and decision receipts before reviewing")?;
    let e = c
        .execution
        .as_ref()
        .context("No native execution evidence")?;
    ensure!(
        e.valid() && e.phase == ExecutionPhase::WaitingForApproval,
        "No current valid remote permission request"
    );
    let a = e
        .pending_approvals
        .first()
        .context("No pending remote permission")?;
    ensure!(
        !decisions.iter().any(|d| d.approval_id == a.id),
        "A cloud decision is already recorded; wait for device evidence, do not decide again"
    );
    let text = safe_text(&serde_json::to_string_pretty(
        &json!({"device_id":c.device_id,"command_id":c.id,
        "command_revision":c.revision,"execution_id":e.execution_id,"local_session_id":e.session_id,
        "local_turn_id":e.turn_id,"workspace_id":c.input.workspace_id,"profile_id":c.input.profile_id,
        "sandbox":c.input.sandbox,"permission":a}),
    )?);
    let review = Review {
        command: c.clone(),
        approval: a.clone(),
        text,
        scroll: 0,
        seen_to: 0,
        width: 0,
        page: 1,
        read: false,
    };
    ensure!(
        review.current(c),
        "Remote work expired, changed or cancelled; refresh current facts"
    );
    app.review = None;
    app.remote_review = Some(review);
    Ok(())
}
pub(super) fn invalidate(app: &mut Model) {
    if app.remote_review.as_ref().is_some_and(|r| {
        !app.task.as_ref().is_some_and(|c| r.current(c))
            || app
                .task_decisions
                .as_ref()
                .is_some_and(|v| v.iter().any(|d| d.approval_id == r.approval.id))
    }) {
        app.remote_review = None;
        app.status =
            "Remote permission scope changed; inspect the current task before deciding".into();
    }
}
#[derive(Debug)]
struct BeforePost;
impl std::fmt::Display for BeforePost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "fresh scope unavailable or changed before decision submission"
        )
    }
}
impl std::error::Error for BeforePost {}
pub(super) fn key(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
    key: KeyEvent,
) {
    let Some(r) = app.remote_review.as_mut() else {
        return;
    };
    match key.code {
        KeyCode::Esc => app.remote_review = None,
        KeyCode::PageDown => r.scroll = r.scroll.saturating_add(r.page),
        KeyCode::PageUp => r.scroll = r.scroll.saturating_sub(r.page),
        KeyCode::Char('y' | 'd' | 'k')
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && !app.remote_deciding
                && app.remote_pending.is_none() =>
        {
            let choice = match key.code {
                KeyCode::Char('y') => DeviceApprovalChoice::ApproveOnce,
                KeyCode::Char('d') => DeviceApprovalChoice::Deny,
                _ => DeviceApprovalChoice::Cancel,
            };
            if !r.read
                || !app.task.as_ref().is_some_and(|c| r.current(c))
                || !r.approval.choices.contains(&choice)
                || (choice == DeviceApprovalChoice::ApproveOnce && r.approval.truncated)
            {
                app.status =
                    "Read the full current scope; only offered choices are available".into();
                return;
            }
            let pending = Pending {
                device: r.command.device_id,
                command: r.command.id,
                approval: r.approval.id,
                input: DecideDeviceApproval {
                    request_id: RequestId::new(),
                    fingerprint: r.approval.fingerprint.clone(),
                    choice,
                },
            };
            let review = r.clone();
            let c = client.clone();
            let out = live::effect_sender(app, tx, jobs);
            let p = pending.clone();
            app.remote_pending = Some(pending);
            app.remote_deciding = true;
            jobs.spawn(async move {
                let result = async {
                    let current: DeviceCommand = c
                        .get(&format!("v1/devices/{}/commands/{}", p.device, p.command))
                        .await
                        .context(BeforePost)?;
                    ensure!(review.current(&current), BeforePost);
                    post(&c, &p).await.map(Some)
                }
                .await;
                let _ = out.send(Complete::RemoteDecision(p, result)).await;
            });
        }
        _ => {}
    }
}
async fn post(client: &Client, p: &Pending) -> Result<DeviceApprovalDecision> {
    client
        .post(
            &format!(
                "v1/devices/{}/commands/{}/approvals/{}",
                p.device, p.command, p.approval
            ),
            Some(serde_json::to_value(&p.input)?),
        )
        .await
}
pub(super) fn recover(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
    retry: bool,
) {
    if app.remote_deciding {
        return;
    }
    let Some(p) = app.remote_pending.clone() else {
        return;
    };
    app.remote_deciding = true;
    let c = client.clone();
    let out = live::effect_sender(app, tx, jobs);
    jobs.spawn(async move {
        let result = if retry {
            post(&c, &p).await.map(Some)
        } else {
            async {
                let decisions: Vec<DeviceApprovalDecision> = c
                    .get(&format!(
                        "v1/devices/{}/commands/{}/approvals",
                        p.device, p.command
                    ))
                    .await?;
                Ok(decisions.into_iter().find(|r| r.approval_id == p.approval))
            }
            .await
        };
        let _ = out.send(Complete::RemoteDecision(p, result)).await;
    });
}
pub(super) fn complete(
    app: &mut Model,
    pending: Pending,
    result: Result<Option<DeviceApprovalDecision>>,
) -> Result<()> {
    if !app.remote_pending.as_ref().is_some_and(|p| {
        p.input == pending.input
            && p.device == pending.device
            && p.command == pending.command
            && p.approval == pending.approval
    }) {
        return Ok(());
    }
    app.remote_deciding = false;
    app.remote_review = None;
    match result {
        Ok(Some(receipt)) => {
            ensure!(
                receipt.approval_id == pending.approval
                    && receipt.decision.fingerprint == pending.input.fingerprint,
                "decision receipt identity changed"
            );
            app.status = if receipt.decision == pending.input {
                "Decision recorded in cloud; device receipt and Worker application are not proven by this receipt".into()
            } else {
                "Another explicit decision is recorded; this request was not accepted; Worker application is not proven".into()
            };
            app.remote_pending = None;
            if app.surface == Surface::Task(pending.device, pending.command) {
                app.task_decisions = None;
            }
        }
        Err(e) if e.is::<BeforePost>() => {
            app.remote_pending = None;
            app.status =
                "Scope changed or unavailable; no decision submitted. Refresh current task.".into();
        }
        _ => {
            app.status = format!(
                "Decision {} outcome uncertain. Ctrl-R reads receipts; /decision-retry explicitly retries the same request. No automatic POST replay.",
                pending.input.request_id
            )
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desk::tests::remote_record;
    use lain_wire::{
        DeviceExecutionId, WorkSessionId, WorkerApprovalId, WorkerTurnId,
        device_commands::CommandState,
    };

    fn approval(conversation: ConversationId, truncated: bool) -> DeviceCommand {
        let mut c = remote_record(conversation).command;
        c.state = CommandState::Running;
        let mut e = ExecutionEvidence {
            execution_id: DeviceExecutionId::new(),
            session_id: Some(WorkSessionId::new()),
            turn_id: Some(WorkerTurnId::new()),
            phase: ExecutionPhase::WaitingForApproval,
            approval_reports: vec![],
            result: None,
            pending_approvals: vec![DeviceApproval {
                id: WorkerApprovalId::new(),
                kind: "command".into(),
                description: "command: read fixture\nworkspace: selected fixture".into(),
                truncated,
                choices: if truncated {
                    vec![DeviceApprovalChoice::Deny]
                } else {
                    vec![
                        DeviceApprovalChoice::ApproveOnce,
                        DeviceApprovalChoice::Deny,
                    ]
                },
                fingerprint: String::new(),
            }],
        };
        e.pending_approvals[0].fingerprint = approval_fingerprint(
            e.execution_id,
            e.session_id,
            e.turn_id,
            &e.pending_approvals[0],
        );
        c.execution = Some(e);
        c
    }

    #[tokio::test]
    async fn stale_or_unread_remote_scope_cannot_create_a_decision() {
        let mut app = Model::new(ConversationId::new());
        let c = approval(app.conversation, false);
        app.surface = Surface::Task(c.device_id, c.id);
        app.task = Some(c.clone());
        app.task_decisions = Some(vec![]);
        open(&mut app).unwrap();
        let client =
            Client::new("http://127.0.0.1:1", "fixture-test-only-credential-123456").unwrap();
        let (tx, _) = mpsc::channel(8);
        let mut jobs = JoinSet::new();
        key(
            &client,
            &mut app,
            &tx,
            &mut jobs,
            KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
        );
        assert!(app.remote_pending.is_none());
        app.remote_review.as_mut().unwrap().read = true;
        app.task.as_mut().unwrap().revision += 1;
        key(
            &client,
            &mut app,
            &tx,
            &mut jobs,
            KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
        );
        assert!(app.remote_pending.is_none() && jobs.is_empty());
    }

    #[tokio::test]
    async fn truncated_scope_never_offers_approve_and_pending_identity_survives_unknown_result() {
        let mut app = Model::new(ConversationId::new());
        let c = approval(app.conversation, true);
        app.surface = Surface::Task(c.device_id, c.id);
        app.task = Some(c);
        app.task_decisions = Some(vec![]);
        open(&mut app).unwrap();
        app.remote_review.as_mut().unwrap().read = true;
        assert!(
            !app.remote_review
                .as_ref()
                .unwrap()
                .choices_hint()
                .contains("Ctrl-Y")
        );
        let client =
            Client::new("http://127.0.0.1:1", "fixture-test-only-credential-123456").unwrap();
        let (tx, _) = mpsc::channel(8);
        let mut jobs = JoinSet::new();
        key(
            &client,
            &mut app,
            &tx,
            &mut jobs,
            KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
        );
        assert!(app.remote_pending.is_none());
        key(
            &client,
            &mut app,
            &tx,
            &mut jobs,
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
        );
        let pending = app.remote_pending.clone().unwrap();
        assert_eq!(pending.input.choice, DeviceApprovalChoice::Deny);
        complete(
            &mut app,
            pending.clone(),
            Err(anyhow::anyhow!("fixture lost response")),
        )
        .unwrap();
        assert_eq!(app.remote_pending.as_ref().unwrap().input, pending.input);
        let receipt = DeviceApprovalDecision {
            approval_id: pending.approval,
            decision: pending.input.clone(),
            received_at_ms: 1,
        };
        complete(&mut app, pending, Ok(Some(receipt))).unwrap();
        assert!(app.remote_pending.is_none());
        assert!(app.status.contains("recorded") && app.status.contains("not proven"));
    }

    #[test]
    fn remote_scope_review_requires_contiguous_pages_even_after_resize_or_jump() {
        use ratatui::{Terminal, backend::TestBackend};
        let mut app = Model::new(ConversationId::new());
        let mut c = approval(app.conversation, false);
        let e = c.execution.as_mut().unwrap();
        e.pending_approvals[0].description = "scope ".repeat(333);
        e.pending_approvals[0].fingerprint = approval_fingerprint(
            e.execution_id,
            e.session_id,
            e.turn_id,
            &e.pending_approvals[0],
        );
        app.task = Some(c);
        app.task_decisions = Some(vec![]);
        open(&mut app).unwrap();
        for (width, height) in [(40, 14), (80, 24), (120, 40)] {
            app.remote_review.as_mut().unwrap().scroll = u16::MAX;
            Terminal::new(TestBackend::new(width, height))
                .unwrap()
                .draw(|f| draw::render(f, &mut app))
                .unwrap();
            assert!(
                !app.remote_review.as_ref().unwrap().read,
                "jumping cannot skip unshown scope"
            );
        }
        let r = app.remote_review.as_mut().unwrap();
        r.scroll = 0;
        r.seen_to = 0;
        let mut terminal = Terminal::new(TestBackend::new(40, 14)).unwrap();
        for _ in 0..1000 {
            terminal.draw(|f| draw::render(f, &mut app)).unwrap();
            let r = app.remote_review.as_mut().unwrap();
            if r.read {
                break;
            }
            r.scroll = r.scroll.saturating_add(r.page);
        }
        assert!(app.remote_review.as_ref().unwrap().read);
    }
}
