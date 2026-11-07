use super::*;
use crate::backend::ControlError;
use lain_wire::{DeviceId, DeviceWorkspaceId, ProjectId, projects::*};

impl Kernel {
    pub(crate) fn projects(&self) -> Result<Vec<Project>> {
        self.lock()?.store.projects()
    }
    pub(crate) fn project(&self, id: ProjectId) -> Result<Project> {
        self.lock()?.store.project(id)
    }
    pub(crate) fn project_workspace(
        &self,
        device: DeviceId,
        workspace: DeviceWorkspaceId,
    ) -> Result<Option<ProjectWorkspace>> {
        self.lock()?.store.project_workspace(device, workspace)
    }
    pub(crate) fn project_create(&self, input: CreateProject) -> Result<Project> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner.store.project_create(input)
    }
    pub(crate) fn project_change(&self, id: ProjectId, input: ChangeProject) -> Result<Project> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner.store.project_change(id, input)
    }
    pub(crate) fn set_project_workspace(
        &self,
        input: SetProjectWorkspace,
    ) -> Result<ProjectWorkspace> {
        let mut inner = self.lock()?;
        ensure!(inner.state == KernelState::Ready, ControlError::Unavailable);
        inner.store.set_project_workspace(input)
    }
}
