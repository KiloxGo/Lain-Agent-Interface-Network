use super::*;
use std::io::Read;
fn exists(path: &Path) -> Result<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

pub(super) fn read<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    lain_local::private(path, false, false)?;
    let file = std::fs::File::open(path)?;
    let mut bytes = zeroize::Zeroizing::new(Vec::new());
    file.take(131073).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 131072,
        "protected device record exceeds bounds"
    );
    serde_json::from_slice(&bytes)
        .map_err(|_| anyhow::anyhow!("invalid protected device record; retained for inspection"))
}
pub(super) fn directory(path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir(path)?;
        lain_local::private(path, true, true)?;
    }
    lain_local::private(path, true, false)
}
pub(super) fn write(path: &Path, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    if exists(path)? {
        let old: serde_json::Value = read(path)?;
        ensure!(
            old == serde_json::from_slice::<serde_json::Value>(&bytes)?,
            "immutable device journal conflict"
        );
    } else {
        let pending = stage(path, &bytes)?;
        // serve() owns the directory's exclusive host lock. No second journal
        // writer is admitted; existing published records are never replaced.
        if exists(path)? {
            let _ = std::fs::remove_file(&pending);
            return write(path, value);
        }
        std::fs::rename(&pending, path).context("publish complete device record")?;
        #[cfg(unix)]
        std::fs::File::open(path.parent().context("journal parent missing")?)?.sync_all()?;
    }
    Ok(())
}
pub(super) fn stage(path: &Path, bytes: &[u8]) -> Result<PathBuf> {
    let name = path
        .file_name()
        .context("journal record has no name")?
        .to_str()
        .context("journal name not UTF-8")?;
    let pending = path.with_file_name(format!(".{name}.pending-{}", uuid::Uuid::new_v4()));
    lain_local::create_private(&pending, bytes)?;
    Ok(pending)
}
#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Intent {
    pub device_id: DeviceId,
    pub command_id: DeviceCommandId,
    pub input: QueueDeviceWork,
    pub registration: String,
    pub policy: String,
    pub execution: DeviceExecutionId,
    pub session: lain_wire::WorkSessionId,
    pub turn: lain_wire::WorkerTurnId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation: Option<lain_wire::device_work::DeviceContinuation>,
}
impl Intent {
    fn new(c: &DeviceCommand, registration: String, policy: String) -> Self {
        let id = |name: &str| uuid::Uuid::new_v5(&c.id.0, name.as_bytes());
        Self {
            device_id: c.device_id,
            command_id: c.id,
            input: c.input.clone(),
            registration,
            policy,
            execution: DeviceExecutionId(id("lain-device-execution-v1")),
            session: c.continuation.as_ref().map_or(
                lain_wire::WorkSessionId(id("lain-device-session-v1")),
                |p| p.session_id,
            ),
            turn: lain_wire::WorkerTurnId(id("lain-device-turn-v1")),
            continuation: c.continuation.clone(),
        }
    }
    pub fn session(&self) -> domain::WorkSessionId {
        self.session.to_string().parse().expect("typed UUID")
    }
    pub fn turn(&self) -> domain::TurnId {
        self.turn.to_string().parse().expect("typed UUID")
    }
}
pub(super) struct Journal {
    pub path: PathBuf,
    pub intent: Intent,
    pub last: Option<ReportCommandEvent>,
}
impl Journal {
    pub fn verify_parent(
        root: &Path,
        c: &DeviceCommand,
        registration: &str,
        policy: &str,
    ) -> Result<()> {
        if let Some(lineage) = &c.continuation {
            ensure!(
                lineage.parent_command_id != c.id && lineage.root_command_id != c.id,
                "cyclic native continuation"
            );
            let parent_path = root.join(lineage.parent_command_id.to_string());
            let parent: Intent = read(&parent_path.join("intent.json"))
                .context("original native parent intent required")?;
            ensure!(
                parent.device_id == c.device_id
                    && parent.command_id == lineage.parent_command_id
                    && parent.registration == registration
                    && parent.policy == policy
                    && parent.session == lineage.session_id
                    && parent.turn == lineage.parent_turn_id
                    && parent
                        .continuation
                        .as_ref()
                        .map_or(parent.command_id, |f| f.root_command_id)
                        == lineage.root_command_id
                    && parent.input.workspace_id == c.input.workspace_id
                    && parent.input.profile_id == c.input.profile_id
                    && parent.input.sandbox == c.input.sandbox,
                "continuation differs from immutable parent scope"
            );
            let submitted: bool = read(&parent_path.join("submission.json"))
                .context("parent was not submitted by this native host")?;
            ensure!(submitted, "invalid parent submission fence");
        }
        Ok(())
    }
    pub fn open(
        root: &Path,
        c: &DeviceCommand,
        registration: String,
        policy: String,
    ) -> Result<Self> {
        Self::verify_parent(root, c, &registration, &policy)?;
        let path = root.join(c.id.to_string());
        directory(&path)?;
        let input = Intent::new(c, registration, policy);
        let selected = path.join("intent.json");
        write(&selected, &input)?;
        let intent: Intent = read(&selected)?;
        ensure!(intent == input, "command scope differs from local journal");
        let mut last = None;
        for sequence in 1..=257 {
            let p = path.join(format!("event-{sequence}.json"));
            if !exists(&p)? {
                break;
            }
            let e: ReportCommandEvent = read(&p)?;
            ensure!(e.sequence == sequence, "invalid event journal order");
            last = Some(e);
        }
        Ok(Self { path, intent, last })
    }
    pub fn claimed(&self, name: &str) -> Result<bool> {
        let p = self.path.join(format!("{name}.json"));
        if !exists(&p)? {
            return Ok(false);
        }
        let v: bool = read(&p)?;
        ensure!(v, "invalid effect fence");
        Ok(true)
    }
    pub fn claim(&self, name: &str) -> Result<()> {
        write(&self.path.join(format!("{name}.json")), &true)
    }
    pub fn event(
        &mut self,
        kind: CommandEventKind,
        evidence: Option<ExecutionEvidence>,
        detail: &str,
    ) -> Result<()> {
        ensure!(
            evidence.as_ref().is_none_or(ExecutionEvidence::valid),
            "invalid native execution evidence"
        );
        if let Some(e) = &self.last {
            let receipt: CommandEventReceipt =
                read(&self.path.join(format!("receipt-{}.json", e.sequence)))
                    .context("previous event receipt unresolved")?;
            ensure!(
                receipt.event == *e,
                "previous event receipt differs from immutable request"
            );
        }
        let e = ReportCommandEvent {
            request_id: RequestId::new(),
            sequence: self.last.as_ref().map_or(1, |e| e.sequence + 1),
            kind,
            detail: detail.into(),
            execution: evidence,
        };
        ensure!(e.sequence <= 257, "device event capacity reached");
        crate::profiles::check_sensitive(&e)?;
        write(&self.path.join(format!("event-{}.json", e.sequence)), &e)?;
        self.last = Some(e);
        Ok(())
    }
    pub async fn flush(&self, cloud: &cloud::Cloud) -> Result<()> {
        if let Some(e) = &self.last {
            let p = self.path.join(format!("receipt-{}.json", e.sequence));
            if exists(&p)? {
                let r: CommandEventReceipt = read(&p)?;
                ensure!(r.event == *e, "mismatched local receipt");
            } else {
                let r: CommandEventReceipt = cloud
                    .post(&format!("{}/events", self.intent.command_id), e)
                    .await?;
                ensure!(r.event == *e, "server receipt differs from immutable event");
                write(&p, &r)?;
            }
        }
        Ok(())
    }
}
