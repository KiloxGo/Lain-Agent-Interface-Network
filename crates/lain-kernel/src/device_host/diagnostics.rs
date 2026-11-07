//! Explicit read-only prerequisites, deliberately not a host presence protocol.
use super::*;

#[derive(Debug, Serialize)]
pub struct HostCheck {
    pub device_id: DeviceId,
    pub checked_at_ms: u64,
    pub ready_for_explicit_start: bool,
    pub local_authority_expired: bool,
    pub backend_version: Option<String>,
    pub backend_status: &'static str,
    pub cloud_authority: &'static str,
    pub last_inventory_received_at_ms: Option<u64>,
    pub workspaces: Vec<WorkspaceCheck>,
    pub workspace_write: bool,
    pub offline_grace_seconds: u32,
    pub service_availability: &'static str,
    pub provider_access: &'static str,
    /// Argument data, not a shell command with platform-dependent escaping.
    pub start_argv: Vec<String>,
    pub stop_instruction: &'static str,
}
#[derive(Debug, Serialize)]
pub struct WorkspaceCheck {
    pub workspace_id: lain_wire::DeviceWorkspaceId,
    pub canonical_directory_available: bool,
}

pub async fn check(directory: PathBuf) -> Result<HostCheck> {
    let (credential, registration, policy) = settings(&directory)?;
    let checked_at_ms = domain::now_ms()?;
    let workspaces: Vec<_> = registration
        .workspaces
        .iter()
        .map(|w| {
            let p = PathBuf::from(&w.path);
            WorkspaceCheck {
                workspace_id: w.id,
                canonical_directory_available: p.is_dir()
                    && p.canonicalize().ok().as_ref() == Some(&p),
            }
        })
        .collect();
    let version = crate::backend::probe_device_executable(PathBuf::from(&policy.codex_path))
        .await
        .ok();
    let cloud = cloud::Cloud::new(&credential.endpoint, &credential.token, registration.id)?;
    // The transport base is /device-agent/<id>/commands/. This is the existing
    // authenticated own-device GET, without claiming, reporting or mutating it.
    let own = tokio::time::timeout(
        Duration::from_secs(5),
        cloud.get::<DeviceSnapshot>(&format!("../../{}", registration.id)),
    )
    .await;
    let (cloud_authority, received) = match own {
        Ok(Ok(s)) if s.device.registration == registration => {
            let status = if s.expired || s.device.registration.expires_at_ms <= domain::now_ms()? {
                "expired"
            } else {
                match s.device.state {
                    DeviceState::Pending => "not_claimed",
                    DeviceState::Active => "active_registration_not_service_presence",
                    DeviceState::Revoked => "revoked",
                }
            };
            (status, s.latest_report.map(|r| r.received_at_ms))
        }
        Ok(Ok(_)) => ("manifest_mismatch", None),
        Ok(Err(e))
            if e.downcast_ref::<cloud::CloudError>()
                .is_some_and(|e| matches!(e, cloud::CloudError::Denied)) =>
        {
            ("denied", None)
        }
        _ => ("unavailable", None),
    };
    let local_authority_expired = registration.expires_at_ms <= domain::now_ms()?;
    Ok(HostCheck {
        device_id: registration.id,
        checked_at_ms,
        ready_for_explicit_start: !local_authority_expired
            && version.is_some()
            && cloud_authority == "active_registration_not_service_presence"
            && workspaces.iter().all(|w| w.canonical_directory_available),
        local_authority_expired,
        backend_status: if version.is_some() {
            "supported_native_version"
        } else {
            "unavailable_or_unsupported_version"
        },
        backend_version: version,
        cloud_authority,
        last_inventory_received_at_ms: received,
        workspaces,
        workspace_write: policy.workspace_write,
        offline_grace_seconds: policy.offline_grace_seconds,
        service_availability: "not_proven_by_prerequisite_check",
        provider_access: "not_probed",
        start_argv: vec![
            "lain-device-agent".into(),
            "--directory".into(),
            directory.to_string_lossy().into_owned(),
        ],
        stop_instruction: "Foreground owner: Ctrl-C/Ctrl-Break; wait for owned-runtime shutdown. Installed-service lifecycle remains operator-managed.",
    })
}
