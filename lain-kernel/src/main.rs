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
