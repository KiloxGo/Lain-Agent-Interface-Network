use super::*;
use lain_wire::device_execution::*;
use lain_wire::{DeviceCommandId, device_commands::*};

#[derive(Subcommand)]
pub enum Commands {
    /// Send exact follow-up to the same known idle remote Worker session.
    Continue {
        id: DeviceId,
        command_id: DeviceCommandId,
        #[arg(long)]
        conversation: lain_wire::ConversationId,
        #[arg(long)]
        file: PathBuf,
    },
    /// Read the single recorded child; null is absence, not proof a POST failed.
    Continuation {
        id: DeviceId,
        command_id: DeviceCommandId,
        #[arg(long)]
        conversation: lain_wire::ConversationId,
    },
    /// Queue exact work for an explicitly configured native device service.
    Queue {
        id: DeviceId,
        #[arg(long)]
        file: PathBuf,
        /// Link the task to this existing Lain conversation for factual updates.
        #[arg(long)]
        conversation: Option<lain_wire::ConversationId>,
    },
    Show {
        id: DeviceId,
        command_id: DeviceCommandId,
    },
    /// Explicit decision bound to the displayed pending approval fingerprint.
    Decide {
        id: DeviceId,
        command_id: DeviceCommandId,
        approval_id: lain_wire::WorkerApprovalId,
        #[arg(long)]
        file: PathBuf,
    },
    Decisions {
        id: DeviceId,
        command_id: DeviceCommandId,
    },
    /// Read this device's receipts; does not apply decisions to any Worker.
    FetchDecisions {
        #[arg(long)]
        directory: PathBuf,
        command_id: DeviceCommandId,
    },
    Cancel {
        id: DeviceId,
        command_id: DeviceCommandId,
        #[arg(long)]
        revision: u64,
    },
    Changes {
        id: DeviceId,
        #[arg(long, default_value_t = 0)]
        after: u64,
        #[arg(long)]
        through: Option<u64>,
    },
    /// One bounded device-authenticated read; never execute from a change notice.
    Fetch {
        #[arg(long)]
        directory: PathBuf,
        #[arg(long, default_value_t = 0)]
        after: u64,
        #[arg(long)]
        through: Option<u64>,
        #[arg(long, default_value_t = 0)]
        wait_ms: u32,
    },
    Inspect {
        #[arg(long)]
        directory: PathBuf,
        command_id: DeviceCommandId,
    },
    /// Journal an immutable event locally before posting its durable receipt.
    Event {
        #[arg(long)]
        directory: PathBuf,
        command_id: DeviceCommandId,
        #[arg(long)]
        file: PathBuf,
    },
    Receipt {
        #[arg(long)]
        directory: PathBuf,
        command_id: DeviceCommandId,
        request_id: RequestId,
    },
}
impl Commands {
    pub fn local(&self) -> bool {
        matches!(
            self,
            Self::Fetch { .. }
                | Self::Inspect { .. }
                | Self::Event { .. }
                | Self::Receipt { .. }
                | Self::FetchDecisions { .. }
        )
    }
}
fn query(after: u64, through: Option<u64>, wait_ms: u32) -> Result<String> {
    ensure!(wait_ms <= 25000, "wait-ms must be at most 25000");
    Ok(format!(
        "changes?after={after}&limit=32&wait_ms={wait_ms}{}",
        through.map_or_else(String::new, |n| format!("&through={n}"))
    ))
}
pub(super) async fn run(client: &Client, command: Commands, machine: bool) -> Result<()> {
    let value: Value = match command {
        Commands::Decide {
            id,
            command_id,
            approval_id,
            file,
        } => {
            let input: DecideDeviceApproval = read(&file)?;
            client
                .post(
                    &format!("v1/devices/{id}/commands/{command_id}/approvals/{approval_id}"),
                    Some(serde_json::to_value(input)?),
                )
                .await?
        }
        Commands::Decisions { id, command_id } => {
            client
                .get(&format!("v1/devices/{id}/commands/{command_id}/approvals"))
                .await?
        }
        Commands::Continue {
            id,
            command_id,
            conversation,
            file,
        } => {
            let input: lain_wire::device_work::FollowupDeviceWork = read(&file)?;
            client
                .post(
                    &format!(
                        "v1/conversations/{conversation}/device-work/{id}/{command_id}/followups"
                    ),
                    Some(serde_json::to_value(input)?),
                )
                .await?
        }
        Commands::Continuation {
            id,
            command_id,
            conversation,
        } => {
            client
                .get(&format!(
                    "v1/conversations/{conversation}/device-work/{id}/{command_id}/followups"
                ))
                .await?
        }
        Commands::Queue {
            id,
            file,
            conversation,
        } => {
            let input: QueueDeviceWork = read(&file)?;
            let path = conversation.map_or_else(
                || format!("v1/devices/{id}/commands"),
                |c| format!("v1/conversations/{c}/device-work/{id}"),
            );
            client
                .post(&path, Some(serde_json::to_value(input)?))
                .await?
        }
        Commands::Show { id, command_id } => {
            client
                .get(&format!("v1/devices/{id}/commands/{command_id}"))
                .await?
        }
        Commands::Cancel {
            id,
            command_id,
            revision,
        } => {
            client
                .post(
                    &format!("v1/devices/{id}/commands/{command_id}/cancel"),
                    Some(json!({"expected_revision":revision})),
                )
                .await?
        }
        Commands::Changes { id, after, through } => {
            client
                .get(&format!(
                    "v1/devices/{id}/commands/{}",
                    query(after, through, 0)?
                ))
                .await?
        }
        _ => unreachable!("device commands use only their own credential"),
    };
    emit(&value, machine)
}
fn outbox(directory: &Path) -> Result<PathBuf> {
    let p = directory.join("command-events");
    if !p.exists() {
        match std::fs::create_dir(&p) {
            Ok(()) => local::private(&p, true, true)?,
            Err(_) if p.exists() => {}
            Err(e) => return Err(e.into()),
        }
    }
    local::private(&p, true, false)?;
    Ok(p)
}
fn journal(
    directory: &Path,
    cid: DeviceCommandId,
    input: &ReportCommandEvent,
) -> Result<(PathBuf, PathBuf)> {
    ensure!(
        input.sequence > 0 && input.sequence <= 257 && input.detail.len() <= 4096,
        "invalid event bounds"
    );
    ensure!(
        input
            .execution
            .as_ref()
            .is_none_or(ExecutionEvidence::valid),
        "invalid execution evidence"
    );
    local::actor_config::non_secret(&serde_json::to_string(input)?, &Default::default())?;
    let p = outbox(directory)?;
    let lock = p.join("journal.lock");
    if !lock.exists() {
        match local::create_private(&lock, b"") {
            Ok(()) => {}
            Err(_) if lock.exists() => {}
            Err(e) => return Err(e),
        }
    }
    local::private(&lock, false, false)?;
    let guard = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(lock)?;
    guard
        .try_lock()
        .context("another device event is being journaled")?;
    let selected = p.join(format!("{cid}-{}.request.json", input.sequence));
    let receipt = p.join(format!("{cid}-{}.receipt.json", input.sequence));
    if input.sequence > 1 {
        let previous = p.join(format!("{cid}-{}.receipt.json", input.sequence - 1));
        local::private(&previous, false, false)
            .context("inspect or resend the preceding immutable event before advancing")?;
    }
    if selected.exists() {
        local::private(&selected, false, false)?;
        let old: ReportCommandEvent = read(&selected)?;
        ensure!(
            old == *input,
            "event sequence already journaled with different identity/content"
        );
    } else {
        local::create_private(&selected, &serde_json::to_vec(input)?)?;
    }
    Ok((selected, receipt))
}
pub(super) async fn local_run(command: &Commands, machine: bool) -> Result<()> {
    let directory = match command {
        Commands::Fetch { directory, .. }
        | Commands::Inspect { directory, .. }
        | Commands::FetchDecisions { directory, .. }
        | Commands::Event { directory, .. }
        | Commands::Receipt { directory, .. } => directory,
        _ => unreachable!(),
    };
    let (c, _) = load(directory)?;
    let client = Client::device(&c.endpoint, &c.token)?;
    let base = format!("v1/device-agent/{}/commands", c.device_id);
    let value = match command {
        Commands::FetchDecisions { command_id, .. } => {
            client
                .get::<Value>(&format!("{base}/{command_id}/approvals"))
                .await?
        }
        Commands::Fetch {
            after,
            through,
            wait_ms,
            ..
        } => {
            client
                .get::<Value>(&format!("{base}/{}", query(*after, *through, *wait_ms)?))
                .await?
        }
        Commands::Inspect { command_id, .. } => {
            client.get::<Value>(&format!("{base}/{command_id}")).await?
        }
        Commands::Receipt {
            command_id,
            request_id,
            ..
        } => {
            client
                .get::<Value>(&format!("{base}/{command_id}/receipts/{request_id}"))
                .await?
        }
        Commands::Event {
            command_id, file, ..
        } => {
            let input: ReportCommandEvent = read(file)?;
            ensure!(
                !serde_json::to_string(&input)?.contains(&c.token),
                "credential cannot enter the device journal"
            );
            let (_, receipt_file) = journal(directory, *command_id, &input)?;
            let receipt: CommandEventReceipt = client
                .post(
                    &format!("{base}/{command_id}/events"),
                    Some(serde_json::to_value(&input)?),
                )
                .await?;
            ensure!(
                receipt.event == input,
                "server receipt does not match the journaled event"
            );
            if receipt_file.exists() {
                local::private(&receipt_file, false, false)?;
                let old: CommandEventReceipt = read(&receipt_file)?;
                ensure!(old == receipt, "retained event receipt differs");
            } else {
                local::create_private(&receipt_file, &serde_json::to_vec(&receipt)?)?;
            }
            serde_json::to_value(receipt)?
        }
        _ => unreachable!(),
    };
    emit(&value, machine)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_command_journal_retains_unknown_posts_and_forbids_changed_sequence() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("device");
        std::fs::create_dir(&dir).unwrap();
        local::private(&dir, true, true).unwrap();
        let id = DeviceCommandId::new();
        let e = ReportCommandEvent {
            request_id: RequestId::new(),
            sequence: 1,
            kind: CommandEventKind::Accepted,
            detail: "durable fixture acceptance".into(),
            execution: None,
        };
        let (request, receipt) = journal(&dir, id, &e).unwrap();
        local::private(&request, false, false).unwrap();
        assert!(!receipt.exists());
        assert_eq!(read::<ReportCommandEvent>(&request).unwrap(), e);
        journal(&dir, id, &e).unwrap();
        let mut changed = e.clone();
        changed.request_id = RequestId::new();
        assert!(journal(&dir, id, &changed).is_err());
        let mut next = e.clone();
        next.sequence = 2;
        next.request_id = RequestId::new();
        next.kind = CommandEventKind::Started;
        assert!(journal(&dir, id, &next).is_err());
        let r = CommandEventReceipt {
            event: e,
            received_at_ms: 100,
            command_revision: 2,
            state: CommandState::Accepted,
        };
        local::create_private(&receipt, &serde_json::to_vec(&r).unwrap()).unwrap();
        journal(&dir, id, &next).unwrap();
    }
    #[test]
    fn secret_shaped_observations_never_enter_the_local_event_journal() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("device");
        std::fs::create_dir(&dir).unwrap();
        local::private(&dir, true, true).unwrap();
        let e = ReportCommandEvent {
            request_id: RequestId::new(),
            sequence: 1,
            kind: CommandEventKind::Accepted,
            detail: format!("lain-device-{}", "ab".repeat(32)),
            execution: None,
        };
        assert!(journal(&dir, DeviceCommandId::new(), &e).is_err());
        assert!(!dir.join("command-events").exists());
        assert!(
            Commands::Fetch {
                directory: dir,
                after: 0,
                through: None,
                wait_ms: 0
            }
            .local()
        );
    }
}
