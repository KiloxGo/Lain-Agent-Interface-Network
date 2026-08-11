use std::{future::Future, sync::Arc};

use axum::{
    Json, Router,
    extract::{
        DefaultBodyLimit, Request as AxumRequest, State, WebSocketUpgrade, ws::Message,
        ws::WebSocket,
    },
    http::{
        HeaderMap, HeaderValue, Method, StatusCode,
        header::{CONTENT_TYPE, ORIGIN},
    },
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Serialize;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tower_http::{
    cors::CorsLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

use crate::{
    adapter::llm::{ChatMessage, LlmClient},
    config::Config,
    error::AppError,
    memory,
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
    pub llm: Option<Arc<LlmClient>>,
    pub tasks: TaskTracker,
    pub shutdown: CancellationToken,
}

impl AppState {
    pub async fn initialize(config: Config) -> Self {
        let mut checks = Vec::with_capacity(3);

        let memory_configured = memory::configure_data_dir(config.data_dir.clone());
        let memory_ready = match memory_configured {
            Ok(()) => match tokio::fs::create_dir_all(config.data_dir.join("memory")).await {
                Ok(()) => {
                    checks.push(ReadinessCheck {
                        name: "memory",
                        ready: true,
                        detail: "data directory is writable".to_string(),
                    });
                    true
                }
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
                    false
                }
            },
            Err(error) => {
                tracing::error!(error = %error, "data directory configuration failed");
                checks.push(ReadinessCheck {
                    name: "memory",
                    ready: false,
                    detail: "data directory configuration failed".to_string(),
                });
                false
            }
        };

        let system_prompt = match tokio::fs::read_to_string(&config.system_prompt_path).await {
            Ok(prompt) if !prompt.trim().is_empty() => {
                checks.push(ReadinessCheck {
                    name: "identity",
                    ready: true,
                    detail: "system prompt loaded".to_string(),
                });
                Some(prompt)
            }
            Ok(_) => {
                checks.push(ReadinessCheck {
                    name: "identity",
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
                    name: "identity",
                    ready: false,
                    detail: "system prompt is unavailable".to_string(),
                });
                None
            }
        };

        let provider_ready = config.provider.is_configured();
        checks.push(ReadinessCheck {
            name: "provider",
            ready: provider_ready,
            detail: if provider_ready {
                format!("{} configured", config.provider.model)
            } else {
                "DEEPSEEK_API_KEY is not configured".to_string()
            },
        });

        let identity_ready = system_prompt.is_some();
        let llm = if memory_ready && provider_ready && identity_ready {
            system_prompt.and_then(|prompt| match LlmClient::new(&config.provider, prompt) {
                Ok(client) => Some(Arc::new(client)),
                Err(error) => {
                    tracing::error!(error = %error, "provider client initialization failed");
                    None
                }
            })
        } else {
            None
        };

        if provider_ready
            && identity_ready
            && memory_ready
            && llm.is_none()
            && let Some(check) = checks.iter_mut().find(|check| check.name == "provider")
        {
            check.ready = false;
            check.detail = "provider client initialization failed".to_string();
        }

        Self {
            config: Arc::new(config),
            readiness: ReadinessState::from_checks(checks),
            llm,
            tasks: TaskTracker::new(),
            shutdown: CancellationToken::new(),
        }
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
        .allow_methods([Method::GET])
        .allow_headers([CONTENT_TYPE]);

    Router::new()
        .route("/api/health", get(health))
        .route("/api/ready", get(ready))
        .route(
            "/ws",
            get(ws_handler).route_layer(middleware::from_fn(validate_websocket_origin)),
        )
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
    if !state.readiness.is_ready() || state.llm.is_none() {
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
        .on_upgrade(move |socket| handle_socket(socket, state, request_id))
        .into_response();
    response
        .headers_mut()
        .insert("deprecation", HeaderValue::from_static("true"));
    Ok(response)
}

async fn handle_socket(mut socket: WebSocket, state: AppState, request_id: String) {
    let Some(client) = state.llm.clone() else {
        return;
    };
    let span = tracing::info_span!("websocket", request_id);
    let _guard = span.enter();
    let mut messages = Vec::new();

    loop {
        let incoming = tokio::select! {
            _ = state.shutdown.cancelled() => break,
            incoming = socket.recv() => incoming,
        };
        let Some(Ok(Message::Text(text))) = incoming else {
            break;
        };

        if text.len() > state.config.max_message_bytes {
            let error = format!(
                "...Noise...[message_too_large] maximum {} bytes",
                state.config.max_message_bytes
            );
            if socket.send(Message::Text(error.into())).await.is_err() {
                break;
            }
            continue;
        }

        let is_history_query = text.contains("之前")
            || text.contains("上次")
            || text.contains("我说过")
            || text.contains("记得");

        if is_history_query {
            let keyword = text
                .replace("之前", "")
                .replace("上次", "")
                .replace("我说过", "")
                .replace("记得", "")
                .replace("什么", "")
                .trim()
                .to_string();

            if !keyword.is_empty() {
                let memories = memory::search_history(&keyword, 3).await;
                if !memories.is_empty() {
                    messages.push(ChatMessage {
                        role: "system".to_string(),
                        content: format!(
                            "[Wired Archive] Related records found:\n{}",
                            memories.join("\n---\n")
                        ),
                    });
                }
            }
        } else {
            let related = memory::search_related(&text, 2).await;
            if !related.is_empty() {
                messages.push(ChatMessage {
                    role: "system".to_string(),
                    content: format!(
                        "[Wired Echo] Related transmissions detected in archive:\n{}",
                        related.join("\n---\n")
                    ),
                });
            }
        }

        messages.push(ChatMessage {
            role: "user".to_string(),
            content: text.to_string(),
        });

        match client.chat(messages.clone()).await {
            Ok(reply) => {
                for (key, value) in memory::extract_memory(&reply) {
                    if let Err(error) = memory::update_profile(&key, &value).await {
                        tracing::warn!(error = %error, key, "memory profile update failed");
                    }
                }

                if let Some(raw_topic) = memory::extract_topic(&reply)
                    && let Err(error) = memory::log_to_topic(&raw_topic, &text, &reply).await
                {
                    tracing::warn!(error = %error, topic = raw_topic, "topic log failed");
                }

                let clean_reply = memory::strip_all_tags(&reply);
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: clean_reply.clone(),
                });
                if messages.len() > 10 {
                    messages = messages.split_off(messages.len() - 10);
                }

                if socket
                    .send(Message::Text(clean_reply.into()))
                    .await
                    .is_err()
                {
                    break;
                }

                let user_text = text.to_string();
                let lain_text = reply;
                state.spawn_io_task("exchange_log", async move {
                    memory::log_exchange(&user_text, &lain_text).await
                });
            }
            Err(error) => {
                tracing::warn!(code = error.code(), error = %error, "provider request failed");
                let error_message = format!("...Noise...[{}]", error.code());
                if socket
                    .send(Message::Text(error_message.into()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, path::PathBuf, time::Duration};

    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    use crate::config::ProviderConfig;

    use super::*;

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
                data_dir: PathBuf::from("synthetic-data"),
                provider: ProviderConfig::for_test(),
                shutdown_timeout: Duration::from_secs(1),
                max_message_bytes: 1024,
            }),
            readiness: ReadinessState::from_checks(checks),
            llm: None,
            tasks: TaskTracker::new(),
            shutdown: CancellationToken::new(),
        }
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
}
