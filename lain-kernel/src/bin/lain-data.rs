use std::{path::PathBuf, process::ExitCode};

use lain_kernel::{
    config::Config,
    migration::{export_snapshot_markdown, import_exchanges, read_legacy_directory},
    storage::Storage,
};

#[tokio::main]
async fn main() -> ExitCode {
    let _ = dotenvy::dotenv();
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("lain-data: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let Some(command) = arguments.first().map(String::as_str) else {
        return Err("usage: lain-data <import-markdown|export-markdown> <path> [--dry-run]".into());
    };
    let Some(path) = arguments.get(1).map(PathBuf::from) else {
        return Err("a source or destination path is required".into());
    };
    let config = Config::load()?;
    match command {
        "import-markdown" => {
            let exchanges = read_legacy_directory(&path).await?;
            if arguments.iter().any(|argument| argument == "--dry-run") {
                serde_json::to_writer_pretty(
                    std::io::stdout().lock(),
                    &serde_json::json!({ "parsed": exchanges.len(), "dry_run": true }),
                )?;
                return Ok(());
            }
            let storage = Storage::initialize(&config.data_dir).await?;
            let report = import_exchanges(&storage, &exchanges).await?;
            serde_json::to_writer_pretty(std::io::stdout().lock(), &report)?;
        }
        "export-markdown" => {
            let storage = Storage::initialize(&config.data_dir).await?;
            let snapshot = storage.snapshot(&storage.scope().session_id).await?;
            let markdown = export_snapshot_markdown(&snapshot);
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::write(path, markdown).await?;
        }
        _ => return Err(format!("unknown command: {command}").into()),
    }
    Ok(())
}
