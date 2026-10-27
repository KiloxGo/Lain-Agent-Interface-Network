use std::sync::Arc;

use anyhow::{Context, Result};
use lain_kernel::{api, backend::Supervisor, config::Config, kernel::Kernel};

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
    let channel_config = lain_kernel::channels::napcat::NapCatConfig::load()?;
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
    let workers = Supervisor::new(
        kernel.clone(),
        config.codex_path,
        config.api_token.clone(),
        config.worker_idle_timeout,
    )
    .await;
    workers.recover().await;
    let actor = lain_kernel::actor::Actor::from_environment(
        kernel.clone(),
        Some(workers.clone()),
        &config.api_token,
    );
    let app = api::actor_router(
        kernel.clone(),
        config.api_token.clone(),
        Some(workers.clone()),
        actor.clone(),
    );
    let app = if let Some(directory) = config.web_dir {
        api::with_web(app, directory)
    } else {
        app
    };
    kernel.mark_ready()?;
    let reading_maintenance = tokio::spawn(lain_kernel::reading::maintain(kernel.clone()));
    let archive_maintenance = tokio::spawn(lain_kernel::archives::maintain(kernel.clone()));
    let knowledge_sync = tokio::spawn(lain_kernel::knowledge::maintain(
        kernel.clone(),
        actor.clone(),
    ));
    let channel_tasks = if let Some(channel_config) = channel_config {
        let binding =
            kernel.channel_bind(channel_config.binding_spec(), channel_config.allow_reply)?;
        let service = lain_kernel::channels::ChannelService::new(
            kernel.clone(),
            actor,
            binding,
            &config.api_token,
        );
        let (channel, transport) = lain_kernel::channels::napcat::start(
            channel_config,
            service.clone(),
            kernel.subscribe_shutdown(),
            &config.api_token,
        )?;
        let stop_transport = transport.abort_handle();
        let application = tokio::spawn(async move {
            struct Guard(tokio::task::AbortHandle);
            impl Drop for Guard {
                fn drop(&mut self) {
                    self.0.abort();
                }
            }
            let _guard = Guard(stop_transport);
            if service.run(channel).await.is_err() {
                tracing::error!("channel application stopped; inspect durable channel state");
                // Do not keep accepting input after the durable consumer has failed.
            }
        });
        Some((transport, application))
    } else {
        None
    };
    let result = api::serve_router(listener, kernel.clone(), app, shutdown).await;
    let _ = reading_maintenance.await;
    let _ = archive_maintenance.await;
    let _ = knowledge_sync.await;
    if let Some((transport, application)) = channel_tasks {
        // Wake transport first, preserving unknown state for any in-flight send.
        let _ = transport.await;
        let _ = application.await;
    }
    let knowledge_shutdown = lain_kernel::knowledge::drain(&kernel).await;
    let worker_shutdown = workers.shutdown().await;
    drop(workers);
    drop(kernel);
    match &result {
        Ok(()) => tracing::info!("laind stopped; storage closed"),
        Err(error) => tracing::error!(%error, "laind stopped with an error"),
    }
    result.and(knowledge_shutdown).and(worker_shutdown)
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
