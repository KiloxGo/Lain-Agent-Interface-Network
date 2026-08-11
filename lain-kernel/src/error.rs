use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

use crate::{config::ConfigError, provider::ModelError};

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("{context}: {source}")]
    Io {
        context: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Provider(#[from] ModelError),
    #[error("service is not ready: {0}")]
    NotReady(String),
    #[error("WebSocket origin is not allowed")]
    ForbiddenOrigin,
    #[error("shutdown timed out after {0} seconds")]
    ShutdownTimeout(u64),
}

impl AppError {
    pub fn io(context: &'static str, source: std::io::Error) -> Self {
        Self::Io { context, source }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::Config(_) => "config_invalid",
            Self::Io { .. } => "io_error",
            Self::Provider(error) => error.code(),
            Self::NotReady(_) => "service_not_ready",
            Self::ForbiddenOrigin => "forbidden_origin",
            Self::ShutdownTimeout(_) => "shutdown_timeout",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Io { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Provider(_) => StatusCode::BAD_GATEWAY,
            Self::NotReady(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::ForbiddenOrigin => StatusCode::FORBIDDEN,
            Self::ShutdownTimeout(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        let message = match &self {
            Self::Config(_) => "service configuration is invalid".to_string(),
            Self::Io { .. } => "internal I/O operation failed".to_string(),
            Self::Provider(_) => "provider request failed".to_string(),
            Self::NotReady(failures) => format!("service is not ready: {failures}"),
            Self::ForbiddenOrigin => "WebSocket origin is not allowed".to_string(),
            Self::ShutdownTimeout(seconds) => {
                format!("shutdown timed out after {seconds} seconds")
            }
        };
        let body = ErrorBody {
            code: self.code(),
            message,
        };
        (status, Json(body)).into_response()
    }
}
