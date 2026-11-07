use super::archives::safe;
use super::*;
use lain_wire::{ConversationId, memory_context::*};

pub(super) fn routes() -> Router<ApiState> {
    Router::new().route(
        "/v1/conversations/{id}/memory-context",
        get(show).post(select),
    )
}
async fn show(
    State(s): State<ApiState>,
    Path(id): Path<ConversationId>,
) -> Result<Json<Option<ConversationMemoryContext>>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.memory_context(id))
            .await
            .map_err(control_error)?,
    )
}
async fn select(
    State(s): State<ApiState>,
    Path(id): Path<ConversationId>,
    Json(input): Json<SetMemoryContext>,
) -> Result<Json<ConversationMemoryContext>, ApiError> {
    let Json(input) = safe(&s, input)?;
    safe(
        &s,
        blocking(&s.kernel, move |k| k.set_memory_context(id, input))
            .await
            .map_err(control_error)?,
    )
}
