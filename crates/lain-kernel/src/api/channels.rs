use super::*;
use crate::channels::domain::{Binding, ChannelId, ChannelSnapshot, ChannelStatus};
use lain_wire::source::*;

pub(super) fn routes() -> Router<ApiState> {
    Router::new()
        .route("/v1/channels", get(bindings))
        .route("/v1/channels/status", get(statuses))
        .route("/v1/channels/{id}", get(snapshot))
        .route("/v1/channels/{id}/reply-grant", post(grant))
        .route(
            "/v1/channels/{id}/memory-grant",
            get(memory_grant).post(set_memory_grant),
        )
        .route(
            "/v1/channels/{id}/memory-grant/history",
            get(memory_grant_history),
        )
        .route("/v1/channels/{id}/memory-effects", get(memory_effects))
        .route(
            "/v1/channels/{id}/source-grant",
            get(source_grant).post(set_source_grant),
        )
        .route("/v1/channels/{id}/source-grant/history", get(source_grants))
        .route("/v1/channels/{id}/sources/search", post(source_search))
        .route("/v1/channels/{id}/sources/inspect", post(source_inspect))
        .route("/v1/channels/{id}/source-reads", get(source_reads))
}
async fn source_grant(
    State(state): State<ApiState>,
    Path(id): Path<ChannelId>,
) -> Result<Json<SourceGrant>, ApiError> {
    blocking(&state.kernel, move |k| k.source_grant(id))
        .await
        .map(Json)
        .map_err(control_error)
}
async fn set_source_grant(
    State(state): State<ApiState>,
    Path(id): Path<ChannelId>,
    Json(change): Json<SetSourceGrant>,
) -> Result<Json<SourceGrant>, ApiError> {
    blocking(&state.kernel, move |k| k.set_source_grant(id, change))
        .await
        .map(Json)
        .map_err(control_error)
}
async fn source_grants(
    State(state): State<ApiState>,
    Path(id): Path<ChannelId>,
) -> Result<Json<Vec<SourceGrant>>, ApiError> {
    blocking(&state.kernel, move |k| k.source_grants(id))
        .await
        .map(Json)
        .map_err(control_error)
}
async fn source_search(
    State(state): State<ApiState>,
    Path(id): Path<ChannelId>,
    Json(query): Json<SourceQuery>,
) -> Result<Json<SourcePage>, ApiError> {
    state
        .actor
        .check_memory_input(&query)
        .map_err(control_error)?;
    let redactor = state.actor.source_redactor();
    blocking(&state.kernel, move |k| k.source_search(id, query, redactor))
        .await
        .map(Json)
        .map_err(control_error)
}
async fn source_inspect(
    State(state): State<ApiState>,
    Path(id): Path<ChannelId>,
    Json(query): Json<InspectSource>,
) -> Result<Json<SourceRecord>, ApiError> {
    let redactor = state.actor.source_redactor();
    blocking(&state.kernel, move |k| {
        k.source_inspect(id, query, redactor)
    })
    .await
    .map(Json)
    .map_err(control_error)
}
async fn source_reads(
    State(state): State<ApiState>,
    Path(id): Path<ChannelId>,
    Query(query): Query<EffectQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    blocking(&state.kernel, move |k| k.source_reads(id, query.after))
        .await
        .map(Json)
        .map_err(control_error)
}
async fn memory_grant(
    State(state): State<ApiState>,
    Path(id): Path<ChannelId>,
) -> Result<Json<crate::memory::MemoryGrant>, ApiError> {
    blocking(&state.kernel, move |k| k.memory_grant(id))
        .await
        .map(Json)
        .map_err(control_error)
}
async fn set_memory_grant(
    State(state): State<ApiState>,
    Path(id): Path<ChannelId>,
    Json(change): Json<crate::memory::SetMemoryGrant>,
) -> Result<Json<crate::memory::MemoryGrant>, ApiError> {
    blocking(&state.kernel, move |k| k.set_memory_grant(id, change))
        .await
        .map(Json)
        .map_err(control_error)
}
async fn memory_grant_history(
    State(state): State<ApiState>,
    Path(id): Path<ChannelId>,
) -> Result<Json<Vec<crate::memory::MemoryGrant>>, ApiError> {
    blocking(&state.kernel, move |k| k.memory_grant_history(id))
        .await
        .map(Json)
        .map_err(control_error)
}
#[derive(Deserialize, Default)]
struct EffectQuery {
    #[serde(default)]
    after: u64,
}
async fn memory_effects(
    State(state): State<ApiState>,
    Path(id): Path<ChannelId>,
    Query(query): Query<EffectQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    blocking(&state.kernel, move |k| k.memory_effects(id, query.after))
        .await
        .map(Json)
        .map_err(control_error)
}
async fn statuses(State(state): State<ApiState>) -> Result<Json<Vec<ChannelStatus>>, ApiError> {
    blocking(&state.kernel, Kernel::channel_statuses)
        .await
        .map(Json)
        .map_err(control_error)
}
async fn bindings(State(state): State<ApiState>) -> Result<Json<Vec<Binding>>, ApiError> {
    blocking(&state.kernel, Kernel::channel_bindings)
        .await
        .map(Json)
        .map_err(control_error)
}
async fn snapshot(
    State(state): State<ApiState>,
    Path(id): Path<ChannelId>,
) -> Result<Json<ChannelSnapshot>, ApiError> {
    blocking(&state.kernel, move |k| k.channel_snapshot(id))
        .await
        .map(Json)
        .map_err(control_error)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Grant {
    enabled: bool,
    expected_revision: u64,
}
async fn grant(
    State(state): State<ApiState>,
    Path(id): Path<ChannelId>,
    Json(input): Json<Grant>,
) -> Result<Json<Binding>, ApiError> {
    blocking(&state.kernel, move |k| {
        k.channel_grant(id, input.enabled, input.expected_revision)
    })
    .await
    .map(Json)
    .map_err(control_error)
}
