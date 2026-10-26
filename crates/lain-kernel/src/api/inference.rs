use super::*;
use lain_wire::{InferenceBudgetId, inference::*};
pub(super) fn routes() -> Router<ApiState> {
    Router::new()
        .route("/v1/inference/budget", get(show).post(set))
        .route("/v1/inference/budgets", get(budgets))
        .route("/v1/inference/requests", get(requests))
}
async fn show(State(s): State<ApiState>) -> Result<Json<InferenceSnapshot>, ApiError> {
    blocking(&s.kernel, Kernel::inference_snapshot)
        .await
        .map(Json)
        .map_err(control_error)
}
async fn set(
    State(s): State<ApiState>,
    Json(q): Json<SetInferenceBudget>,
) -> Result<Json<InferenceDecision>, ApiError> {
    blocking(&s.kernel, move |k| k.set_inference_budget(q))
        .await
        .map(Json)
        .map_err(control_error)
}
async fn budgets(State(s): State<ApiState>) -> Result<Json<Vec<InferenceBudgetStatus>>, ApiError> {
    blocking(&s.kernel, Kernel::inference_budgets)
        .await
        .map(Json)
        .map_err(control_error)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Requests {
    budget: Option<InferenceBudgetId>,
}
async fn requests(
    State(s): State<ApiState>,
    Query(q): Query<Requests>,
) -> Result<Json<Vec<InferenceRequest>>, ApiError> {
    blocking(&s.kernel, move |k| k.inference_requests(q.budget))
        .await
        .map(Json)
        .map_err(control_error)
}
