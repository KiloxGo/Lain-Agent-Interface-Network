use crate::{api::Client, emit, local};
use anyhow::{Context, Result, ensure};
use clap::Subcommand;
use lain_wire::{DeviceId, DeviceWorkspaceId, RequestId, devices::*};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    io::Read,
    path::{Path, PathBuf},
};
use zeroize::{Zeroize, Zeroizing};
mod commands;

#[derive(Subcommand)]
pub enum Commands {
    /// Explicitly allow one native Codex host; never starts it or logs in.
    HostConfigure {
        #[arg(long)]
        directory: PathBuf,
        #[arg(long)]
        codex_path: PathBuf,
        #[arg(long)]
        workspace_write: bool,
        #[arg(long, default_value_t = 30)]
        offline_grace_seconds: u32,
    },
    Command {
        #[command(subcommand)]
        command: commands::Commands,
    },
    /// Generate a private device credential and a separate registration document.
    Init {
        #[arg(long)]
        directory: PathBuf,
        #[arg(long)]
        endpoint: String,
        #[arg(long)]
        label: String,
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        expires_at_ms: u64,
        /// Declare intended Codex work; installation/login/readiness are checked separately.
        #[arg(long)]
        codex_work: bool,
    },
    /// Authenticate this device without the Kernel administrator credential.
    Claim {
        #[arg(long)]
        directory: PathBuf,
    },
    Status {
        #[arg(long)]
        directory: PathBuf,
    },
    /// One explicit inventory report; inspect status before retrying uncertain HTTP.
    Report {
        #[arg(long)]
        directory: PathBuf,
        #[arg(long)]
        sequence: u64,
        #[arg(long)]
        request_id: Option<RequestId>,
    },
    List,
    Show {
        id: DeviceId,
    },
    /// Admit an exact public registration; does not grant Worker execution.
    Pair {
        #[arg(long)]
        file: PathBuf,
    },
    Revoke {
        id: DeviceId,
        #[arg(long)]
        revision: u64,
    },
}
impl Commands {
    pub fn local(&self) -> bool {
        if let Self::Command { command } = self {
            return command.local();
        }
        matches!(
            self,
            Self::Init { .. }
                | Self::Claim { .. }
                | Self::Status { .. }
                | Self::Report { .. }
                | Self::HostConfigure { .. }
        )
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Credential {
    version: u8,
    endpoint: String,
    device_id: DeviceId,
    claim_request_id: RequestId,
    token: String,
}
impl Drop for Credential {
    fn drop(&mut self) {
        self.token.zeroize();
    }
}
fn now() -> Result<u64> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis()
        .try_into()?)
}
fn read<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let f = std::fs::File::open(path).context("open selected device JSON")?;
    ensure!(
        f.metadata()?.is_file(),
        "device input must be an ordinary JSON file"
    );
    let mut bytes = Zeroizing::new(Vec::new());
    f.take(24577).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 24576, "device input too large");
    serde_json::from_slice(&bytes)
        .map_err(|_| anyhow::anyhow!("invalid device JSON; existing files retained"))
}
fn load(directory: &Path) -> Result<(Credential, RegisterDevice)> {
    ensure!(directory.is_absolute(), "device directory must be absolute");
    local::outside_checkout(directory)?;
    local::private(directory, true, false)?;
    for name in ["device-credential.json", "registration.json"] {
        local::private(&directory.join(name), false, false)?;
    }
    let c: Credential = read(&directory.join("device-credential.json"))?;
    let r: RegisterDevice = read(&directory.join("registration.json"))?;
    ensure!(
        c.version == 1
            && r.valid()
            && c.device_id == r.id
            && device_credential_fingerprint(&c.token).as_ref() == Some(&r.credential_fingerprint),
        "device credential/registration mismatch"
    );
    Ok((c, r))
}
fn initialize(
    directory: &Path,
    endpoint: &str,
    label: &str,
    workspace: &Path,
    expires_at_ms: u64,
    codex_work: bool,
) -> Result<Value> {
    ensure!(
        directory.is_absolute() && !directory.exists(),
        "select a new absolute device directory; existing files are never overwritten"
    );
    let workspace = workspace
        .canonicalize()
        .context("resolve selected native workspace")?;
    ensure!(workspace.is_dir(), "device workspace must be a directory");
    let platform = match std::env::consts::OS {
        "windows" => DevicePlatform::Windows,
        "macos" => DevicePlatform::Macos,
        "linux" => DevicePlatform::Linux,
        _ => anyhow::bail!("unsupported native platform"),
    };
    let mut bytes = Zeroizing::new([0u8; 32]);
    getrandom::fill(bytes.as_mut())
        .map_err(|_| anyhow::anyhow!("secure OS randomness unavailable"))?;
    let token = format!(
        "lain-device-{}",
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    );
    let c = Credential {
        version: 1,
        endpoint: endpoint.to_owned(),
        device_id: DeviceId::new(),
        claim_request_id: RequestId::new(),
        token,
    };
    let _ = Client::device(endpoint, &c.token)?;
    let now = now()?;
    let r = RegisterDevice {
        id: c.device_id,
        request_id: RequestId::new(),
        label: label.to_owned(),
        platform,
        credential_fingerprint: device_credential_fingerprint(&c.token)
            .context("invalid generated device credential")?,
        workspaces: vec![DeviceWorkspace {
            id: DeviceWorkspaceId::new(),
            path: workspace
                .to_str()
                .context("workspace must be UTF-8")?
                .to_owned(),
        }],
        capabilities: if codex_work {
            vec![DeviceCapability::CodexManagedWork]
        } else {
            vec![]
        },
        expires_at_ms,
    };
    ensure!(
        r.valid() && expires_at_ms > now && expires_at_ms <= now.saturating_add(31 * 86400000),
        "invalid device registration or finite expiry"
    );
    std::fs::create_dir(directory).context("create new device directory (parent must exist)")?;
    local::private(directory, true, true)?;
    local::outside_checkout(directory)?;
    let encoded = Zeroizing::new(serde_json::to_vec_pretty(&c)?);
    local::create_private(&directory.join("device-credential.json"), &encoded)?;
    local::create_private(
        &directory.join("registration.json"),
        &serde_json::to_vec_pretty(&r)?,
    )?;
    Ok(
        json!({"device_id":r.id,"registration":directory.join("registration.json"),"credential_file":directory.join("device-credential.json"),"credential_fingerprint":r.credential_fingerprint,"status":"prepared_not_paired","capabilities":if codex_work {"explicit_declaration_not_readiness"} else {"none_claimed_without_native_probe"},"declared_capabilities":r.capabilities,"readiness":"not_probed","service_started":false}),
    )
}
pub async fn local_run(command: &Commands, machine: bool) -> Result<()> {
    if let Commands::Command { command } = command {
        return commands::local_run(command, machine).await;
    }
    let result = match command {
        Commands::HostConfigure {
            directory,
            codex_path,
            workspace_write,
            offline_grace_seconds,
        } => {
            let (credential, _) = load(directory)?;
            ensure!(
                codex_path.is_absolute() && codex_path.is_file(),
                "select an absolute native Codex executable"
            );
            let policy = lain_wire::device_execution::DeviceHostPolicy {
                version: 1,
                codex_path: codex_path
                    .canonicalize()?
                    .to_str()
                    .context("native executable must be UTF-8")?
                    .into(),
                workspace_write: *workspace_write,
                offline_grace_seconds: *offline_grace_seconds,
            };
            ensure!(policy.valid(), "offline grace must be 0..60 seconds");
            let bytes = serde_json::to_string(&policy)?;
            local::actor_config::non_secret(&bytes, &Default::default())?;
            ensure!(
                !bytes.contains(&credential.token),
                "credential cannot enter native policy"
            );
            local::create_private(&directory.join("host-policy.json"), bytes.as_bytes())?;
            json!({"status":"configured_not_started","profile":"codex/local","policy_file":directory.join("host-policy.json"),"workspace_write":workspace_write,"offline_grace_seconds":offline_grace_seconds})
        }
        Commands::Init {
            directory,
            endpoint,
            label,
            workspace,
            expires_at_ms,
            codex_work,
        } => initialize(
            directory,
            endpoint,
            label,
            workspace,
            *expires_at_ms,
            *codex_work,
        )?,
        Commands::Claim { directory }
        | Commands::Status { directory }
        | Commands::Report { directory, .. } => {
            let (c, r) = load(directory)?;
            let client = Client::device(&c.endpoint, &c.token)?;
            let path = format!("v1/device-agent/{}", c.device_id);
            match command {
                Commands::Claim { .. } => {
                    client
                        .post::<Value>(
                            &format!("{path}/claim"),
                            Some(json!({"request_id":c.claim_request_id,"registration_fingerprint":registration_fingerprint(&r)})),
                        )
                        .await?
                }
                Commands::Status { .. } => client.get::<Value>(&path).await?,
                Commands::Report {
                    sequence,
                    request_id,
                    ..
                } => {
                    let input = ReportDevice {
                        request_id: request_id.unwrap_or_default(),
                        sequence: *sequence,
                        reported_at_ms: now()?,
                        available_workspaces: r
                            .workspaces
                            .iter()
                            .filter(|w| {
                                let p = Path::new(&w.path);
                                p.is_dir()
                                    && p.canonicalize()
                                        .ok()
                                        .as_ref()
                                        .is_some_and(|p| p.to_str() == Some(w.path.as_str()))
                            })
                            .map(|w| w.id)
                            .collect(),
                        capabilities: vec![],
                    };
                    client
                        .post::<Value>(
                            &format!("{path}/reports"),
                            Some(serde_json::to_value(input)?),
                        )
                        .await?
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!("owner commands use the owner connection"),
    };
    emit(&result, machine)
}
pub async fn run(client: &Client, command: Commands, machine: bool) -> Result<()> {
    if let Commands::Command { command } = command {
        return commands::run(client, command, machine).await;
    }
    let result = match command {
        Commands::List => client.get::<Value>("v1/devices").await?,
        Commands::Show { id } => client.get::<Value>(&format!("v1/devices/{id}")).await?,
        Commands::Pair { file } => {
            let r: RegisterDevice = read(&file)?;
            ensure!(r.valid(), "invalid device registration");
            client
                .post::<Value>("v1/devices", Some(serde_json::to_value(r)?))
                .await?
        }
        Commands::Revoke { id, revision } => {
            client
                .post::<Value>(
                    &format!("v1/devices/{id}/revoke"),
                    Some(json!({"expected_revision":revision})),
                )
                .await?
        }
        _ => unreachable!("local commands do not load owner credentials"),
    };
    emit(&result, machine)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_prepare_protects_separates_and_never_overwrites_credentials() {
        let root = tempfile::tempdir().unwrap();
        let workspace = root.path().join("workspace");
        std::fs::create_dir(&workspace).unwrap();
        let dir = root.path().join("device");
        let expiry = now().unwrap() + 600000;
        let result = initialize(
            &dir,
            "http://127.0.0.1:8787",
            "fixture-native",
            &workspace,
            expiry,
            false,
        )
        .unwrap();
        let (credential, r) = load(&dir).unwrap();
        assert_eq!(
            r.workspaces[0].path,
            workspace.canonicalize().unwrap().to_str().unwrap()
        );
        assert!(r.capabilities.is_empty());
        assert!(!result.to_string().contains(&credential.token));
        let registration = std::fs::read_to_string(dir.join("registration.json")).unwrap();
        assert!(!registration.contains(&credential.token));
        let before = std::fs::read(dir.join("device-credential.json")).unwrap();
        assert!(
            initialize(
                &dir,
                "http://127.0.0.1:8787",
                "fixture-native",
                &workspace,
                expiry,
                false
            )
            .is_err()
        );
        assert!(std::fs::read(dir.join("device-credential.json")).unwrap() == before);
        let mut bad = r;
        bad.credential_fingerprint = "a".repeat(64);
        std::fs::write(
            dir.join("registration.json"),
            serde_json::to_vec(&bad).unwrap(),
        )
        .unwrap();
        assert!(load(&dir).is_err());
    }
    #[test]
    fn native_prepare_rejects_git_locations_and_unprotected_transport_before_any_key_write() {
        let root = tempfile::tempdir().unwrap();
        let workspace = root.path().join("workspace");
        std::fs::create_dir(&workspace).unwrap();
        let expiry = now().unwrap() + 600000;
        let dir = root.path().join("insecure");
        assert!(
            initialize(
                &dir,
                "http://192.0.2.1:8787",
                "fixture",
                &workspace,
                expiry,
                false
            )
            .is_err()
        );
        assert!(!dir.exists());
        let repo = root.path().join("repo");
        std::fs::create_dir(&repo).unwrap();
        std::fs::write(repo.join(".git"), "fixture marker").unwrap();
        let rejected = repo.join("device");
        assert!(
            initialize(
                &rejected,
                "http://127.0.0.1:8787",
                "fixture",
                &workspace,
                expiry,
                false
            )
            .is_err()
        );
        assert!(!rejected.join("device-credential.json").exists());
    }
    #[test]
    fn codex_work_declaration_is_explicit_and_does_not_claim_readiness() {
        let root = tempfile::tempdir().unwrap();
        let workspace = root.path().join("workspace");
        std::fs::create_dir(&workspace).unwrap();
        let dir = root.path().join("device");
        let result = initialize(
            &dir,
            "http://127.0.0.1:8787",
            "work-pc",
            &workspace,
            now().unwrap() + 600000,
            true,
        )
        .unwrap();
        let (_, r) = load(&dir).unwrap();
        assert_eq!(r.capabilities, vec![DeviceCapability::CodexManagedWork]);
        assert_eq!(result["status"], "prepared_not_paired");
        assert!(!dir.join("host-policy.json").exists());
        assert!(!dir.join("execution").exists());
    }
    #[test]
    fn device_client_transport_does_not_allow_cleartext_remote_or_embedded_credentials() {
        let token = format!("lain-device-{}", "ab".repeat(32));
        for endpoint in [
            "http://192.0.2.1/",
            "http://localhost.example/",
            "https://user:pass@example.com/",
            "https://example.com/?credential=x",
            "https://example.com/#fragment",
        ] {
            assert!(Client::device(endpoint, &token).is_err());
        }
        for endpoint in [
            "https://example.com/lain/",
            "http://127.0.0.1:8787/",
            "http://[::1]:8787/",
        ] {
            assert!(Client::device(endpoint, &token).is_ok());
        }
    }
}
