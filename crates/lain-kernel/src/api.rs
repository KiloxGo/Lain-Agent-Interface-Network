mod channels;
mod inference;
mod memory;
mod reading;
mod sharing;

use std::{convert::Infallible, future::Future, path::PathBuf, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, FromRef, Path, Query, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{
        IntoResponse, Response, Sse,
        sse::{Event as SseEvent, KeepAlive},
    },
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

use crate::{
    backend::{BackendInfo, ControlError, Supervisor},
    config::ApiToken,
    domain::{
        Approval, ApprovalDecision, ApprovalId, RuntimeProfileId, SessionOptions, Turn,
        WorkSession, WorkSessionId,
    },
    kernel::{Kernel, KernelState, blocking},
};

#[derive(Clone)]
struct ApiState {
    kernel: Arc<Kernel>,
    workers: Option<Arc<Supervisor>>,
    actor: Arc<crate::actor::Actor>,
}

impl FromRef<ApiState> for Arc<Kernel> {
    fn from_ref(state: &ApiState) -> Self {
        state.kernel.clone()
    }
}

impl ApiState {
    fn workers(&self) -> Result<&Arc<Supervisor>, ApiError> {
        self.workers.as_ref().ok_or(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "worker_unavailable",
        ))
    }
}

pub fn router(kernel: Arc<Kernel>, token: ApiToken) -> Router {
    controlled_router(kernel, token, None)
}

pub fn controlled_router(
    kernel: Arc<Kernel>,
    token: ApiToken,
    workers: Option<Arc<Supervisor>>,
) -> Router {
    let actor = crate::actor::Actor::from_environment(kernel.clone(), workers.clone(), &token);
    actor_router(kernel, token, workers, actor)
}

pub fn actor_router(
    kernel: Arc<Kernel>,
    token: ApiToken,
    workers: Option<Arc<Supervisor>>,
    actor: Arc<crate::actor::Actor>,
) -> Router {
    Router::new()
        .merge(memory::routes())
        .merge(channels::routes())
        .merge(reading::routes())
        .merge(sharing::routes())
        .merge(inference::routes())
        .route(
            "/health",
            get(|| async { Json(serde_json::json!({"status": "ok", "service":"lain-kernel", "version":env!("CARGO_PKG_VERSION")})) }),
        )
        .route("/ready", get(readiness))
        .route("/v1/diagnostics", get(diagnostics))
        .route("/v1/conversations", get(conversations).post(new_conversation))
        .route("/v1/conversations/{id}", get(conversation))
        .route("/v1/conversations/{id}/turns", post(actor_submit))
        .route("/v1/conversations/{id}/delegations", get(delegations))
        .route("/v1/conversations/{id}/notices", get(conversation_notices))
        .route("/v1/conversations/{id}/notices/events", get(conversation_notice_events))
        .route("/v1/conversations/{id}/notices/{notice}/ack", post(acknowledge_notice))
        .route("/v1/actor-turns/{id}", get(actor_turn))
        .route("/v1/actor-turns/{id}/events", get(actor_events))
        .route("/v1/actor-turns/{id}/cancel", post(actor_cancel))
        .route("/v1/actor-turns/{id}/actions/{action}/decision", post(actor_decide))
        .route("/v1/work-sessions", get(list_sessions).post(create_session))
        .route("/v1/work-sessions/{id}", get(inspect_session))
        .route("/v1/work-sessions/{id}/turns", get(turns).post(prompt))
        .route("/v1/work-sessions/{id}/approvals", get(approvals))
        .route("/v1/work-sessions/{id}/history", get(history))
        .route("/v1/work-sessions/{id}/cancel", post(cancel))
        .route("/v1/work-sessions/{id}/resume", post(resume))
        .route(
            "/v1/work-sessions/{id}/approvals/{approval}/decision",
            post(decide),
        )
        .route("/v1/backends", get(backends))
        .route(
            "/v1/runtime-profiles",
            get(profiles).post(create_profile).put(update_profile),
        )
        .route("/v1/runtime-profiles/inspect", get(inspect_profile))
        .route("/v1/runtime-profiles/validate", post(validate_profile))
        .route(
            "/v1/workspace-profile-policy",
            get(workspace_policy).put(update_policy),
        )
        .route(
            "/v1/work-sessions/{id}/resolved-profile",
            get(resolved_profile),
        )
        .route("/v1/events", get(events))
        .route(
            "/v1/{*path}",
            get(|| async { ApiError(StatusCode::NOT_FOUND, "not_found") })
                .post(|| async { ApiError(StatusCode::NOT_FOUND, "not_found") }),
        )
        .fallback(|| async { ApiError(StatusCode::NOT_FOUND, "not_found") })
        .with_state(ApiState { kernel, workers, actor })
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(middleware::from_fn_with_state(token, authenticate))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateSession {
    workspace: PathBuf,
    profile_id: Option<RuntimeProfileId>,
    #[serde(default)]
    options: SessionOptions,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Prompt {
    text: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Decision {
    decision: ApprovalDecision,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct HistoryQuery {
    after: Option<u64>,
    before: Option<u64>,
    until: Option<u64>,
    limit: Option<usize>,
}
async fn history(
    State(kernel): State<Arc<Kernel>>,
    Path(id): Path<WorkSessionId>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<crate::events::HistoryPage>, ApiError> {
    let limit = query.limit.unwrap_or(100);
    if !(1..=200).contains(&limit)
        || (query.after.is_some() && query.before.is_some())
        || query.before.is_some_and(|n| n == 0 || n > i64::MAX as u64)
        || query.after.is_some_and(|n| n > i64::MAX as u64)
        || query.until.is_some_and(|n| n > i64::MAX as u64)
    {
        return Err(ApiError(StatusCode::BAD_REQUEST, "invalid_history_query"));
    }
    if blocking(&kernel, move |k| k.session(id))
        .await
        .map_err(internal_error)?
        .is_none()
    {
        return Err(ApiError(StatusCode::NOT_FOUND, "session_not_found"));
    }
    blocking(&kernel, move |k| {
        if let Some(before) = query.before {
            k.history_before(id, before, query.until, limit)
        } else {
            k.history(id, query.after.unwrap_or(0), query.until, limit)
        }
    })
    .await
    .map(Json)
    .map_err(internal_error)
}

async fn create_session(
    State(state): State<ApiState>,
    Json(input): Json<CreateSession>,
) -> Result<(StatusCode, Json<WorkSession>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .workers()?
                .create_with_profile(input.workspace, input.profile_id, input.options)
                .await
                .map_err(control_error)?,
        ),
    ))
}
async fn prompt(
    State(state): State<ApiState>,
    Path(id): Path<WorkSessionId>,
    Json(input): Json<Prompt>,
) -> Result<(StatusCode, Json<Turn>), ApiError> {
    Ok((
        StatusCode::ACCEPTED,
        Json(
            state
                .workers()?
                .prompt(id, input.text)
                .await
                .map_err(control_error)?,
        ),
    ))
}
async fn cancel(
    State(state): State<ApiState>,
    Path(id): Path<WorkSessionId>,
) -> Result<(StatusCode, Json<Turn>), ApiError> {
    Ok((
        StatusCode::ACCEPTED,
        Json(state.workers()?.cancel(id).await.map_err(control_error)?),
    ))
}
async fn resume(
    State(state): State<ApiState>,
    Path(id): Path<WorkSessionId>,
) -> Result<Json<WorkSession>, ApiError> {
    Ok(Json(
        state.workers()?.resume(id).await.map_err(control_error)?,
    ))
}
async fn decide(
    State(state): State<ApiState>,
    Path((id, approval)): Path<(WorkSessionId, ApprovalId)>,
    Json(input): Json<Decision>,
) -> Result<(StatusCode, Json<Approval>), ApiError> {
    Ok((
        StatusCode::ACCEPTED,
        Json(
            state
                .workers()?
                .decide(id, approval, input.decision)
                .await
                .map_err(control_error)?,
        ),
    ))
}
async fn turns(
    State(kernel): State<Arc<Kernel>>,
    Path(id): Path<WorkSessionId>,
) -> Result<Json<Vec<Turn>>, ApiError> {
    if blocking(&kernel, move |k| k.session(id))
        .await
        .map_err(internal_error)?
        .is_none()
    {
        return Err(ApiError(StatusCode::NOT_FOUND, "session_not_found"));
    }
    blocking(&kernel, move |k| k.turns(id))
        .await
        .map(Json)
        .map_err(internal_error)
}
async fn approvals(
    State(kernel): State<Arc<Kernel>>,
    Path(id): Path<WorkSessionId>,
) -> Result<Json<Vec<Approval>>, ApiError> {
    if blocking(&kernel, move |k| k.session(id))
        .await
        .map_err(internal_error)?
        .is_none()
    {
        return Err(ApiError(StatusCode::NOT_FOUND, "session_not_found"));
    }
    blocking(&kernel, move |k| k.approvals(id))
        .await
        .map(Json)
        .map_err(internal_error)
}
async fn backends(State(state): State<ApiState>) -> Json<Vec<BackendInfo>> {
    Json(
        state
            .workers
            .as_ref()
            .map(|s| vec![s.info()])
            .unwrap_or_default(),
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileQuery {
    id: RuntimeProfileId,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileUpdate {
    definition: crate::profiles::ProfileDefinition,
    expected_revision: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceQuery {
    workspace: PathBuf,
}
async fn profiles(
    State(state): State<ApiState>,
) -> Result<Json<Vec<crate::profiles::ProfileView>>, ApiError> {
    state
        .workers()?
        .profiles()
        .await
        .map(Json)
        .map_err(internal_error)
}
async fn inspect_profile(
    State(state): State<ApiState>,
    Query(query): Query<ProfileQuery>,
) -> Result<Json<crate::profiles::ProfileView>, ApiError> {
    state
        .workers()?
        .profiles()
        .await
        .map_err(internal_error)?
        .into_iter()
        .find(|p| p.profile.definition.id == query.id)
        .map(Json)
        .ok_or(ApiError(StatusCode::NOT_FOUND, "profile_not_found"))
}
async fn validate_profile(
    State(state): State<ApiState>,
    Json(query): Json<ProfileQuery>,
) -> Result<Json<crate::profiles::ProfileView>, ApiError> {
    inspect_profile(State(state), Query(query)).await
}
async fn create_profile(
    State(state): State<ApiState>,
    Json(definition): Json<crate::profiles::ProfileDefinition>,
) -> Result<(StatusCode, Json<crate::profiles::RuntimeProfile>), ApiError> {
    state
        .workers()?
        .safe_configuration(&definition)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "invalid_or_sensitive_profile"))?;
    let profile = blocking(&state.kernel, move |k| k.save_profile(definition, None))
        .await
        .map_err(control_error)?;
    Ok((StatusCode::CREATED, Json(profile)))
}
async fn update_profile(
    State(state): State<ApiState>,
    Json(input): Json<ProfileUpdate>,
) -> Result<Json<crate::profiles::RuntimeProfile>, ApiError> {
    state
        .workers()?
        .safe_configuration(&input.definition)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "invalid_or_sensitive_profile"))?;
    blocking(&state.kernel, move |k| {
        k.save_profile(input.definition, Some(input.expected_revision))
    })
    .await
    .map(Json)
    .map_err(control_error)
}
async fn workspace_policy(
    State(kernel): State<Arc<Kernel>>,
    Query(query): Query<WorkspaceQuery>,
) -> Result<Json<Option<crate::profiles::WorkspacePolicy>>, ApiError> {
    blocking(&kernel, move |k| k.workspace_policy(query.workspace))
        .await
        .map(Json)
        .map_err(control_error)
}
async fn update_policy(
    State(kernel): State<Arc<Kernel>>,
    Json(policy): Json<crate::profiles::WorkspacePolicy>,
) -> Result<Json<crate::profiles::WorkspacePolicy>, ApiError> {
    policy
        .validate()
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "invalid_workspace_policy"))?;
    blocking(&kernel, move |k| k.save_workspace_policy(policy))
        .await
        .map(Json)
        .map_err(control_error)
}
async fn resolved_profile(
    State(kernel): State<Arc<Kernel>>,
    Path(id): Path<WorkSessionId>,
) -> Result<Json<crate::profiles::ResolvedProfileView>, ApiError> {
    if blocking(&kernel, move |k| k.session(id))
        .await
        .map_err(internal_error)?
        .is_none()
    {
        return Err(ApiError(StatusCode::NOT_FOUND, "session_not_found"));
    }
    let resolved = blocking(&kernel, move |k| k.resolved_profile(id))
        .await
        .map_err(control_error)?;
    Ok(Json(crate::profiles::ResolvedProfileView {
        secret_status: crate::profiles::secret_status(resolved.secret_ref.as_ref()),
        resolved,
    }))
}

async fn events(State(kernel): State<Arc<Kernel>>) -> Result<impl IntoResponse, ApiError> {
    // Durable mutations cannot interleave snapshot creation and subscription.
    let (snapshot, mut events) = blocking(&kernel, Kernel::snapshot_and_subscribe)
        .await
        .map_err(internal_error)?;
    let mut shutdown = kernel.subscribe_shutdown();
    let (sender, receiver) = tokio::sync::mpsc::channel::<Result<SseEvent, Infallible>>(32);
    let initial = SseEvent::default()
        .event("snapshot")
        .json_data(snapshot)
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "serialization_error"))?;
    tokio::spawn(async move {
        if sender.send(Ok(initial)).await.is_err() {
            return;
        }
        loop {
            if *shutdown.borrow() {
                break;
            }
            let (event, last) = tokio::select! {
                _=shutdown.changed()=>break,
                _=sender.closed()=>break,
                event=events.recv()=>match event {
                    Ok(event)=>(SseEvent::default().event("event").id(event.id.to_string()).json_data(event).expect("serializable event"),false),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_))=>(SseEvent::default().event("resync_required").data("take a fresh snapshot"),true),
                    Err(_)=>break,
                }
            };
            tokio::select! {_=shutdown.changed()=>break,result=sender.send(Ok(event))=>if result.is_err(){break;}}
            if last {
                break;
            }
        }
    });
    Ok(
        Sse::new(tokio_stream::wrappers::ReceiverStream::new(receiver))
            .keep_alive(KeepAlive::default()),
    )
}

fn control_error(error: anyhow::Error) -> ApiError {
    if let Some(error) = error.downcast_ref::<ControlError>() {
        return match error {
            ControlError::NotFound => ApiError(StatusCode::NOT_FOUND, "not_found"),
            ControlError::Conflict => ApiError(StatusCode::CONFLICT, "state_conflict"),
            ControlError::Unavailable => {
                ApiError(StatusCode::SERVICE_UNAVAILABLE, "worker_unavailable")
            }
            ControlError::InvalidInput => ApiError(StatusCode::BAD_REQUEST, "invalid_input"),
            ControlError::ProfileDenied => {
                ApiError(StatusCode::FORBIDDEN, "workspace_profile_denied")
            }
            ControlError::InferenceDenied => {
                ApiError(StatusCode::TOO_MANY_REQUESTS, "inference_budget_denied")
            }
        };
    }
    if error.downcast_ref::<crate::domain::DomainError>().is_some() {
        return ApiError(StatusCode::BAD_REQUEST, "invalid_workspace_or_state");
    }
    tracing::warn!(%error,"worker control operation failed");
    ApiError(StatusCode::BAD_GATEWAY, "worker_operation_failed")
}

async fn authenticate(State(token): State<ApiToken>, request: Request, next: Next) -> Response {
    let mut headers = request.headers().get_all(header::AUTHORIZATION).iter();
    let valid = headers
        .next()
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|v| token.matches(v))
        && headers.next().is_none();
    let mut response = if valid {
        let response = next.run(request).await;
        // Axum extractor diagnostics can echo rejected user values. Preserve our
        // JSON error codes, replace framework text before it reaches any client.
        if matches!(
            response.status(),
            StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY
        ) && response
            .headers()
            .get(header::CONTENT_TYPE)
            .is_none_or(|v| v != "application/json")
        {
            ApiError(response.status(), "invalid_input_format").into_response()
        } else {
            response
        }
    } else {
        let mut response = ApiError(StatusCode::UNAUTHORIZED, "unauthorized").into_response();
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, "Bearer".parse().unwrap());
        response
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    response
}

#[derive(Serialize)]
struct Readiness {
    state: KernelState,
}

async fn readiness(
    State(kernel): State<Arc<Kernel>>,
) -> Result<(StatusCode, Json<Readiness>), ApiError> {
    let state = blocking(&kernel, Kernel::state)
        .await
        .map_err(internal_error)?;
    Ok((
        if state == KernelState::Ready {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        },
        Json(Readiness { state }),
    ))
}

async fn list_sessions(
    State(kernel): State<Arc<Kernel>>,
) -> Result<Json<Vec<WorkSession>>, ApiError> {
    blocking(&kernel, Kernel::list_sessions)
        .await
        .map(Json)
        .map_err(internal_error)
}

async fn inspect_session(
    State(kernel): State<Arc<Kernel>>,
    Path(raw_id): Path<String>,
) -> Result<Json<WorkSession>, ApiError> {
    let id: WorkSessionId = raw_id
        .parse()
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "invalid_session_id"))?;
    blocking(&kernel, move |k| k.session(id))
        .await
        .map_err(internal_error)?
        .map(Json)
        .ok_or(ApiError(StatusCode::NOT_FOUND, "session_not_found"))
}

struct ApiError(StatusCode, &'static str);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({"error": self.1}))).into_response()
    }
}

fn internal_error(error: anyhow::Error) -> ApiError {
    tracing::error!(error = %error, "Kernel API operation failed");
    ApiError(StatusCode::INTERNAL_SERVER_ERROR, "internal_error")
}

/// Listener must already be bound; shutdown removes readiness before draining HTTP.
pub async fn serve(
    listener: TcpListener,
    kernel: Arc<Kernel>,
    token: ApiToken,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<()> {
    serve_with_workers(listener, kernel, token, None, shutdown).await
}

pub async fn serve_with_workers(
    listener: TcpListener,
    kernel: Arc<Kernel>,
    token: ApiToken,
    workers: Option<Arc<Supervisor>>,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<()> {
    let app = controlled_router(kernel.clone(), token, workers);
    serve_router(listener, kernel, app, shutdown).await
}

/// Static files are public; API routes retain their bearer authentication.
pub fn web_router(
    kernel: Arc<Kernel>,
    token: ApiToken,
    workers: Option<Arc<Supervisor>>,
    directory: PathBuf,
) -> Router {
    with_web(controlled_router(kernel, token, workers), directory)
}

pub fn with_web(app: Router, directory: PathBuf) -> Router {
    use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};
    app
        .fallback_service(ServeDir::new(directory))
        .layer(SetResponseHeaderLayer::overriding(header::CONTENT_SECURITY_POLICY, axum::http::HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self'; font-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'")))
        .layer(SetResponseHeaderLayer::overriding(header::X_CONTENT_TYPE_OPTIONS, axum::http::HeaderValue::from_static("nosniff")))
        .layer(SetResponseHeaderLayer::overriding(header::REFERRER_POLICY, axum::http::HeaderValue::from_static("no-referrer")))
        .layer(SetResponseHeaderLayer::overriding(header::CACHE_CONTROL, axum::http::HeaderValue::from_static("no-store")))
}

pub async fn serve_router(
    listener: TcpListener,
    kernel: Arc<Kernel>,
    app: Router,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<()> {
    if kernel.state()? == KernelState::Recovering {
        kernel.mark_ready()?;
    }
    let (draining, drained) = tokio::sync::oneshot::channel();
    let shutdown_kernel = kernel.clone();
    let server = axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown.await;
            if let Err(error) = shutdown_kernel.shutdown() {
                tracing::error!(%error, "shutdown state failed");
            }
            let _ = draining.send(());
        })
        .into_future();
    tokio::pin!(server);
    let result = tokio::select! {
        result = &mut server => result.context("HTTP server failed"),
        _ = drained => {
            // Bound draining even if a client keeps a connection open indefinitely.
            tokio::time::timeout(Duration::from_secs(5), &mut server).await
                .context("HTTP shutdown exceeded five seconds")?.context("HTTP server failed")
        }
    };
    kernel.shutdown()?;
    result
}

async fn diagnostics(State(state): State<ApiState>) -> Result<Json<serde_json::Value>, ApiError> {
    let kernel_state = blocking(&state.kernel, Kernel::state)
        .await
        .map_err(internal_error)?;
    let storage = blocking(&state.kernel, Kernel::storage_healthy)
        .await
        .map_err(internal_error)?;
    let profiles = if let Some(workers) = &state.workers {
        workers.profiles().await.map_err(internal_error)?
    } else {
        Vec::new()
    };
    Ok(Json(
        serde_json::json!({"kernel":kernel_state,"storage":if storage {"healthy"} else {"unhealthy"},"backends":state.workers.as_ref().map(|w|vec![w.info()]).unwrap_or_default(),"profiles":profiles,"actor":state.actor.diagnostics(),"model_calls":0}),
    ))
}
async fn conversations(
    State(kernel): State<Arc<Kernel>>,
) -> Result<Json<Vec<crate::actor::domain::Conversation>>, ApiError> {
    blocking(&kernel, Kernel::conversations)
        .await
        .map(Json)
        .map_err(internal_error)
}
async fn new_conversation(
    State(kernel): State<Arc<Kernel>>,
) -> Result<(StatusCode, Json<crate::actor::domain::Conversation>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(
            blocking(&kernel, Kernel::create_conversation)
                .await
                .map_err(control_error)?,
        ),
    ))
}
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct ConversationQuery {
    after: Option<u64>,
    limit: Option<usize>,
    recent: Option<bool>,
}
async fn conversation(
    State(kernel): State<Arc<Kernel>>,
    Path(id): Path<crate::actor::domain::ConversationId>,
    Query(query): Query<ConversationQuery>,
) -> Result<Json<crate::actor::domain::ConversationPage>, ApiError> {
    blocking(&kernel, move |k| {
        if query.recent == Some(true) {
            if query.after.is_some() || query.limit.is_some() {
                return Err(ControlError::InvalidInput.into());
            }
            let mut page = k.actor_page(id, 0, 1)?;
            page.turns = k.actor_recent(id)?;
            page.next_cursor = page.turns.last().map(|t| t.sequence).unwrap_or(0);
            page.has_more = false;
            Ok(page)
        } else {
            k.actor_page(id, query.after.unwrap_or(0), query.limit.unwrap_or(20))
        }
    })
    .await
    .map(Json)
    .map_err(control_error)
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct NoticeQuery {
    after: Option<u64>,
    limit: Option<usize>,
}
async fn delegations(
    State(kernel): State<Arc<Kernel>>,
    Path(id): Path<crate::actor::domain::ConversationId>,
) -> Result<Json<Vec<crate::delegation::Delegation>>, ApiError> {
    blocking(&kernel, move |k| k.delegations(id))
        .await
        .map(Json)
        .map_err(control_error)
}
async fn conversation_notices(
    State(kernel): State<Arc<Kernel>>,
    Path(id): Path<crate::actor::domain::ConversationId>,
    Query(query): Query<NoticeQuery>,
) -> Result<Json<crate::delegation::NoticePage>, ApiError> {
    blocking(&kernel, move |k| {
        k.notices(id, query.after.unwrap_or(0), query.limit.unwrap_or(50))
    })
    .await
    .map(Json)
    .map_err(control_error)
}
async fn acknowledge_notice(
    State(kernel): State<Arc<Kernel>>,
    Path((id, notice)): Path<(
        crate::actor::domain::ConversationId,
        crate::delegation::NoticeId,
    )>,
) -> Result<Json<serde_json::Value>, ApiError> {
    blocking(&kernel, move |k| k.acknowledge_notice(id, notice))
        .await
        .map_err(control_error)?;
    Ok(Json(serde_json::json!({"acknowledged":true})))
}
async fn conversation_notice_events(
    State(state): State<ApiState>,
    Path(id): Path<crate::actor::domain::ConversationId>,
    Query(query): Query<NoticeQuery>,
) -> Result<impl IntoResponse, ApiError> {
    if query.limit.is_some() {
        return Err(control_error(ControlError::InvalidInput.into()));
    }
    // Reuse both existing observation channels as wakeups. The durable notice
    // cursor, not broadcast delivery, owns catch-up and crash recovery.
    let mut actor_events = state.actor.subscribe();
    let (mut page, mut worker_events) = blocking(&state.kernel, move |k| {
        k.notices_and_subscribe(id, query.after.unwrap_or(0))
    })
    .await
    .map_err(control_error)?;
    let mut shutdown = state.kernel.subscribe_shutdown();
    let (sender, receiver) =
        tokio::sync::mpsc::channel::<Result<SseEvent, Infallible>>(NOTICE_QUEUE_CAPACITY);
    tokio::spawn(async move {
        loop {
            let cursor = page.next_cursor;
            let more = page.has_more;
            let event = SseEvent::default()
                .event("notices")
                .id(cursor.to_string())
                .json_data(&page)
                .expect("serializable notices");
            // Bound a slow client's retained pages and release it on shutdown.
            tokio::select! {
                _=shutdown.changed()=>break,
                _=sender.closed()=>break,
                sent=send_notice_page(&sender, event)=>if !sent { break; },
            }
            if !more {
                loop {
                    let wake = tokio::select! {
                        _=shutdown.changed()=>return,
                        _=sender.closed()=>return,
                        event=worker_events.recv()=> match event {
                            Ok(event)=>matches!(event.kind, crate::events::EventKind::TurnCompleted { .. } | crate::events::EventKind::TurnStateChanged { .. }),
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_))=>true,
                            Err(_)=>return,
                        },
                        event=actor_events.recv()=> match event {
                            Ok(crate::actor::Observation::Snapshot(turn))=>turn.conversation_id == id,
                            Ok(_)=>false,
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_))=>true,
                            Err(_)=>return,
                        },
                    };
                    if wake {
                        break;
                    }
                }
            }
            page = match blocking(&state.kernel, move |k| k.notices(id, cursor, 100)).await {
                Ok(page) => page,
                Err(_) => break,
            };
        }
    });
    Ok(
        Sse::new(tokio_stream::wrappers::ReceiverStream::new(receiver))
            .keep_alive(KeepAlive::default()),
    )
}
const NOTICE_QUEUE_CAPACITY: usize = 8;
async fn send_notice_page(
    sender: &tokio::sync::mpsc::Sender<Result<SseEvent, Infallible>>,
    event: SseEvent,
) -> bool {
    matches!(
        tokio::time::timeout(std::time::Duration::from_secs(10), sender.send(Ok(event))).await,
        Ok(Ok(()))
    )
}

#[cfg(test)]
mod notice_delivery_tests {
    use super::*;
    #[tokio::test(start_paused = true)]
    async fn slow_or_closed_notice_consumers_release_bounded_delivery() {
        let (tx, rx) = tokio::sync::mpsc::channel(NOTICE_QUEUE_CAPACITY);
        for _ in 0..NOTICE_QUEUE_CAPACITY {
            assert!(
                send_notice_page(
                    &tx,
                    SseEvent::default().event("notices").data("fixture page")
                )
                .await
            );
        }
        let started = tokio::time::Instant::now();
        assert!(!send_notice_page(&tx, SseEvent::default().data("full")).await);
        assert_eq!(started.elapsed(), std::time::Duration::from_secs(10));
        drop(rx);
        let started = tokio::time::Instant::now();
        assert!(!send_notice_page(&tx, SseEvent::default().data("closed")).await);
        assert_eq!(started.elapsed(), std::time::Duration::ZERO);
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ActorInput {
    request_id: uuid::Uuid,
    text: String,
}
async fn actor_submit(
    State(state): State<ApiState>,
    Path(id): Path<crate::actor::domain::ConversationId>,
    Json(input): Json<ActorInput>,
) -> Result<(StatusCode, Json<crate::actor::domain::ActorTurn>), ApiError> {
    state
        .actor
        .submit(id, input.request_id, input.text)
        .await
        .map(|turn| (StatusCode::ACCEPTED, Json(turn)))
        .map_err(control_error)
}
async fn actor_turn(
    State(kernel): State<Arc<Kernel>>,
    Path(id): Path<crate::actor::domain::ActorTurnId>,
) -> Result<Json<crate::actor::domain::ActorTurn>, ApiError> {
    blocking(&kernel, move |k| k.actor_turn(id))
        .await
        .map(Json)
        .map_err(control_error)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ActorDecision {
    approve: bool,
}
async fn actor_decide(
    State(state): State<ApiState>,
    Path((id, action)): Path<(
        crate::actor::domain::ActorTurnId,
        crate::actor::domain::ActionId,
    )>,
    Json(input): Json<ActorDecision>,
) -> Result<Json<crate::actor::domain::ActorTurn>, ApiError> {
    state
        .actor
        .decide(id, action, input.approve)
        .await
        .map(Json)
        .map_err(control_error)
}
async fn actor_cancel(
    State(state): State<ApiState>,
    Path(id): Path<crate::actor::domain::ActorTurnId>,
) -> Result<Json<crate::actor::domain::ActorTurn>, ApiError> {
    state
        .actor
        .cancel(id)
        .await
        .map(Json)
        .map_err(control_error)
}
async fn actor_events(
    State(state): State<ApiState>,
    Path(id): Path<crate::actor::domain::ActorTurnId>,
) -> Result<impl IntoResponse, ApiError> {
    // Subscribe before reading a snapshot; revisions discard overlapping earlier events.
    let mut events = state.actor.subscribe();
    let initial = blocking(&state.kernel, move |k| k.actor_turn(id))
        .await
        .map_err(control_error)?;
    let mut shutdown = state.kernel.subscribe_shutdown();
    let (sender, receiver) = tokio::sync::mpsc::channel::<Result<SseEvent, Infallible>>(16);
    tokio::spawn(async move {
        use crate::actor::{Observation, domain::TurnState};
        let mut revision = initial.revision;
        let finished = initial.state != TurnState::Running;
        let mut committed: std::collections::BTreeSet<_> =
            initial.messages.iter().map(|m| m.id).collect();
        let mut previews = std::collections::BTreeMap::new();
        let checkpoint = if finished {
            None
        } else {
            state.actor.preview_checkpoint(id)
        };
        if sender
            .send(Ok(SseEvent::default()
                .event("turn")
                .json_data(initial)
                .unwrap()))
            .await
            .is_err()
            || finished
        {
            return;
        }
        if let Some(checkpoint) = checkpoint
            && !committed.contains(&checkpoint.message_id)
        {
            previews.insert(checkpoint.preview_id, checkpoint.sequence);
            if sender
                .send(Ok(SseEvent::default()
                    .event("preview")
                    .json_data(checkpoint)
                    .unwrap()))
                .await
                .is_err()
            {
                return;
            }
        }
        loop {
            let observation = tokio::select! {
                _=shutdown.changed()=>break,
                _=sender.closed()=>break,
                value=events.recv()=>match value {
                    Ok(event)=>event,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_))=>{
                        let _=sender.send(Ok(SseEvent::default().event("resync_required").data("reconnect to the same turn"))).await;
                        break;
                    },
                    Err(_)=>break,
                }
            };
            let (event, finished) = match observation {
                Observation::Snapshot(turn) if turn.id == id && turn.revision > revision => {
                    revision = turn.revision;
                    committed.extend(turn.messages.iter().map(|m| m.id));
                    let finished = turn.state != TurnState::Running;
                    (
                        SseEvent::default().event("turn").json_data(turn).unwrap(),
                        finished,
                    )
                }
                Observation::Preview(preview)
                    if preview.turn_id == id && !committed.contains(&preview.message_id) =>
                {
                    if previews
                        .get(&preview.preview_id)
                        .is_some_and(|seq| *seq >= preview.sequence)
                    {
                        continue;
                    }
                    previews.insert(preview.preview_id, preview.sequence);
                    (
                        SseEvent::default()
                            .event("preview")
                            .json_data(preview)
                            .unwrap(),
                        false,
                    )
                }
                _ => continue,
            };
            tokio::select! {_=shutdown.changed()=>break,_=sender.closed()=>break,result=sender.send(Ok(event))=>if result.is_err(){break;}}
            if finished {
                break;
            }
        }
    });
    Ok(
        Sse::new(tokio_stream::wrappers::ReceiverStream::new(receiver))
            .keep_alive(KeepAlive::default()),
    )
}
