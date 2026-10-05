//! Lain-owned configuration. No protocol objects or resolved secrets in durable types.
use crate::domain::{BackendKind, RuntimeProfileId, SandboxMode, SessionOptions};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use zeroize::Zeroizing;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SecretRef(String);
impl TryFrom<String> for SecretRef {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self> {
        let name = value.strip_prefix("secret://env/").unwrap_or("");
        ensure!(
            !name.is_empty()
                && name.len() <= 64
                && name.split('-').all(|s| !s.is_empty()
                    && s.bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())),
            "invalid namespaced SecretRef"
        );
        Ok(Self(value))
    }
}
impl FromStr for SecretRef {
    type Err = anyhow::Error;
    fn from_str(value: &str) -> Result<Self> {
        value.to_owned().try_into()
    }
}
impl From<SecretRef> for String {
    fn from(value: SecretRef) -> Self {
        value.0
    }
}
impl fmt::Debug for SecretRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl SecretRef {
    pub fn env_name(&self) -> String {
        format!(
            "LAIN_SECRET_{}",
            self.0[13..].replace('-', "_").to_ascii_uppercase()
        )
    }
}
// Not serializable, never included in error messages; buffers are cleared on drop.
pub(crate) struct SecretValue(pub Zeroizing<String>);
impl fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretValue([REDACTED])")
    }
}
pub(crate) fn resolve_secret(reference: &SecretRef) -> Result<SecretValue> {
    let value = std::env::var(reference.env_name()).ok().map(Zeroizing::new);
    ensure!(
        value
            .as_ref()
            .is_some_and(|s| !s.is_empty() && s.len() <= 8192 && !s.chars().any(char::is_control)),
        "profile credential unavailable; set its LAIN_SECRET variable"
    );
    Ok(SecretValue(value.unwrap()))
}
pub fn secret_status(reference: Option<&SecretRef>) -> &'static str {
    match reference {
        None => "not_required",
        Some(r) if resolve_secret(r).is_ok() => "available",
        Some(_) => "missing",
    }
}
pub(crate) fn known_secrets() -> Vec<Zeroizing<String>> {
    std::env::vars_os()
        .filter_map(|(key, value)| {
            let key = key.to_str()?;
            if key == "LAIN_API_TOKEN" || key.starts_with("LAIN_SECRET_") {
                value
                    .into_string()
                    .ok()
                    .filter(|s| !s.is_empty())
                    .map(Zeroizing::new)
            } else {
                None
            }
        })
        .collect()
}
pub(crate) fn check_sensitive(value: &impl Serialize) -> Result<()> {
    let encoded = serde_json::to_string(value)?;
    ensure!(
        !known_secrets().iter().any(|s| {
            let secret = serde_json::to_string(s.as_str()).expect("string encoding");
            encoded.contains(&secret[1..secret.len() - 1])
        }),
        "sensitive profile configuration rejected"
    );
    Ok(())
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalPolicy {
    #[default]
    OnRequest,
    Untrusted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileDefinition {
    pub id: RuntimeProfileId,
    pub backend: BackendKind,
    pub driver: String,
    #[serde(default = "enabled")]
    pub enabled: bool,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub base_url: Option<String>,
    pub reasoning_effort: Option<String>,
    #[serde(default)]
    pub sandbox_default: SandboxMode,
    #[serde(default)]
    pub approval_policy: ApprovalPolicy,
    pub secret_ref: Option<SecretRef>,
}
fn enabled() -> bool {
    true
}
pub fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.to_ascii_lowercase().starts_with("sk-")
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._/".contains(&b))
}
impl ProfileDefinition {
    pub fn local() -> Self {
        Self {
            id: "codex/local".parse().unwrap(),
            backend: "codex".parse().unwrap(),
            driver: "app-server".into(),
            enabled: true,
            model: None,
            provider: None,
            base_url: None,
            reasoning_effort: None,
            sandbox_default: SandboxMode::ReadOnly,
            approval_policy: ApprovalPolicy::OnRequest,
            secret_ref: None,
        }
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.backend.to_string() == "codex" && self.driver == "app-server",
            "unsupported backend or driver"
        );
        ensure!(
            identifier(&self.id.to_string()),
            "invalid profile identifier"
        );
        ensure!(
            self.model.as_deref().is_none_or(identifier)
                && self.provider.as_deref().is_none_or(identifier),
            "invalid model or provider identifier"
        );
        ensure!(
            self.reasoning_effort.as_deref().is_none_or(|s| matches!(
                s,
                "none" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max" | "ultra"
            )),
            "unsupported reasoning effort"
        );
        if let Some(value) = &self.base_url {
            validate_endpoint(value)?;
        }
        ensure!(
            (self.provider.is_none() && self.base_url.is_none() && self.secret_ref.is_none())
                || (self.provider.is_some() && self.secret_ref.is_some()),
            "provider overrides require an explicit provider and SecretRef"
        );
        if self.provider.is_some() {
            ensure!(
                self.model.is_some(),
                "provider profile requires an explicit model"
            );
        }
        Ok(())
    }
}
pub fn validate_endpoint(value: &str) -> Result<()> {
    ensure!(
        value.len() <= 2048
            && !value.chars().any(|c| c.is_control() || c.is_whitespace())
            && !value.contains('\\'),
        "invalid provider endpoint"
    );
    let url = url::Url::parse(value).map_err(|_| anyhow::anyhow!("invalid provider endpoint"))?;
    let local = match url.host() {
        Some(url::Host::Ipv4(a)) => a.is_loopback(),
        Some(url::Host::Ipv6(a)) => a.is_loopback(),
        Some(url::Host::Domain(d)) => d == "localhost",
        None => false,
    };
    ensure!(
        url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
            && (url.scheme() == "https" || (url.scheme() == "http" && local)),
        "endpoint requires HTTPS (HTTP only on loopback), without credentials, query or fragment"
    );
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeProfile {
    #[serde(flatten)]
    pub definition: ProfileDefinition,
    pub revision: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedRuntimeProfile {
    pub profile_id: RuntimeProfileId,
    pub revision: u64,
    pub backend: BackendKind,
    pub driver: String,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub base_url: Option<String>,
    pub reasoning_effort: Option<String>,
    pub sandbox: SandboxMode,
    pub approval_policy: ApprovalPolicy,
    pub secret_ref: Option<SecretRef>,
}
impl RuntimeProfile {
    pub fn resolve(&self, options: &SessionOptions) -> Result<ResolvedRuntimeProfile> {
        let d = &self.definition;
        d.validate()?;
        ensure!(d.enabled, "profile disabled");
        ensure!(
            options.model.as_deref().is_none_or(identifier),
            "invalid model override"
        );
        Ok(ResolvedRuntimeProfile {
            profile_id: d.id.clone(),
            revision: self.revision,
            backend: d.backend.clone(),
            driver: d.driver.clone(),
            model: options.model.clone().or_else(|| d.model.clone()),
            provider: d.provider.clone(),
            base_url: d.base_url.clone().or_else(|| {
                d.provider
                    .as_ref()
                    .map(|_| "https://api.openai.com/v1".into())
            }),
            reasoning_effort: d.reasoning_effort.clone(),
            sandbox: options.sandbox.unwrap_or(d.sandbox_default),
            approval_policy: d.approval_policy,
            secret_ref: d.secret_ref.clone(),
        })
    }
}
impl ResolvedRuntimeProfile {
    pub fn legacy(options: &SessionOptions) -> Result<Self> {
        RuntimeProfile {
            definition: ProfileDefinition::local(),
            revision: 0,
        }
        .resolve(options)
    }
    pub fn validate(&self) -> Result<()> {
        ProfileDefinition {
            id: self.profile_id.clone(),
            backend: self.backend.clone(),
            driver: self.driver.clone(),
            enabled: true,
            model: self.model.clone(),
            provider: self.provider.clone(),
            base_url: self.base_url.clone(),
            reasoning_effort: self.reasoning_effort.clone(),
            sandbox_default: self.sandbox,
            approval_policy: self.approval_policy,
            secret_ref: self.secret_ref.clone(),
        }
        .validate()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspacePolicy {
    pub workspace: std::path::PathBuf,
    pub default_profile: RuntimeProfileId,
    pub allowed_profiles: Vec<RuntimeProfileId>,
}
impl WorkspacePolicy {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.workspace.is_absolute()
                && !self.allowed_profiles.is_empty()
                && self.allowed_profiles.len() <= 64
                && self.allowed_profiles.contains(&self.default_profile),
            "invalid workspace profile policy"
        );
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileHealth {
    Valid,
    Invalid,
    Unavailable,
    Unknown,
}
#[derive(Debug, Clone, Serialize)]
pub struct ProfileView {
    #[serde(flatten)]
    pub profile: RuntimeProfile,
    pub health: ProfileHealth,
    pub explanation: &'static str,
    pub secret_status: &'static str,
    pub is_default: bool,
    pub supported_overrides: Vec<&'static str>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedProfileView {
    #[serde(flatten)]
    pub resolved: ResolvedRuntimeProfile,
    pub secret_status: &'static str,
}
