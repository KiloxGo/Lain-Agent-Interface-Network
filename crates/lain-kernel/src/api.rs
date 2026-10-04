use std::{future::Future, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use axum::{
    Json, Router,
    extract::{Path, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Serialize;
use tokio::net::TcpListener;

use crate::{
    config::ApiToken,
    domain::{WorkSession, WorkSessionId},
    kernel::{Kernel, KernelState},
};

pub fn router(kernel: Arc<Kernel>, token: ApiToken) -> Router {
    Router::new()
        .route(
            "/health",
            get(|| async { Json(serde_json::json!({"status": "ok"})) }),
        )
        .route("/ready", get(readiness))
        .route("/v1/work-sessions", get(list_sessions))
        .route("/v1/work-sessions/{id}", get(inspect_session))
        .fallback(|| async { ApiError(StatusCode::NOT_FOUND, "not_found") })
        .with_state(kernel)
        .layer(middleware::from_fn_with_state(token, authenticate))
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
    let state = kernel.state().map_err(internal_error)?;
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
    kernel.list_sessions().map(Json).map_err(internal_error)
}

async fn inspect_session(
    State(kernel): State<Arc<Kernel>>,
    Path(raw_id): Path<String>,
) -> Result<Json<WorkSession>, ApiError> {
    let id: WorkSessionId = raw_id
        .parse()
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "invalid_session_id"))?;
    kernel
        .session(id)
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
    let app = router(kernel.clone(), token);
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
