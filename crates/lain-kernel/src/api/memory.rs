use super::*;
use crate::memory::*;
use lain_wire::RequestId;

pub(super) fn routes() -> Router<ApiState> {
    Router::new()
        .route("/v1/memory", post(add))
        .route("/v1/memory/query", post(query))
        .route("/v1/memory/{id}", get(show))
        .route("/v1/memory/{id}/status", post(change))
        .route("/v1/memory/{id}/delete", post(remove))
        .route("/v1/memory-candidates", get(candidates).post(propose))
        .route("/v1/memory-candidates/{id}", get(candidate))
        .route("/v1/memory-candidates/{id}/decision", post(review))
        .route("/v1/session-summaries/{id}", get(summary))
        .route(
            "/v1/work-sessions/{id}/summaries",
            get(summaries).post(create_summary),
        )
}
async fn add(
    State(state): State<ApiState>,
    Json(input): Json<AddMemory>,
) -> Result<Json<MemoryRecord>, ApiError> {
    state
        .actor
        .check_memory_input(&input)
        .map_err(control_error)?;
    let k = state.kernel;
    Ok(Json(
        blocking(&k, move |k| k.add_memory(input))
            .await
            .map_err(control_error)?,
    ))
}
async fn query(
    State(k): State<Arc<Kernel>>,
    Json(q): Json<MemoryQuery>,
) -> Result<Json<MemoryPage>, ApiError> {
    Ok(Json(
        blocking(&k, move |k| k.memory_page(q))
            .await
            .map_err(control_error)?,
    ))
}
async fn show(
    State(k): State<Arc<Kernel>>,
    Path(id): Path<MemoryId>,
) -> Result<Json<MemoryRecord>, ApiError> {
    Ok(Json(
        blocking(&k, move |k| k.memory(id))
            .await
            .map_err(control_error)?,
    ))
}
async fn change(
    State(k): State<Arc<Kernel>>,
    Path(id): Path<MemoryId>,
    Json(input): Json<ChangeMemory>,
) -> Result<Json<MemoryRecord>, ApiError> {
    Ok(Json(
        blocking(&k, move |k| k.change_memory(id, input))
            .await
            .map_err(control_error)?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Revision {
    expected_revision: u64,
}
async fn remove(
    State(k): State<Arc<Kernel>>,
    Path(id): Path<MemoryId>,
    Json(input): Json<Revision>,
) -> Result<Json<serde_json::Value>, ApiError> {
    blocking(&k, move |k| k.delete_memory(id, input.expected_revision))
        .await
        .map_err(control_error)?;
    Ok(Json(serde_json::json!({"deleted":id})))
}
#[derive(Deserialize)]
struct Page {
    #[serde(default)]
    after: u64,
    #[serde(default = "page_limit")]
    limit: u16,
}
fn page_limit() -> u16 {
    50
}
async fn candidates(
    State(k): State<Arc<Kernel>>,
    Query(p): Query<Page>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(
        blocking(&k, move |k| k.memory_candidates(p.after, p.limit))
            .await
            .map_err(control_error)?,
    ))
}
async fn candidate(
    State(k): State<Arc<Kernel>>,
    Path(id): Path<CandidateId>,
) -> Result<Json<MemoryCandidate>, ApiError> {
    Ok(Json(
        blocking(&k, move |k| k.memory_candidate(id))
            .await
            .map_err(control_error)?,
    ))
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SummaryCandidate {
    request_id: RequestId,
    summary_id: SummaryId,
    draft: MemoryDraft,
}
async fn propose(
    State(state): State<ApiState>,
    Json(input): Json<SummaryCandidate>,
) -> Result<Json<MemoryCandidate>, ApiError> {
    state
        .actor
        .check_memory_input(&input)
        .map_err(control_error)?;
    let k = state.kernel;
    Ok(Json(
        blocking(&k, move |k| {
            k.propose_summary_memory(input.request_id, input.summary_id, input.draft)
        })
        .await
        .map_err(control_error)?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Review {
    accept: bool,
}
async fn review(
    State(k): State<Arc<Kernel>>,
    Path(id): Path<CandidateId>,
    Json(input): Json<Review>,
) -> Result<Json<MemoryCandidate>, ApiError> {
    Ok(Json(
        blocking(&k, move |k| k.review_memory_candidate(id, input.accept))
            .await
            .map_err(control_error)?,
    ))
}
async fn summary(
    State(k): State<Arc<Kernel>>,
    Path(id): Path<SummaryId>,
) -> Result<Json<SessionSummary>, ApiError> {
    Ok(Json(
        blocking(&k, move |k| k.session_summary(id))
            .await
            .map_err(control_error)?,
    ))
}
async fn summaries(
    State(k): State<Arc<Kernel>>,
    Path(id): Path<WorkSessionId>,
) -> Result<Json<Vec<SessionSummary>>, ApiError> {
    Ok(Json(
        blocking(&k, move |k| k.session_summaries(id))
            .await
            .map_err(control_error)?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SummaryTurn {
    turn_id: crate::domain::TurnId,
}
async fn create_summary(
    State(k): State<Arc<Kernel>>,
    Path(id): Path<WorkSessionId>,
    Json(input): Json<SummaryTurn>,
) -> Result<Json<SessionSummary>, ApiError> {
    Ok(Json(
        blocking(&k, move |k| k.create_session_summary(id, input.turn_id))
            .await
            .map_err(control_error)?,
    ))
}
