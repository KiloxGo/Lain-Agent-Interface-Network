use super::{Store, devices};
use crate::{backend::ControlError, domain::now_ms};
use anyhow::{Result, ensure};
use lain_wire::{
    DeviceId, DeviceWorkspaceId, ProjectId, RequestId, devices::DeviceState, projects::*,
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Serialize, de::DeserializeOwned};

fn decode<T: DeserializeOwned>(raw: String) -> Result<T> {
    serde_json::from_str(&raw).map_err(|_| anyhow::anyhow!("invalid project storage record"))
}
pub(super) fn project(db: &Connection, id: ProjectId) -> Result<Project> {
    let p: Project = decode(
        db.query_row(
            "SELECT snapshot FROM projects WHERE id=?1",
            [id.to_string()],
            |r| r.get(0),
        )
        .optional()?
        .ok_or(ControlError::NotFound)?,
    )?;
    ensure!(
        p.id == id && p.valid(),
        "invalid project storage identity or lifecycle"
    );
    Ok(p)
}
pub(super) fn workspace(
    db: &Connection,
    device: DeviceId,
    workspace: DeviceWorkspaceId,
) -> Result<Option<ProjectWorkspace>> {
    let row: Option<(Option<String>, String)> = db.query_row("SELECT project_id,snapshot FROM project_workspaces WHERE device_id=?1 AND workspace_id=?2", (device.to_string(), workspace.to_string()), |r| Ok((r.get(0)?, r.get(1)?))).optional()?;
    row.map(|(indexed_project, raw)| {
        let m: ProjectWorkspace = decode(raw)?;
        ensure!(
            m.valid()
                && m.device_id == device
                && m.workspace_id == workspace
                && m.project_id.map(|id| id.to_string()) == indexed_project,
            "invalid project mapping storage identity"
        );
        Ok(m)
    })
    .transpose()
}
fn hash(value: &impl Serialize) -> Result<String> {
    devices::safe(value)?;
    Ok(lain_wire::actor_configuration_revision(
        &serde_json::to_vec(value)?,
    ))
}
fn receipt<T: DeserializeOwned>(
    db: &Connection,
    request: RequestId,
    kind: &str,
    target: &str,
    hash: &str,
) -> Result<Option<T>> {
    let r: Option<(String, String, String, String)> = db
        .query_row(
            "SELECT kind,target,payload_hash,snapshot FROM project_requests WHERE request_id=?1",
            [request.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    r.map(|(k, t, h, s)| {
        ensure!(
            k == kind && t == target && h == hash,
            ControlError::Conflict
        );
        decode(s)
    })
    .transpose()
}
fn save(
    tx: &Transaction<'_>,
    request: RequestId,
    kind: &str,
    target: &str,
    hash: &str,
    snapshot: &impl Serialize,
) -> Result<()> {
    tx.execute(
        "INSERT INTO project_requests VALUES(?1,?2,?3,?4,?5)",
        params![
            request.to_string(),
            kind,
            target,
            hash,
            serde_json::to_string(snapshot)?
        ],
    )?;
    Ok(())
}
impl Store {
    pub fn project(&self, id: ProjectId) -> Result<Project> {
        project(&self.connection, id)
    }
    pub fn projects(&self) -> Result<Vec<Project>> {
        let mut q = self
            .connection
            .prepare("SELECT id FROM projects ORDER BY id LIMIT 129")?;
        let ids: Vec<String> = q
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        ensure!(ids.len() <= 128, "project registry exceeds its bound");
        ids.into_iter()
            .map(|id| project(&self.connection, id.parse()?))
            .collect()
    }
    pub fn project_workspace(
        &self,
        device: DeviceId,
        id: DeviceWorkspaceId,
    ) -> Result<Option<ProjectWorkspace>> {
        workspace(&self.connection, device, id)
    }
    pub fn project_create(&mut self, input: CreateProject) -> Result<Project> {
        ensure!(
            !input.id.0.is_nil() && !input.request_id.0.is_nil() && valid_name(&input.name),
            ControlError::InvalidInput
        );
        let digest = hash(&input)?;
        let target = input.id.to_string();
        let tx = self.connection.transaction()?;
        if let Some(p) = receipt::<Project>(&tx, input.request_id, "create", &target, &digest)? {
            ensure!(p.id == input.id && p.valid(), "invalid project receipt");
            return Ok(p);
        }
        ensure!(
            !tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM projects WHERE id=?1)",
                [&target],
                |r| r.get::<_, bool>(0)
            )?,
            ControlError::Conflict
        );
        ensure!(
            tx.query_row("SELECT count(*) FROM projects", [], |r| r.get::<_, i64>(0))? < 128,
            ControlError::Unavailable
        );
        let now = now_ms()?;
        let p = Project {
            id: input.id,
            name: input.name,
            state: ProjectState::Active,
            revision: 1,
            created_at_ms: now,
            updated_at_ms: now,
        };
        tx.execute(
            "INSERT INTO projects VALUES(?1,?2)",
            (&target, serde_json::to_string(&p)?),
        )?;
        save(&tx, input.request_id, "create", &target, &digest, &p)?;
        tx.commit()?;
        Ok(p)
    }
    pub fn project_change(&mut self, id: ProjectId, input: ChangeProject) -> Result<Project> {
        ensure!(
            !id.0.is_nil()
                && !input.request_id.0.is_nil()
                && valid_name(&input.name)
                && input.expected_revision > 0,
            ControlError::InvalidInput
        );
        let digest = hash(&input)?;
        let target = id.to_string();
        let tx = self.connection.transaction()?;
        if let Some(p) = receipt::<Project>(&tx, input.request_id, "change", &target, &digest)? {
            ensure!(p.id == id && p.valid(), "invalid project receipt");
            return Ok(p);
        }
        let mut p = project(&tx, id)?;
        ensure!(
            p.revision == input.expected_revision,
            ControlError::Conflict
        );
        p.revision = p
            .revision
            .checked_add(1)
            .filter(|r| *r <= i64::MAX as u64)
            .ok_or(ControlError::Conflict)?;
        p.name = input.name;
        p.state = input.state;
        p.updated_at_ms = now_ms()?;
        tx.execute(
            "UPDATE projects SET snapshot=?2 WHERE id=?1",
            (&target, serde_json::to_string(&p)?),
        )?;
        save(&tx, input.request_id, "change", &target, &digest, &p)?;
        tx.commit()?;
        Ok(p)
    }
    pub fn set_project_workspace(
        &mut self,
        input: SetProjectWorkspace,
    ) -> Result<ProjectWorkspace> {
        ensure!(
            !input.request_id.0.is_nil()
                && !input.device_id.0.is_nil()
                && !input.workspace_id.0.is_nil()
                && input.expected_device_revision > 0
                && input.expected_revision <= i64::MAX as u64
                && match (input.project_id, input.expected_project_revision) {
                    (Some(id), Some(rev)) => !id.0.is_nil() && rev > 0,
                    (None, None) => true,
                    _ => false,
                },
            ControlError::InvalidInput
        );
        let digest = hash(&input)?;
        let target = format!("{}/{}", input.device_id, input.workspace_id);
        let tx = self.connection.transaction()?;
        if let Some(m) =
            receipt::<ProjectWorkspace>(&tx, input.request_id, "workspace", &target, &digest)?
        {
            ensure!(
                m.valid() && m.device_id == input.device_id && m.workspace_id == input.workspace_id,
                "invalid mapping receipt"
            );
            return Ok(m);
        }
        let d = devices::device(&tx, input.device_id)?;
        ensure!(
            d.revision == input.expected_device_revision,
            ControlError::Conflict
        );
        ensure!(
            d.registration
                .workspaces
                .iter()
                .any(|w| w.id == input.workspace_id),
            ControlError::InvalidInput
        );
        if let Some(id) = input.project_id {
            ensure!(
                d.state == DeviceState::Active && now_ms()? < d.registration.expires_at_ms,
                ControlError::Unavailable
            );
            let p = project(&tx, id)?;
            ensure!(
                Some(p.revision) == input.expected_project_revision,
                ControlError::Conflict
            );
            ensure!(p.state == ProjectState::Active, ControlError::Unavailable);
        }
        let old = workspace(&tx, input.device_id, input.workspace_id)?;
        let revision = old.map_or(0, |m| m.revision);
        ensure!(revision == input.expected_revision, ControlError::Conflict);
        let m = ProjectWorkspace {
            device_id: input.device_id,
            workspace_id: input.workspace_id,
            device_revision: d.revision,
            project_id: input.project_id,
            reviewed_project_revision: input.expected_project_revision,
            revision: revision
                .checked_add(1)
                .filter(|r| *r <= i64::MAX as u64)
                .ok_or(ControlError::Conflict)?,
            request_id: input.request_id,
            decided_at_ms: now_ms()?,
        };
        tx.execute("INSERT INTO project_workspaces VALUES(?1,?2,?3,?4) ON CONFLICT(device_id,workspace_id) DO UPDATE SET project_id=excluded.project_id,snapshot=excluded.snapshot", params![m.device_id.to_string(),m.workspace_id.to_string(),m.project_id.map(|p| p.to_string()),serde_json::to_string(&m)?])?;
        save(&tx, input.request_id, "workspace", &target, &digest, &m)?;
        tx.commit()?;
        Ok(m)
    }
}
