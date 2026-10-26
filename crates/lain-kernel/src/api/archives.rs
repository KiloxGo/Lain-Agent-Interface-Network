use super::*;
use lain_wire::{ArchiveRecordId, ArchiveSourceId, archives::*};

pub(super) fn routes() -> Router<ApiState> {
    Router::new()
        .route("/v1/archives/sources", get(list).post(register))
        .route("/v1/archives/sources/{id}", get(source))
        .route("/v1/archives/sources/{id}/grant", post(set_grant))
        .route("/v1/archives/sources/{id}/grants", get(grants))
        .route("/v1/archives/sources/{id}/import", post(import))
        .route("/v1/archives/sources/{id}/query", post(query))
        .route("/v1/archives/records/{id}", get(record))
        .route("/v1/archives/records/{id}/edits", get(edits))
        .route("/v1/archives/records/{id}/forget", post(forget))
        .route("/v1/archives/records/{id}/correct", post(correct))
}
fn safe<T: serde::Serialize>(s: &ApiState, value: T) -> Result<Json<T>, ApiError> {
    s.actor.check_memory_input(&value).map_err(control_error)?;
    Ok(Json(value))
}
async fn grants(
    State(s): State<ApiState>,
    Path(id): Path<ArchiveSourceId>,
) -> Result<Json<Vec<ArchiveGrant>>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.archive_grants(id))
            .await
            .map_err(control_error)?,
    )
}
async fn edits(
    State(s): State<ApiState>,
    Path(id): Path<ArchiveRecordId>,
) -> Result<Json<Vec<ArchiveEdit>>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.archive_edits(id))
            .await
            .map_err(control_error)?,
    )
}
async fn list(State(s): State<ApiState>) -> Result<Json<Vec<ArchiveSourceView>>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, Kernel::archive_sources)
            .await
            .map_err(control_error)?,
    )
}
async fn register(
    State(s): State<ApiState>,
    Json(q): Json<ArchiveSourceKey>,
) -> Result<Json<ArchiveSourceView>, ApiError> {
    let r = s.actor.source_redactor();
    safe(
        &s,
        blocking(&s.kernel, move |k| k.archive_register(q, r))
            .await
            .map_err(control_error)?,
    )
}
async fn source(
    State(s): State<ApiState>,
    Path(id): Path<ArchiveSourceId>,
) -> Result<Json<ArchiveSourceView>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.archive_source(id))
            .await
            .map_err(control_error)?,
    )
}
async fn set_grant(
    State(s): State<ApiState>,
    Path(id): Path<ArchiveSourceId>,
    Json(q): Json<SetArchiveGrant>,
) -> Result<Json<ArchiveGrant>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.archive_set_grant(id, q))
            .await
            .map_err(control_error)?,
    )
}
async fn import(
    State(s): State<ApiState>,
    Path(id): Path<ArchiveSourceId>,
    Json(q): Json<ImportArchive>,
) -> Result<Json<ArchiveImportReceipt>, ApiError> {
    let r = s.actor.source_redactor();
    safe(
        &s,
        blocking(&s.kernel, move |k| k.archive_import(id, q, r))
            .await
            .map_err(control_error)?,
    )
}
async fn query(
    State(s): State<ApiState>,
    Path(id): Path<ArchiveSourceId>,
    Json(q): Json<ArchiveQuery>,
) -> Result<Json<ArchivePage>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.archive_query(id, q))
            .await
            .map_err(control_error)?,
    )
}
async fn record(
    State(s): State<ApiState>,
    Path(id): Path<ArchiveRecordId>,
) -> Result<Json<ArchiveRecord>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.archive_record(id))
            .await
            .map_err(control_error)?,
    )
}
async fn forget(
    State(s): State<ApiState>,
    Path(id): Path<ArchiveRecordId>,
    Json(q): Json<ArchiveRecordRevision>,
) -> Result<Json<ArchiveRecord>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.archive_forget(id, q))
            .await
            .map_err(control_error)?,
    )
}
async fn correct(
    State(s): State<ApiState>,
    Path(id): Path<ArchiveRecordId>,
    Json(q): Json<CorrectArchiveRecord>,
) -> Result<Json<ArchiveRecord>, ApiError> {
    let r = s.actor.source_redactor();
    safe(
        &s,
        blocking(&s.kernel, move |k| k.archive_correct(id, q, r))
            .await
            .map_err(control_error)?,
    )
}
