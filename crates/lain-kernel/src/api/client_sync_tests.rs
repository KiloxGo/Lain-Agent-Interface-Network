use super::*;
use axum::body::{Body, to_bytes};
use axum::http::Request as HttpRequest;
use futures_util::StreamExt;
use serde_json::{Value, json};
use tower::ServiceExt;

const OWNER: &str = "sync-fixture-owner-12345678901234567890";
struct NeverInfer;
impl crate::actor::model::ActorModel for NeverInfer {
    fn respond(
        &self,
        _: crate::actor::model::Request,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<crate::actor::model::Reply>> + Send + '_>>
    {
        Box::pin(async { anyhow::bail!("bootstrap must not perform inference") })
    }
}
fn app(k: Arc<Kernel>) -> Router {
    let token = OWNER.parse().unwrap();
    let actor = crate::actor::Actor::with_model(
        k.clone(),
        None,
        OWNER.parse().unwrap(),
        Arc::new(NeverInfer),
    );
    actor_router(k, token, None, actor)
}
async fn bootstrap(
    app: &Router,
    conversation: Option<lain_wire::ConversationId>,
    protocol: u32,
    auth: &str,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            HttpRequest::builder()
                .method("POST")
                .uri("/v1/client-sync/bootstrap")
                .header(header::AUTHORIZATION, format!("Bearer {auth}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"protocol":protocol,"conversation_id":conversation}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1048576).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
#[tokio::test]
async fn startup_sync_is_authenticated_bounded_read_only_and_keeps_kernel_identity() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.db");
    let k = Arc::new(Kernel::open(&path).unwrap());
    k.mark_ready().unwrap();
    let c = k.create_conversation().unwrap();
    let a = app(k.clone());
    assert_eq!(
        bootstrap(
            &a,
            Some(c.id),
            1,
            "lain-device-0000000000000000000000000000000000000000000000000000000000000000"
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        bootstrap(&a, Some(c.id), 99, OWNER).await.0,
        StatusCode::UPGRADE_REQUIRED
    );
    let (status, first) = bootstrap(&a, Some(c.id), 1, OWNER).await;
    assert_eq!(status, StatusCode::OK, "{first}");
    let (_, second) = bootstrap(&a, Some(c.id), 1, OWNER).await;
    assert_eq!(
        first["cursor"], second["cursor"],
        "reads cannot mutate the sync revision"
    );
    assert_eq!(first["conversation_id"], json!(c.id));
    assert_eq!(first["turns"], json!([]));
    assert_eq!(first["work"], json!([]));
    assert!(
        first["resources"]["persona_revision"]
            .as_str()
            .is_some_and(|v| v.len() == 64)
    );
    assert!(!first.to_string().contains("You are Lain"));
    let identity = first["cursor"]["instance_id"].clone();
    drop(a);
    drop(k);
    let k = Arc::new(Kernel::open(&path).unwrap());
    k.mark_ready().unwrap();
    let (_, restored) = bootstrap(&app(k), Some(c.id), 1, OWNER).await;
    assert_eq!(restored["cursor"]["instance_id"], identity);
}
#[tokio::test]
async fn startup_sync_closes_snapshot_subscription_gap_and_revalidates_queued_snapshot() {
    use crate::memory::*;
    let dir = tempfile::tempdir().unwrap();
    let k = Arc::new(Kernel::open(&dir.path().join("state.db")).unwrap());
    k.mark_ready().unwrap();
    let c = k.create_conversation().unwrap();
    let r = k
        .add_memory(AddMemory {
            request_id: lain_wire::RequestId::new(),
            draft: MemoryDraft {
                content: "source only for this fixture".into(),
                kind: MemoryKind::Episodic,
                scope: MemoryScope::Global,
            },
        })
        .unwrap();
    let (t, _) = k
        .actor_submit_scoped(c.id, uuid::Uuid::new_v4(), "read fixture".into(), None)
        .unwrap();
    k.actor_memory_inspect(t.id, r.id, MemoryScope::Global)
        .unwrap();
    k.actor_update(t.id, |t| {
        t.message(
            crate::actor::domain::MessageRole::Assistant,
            "old-sync-marker".into(),
        )?;
        t.state = crate::actor::domain::TurnState::Completed;
        Ok(())
    })
    .unwrap();
    let a = app(k.clone());
    let (status, snapshot) = bootstrap(&a, Some(c.id), 1, OWNER).await;
    assert_eq!(status, StatusCode::OK);
    assert!(snapshot.to_string().contains("old-sync-marker"));
    // Mutation between snapshot and subscription must be caught without polling.
    k.delete_memory(r.id, r.revision).unwrap();
    let uri = format!(
        "/v1/client-sync/events?instance_id={}&boot_id={}&after={}&lease_until_ms={}",
        snapshot["cursor"]["instance_id"].as_str().unwrap(),
        snapshot["cursor"]["boot_id"].as_str().unwrap(),
        snapshot["cursor"]["revision"],
        snapshot["valid_until_ms"]
    );
    let response = a
        .clone()
        .oneshot(
            HttpRequest::builder()
                .uri(uri)
                .header(header::AUTHORIZATION, format!("Bearer {OWNER}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = tokio::time::timeout(
        Duration::from_secs(2),
        response.into_body().into_data_stream().next(),
    )
    .await
    .unwrap()
    .unwrap()
    .unwrap();
    let event = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(event.contains("invalidate"), "{event}");
    assert!(!event.contains("old-sync-marker"));
    let (_, fresh) = bootstrap(&a, Some(c.id), 1, OWNER).await;
    assert!(
        fresh["cursor"]["revision"].as_u64().unwrap()
            > snapshot["cursor"]["revision"].as_u64().unwrap()
    );
    assert!(!fresh.to_string().contains("old-sync-marker"));
    assert_eq!(
        fresh["turns"][0]["availability"],
        json!("source_unavailable")
    );
}

#[tokio::test]
async fn startup_sync_live_notifications_expiry_and_invalid_cursors_force_a_new_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let k = Arc::new(Kernel::open(&dir.path().join("state.db")).unwrap());
    k.mark_ready().unwrap();
    let a = app(k.clone());
    let (_, snapshot) = bootstrap(&a, None, 1, OWNER).await;
    let cursor: lain_wire::client_sync::SyncCursor =
        serde_json::from_value(snapshot["cursor"].clone()).unwrap();
    async fn subscribe(
        a: &Router,
        c: lain_wire::client_sync::SyncCursor,
        until: u64,
    ) -> axum::response::Response {
        a.clone().oneshot(HttpRequest::builder().uri(format!("/v1/client-sync/events?instance_id={}&boot_id={}&after={}&lease_until_ms={until}",c.instance_id,c.boot_id,c.revision)).header(header::AUTHORIZATION,format!("Bearer {OWNER}")).body(Body::empty()).unwrap()).await.unwrap()
    }
    // Read-only snapshots and status checks cannot wake their own subscription.
    let mut changes = k.subscribe_sync();
    let _ = k.sync_status().unwrap();
    assert!(!changes.has_changed().unwrap());
    k.create_conversation().unwrap();
    tokio::time::timeout(Duration::from_secs(2), changes.changed())
        .await
        .unwrap()
        .unwrap();
    for mode in ["old", "foreign", "future", "expired", "window"] {
        let (_, fresh) = bootstrap(&a, None, 1, OWNER).await;
        let mut q: lain_wire::client_sync::SyncCursor =
            serde_json::from_value(fresh["cursor"].clone()).unwrap();
        let now = crate::domain::now_ms().unwrap();
        let mut until = now + 1000;
        let expected = match mode {
            "old" => {
                q = cursor;
                "changed"
            }
            "foreign" => {
                q.instance_id = uuid::Uuid::new_v4();
                "resnapshot"
            }
            "future" => {
                q.revision += 1;
                "resnapshot"
            }
            "expired" => {
                until = now + 25;
                "expired"
            }
            _ => {
                for _ in 0..258 {
                    k.create_conversation().unwrap();
                }
                "resnapshot"
            }
        };
        let response = subscribe(&a, q, until).await;
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = tokio::time::timeout(
            Duration::from_secs(2),
            response.into_body().into_data_stream().next(),
        )
        .await
        .unwrap()
        .unwrap()
        .unwrap();
        assert!(
            String::from_utf8_lossy(&bytes).contains(expected),
            "{mode}: {}",
            String::from_utf8_lossy(&bytes)
        );
    }
}
