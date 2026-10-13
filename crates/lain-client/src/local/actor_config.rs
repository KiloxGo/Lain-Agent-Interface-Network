//! Explicit owner-selected local files. No shell evaluation or workspace discovery.
use super::*;
use std::collections::BTreeMap;

type Values = BTreeMap<String, Zeroizing<String>>;
const BUILTIN_PERSONA: &str = include_str!("../../../../identity/persona.md");
fn non_secret(text: &str, values: &Values) -> Result<()> {
    static PATTERN: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(
        r"(?i)(sk-[a-z0-9_-]{12,}|bearer\s+\S+|(?:api[_-]?key|access[_-]?token|refresh[_-]?token|password|secret)\s*[:=]|-----BEGIN [A-Z ]*PRIVATE KEY-----)"
    ).unwrap()
    });
    ensure!(
        !PATTERN.is_match(text)
            && !values
                .iter()
                .any(|(key, value)| key.starts_with("LAIN_SECRET_")
                    && !value.is_empty()
                    && text.contains(value.as_str())),
        "sensitive content is not allowed in Actor configuration or persona"
    );
    Ok(())
}

// Local schema view only; the Kernel independently validates configuration and
// remains the authority for model/credential readiness and provider compatibility.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ActorFile {
    driver: String,
    model: String,
    endpoint: String,
    secret_ref: String,
    #[serde(default)]
    reasoning_effort: Option<String>,
    #[serde(default = "timeout")]
    timeout_seconds: u64,
    #[serde(default = "output")]
    max_output_tokens: u32,
    #[serde(default = "budget")]
    max_turn_tokens: u64,
}
fn timeout() -> u64 {
    45
}
fn output() -> u32 {
    2048
}
fn budget() -> u64 {
    16384
}
fn secret_variable(reference: &str) -> Result<String> {
    let name = reference.strip_prefix("secret://env/").unwrap_or("");
    ensure!(
        !name.is_empty()
            && name.len() <= 64
            && name.split('-').all(|s| !s.is_empty()
                && s.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())),
        "Actor requires a namespaced secret://env/ reference"
    );
    Ok(format!(
        "LAIN_SECRET_{}",
        name.replace('-', "_").to_ascii_uppercase()
    ))
}
fn allowed(key: &str) -> bool {
    if matches!(key, "LAIN_ACTOR_CONFIG" | "LAIN_PERSONA_PATH") {
        return true;
    }
    key.strip_prefix("LAIN_SECRET_").is_some_and(|name| {
        !name.is_empty()
            && name.len() <= 64
            && name.split('_').all(|part| {
                !part.is_empty()
                    && part
                        .bytes()
                        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
            })
    })
}
fn actor_file(path: &Path, known: &Values) -> Result<(String, String)> {
    ensure!(
        fs::metadata(path)?.is_file() && fs::metadata(path)?.len() <= 16384,
        "Actor JSON must be a regular file of at most 16 KiB"
    );
    let bytes = fs::read(path)?;
    let config: ActorFile = serde_json::from_slice(&bytes).map_err(|_| {
        anyhow::anyhow!("invalid non-secret Actor JSON; raw credentials are not allowed")
    })?;
    ensure!(
        config.driver == "openai-responses"
            && !config.model.is_empty()
            && config.model.len() <= 128
            && !config.model.to_ascii_lowercase().starts_with("sk-")
            && config
                .model
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-._/".contains(&b)),
        "Actor requires an explicit valid model and openai-responses driver"
    );
    ensure!(
        config.endpoint.len() <= 2048
            && !config
                .endpoint
                .chars()
                .any(|c| c.is_control() || c.is_whitespace())
            && !config.endpoint.contains('\\'),
        "invalid Actor endpoint"
    );
    let url = reqwest::Url::parse(&config.endpoint)
        .map_err(|_| anyhow::anyhow!("invalid Actor endpoint"))?;
    let local = url.host_str().is_some_and(|host| {
        host == "localhost"
            || host
                .trim_matches(['[', ']'])
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
    });
    ensure!(
        url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
            && (url.scheme() == "https" || (url.scheme() == "http" && local)),
        "Actor endpoint requires HTTPS (HTTP only on loopback) and no credentials/query/fragment"
    );
    ensure!(
        (1..=90).contains(&config.timeout_seconds)
            && (128..=8192).contains(&config.max_output_tokens)
            && (1024..=65536).contains(&config.max_turn_tokens),
        "invalid Actor timeout or token budget"
    );
    ensure!(
        config
            .reasoning_effort
            .as_deref()
            .is_none_or(|s| matches!(s, "none" | "minimal" | "low" | "medium" | "high" | "xhigh")),
        "unsupported Actor reasoning setting"
    );
    non_secret(
        &serde_json::to_string(&(&config.model, &config.endpoint))?,
        known,
    )?;
    Ok((
        lain_wire::actor_configuration_revision(&bytes),
        secret_variable(&config.secret_ref)?,
    ))
}
fn selected_file(path: &Path) -> Result<PathBuf> {
    ensure!(path.is_file(), "selected configuration file does not exist");
    path.canonicalize()
        .context("resolve explicitly selected configuration file")
}
fn protected_env_file(path: &Path) -> Result<PathBuf> {
    // Check before canonicalizing so an explicitly selected link cannot bypass
    // the owner/type/link/permission checks.
    private(path, false, false)
        .context("env file must be owner-only: Unix 0600 or owner-restricted Windows ACL")?;
    let path = selected_file(path)?;
    private(
        path.parent().context("env-file parent missing")?,
        true,
        false,
    )
    .context(
        "put credentials in a private owner-only directory, such as the Lain local directory",
    )?;
    outside_checkout(&path)?;
    Ok(path)
}
fn parse_env(text: &str) -> Result<Values> {
    ensure!(text.len() <= 65536, "env file exceeds 64 KiB");
    let mut values = Values::new();
    for line in text.lines() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .context("env file requires unquoted KEY=VALUE lines; no export or shell statements")?;
        ensure!(
            allowed(key),
            "unsupported env-file key; only LAIN_ACTOR_CONFIG, LAIN_PERSONA_PATH and namespaced LAIN_SECRET variables are allowed"
        );
        ensure!(
            !value.is_empty() && value.len() <= 8192 && !value.chars().any(char::is_control),
            "env-file values must be nonempty, bounded single-line text"
        );
        ensure!(
            !values.contains_key(key) && values.len() < 128,
            "duplicate env-file key or too many entries"
        );
        values.insert(key.into(), Zeroizing::new(value.into()));
    }
    Ok(values)
}
fn load_env(path: &Path) -> Result<(PathBuf, Values)> {
    let path = protected_env_file(path)?;
    ensure!(
        fs::metadata(&path)?.len() <= 65536,
        "env file exceeds 64 KiB"
    );
    let text = Zeroizing::new(
        fs::read_to_string(&path).context("env file must contain UTF-8 KEY=VALUE text")?,
    );
    let mut values = parse_env(&text)?;
    for key in ["LAIN_ACTOR_CONFIG", "LAIN_PERSONA_PATH"] {
        if let Some(value) = values.get_mut(key) {
            let selected = PathBuf::from(value.as_str());
            let absolute = if selected.is_absolute() {
                selected
            } else {
                path.parent()
                    .context("env-file parent missing")?
                    .join(selected)
            };
            **value = selected_file(&absolute)?
                .to_str()
                .context("selected path must be Unicode")?
                .into();
        }
    }
    Ok((path, values))
}
fn process_values() -> Result<Values> {
    let mut result = Values::new();
    for (key, value) in std::env::vars_os() {
        let Some(key) = key.to_str() else {
            continue;
        };
        #[cfg(windows)]
        let canonical_key = key.to_ascii_uppercase();
        #[cfg(windows)]
        let key = canonical_key.as_str();
        if allowed(key) {
            let value = value
                .into_string()
                .map_err(|_| anyhow::anyhow!("selected Actor environment must be Unicode"))?;
            result.insert(key.into(), Zeroizing::new(value));
        }
    }
    Ok(result)
}
pub(super) struct Effective {
    pub values: Values,
    actor_path: Option<PathBuf>,
    env_file: Option<PathBuf>,
    source: &'static str,
    pub revision: Option<String>,
    persona_revision: String,
    secret_variable: Option<String>,
}
impl Effective {
    pub(super) fn load(settings: &Settings, explicit: Option<&Path>) -> Result<Self> {
        Self::resolve(settings, explicit, process_values()?)
    }
    fn resolve(settings: &Settings, explicit: Option<&Path>, process: Values) -> Result<Self> {
        let (env_file, mut values) = match explicit.or(settings.env_file.as_deref()) {
            Some(path) => {
                let (path, values) = load_env(path)?;
                (Some(path), values)
            }
            None => (None, Values::new()),
        };
        let source = if process.contains_key("LAIN_ACTOR_CONFIG") {
            "process_environment"
        } else if values.contains_key("LAIN_ACTOR_CONFIG") {
            "env_file"
        } else if settings.actor_config.is_some() {
            "saved_setting"
        } else {
            "default"
        };
        if let Some(path) = &settings.actor_config {
            values
                .entry("LAIN_ACTOR_CONFIG".into())
                .or_insert(Zeroizing::new(
                    path.to_str().context("saved path must be Unicode")?.into(),
                ));
        }
        values.extend(process); // Explicit process values always win, including empty values.
        let actor_path = values
            .get("LAIN_ACTOR_CONFIG")
            .map(|s| selected_file(Path::new(s.as_str())))
            .transpose()?;
        if let Some(path) = &actor_path {
            values.insert(
                "LAIN_ACTOR_CONFIG".into(),
                Zeroizing::new(path.to_str().context("Actor path must be Unicode")?.into()),
            );
        }
        let mut persona = BUILTIN_PERSONA.to_owned();
        if let Some(value) = values.get_mut("LAIN_PERSONA_PATH") {
            let path = selected_file(Path::new(value.as_str()))?;
            ensure!(fs::metadata(&path)?.len() <= 8192, "persona exceeds 8 KiB");
            **value = path
                .to_str()
                .context("persona path must be Unicode")?
                .into();
            persona = fs::read_to_string(path)?;
        }
        ensure!(
            !persona.trim().is_empty() && persona.len() <= 8192,
            "persona must be nonempty and bounded"
        );
        non_secret(&persona, &values)?;
        let persona_revision = lain_wire::actor_configuration_revision(persona.as_bytes());
        let (revision, secret_variable) = match &actor_path {
            Some(path) => {
                let (revision, variable) = actor_file(path, &values)?;
                (Some(revision), Some(variable))
            }
            None => (None, None),
        };
        Ok(Self {
            values,
            actor_path,
            env_file,
            source,
            revision,
            persona_revision,
            secret_variable,
        })
    }
    pub(super) fn view(&self) -> serde_json::Value {
        let available = self.secret_variable.as_ref().is_some_and(|key| {
            self.values.get(key).is_some_and(|v| {
                !v.is_empty() && v.len() <= 8192 && !v.chars().any(char::is_control)
            })
        });
        serde_json::json!({"actor_file":self.actor_path,"env_file":self.env_file,"source":self.source,
            "configuration_revision":self.revision,"persona_revision":self.persona_revision,"secret_variable":self.secret_variable,
            "credential_status":if available {"available"} else {"missing"},
            "apply":"Next explicit daemon start; inspect lain work list before gracefully stopping your owned foreground lain up. Secret rotation requires restart and cannot be compared by value."})
    }
    pub(super) fn comparison(&self, diagnostics: &serde_json::Value) -> &'static str {
        match (
            self.revision.as_deref(),
            diagnostics["actor"]["configuration_revision"].as_str(),
        ) {
            (Some(desired), Some(actual)) if desired == actual => {
                match diagnostics["actor"]["persona_revision"].as_str() {
                    Some(actual) if actual == self.persona_revision => "matches",
                    Some(_) => "restart_required",
                    None => "not_observable",
                }
            }
            (Some(_), Some(_)) => "restart_required",
            (Some(_), None) if diagnostics["actor"]["status"] == "not_configured" => {
                "restart_required"
            }
            _ => "not_observable",
        }
    }
}
fn update(change: impl FnOnce(&mut Settings)) -> Result<serde_json::Value> {
    let path = directory()?;
    read_settings(&path).context("run lain setup --local first")?;
    private(&path.join("setup.lock"), false, false)?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path.join("setup.lock"))?;
    lock.try_lock()
        .context("another local configuration operation is running")?;
    let mut settings = read_settings(&path)?;
    change(&mut settings);
    let view = Effective::load(&settings, None)?.view();
    let staged = path.join(format!("client-{}.tmp", uuid::Uuid::new_v4()));
    create_private(&staged, &serde_json::to_vec_pretty(&settings)?)?;
    if fs::rename(&staged, path.join("client.json")).is_err() {
        let _ = fs::remove_file(staged);
        bail!("could not atomically save local settings; previous configuration was retained");
    }
    Ok(view)
}
pub(crate) fn set_actor(file: &Path) -> Result<serde_json::Value> {
    let selected = selected_file(file)?;
    actor_file(&selected, &process_values()?)?;
    update(|settings| settings.actor_config = Some(selected))
}
pub(crate) fn set_env_file(file: &Path) -> Result<serde_json::Value> {
    let (selected, _) = load_env(file)?;
    update(|settings| settings.env_file = Some(selected))
}
pub(crate) fn clear() -> Result<serde_json::Value> {
    update(|settings| {
        settings.actor_config = None;
        settings.env_file = None;
    })
}
pub(crate) fn show() -> Result<serde_json::Value> {
    let settings = read_settings(&directory()?).context("run lain setup --local first")?;
    Ok(Effective::load(&settings, None)?.view())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(path: &Path, model: &str) {
        fs::write(
            path,
            serde_json::to_vec(
                &serde_json::json!({"driver":"openai-responses","model":model,
            "endpoint":"http://127.0.0.1:1234/v1","secret_ref":"secret://env/actor"}),
            )
            .unwrap(),
        )
        .unwrap();
    }
    #[test]
    fn env_format_is_allowlisted_literal_bounded_and_never_echoes_values() {
        let text = "# owner selection\r\nLAIN_SECRET_ACTOR=literal-$(touch ignored)-${HOME}\r\nLAIN_SECRET_GATEWAY_A=value=with=equals\n";
        let parsed = parse_env(text).unwrap();
        assert_eq!(
            parsed["LAIN_SECRET_ACTOR"].as_str(),
            "literal-$(touch ignored)-${HOME}"
        );
        assert_eq!(
            parsed["LAIN_SECRET_GATEWAY_A"].as_str(),
            "value=with=equals"
        );
        for bad in [
            "LAIN_API_TOKEN=do-not-echo",
            "PATH=do-not-echo",
            "export LAIN_SECRET_ACTOR=do-not-echo",
            "LAIN_SECRET__ACTOR=do-not-echo",
            "LAIN_SECRET_ACTOR=do-not-echo\nLAIN_SECRET_ACTOR=duplicate",
            "LAIN_SECRET_ACTOR=do-not-echo\u{1b}",
            "LAIN_SECRET_ACTOR=",
            "shell-statement-do-not-echo",
        ] {
            let error = match parse_env(bad) {
                Err(error) => error.to_string(),
                Ok(_) => panic!("malformed env file accepted"),
            };
            assert!(!error.contains("do-not-echo") && !error.contains("duplicate\n"));
        }
    }
    #[test]
    fn native_paths_with_spaces_and_deterministic_precedence_remain_secret_free() {
        let temp = tempfile::tempdir().unwrap();
        let local = temp.path().join("private local settings");
        setup_at(&local, "http://127.0.0.1:8787").unwrap();
        let saved = local.join("saved actor.json");
        fixture(&saved, "saved");
        let from_file = local.join("file actor.json");
        fixture(&from_file, "env-file");
        let from_process = local.join("process actor.json");
        fixture(&from_process, "process");
        let env = local.join("actor secrets.env");
        create_private(
            &env,
            b"LAIN_ACTOR_CONFIG=file actor.json\r\nLAIN_SECRET_ACTOR=file-credential-123\r\n",
        )
        .unwrap();
        let mut settings = read_settings(&local).unwrap();
        settings.actor_config = Some(saved.clone());
        settings.env_file = Some(env);
        let file = Effective::resolve(&settings, None, Values::new()).unwrap();
        assert_eq!(file.actor_path, Some(from_file.canonicalize().unwrap()));
        assert_eq!(file.source, "env_file");
        let mut process = Values::new();
        process.insert(
            "LAIN_ACTOR_CONFIG".into(),
            Zeroizing::new(from_process.to_str().unwrap().into()),
        );
        process.insert(
            "LAIN_SECRET_ACTOR".into(),
            Zeroizing::new("process-credential-123".into()),
        );
        let resolved = Effective::resolve(&settings, None, process).unwrap();
        assert_eq!(
            resolved.actor_path,
            Some(from_process.canonicalize().unwrap())
        );
        assert_eq!(resolved.source, "process_environment");
        assert_eq!(
            resolved.values["LAIN_SECRET_ACTOR"].as_str(),
            "process-credential-123"
        );
        let view = resolved.view().to_string();
        assert!(!view.contains("process-credential-123") && !view.contains("file-credential-123"));
        let diagnostics = serde_json::json!({"actor":{"configuration_revision":resolved.revision,"persona_revision":resolved.persona_revision}});
        assert_eq!(resolved.comparison(&diagnostics), "matches");
        fixture(&from_process, "changed");
        let changed = actor_file(&from_process, &Values::new()).unwrap().0;
        assert_eq!(
            resolved.comparison(&serde_json::json!({"actor":{"configuration_revision":changed}})),
            "restart_required"
        );
        settings.env_file = None;
        assert_eq!(
            Effective::resolve(&settings, None, Values::new())
                .unwrap()
                .actor_path,
            Some(saved.canonicalize().unwrap())
        );
        settings.actor_config = None;
        assert!(
            Effective::resolve(&settings, None, Values::new())
                .unwrap()
                .actor_path
                .is_none()
        );
    }
    #[cfg(unix)]
    #[test]
    fn native_env_credentials_reject_permissions_links_and_git_location() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let temp = tempfile::tempdir().unwrap();
        private(temp.path(), true, true).unwrap();
        let env = temp.path().join("secrets.env");
        create_private(&env, b"LAIN_SECRET_ACTOR=fixture-credential").unwrap();
        assert!(load_env(&env).is_ok());
        fs::set_permissions(&env, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(load_env(&env).is_err());
        fs::set_permissions(&env, fs::Permissions::from_mode(0o600)).unwrap();
        let link = temp.path().join("link.env");
        symlink(&env, &link).unwrap();
        assert!(load_env(&link).is_err());
        fs::create_dir(temp.path().join(".git")).unwrap();
        assert!(load_env(&env).is_err());
    }
    #[cfg(windows)]
    #[test]
    fn native_env_credentials_reject_non_owner_acl() {
        let temp = tempfile::tempdir().unwrap();
        private(temp.path(), true, true).unwrap();
        let env = temp.path().join("secrets.env");
        create_private(&env, b"LAIN_SECRET_ACTOR=fixture-credential").unwrap();
        assert!(load_env(&env).is_ok());
        let output = std::process::Command::new("icacls.exe")
            .arg(&env)
            .args(["/grant", "*S-1-1-0:(R)"])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(load_env(&env).is_err());
    }
}
