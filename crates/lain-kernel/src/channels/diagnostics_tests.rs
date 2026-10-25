use super::*;
use futures_util::SinkExt;
use tokio_tungstenite::{
    accept_async, accept_hdr_async,
    tungstenite::{
        Message,
        http::{Response, StatusCode},
    },
};

#[tokio::test]
async fn management_api_authenticates_diagnostics_and_revisioned_memory_grants() {
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;
    async fn send(app: axum::Router, path: &str, body: Option<Value>, auth: bool) -> (u16, Value) {
        let mut request =
            Request::builder()
                .uri(path)
                .method(if body.is_some() { "POST" } else { "GET" });
        if auth {
            request = request.header("authorization", format!("Bearer {TOKEN}"));
        }
        if body.is_some() {
            request = request.header("content-type", "application/json");
        }
        let response = app
            .oneshot(
                request
                    .body(
                        body.map(|v| Body::from(v.to_string()))
                            .unwrap_or_else(Body::empty),
                    )
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.headers()["cache-control"], "no-store");
        let status = response.status().as_u16();
        let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains(TOKEN));
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("api.db"));
    let b = k.channel_bind(spec(), true).unwrap();
    let root = format!("/v1/channels/{}", b.id);
    let app = crate::api::router(k.clone(), TOKEN.parse().unwrap());
    for path in [
        "/v1/channels/status".into(),
        root.clone(),
        format!("{root}/memory-grant"),
        format!("{root}/memory-grant/history"),
        format!("{root}/memory-effects"),
    ] {
        assert_eq!(send(app.clone(), &path, None, false).await.0, 401);
    }
    let path = format!("{root}/memory-grant");
    let enable = json!({"enabled":true,"expected_revision":0,"expires_at_ms":null});
    assert_eq!(
        send(app.clone(), &path, Some(enable.clone()), false)
            .await
            .0,
        401
    );
    let status = send(app.clone(), "/v1/channels/status", None, true).await;
    assert_eq!(status.0, 200);
    assert_eq!(status.1[0]["connection"]["phase"], "not_started");
    let initial = send(app.clone(), &path, None, true).await.1;
    assert_eq!(initial["enabled"], false);
    assert_eq!(initial["revision"], 0);
    assert_eq!(
        send(
            app.clone(),
            &path,
            Some(json!({"enabled":true,"expected_revision":0,"expires_at_ms":1})),
            true
        )
        .await
        .0,
        400
    );
    assert_eq!(
        send(app.clone(), &path, Some(enable.clone()), true).await.1["revision"],
        1
    );
    assert_eq!(send(app.clone(), &path, Some(enable), true).await.0, 409);
    assert_eq!(
        send(
            app.clone(),
            &path,
            Some(json!({"enabled":false,"expected_revision":1,"expires_at_ms":null})),
            true
        )
        .await
        .1["revision"],
        2
    );
    assert_eq!(
        send(
            app.clone(),
            &format!("{root}/memory-grant/history"),
            None,
            true
        )
        .await
        .1
        .as_array()
        .unwrap()
        .len(),
        2
    );
    assert!(
        send(app, &format!("{root}/memory-effects"), None, true)
            .await
            .1["effects"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        k.channel_binding(b.id).unwrap().grant_revision,
        b.grant_revision
    );
}

#[test]
fn durable_binding_never_restores_online_and_shutdown_closes_observations() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.db");
    let k = kernel(&path);
    let b = k.channel_bind(spec(), true).unwrap();
    assert_eq!(
        k.channel_snapshot(b.id).unwrap().connection.phase,
        ConnectionPhase::NotStarted
    );
    k.channel_connection(b.id, ConnectionPhase::Online, None)
        .unwrap();
    assert_eq!(
        k.channel_statuses().unwrap()[0].connection.phase,
        ConnectionPhase::Online
    );
    k.shutdown().unwrap();
    assert_eq!(
        k.channel_snapshot(b.id).unwrap().connection.phase,
        ConnectionPhase::Stopped
    );
    drop(k);
    let k = kernel(&path);
    assert_eq!(
        k.channel_snapshot(b.id).unwrap().connection.phase,
        ConnectionPhase::NotStarted
    );
}

#[tokio::test]
#[allow(clippy::result_large_err)]
async fn transport_failures_have_safe_distinct_categories_and_abort_stops_projection() {
    for (mode, expected) in [
        ("refused", ConnectionFailure::ConnectionUnavailable),
        ("auth", ConnectionFailure::AuthenticationFailed),
        ("account", ConnectionFailure::AccountMismatch),
        ("protocol", ConnectionFailure::ProtocolError),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let k = kernel(&dir.path().join("state.db"));
        let (s, model) = service(&k, 0);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            if mode == "refused" {
                drop(listener);
                return;
            }
            let (tcp, _) = listener.accept().await.unwrap();
            if mode == "auth" {
                let _ = accept_hdr_async(
                    tcp,
                    |_: &tokio_tungstenite::tungstenite::handshake::server::Request, _| {
                        Err(Response::builder()
                            .status(StatusCode::UNAUTHORIZED)
                            .body(Some("fixture-private-header-value".into()))
                            .unwrap())
                    },
                )
                .await;
            } else {
                let mut ws = accept_async(tcp).await.unwrap();
                let login = ws_json(&mut ws).await;
                if mode == "account" {
                    ws_send(&mut ws,json!({"status":"ok","retcode":0,"data":{"user_id":9999},"echo":login["echo"]})).await;
                } else {
                    ws.send(Message::Text("invalid-json-fixture-private-value".into()))
                        .await
                        .unwrap();
                }
            }
        });
        if mode == "refused" {
            server.await.unwrap();
        }
        let (_, transport) = napcat::start_authenticated(
            qq_config(format!("ws://{address}/")),
            s.clone(),
            k.subscribe_shutdown(),
            crate::profiles::SecretValue(zeroize::Zeroizing::new("fixture-token".into())),
        );
        eventually(|| k.channel_snapshot(s.id()).unwrap().connection.failure == Some(expected))
            .await;
        let status = k.channel_snapshot(s.id()).unwrap().connection;
        assert_ne!(status.phase, ConnectionPhase::Online);
        assert!(!serde_json::to_string(&status).unwrap().contains("fixture"));
        assert_eq!(model.calls.load(Ordering::SeqCst), 0);
        transport.abort();
        let _ = transport.await;
        assert_eq!(
            k.channel_snapshot(s.id()).unwrap().connection.phase,
            ConnectionPhase::Stopped
        );
        k.shutdown().unwrap();
    }
}

#[tokio::test]
async fn online_requires_identity_and_connection_loss_is_observable() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.db"));
    let (s, _) = service(&k, 0);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (ready, seen) = tokio::sync::oneshot::channel();
    let (verify, verified) = tokio::sync::oneshot::channel();
    let (close, closing) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(tcp).await.unwrap();
        let login = ws_json(&mut ws).await;
        ready.send(()).unwrap();
        verified.await.unwrap();
        ws_send(
            &mut ws,
            json!({"status":"ok","retcode":0,"data":{"user_id":1001},"echo":login["echo"]}),
        )
        .await;
        closing.await.unwrap();
        ws.close(None).await.unwrap();
    });
    let (channel, transport) = napcat::start_authenticated(
        qq_config(format!("ws://{address}/")),
        s.clone(),
        k.subscribe_shutdown(),
        crate::profiles::SecretValue(zeroize::Zeroizing::new("fixture-token".into())),
    );
    seen.await.unwrap();
    assert_eq!(
        k.channel_snapshot(s.id()).unwrap().connection.phase,
        ConnectionPhase::Authenticating
    );
    assert!(!*channel.connected().borrow());
    verify.send(()).unwrap();
    eventually(|| k.channel_snapshot(s.id()).unwrap().connection.phase == ConnectionPhase::Online)
        .await;
    assert!(*channel.connected().borrow());
    close.send(()).unwrap();
    eventually(|| {
        k.channel_snapshot(s.id()).unwrap().connection.failure
            == Some(ConnectionFailure::Disconnected)
    })
    .await;
    k.shutdown().unwrap();
    transport.await.unwrap();
    server.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn identity_timeout_is_distinct_and_does_not_mark_online() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.db"));
    let (s, _) = service(&k, 0);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (ready, seen) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(tcp).await.unwrap();
        ws_json(&mut ws).await;
        ready.send(()).unwrap();
        std::future::pending::<()>().await;
    });
    let (_, transport) = napcat::start_authenticated(
        qq_config(format!("ws://{address}/")),
        s.clone(),
        k.subscribe_shutdown(),
        crate::profiles::SecretValue(zeroize::Zeroizing::new("fixture-token".into())),
    );
    seen.await.unwrap();
    assert_eq!(
        k.channel_snapshot(s.id()).unwrap().connection.phase,
        ConnectionPhase::Authenticating
    );
    tokio::time::advance(Duration::from_secs(11)).await;
    eventually(|| {
        k.channel_snapshot(s.id()).unwrap().connection.failure
            == Some(ConnectionFailure::IdentityTimeout)
    })
    .await;
    k.shutdown().unwrap();
    transport.await.unwrap();
    server.abort();
}

#[tokio::test]
#[allow(clippy::result_large_err)]
async fn repeated_authentication_failures_emit_one_safe_warning() {
    use tracing_subscriber::{Layer, layer::SubscriberExt};
    struct Warnings {
        channel: String,
        count: Arc<AtomicUsize>,
    }
    impl<S: tracing::Subscriber> Layer<S> for Warnings {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _: tracing_subscriber::layer::Context<'_, S>,
        ) {
            if event.metadata().level() == &tracing::Level::WARN
                && event.metadata().target().ends_with("channels::napcat")
            {
                #[derive(Default)]
                struct Fields {
                    channel: String,
                    encoded: String,
                }
                impl tracing::field::Visit for Fields {
                    fn record_debug(
                        &mut self,
                        field: &tracing::field::Field,
                        value: &dyn std::fmt::Debug,
                    ) {
                        let value = format!("{value:?}");
                        if field.name() == "channel" {
                            self.channel = value.clone();
                        }
                        self.encoded.push_str(&value);
                    }
                }
                let mut fields = Fields::default();
                event.record(&mut fields);
                if fields.channel == self.channel {
                    assert!(!fields.encoded.contains("fixture-private"));
                    assert!(!fields.encoded.contains("fixture-token"));
                    self.count.fetch_add(1, Ordering::SeqCst);
                }
            }
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("repeat.db"));
    let (s, _) = service(&k, 0);
    let count = Arc::new(AtomicUsize::new(0));
    // Spawned tasks require process-wide capture; filter the unique binding to isolate parallel tests.
    let subscriber = tracing_subscriber::registry().with(Warnings {
        channel: s.id().to_string(),
        count: count.clone(),
    });
    tracing::subscriber::set_global_default(subscriber)
        .expect("one diagnostics capture per test process");
    async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            for _ in 0..3 {
                let (tcp, _) = listener.accept().await.unwrap();
                let _ = accept_hdr_async(
                    tcp,
                    |_: &tokio_tungstenite::tungstenite::handshake::server::Request, _| {
                        Err(Response::builder()
                            .status(StatusCode::FORBIDDEN)
                            .body(Some("fixture-private-reason".into()))
                            .unwrap())
                    },
                )
                .await;
            }
        });
        let (_, task) = napcat::start_authenticated(
            qq_config(format!("ws://{address}/")),
            s,
            k.subscribe_shutdown(),
            crate::profiles::SecretValue(zeroize::Zeroizing::new("fixture-token".into())),
        );
        tokio::time::timeout(Duration::from_secs(15), server)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 1);
        k.shutdown().unwrap();
        task.await.unwrap();
    }
    .await;
}
