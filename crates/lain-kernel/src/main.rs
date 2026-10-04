use std::sync::Arc;

use anyhow::{Context, Result};
use lain_kernel::{api, config::Config, kernel::Kernel};

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            // Config errors contain setting names, never their supplied values.
            eprintln!("laind failed: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<()> {
    let config = Config::load().context("load daemon configuration")?;
    tracing_subscriber::fmt()
        .json()
        .with_max_level(config.log_level)
        .with_writer(std::io::stderr)
        .init();
    tracing::info!(state = "booting", "laind starting");
    // Register signals before opening storage or announcing readiness.
    let shutdown = shutdown_signal()?;
    let kernel = Arc::new(
        tokio::task::spawn_blocking(move || Kernel::open(&config.database_path))
            .await
            .context("storage startup task failed")??,
    );
    let listener = tokio::net::TcpListener::bind(config.listen)
        .await
        .context("bind local API")?;
    tracing::info!(address = %listener.local_addr()?, "API listener bound");
    let result = api::serve(listener, kernel.clone(), config.api_token, shutdown).await;
    drop(kernel);
    match &result {
        Ok(()) => tracing::info!("laind stopped; storage closed"),
        Err(error) => tracing::error!(%error, "laind stopped with an error"),
    }
    result
}

#[cfg(unix)]
fn shutdown_signal() -> Result<impl Future<Output = ()> + Send> {
    use tokio::signal::unix::{SignalKind, signal};
    let mut interrupt = signal(SignalKind::interrupt()).context("register SIGINT")?;
    let mut terminate = signal(SignalKind::terminate()).context("register SIGTERM")?;
    Ok(async move {
        tokio::select! { _ = interrupt.recv() => {}, _ = terminate.recv() => {} }
    })
}

#[cfg(windows)]
fn shutdown_signal() -> Result<impl Future<Output = ()> + Send> {
    let mut interrupt = tokio::signal::windows::ctrl_c().context("register Ctrl-C")?;
    let mut terminate = tokio::signal::windows::ctrl_break().context("register Ctrl-Break")?;
    Ok(async move {
        tokio::select! { _ = interrupt.recv() => {}, _ = terminate.recv() => {} }
    })
}
