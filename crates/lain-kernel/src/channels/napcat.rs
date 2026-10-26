//! OneBot 11 forward-WebSocket text adapter. All QQ identities/protocol live here.
use super::{ChannelService, ConversationChannel, Outgoing, domain::*};
use crate::profiles::{SecretRef, SecretValue, resolve_secret};
use anyhow::{Context, Result, ensure};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{future::Future, io::Read, net::IpAddr, pin::Pin, sync::Arc, time::Duration};
use tokio::sync::{mpsc, oneshot, watch};
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{Message, client::IntoClientRequest, protocol::WebSocketConfig},
};
use tracing::instrument::WithSubscriber;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NapCatConfig {
    pub name: String,
    pub endpoint: String,
    pub self_id: String,
    pub user_id: String,
    pub secret_ref: SecretRef,
    /// Initial grant only. A persisted revocation requires explicit API re-enablement.
    pub allow_reply: bool,
}
impl NapCatConfig {
    pub fn load() -> Result<Option<Self>> {
        let Some(path) = std::env::var_os("LAIN_NAPCAT_CONFIG") else {
            return Ok(None);
        };
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .context("open channel configuration")?
            .take(8193)
            .read_to_end(&mut bytes)
            .context("read channel configuration")?;
        ensure!(bytes.len() <= 8192, "channel configuration too large");
        let value: Self = serde_json::from_slice(&bytes)
            .map_err(|_| anyhow::anyhow!("invalid NapCat configuration"))?;
        value.validate()?;
        Ok(Some(value))
    }
    pub fn binding_spec(&self) -> BindingSpec {
        BindingSpec {
            name: self.name.clone(),
            adapter: "napcat-onebot11".into(),
            account: self.self_id.clone(),
            peer: self.user_id.clone(),
        }
    }
    pub fn validate(&self) -> Result<()> {
        self.binding_spec().validate()?;
        ensure!(
            valid_account(&self.self_id) && valid_account(&self.user_id),
            "invalid channel account"
        );
        let endpoint = url::Url::parse(&self.endpoint)
            .map_err(|_| anyhow::anyhow!("invalid channel endpoint"))?;
        let loopback = endpoint.host_str().is_some_and(|h| {
            h.trim_matches(['[', ']'])
                .parse::<IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
        });
        ensure!(
            endpoint.scheme() == "wss" || (endpoint.scheme() == "ws" && loopback),
            "channel requires TLS or literal loopback"
        );
        ensure!(
            endpoint.host_str().is_some()
                && endpoint.username().is_empty()
                && endpoint.password().is_none()
                && endpoint.query().is_none()
                && endpoint.fragment().is_none(),
            "channel endpoint cannot contain credentials/query/fragment"
        );
        crate::profiles::check_sensitive(&self.binding_spec())?;
        Ok(())
    }
}
fn valid_account(value: &str) -> bool {
    value
        .parse::<u64>()
        .is_ok_and(|n| n > 0 && n.to_string() == value)
}
fn native_id(value: &Value) -> Option<String> {
    match value {
        Value::Number(n) => n
            .as_i64()
            .map(|n| n.to_string())
            .or_else(|| n.as_u64().map(|n| n.to_string())),
        Value::String(s) if !s.is_empty() && s.len() <= 32 => s
            .parse::<i64>()
            .ok()
            .map(|n| n.to_string())
            .or_else(|| s.parse::<u64>().ok().map(|n| n.to_string())),
        _ => None,
    }
}
/// Unsupported mixed media is rejected as a whole, never silently interpreted as full text.
pub fn parse_private(config: &NapCatConfig, event: &Value) -> Option<Input> {
    if event.get("post_type")?.as_str()? != "message"
        || event.get("message_type")?.as_str()? != "private"
        || event.get("sub_type")?.as_str()? != "friend"
        || native_id(event.get("self_id")?)? != config.self_id
        || native_id(event.get("user_id")?)? != config.user_id
        || event.get("group_id").is_some()
    {
        return None;
    }
    if event
        .get("sender")
        .and_then(|s| s.get("user_id"))
        .is_some_and(|id| native_id(id).as_deref() != Some(config.user_id.as_str()))
    {
        return None;
    }
    let segments = event.get("message")?.as_array()?;
    if segments.is_empty() || segments.len() > 64 {
        return None;
    }
    let mut text = String::new();
    for segment in segments {
        if segment.get("type")?.as_str()? != "text" {
            return None;
        }
        text.push_str(segment.get("data")?.get("text")?.as_str()?);
        if text.len() > MAX_TEXT {
            return None;
        }
    }
    let input = Input {
        account: config.self_id.clone(),
        peer: config.user_id.clone(),
        external_id: native_id(event.get("message_id")?)?,
        text,
    };
    input.validate().ok()?;
    Some(input)
}
struct SendCommand {
    delivery: Outgoing,
    result: oneshot::Sender<SendOutcome>,
}
pub struct NapCatChannel {
    sender: mpsc::Sender<SendCommand>,
    connected: watch::Receiver<bool>,
}
impl ConversationChannel for NapCatChannel {
    fn connected(&self) -> watch::Receiver<bool> {
        self.connected.clone()
    }
    fn send(&self, delivery: Delivery) -> Pin<Box<dyn Future<Output = SendOutcome> + Send + '_>> {
        self.enqueue(Outgoing::Reply(delivery))
    }
    fn send_share(
        &self,
        delivery: lain_wire::sharing::ShareDelivery,
    ) -> Pin<Box<dyn Future<Output = SendOutcome> + Send + '_>> {
        self.enqueue(Outgoing::Share(delivery))
    }
}
impl NapCatChannel {
    fn enqueue(
        &self,
        delivery: Outgoing,
    ) -> Pin<Box<dyn Future<Output = SendOutcome> + Send + '_>> {
        Box::pin(async move {
            if !*self.connected.borrow() {
                return SendOutcome::NotSent;
            }
            let (result, receive) = oneshot::channel();
            if self
                .sender
                .try_send(SendCommand { delivery, result })
                .is_err()
            {
                return SendOutcome::NotSent;
            }
            receive.await.unwrap_or(SendOutcome::Unknown)
        })
    }
}
pub fn start(
    config: NapCatConfig,
    service: Arc<ChannelService>,
    shutdown: watch::Receiver<bool>,
    api_token: &crate::config::ApiToken,
) -> Result<(Arc<NapCatChannel>, tokio::task::JoinHandle<()>)> {
    config.validate()?;
    let token = resolve_secret(&config.secret_ref)?;
    ensure!(
        !api_token.matches(token.0.as_str()),
        "channel and Kernel credentials must differ"
    );
    Ok(start_authenticated(config, service, shutdown, token))
}
pub(super) fn start_authenticated(
    config: NapCatConfig,
    service: Arc<ChannelService>,
    shutdown: watch::Receiver<bool>,
    token: SecretValue,
) -> (Arc<NapCatChannel>, tokio::task::JoinHandle<()>) {
    let (sender, receiver) = mpsc::channel(1);
    let (connected, state) = watch::channel(false);
    let channel = Arc::new(NapCatChannel {
        sender,
        connected: state,
    });
    let task = tokio::spawn(
        connection_loop(config, service, receiver, connected, shutdown, token)
            .with_current_subscriber(),
    );
    (channel, task)
}
async fn connection_loop(
    config: NapCatConfig,
    service: Arc<ChannelService>,
    mut receiver: mpsc::Receiver<SendCommand>,
    connected: watch::Sender<bool>,
    mut shutdown: watch::Receiver<bool>,
    token: SecretValue,
) {
    struct Guard {
        service: Arc<ChannelService>,
        connected: watch::Sender<bool>,
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            self.connected.send_replace(false);
            self.service.connection(ConnectionPhase::Stopped, None);
        }
    }
    let _guard = Guard {
        service: service.clone(),
        connected: connected.clone(),
    };
    let mut backoff = 1;
    let mut last_failure = None;
    loop {
        if *shutdown.borrow() {
            break;
        }
        let result = tokio::select! {
            _ = shutdown.changed() => break,
            result = session(&config, &service, &mut receiver, &connected, &token) => result,
        };
        let was_online = *connected.borrow();
        connected.send_replace(false);
        while let Ok(command) = receiver.try_recv() {
            let _ = command.result.send(SendOutcome::NotSent);
        }
        if let Err(error) = result {
            let failure = error
                .downcast_ref::<ConnectionFailure>()
                .copied()
                .unwrap_or(ConnectionFailure::Disconnected);
            service.connection(ConnectionPhase::Reconnecting, Some(failure));
            if was_online || last_failure != Some(failure) {
                tracing::warn!(channel = %service.id(), ?failure, "channel unavailable; inspect channel status; source coverage may have a gap");
            }
            last_failure = Some(failure);
        }
        tokio::select! { _ = shutdown.changed() => break, _ = tokio::time::sleep(Duration::from_secs(backoff)) => {} }
        backoff = (backoff * 2).min(30);
    }
    connected.send_replace(false);
}
async fn session(
    config: &NapCatConfig,
    service: &ChannelService,
    receiver: &mut mpsc::Receiver<SendCommand>,
    connected: &watch::Sender<bool>,
    token: &SecretValue,
) -> Result<()> {
    config
        .validate()
        .map_err(|_| ConnectionFailure::ProtocolError)?;
    service.connection(ConnectionPhase::Connecting, None);
    let recipient = config
        .user_id
        .parse::<u64>()
        .map_err(|_| anyhow::anyhow!("invalid channel account"))?;
    let mut request = config
        .endpoint
        .as_str()
        .into_client_request()
        .map_err(|_| anyhow::anyhow!("invalid WebSocket request"))?;
    let mut auth = format!("Bearer {}", token.0.as_str())
        .parse::<tokio_tungstenite::tungstenite::http::HeaderValue>()
        .map_err(|_| anyhow::anyhow!("invalid channel credential"))?;
    auth.set_sensitive(true);
    request.headers_mut().insert("Authorization", auth);
    let ws_config = WebSocketConfig::default()
        .max_message_size(Some(64 * 1024))
        .max_frame_size(Some(64 * 1024));
    let (mut socket, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_async_with_config(request, Some(ws_config), false),
    )
    .await
    .map_err(|_| ConnectionFailure::ConnectionUnavailable)?
    .map_err(|error| match error {
        tokio_tungstenite::tungstenite::Error::Http(response)
            if matches!(response.status().as_u16(), 401 | 403) =>
        {
            ConnectionFailure::AuthenticationFailed
        }
        tokio_tungstenite::tungstenite::Error::Protocol(_) => ConnectionFailure::ProtocolError,
        _ => ConnectionFailure::ConnectionUnavailable,
    })?;
    service.connection(ConnectionPhase::Authenticating, None);
    let identity_echo = uuid::Uuid::new_v4().to_string();
    bounded_write(
        socket.send(Message::Text(
            json!({"action":"get_login_info","params":{},"echo":identity_echo})
                .to_string()
                .into(),
        )),
    )
    .await?;
    // Do not ingest events until the connection proves its account. Bound pre-auth buffer.
    let identity_deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let mut buffered = Vec::new();
    loop {
        let frame = tokio::time::timeout_at(identity_deadline, socket.next())
            .await
            .map_err(|_| ConnectionFailure::IdentityTimeout)?
            .context("channel closed")?
            .map_err(|_| anyhow::anyhow!("identity transport failed"))?;
        match frame {
            Message::Text(text) => {
                let value: Value =
                    serde_json::from_str(&text).map_err(|_| ConnectionFailure::ProtocolError)?;
                if value.get("echo").and_then(Value::as_str) == Some(&identity_echo) {
                    ensure!(
                        value.get("status").and_then(Value::as_str) == Some("ok")
                            && value.get("retcode").and_then(Value::as_i64) == Some(0),
                        ConnectionFailure::ProtocolError
                    );
                    let account = value
                        .get("data")
                        .and_then(|d| d.get("user_id"))
                        .and_then(native_id)
                        .ok_or(ConnectionFailure::ProtocolError)?;
                    ensure!(
                        account == config.self_id,
                        ConnectionFailure::AccountMismatch
                    );
                    break;
                }
                if let Some(input) = parse_private(config, &value) {
                    ensure!(buffered.len() < 16, ConnectionFailure::ProtocolError);
                    buffered.push(input);
                }
            }
            Message::Close(_) => anyhow::bail!("channel closed before identity"),
            Message::Ping(_) => {
                bounded_write(socket.flush()).await?;
            }
            _ => {}
        }
    }
    connected.send_replace(true);
    service.connection(ConnectionPhase::Online, None);
    for input in buffered {
        if service.receive(input).await.is_err() {
            tracing::warn!(channel = %service.id(), "channel input rejected; not acknowledged as accepted");
        }
    }
    let mut pending: Option<(String, oneshot::Sender<SendOutcome>, tokio::time::Instant)> = None;
    let mut next_send_at = tokio::time::Instant::now();
    let mut last_frame = tokio::time::Instant::now();
    let mut ping = tokio::time::interval(Duration::from_secs(20));
    loop {
        let deadline = pending
            .as_ref()
            .map(|p| p.2)
            .unwrap_or_else(|| last_frame + Duration::from_secs(60));
        tokio::select! {
            _ = tokio::time::sleep_until(deadline) => { if let Some((_, result, _)) = pending.take() { let _ = result.send(SendOutcome::Unknown); } anyhow::bail!("channel response timeout"); },
            _ = ping.tick() => { bounded_write(socket.send(Message::Ping(Vec::new().into()))).await?; },
            command = async { tokio::time::sleep_until(next_send_at).await; receiver.recv().await }, if pending.is_none() => {
                let Some(command) = command else { return Ok(()); };
                if command.result.is_closed() { continue; }
                if !service.may_send_outgoing(&command.delivery).await { let _ = command.result.send(SendOutcome::NotSent); continue; }
                // The outer send deadline may have elapsed during the storage check.
                if command.result.is_closed() { continue; }
                let echo = command.delivery.id().to_string();
                next_send_at = tokio::time::Instant::now() + Duration::from_secs(1);
                let frame = json!({"action":"send_private_msg","params":{"user_id":recipient,"message":[{"type":"text","data":{"text":command.delivery.text()}}]},"echo":echo});
                // A failed write may have partially reached the server. Never retry it.
                pending = Some((echo, command.result, tokio::time::Instant::now() + Duration::from_secs(15)));
                bounded_write(socket.send(Message::Text(frame.to_string().into()))).await?;
            },
            frame = socket.next() => {
                let frame = frame.context("channel closed")?.map_err(|_| anyhow::anyhow!("channel read failed"))?;
                last_frame = tokio::time::Instant::now();
                match frame {
                    Message::Text(text) => {
                        let value: Value = serde_json::from_str(&text).map_err(|_| ConnectionFailure::ProtocolError)?;
                        if let Some((echo, _, _)) = &pending
                            && value.get("echo").and_then(Value::as_str) == Some(echo.as_str()) {
                                let (_, result, _) = pending.take().unwrap();
                                let outcome = if value.get("status").and_then(Value::as_str) == Some("ok") && value.get("retcode").and_then(Value::as_i64) == Some(0) {
                                    value.get("data").and_then(|d| d.get("message_id")).and_then(native_id).map(SendOutcome::Confirmed).unwrap_or(SendOutcome::Unknown)
                                } else if value.get("status").and_then(Value::as_str) == Some("failed") && value.get("retcode").and_then(Value::as_i64).is_some_and(|n| n != 0) { SendOutcome::Rejected }
                                else { SendOutcome::Unknown };
                                let _ = result.send(outcome); continue;
                        }
                        if let Some(input) = parse_private(config, &value)
                            && service.receive(input).await.is_err() {
                            tracing::warn!(channel = %service.id(), "channel input rejected; inspect grant, size or backlog");
                        }
                    },
                    Message::Ping(_) => { bounded_write(socket.flush()).await?; },
                    Message::Close(_) => anyhow::bail!("channel closed"),
                    _ => {},
                }
            }
        }
    }
}

// A blocked socket must not retain a command after the service has timed out.
// On cancellation/error the caller drops this connection; a partial write is Unknown.
async fn bounded_write(
    write: impl Future<Output = std::result::Result<(), tokio_tungstenite::tungstenite::Error>>,
) -> Result<()> {
    tokio::time::timeout(Duration::from_secs(3), write)
        .await
        .map_err(|_| anyhow::anyhow!("channel write timeout; outcome uncertain"))?
        .map_err(|_| anyhow::anyhow!("channel write failed; outcome uncertain"))
}
