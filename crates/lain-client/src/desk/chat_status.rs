//! Read-only Kernel chat configuration, separate from transport/provider access.
use super::*;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Availability {
    #[default]
    Unknown,
    Configured,
    NotConfigured,
    CredentialMissing,
    InvalidConfiguration,
}
impl Availability {
    fn parse(value: &Value) -> Self {
        match value["actor"]["status"].as_str() {
            Some("configured" | "test_model") => Self::Configured,
            Some("not_configured") => Self::NotConfigured,
            Some("credential_missing") => Self::CredentialMissing,
            Some("invalid_configuration") => Self::InvalidConfiguration,
            _ => Self::Unknown,
        }
    }
    pub fn hint(self) -> Option<&'static str> {
        match self {
            Self::NotConfigured => Some(
                "Chat model is not configured. Configure the Kernel, then Ctrl-R. Your draft stays; Work/Devices remain available.",
            ),
            Self::CredentialMissing => Some(
                "Chat credential is missing on the Kernel. Check its configuration, then Ctrl-R. Your draft stays; Work/Devices remain available.",
            ),
            Self::InvalidConfiguration => Some(
                "Chat configuration is invalid on the Kernel. Correct it, then Ctrl-R. Your draft stays; Work/Devices remain available.",
            ),
            _ => None,
        }
    }
    pub fn badge(self) -> Option<&'static str> {
        match self {
            Self::NotConfigured => Some("Chat model not configured · Tab Work/Devices"),
            Self::CredentialMissing => Some("Chat credential missing · Tab Work/Devices"),
            Self::InvalidConfiguration => Some("Chat configuration invalid · Tab Work/Devices"),
            _ => None,
        }
    }
}
pub(super) async fn read(client: &Client) -> Result<Availability> {
    let value: Value = tokio::time::timeout(Duration::from_secs(3), client.get("v1/diagnostics"))
        .await
        .context("Chat configuration check timed out")??;
    Ok(Availability::parse(&value))
}
pub(super) fn apply(app: &mut Model, result: Result<Availability>) {
    app.chat_checking = false;
    app.chat_status = result.unwrap_or_default();
    if app.pending.is_none() && !app.active() && !app.request_busy {
        app.status=app.chat_status.hint().unwrap_or(match app.chat_status {
            Availability::Unknown => "Chat configuration status unavailable. Ctrl-R checks again; no message was submitted.",
            _ => "Ready. /help for commands",
        }).into();
    }
}
pub(super) fn refresh(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    jobs: &mut JoinSet<()>,
) {
    if app.chat_checking {
        return;
    }
    app.chat_checking = true;
    let c = client.clone();
    let out = live::read_sender(app, tx, jobs);
    live::spawn_read(app, jobs, async move {
        let _ = out.send(Complete::ChatStatus(read(&c).await)).await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostics_are_allowlisted_and_never_render_raw_help_or_secrets() {
        for (status, expected) in [
            ("not_configured", Availability::NotConfigured),
            ("credential_missing", Availability::CredentialMissing),
            ("invalid_configuration", Availability::InvalidConfiguration),
            ("configured", Availability::Configured),
            ("test_model", Availability::Configured),
        ] {
            let value = json!({"actor":{"status":status,"help":"untrusted raw value","secret_ref":"untrusted raw value"}});
            let actual = Availability::parse(&value);
            assert!(actual == expected);
            assert!(!actual.hint().unwrap_or_default().contains("untrusted"));
        }
        for value in [json!({}), json!({"actor":{"status":"untrusted raw value"}})] {
            assert!(Availability::parse(&value) == Availability::Unknown);
        }
    }
    #[tokio::test]
    async fn status_read_only_gets_diagnostics_without_provider_or_worker_requests() {
        use axum::{Json, Router, routing::get};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new().route(
                    "/v1/diagnostics",
                    get(|| async { Json(json!({"actor":{"status":"not_configured"}})) }),
                ),
            )
            .await
            .unwrap();
        });
        let c = Client::new(
            &format!("http://{address}"),
            "fixture-chat-status-test-credential",
        )
        .unwrap();
        assert!(read(&c).await.unwrap() == Availability::NotConfigured);
        server.abort();
        assert!(read(&c).await.is_err());
    }
}
