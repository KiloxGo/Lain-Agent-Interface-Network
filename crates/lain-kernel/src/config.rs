use std::{
    ffi::OsString,
    fmt,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    str::FromStr,
};

use subtle::ConstantTimeEq;
use thiserror::Error;

/// Used for API authentication and output redaction; never serialized or persisted.
#[derive(Clone)]
pub struct ApiToken(String);

impl FromStr for ApiToken {
    type Err = ConfigError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if !(32..=256).contains(&value.len())
            || value.starts_with("lain-device-")
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        {
            return Err(ConfigError::Invalid(
                "LAIN_API_TOKEN (32–256 letters, digits, '-' or '_')",
            ));
        }
        Ok(Self(value.to_owned()))
    }
}

#[cfg(test)]
#[test]
fn device_credentials_cannot_be_configured_as_kernel_administrator_tokens() {
    let device = format!("lain-device-{}", "ab".repeat(32));
    assert!(lain_wire::devices::device_credential_fingerprint(&device).is_some());
    assert!(device.parse::<ApiToken>().is_err());
}

impl fmt::Debug for ApiToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiToken([REDACTED])")
    }
}

impl ApiToken {
    pub(crate) fn redaction_values(&self) -> Vec<String> {
        vec![self.0.clone()]
    }
    pub(crate) fn matches(&self, candidate: &str) -> bool {
        bool::from(self.0.as_bytes().ct_eq(candidate.as_bytes()))
    }
}

#[derive(Debug)]
pub struct Config {
    pub database_path: PathBuf,
    pub listen: SocketAddr,
    pub api_token: ApiToken,
    pub log_level: tracing::Level,
    pub codex_path: Option<PathBuf>,
    pub worker_idle_timeout: std::time::Duration,
    pub web_dir: Option<PathBuf>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required setting {0}")]
    Missing(&'static str),
    #[error("invalid setting {0}")]
    Invalid(&'static str),
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| std::env::var_os(key))
    }

    pub fn from_lookup(lookup: impl Fn(&str) -> Option<OsString>) -> Result<Self, ConfigError> {
        let text = |key| {
            lookup(key)
                .map(|v| v.into_string().map_err(|_| ConfigError::Invalid(key)))
                .transpose()
        };
        let database_path = lookup("LAIN_DATABASE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("data/lain.sqlite3"));
        if database_path.as_os_str().is_empty()
            || database_path == std::path::Path::new(":memory:")
            || database_path.file_name().is_none()
        {
            return Err(ConfigError::Invalid("LAIN_DATABASE_PATH"));
        }
        let listen = text("LAIN_LISTEN")?
            .map(|v| v.parse().map_err(|_| ConfigError::Invalid("LAIN_LISTEN")))
            .transpose()?
            .unwrap_or(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8787));
        if !listen.ip().is_loopback() {
            return Err(ConfigError::Invalid("LAIN_LISTEN (requires loopback)"));
        }
        let api_token = text("LAIN_API_TOKEN")?
            .ok_or(ConfigError::Missing("LAIN_API_TOKEN"))?
            .parse()?;
        let log_level = text("LAIN_LOG")?
            .unwrap_or_else(|| "info".to_owned())
            .parse()
            .map_err(|_| ConfigError::Invalid("LAIN_LOG"))?;
        let idle_seconds = text("LAIN_WORKER_IDLE_TIMEOUT")?
            .map(|v| v.parse::<u64>())
            .transpose()
            .map_err(|_| ConfigError::Invalid("LAIN_WORKER_IDLE_TIMEOUT"))?
            .unwrap_or(300);
        if !(1..=86400).contains(&idle_seconds) {
            return Err(ConfigError::Invalid(
                "LAIN_WORKER_IDLE_TIMEOUT (1–86400 seconds)",
            ));
        }
        Ok(Self {
            database_path,
            listen,
            api_token,
            log_level,
            codex_path: lookup("LAIN_CODEX_PATH").map(PathBuf::from),
            worker_idle_timeout: std::time::Duration::from_secs(idle_seconds),
            web_dir: lookup("LAIN_WEB_DIR")
                .map(|value| {
                    let path = PathBuf::from(value)
                        .canonicalize()
                        .map_err(|_| ConfigError::Invalid("LAIN_WEB_DIR"))?;
                    if !path.is_dir() || !path.join("index.html").is_file() {
                        return Err(ConfigError::Invalid(
                            "LAIN_WEB_DIR (requires built index.html)",
                        ));
                    }
                    Ok(path)
                })
                .transpose()?,
        })
    }
}
