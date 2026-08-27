pub mod app;
pub mod config;
pub mod context;
pub mod domain;
pub mod error;
pub mod identity;
pub mod mcp;
pub mod memory;
pub mod migration;
pub mod protocol;
pub mod provider;
pub mod storage;
pub mod tools;
pub mod transport;

use app::{AppState, build_router};
use config::Config;
use error::AppError;
use std::future::Future;
use tokio::{net::TcpListener, time::timeout};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

pub async fn run(config: Config) -> Result<(), AppError> {
    run_until(config, shutdown_signal()).await
}

async fn run_until<S>(config: Config, signal: S) -> Result<(), AppError>
where
    S: Future<Output = ()>,
{
    let bind_addr = config.bind_addr;
    let shutdown_timeout = config.shutdown_timeout;
    let state = AppState::initialize(config).await;
    let shutdown = state.shutdown.clone();
    let tasks = state.tasks.clone();
    let app = build_router(state);

    let listener = TcpListener::bind(bind_addr)
        .await
        .map_err(|error| AppError::io("failed to bind HTTP listener", error))?;
    tracing::info!(%bind_addr, "lain kernel listening");

    let server_shutdown = CancellationToken::new();
    let server = axum::serve(listener, app)
        .with_graceful_shutdown(server_shutdown.clone().cancelled_owned())
        .into_future();
    tokio::pin!(server);

    tokio::select! {
        result = &mut server => {
            result.map_err(|error| AppError::io("HTTP server failed", error))?;
            shutdown.cancel();
            drain_tasks(tasks, shutdown_timeout).await
        }
        () = signal => {
            shutdown.cancel();
            tasks.close();
            server_shutdown.cancel();

            timeout(shutdown_timeout, async {
                (&mut server)
                    .await
                    .map_err(|error| AppError::io("HTTP server failed during shutdown", error))?;
                tasks.wait().await;
                Ok::<(), AppError>(())
            })
            .await
            .map_err(|_| AppError::ShutdownTimeout(shutdown_timeout.as_secs()))?
        }
    }
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        match signal(SignalKind::terminate()) {
            Ok(mut terminate) => {
                tokio::select! {
                    result = tokio::signal::ctrl_c() => {
                        if let Err(error) = result {
                            tracing::error!(error = %error, "failed to listen for Ctrl+C");
                        }
                    }
                    _ = terminate.recv() => {}
                }
            }
            Err(error) => {
                tracing::warn!(error = %error, "failed to install SIGTERM handler");
                if let Err(error) = tokio::signal::ctrl_c().await {
                    tracing::error!(error = %error, "failed to listen for Ctrl+C");
                }
            }
        }
    }

    #[cfg(not(unix))]
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(error = %error, "failed to listen for Ctrl+C");
    }

    tracing::info!("shutdown signal received");
}

async fn drain_tasks(tasks: TaskTracker, wait: std::time::Duration) -> Result<(), AppError> {
    tasks.close();
    timeout(wait, tasks.wait())
        .await
        .map_err(|_| AppError::ShutdownTimeout(wait.as_secs()))
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, path::PathBuf, time::Duration};

    use tempfile::TempDir;

    use crate::config::{McpConfig, ProviderConfig};

    use super::*;

    #[tokio::test]
    async fn tracked_tasks_drain_before_timeout() {
        let tasks = TaskTracker::new();
        tasks.spawn(async { tokio::task::yield_now().await });
        drain_tasks(tasks, std::time::Duration::from_secs(1))
            .await
            .expect("tasks should drain");
    }

    #[tokio::test]
    async fn server_accepts_injected_shutdown_and_exits_cleanly() {
        let fixture = TempDir::new().expect("temporary fixture");
        let prompt_path = fixture.path().join("prompt.md");
        std::fs::write(&prompt_path, "Synthetic identity.").expect("write prompt fixture");
        let config = Config {
            bind_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
            system_prompt_path: prompt_path,
            identity_dir: fixture.path().join("identity"),
            data_dir: fixture.path().join("data"),
            workspace_root: None,
            provider: ProviderConfig::for_test(),
            shutdown_timeout: Duration::from_secs(1),
            max_message_bytes: 1024,
            memory: crate::config::MemoryConfig {
                extraction_enabled: false,
                poll_interval: Duration::from_secs(1),
                lease_duration: Duration::from_secs(30),
                max_candidates_per_message: 8,
            },
            mcp: Some(McpConfig {
                name: "fixture".to_string(),
                command: "node".to_string(),
                args: vec![
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("../evals/fixtures/mcp-stdio-server.mjs")
                        .to_string_lossy()
                        .into_owned(),
                    "modern".to_string(),
                ],
            }),
        };

        run_until(config, async {})
            .await
            .expect("server should shut down cleanly");
    }
}
