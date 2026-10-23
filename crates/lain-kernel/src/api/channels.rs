use super::*;
use crate::channels::domain::{Binding, ChannelId, ChannelSnapshot};

pub(super) fn routes() -> Router<ApiState> {
    Router::new()
        .route("/v1/channels", get(bindings))
        .route("/v1/channels/{id}", get(snapshot))
        .route("/v1/channels/{id}/reply-grant", post(grant))
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
