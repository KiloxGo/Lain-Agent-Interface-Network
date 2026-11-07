use anyhow::{Context, Result, ensure};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("device host stopped: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
async fn run() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let first = args
        .next()
        .context("use lain-device-agent --directory <protected-device-directory>")?;
    if first == "--help" {
        println!(
            "lain-device-agent [--check] --directory <protected-device-directory>\n--check prints read-only JSON prerequisites; does not start service or Worker.\nNative Worker service; explicit host-policy.json required. Ctrl-C/Ctrl-Break stops owned runtimes."
        );
        return Ok(());
    }
    let check = first == "--check";
    ensure!(
        first == "--directory" || (check && args.next().is_some_and(|a| a == "--directory")),
        "expected [--check] --directory"
    );
    let directory = PathBuf::from(args.next().context("device directory required")?);
    ensure!(args.next().is_none(), "unexpected device host arguments");
    if check {
        println!(
            "{}",
            serde_json::to_string_pretty(&lain_kernel::device_host::check(directory).await?)?
        );
        return Ok(());
    }
    let shutdown = shutdown_signal()?;
    tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::WARN)
        .with_writer(std::io::stderr)
        .init();
    lain_kernel::device_host::serve(directory, shutdown).await
}
#[cfg(unix)]
fn shutdown_signal() -> Result<impl Future<Output = ()>> {
    use tokio::signal::unix::{SignalKind, signal};
    let mut interrupt = signal(SignalKind::interrupt())?;
    let mut terminate = signal(SignalKind::terminate())?;
    Ok(async move {
        tokio::select! {_=interrupt.recv()=>{},_=terminate.recv()=>{}}
    })
}
#[cfg(windows)]
fn shutdown_signal() -> Result<impl Future<Output = ()>> {
    let mut interrupt = tokio::signal::windows::ctrl_c()?;
    let mut terminate = tokio::signal::windows::ctrl_break()?;
    Ok(async move {
        tokio::select! {_=interrupt.recv()=>{},_=terminate.recv()=>{}}
    })
}
