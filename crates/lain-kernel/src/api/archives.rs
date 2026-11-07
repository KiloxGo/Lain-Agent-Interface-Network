use super::*;
use lain_wire::{ArchiveRecordId, ArchiveSourceId, RequestId, archives::*};

#[cfg(test)]
mod reader_tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;
    #[tokio::test]
    async fn reader_status_is_owner_only_bounded_configuration_metadata_without_secrets() {
        let dir = tempfile::tempdir().unwrap();
        let k = Arc::new(Kernel::open(&dir.path().join("state.sqlite")).unwrap());
        k.mark_ready().unwrap();
        const TOKEN: &str = "fixture-reader-status-owner-credential-123456";
        let app = router(k, TOKEN.parse().unwrap());
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/archives/knowledge-reader")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 401);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/archives/knowledge-reader")
                    .header("authorization", format!("Bearer {TOKEN}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
        assert_eq!(body.as_object().unwrap().len(), 3);
        assert!(body["configured"].is_boolean() && body["credentials_available"].is_boolean());
        assert!(
            body["revision"].is_null()
                || body["revision"]
                    .as_str()
                    .is_some_and(|r| r.len() == 64 && r.bytes().all(|b| b.is_ascii_hexdigit()))
        );
        for forbidden in [
            "endpoint",
            "secret_ref",
            "token",
            "key",
            "messages",
            "source_id",
        ] {
            assert!(body.get(forbidden).is_none());
        }
    }
}

pub(super) fn routes() -> Router<ApiState> {
    Router::new()
        .route("/v1/archives/knowledge-reader", get(knowledge_reader))
        .route(
            "/v1/archives/memory/{id}",
            get(archive_memory_grant).post(archive_memory_set),
        )
        .route(
            "/v1/archives/memory/{id}/grants",
            get(archive_memory_grants),
        )
        .route("/v1/archives/sources", get(list).post(register))
        .route("/v1/archives/status", get(statuses))
        .route("/v1/archives/sources/{id}/status", get(status))
        .route(
            "/v1/archives/recall/{id}",
            get(recall_grant).post(recall_set),
        )
        .route("/v1/archives/recall/{id}/reads", get(recall_audit))
        .route("/v1/archives/sources/{id}", get(source))
        .route("/v1/archives/sources/{id}/grant", post(set_grant))
        .route("/v1/archives/sources/{id}/grants", get(grants))
        .route("/v1/archives/sources/{id}/import", post(import))
        .route("/v1/archives/sources/{id}/query", post(query))
        .route("/v1/archives/sources/{id}/read", post(read))
        .route("/v1/archives/sources/{id}/knowledge", get(knowledge_state))
        .route(
            "/v1/archives/sources/{id}/knowledge/sync/grants",
            get(knowledge_sync_grants),
        )
        .route(
            "/v1/archives/sources/{id}/knowledge/sync",
            get(knowledge_sync).post(set_knowledge_sync),
        )
        .route(
            "/v1/archives/sources/{id}/knowledge/pull",
            post(knowledge_pull),
        )
        .route("/v1/archives/reads/{id}", get(read_job))
        .route("/v1/archives/reads/{id}/cancel", post(cancel_read))
        .route("/v1/archives/records/{id}", get(record))
        .route("/v1/archives/records/{id}/edits", get(edits))
        .route("/v1/archives/records/{id}/forget", post(forget))
        .route("/v1/archives/records/{id}/correct", post(correct))
}
async fn knowledge_reader(
    State(s): State<ApiState>,
) -> Result<Json<KnowledgeReaderStatus>, ApiError> {
    safe(&s, crate::knowledge::Config::status())
}
async fn statuses(State(s): State<ApiState>) -> Result<Json<Vec<ArchiveSourceStatus>>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.archive_status())
            .await
            .map_err(control_error)?,
    )
}
async fn status(
    State(s): State<ApiState>,
    Path(id): Path<ArchiveSourceId>,
) -> Result<Json<ArchiveSourceStatus>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.archive_source_status(id))
            .await
            .map_err(control_error)?,
    )
}
async fn archive_memory_grant(
    State(s): State<ApiState>,
    Path(id): Path<lain_wire::ChannelId>,
) -> Result<Json<ArchiveMemoryGrant>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.archive_memory_grant(id))
            .await
            .map_err(control_error)?,
    )
}
async fn archive_memory_grants(
    State(s): State<ApiState>,
    Path(id): Path<lain_wire::ChannelId>,
) -> Result<Json<Vec<ArchiveMemoryGrant>>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.archive_memory_grants(id))
            .await
            .map_err(control_error)?,
    )
}
async fn archive_memory_set(
    State(s): State<ApiState>,
    Path(id): Path<lain_wire::ChannelId>,
    Json(q): Json<SetArchiveMemory>,
) -> Result<Json<ArchiveMemoryGrant>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.archive_memory_set(id, q))
            .await
            .map_err(control_error)?,
    )
}
async fn knowledge_sync_grants(
    State(s): State<ApiState>,
    Path(id): Path<ArchiveSourceId>,
) -> Result<Json<Vec<KnowledgeSyncGrant>>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.knowledge_sync_grants(id))
            .await
            .map_err(control_error)?,
    )
}
async fn knowledge_sync(
    State(s): State<ApiState>,
    Path(id): Path<ArchiveSourceId>,
) -> Result<Json<KnowledgeSyncGrant>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.knowledge_sync_grant(id))
            .await
            .map_err(control_error)?,
    )
}
async fn set_knowledge_sync(
    State(s): State<ApiState>,
    Path(id): Path<ArchiveSourceId>,
    Json(q): Json<SetKnowledgeSync>,
) -> Result<Json<KnowledgeSyncGrant>, ApiError> {
    let revision = if q.scope.is_some() {
        Some(
            crate::knowledge::Config::load()
                .map_err(control_error)?
                .revision(),
        )
    } else {
        None
    };
    safe(
        &s,
        blocking(&s.kernel, move |k| k.knowledge_sync_set(id, q, revision))
            .await
            .map_err(control_error)?,
    )
}
async fn knowledge_state(
    State(s): State<ApiState>,
    Path(id): Path<ArchiveSourceId>,
) -> Result<Json<Option<KnowledgeArchiveState>>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.knowledge_state(id))
            .await
            .map_err(control_error)?,
    )
}
async fn knowledge_pull(
    State(s): State<ApiState>,
    Path(id): Path<ArchiveSourceId>,
    Json(q): Json<ReadArchive>,
) -> Result<Json<ArchiveReadJob>, ApiError> {
    let r = s.actor.source_redactor();
    safe(
        &s,
        crate::knowledge::start(s.kernel.clone(), id, q, r)
            .await
            .map_err(control_error)?,
    )
}
async fn recall_grant(
    State(s): State<ApiState>,
    Path(id): Path<lain_wire::ChannelId>,
) -> Result<Json<ArchiveRecallGrant>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.archive_recall_grant(id))
            .await
            .map_err(control_error)?,
    )
}
async fn recall_set(
    State(s): State<ApiState>,
    Path(id): Path<lain_wire::ChannelId>,
    Json(q): Json<SetArchiveRecall>,
) -> Result<Json<ArchiveRecallGrant>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.archive_recall_set(id, q))
            .await
            .map_err(control_error)?,
    )
}
#[derive(serde::Deserialize)]
struct RecallAfter {
    #[serde(default)]
    after: u64,
}
async fn recall_audit(
    State(s): State<ApiState>,
    Path(id): Path<lain_wire::ChannelId>,
    Query(q): Query<RecallAfter>,
) -> Result<Json<serde_json::Value>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.archive_recall_audit(id, q.after))
            .await
            .map_err(control_error)?,
    )
}
async fn cancel_read(
    State(s): State<ApiState>,
    Path(id): Path<RequestId>,
) -> Result<Json<ArchiveReadJob>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| {
            k.archive_read_fail(
                id,
                ArchiveReadState::Cancelled,
                ArchiveReadFailure::Interrupted,
                None,
            )
        })
        .await
        .map_err(control_error)?,
    )
}
async fn read(
    State(s): State<ApiState>,
    Path(id): Path<ArchiveSourceId>,
    Json(q): Json<ReadWorkerArchive>,
) -> Result<Json<ArchiveReadJob>, ApiError> {
    safe(
        &s,
        s.workers()?
            .read_archive(id, q)
            .await
            .map_err(control_error)?,
    )
}
async fn read_job(
    State(s): State<ApiState>,
    Path(id): Path<RequestId>,
) -> Result<Json<ArchiveReadJob>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.archive_read_job(id))
            .await
            .map_err(control_error)?,
    )
}
pub(super) fn safe<T: serde::Serialize>(s: &ApiState, value: T) -> Result<Json<T>, ApiError> {
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
