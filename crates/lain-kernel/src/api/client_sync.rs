use super::*;
use lain_wire::client_sync::*;
pub(super) fn routes() -> Router<ApiState> {
    Router::new()
        .route("/v1/client-sync/bootstrap", post(bootstrap))
        .route("/v1/client-sync/events", get(events))
}
async fn bootstrap(
    State(s): State<ApiState>,
    Json(input): Json<BootstrapRequest>,
) -> Result<Json<BootstrapSnapshot>, ApiError> {
    if input.protocol != PROTOCOL {
        return Err(ApiError(
            StatusCode::UPGRADE_REQUIRED,
            "unsupported_sync_protocol",
        ));
    }
    let resources = s.actor.sync_resources().map_err(control_error)?;
    let snapshot = blocking(&s.kernel, move |k| k.bootstrap(input, resources))
        .await
        .map_err(control_error)?;
    s.actor
        .check_memory_input(&snapshot)
        .map_err(control_error)?;
    Ok(Json(snapshot))
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct WatchQuery {
    instance_id: uuid::Uuid,
    boot_id: uuid::Uuid,
    after: u64,
    lease_until_ms: u64,
}
fn reason(status: &SyncStatus, q: &WatchQuery, now: u64) -> Option<InvalidationReason> {
    if status.cursor.instance_id != q.instance_id
        || status.cursor.boot_id != q.boot_id
        || q.after > status.cursor.revision
        || status.cursor.revision.saturating_sub(q.after) > CURSOR_WINDOW
    {
        Some(InvalidationReason::Resnapshot)
    } else if q.after != status.cursor.revision {
        Some(InvalidationReason::Changed)
    } else if now >= q.lease_until_ms {
        Some(InvalidationReason::Expired)
    } else {
        None
    }
}
async fn events(
    State(s): State<ApiState>,
    Query(q): Query<WatchQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let now = crate::domain::now_ms().map_err(|e| control_error(e.into()))?;
    if q.after > i64::MAX as u64
        || q.instance_id.is_nil()
        || q.boot_id.is_nil()
        || q.lease_until_ms > now.saturating_add(MAX_LEASE_MS)
    {
        return Err(control_error(ControlError::InvalidInput.into()));
    }
    // Subscribe before comparing, so a write at either side is observed. No
    // source payload is queued; unfold reads only when the HTTP body is polled.
    let changes = s.kernel.subscribe_sync();
    let shutdown = s.kernel.subscribe_shutdown();
    let stream = futures_util::stream::unfold(
        Some((s.kernel, q, changes, shutdown)),
        |state| async move {
            let (kernel, q, mut changes, mut shutdown) = state?;
            loop {
                let status = match blocking(&kernel, Kernel::sync_status).await {
                    Ok(v) => v,
                    Err(_) => return None,
                };
                let now = crate::domain::now_ms().ok()?;
                if let Some(reason) = reason(&status, &q, now) {
                    let event = Invalidation {
                        cursor: status.cursor,
                        reason,
                    };
                    return Some((
                        Ok::<_, Infallible>(
                            SseEvent::default()
                                .event("invalidate")
                                .json_data(event)
                                .expect("serializable invalidation"),
                        ),
                        None,
                    ));
                }
                let wait = q
                    .lease_until_ms
                    .min(status.valid_until_ms)
                    .saturating_sub(now);
                tokio::select! {
                    result=changes.changed()=>if result.is_err(){return None;},
                    _=shutdown.changed()=>return None,
                    _=tokio::time::sleep(Duration::from_millis(wait.max(1)))=>{
                        let status=blocking(&kernel,Kernel::sync_status).await.ok()?;
                        return Some((Ok(SseEvent::default().event("invalidate").json_data(Invalidation{cursor:status.cursor,reason:InvalidationReason::Expired}).expect("serializable expiry")),None));
                    }
                }
            }
        },
    );
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}
