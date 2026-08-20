use std::{
    env, fmt,
    net::SocketAddr,
    path::{Path, PathBuf},
    time::Duration,
};

use thiserror::Error;

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:3000";
const DEFAULT_IDENTITY_DIR: &str = "identity/v1";
const DEFAULT_BASE_URL: &str = "https://api.deepseek.com";
const DEFAULT_MODEL: &str = "deepseek-v4-flash";
const DEFAULT_PRO_MODEL: &str = "deepseek-v4-pro";
const DEFAULT_MAX_OUTPUT_TOKENS: u32 = 4096;
const DEFAULT_SHUTDOWN_TIMEOUT_SECS: u64 = 10;
const DEFAULT_MAX_MESSAGE_BYTES: usize = 64 * 1024;
const DEFAULT_MEMORY_POLL_SECS: u64 = 2;
const DEFAULT_MEMORY_LEASE_SECS: u64 = 60;
const DEFAULT_MEMORY_MAX_CANDIDATES: usize = 8;

#[derive(Clone)]
pub struct ProviderConfig {
    api_key: Option<String>,
    pub base_url: String,
    pub model: String,
    pub pro_model: String,
    pub request_timeout: Duration,
    pub max_output_tokens: u32,
}

impl ProviderConfig {
    pub fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }

    pub fn is_configured(&self) -> bool {
        self.api_key.is_some()
    }

    #[cfg(test)]
    pub(crate) fn for_test() -> Self {
        Self {
            api_key: None,
            base_url: DEFAULT_BASE_URL.to_string(),
            model: DEFAULT_MODEL.to_string(),
            pro_model: DEFAULT_PRO_MODEL.to_string(),
            request_timeout: Duration::from_secs(1),
            max_output_tokens: DEFAULT_MAX_OUTPUT_TOKENS,
        }
    }

    #[cfg(test)]
    pub(crate) fn for_test_endpoint(base_url: String, request_timeout: Duration) -> Self {
        Self {
            api_key: Some("synthetic-provider-key".to_string()),
            base_url,
            model: DEFAULT_MODEL.to_string(),
            pro_model: DEFAULT_PRO_MODEL.to_string(),
            request_timeout,
            max_output_tokens: DEFAULT_MAX_OUTPUT_TOKENS,
        }
    }
}

impl fmt::Debug for ProviderConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderConfig")
            .field("api_key", &self.api_key.as_ref().map(|_| "[REDACTED]"))
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("pro_model", &self.pro_model)
            .field("request_timeout", &self.request_timeout)
            .field("max_output_tokens", &self.max_output_tokens)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub system_prompt_path: PathBuf,
    pub identity_dir: PathBuf,
    pub data_dir: PathBuf,
    pub provider: ProviderConfig,
    pub shutdown_timeout: Duration,
    pub max_message_bytes: usize,
    pub memory: MemoryConfig,
}

#[derive(Debug, Clone)]
pub struct MemoryConfig {
    pub extraction_enabled: bool,
    pub poll_interval: Duration,
    pub lease_duration: Duration,
    pub max_candidates_per_message: usize,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("cannot determine current directory: {0}")]
    CurrentDirectory(#[source] std::io::Error),
    #[error("invalid socket address in {name}: {value}")]
    InvalidSocketAddress { name: &'static str, value: String },
    #[error("invalid URL in {name}: {value}")]
    InvalidUrl { name: &'static str, value: String },
    #[error("{name} must be a positive integer, got {value}")]
    InvalidPositiveInteger { name: &'static str, value: String },
    #[error("{name} must be true or false, got {value}")]
    InvalidBoolean { name: &'static str, value: String },
    #[error("{name} cannot be empty")]
    EmptyModel { name: &'static str },
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let current_dir = env::current_dir().map_err(ConfigError::CurrentDirectory)?;
        Self::from_source(&current_dir, |name| env::var(name).ok())
    }

    fn from_source<F>(current_dir: &Path, get: F) -> Result<Self, ConfigError>
    where
        F: Fn(&str) -> Option<String>,
    {
        let bind_value = get("LAIN_BIND_ADDR").unwrap_or_else(|| DEFAULT_BIND_ADDR.to_string());
        let bind_addr =
            bind_value
                .parse::<SocketAddr>()
                .map_err(|_| ConfigError::InvalidSocketAddress {
                    name: "LAIN_BIND_ADDR",
                    value: bind_value,
                })?;

        let prompt_value = get("LAIN_SYSTEM_PROMPT")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("prompts/lain_system.md"));
        let system_prompt_path = absolute_path(current_dir, prompt_value);
        let identity_value = get("LAIN_IDENTITY_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_IDENTITY_DIR));
        let identity_dir = absolute_path(current_dir, identity_value);

        let data_value = get("LAIN_DATA_DIR")
            .map(PathBuf::from)
            .or_else(|| dirs::home_dir().map(|home| home.join(".lain")))
            .unwrap_or_else(|| current_dir.join(".lain"));
        let data_dir = absolute_path(current_dir, data_value);

        let base_url = get("DEEPSEEK_BASE_URL").unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let parsed_url = reqwest::Url::parse(&base_url).map_err(|_| ConfigError::InvalidUrl {
            name: "DEEPSEEK_BASE_URL",
            value: base_url.clone(),
        })?;
        if !matches!(parsed_url.scheme(), "http" | "https") {
            return Err(ConfigError::InvalidUrl {
                name: "DEEPSEEK_BASE_URL",
                value: base_url,
            });
        }

        let model = get("DEEPSEEK_MODEL").unwrap_or_else(|| DEFAULT_MODEL.to_string());
        if model.trim().is_empty() {
            return Err(ConfigError::EmptyModel {
                name: "DEEPSEEK_MODEL",
            });
        }
        let pro_model = get("DEEPSEEK_PRO_MODEL").unwrap_or_else(|| DEFAULT_PRO_MODEL.to_string());
        if pro_model.trim().is_empty() {
            return Err(ConfigError::EmptyModel {
                name: "DEEPSEEK_PRO_MODEL",
            });
        }

        let shutdown_timeout_secs = parse_positive_u64(
            "LAIN_SHUTDOWN_TIMEOUT_SECS",
            get("LAIN_SHUTDOWN_TIMEOUT_SECS"),
            DEFAULT_SHUTDOWN_TIMEOUT_SECS,
        )?;
        let request_timeout_secs =
            parse_positive_u64("DEEPSEEK_TIMEOUT_SECS", get("DEEPSEEK_TIMEOUT_SECS"), 180)?;
        let max_output_tokens = parse_positive_u32(
            "DEEPSEEK_MAX_OUTPUT_TOKENS",
            get("DEEPSEEK_MAX_OUTPUT_TOKENS"),
            DEFAULT_MAX_OUTPUT_TOKENS,
        )?;
        let max_message_bytes = parse_positive_usize(
            "LAIN_MAX_MESSAGE_BYTES",
            get("LAIN_MAX_MESSAGE_BYTES"),
            DEFAULT_MAX_MESSAGE_BYTES,
        )?;
        let memory_extraction_enabled = parse_bool(
            "LAIN_MEMORY_EXTRACTION_ENABLED",
            get("LAIN_MEMORY_EXTRACTION_ENABLED"),
            false,
        )?;
        let memory_poll_secs = parse_positive_u64(
            "LAIN_MEMORY_POLL_SECS",
            get("LAIN_MEMORY_POLL_SECS"),
            DEFAULT_MEMORY_POLL_SECS,
        )?;
        let memory_lease_secs = parse_positive_u64(
            "LAIN_MEMORY_LEASE_SECS",
            get("LAIN_MEMORY_LEASE_SECS"),
            DEFAULT_MEMORY_LEASE_SECS,
        )?;
        let memory_max_candidates = parse_positive_usize(
            "LAIN_MEMORY_MAX_CANDIDATES",
            get("LAIN_MEMORY_MAX_CANDIDATES"),
            DEFAULT_MEMORY_MAX_CANDIDATES,
        )?;

        Ok(Self {
            bind_addr,
            system_prompt_path,
            identity_dir,
            data_dir,
            provider: ProviderConfig {
                api_key: get("DEEPSEEK_API_KEY").filter(|value| !value.trim().is_empty()),
                base_url: parsed_url.as_str().trim_end_matches('/').to_string(),
                model,
                pro_model,
                request_timeout: Duration::from_secs(request_timeout_secs),
                max_output_tokens,
            },
            shutdown_timeout: Duration::from_secs(shutdown_timeout_secs),
            max_message_bytes,
            memory: MemoryConfig {
                extraction_enabled: memory_extraction_enabled,
                poll_interval: Duration::from_secs(memory_poll_secs),
                lease_duration: Duration::from_secs(memory_lease_secs),
                max_candidates_per_message: memory_max_candidates,
            },
        })
    }
}

fn absolute_path(current_dir: &Path, value: PathBuf) -> PathBuf {
    if value.is_absolute() {
        value
    } else {
        current_dir.join(value)
    }
}

fn parse_positive_u64(
    name: &'static str,
    value: Option<String>,
    default: u64,
) -> Result<u64, ConfigError> {
    let Some(value) = value else {
        return Ok(default);
    };
    value
        .parse::<u64>()
        .ok()
        .filter(|parsed| *parsed > 0)
        .ok_or(ConfigError::InvalidPositiveInteger { name, value })
}

fn parse_positive_usize(
    name: &'static str,
    value: Option<String>,
    default: usize,
) -> Result<usize, ConfigError> {
    let Some(value) = value else {
        return Ok(default);
    };
    value
        .parse::<usize>()
        .ok()
        .filter(|parsed| *parsed > 0)
        .ok_or(ConfigError::InvalidPositiveInteger { name, value })
}

fn parse_positive_u32(
    name: &'static str,
    value: Option<String>,
    default: u32,
) -> Result<u32, ConfigError> {
    let Some(value) = value else {
        return Ok(default);
    };
    value
        .parse::<u32>()
        .ok()
        .filter(|parsed| *parsed > 0)
        .ok_or(ConfigError::InvalidPositiveInteger { name, value })
}

fn parse_bool(
    name: &'static str,
    value: Option<String>,
    default: bool,
) -> Result<bool, ConfigError> {
    let Some(value) = value else {
        return Ok(default);
    };
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(ConfigError::InvalidBoolean { name, value }),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn config_from(values: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let values: HashMap<String, String> = values
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect();
        let fixture_root = std::env::temp_dir().join("lain-kernel-config-tests");
        Config::from_source(&fixture_root, |name| values.get(name).cloned())
    }

    #[test]
    fn defaults_are_local_and_absolute() {
        let config = config_from(&[("LAIN_DATA_DIR", "fixtures/data")]).expect("valid config");
        assert_eq!(
            config.bind_addr,
            "127.0.0.1:3000".parse().expect("valid address")
        );
        assert!(config.system_prompt_path.is_absolute());
        assert!(config.identity_dir.is_absolute());
        assert!(config.data_dir.is_absolute());
        assert!(!config.provider.is_configured());
        assert!(!config.memory.extraction_enabled);
    }

    #[test]
    fn debug_output_redacts_api_key() {
        let config =
            config_from(&[("DEEPSEEK_API_KEY", "synthetic-secret-value")]).expect("valid config");
        let debug = format!("{:?}", config.provider);
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("synthetic-secret-value"));
    }

    #[test]
    fn rejects_invalid_bind_address() {
        let error = config_from(&[("LAIN_BIND_ADDR", "not-an-address")])
            .expect_err("invalid address must fail");
        assert!(matches!(error, ConfigError::InvalidSocketAddress { .. }));
    }

    #[test]
    fn rejects_non_http_provider_url() {
        let error = config_from(&[("DEEPSEEK_BASE_URL", "file:///tmp/provider")])
            .expect_err("non-http URL must fail");
        assert!(matches!(error, ConfigError::InvalidUrl { .. }));
    }

    #[test]
    fn memory_worker_is_explicitly_opt_in() {
        let enabled =
            config_from(&[("LAIN_MEMORY_EXTRACTION_ENABLED", "true")]).expect("valid config");
        assert!(enabled.memory.extraction_enabled);
        let error = config_from(&[("LAIN_MEMORY_EXTRACTION_ENABLED", "sometimes")])
            .expect_err("invalid boolean must fail");
        assert!(matches!(error, ConfigError::InvalidBoolean { .. }));
    }
}
