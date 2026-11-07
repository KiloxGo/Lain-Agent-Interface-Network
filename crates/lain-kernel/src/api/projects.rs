use super::archives::safe;
use super::*;
use lain_wire::{DeviceId, DeviceWorkspaceId, ProjectId, projects::*};

pub(super) fn routes() -> Router<ApiState> {
    Router::new()
        .route("/v1/projects", get(list).post(create))
        .route("/v1/projects/{id}", get(show))
        .route("/v1/projects/{id}/change", post(change))
        .route("/v1/project-workspaces", post(assign))
        .route(
            "/v1/devices/{device}/workspaces/{workspace}/project",
            get(workspace),
        )
}
async fn list(State(s): State<ApiState>) -> Result<Json<Vec<Project>>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, Kernel::projects)
            .await
            .map_err(control_error)?,
    )
}
async fn show(
    State(s): State<ApiState>,
    Path(id): Path<ProjectId>,
) -> Result<Json<Project>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.project(id))
            .await
            .map_err(control_error)?,
    )
}
async fn create(
    State(s): State<ApiState>,
    Json(input): Json<CreateProject>,
) -> Result<Json<Project>, ApiError> {
    let Json(input) = safe(&s, input)?;
    safe(
        &s,
        blocking(&s.kernel, move |k| k.project_create(input))
            .await
            .map_err(control_error)?,
    )
}
async fn change(
    State(s): State<ApiState>,
    Path(id): Path<ProjectId>,
    Json(input): Json<ChangeProject>,
) -> Result<Json<Project>, ApiError> {
    let Json(input) = safe(&s, input)?;
    safe(
        &s,
        blocking(&s.kernel, move |k| k.project_change(id, input))
            .await
            .map_err(control_error)?,
    )
}
async fn workspace(
    State(s): State<ApiState>,
    Path((device, workspace)): Path<(DeviceId, DeviceWorkspaceId)>,
) -> Result<Json<Option<ProjectWorkspace>>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.project_workspace(device, workspace))
            .await
            .map_err(control_error)?,
    )
}
async fn assign(
    State(s): State<ApiState>,
    Json(input): Json<SetProjectWorkspace>,
) -> Result<Json<ProjectWorkspace>, ApiError> {
    let Json(input) = safe(&s, input)?;
    safe(
        &s,
        blocking(&s.kernel, move |k| k.set_project_workspace(input))
            .await
            .map_err(control_error)?,
    )
}
