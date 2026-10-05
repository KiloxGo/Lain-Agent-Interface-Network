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
        Approval, ApprovalDecision, ApprovalId, SessionOptions, Turn, WorkSession, WorkSessionId,
    },
    kernel::{Kernel, KernelState, blocking},
};

#[derive(Clone)]
struct ApiState {
    kernel: Arc<Kernel>,
    workers: Option<Arc<Supervisor>>,
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
    Router::new()
        .route(
            "/health",
            get(|| async { Json(serde_json::json!({"status": "ok"})) }),
        )
        .route("/ready", get(readiness))
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
        .route("/v1/events", get(events))
        .route(
            "/v1/{*path}",
            get(|| async { ApiError(StatusCode::NOT_FOUND, "not_found") })
                .post(|| async { ApiError(StatusCode::NOT_FOUND, "not_found") }),
        )
        .fallback(|| async { ApiError(StatusCode::NOT_FOUND, "not_found") })
        .with_state(ApiState { kernel, workers })
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(middleware::from_fn_with_state(token, authenticate))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateSession {
    workspace: PathBuf,
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
                .create(input.workspace, input.options)
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
        next.run(request).await
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
    use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};
    controlled_router(kernel, token, workers)
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
    kernel.mark_ready()?;
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
