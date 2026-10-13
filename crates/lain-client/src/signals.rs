//! Genuine native shutdown signals shared by the launcher and text client.
use anyhow::Result;

#[cfg(unix)]
pub(crate) fn shutdown() -> Result<impl Future<Output = ()> + Send + 'static> {
    use tokio::signal::unix::{SignalKind, signal};
    let mut interrupt = signal(SignalKind::interrupt())?;
    let mut terminate = signal(SignalKind::terminate())?;
    Ok(async move {
        tokio::select! { _=interrupt.recv()=>{}, _=terminate.recv()=>{} }
    })
}
#[cfg(windows)]
pub(crate) fn shutdown() -> Result<impl Future<Output = ()> + Send + 'static> {
    let mut interrupt = tokio::signal::windows::ctrl_c()?;
    let mut terminate = tokio::signal::windows::ctrl_break()?;
    Ok(async move {
        tokio::select! { _=interrupt.recv()=>{}, _=terminate.recv()=>{} }
    })
}
