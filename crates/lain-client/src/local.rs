//! Explicit local launcher/configuration boundary; never used to resolve remote workspaces.
use crate::api::Client;
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    net::SocketAddr,
    path::{Path, PathBuf},
    process::Stdio,
};
use zeroize::Zeroizing;
pub(crate) mod actor_config;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    version: u32,
    url: String,
    database: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    actor_config: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    env_file: Option<PathBuf>,
}
pub struct Connection {
    pub url: String,
    pub token: Zeroizing<String>,
}
pub fn directory() -> Result<PathBuf> {
    let path = if let Some(value) = std::env::var_os("LAIN_LOCAL_DIR") {
        PathBuf::from(value)
    } else {
        platform_directory()?
    };
    ensure!(path.is_absolute(), "LAIN_LOCAL_DIR must be absolute");
    // Local credentials must never be placed in a source checkout.
    ensure!(
        !path.ancestors().any(|p| p.join(".git").exists()),
        "local settings must be outside a Git checkout"
    );
    Ok(path)
}
#[cfg(target_os = "macos")]
fn platform_directory() -> Result<PathBuf> {
    Ok(
        PathBuf::from(std::env::var_os("HOME").context("HOME unavailable")?)
            .join("Library/Application Support/Lain"),
    )
}
#[cfg(windows)]
fn platform_directory() -> Result<PathBuf> {
    Ok(
        PathBuf::from(std::env::var_os("LOCALAPPDATA").context("LOCALAPPDATA unavailable")?)
            .join("Lain"),
    )
}
#[cfg(all(unix, not(target_os = "macos")))]
fn platform_directory() -> Result<PathBuf> {
    Ok(match std::env::var_os("XDG_CONFIG_HOME") {
        Some(p) => PathBuf::from(p).join("lain"),
        None => PathBuf::from(std::env::var_os("HOME").context("HOME unavailable")?)
            .join(".config/lain"),
    })
}
fn reject_link(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path).context("inspect local settings")?;
    ensure!(
        !metadata.file_type().is_symlink(),
        "local settings cannot be symbolic links"
    );
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        ensure!(
            metadata.file_attributes() & 0x400 == 0,
            "local settings cannot be reparse points"
        );
    }
    Ok(())
}
#[cfg(unix)]
pub(crate) fn private(path: &Path, directory: bool, set: bool) -> Result<()> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    reject_link(path)?;
    let metadata = fs::metadata(path)?;
    ensure!(
        metadata.uid() == unsafe { libc::geteuid() },
        "local settings must belong to the current owner"
    );
    ensure!(
        metadata.is_dir() == directory,
        "unexpected local settings file type"
    );
    if !directory {
        ensure!(
            metadata.is_file() && metadata.nlink() == 1,
            "credentials must be ordinary, unlinked files"
        );
    }
    let mode = if directory { 0o700 } else { 0o600 };
    if set {
        fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    }
    ensure!(
        fs::metadata(path)?.permissions().mode() & 0o777 == mode,
        "local settings permissions are not private; restore 0700 directories and 0600 files"
    );
    Ok(())
}
#[cfg(windows)]
pub(crate) fn private(path: &Path, directory: bool, set: bool) -> Result<()> {
    reject_link(path)?;
    ensure!(
        fs::metadata(path)?.is_dir() == directory,
        "unexpected local settings file type"
    );
    // `set` is used only immediately after exclusive creation of a new object.
    // Elevated Windows tokens may create Administrators-owned objects; assign the
    // current user before checking. Existing objects always use check-only mode.
    // Windows-only native .NET ACL API. The path is data in an environment variable,
    // never interpolated into PowerShell source. No credential is sent to this process.
    let script = r#"
$ErrorActionPreference = 'Stop'
# Use the modules shipped with this Windows PowerShell process, not inherited
# PowerShell 7/user module paths with incompatible framework assemblies.
$env:PSModulePath = [System.IO.Path]::Combine($PSHOME, 'Modules')
$p = $env:LAIN_ACL_PATH
$sid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User
$item = Get-Item -LiteralPath $p -Force
if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0 -or $item.LinkType -eq 'HardLink') { throw 'linked file rejected' }
$a = Get-Acl -LiteralPath $p
if ($env:LAIN_ACL_SET -ne '1' -and $a.GetOwner([System.Security.Principal.SecurityIdentifier]).Value -ne $sid.Value) { throw 'owner mismatch' }
if ($env:LAIN_ACL_SET -eq '1') {
    if ($env:LAIN_ACL_DIR -eq '1') {
        $a = New-Object System.Security.AccessControl.DirectorySecurity
        $r = New-Object System.Security.AccessControl.FileSystemAccessRule($sid, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
    } else {
        $a = New-Object System.Security.AccessControl.FileSecurity
        $r = New-Object System.Security.AccessControl.FileSystemAccessRule($sid, 'FullControl', 'Allow')
    }
    $a.SetOwner($sid)
    $a.SetAccessRuleProtection($true, $false)
    $a.AddAccessRule($r)
    Set-Acl -LiteralPath $p -AclObject $a
    $a = Get-Acl -LiteralPath $p
}
if (-not $a.AreAccessRulesProtected) { throw 'ACL inheritance must be disabled' }
$rules = @($a.GetAccessRules($true, $true, [System.Security.Principal.SecurityIdentifier]))
if ($rules.Count -eq 0) { throw 'empty ACL' }
foreach ($r in $rules) {
    if ($r.IdentityReference.Value -ne $sid.Value -or $r.AccessControlType -ne 'Allow') { throw 'non-owner ACL' }
}
if ($a.GetOwner([System.Security.Principal.SecurityIdentifier]).Value -ne $sid.Value) { throw 'owner mismatch' }
"#;
    let mut command = std::process::Command::new("powershell.exe");
    command.env_clear();
    for (key, value) in std::env::vars_os() {
        if matches!(
            key.to_string_lossy().to_ascii_uppercase().as_str(),
            "SYSTEMROOT"
                | "WINDIR"
                | "PATH"
                | "TEMP"
                | "TMP"
                | "USERPROFILE"
                | "LOCALAPPDATA"
                | "APPDATA"
                | "COMSPEC"
        ) {
            command.env(key, value);
        }
    }
    let status = command
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            script,
        ])
        .env("LAIN_ACL_PATH", path)
        .env("LAIN_ACL_SET", if set { "1" } else { "0" })
        .env("LAIN_ACL_DIR", if directory { "1" } else { "0" })
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("Windows PowerShell is required to protect/check local ACLs")?;
    ensure!(
        status.success(),
        "local settings ACL must grant only the current owner; inspect with Get-Acl"
    );
    Ok(())
}
pub(crate) fn create_private(path: &Path, content: &[u8]) -> Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options
        .open(path)
        .context("create local file without overwriting")?;
    // Parent is already private; Windows child inherits its owner-only ACL.
    private(path, false, true)?;
    file.write_all(content)?;
    file.sync_all()?;
    Ok(())
}
pub(crate) fn outside_checkout(path: &Path) -> Result<()> {
    let canonical = path
        .canonicalize()
        .context("resolve local configuration directory")?;
    ensure!(
        !canonical.ancestors().any(|p| p.join(".git").exists()),
        "local credentials must be outside a Git checkout"
    );
    Ok(())
}
fn read_settings(path: &Path) -> Result<Settings> {
    outside_checkout(path)?;
    private(path, true, false)?;
    private(&path.join("client.json"), false, false)?;
    ensure!(
        fs::metadata(path.join("client.json"))?.len() <= 4096,
        "local settings too large"
    );
    let settings: Settings = serde_json::from_slice(&fs::read(path.join("client.json"))?)
        .map_err(|_| anyhow::anyhow!("invalid local settings; existing files were retained"))?;
    ensure!(
        settings.version == 1
            && settings.database.is_absolute()
            && settings
                .actor_config
                .as_ref()
                .is_none_or(|p| p.is_absolute())
            && settings.env_file.as_ref().is_none_or(|p| p.is_absolute()),
        "unsupported local settings"
    );
    local_address(&settings.url)?;
    Ok(settings)
}
fn read_token(path: &Path) -> Result<Zeroizing<String>> {
    private(&path.join("kernel-token"), false, false)?;
    ensure!(
        fs::metadata(path.join("kernel-token"))?.len() <= 256,
        "local credential file too large"
    );
    let token = Zeroizing::new(fs::read_to_string(path.join("kernel-token"))?);
    validate_token(&token)?;
    Ok(token)
}
fn validate_token(token: &str) -> Result<()> {
    ensure!(
        (32..=256).contains(&token.len())
            && !token.starts_with("lain-device-")
            && token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b)),
        "invalid Kernel token; credentials retained"
    );
    Ok(())
}
fn local_address(url: &str) -> Result<SocketAddr> {
    let url = reqwest::Url::parse(url).context("invalid local URL")?;
    ensure!(
        url.scheme() == "http"
            && url.path() == "/"
            && url.query().is_none()
            && url.fragment().is_none()
            && url.username().is_empty()
            && url.password().is_none(),
        "local launcher requires a plain loopback HTTP URL; remote clients use LAIN_URL and LAIN_API_TOKEN"
    );
    let ip: std::net::IpAddr = url
        .host_str()
        .unwrap_or("")
        .trim_matches(['[', ']'])
        .parse()
        .context("local URL requires numeric loopback address")?;
    ensure!(ip.is_loopback(), "local launcher requires loopback");
    Ok(SocketAddr::new(
        ip,
        url.port_or_known_default().unwrap_or(8787),
    ))
}
pub fn setup_at(path: &Path, url: &str) -> Result<serde_json::Value> {
    local_address(url)?;
    if !path.exists() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(path)?;
        }
        #[cfg(windows)]
        {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            // Fail if another process won creation; never reset an existing ACL.
            fs::create_dir(path)?;
        }
        private(path, true, true)?;
    }
    outside_checkout(path)?;
    private(path, true, false)?;
    // A lock serializes setup and permits safe recovery after an interrupted first setup.
    let lock_path = path.join("setup.lock");
    if !lock_path.exists() {
        match create_private(&lock_path, b"") {
            Ok(()) => {}
            Err(_) if lock_path.exists() => {}
            Err(error) => return Err(error),
        }
    }
    private(&lock_path, false, false)?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(lock_path)?;
    lock.try_lock().context("another local setup is running")?;
    let reused = path.join("kernel-token").exists();
    if reused {
        read_token(path)?;
    } else {
        ensure!(
            !path.join("client.json").exists(),
            "token missing from existing setup; restore credentials instead of silently rotating"
        );
        let mut random = Zeroizing::new([0u8; 32]);
        getrandom::fill(random.as_mut())
            .map_err(|_| anyhow::anyhow!("secure OS randomness unavailable"))?;
        let token = Zeroizing::new(
            random
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
        );
        create_private(&path.join("kernel-token"), token.as_bytes())?;
    }
    if path.join("client.json").exists() {
        read_settings(path)?;
    } else {
        let settings = Settings {
            version: 1,
            url: url.to_owned(),
            database: path.join("lain.sqlite3"),
            actor_config: None,
            env_file: None,
        };
        create_private(
            &path.join("client.json"),
            &serde_json::to_vec_pretty(&settings)?,
        )?;
    }
    Ok(
        serde_json::json!({"status":if reused {"reused"} else {"created"},"directory":path,"workflow":"lain up; lain doctor; lain chat"}),
    )
}
pub fn connection() -> Result<Connection> {
    let override_url = std::env::var("LAIN_URL").ok();
    let override_token = std::env::var("LAIN_API_TOKEN").ok().map(Zeroizing::new);
    if let Some(token) = override_token {
        validate_token(&token)?;
        return Ok(Connection {
            url: override_url.unwrap_or_else(|| "http://127.0.0.1:8787".into()),
            token,
        });
    }
    let path = directory()?;
    let settings = read_settings(&path).context(
        "run lain setup --local, or set LAIN_URL and LAIN_API_TOKEN for a remote Kernel",
    )?;
    if let Some(url) = &override_url {
        ensure!(
            reqwest::Url::parse(url)? == reqwest::Url::parse(&settings.url)?,
            "LAIN_URL overrides local configuration; explicitly set LAIN_API_TOKEN for that endpoint"
        );
    }
    Ok(Connection {
        url: settings.url,
        token: read_token(&path)?,
    })
}
pub fn daemon_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("LAIN_DAEMON_PATH") {
        let path = PathBuf::from(path);
        ensure!(
            path.is_file(),
            "LAIN_DAEMON_PATH must select the native laind binary"
        );
        #[cfg(windows)]
        ensure!(
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("exe")),
            "LAIN_DAEMON_PATH must select native laind.exe, not a shell shim"
        );
        return Ok(path);
    }
    let name = format!("laind{}", std::env::consts::EXE_SUFFIX);
    let path = std::env::current_exe()?
        .parent()
        .context("CLI directory unavailable")?
        .join(name);
    ensure!(
        path.is_file(),
        "install laind beside lain, or set LAIN_DAEMON_PATH"
    );
    Ok(path)
}
pub fn path_candidates() -> Vec<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .flat_map(|p| {
            #[cfg(windows)]
            let names = ["lain.exe", "lain.cmd", "lain.ps1", "lain.bat"];
            #[cfg(unix)]
            let names = ["lain"];
            names
                .into_iter()
                .map(move |n| p.join(n))
                .filter(|p| p.is_file())
        })
        .collect()
}
pub async fn up(machine: bool, env_file: Option<&Path>) -> Result<()> {
    ensure!(
        std::env::var_os("LAIN_URL").is_none(),
        "lain up is explicitly local; unset LAIN_URL to use local setup"
    );
    ensure!(
        std::env::var_os("LAIN_LISTEN").is_none()
            && std::env::var_os("LAIN_DATABASE_PATH").is_none(),
        "lain up uses its local listener/database; launch laind directly for manual overrides"
    );
    let path = directory()?;
    let settings = read_settings(&path).context("run lain setup --local first")?;
    let effective = actor_config::Effective::load(&settings, env_file)?;
    let conn = connection()?;
    let address = local_address(&settings.url)?;
    let client = Client::new(&settings.url, &conn.token)?;
    // Any listening process is inspected; none is killed or adopted by PID.
    if tokio::net::TcpStream::connect(address).await.is_ok() {
        let health: serde_json::Value = client.get("health").await.context("port occupied: Kernel authentication mismatch or unrelated server; inspect lain doctor")?;
        ensure!(
            health["service"] == "lain-kernel" && health["version"] == env!("CARGO_PKG_VERSION"),
            "port occupied by an incompatible daemon or unrelated server"
        );
        let _: serde_json::Value = client
            .get("ready")
            .await
            .context("Kernel is running but not ready")?;
        let diagnostics: serde_json::Value = client.get("v1/diagnostics").await?;
        if effective.comparison(&diagnostics) == "restart_required" {
            let works: Vec<serde_json::Value> = client.get("v1/work-sessions").await?;
            let active = works
                .iter()
                .filter(|work| {
                    matches!(
                        work["state"].as_str(),
                        Some(
                            "created"
                                | "starting"
                                | "ready"
                                | "running"
                                | "waiting_for_approval"
                                | "idle"
                                | "disconnected"
                                | "unknown"
                        )
                    )
                })
                .count();
            bail!(
                "Actor configuration differs from the running daemon ({active} open worker sessions). Inspect lain work list before stopping active workers; gracefully stop your owned foreground lain up with Ctrl-C, then run lain up again. No process was stopped."
            );
        }
        return crate::emit(
            &serde_json::json!({"status":"already_running","actor_configuration":effective.comparison(&diagnostics),
            "apply":"Existing daemon retains its startup environment. Inspect work before an explicit graceful restart; credential rotation is not compared."}),
            machine,
        );
    }
    let mut command = std::process::Command::new(daemon_path()?);
    for (key, value) in &effective.values {
        command.env(key, value.as_str());
    }
    command
        .env("LAIN_API_TOKEN", conn.token.as_str())
        .env("LAIN_LISTEN", address.to_string())
        .env("LAIN_DATABASE_PATH", settings.database)
        .current_dir(&path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    // Foreground child shares its console/process group. Ctrl-C/Ctrl-Break is delivered
    // natively to laind; the launcher waits for its graceful storage/worker shutdown.
    let shutdown = crate::signals::shutdown()?;
    let signal = tokio::spawn(shutdown);
    let mut child = command.spawn().context("launch native laind")?;
    let result = tokio::task::spawn_blocking(move || child.wait()).await?;
    signal.abort();
    ensure!(
        result?.success(),
        "laind exited unsuccessfully; inspect its safe diagnostics"
    );
    Ok(())
}
pub async fn doctor(machine: bool, env_file: Option<&Path>) -> Result<()> {
    let mut report = serde_json::json!({"cli_version":env!("CARGO_PKG_VERSION"),"cli_executable":std::env::current_exe().ok(),"path_candidates":path_candidates(),"path_help":"PowerShell: Get-Command lain -All; where.exe lain. zsh: whence -a lain. Invoke the Rust binary by its absolute path if shadowed.","daemon_executable":daemon_path().ok()});
    let mut healthy = false;
    match connection() {
        Err(_) => report["kernel"] = "local configuration missing or invalid; run lain setup --local, or configure remote LAIN_URL and LAIN_API_TOKEN".into(),
        Ok(conn) => {
            let client = Client::new(&conn.url, &conn.token)?;
            match client.get::<serde_json::Value>("v1/diagnostics").await {
                Ok(value) => {
                    healthy = value["kernel"] == "ready" && value["storage"] == "healthy";
                    if std::env::var_os("LAIN_URL").is_none() {
                        let desired=(||->Result<_> {
                            let settings=read_settings(&directory()?)?;
                            actor_config::Effective::load(&settings,env_file)
                        })();
                        match desired {
                            Ok(effective)=>{
                                let comparison=effective.comparison(&value);
                                if comparison == "restart_required" { healthy=false; }
                                report["local_actor"]=effective.view();
                                report["actor_configuration"]=comparison.into();
                            },
                            Err(_)=>{ healthy=false; report["actor_configuration"]="invalid_local_configuration; inspect lain config actor and file permissions".into(); },
                        }
                    } else { report["actor_configuration"]="not_compared_remote".into(); }
                    report["diagnostics"] = value;
                },
                Err(error) => {
                    report["kernel"] = if error.to_string().contains("401") { "Kernel authentication failed; check LAIN_API_TOKEN override against local setup" } else if error.to_string().contains("404") { "Kernel incompatible; rebuild and restart laind" } else { "Kernel not running or unreachable; run lain up for local mode" }.into();
                }
            }
        }
    }
    if machine {
        crate::emit(&report, true)?;
    } else {
        println!("CLI:         {}", env!("CARGO_PKG_VERSION"));
        if let Some(kernel) = report["kernel"].as_str() {
            println!("Kernel:      {kernel}");
        }
        if let Some(d) = report.get("diagnostics") {
            println!(
                "Kernel:      {} (authenticated)",
                d["kernel"].as_str().unwrap_or("unknown")
            );
            println!(
                "Storage:     {}",
                d["storage"].as_str().unwrap_or("unknown")
            );
            if let Some(backends) = d["backends"].as_array() {
                for backend in backends {
                    println!(
                        "Codex:       {} — supported version {}; executable {}",
                        backend["status"].as_str().unwrap_or("unavailable"),
                        backend["supported_version"].as_str().unwrap_or("unknown"),
                        crate::tui::safe_text(&backend["executable"].to_string())
                    );
                }
            }
            println!(
                "Lain Actor:  {}",
                d["actor"]["status"].as_str().unwrap_or("unknown")
            );
            if d["actor"]["status"] != "configured" {
                println!(
                    "             Configure LAIN_ACTOR_CONFIG and its namespaced LAIN_SECRET variable; see docs/LAIN_ACTOR.md"
                );
            }
            if let Some(profiles) = d["profiles"].as_array() {
                for profile in profiles {
                    println!(
                        "Profile:     {} — {}; credential {}",
                        profile["id"].as_str().unwrap_or("unknown"),
                        profile["health"].as_str().unwrap_or("unknown"),
                        profile["secret_status"].as_str().unwrap_or("unknown")
                    );
                }
            }
        }
        if let Some(state) = report["actor_configuration"].as_str() {
            println!("Actor config: {state}");
            if state == "restart_required" {
                println!(
                    "             Inspect lain work list before gracefully stopping your owned foreground daemon; next lain up applies saved files. Secret values are never compared."
                );
            }
        }
        println!(
            "Executable:  {}",
            crate::tui::safe_text(&report["cli_executable"].to_string())
        );
        println!(
            "PATH:        {}",
            report["path_help"].as_str().unwrap_or("")
        );
        if let Some(candidates) = report["path_candidates"].as_array() {
            for path in candidates {
                println!(
                    "             {}",
                    crate::tui::safe_text(path.as_str().unwrap_or(""))
                );
            }
        }
    }
    if !healthy {
        bail!("doctor found a Kernel/configuration problem");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn setup_reuses_credentials_and_fails_closed_without_overwrite() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("private");
        setup_at(&path, "http://127.0.0.1:8787").unwrap();
        let token = fs::read(path.join("kernel-token")).unwrap();
        assert_eq!(token.len(), 64);
        assert_eq!(
            setup_at(&path, "http://127.0.0.1:9999").unwrap()["status"],
            "reused"
        );
        assert_eq!(fs::read(path.join("kernel-token")).unwrap(), token);
        assert_eq!(read_settings(&path).unwrap().url, "http://127.0.0.1:8787");
        fs::write(path.join("client.json"), "broken").unwrap();
        assert!(setup_at(&path, "http://127.0.0.1:8787").is_err());
        assert_eq!(fs::read(path.join("kernel-token")).unwrap(), token);
    }
    #[cfg(unix)]
    #[test]
    fn native_unix_modes_and_symlink_rejection() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("private");
        setup_at(&path, "http://127.0.0.1:8787").unwrap();
        assert_eq!(
            fs::metadata(path.join("kernel-token"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        fs::set_permissions(path.join("kernel-token"), fs::Permissions::from_mode(0o644)).unwrap();
        assert!(read_token(&path).is_err());
        fs::set_permissions(path.join("kernel-token"), fs::Permissions::from_mode(0o600)).unwrap();
        fs::hard_link(path.join("kernel-token"), temp.path().join("token-alias")).unwrap();
        assert!(read_token(&path).is_err());
        let other = temp.path().join("link");
        symlink(&path, &other).unwrap();
        assert!(read_settings(&other).is_err());
    }
    #[cfg(windows)]
    #[test]
    fn native_windows_acl_rejects_non_owner_access() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("private");
        setup_at(&path, "http://127.0.0.1:8787").unwrap();
        private(&path.join("kernel-token"), false, false).unwrap();
        let status = std::process::Command::new("icacls.exe")
            .arg(path.join("kernel-token"))
            .args(["/grant", "*S-1-1-0:(R)"])
            .output()
            .unwrap();
        assert!(status.status.success());
        assert!(read_token(&path).is_err());
    }
    #[cfg(windows)]
    #[test]
    fn native_setup_never_repairs_or_takes_over_an_existing_directory() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("existing");
        fs::create_dir(&path).unwrap();
        assert!(private(&path, true, false).is_err());
        assert!(setup_at(&path, "http://127.0.0.1:8787").is_err());
        assert!(private(&path, true, false).is_err());
        assert!(fs::read_dir(&path).unwrap().next().is_none());
    }
    #[test]
    fn launcher_rejects_remote_and_credential_bearing_urls() {
        for url in [
            "https://example.com",
            "http://192.0.2.1:8787",
            "http://owner:pass@127.0.0.1:8787",
            "http://127.0.0.1:8787/base",
        ] {
            assert!(local_address(url).is_err());
        }
        assert!(local_address("http://[::1]:8787").is_ok());
    }
}
