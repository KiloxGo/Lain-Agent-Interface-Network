//! Stable project identity and explicit assignments, never execution authority.
use crate::{DeviceId, DeviceWorkspaceId, ProjectId, RequestId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectState {
    Active,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateProject {
    pub request_id: RequestId,
    pub id: ProjectId,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeProject {
    pub request_id: RequestId,
    pub expected_revision: u64,
    pub name: String,
    pub state: ProjectState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub state: ProjectState,
    pub revision: u64,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

/// None clears the assignment; the revisioned tombstone is retained.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetProjectWorkspace {
    pub request_id: RequestId,
    pub device_id: DeviceId,
    pub workspace_id: DeviceWorkspaceId,
    pub expected_device_revision: u64,
    pub project_id: Option<ProjectId>,
    pub expected_project_revision: Option<u64>,
    pub expected_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectWorkspace {
    pub device_id: DeviceId,
    pub workspace_id: DeviceWorkspaceId,
    pub device_revision: u64,
    pub project_id: Option<ProjectId>,
    /// Historical reviewed revision, not a claim that the project is unchanged.
    pub reviewed_project_revision: Option<u64>,
    pub revision: u64,
    pub request_id: RequestId,
    pub decided_at_ms: u64,
}

pub fn valid_name(name: &str) -> bool {
    !name.trim().is_empty() && name.len() <= 128 && !name.chars().any(char::is_control)
}
impl Project {
    pub fn valid(&self) -> bool {
        !self.id.0.is_nil()
            && valid_name(&self.name)
            && self.revision > 0
            && self.revision <= i64::MAX as u64
            && self.updated_at_ms >= self.created_at_ms
    }
}
impl ProjectWorkspace {
    pub fn valid(&self) -> bool {
        !self.device_id.0.is_nil()
            && !self.workspace_id.0.is_nil()
            && !self.request_id.0.is_nil()
            && self.device_revision > 0
            && self.revision > 0
            && self.revision <= i64::MAX as u64
            && match (self.project_id, self.reviewed_project_revision) {
                (Some(id), Some(revision)) => {
                    !id.0.is_nil() && revision > 0 && revision <= i64::MAX as u64
                }
                (None, None) => true,
                _ => false,
            }
    }
}
