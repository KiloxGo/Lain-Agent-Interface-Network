//! Owner controls for day evidence and explicitly triggered finite model batches.
use super::archives::safe;
use super::*;
use lain_wire::{ConversationId, RequestId, briefing::*};

pub(super) fn routes() -> Router<ApiState> {
    Router::new()
        .route("/v1/briefing/{conversation}/grant", get(grant).post(set))
        .route(
            "/v1/briefing/{conversation}/grant/requests/{id}",
            get(grant_receipt),
        )
        .route("/v1/briefing/{conversation}/prepare", post(prepare))
        .route("/v1/briefing/{conversation}/manifests/{id}", get(manifest))
        .route("/v1/briefing/{conversation}/manifests/{id}/page", get(page))
        .route(
            "/v1/briefing/{conversation}/manifests/{id}/run",
            get(run_status).post(run),
        )
        .route(
            "/v1/briefing/{conversation}/manifests/{id}/report",
            get(report),
        )
        .route(
            "/v1/briefing/{conversation}/manifests/{id}/process",
            post(process),
        )
        .route(
            "/v1/briefing/{conversation}/manifests/{id}/batches/{from_index}",
            get(batch),
        )
}
async fn run(
    State(s): State<ApiState>,
    Path((c, id)): Path<(ConversationId, RequestId)>,
) -> Result<Json<BriefingRunStatus>, ApiError> {
    safe(
        &s,
        s.actor
            .start_briefing_run(c, id)
            .await
            .map_err(control_error)?,
    )
}
async fn run_status(
    State(s): State<ApiState>,
    Path((c, id)): Path<(ConversationId, RequestId)>,
) -> Result<Json<BriefingRunStatus>, ApiError> {
    let value = blocking(&s.kernel, move |k| k.briefing_run_status(c, id))
        .await
        .map_err(control_error)?;
    safe(&s, value)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReportPosition {
    #[serde(default)]
    after: u16,
    #[serde(default = "report_limit")]
    limit: u16,
}
fn report_limit() -> u16 {
    4
}
async fn report(
    State(s): State<ApiState>,
    Path((c, id)): Path<(ConversationId, RequestId)>,
    Query(q): Query<ReportPosition>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let r = s.actor.source_redactor();
    let value = blocking(&s.kernel, move |k| {
        k.briefing_report(c, id, q.after, q.limit, r)
    })
    .await
    .map_err(control_error)?;
    safe(&s, value)
}
async fn grant(
    State(s): State<ApiState>,
    Path(c): Path<ConversationId>,
) -> Result<Json<BriefingGrant>, ApiError> {
    let value = blocking(&s.kernel, move |k| k.briefing_grant(c))
        .await
        .map_err(control_error)?;
    safe(&s, value)
}
async fn grant_receipt(
    State(s): State<ApiState>,
    Path((c, id)): Path<(ConversationId, RequestId)>,
) -> Result<Json<BriefingGrantReceipt>, ApiError> {
    let value = blocking(&s.kernel, move |k| k.briefing_grant_receipt(c, id))
        .await
        .map_err(control_error)?;
    safe(&s, value)
}
async fn set(
    State(s): State<ApiState>,
    Path(c): Path<ConversationId>,
    Json(q): Json<SetBriefingGrant>,
) -> Result<Json<BriefingGrant>, ApiError> {
    let value = blocking(&s.kernel, move |k| k.briefing_set(c, q))
        .await
        .map_err(control_error)?;
    safe(&s, value)
}
async fn prepare(
    State(s): State<ApiState>,
    Path(c): Path<ConversationId>,
    Json(q): Json<StartBriefing>,
) -> Result<Json<BriefingManifest>, ApiError> {
    let value = blocking(&s.kernel, move |k| k.briefing_start(c, q))
        .await
        .map_err(control_error)?;
    safe(&s, value)
}
async fn manifest(
    State(s): State<ApiState>,
    Path((c, id)): Path<(ConversationId, RequestId)>,
) -> Result<Json<BriefingManifest>, ApiError> {
    let value = blocking(&s.kernel, move |k| k.briefing_manifest(c, id))
        .await
        .map_err(control_error)?;
    safe(&s, value)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Position {
    #[serde(default)]
    from_index: u16,
}
async fn process(
    State(s): State<ApiState>,
    Path((c, id)): Path<(ConversationId, RequestId)>,
    Json(p): Json<Position>,
) -> Result<Json<BriefingBatch>, ApiError> {
    let _turn = s
        .actor
        .process_briefing(c, id, p.from_index)
        .await
        .map_err(control_error)?;
    let r = s.actor.source_redactor();
    let value = blocking(&s.kernel, move |k| k.briefing_batch(c, id, p.from_index, r))
        .await
        .map_err(control_error)?;
    safe(&s, value)
}
async fn batch(
    State(s): State<ApiState>,
    Path((c, id, from_index)): Path<(ConversationId, RequestId, u16)>,
) -> Result<Json<BriefingBatch>, ApiError> {
    let r = s.actor.source_redactor();
    let value = blocking(&s.kernel, move |k| k.briefing_batch(c, id, from_index, r))
        .await
        .map_err(control_error)?;
    safe(&s, value)
}
async fn page(
    State(s): State<ApiState>,
    Path((c, id)): Path<(ConversationId, RequestId)>,
    Query(p): Query<Position>,
) -> Result<Json<BriefingPage>, ApiError> {
    let r = s.actor.source_redactor();
    let value = blocking(&s.kernel, move |k| k.briefing_page(c, id, p.from_index, r))
        .await
        .map_err(control_error)?;
    safe(&s, value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use lain_wire::archives::*;
    use tower::ServiceExt;
    #[tokio::test]
    async fn exact_grant_receipt_survives_later_change_without_replaying_authority() {
        let dir = tempfile::tempdir().unwrap();
        let k = Arc::new(Kernel::open(&dir.path().join("state.sqlite")).unwrap());
        k.mark_ready().unwrap();
        let c = k.create_conversation().unwrap().id;
        let other = k.create_conversation().unwrap().id;
        let q = SetBriefingGrant {
            request_id: RequestId::new(),
            expected_revision: 0,
            scope: None,
        };
        k.briefing_set(c, q.clone()).unwrap();
        k.briefing_set(
            c,
            SetBriefingGrant {
                request_id: RequestId::new(),
                expected_revision: 1,
                scope: None,
            },
        )
        .unwrap();
        const TOKEN: &str = "fixture-grant-receipt-token-0123456789";
        let app = router(k.clone(), TOKEN.parse().unwrap());
        for (conversation, id, expected) in [
            (c, q.request_id, 200),
            (other, q.request_id, 404),
            (c, RequestId::new(), 404),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/v1/briefing/{conversation}/grant/requests/{id}"))
                        .header("authorization", format!("Bearer {TOKEN}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), expected);
            if expected == 200 {
                assert_eq!(response.headers()["cache-control"], "no-store");
                let body: serde_json::Value =
                    serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap())
                        .unwrap();
                assert_eq!(body["request"], serde_json::to_value(&q).unwrap());
                assert_eq!(
                    body["grant"]["conversation_id"],
                    serde_json::to_value(c).unwrap()
                );
                assert_eq!(body["grant"]["revision"], 1);
            }
        }
        assert_eq!(k.briefing_grant(c).unwrap().revision, 2);
    }
    #[tokio::test]
    async fn owner_evidence_api_is_scoped_no_store_and_has_no_inference_or_memory_effect() {
        let dir = tempfile::tempdir().unwrap();
        let k = Arc::new(Kernel::open(&dir.path().join("state.sqlite")).unwrap());
        k.mark_ready().unwrap();
        let conversation = k.create_conversation().unwrap().id;
        let now = crate::domain::now_ms().unwrap();
        let source = k
            .archive_register(
                ArchiveSourceKey::QqGroup {
                    observer_account: "1001".into(),
                    group_id: "3003".into(),
                },
                crate::redaction::Redactor::new(vec![]),
            )
            .unwrap()
            .source
            .id;
        k.archive_set_grant(
            source,
            SetArchiveGrant {
                expected_revision: 0,
                scope: Some(ArchiveScope {
                    expires_at_ms: now + 100000,
                    retention_ms: 100000,
                    max_records: 100,
                }),
            },
        )
        .unwrap();
        const TOKEN: &str = "fixture-briefing-owner-token-0123456789";
        let app = router(k.clone(), TOKEN.parse().unwrap());
        let path = format!("/v1/briefing/{conversation}/grant");
        assert_eq!(
            app.clone()
                .oneshot(Request::builder().uri(&path).body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            401
        );
        let q = SetBriefingGrant {
            request_id: RequestId::new(),
            expected_revision: 0,
            scope: Some(BriefingScope {
                sources: vec![ArchiveRecallSource {
                    source_id: source,
                    grant_revision: 1,
                }],
                from_ms: 0,
                expires_at_ms: now + 60000,
                utc_offset_minutes: 480,
                max_records: 100,
            }),
        };
        let set = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&path)
                    .header("authorization", format!("Bearer {TOKEN}"))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&q).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(set.status(), 200);
        assert_eq!(set.headers()["cache-control"], "no-store");
        let q = StartBriefing {
            request_id: RequestId::new(),
            grant_revision: 1,
            day: BriefingDay::Yesterday,
        };
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/briefing/{conversation}/prepare"))
                    .header("authorization", format!("Bearer {TOKEN}"))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&q).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), 200);
        let body: BriefingManifest =
            serde_json::from_slice(&to_bytes(res.into_body(), 65536).await.unwrap()).unwrap();
        assert!(body.records.is_empty());
        let other = k.create_conversation().unwrap().id;
        let res = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/v1/briefing/{other}/manifests/{}/page",
                        q.request_id
                    ))
                    .header("authorization", format!("Bearer {TOKEN}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), 404);
        assert_eq!(k.inference_snapshot().unwrap().reserved_requests, 0);
        assert!(k.list_sessions().unwrap().is_empty());
    }
}
