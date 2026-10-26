use super::*;
use lain_wire::{ExperienceId, ReadingId, reading::*};

pub(super) fn routes() -> Router<ApiState> {
    Router::new()
        .route("/v1/reading/grant", get(grant).post(set_grant))
        .route("/v1/reading/budget", get(budget))
        .route("/v1/reading/audit", get(audit))
        .route("/v1/reading/grant/history", get(grants))
        .route("/v1/reading/activities", get(list).post(start))
        .route("/v1/reading/activities/{id}", get(inspect))
        .route("/v1/reading/activities/{id}/cancel", post(cancel))
        .route("/v1/reading/activities/{id}/process", post(process))
        .route("/v1/experiences/search", post(search_experiences))
        .route("/v1/experiences/{id}", get(experience))
        .route("/v1/experiences/{id}/history", get(history))
        .route("/v1/experiences/{id}/withdraw", post(withdraw))
        .route("/v1/experiences/{id}/correct", post(correct))
        .route("/v1/reading/suppressed", get(suppressed))
        .route("/v1/reading/restore-source", post(restore))
}
async fn grant(State(s): State<ApiState>) -> Result<Json<ReadingGrant>, ApiError> {
    blocking(&s.kernel, Kernel::reading_grant)
        .await
        .map(Json)
        .map_err(control_error)
}
async fn budget(State(s): State<ApiState>) -> Result<Json<serde_json::Value>, ApiError> {
    blocking(&s.kernel, Kernel::reading_budget)
        .await
        .map(Json)
        .map_err(control_error)
}
async fn audit(State(s): State<ApiState>) -> Result<Json<serde_json::Value>, ApiError> {
    blocking(&s.kernel, Kernel::reading_audit)
        .await
        .map(Json)
        .map_err(control_error)
}
async fn history(
    State(s): State<ApiState>,
    Path(id): Path<ExperienceId>,
) -> Result<Json<serde_json::Value>, ApiError> {
    blocking(&s.kernel, move |k| k.experience_history(id))
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
            if crate::reading::run(s.kernel.clone(), id, url, redactor)
                .await
                .is_err()
            {
                // Persisted claim remains inspectable; restart reconciles it as Unknown.
                tracing::warn!(activity = %id, "reading activity storage unavailable");
            }
            // Only the separately configured processing scope permits a model job.
            let enabled = blocking(&s.kernel, move |k| {
                Ok(k.reading_grant()?
                    .scope
                    .is_some_and(|g| g.processing.is_some()))
            })
            .await
            .unwrap_or(false);
            if enabled && s.actor.process_reading(id).await.is_err() {
                tracing::info!(activity=%id,"reading processing not started; inspect scope, budget and activity state");
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
async fn process(
    State(s): State<ApiState>,
    Path(id): Path<ReadingId>,
) -> Result<Json<crate::actor::domain::ActorTurn>, ApiError> {
    s.actor
        .process_reading(id)
        .await
        .map(Json)
        .map_err(control_error)
}
async fn search_experiences(
    State(s): State<ApiState>,
    Json(q): Json<ExperienceQuery>,
) -> Result<Json<Vec<ReadingExperience>>, ApiError> {
    s.actor.check_memory_input(&q).map_err(control_error)?;
    let redactor = s.actor.source_redactor();
    blocking(&s.kernel, move |k| k.experiences(q, redactor))
        .await
        .map(Json)
        .map_err(control_error)
}
async fn experience(
    State(s): State<ApiState>,
    Path(id): Path<ExperienceId>,
) -> Result<Json<ReadingExperience>, ApiError> {
    let redactor = s.actor.source_redactor();
    blocking(&s.kernel, move |k| k.experience(id, redactor))
        .await
        .map(Json)
        .map_err(control_error)
}
async fn withdraw(
    State(s): State<ApiState>,
    Path(id): Path<ExperienceId>,
    Json(q): Json<WithdrawExperience>,
) -> Result<Json<ReadingExperience>, ApiError> {
    blocking(&s.kernel, move |k| {
        k.withdraw_experience(id, q.expected_revision)
    })
    .await
    .map(Json)
    .map_err(control_error)
}
async fn correct(
    State(s): State<ApiState>,
    Path(id): Path<ExperienceId>,
    Json(q): Json<CorrectExperience>,
) -> Result<Json<ReadingExperience>, ApiError> {
    s.actor.check_memory_input(&q).map_err(control_error)?;
    blocking(&s.kernel, move |k| k.correct_experience(id, q))
        .await
        .map(Json)
        .map_err(control_error)
}
async fn suppressed(State(s): State<ApiState>) -> Result<Json<serde_json::Value>, ApiError> {
    blocking(&s.kernel, Kernel::reading_suppressions)
        .await
        .map(Json)
        .map_err(control_error)
}
async fn restore(
    State(s): State<ApiState>,
    Json(q): Json<RestoreReadingSource>,
) -> Result<Json<serde_json::Value>, ApiError> {
    s.actor.check_memory_input(&q).map_err(control_error)?;
    blocking(&s.kernel,move |k|k.restore_reading_source(q)).await.map(|_|Json(serde_json::json!({"restored_for_new_explicit_acquisitions":true,"old_withdrawals_restored":false}))).map_err(control_error)
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
    let activity = blocking(&s.kernel, move |k| k.cancel_reading(id))
        .await
        .map_err(control_error)?;
    let redactor = s.actor.source_redactor();
    let result = blocking(&s.kernel, move |k| k.reading_result(id, redactor))
        .await
        .map_err(control_error)?;
    if let Some(turn) = result.processing_turn {
        let _ = s.actor.cancel(turn).await;
    }
    Ok(Json(activity))
}
