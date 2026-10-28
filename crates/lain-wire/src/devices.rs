//! Device declarations and receipts are claims, never Worker authority.
use crate::{DeviceId, DeviceWorkspaceId, RequestId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevicePlatform {
    Windows,
    Macos,
    Linux,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceCapability {
    CodexVisibleArchive,
    CodexManagedWork,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceWorkspace {
    pub id: DeviceWorkspaceId,
    pub path: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterDevice {
    pub id: DeviceId,
    pub request_id: RequestId,
    pub label: String,
    pub platform: DevicePlatform,
    pub credential_fingerprint: String,
    pub workspaces: Vec<DeviceWorkspace>,
    pub capabilities: Vec<DeviceCapability>,
    pub expires_at_ms: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceState {
    Pending,
    Active,
    Revoked,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    pub registration: RegisterDevice,
    pub state: DeviceState,
    pub revision: u64,
    pub registered_at_ms: u64,
    pub claim_until_ms: u64,
    pub claimed_at_ms: Option<u64>,
    pub claim_request_id: Option<RequestId>,
    pub revoked_at_ms: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceCapabilityReport {
    pub capability: DeviceCapability,
    pub version: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportDevice {
    pub request_id: RequestId,
    pub sequence: u64,
    pub reported_at_ms: u64,
    pub available_workspaces: Vec<DeviceWorkspaceId>,
    pub capabilities: Vec<DeviceCapabilityReport>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceReport {
    pub device_id: DeviceId,
    pub device_revision: u64,
    pub received_at_ms: u64,
    pub report: ReportDevice,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceSnapshot {
    pub device: Device,
    pub expired: bool,
    pub latest_report: Option<DeviceReport>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimDevice {
    pub request_id: RequestId,
    pub registration_fingerprint: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevokeDevice {
    pub expected_revision: u64,
}

/// Verifier for an OS-random 256-bit device credential; not a password KDF.
/// Never pass the returned fingerprint as bearer proof or log the input.
pub fn device_credential_fingerprint(token: &str) -> Option<String> {
    let key = token.strip_prefix("lain-device-")?;
    if key.len() != 64
        || !key
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    let mut digest = Sha256::new();
    digest.update(b"lain-device-credential-v1\0");
    digest.update(token.as_bytes());
    Some(format!("{:x}", digest.finalize()))
}

/// Bind device proof to the exact non-secret manifest reviewed on both ends.
pub fn registration_fingerprint(registration: &RegisterDevice) -> String {
    let mut digest = Sha256::new();
    digest.update(b"lain-device-registration-v1\0");
    digest.update(serde_json::to_vec(registration).expect("typed registration serialization"));
    format!("{:x}", digest.finalize())
}

impl RegisterDevice {
    pub fn valid(&self) -> bool {
        let mut ids = std::collections::BTreeSet::new();
        let mut paths = std::collections::BTreeSet::new();
        let capabilities: std::collections::BTreeSet<_> = self.capabilities.iter().collect();
        !self.label.trim().is_empty()
            && self.label.len() <= 128
            && !self.label.chars().any(char::is_control)
            && self.credential_fingerprint.len() == 64
            && self
                .credential_fingerprint
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            && (1..=8).contains(&self.workspaces.len())
            && capabilities.len() == self.capabilities.len()
            && capabilities.len() <= 4
            && self.workspaces.iter().all(|w| {
                ids.insert(w.id)
                    && paths.insert(w.path.clone())
                    && !w.path.trim().is_empty()
                    && w.path.len() <= 2048
                    && !w.path.chars().any(char::is_control)
                    && match self.platform {
                        DevicePlatform::Windows => {
                            let b = w.path.as_bytes();
                            (b.len() >= 3
                                && b[0].is_ascii_alphabetic()
                                && b[1] == b':'
                                && matches!(b[2], b'\\' | b'/'))
                                || w.path.starts_with("\\\\")
                        }
                        _ => w.path.starts_with('/'),
                    }
            })
    }
}
