use super::*;
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::{
        IntoResponse, Response, Sse,
        sse::{Event, KeepAlive},
    },
    routing::{get, post},
};
use lain_wire::client_sync::*;
use std::{
    convert::Infallible,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
};
use tokio::{sync::Notify, time::Instant};

struct Server {
    page: Mutex<BootstrapSnapshot>,
    revision: AtomicU64,
    readable: AtomicBool,
    authorized: AtomicBool,
    disconnected: AtomicBool,
    notice_denied: AtomicBool,
    wake: Notify,
    bootstrap_calls: AtomicUsize,
    forbidden: AtomicUsize,
}
async fn bootstrap(State(s): State<Arc<Server>>) -> Result<Json<BootstrapSnapshot>, StatusCode> {
    s.bootstrap_calls.fetch_add(1, Ordering::SeqCst);
    if !s.authorized.load(Ordering::SeqCst) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let mut page = s.page.lock().unwrap().clone();
    page.cursor.revision = s.revision.load(Ordering::SeqCst);
    if !s.readable.load(Ordering::SeqCst) {
        for t in &mut page.turns {
            t.availability = Some(lain_wire::ActorAvailability::SourceUnavailable);
            t.messages.clear();
            t.actions.clear();
            t.error = Some("Source unavailable; previous content withheld.".into());
        }
    }
    Ok(Json(page))
}
async fn changes(
    State(s): State<Arc<Server>>,
    Query(q): Query<BTreeMap<String, String>>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    let after = q["after"].parse::<u64>().unwrap();
    let stream = futures_util::stream::once(async move {
        loop {
            let ready = s.wake.notified();
            tokio::pin!(ready);
            ready.as_mut().enable();
            if s.disconnected.load(Ordering::SeqCst) {
                return None;
            }
            if s.revision.load(Ordering::SeqCst) > after {
                break;
            }
            ready.await;
        }
        let mut cursor = s.page.lock().unwrap().cursor;
        cursor.revision = s.revision.load(Ordering::SeqCst);
        Some(Ok(Event::default()
            .event("invalidate")
            .json_data(Invalidation {
                cursor,
                reason: InvalidationReason::Changed,
            })
            .unwrap()))
    })
    .filter_map(std::future::ready);
    Sse::new(stream).keep_alive(KeepAlive::default())
}
fn idle(
    name: &'static str,
    value: Value,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    Sse::new(
        futures_util::stream::once(async move {
            Ok(Event::default().event(name).json_data(value).unwrap())
        })
        .chain(futures_util::stream::pending()),
    )
    .keep_alive(KeepAlive::default())
}
async fn notices(State(s): State<Arc<Server>>) -> Response {
    if s.notice_denied.load(Ordering::SeqCst) {
        return StatusCode::FORBIDDEN.into_response();
    }
    idle(
        "notices",
        json!({"notices":[],"next_cursor":0,"has_more":false}),
    )
    .into_response()
}
async fn fixture() -> (
    Client,
    Arc<Server>,
    tokio::task::JoinHandle<()>,
    ConversationId,
) {
    let c = ConversationId::new();
    let mut t = super::tests::running(c);
    t.state = ActorTurnState::Completed;
    t.messages.push(lain_wire::ActorMessage {
        id: MessageId::new(),
        conversation_id: c,
        role: MessageRole::Assistant,
        content: "live-private-marker".into(),
        timestamp_ms: 1,
        status: lain_wire::MessageStatus::Completed,
    });
    let page = super::tests::bootstrap_page(c, vec![t]);
    let s = Arc::new(Server {
        page: Mutex::new(page),
        revision: AtomicU64::new(1),
        readable: AtomicBool::new(true),
        authorized: AtomicBool::new(true),
        disconnected: AtomicBool::new(false),
        notice_denied: AtomicBool::new(false),
        wake: Notify::new(),
        bootstrap_calls: AtomicUsize::new(0),
        forbidden: AtomicUsize::new(0),
    });
    let fallback = s.clone();
    let app = Router::new()
        .route("/v1/client-sync/bootstrap", post(bootstrap))
        .route("/v1/client-sync/events", get(changes))
        .route(
            "/v1/events",
            get(|| async {
                idle(
                    "snapshot",
                    json!({"history_cursor":1,"sessions":[],"turns":[],"approvals":[]}),
                )
            }),
        )
        .route(
            "/v1/diagnostics",
            get(|| async { Json(json!({"actor":{"status":"configured"}})) }),
        )
        .route(
            "/v1/conversations/{id}/device-work/events",
            get(|| async { idle("device_work_snapshot", json!({"work":[],"truncated":false})) }),
        )
        .route("/v1/conversations/{id}/notices/events", get(notices))
        .fallback(move || {
            let s = fallback.clone();
            async move {
                s.forbidden.fetch_add(1, Ordering::SeqCst);
                StatusCode::METHOD_NOT_ALLOWED
            }
        })
        .with_state(s.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (
        Client::new(
            &format!("http://{address}"),
            "live-sync-fixture-owner-12345678901234567890",
        )
        .unwrap(),
        s,
        server,
        c,
    )
}
async fn pump_until(
    client: &Client,
    app: &mut Model,
    tx: &mpsc::Sender<Complete>,
    rx: &mut mpsc::Receiver<Complete>,
    jobs: &mut JoinSet<()>,
    predicate: impl Fn(&Model) -> bool,
) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !predicate(app) {
            let next = rx.recv().await.unwrap();
            handle_complete(client, app, tx, jobs, next).unwrap();
        }
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn live_sync_transport_invalidates_and_refetches_without_replaying_any_operation() {
    let (client, s, server, c) = fixture().await;
    let mut app = Model::new(c);
    app.live = Some(live::State::new());
    app.input.insert("next draft");
    let id = RequestId::new();
    app.pending = Some(Pending {
        id,
        text: "original uncertain input".into(),
        uncertain: true,
    });
    let (tx, mut rx) = mpsc::channel(64);
    let mut jobs = JoinSet::new();
    let lease = client.bootstrap_lease(Some(c)).await.unwrap();
    let old = lease.snapshot.turns[0].clone();
    live::install(&client, &mut app, lease, &tx, &mut jobs).unwrap();
    let old_generation = live::epoch(&app).unwrap();
    assert!(app.transcript.contains("live-private-marker"));
    s.readable.store(false, Ordering::SeqCst);
    s.revision.fetch_add(1, Ordering::SeqCst);
    s.wake.notify_waiters();
    pump_until(&client, &mut app, &tx, &mut rx, &mut jobs, |a| {
        live::epoch(a) != Some(old_generation) && live::fresh(a)
    })
    .await;
    assert!(!app.transcript.contains("live-private-marker"));
    assert_eq!(app.input.text, "next draft");
    assert_eq!(app.pending.as_ref().unwrap().id, id);
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::Read(
            old_generation,
            Box::new(Complete::Actor(ActorUpdate::Snapshot(Box::new(old)))),
        ),
    )
    .unwrap();
    assert!(!app.transcript.contains("live-private-marker"));
    assert_eq!(s.forbidden.load(Ordering::SeqCst), 0);
    assert!(s.bootstrap_calls.load(Ordering::SeqCst) >= 2);
    jobs.abort_all();
    server.abort();
}

#[tokio::test]
async fn activity_survives_checked_sync_without_retaining_old_bodies_or_moving_to_bottom() {
    let (client, s, server, c) = fixture().await;
    let mut app = Model::new(c);
    app.live = Some(live::State::new());
    let (tx, mut rx) = mpsc::channel(64);
    let mut jobs = JoinSet::new();
    let lease = client.bootstrap_lease(Some(c)).await.unwrap();
    live::install(&client, &mut app, lease, &tx, &mut jobs).unwrap();
    let mut record = super::tests::remote_record(c);
    let page = |r: DeviceWorkRecord| {
        Complete::RemoteWork(Ok(DeviceWorkPage {
            work: vec![r],
            truncated: false,
        }))
    };
    handle_complete(&client, &mut app, &tx, &mut jobs, page(record.clone())).unwrap();
    let before = live::epoch(&app);
    s.revision.fetch_add(1, Ordering::SeqCst);
    s.wake.notify_waiters();
    pump_until(&client, &mut app, &tx, &mut rx, &mut jobs, |a| {
        live::epoch(a) != before && !live::fresh(a)
    })
    .await;
    assert!(app.text_view.doc.entries.is_empty() && app.work_text.doc.entries.is_empty());
    pump_until(&client, &mut app, &tx, &mut rx, &mut jobs, live::fresh).await;
    record.command.state = lain_wire::device_commands::CommandState::Unknown;
    record.command.revision += 1;
    app.append("temporary /help entry that bootstrap will not restore");
    handle_complete(&client, &mut app, &tx, &mut jobs, page(record.clone())).unwrap();
    let id = activity::task_id(&record.command);
    assert!(
        app.text_view.doc.version(id).is_some(),
        "completion across invalidation must not become a cold baseline"
    );
    let mut later = super::tests::running(c);
    later.sequence = 2;
    later.state = ActorTurnState::Completed;
    later.messages.push(lain_wire::ActorMessage {
        id: MessageId::new(),
        conversation_id: c,
        role: MessageRole::User,
        content: "later conversation".into(),
        timestamp_ms: 2,
        status: lain_wire::MessageStatus::Completed,
    });
    let later_message = later.messages[0].id.0;
    s.page.lock().unwrap().turns.push(later);
    let before = live::epoch(&app);
    s.revision.fetch_add(1, Ordering::SeqCst);
    s.wake.notify_waiters();
    pump_until(&client, &mut app, &tx, &mut rx, &mut jobs, |a| {
        live::epoch(a) != before && !live::fresh(a)
    })
    .await;
    assert!(app.text_view.doc.entries.is_empty());
    pump_until(&client, &mut app, &tx, &mut rx, &mut jobs, live::fresh).await;
    handle_complete(&client, &mut app, &tx, &mut jobs, page(record)).unwrap();
    let cards = &app.text_view.doc.entries;
    assert!(
        cards.iter().position(|e| e.id == id).unwrap()
            < cards.iter().position(|e| e.id == later_message).unwrap(),
        "known card must restore at its prior message anchor"
    );
    live::retire(&mut app);
    assert!(!app.activity.warm && app.activity.seen.is_empty());
    assert!(app.text_view.doc.entries.is_empty());
    jobs.abort_all();
    server.abort();
}
#[tokio::test]
async fn live_sync_local_lease_clears_output_before_a_new_read_is_installed() {
    let (client, s, server, c) = fixture().await;
    let mut app = Model::new(c);
    app.live = Some(live::State::new());
    let (tx, mut rx) = mpsc::channel(64);
    let mut jobs = JoinSet::new();
    let mut lease = client.bootstrap_lease(Some(c)).await.unwrap();
    lease.deadline = Instant::now() + Duration::from_millis(35);
    live::install(&client, &mut app, lease, &tx, &mut jobs).unwrap();
    let g = live::epoch(&app);
    pump_until(&client, &mut app, &tx, &mut rx, &mut jobs, |a| {
        live::epoch(a) != g
    })
    .await;
    assert!(!app.transcript.contains("live-private-marker"));
    assert!(!live::fresh(&app));
    assert_eq!(s.forbidden.load(Ordering::SeqCst), 0);
    jobs.abort_all();
    server.abort();
}
#[tokio::test]
async fn live_sync_authorization_failure_stops_reconnect_and_preserves_input() {
    let (client, s, server, c) = fixture().await;
    let mut app = Model::new(c);
    app.live = Some(live::State::new());
    app.input.insert("do not send this");
    let (tx, mut rx) = mpsc::channel(64);
    let mut jobs = JoinSet::new();
    let lease = client.bootstrap_lease(Some(c)).await.unwrap();
    live::install(&client, &mut app, lease, &tx, &mut jobs).unwrap();
    s.authorized.store(false, Ordering::SeqCst);
    s.revision.fetch_add(1, Ordering::SeqCst);
    s.wake.notify_waiters();
    pump_until(&client, &mut app, &tx, &mut rx, &mut jobs, |a| {
        a.status.starts_with("Authentication")
    })
    .await;
    assert!(!live::fresh(&app));
    assert!(!app.transcript.contains("live-private-marker"));
    assert_eq!(app.input.text, "do not send this");
    let before = s.bootstrap_calls.load(Ordering::SeqCst);
    tokio::time::sleep(Duration::from_millis(70)).await;
    assert_eq!(before, s.bootstrap_calls.load(Ordering::SeqCst));
    assert_eq!(s.forbidden.load(Ordering::SeqCst), 0);
    jobs.abort_all();
    server.abort();
}

#[tokio::test]
async fn live_sync_old_effect_receipt_is_kept_without_restoring_its_old_body() {
    let (client, s, server, c) = fixture().await;
    let mut app = Model::new(c);
    app.live = Some(live::State::new());
    let (tx, mut rx) = mpsc::channel(64);
    let mut jobs = JoinSet::new();
    let lease = client.bootstrap_lease(Some(c)).await.unwrap();
    let mut receipt = lease.snapshot.turns[0].clone();
    live::install(&client, &mut app, lease, &tx, &mut jobs).unwrap();
    let g = live::epoch(&app).unwrap();
    let id = RequestId::new();
    receipt.request_id = id;
    app.pending = Some(Pending {
        id,
        text: "exact original request".into(),
        uncertain: true,
    });
    s.readable.store(false, Ordering::SeqCst);
    s.revision.fetch_add(1, Ordering::SeqCst);
    s.wake.notify_waiters();
    pump_until(&client, &mut app, &tx, &mut rx, &mut jobs, |a| {
        live::epoch(a) != Some(g) && live::fresh(a)
    })
    .await;
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::Effect(g, Box::new(Complete::Submit(id, Ok(receipt)))),
    )
    .unwrap();
    assert!(
        app.pending.is_none(),
        "the exact acceptance receipt cannot be discarded with its display generation"
    );
    assert!(!app.transcript.contains("live-private-marker"));
    assert_eq!(s.forbidden.load(Ordering::SeqCst), 0);
    jobs.abort_all();
    server.abort();
}

#[tokio::test]
async fn live_sync_read_auth_failure_retires_cache_and_late_receipt_cannot_restart_it() {
    let (client, s, server, c) = fixture().await;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("profile");
    let mut app = Model::new(c);
    let mut state = live::State::new();
    state.cache = Some(crate::sync_cache::Cache::open(root.clone()).unwrap());
    app.live = Some(state);
    app.input.insert("keep draft");
    let (tx, _rx) = mpsc::channel(64);
    let mut jobs = JoinSet::new();
    let lease = client.bootstrap_lease(Some(c)).await.unwrap();
    let mut receipt = lease.snapshot.turns[0].clone();
    live::install(&client, &mut app, lease, &tx, &mut jobs).unwrap();
    let g = live::epoch(&app).unwrap();
    let id = RequestId::new();
    receipt.request_id = id;
    app.pending = Some(Pending {
        id,
        text: "original input".into(),
        uncertain: true,
    });
    assert!(root.join("client-sync-cache.json").exists());
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::Read(
            g,
            Box::new(Complete::Sessions(
                0,
                Err(crate::api::HttpFailure(401).into()),
            )),
        ),
    )
    .unwrap();
    assert!(
        !live::fresh(&app),
        "a denied ordinary read must retire the entire current view"
    );
    assert!(!app.transcript.contains("live-private-marker"));
    assert!(!root.join("client-sync-cache.json").exists());
    assert_eq!(app.input.text, "keep draft");
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::Effect(g, Box::new(Complete::Submit(id, Ok(receipt)))),
    )
    .unwrap();
    assert!(app.pending.is_none());
    let calls = s.bootstrap_calls.load(Ordering::SeqCst);
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(
        s.bootstrap_calls.load(Ordering::SeqCst),
        calls,
        "late receipts cannot restart an auth-blocked connection"
    );
    assert_eq!(s.forbidden.load(Ordering::SeqCst), 0);
    jobs.abort_all();
    server.abort();
}

#[tokio::test]
async fn live_sync_disconnect_reconnects_without_effects_and_refuses_observed_revision_regression()
{
    let (client, s, server, c) = fixture().await;
    let mut app = Model::new(c);
    app.live = Some(live::State::new());
    app.input.insert("unsent draft");
    let (tx, mut rx) = mpsc::channel(64);
    let mut jobs = JoinSet::new();
    let lease = client.bootstrap_lease(Some(c)).await.unwrap();
    live::install(&client, &mut app, lease, &tx, &mut jobs).unwrap();
    s.disconnected.store(true, Ordering::SeqCst);
    s.wake.notify_waiters();
    pump_until(&client, &mut app, &tx, &mut rx, &mut jobs, |a| {
        !live::fresh(a)
    })
    .await;
    assert!(!app.transcript.contains("live-private-marker"));
    s.disconnected.store(false, Ordering::SeqCst);
    s.readable.store(false, Ordering::SeqCst);
    pump_until(&client, &mut app, &tx, &mut rx, &mut jobs, live::fresh).await;
    assert_eq!(app.input.text, "unsent draft");
    let mut cursor = s.page.lock().unwrap().cursor;
    cursor.revision = 5;
    let g = live::epoch(&app).unwrap();
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::Live(
            g,
            live::Update::Changed(Ok(Invalidation {
                cursor,
                reason: InvalidationReason::Changed,
            })),
        ),
    )
    .unwrap();
    // The server returns revision 1: newer than the last displayed snapshot is
    // insufficient once revision 5 has already been observed on the wire.
    pump_until(&client, &mut app, &tx, &mut rx, &mut jobs, |a| {
        a.status.starts_with("Synchronization unavailable")
    })
    .await;
    assert!(!live::fresh(&app));
    assert!(!app.transcript.contains("live-private-marker"));
    assert_eq!(s.forbidden.load(Ordering::SeqCst), 0);
    jobs.abort_all();
    server.abort();
}

#[tokio::test]
async fn live_sync_queued_expired_snapshot_never_restores_content() {
    let (client, s, server, c) = fixture().await;
    let mut app = Model::new(c);
    app.live = Some(live::State::new());
    let (tx, _rx) = mpsc::channel(64);
    let mut jobs = JoinSet::new();
    let mut lease = client.bootstrap_lease(Some(c)).await.unwrap();
    lease.deadline = Instant::now() - Duration::from_millis(1);
    let g = live::epoch(&app).unwrap();
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::Live(g, live::Update::Snapshot(Ok(Box::new(lease)))),
    )
    .unwrap();
    assert!(!live::fresh(&app));
    assert!(!app.transcript.contains("live-private-marker"));
    assert_ne!(live::epoch(&app), Some(g));
    assert_eq!(s.forbidden.load(Ordering::SeqCst), 0);
    jobs.abort_all();
    server.abort();
}

#[tokio::test]
async fn live_sync_retirement_cancels_reads_but_retains_effect_receipt_delivery() {
    let (client, _, server, c) = fixture().await;
    let mut app = Model::new(c);
    app.live = Some(live::State::new());
    let (tx, mut rx) = mpsc::channel(64);
    let mut jobs = JoinSet::new();
    let out = live::effect_sender(&mut app, &tx, &mut jobs);
    let (started, ready) = tokio::sync::oneshot::channel();
    let (dropped, cancelled) = tokio::sync::oneshot::channel();
    struct ReadGuard(Option<tokio::sync::oneshot::Sender<()>>);
    impl Drop for ReadGuard {
        fn drop(&mut self) {
            let _ = self.0.take().unwrap().send(());
        }
    }
    live::spawn_read(&mut app, &mut jobs, async move {
        let _guard = ReadGuard(Some(dropped));
        let _ = started.send(());
        std::future::pending::<()>().await;
    });
    ready.await.unwrap();
    let g = live::epoch(&app).unwrap();
    live::retire(&mut app);
    tokio::time::timeout(Duration::from_secs(1), cancelled)
        .await
        .unwrap()
        .unwrap();
    let id = RequestId::new();
    let mut receipt = super::tests::running(c);
    receipt.request_id = id;
    app.pending = Some(Pending {
        id,
        text: "accepted original".into(),
        uncertain: true,
    });
    out.send(Complete::Submit(id, Ok(receipt))).await.unwrap();
    let event = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(&event, Complete::Effect(epoch, _) if *epoch == g));
    handle_complete(&client, &mut app, &tx, &mut jobs, event).unwrap();
    assert!(app.pending.is_none());
    assert!(app.transcript.is_empty());
    jobs.abort_all();
    server.abort();
}

#[tokio::test]
async fn live_sync_rechecks_chat_configuration_after_refresh_without_sending_draft() {
    let (client, s, server, c) = fixture().await;
    let mut app = Model::new(c);
    app.live = Some(live::State::new());
    app.chat_status = chat_status::Availability::NotConfigured;
    app.input.insert("send only after Enter");
    let (tx, mut rx) = mpsc::channel(64);
    let mut jobs = JoinSet::new();
    live::refresh(&client, &mut app, &tx, &mut jobs);
    pump_until(&client, &mut app, &tx, &mut rx, &mut jobs, |a| {
        live::fresh(a) && a.chat_status == chat_status::Availability::Configured
    })
    .await;
    assert_eq!(app.input.text, "send only after Enter");
    assert!(app.pending.is_none());
    assert_eq!(s.forbidden.load(Ordering::SeqCst), 0);
    jobs.abort_all();
    server.abort();
}

#[tokio::test]
async fn live_sync_effect_recovery_auth_failure_retires_view_and_keeps_exact_pending_cancel() {
    let (client, s, server, c) = fixture().await;
    let mut app = Model::new(c);
    app.live = Some(live::State::new());
    let (tx, _rx) = mpsc::channel(64);
    let mut jobs = JoinSet::new();
    let lease = client.bootstrap_lease(Some(c)).await.unwrap();
    live::install(&client, &mut app, lease, &tx, &mut jobs).unwrap();
    let target = super::tests::remote_record(c).command;
    app.cancel_pending = Some(cancel::Pending {
        target: target.clone(),
    });
    app.cancel_busy = true;
    let g = live::epoch(&app).unwrap();
    handle_complete(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        Complete::Effect(
            g,
            Box::new(Complete::RemoteCancelled(Box::new((
                cancel::Pending {
                    target: target.clone(),
                },
                Err(cancel::Failure::Uncertain(
                    crate::api::HttpFailure(403).into(),
                )),
            )))),
        ),
    )
    .unwrap();
    assert!(
        !live::fresh(&app),
        "a failed receipt GET also revokes the visible connection"
    );
    assert!(!app.transcript.contains("live-private-marker"));
    assert_eq!(app.cancel_pending.as_ref().unwrap().target, target);
    assert!(!app.cancel_busy);
    assert!(app.status.starts_with("Authentication"));
    assert_eq!(s.forbidden.load(Ordering::SeqCst), 0);
    jobs.abort_all();
    server.abort();
}

#[tokio::test]
async fn live_sync_notice_auth_failure_blocks_until_explicit_reconnect() {
    let (client, s, server, c) = fixture().await;
    s.notice_denied.store(true, Ordering::SeqCst);
    let mut app = Model::new(c);
    app.live = Some(live::State::new());
    app.input.insert("still unsent");
    let (tx, mut rx) = mpsc::channel(64);
    let mut jobs = JoinSet::new();
    let lease = client.bootstrap_lease(Some(c)).await.unwrap();
    live::install(&client, &mut app, lease, &tx, &mut jobs).unwrap();
    pump_until(&client, &mut app, &tx, &mut rx, &mut jobs, |a| {
        a.status.starts_with("Authentication")
    })
    .await;
    assert!(!live::fresh(&app));
    assert!(!app.transcript.contains("live-private-marker"));
    assert_eq!(s.bootstrap_calls.load(Ordering::SeqCst), 1);
    s.notice_denied.store(false, Ordering::SeqCst);
    handle_key(
        &client,
        &mut app,
        &tx,
        &mut jobs,
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
    )
    .unwrap();
    pump_until(&client, &mut app, &tx, &mut rx, &mut jobs, live::fresh).await;
    assert_eq!(app.input.text, "still unsent");
    assert_eq!(s.forbidden.load(Ordering::SeqCst), 0);
    jobs.abort_all();
    server.abort();
}
