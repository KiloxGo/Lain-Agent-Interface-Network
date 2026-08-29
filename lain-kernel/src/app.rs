use std::{collections::BTreeMap, future::Future, path::Path as StdPath, sync::Arc};

use axum::{
    Json, Router,
    extract::{
        DefaultBodyLimit, Path, Request as AxumRequest, State, WebSocketUpgrade, ws::Message,
        ws::WebSocket,
    },
    http::{
        HeaderMap, HeaderValue, Method, StatusCode,
        header::{CONTENT_TYPE, ORIGIN},
    },
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tower_http::{
    cors::CorsLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

use crate::{
    config::Config,
    error::AppError,
    identity::{IdentityKernel, sha256_hex},
    mcp::McpClient,
    memory,
    provider::{
        FinishReason, ModelError, ModelEvent, ModelMessage, ModelRequest, ModelRouter, Role,
        RouteKind, SharedProvider, TokenUsage, ToolChoice, deepseek::DeepSeekProvider,
        provider_diagnostics as build_provider_diagnostics,
    },
    storage::Storage,
    tools::WorkspaceTools,
};

#[derive(Debug, Clone, Serialize)]
pub struct ReadinessCheck {
    pub name: &'static str,
    pub ready: bool,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct ReadinessState {
    checks: Arc<Vec<ReadinessCheck>>,
}

impl ReadinessState {
    pub fn is_ready(&self) -> bool {
        self.checks.iter().all(|check| check.ready)
    }

    pub fn checks(&self) -> &[ReadinessCheck] {
        self.checks.as_ref()
    }

    fn from_checks(checks: Vec<ReadinessCheck>) -> Self {
        Self {
            checks: Arc::new(checks),
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub readiness: ReadinessState,
    pub provider: Option<SharedProvider>,
    pub system_prompt: Option<Arc<str>>,
    pub identity: Option<Arc<IdentityKernel>>,
    pub storage: Option<Storage>,
    pub workspace_tools: Option<WorkspaceTools>,
    pub mcp: Option<Arc<McpClient>>,
    pub router: ModelRouter,
    pub tasks: TaskTracker,
    pub shutdown: CancellationToken,
}

impl AppState {
    pub async fn initialize(config: Config) -> Self {
        let mut checks = Vec::with_capacity(5);

        let workspace_tools = config.workspace_root.as_ref().and_then(|root| {
            match WorkspaceTools::new(root) {
                Ok(tools) => Some(tools),
                Err(error) => {
                    tracing::warn!(workspace = %root.display(), error = %error, "workspace tools disabled");
                    None
                }
            }
        });

        let memory_configured = memory::configure_data_dir(config.data_dir.clone());
        match memory_configured {
            Ok(()) => match tokio::fs::create_dir_all(config.data_dir.join("memory")).await {
                Ok(()) => checks.push(ReadinessCheck {
                    name: "memory",
                    ready: true,
                    detail: "data directory is writable".to_string(),
                }),
                Err(error) => {
                    tracing::error!(
                        error = %error,
                        data_dir = %config.data_dir.display(),
                        "data directory is unavailable"
                    );
                    checks.push(ReadinessCheck {
                        name: "memory",
                        ready: false,
                        detail: "data directory is unavailable".to_string(),
                    });
                }
            },
            Err(error) => {
                tracing::error!(error = %error, "data directory configuration failed");
                checks.push(ReadinessCheck {
                    name: "memory",
                    ready: false,
                    detail: "data directory configuration failed".to_string(),
                });
            }
        }

        let session_title = workspace_session_title(config.workspace_root.as_deref());
        let storage = match Storage::initialize_for_session(&config.data_dir, &session_title).await
        {
            Ok(storage) => {
                checks.push(ReadinessCheck {
                    name: "storage",
                    ready: true,
                    detail: "SQLite event store initialized".to_string(),
                });
                Some(storage)
            }
            Err(error) => {
                tracing::error!(error = %error, "persistent storage initialization failed");
                checks.push(ReadinessCheck {
                    name: "storage",
                    ready: false,
                    detail: "SQLite event store is unavailable".to_string(),
                });
                None
            }
        };

        let system_prompt = match tokio::fs::read_to_string(&config.system_prompt_path).await {
            Ok(prompt) if !prompt.trim().is_empty() => {
                checks.push(ReadinessCheck {
                    name: "legacy_prompt",
                    ready: true,
                    detail: "system prompt loaded".to_string(),
                });
                Some(Arc::<str>::from(prompt))
            }
            Ok(_) => {
                checks.push(ReadinessCheck {
                    name: "legacy_prompt",
                    ready: false,
                    detail: "system prompt is empty".to_string(),
                });
                None
            }
            Err(error) => {
                tracing::error!(
                    error = %error,
                    prompt_path = %config.system_prompt_path.display(),
                    "system prompt is unavailable"
                );
                checks.push(ReadinessCheck {
                    name: "legacy_prompt",
                    ready: false,
                    detail: "system prompt is unavailable".to_string(),
                });
                None
            }
        };

        let identity = match IdentityKernel::load(&config.identity_dir).await {
            Ok(identity) => {
                let identity = Arc::new(identity);
                let activation = match &storage {
                    Some(storage) => Some(storage.activate_identity(&identity).await),
                    None => {
                        checks.push(ReadinessCheck {
                            name: "identity",
                            ready: false,
                            detail: "identity cannot be activated without storage".to_string(),
                        });
                        None
                    }
                };
                match activation {
                    Some(Ok(_)) => {
                        checks.push(ReadinessCheck {
                            name: "identity",
                            ready: true,
                            detail: format!(
                                "identity {} validated and activated",
                                identity.version()
                            ),
                        });
                        Some(identity)
                    }
                    Some(Err(error)) => {
                        tracing::error!(
                            error = %error,
                            identity_dir = %config.identity_dir.display(),
                            "identity activation failed"
                        );
                        checks.push(ReadinessCheck {
                            name: "identity",
                            ready: false,
                            detail: "identity activation failed".to_string(),
                        });
                        None
                    }
                    None => None,
                }
            }
            Err(error) => {
                tracing::error!(
                    error = %error,
                    identity_dir = %config.identity_dir.display(),
                    "identity validation failed"
                );
                checks.push(ReadinessCheck {
                    name: "identity",
                    ready: false,
                    detail: "identity validation failed".to_string(),
                });
                None
            }
        };

        let provider = if config.provider.is_configured() {
            match DeepSeekProvider::new(&config.provider) {
                Ok(provider) => {
                    checks.push(ReadinessCheck {
                        name: "provider",
                        ready: true,
                        detail: format!(
                            "{} / {} configured",
                            config.provider.model, config.provider.pro_model
                        ),
                    });
                    Some(Arc::new(provider) as SharedProvider)
                }
                Err(error) => {
                    tracing::error!(code = error.code(), error = %error, "provider initialization failed");
                    checks.push(ReadinessCheck {
                        name: "provider",
                        ready: false,
                        detail: "provider initialization failed".to_string(),
                    });
                    None
                }
            }
        } else {
            checks.push(ReadinessCheck {
                name: "provider",
                ready: false,
                detail: "DEEPSEEK_API_KEY is not configured".to_string(),
            });
            None
        };

        let router = ModelRouter::new(
            config.provider.model.clone(),
            config.provider.pro_model.clone(),
        );
        let tasks = TaskTracker::new();
        let shutdown = CancellationToken::new();
        // ponytail: one opt-in server covers the first bridge; add a supervised server map
        // only after a second real integration or restart requirement appears.
        let mcp = match config.mcp.as_ref() {
            Some(mcp_config) => match McpClient::connect(mcp_config).await {
                Ok(client) => {
                    let client = Arc::new(client);
                    tracing::info!(
                        namespace = client.info().namespace,
                        protocol_version = client.info().protocol_version,
                        "MCP stdio server connected"
                    );
                    let lifecycle = Arc::clone(&client);
                    let lifecycle_shutdown = shutdown.clone();
                    tasks.spawn(async move {
                        lifecycle_shutdown.cancelled().await;
                        lifecycle.close().await;
                    });
                    Some(client)
                }
                Err(error) => {
                    tracing::warn!(error = %error, "MCP stdio server unavailable; continuing without MCP");
                    None
                }
            },
            None => None,
        };
        let state = Self {
            config: Arc::new(config),
            readiness: ReadinessState::from_checks(checks),
            provider,
            system_prompt,
            identity,
            storage,
            workspace_tools,
            mcp,
            router,
            tasks,
            shutdown,
        };
        state.start_memory_worker();
        state
    }

    fn start_memory_worker(&self) {
        if !self.config.memory.extraction_enabled {
            tracing::info!("memory extraction worker disabled by configuration");
            return;
        }
        let (Some(storage), Some(provider)) = (self.storage.clone(), self.provider.clone()) else {
            tracing::warn!(
                "memory extraction worker requested but storage or provider is unavailable"
            );
            return;
        };
        let router = self.router.clone();
        let provider_config = self.config.provider.clone();
        let memory_config = self.config.memory.clone();
        let shutdown = self.shutdown.clone();
        self.tasks.spawn(async move {
            memory::worker::run(
                storage,
                provider,
                router,
                provider_config,
                memory_config,
                shutdown,
            )
            .await;
        });
    }

    fn spawn_io_task<F>(&self, task_name: &'static str, future: F)
    where
        F: Future<Output = std::io::Result<()>> + Send + 'static,
    {
        self.tasks.spawn(async move {
            if let Err(error) = future.await {
                tracing::warn!(error = %error, task_name, "background task failed");
            }
        });
    }
}

fn workspace_session_title(workspace_root: Option<&StdPath>) -> String {
    workspace_root.map_or_else(
        || "Main".to_string(),
        |root| {
            let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
            let path = canonical.to_string_lossy();
            let normalized = if cfg!(windows) {
                path.to_lowercase()
            } else {
                path.into_owned()
            };
            format!("workspace:{}", sha256_hex(normalized.as_bytes()))
        },
    )
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

#[derive(Debug, Serialize)]
struct ReadyResponse {
    status: &'static str,
    checks: Vec<ReadinessCheck>,
}

pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("http://127.0.0.1:5173"),
            HeaderValue::from_static("http://localhost:5173"),
        ])
        .allow_methods([Method::GET, Method::PATCH, Method::POST, Method::DELETE])
        .allow_headers([CONTENT_TYPE]);
    let websocket_origin = middleware::from_fn(validate_websocket_origin);

    Router::new()
        .route("/api/health", get(health))
        .route("/api/ready", get(ready))
        .route("/api/provider", get(provider_diagnostics))
        .route(
            "/api/memory",
            get(memory_overview).route_layer(websocket_origin.clone()),
        )
        .route(
            "/api/memory/{item_id}",
            patch(update_memory_item)
                .delete(retract_memory_item)
                .route_layer(websocket_origin.clone()),
        )
        .route(
            "/api/memory/pending/{candidate_id}/confirm",
            post(confirm_memory_candidate).route_layer(websocket_origin.clone()),
        )
        .route(
            "/api/memory/pending/{candidate_id}",
            delete(reject_memory_candidate).route_layer(websocket_origin.clone()),
        )
        .route("/ws", get(ws_handler).route_layer(websocket_origin.clone()))
        .route(
            "/ws/stream",
            get(stream_ws_handler).route_layer(websocket_origin.clone()),
        )
        .route("/v1/ws", get(v1_ws_handler).route_layer(websocket_origin))
        .with_state(state)
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    let ready = state.readiness.is_ready();
    let status = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(ReadyResponse {
            status: if ready { "ready" } else { "not_ready" },
            checks: state.readiness.checks().to_vec(),
        }),
    )
}

async fn provider_diagnostics(
    State(state): State<AppState>,
) -> Json<crate::provider::ProviderDiagnostics> {
    Json(build_provider_diagnostics(
        &state.config.provider,
        state.provider.is_some(),
    ))
}

#[derive(Debug, Serialize)]
struct MemoryOverview {
    items: Vec<memory::types::MemoryItemView>,
    pending: Vec<memory::types::PendingMemoryView>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MemoryItemUpdate {
    pinned: Option<bool>,
    never_recall: Option<bool>,
    status: Option<String>,
}

async fn memory_overview(State(state): State<AppState>) -> Result<Json<MemoryOverview>, AppError> {
    let storage = state
        .storage
        .ok_or_else(|| AppError::NotReady("storage is unavailable".to_string()))?;
    let (items, pending) = tokio::try_join!(
        storage.list_memory_items(false),
        storage.list_pending_memory()
    )?;
    Ok(Json(MemoryOverview { items, pending }))
}

async fn update_memory_item(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
    Json(update): Json<MemoryItemUpdate>,
) -> Result<Json<memory::types::MemoryItemView>, AppError> {
    let storage = state
        .storage
        .ok_or_else(|| AppError::NotReady("storage is unavailable".to_string()))?;
    Ok(Json(
        storage
            .update_memory_item(
                &item_id,
                update.pinned,
                update.never_recall,
                update.status.as_deref(),
            )
            .await?,
    ))
}

async fn retract_memory_item(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
) -> Result<Json<memory::types::MemoryItemView>, AppError> {
    let storage = state
        .storage
        .ok_or_else(|| AppError::NotReady("storage is unavailable".to_string()))?;
    Ok(Json(
        storage
            .update_memory_item(&item_id, None, None, Some("retracted"))
            .await?,
    ))
}

async fn confirm_memory_candidate(
    State(state): State<AppState>,
    Path(candidate_id): Path<String>,
) -> Result<StatusCode, AppError> {
    let storage = state
        .storage
        .ok_or_else(|| AppError::NotReady("storage is unavailable".to_string()))?;
    storage.confirm_memory_candidate(&candidate_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn reject_memory_candidate(
    State(state): State<AppState>,
    Path(candidate_id): Path<String>,
) -> Result<StatusCode, AppError> {
    let storage = state
        .storage
        .ok_or_else(|| AppError::NotReady("storage is unavailable".to_string()))?;
    storage.reject_memory_candidate(&candidate_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn validate_websocket_origin(request: AxumRequest, next: Next) -> Result<Response, AppError> {
    if let Some(origin) = request.headers().get(ORIGIN)
        && origin != "http://127.0.0.1:5173"
        && origin != "http://localhost:5173"
    {
        return Err(AppError::ForbiddenOrigin);
    }

    Ok(next.run(request).await)
}

async fn ws_handler(
    headers: HeaderMap,
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Result<Response, AppError> {
    upgrade_websocket(headers, ws, state, WireMode::Legacy).await
}

async fn stream_ws_handler(
    headers: HeaderMap,
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Result<Response, AppError> {
    upgrade_websocket(headers, ws, state, WireMode::Events).await
}

async fn v1_ws_handler(
    headers: HeaderMap,
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Result<Response, AppError> {
    crate::transport::v1::upgrade(headers, ws, state).await
}

async fn upgrade_websocket(
    headers: HeaderMap,
    ws: WebSocketUpgrade,
    state: AppState,
    mode: WireMode,
) -> Result<Response, AppError> {
    if !state.readiness.is_ready() || state.provider.is_none() || state.system_prompt.is_none() {
        let failures = state
            .readiness
            .checks()
            .iter()
            .filter(|check| !check.ready)
            .map(|check| check.name)
            .collect::<Vec<_>>()
            .join(",");
        return Err(AppError::NotReady(failures));
    }

    let request_id = headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
    let mut response = ws
        .max_message_size(state.config.max_message_bytes)
        .max_frame_size(state.config.max_message_bytes)
        .on_upgrade(move |socket| handle_socket(socket, state, request_id, mode))
        .into_response();
    response
        .headers_mut()
        .insert("deprecation", HeaderValue::from_static("true"));
    if mode == WireMode::Events {
        response
            .headers_mut()
            .insert("x-lain-protocol", HeaderValue::from_static("p2-stream-v1"));
    }
    Ok(response)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WireMode {
    Legacy,
    Events,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StreamClientMessage {
    Message { text: String },
    Cancel,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StreamServerEvent<'a> {
    Started {
        request_id: &'a str,
        model: &'a str,
    },
    Delta {
        text: &'a str,
    },
    Finished {
        reason: FinishReason,
        usage: Option<&'a TokenUsage>,
    },
    Error {
        code: &'a str,
        partial: bool,
    },
}

async fn handle_socket(
    mut socket: WebSocket,
    state: AppState,
    connection_request_id: String,
    mode: WireMode,
) {
    let Some(provider) = state.provider.clone() else {
        return;
    };
    let Some(system_prompt) = state.system_prompt.clone() else {
        return;
    };
    let span = tracing::info_span!("websocket", request_id = connection_request_id);
    let _guard = span.enter();
    let mut history = Vec::<ModelMessage>::new();
    let mut turn = 0_u64;

    loop {
        let incoming = tokio::select! {
            () = state.shutdown.cancelled() => break,
            incoming = socket.recv() => incoming,
        };
        let frame = match incoming {
            Some(Ok(Message::Text(frame))) => frame,
            Some(Ok(Message::Close(_))) => {
                let _ = socket.flush().await;
                break;
            }
            _ => break,
        };
        let Some(text) = parse_user_message(&frame, mode) else {
            if mode == WireMode::Events {
                let _ = send_wire_error(&mut socket, "invalid_client_message", false).await;
            }
            continue;
        };

        if text.len() > state.config.max_message_bytes {
            if !send_model_error(
                &mut socket,
                mode,
                "message_too_large",
                false,
                Some(state.config.max_message_bytes),
            )
            .await
            {
                break;
            }
            continue;
        }

        add_retrieval_context(&mut history, &text).await;
        history.push(ModelMessage::text(Role::User, text.clone()));

        let profile = match memory::load_profile().await {
            Ok(profile) => profile.unwrap_or_default(),
            Err(error) => {
                tracing::warn!(error = %error, "profile loading failed");
                if !send_model_error(&mut socket, mode, "profile_load_failed", false, None).await {
                    break;
                }
                continue;
            }
        };
        let system_content = if profile.is_empty() {
            system_prompt.to_string()
        } else {
            format!("{system_prompt}\n\n##User Context\n{profile}")
        };
        let mut request_messages = vec![ModelMessage::text(Role::System, system_content)];
        request_messages.extend(history.clone());

        turn += 1;
        let run_id = format!("{connection_request_id}:{turn}");
        let route_kind = classify_route(&text);
        let route = state.router.route(route_kind);
        tracing::info!(
            run_id,
            model = route.model,
            reason = route.reason,
            "model route selected"
        );
        let request = ModelRequest {
            request_id: run_id.clone(),
            model: route.model,
            messages: request_messages,
            tools: Vec::new(),
            tool_choice: ToolChoice::None,
            temperature: None,
            top_p: None,
            max_output_tokens: Some(state.config.provider.max_output_tokens),
            response_format: route.response_format,
            thinking_mode: route.thinking_mode,
            timeout: state.config.provider.request_timeout,
            metadata: BTreeMap::from([("route".to_string(), format!("{route_kind:?}"))]),
        };
        let run_cancellation = state.shutdown.child_token();
        let stream_result = if mode == WireMode::Events {
            let start = provider.stream(request, run_cancellation.clone());
            tokio::pin!(start);
            loop {
                tokio::select! {
                    () = state.shutdown.cancelled() => {
                        run_cancellation.cancel();
                        return;
                    }
                    result = &mut start => break Some(result),
                    incoming = socket.recv() => {
                        match incoming {
                            Some(Ok(Message::Text(message))) if is_cancel_message(&message) => {
                                run_cancellation.cancel();
                                if !send_wire(
                                    &mut socket,
                                    &StreamServerEvent::Finished {
                                        reason: FinishReason::Cancelled,
                                        usage: None,
                                    },
                                )
                                .await
                                {
                                    return;
                                }
                                break None;
                            }
                            Some(Ok(Message::Close(_))) => {
                                run_cancellation.cancel();
                                let _ = socket.flush().await;
                                return;
                            }
                            None | Some(Err(_)) => {
                                run_cancellation.cancel();
                                return;
                            }
                            _ => {
                                if !send_wire_error(&mut socket, "run_in_progress", false).await {
                                    run_cancellation.cancel();
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        } else {
            Some(provider.stream(request, run_cancellation.clone()).await)
        };
        let Some(stream_result) = stream_result else {
            continue;
        };
        let mut events = match stream_result {
            Ok(events) => events,
            Err(error) => {
                log_model_error(&run_id, &error);
                if !send_model_error(&mut socket, mode, error.code(), error.partial, None).await {
                    break;
                }
                continue;
            }
        };

        let mut raw_reply = String::new();
        let mut visible_filter = VisibleTextFilter::default();
        let mut usage = None;
        let mut finish_reason = None;
        let mut stream_error = None;
        let mut cancelled = false;

        loop {
            let next_event = if mode == WireMode::Events {
                loop {
                    tokio::select! {
                        () = state.shutdown.cancelled() => {
                            run_cancellation.cancel();
                            return;
                        }
                        event = events.next() => break event,
                        incoming = socket.recv() => {
                            match incoming {
                                Some(Ok(Message::Text(message))) if is_cancel_message(&message) => {
                                    run_cancellation.cancel();
                                    cancelled = true;
                                    break None;
                                }
                                Some(Ok(Message::Close(_))) => {
                                    run_cancellation.cancel();
                                    let _ = socket.flush().await;
                                    return;
                                }
                                None | Some(Err(_)) => {
                                    run_cancellation.cancel();
                                    return;
                                }
                                _ => {
                                    if !send_wire_error(&mut socket, "run_in_progress", false).await {
                                        run_cancellation.cancel();
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                tokio::select! {
                    () = state.shutdown.cancelled() => {
                        run_cancellation.cancel();
                        return;
                    }
                    event = events.next() => event,
                }
            };

            if cancelled {
                break;
            }
            let Some(event) = next_event else {
                break;
            };
            match event {
                Ok(ModelEvent::ResponseStarted {
                    request_id, model, ..
                }) => {
                    if mode == WireMode::Events
                        && !send_wire(
                            &mut socket,
                            &StreamServerEvent::Started {
                                request_id: &request_id,
                                model: &model,
                            },
                        )
                        .await
                    {
                        run_cancellation.cancel();
                        return;
                    }
                }
                Ok(ModelEvent::TextDelta { text: delta }) => {
                    raw_reply.push_str(&delta);
                    if mode == WireMode::Events {
                        let visible = visible_filter.push(&delta);
                        if !visible.is_empty()
                            && !send_wire(&mut socket, &StreamServerEvent::Delta { text: &visible })
                                .await
                        {
                            run_cancellation.cancel();
                            return;
                        }
                    }
                }
                Ok(ModelEvent::Usage { usage: current }) => usage = Some(current),
                Ok(ModelEvent::Finish { reason }) => finish_reason = Some(reason),
                Ok(ModelEvent::ReasoningDelta { .. })
                | Ok(ModelEvent::ToolCallStarted { .. })
                | Ok(ModelEvent::ToolCallArgumentsDelta { .. })
                | Ok(ModelEvent::ToolCallCompleted { .. })
                | Ok(ModelEvent::ProviderWarning { .. }) => {}
                Err(error) => {
                    stream_error = Some(error);
                    break;
                }
            }
        }

        if cancelled {
            if !send_wire(
                &mut socket,
                &StreamServerEvent::Finished {
                    reason: FinishReason::Cancelled,
                    usage: usage.as_ref(),
                },
            )
            .await
            {
                break;
            }
            continue;
        }

        if let Some(error) = stream_error {
            log_model_error(&run_id, &error);
            if !send_model_error(&mut socket, mode, error.code(), error.partial, None).await {
                break;
            }
            continue;
        }

        if mode == WireMode::Events {
            let final_visible = visible_filter.finish();
            if !final_visible.is_empty()
                && !send_wire(
                    &mut socket,
                    &StreamServerEvent::Delta {
                        text: &final_visible,
                    },
                )
                .await
            {
                break;
            }
        }

        let clean_reply = memory::strip_all_tags(&raw_reply);
        if mode == WireMode::Legacy {
            if socket
                .send(Message::Text(clean_reply.clone().into()))
                .await
                .is_err()
            {
                break;
            }
        } else if !send_wire(
            &mut socket,
            &StreamServerEvent::Finished {
                reason: finish_reason.clone().unwrap_or(FinishReason::Unknown),
                usage: usage.as_ref(),
            },
        )
        .await
        {
            break;
        }

        tracing::info!(
            run_id,
            finish_reason = ?finish_reason,
            input_tokens = usage.as_ref().map(|usage| usage.input_tokens),
            output_tokens = usage.as_ref().map(|usage| usage.output_tokens),
            cached_input_tokens = usage.as_ref().map(|usage| usage.cached_input_tokens),
            "model run completed"
        );
        finalize_successful_reply(&state, &mut history, &text, &raw_reply, clean_reply).await;
    }
}

fn parse_user_message(frame: &str, mode: WireMode) -> Option<String> {
    match mode {
        WireMode::Legacy => Some(frame.to_string()),
        WireMode::Events => match serde_json::from_str::<StreamClientMessage>(frame).ok()? {
            StreamClientMessage::Message { text } if !text.trim().is_empty() => Some(text),
            StreamClientMessage::Message { .. } | StreamClientMessage::Cancel => None,
        },
    }
}

fn is_cancel_message(frame: &str) -> bool {
    matches!(
        serde_json::from_str::<StreamClientMessage>(frame),
        Ok(StreamClientMessage::Cancel)
    )
}

pub(crate) fn classify_route(text: &str) -> RouteKind {
    let lower = text.to_lowercase();
    if lower.contains("deep reasoning")
        || lower.contains("use pro")
        || text.contains("深入推理")
        || text.contains("复杂推理")
    {
        RouteKind::ComplexTask
    } else {
        RouteKind::Chat
    }
}

async fn add_retrieval_context(history: &mut Vec<ModelMessage>, text: &str) {
    let is_history_query = text.contains("之前")
        || text.contains("上次")
        || text.contains("我说过")
        || text.contains("记得");
    let retrieved = if is_history_query {
        let keyword = text
            .replace("之前", "")
            .replace("上次", "")
            .replace("我说过", "")
            .replace("记得", "")
            .replace("什么", "")
            .trim()
            .to_string();
        if keyword.is_empty() {
            Vec::new()
        } else {
            memory::search_history(&keyword, 3).await
        }
    } else {
        memory::search_related(text, 2).await
    };

    if !retrieved.is_empty() {
        history.push(ModelMessage::text(
            Role::System,
            format!(
                "{}\n{}",
                if is_history_query {
                    "[Wired Archive] Related records found:"
                } else {
                    "[Wired Echo] Related transmissions detected in archive:"
                },
                retrieved.join("\n---\n")
            ),
        ));
    }
}

async fn finalize_successful_reply(
    state: &AppState,
    history: &mut Vec<ModelMessage>,
    user_text: &str,
    raw_reply: &str,
    clean_reply: String,
) {
    for (key, value) in memory::extract_memory(raw_reply) {
        if let Err(error) = memory::update_profile(&key, &value).await {
            tracing::warn!(error = %error, key, "memory profile update failed");
        }
    }
    if let Some(raw_topic) = memory::extract_topic(raw_reply)
        && let Err(error) = memory::log_to_topic(&raw_topic, user_text, raw_reply).await
    {
        tracing::warn!(error = %error, topic = raw_topic, "topic log failed");
    }

    history.push(ModelMessage::text(Role::Assistant, clean_reply));
    if history.len() > 10 {
        *history = history.split_off(history.len() - 10);
    }

    let user_text = user_text.to_string();
    let lain_text = raw_reply.to_string();
    state.spawn_io_task("exchange_log", async move {
        memory::log_exchange(&user_text, &lain_text).await
    });
}

fn log_model_error(run_id: &str, error: &ModelError) {
    tracing::warn!(
        run_id,
        code = error.code(),
        retryable = error.is_retryable(),
        partial = error.partial,
        provider_request_id = error.provider_request_id,
        error = %error,
        "provider request failed"
    );
}

async fn send_model_error(
    socket: &mut WebSocket,
    mode: WireMode,
    code: &str,
    partial: bool,
    max_bytes: Option<usize>,
) -> bool {
    match mode {
        WireMode::Events => send_wire_error(socket, code, partial).await,
        WireMode::Legacy => {
            let message = max_bytes.map_or_else(
                || format!("...Noise...{code}"),
                |maximum| format!("...Noise...[{code}] maximum {maximum} bytes"),
            );
            socket.send(Message::Text(message.into())).await.is_ok()
        }
    }
}

async fn send_wire_error(socket: &mut WebSocket, code: &str, partial: bool) -> bool {
    send_wire(socket, &StreamServerEvent::Error { code, partial }).await
}

async fn send_wire(socket: &mut WebSocket, event: &StreamServerEvent<'_>) -> bool {
    let Ok(json) = serde_json::to_string(event) else {
        tracing::error!("stream wire event serialization failed");
        return false;
    };
    socket.send(Message::Text(json.into())).await.is_ok()
}

#[derive(Debug, Default)]
pub(crate) struct VisibleTextFilter {
    pending: String,
}

impl VisibleTextFilter {
    pub(crate) fn push(&mut self, delta: &str) -> String {
        const TAGS: [(&str, &str); 2] = [("<memory>", "</memory>"), ("<topic>", "</topic>")];
        self.pending.push_str(delta);
        let mut visible = String::new();

        loop {
            let next_tag = TAGS
                .iter()
                .filter_map(|(open, close)| {
                    self.pending.find(open).map(|index| (index, open, close))
                })
                .min_by_key(|(index, _, _)| *index);
            if let Some((index, open, close)) = next_tag {
                visible.push_str(&self.pending[..index]);
                self.pending.drain(..index + open.len());
                if let Some(end) = self.pending.find(close) {
                    self.pending.drain(..end + close.len());
                    continue;
                }
                self.pending.insert_str(0, open);
                break;
            }

            let hold = TAGS
                .iter()
                .map(|(open, _)| longest_suffix_prefix(&self.pending, open))
                .max()
                .unwrap_or(0);
            let emit_end = self.pending.len().saturating_sub(hold);
            visible.push_str(&self.pending[..emit_end]);
            self.pending.drain(..emit_end);
            break;
        }
        visible
    }

    pub(crate) fn finish(self) -> String {
        if self.pending.starts_with("<memory>") || self.pending.starts_with("<topic>") {
            String::new()
        } else {
            memory::strip_all_tags(&self.pending)
        }
    }
}

fn longest_suffix_prefix(value: &str, prefix: &str) -> usize {
    let maximum = value.len().min(prefix.len().saturating_sub(1));
    (1..=maximum)
        .rev()
        .find(|length| {
            value.is_char_boundary(value.len() - length)
                && prefix.starts_with(&value[value.len() - length..])
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, path::PathBuf, time::Duration};

    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    use crate::{
        config::{MemoryConfig, ProviderConfig},
        provider::mock::MockProvider,
    };

    use super::*;

    #[test]
    fn workspace_session_title_is_stable_without_exposing_the_path() {
        let root = StdPath::new(r"C:\Users\test\secret-project");
        let title = workspace_session_title(Some(root));
        assert_eq!(title, workspace_session_title(Some(root)));
        assert!(title.starts_with("workspace:"));
        assert!(!title.contains("secret-project"));
        if cfg!(windows) {
            assert_eq!(
                title,
                workspace_session_title(Some(StdPath::new(r"c:\users\TEST\SECRET-PROJECT")))
            );
        }
        assert_eq!(workspace_session_title(None), "Main");
    }

    fn test_state(ready: bool) -> AppState {
        let checks = vec![ReadinessCheck {
            name: "synthetic",
            ready,
            detail: "test fixture".to_string(),
        }];
        AppState {
            config: Arc::new(Config {
                bind_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
                system_prompt_path: PathBuf::from("synthetic-prompt"),
                identity_dir: PathBuf::from("synthetic-identity"),
                data_dir: PathBuf::from("synthetic-data"),
                workspace_root: None,
                provider: ProviderConfig::for_test(),
                shutdown_timeout: Duration::from_secs(1),
                max_message_bytes: 1024,
                memory: MemoryConfig {
                    extraction_enabled: false,
                    poll_interval: Duration::from_secs(1),
                    lease_duration: Duration::from_secs(30),
                    max_candidates_per_message: 8,
                },
                mcp: None,
            }),
            readiness: ReadinessState::from_checks(checks),
            provider: None,
            system_prompt: None,
            identity: None,
            storage: None,
            workspace_tools: None,
            mcp: None,
            router: ModelRouter::new("flash".to_string(), "pro".to_string()),
            tasks: TaskTracker::new(),
            shutdown: CancellationToken::new(),
        }
    }

    #[tokio::test]
    async fn memory_api_is_local_origin_scoped_and_returns_empty_overview() {
        let mut state = test_state(true);
        state.storage = Some(Storage::in_memory().await.expect("storage"));
        let response = build_router(state.clone())
            .oneshot(
                Request::builder()
                    .uri("/api/memory")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 16 * 1024)
            .await
            .expect("body");
        let value: serde_json::Value = serde_json::from_slice(&body).expect("JSON");
        assert_eq!(value["items"], serde_json::json!([]));
        assert_eq!(value["pending"], serde_json::json!([]));

        let forbidden = build_router(state)
            .oneshot(
                Request::builder()
                    .uri("/api/memory")
                    .header(ORIGIN, "https://untrusted.example")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn health_is_independent_from_provider_readiness() {
        let response = build_router(test_state(false))
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 1024)
            .await
            .expect("response body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("valid JSON");
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn ready_returns_service_unavailable_with_failed_checks() {
        let response = build_router(test_state(false))
            .oneshot(
                Request::builder()
                    .uri("/api/ready")
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(response.headers().contains_key("x-request-id"));
    }

    #[tokio::test]
    async fn provider_diagnostics_never_requires_a_secret() {
        let mut state = test_state(true);
        state.provider = Some(Arc::new(MockProvider::default()));
        let response = build_router(state)
            .oneshot(
                Request::builder()
                    .uri("/api/provider")
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 4096)
            .await
            .expect("response body");
        let text = String::from_utf8(body.to_vec()).expect("UTF-8 response");
        assert!(text.contains("supports_streaming"));
        assert!(!text.to_lowercase().contains("api_key"));
    }

    #[tokio::test]
    async fn websocket_rejects_untrusted_browser_origin_before_upgrade() {
        let response = build_router(test_state(false))
            .oneshot(
                Request::builder()
                    .uri("/ws")
                    .header(ORIGIN, "https://untrusted.example")
                    .header("connection", "upgrade")
                    .header("upgrade", "websocket")
                    .header("sec-websocket-version", "13")
                    .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("router response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn visible_filter_suppresses_hidden_tags_across_arbitrary_deltas() {
        let source = "在这里。<memory>name: Alice</memory>仍然在。<topic>test</topic>";
        for boundary in 1..source.len() {
            if !source.is_char_boundary(boundary) {
                continue;
            }
            let mut filter = VisibleTextFilter::default();
            let mut visible = filter.push(&source[..boundary]);
            visible.push_str(&filter.push(&source[boundary..]));
            visible.push_str(&filter.finish());
            assert_eq!(visible, "在这里。仍然在。");
        }
    }
}
