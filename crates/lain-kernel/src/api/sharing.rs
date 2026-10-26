use super::*;
use lain_wire::{ChannelId, DeliveryId, ExperienceId, sharing::*};

pub(super) fn routes() -> Router<ApiState> {
    Router::new()
        .route("/v1/channels/{id}/sharing/receipts/search", post(search))
        .route(
            "/v1/channels/{id}/sharing/receipts/{delivery}",
            get(receipt),
        )
        .route("/v1/channels/{id}/sharing/audit", get(audit))
        .route("/v1/channels/{id}/sharing", get(snapshot))
        .route("/v1/channels/{id}/sharing/grant", post(set_grant))
        .route("/v1/channels/{id}/attention", post(attention))
        .route(
            "/v1/channels/{id}/sharing/consider/{experience}",
            post(consider),
        )
}
async fn search(
    State(s): State<ApiState>,
    Path(id): Path<ChannelId>,
    Json(q): Json<ShareQuery>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    s.actor.check_memory_input(&q).map_err(control_error)?;
    let r = s.actor.source_redactor();
    blocking(&s.kernel, move |k| k.share_receipts(id, q, r))
        .await
        .map(Json)
        .map_err(control_error)
}
async fn receipt(
    State(s): State<ApiState>,
    Path((id, d)): Path<(ChannelId, DeliveryId)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let r = s.actor.source_redactor();
    blocking(&s.kernel, move |k| k.share_receipt(id, d, r))
        .await
        .map(Json)
        .map_err(control_error)
}
async fn audit(
    State(s): State<ApiState>,
    Path(id): Path<ChannelId>,
) -> Result<Json<serde_json::Value>, ApiError> {
    blocking(&s.kernel, move |k| k.share_read_audit(id))
        .await
        .map(Json)
        .map_err(control_error)
}
async fn snapshot(
    State(s): State<ApiState>,
    Path(id): Path<ChannelId>,
) -> Result<Json<SharingSnapshot>, ApiError> {
    blocking(&s.kernel, move |k| k.sharing_snapshot(id))
        .await
        .map(Json)
        .map_err(control_error)
}
async fn set_grant(
    State(s): State<ApiState>,
    Path(id): Path<ChannelId>,
    Json(q): Json<SetShareGrant>,
) -> Result<Json<ShareGrant>, ApiError> {
    s.actor.check_memory_input(&q).map_err(control_error)?;
    blocking(&s.kernel, move |k| k.set_share_grant(id, q))
        .await
        .map(Json)
        .map_err(control_error)
}
async fn attention(
    State(s): State<ApiState>,
    Path(id): Path<ChannelId>,
    Json(q): Json<SetAttention>,
) -> Result<Json<Attention>, ApiError> {
    blocking(&s.kernel, move |k| k.set_attention(id, q))
        .await
        .map(Json)
        .map_err(control_error)
}
async fn consider(
    State(s): State<ApiState>,
    Path((id, e)): Path<(ChannelId, ExperienceId)>,
) -> Result<Json<ShareCandidate>, ApiError> {
    blocking(&s.kernel, move |k| k.share_consider(id, e))
        .await
        .map(Json)
        .map_err(control_error)
}
