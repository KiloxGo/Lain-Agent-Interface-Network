use std::{path::PathBuf, process::ExitCode};

use lain_kernel::{
    config::Config,
    identity::IdentityKernel,
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
        return Err(usage().into());
    };
    match command {
        "import-markdown" => {
            let path = required_path(&arguments, 1, "a source path is required")?;
            let config = Config::load()?;
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
            let path = required_path(&arguments, 1, "a destination path is required")?;
            let config = Config::load()?;
            let storage = Storage::initialize(&config.data_dir).await?;
            let snapshot = storage.snapshot(&storage.scope().session_id).await?;
            let markdown = export_snapshot_markdown(&snapshot);
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::write(path, markdown).await?;
        }
        "show-context-plan" => {
            let run_id = arguments.get(1).ok_or("a run id is required")?;
            let config = Config::load()?;
            let storage = Storage::initialize(&config.data_dir).await?;
            let plan = storage
                .load_context_plan(run_id)
                .await?
                .ok_or_else(|| format!("context plan not found for run {run_id}"))?;
            serde_json::to_writer_pretty(std::io::stdout().lock(), &plan)?;
        }
        "identity-diff" => {
            let old_path = required_path(&arguments, 1, "the old identity path is required")?;
            let new_path = required_path(&arguments, 2, "the new identity path is required")?;
            let old = IdentityKernel::load(old_path).await?;
            let new = IdentityKernel::load(new_path).await?;
            serde_json::to_writer_pretty(std::io::stdout().lock(), &old.diff(&new))?;
        }
        _ => return Err(format!("unknown command: {command}").into()),
    }
    Ok(())
}

fn required_path(
    arguments: &[String],
    index: usize,
    message: &'static str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    arguments
        .get(index)
        .map(PathBuf::from)
        .ok_or_else(|| message.into())
}

fn usage() -> &'static str {
    "usage: lain-data <import-markdown|export-markdown> <path> [--dry-run]\n       lain-data show-context-plan <run-id>\n       lain-data identity-diff <old-dir> <new-dir>"
}
