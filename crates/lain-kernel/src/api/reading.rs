use super::*;
use lain_wire::{ReadingId, reading::*};

pub(super) fn routes() -> Router<ApiState> {
    Router::new()
        .route("/v1/reading/grant", get(grant).post(set_grant))
        .route("/v1/reading/grant/history", get(grants))
        .route("/v1/reading/activities", get(list).post(start))
        .route("/v1/reading/activities/{id}", get(inspect))
        .route("/v1/reading/activities/{id}/cancel", post(cancel))
}
async fn grant(State(s): State<ApiState>) -> Result<Json<ReadingGrant>, ApiError> {
    blocking(&s.kernel, Kernel::reading_grant)
        .await
        .map(Json)
        .map_err(control_error)
}
async fn set_grant(
    State(s): State<ApiState>,
    Json(change): Json<SetReadingGrant>,
) -> Result<Json<ReadingGrant>, ApiError> {
    s.actor.check_memory_input(&change).map_err(control_error)?;
    blocking(&s.kernel, move |k| k.set_reading_grant(change))
        .await
        .map(Json)
        .map_err(control_error)
}
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct GrantQuery {
    #[serde(default)]
    after: u64,
}
async fn grants(
    State(s): State<ApiState>,
    Query(q): Query<GrantQuery>,
) -> Result<Json<Vec<ReadingGrant>>, ApiError> {
    blocking(&s.kernel, move |k| k.reading_grants(q.after))
        .await
        .map(Json)
        .map_err(control_error)
}
async fn list(State(s): State<ApiState>) -> Result<Json<Vec<ReadingActivity>>, ApiError> {
    blocking(&s.kernel, Kernel::reading_activities)
        .await
        .map(Json)
        .map_err(control_error)
}
async fn start(
    State(s): State<ApiState>,
    Json(request): Json<StartReading>,
) -> Result<(StatusCode, Json<ReadingActivity>), ApiError> {
    s.actor
        .check_memory_input(&request)
        .map_err(control_error)?;
    let url = request.url.clone();
    let (activity, fresh) = blocking(&s.kernel, move |k| k.admit_reading(request))
        .await
        .map_err(control_error)?;
    if fresh {
        let id = activity.id;
        let redactor = s.actor.source_redactor();
        tokio::spawn(async move {
            if crate::reading::run(s.kernel, id, url, redactor)
                .await
                .is_err()
            {
                // Persisted claim remains inspectable; restart reconciles it as Unknown.
                tracing::warn!(activity = %id, "reading activity storage unavailable");
            }
        });
    }
    Ok((
        if fresh {
            StatusCode::ACCEPTED
        } else {
            StatusCode::OK
        },
        Json(activity),
    ))
}
async fn inspect(
    State(s): State<ApiState>,
    Path(id): Path<ReadingId>,
) -> Result<Json<ReadingResult>, ApiError> {
    let redactor = s.actor.source_redactor();
    blocking(&s.kernel, move |k| k.reading_result(id, redactor))
        .await
        .map(Json)
        .map_err(control_error)
}
async fn cancel(
    State(s): State<ApiState>,
    Path(id): Path<ReadingId>,
) -> Result<Json<ReadingActivity>, ApiError> {
    blocking(&s.kernel, move |k| k.cancel_reading(id))
        .await
        .map(Json)
        .map_err(control_error)
}
