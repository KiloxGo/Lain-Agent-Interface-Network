use super::archives::safe;
use super::*;
use lain_wire::{ConversationId, DeviceCommandId, DeviceId, device_commands::*, device_work::*};

pub(super) fn routes() -> Router<ApiState> {
    Router::new()
        .route("/v1/conversations/{conversation}/device-work", get(page))
        .route(
            "/v1/conversations/{conversation}/device-work/{device}/requests/{request}",
            get(request_receipt),
        )
        .route(
            "/v1/conversations/{conversation}/device-work/events",
            get(events),
        )
        .route(
            "/v1/conversations/{conversation}/device-work/{device}",
            post(dispatch),
        )
        .route(
            "/v1/conversations/{conversation}/device-work/{device}/{command}",
            get(inspect),
        )
        .route(
            "/v1/conversations/{conversation}/device-work/{device}/{command}/followups",
            post(followup).get(followup_receipt),
        )
}
async fn request_receipt(
    State(s): State<ApiState>,
    Path((conversation, device, request)): Path<(ConversationId, DeviceId, lain_wire::RequestId)>,
) -> Result<Json<Option<DeviceWorkRecord>>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| {
            k.device_work_request_receipt(conversation, device, request)
        })
        .await
        .map_err(control_error)?,
    )
}
async fn followup_receipt(
    State(s): State<ApiState>,
    Path((conversation, device, parent)): Path<(ConversationId, DeviceId, DeviceCommandId)>,
) -> Result<Json<Option<DeviceCommand>>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| {
            k.device_work_followup_receipt(conversation, device, parent)
        })
        .await
        .map_err(control_error)?,
    )
}
async fn followup(
    State(s): State<ApiState>,
    Path((conversation, device, parent)): Path<(ConversationId, DeviceId, DeviceCommandId)>,
    Json(input): Json<FollowupDeviceWork>,
) -> Result<Json<DeviceCommand>, ApiError> {
    let Json(input) = safe(&s, input)?;
    safe(
        &s,
        blocking(&s.kernel, move |k| {
            k.device_work_followup(conversation, device, parent, input)
        })
        .await
        .map_err(control_error)?,
    )
}
async fn dispatch(
    State(s): State<ApiState>,
    Path((conversation, device)): Path<(ConversationId, DeviceId)>,
    Json(input): Json<QueueDeviceWork>,
) -> Result<Json<DeviceCommand>, ApiError> {
    let Json(input) = safe(&s, input)?;
    safe(
        &s,
        blocking(&s.kernel, move |k| {
            k.device_work_dispatch(conversation, device, input)
        })
        .await
        .map_err(control_error)?,
    )
}
async fn page(
    State(s): State<ApiState>,
    Path(conversation): Path<ConversationId>,
) -> Result<Json<DeviceWorkPage>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| k.device_work_page(conversation))
            .await
            .map_err(control_error)?,
    )
}
async fn inspect(
    State(s): State<ApiState>,
    Path((conversation, device, command)): Path<(ConversationId, DeviceId, DeviceCommandId)>,
) -> Result<Json<DeviceWorkRecord>, ApiError> {
    safe(
        &s,
        blocking(&s.kernel, move |k| {
            k.device_work_inspect(conversation, device, command)
        })
        .await
        .map_err(control_error)?,
    )
}
async fn events(
    State(s): State<ApiState>,
    Path(conversation): Path<ConversationId>,
) -> Result<Response, ApiError> {
    // Subscribe before the initial read. A missed commit/publication is repaired
    // from the fresh durable snapshot, never by repeating a mutation.
    let mut changes = s.kernel.device_commands_subscribe();
    let mut shutdown = s.kernel.subscribe_shutdown();
    let initial = page(State(s.clone()), Path(conversation)).await?.0;
    let (sender, receiver) = tokio::sync::mpsc::channel::<Result<SseEvent, Infallible>>(2);
    tokio::spawn(async move {
        let mut next = Some(initial);
        let mut last = None;
        loop {
            if *shutdown.borrow() || sender.is_closed() {
                break;
            }
            let page = match next.take() {
                Some(page) => page,
                None => match page(State(s.clone()), Path(conversation)).await {
                    Ok(v) => v.0,
                    Err(_) => break,
                },
            };
            let Ok(encoded) = serde_json::to_string(&page) else {
                break;
            };
            if last.as_ref() != Some(&encoded) {
                let event = SseEvent::default()
                    .event("device_work_snapshot")
                    .data(encoded.clone());
                tokio::select! {
                    _=shutdown.changed()=>break,
                    sent=tokio::time::timeout(Duration::from_secs(5),sender.send(Ok(event)))=>if !matches!(sent,Ok(Ok(()))){break;}
                }
                last = Some(encoded);
            }
            tokio::select! {
                _=shutdown.changed()=>break,
                _=sender.closed()=>break,
                _=changes.recv()=>{},
                // Recheck time-dependent expiry; no work or inference is invoked.
                _=tokio::time::sleep(Duration::from_secs(25))=>{},
            }
        }
    });
    Ok(
        Sse::new(tokio_stream::wrappers::ReceiverStream::new(receiver))
            .keep_alive(KeepAlive::default())
            .into_response(),
    )
}
