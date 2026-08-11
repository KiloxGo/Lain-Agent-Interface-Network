use std::process::ExitCode;

use lain_kernel::config::Config;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> ExitCode {
    let _ = dotenvy::dotenv();
    init_tracing();

    let config = match Config::load() {
        Ok(config) => config,
        Err(error) => {
            tracing::error!(code = "config_invalid", error = %error, "configuration failed");
            return ExitCode::FAILURE;
        }
    };

    if std::env::args_os().any(|argument| argument == "--diagnose-provider") {
        let diagnostics = lain_kernel::provider::provider_diagnostics(
            &config.provider,
            config.provider.is_configured(),
        );
        if let Err(error) = serde_json::to_writer_pretty(std::io::stdout().lock(), &diagnostics) {
            tracing::error!(error = %error, "provider diagnostics serialization failed");
            return ExitCode::FAILURE;
        }
        return ExitCode::SUCCESS;
    }

    match lain_kernel::run(config).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(code = error.code(), error = %error, "kernel stopped with an error");
            ExitCode::FAILURE
        }
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("lain_kernel=info,tower_http=info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}
