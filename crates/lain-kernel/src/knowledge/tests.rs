use super::*;
use crate::domain::now_ms;
use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    routing::get,
};
use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::sync::Semaphore;
const STORE: &str = "c22d2694-7541-453f-b3de-ed7d50d2930c";
async fn start_config(
    k: Arc<Kernel>,
    s: ArchiveSourceId,
    q: ReadArchive,
    r: Redactor,
    c: Config,
) -> Result<ArchiveReadJob> {
    super::start_resolved(
        k,
        s,
        q,
        r,
        c,
        SecretValue(zeroize::Zeroizing::new(
            "fixture-knowledge-read-1234567890".into(),
        )),
    )
    .await
}
struct Remote {
    page: Mutex<Value>,
    calls: AtomicUsize,
    gate: Semaphore,
}
async fn changes(
    State(r): State<Arc<Remote>>,
    Query(q): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> std::result::Result<Json<Value>, StatusCode> {
    if headers.get("authorization").and_then(|h| h.to_str().ok())
        != Some("Bearer fixture-knowledge-read-1234567890")
        || q.get("source_id").map(String::as_str) != Some("qq:1001:private:2002")
    {
        return Err(StatusCode::FORBIDDEN);
    }
    r.calls.fetch_add(1, Ordering::SeqCst);
    r.gate.acquire().await.unwrap().forget();
    let after = q["after"].parse::<u64>().unwrap();
    let mut p = r.page.lock().unwrap().clone();
    p["from_after"] = json!(after);
    if after == p["through"].as_u64().unwrap() {
        p["items"] = json!([]);
        p["next_after"] = json!(after);
    }
    Ok(Json(p))
}
fn page(sequence: u64, text: &str, state: &str) -> Value {
    let now = now_ms().unwrap();
    json!({"schema_version":1,"store_id":STORE,"source":{"id":"qq:1001:private:2002","account":"1001","kind":"private","peer":"2002","revision":1,"enabled":true},"from_after":0,"through":sequence,"next_after":sequence,"has_more":false,"items":[{"sequence":sequence,"message":{"id":1,"source_id":"qq:1001:private:2002","external_id":"1234","sender":"2002","occurred_at_ms":now-3000,"received_at_ms":now-2000,"expires_at_ms":now+60000,"state":state,"revision":sequence,"text":if state=="active"{Some(text)}else{None},"images":if state=="active"{vec![json!({"id":"asset-fixture","description":"a red square, model interpretation","model":"fixture-vision","prompt_revision":"v1","state":"completed"})]}else{vec![]}}}]})
}
async fn fixture() -> (
    tempfile::TempDir,
    Arc<Kernel>,
    ArchiveSourceId,
    Config,
    Arc<Remote>,
    tokio::task::JoinHandle<()>,
) {
    let dir = tempfile::tempdir().unwrap();
    let k = Arc::new(Kernel::open(&dir.path().join("state.db")).unwrap());
    k.mark_ready().unwrap();
    let source = k
        .archive_register(
            ArchiveSourceKey::QqPrivate {
                observer_account: "1001".into(),
                peer_id: "2002".into(),
            },
            Redactor::new(vec![]),
        )
        .unwrap()
        .source
        .id;
    k.archive_set_grant(
        source,
        SetArchiveGrant {
            expected_revision: 0,
            scope: Some(ArchiveScope {
                expires_at_ms: now_ms().unwrap() + 60000,
                retention_ms: 60000,
                max_records: 10,
            }),
        },
    )
    .unwrap();
    let remote = Arc::new(Remote {
        page: Mutex::new(page(1, "bridge original", "active")),
        calls: AtomicUsize::new(0),
        gate: Semaphore::new(20),
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = Router::new()
        .route("/v1/sources/qq:1001:private:2002/changes", get(changes))
        .with_state(remote.clone());
    let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let config = Config {
        endpoint: format!("http://{addr}"),
        store_id: STORE.into(),
        secret_ref: "secret://env/knowledge-fixture".parse().unwrap(),
    };
    // Native HTTP credentials are supplied only to this client's test environment.
    (dir, k, source, config, remote, task)
}
async fn settled(k: &Arc<Kernel>, id: RequestId) -> ArchiveReadJob {
    let j = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let j = k.archive_read_job(id).unwrap();
            if !j.state.unfinished() {
                return j;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    drain(k).await.unwrap();
    j
}
fn request(after: Option<RequestId>) -> ReadArchive {
    ReadArchive {
        request_id: RequestId::new(),
        expected_revision: 1,
        after,
        limit: 4,
    }
}
async fn pull(
    k: &Arc<Kernel>,
    s: ArchiveSourceId,
    c: &Config,
    after: Option<RequestId>,
) -> ArchiveReadJob {
    let j = start_config(
        k.clone(),
        s,
        request(after),
        Redactor::new(vec![]),
        c.clone(),
    )
    .await
    .unwrap();
    settled(k, j.request.request_id).await
}

#[tokio::test]
async fn reply_references_survive_scoped_import_update_and_withdrawal_without_target_reads() {
    let (_dir, k, s, c, r, server) = fixture().await;
    r.page.lock().unwrap()["items"][0]["message"]["segments"] = json!([
        {"type":"reply","reference":"9001","target_body":"not fetched target secret"},
        {"type":"text","text":"bridge original"}
    ]);
    let first = pull(&k, s, &c, None).await;
    assert_eq!(first.state, ArchiveReadState::Completed);
    assert_eq!(first.new_records, 1);
    assert_eq!(first.record_ids.len(), 1);
    let id = first.record_ids[0];
    let record = k.archive_record(id).unwrap();
    let deadline = record.retain_until_ms;
    let body: Value = serde_json::from_str(record.text.as_deref().unwrap()).unwrap();
    assert_eq!(body["native_message_id"], "1234");
    assert_eq!(
        body["reply_references"]["observed_markers"][0]["native_reference_id"],
        "9001"
    );
    assert_eq!(
        body["reply_references"]["observed_markers"][0]["segment_index"],
        0
    );
    assert_eq!(
        body["reply_references"]["observed_markers"][0].get("resolved_target"),
        Some(&Value::Null)
    );
    assert!(body["reply_references"].get("targets").is_none());
    assert_eq!(
        body["reply_references"]["lookup_scope"],
        "current_source_only"
    );
    assert_eq!(
        body["reply_references"]["target_resolution"],
        "not_attempted"
    );
    assert!(body["reply_references"].get("scope").is_none());
    assert_eq!(
        body["reply_references"]["target_chronology"],
        "unknown_without_verified_target_clock"
    );
    assert_eq!(
        body["reply_references"]["target_content"],
        "not_loaded_by_reference"
    );
    assert_eq!(body["reply_references"]["omitted_references"], 0);
    assert!(!record.text.unwrap().contains("not fetched target secret"));
    assert_eq!(r.calls.load(Ordering::SeqCst), 1);
    assert_eq!(k.archive_source(s).unwrap().total_records, 1);

    let mut changed = page(2, "bridge original", "active");
    changed["items"][0]["message"]["segments"] = json!([
        {"type":"reply","reference":"9002"},
        {"type":"text","text":"bridge original"}
    ]);
    *r.page.lock().unwrap() = changed;
    let second = pull(&k, s, &c, Some(first.request.request_id)).await;
    assert_eq!(second.updated_records, 1);
    let updated = k.archive_record(id).unwrap();
    assert_eq!(updated.revision, record.revision + 1);
    assert_eq!(updated.retain_until_ms, deadline);
    let body: Value = serde_json::from_str(updated.text.as_deref().unwrap()).unwrap();
    assert_eq!(
        body["reply_references"]["observed_markers"][0]["native_reference_id"],
        "9002"
    );
    assert!(!updated.text.unwrap().contains("9001"));
    assert_eq!(r.calls.load(Ordering::SeqCst), 2);

    *r.page.lock().unwrap() = page(3, "", "recalled");
    let withdrawn = pull(&k, s, &c, Some(second.request.request_id)).await;
    assert_eq!(withdrawn.withdrawn_records, 1);
    assert!(k.archive_record(id).unwrap().text.is_none());
    assert_eq!(k.archive_source(s).unwrap().total_records, 1);
    assert!(k.list_sessions().unwrap().is_empty());
    server.abort();
    let _ = server.await;
}

#[test]
fn reply_reference_projection_is_bounded_and_legacy_messages_stay_unchanged() {
    let raw = page(1, "bridge original", "active");
    let legacy: Message = serde_json::from_value(raw["items"][0]["message"].clone()).unwrap();
    let old_projection = projection(&legacy).unwrap();
    assert!(
        serde_json::from_str::<Value>(&old_projection)
            .unwrap()
            .get("reply_references")
            .is_none()
    );
    let mut message = raw["items"][0]["message"].clone();
    message["segments"] = json!([{"type":"text","text":"not another original body"},{"type":"at","reference":"2222"}]);
    let ordinary: Message = serde_json::from_value(message.clone()).unwrap();
    assert_eq!(projection(&ordinary).unwrap(), old_projection);
    // Existing plain messages must not spuriously update/withdraw owner edits on upgrade.
    assert_eq!(
        serde_json::to_value(&ordinary).unwrap(),
        serde_json::to_value(&legacy).unwrap()
    );
    message["segments"] = json!(
        (0..128)
            .map(|i| json!({"type":"reply","reference":format!("{}{}", "9".repeat(76), i)}))
            .collect::<Vec<_>>()
    );
    let many: Message = serde_json::from_value(message).unwrap();
    let text = projection(&many).unwrap();
    archives::validate_text(&text).unwrap();
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        body["reply_references"]["observed_markers"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    assert_eq!(body["reply_references"]["omitted_references"], 124);
    assert_eq!(body["original_message_text"]["text"], "bridge original");
}

#[tokio::test]
async fn invalid_reply_metadata_cannot_import_or_refresh_a_source() {
    let (_dir, k, s, c, r, server) = fixture().await;
    let invalid = [
        json!([{"type":"reply","reference":""}]),
        json!([{"type":"reply","reference":"unsafe\nidentity"}]),
        json!([{"type":"reply","reference":"9".repeat(81)}]),
        json!([{"type":"reply","reference":123}]),
        json!([{"type":"reply"}]),
        json!(
            (0..129)
                .map(|_| json!({"type":"reply","reference":"9001"}))
                .collect::<Vec<_>>()
        ),
        json!([{"type":"reply","reference":"fixture-knowledge-read-1234567890"}]),
    ];
    for segments in invalid {
        let mut raw = page(1, "must not import", "active");
        raw["items"][0]["message"]["segments"] = segments;
        *r.page.lock().unwrap() = raw;
        let failed = pull(&k, s, &c, None).await;
        assert_eq!(failed.state, ArchiveReadState::Failed);
        assert_eq!(failed.failure, Some(ArchiveReadFailure::InvalidContent));
        assert_eq!(k.archive_source(s).unwrap().total_records, 0);
        assert!(k.knowledge_state(s).unwrap().is_none());
    }
    let mut tombstone = page(1, "", "forgotten");
    tombstone["items"][0]["message"]["segments"] = json!([{"type":"reply","reference":"9001"}]);
    *r.page.lock().unwrap() = tombstone;
    assert_eq!(pull(&k, s, &c, None).await.state, ArchiveReadState::Failed);
    assert_eq!(r.calls.load(Ordering::SeqCst), 8);
    assert_eq!(k.archive_source(s).unwrap().total_records, 0);
    assert!(k.knowledge_state(s).unwrap().is_none());
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn selected_projection_updates_withdrawals_and_owned_cursors_are_atomic() {
    // fetch() resolves this test-only SecretRef in the same native process.
    let (_dir, k, s, c, r, server) = fixture().await;
    let first = pull(&k, s, &c, None).await;
    assert_eq!(first.state, ArchiveReadState::Completed);
    assert_eq!(first.new_records, 1);
    let id = first.record_ids[0];
    let initial = k.archive_record(id).unwrap();
    assert_eq!(initial.text_origin, ArchiveTextOrigin::KnowledgeProjection);
    let text = initial.text.unwrap();
    assert!(
        text.contains("external_model_inference_not_sender_words") && text.contains("a red square")
    );
    let old_deadline = initial.retain_until_ms;
    assert!(
        start_config(
            k.clone(),
            s,
            request(None),
            Redactor::new(vec![]),
            c.clone()
        )
        .await
        .is_err()
    );
    let second = pull(&k, s, &c, Some(first.request.request_id)).await;
    assert_eq!(second.state, ArchiveReadState::Completed);
    assert_eq!(k.archive_record(id).unwrap().retain_until_ms, old_deadline);
    *r.page.lock().unwrap() = page(2, "bridge corrected upstream", "active");
    let third = pull(&k, s, &c, Some(second.request.request_id)).await;
    assert_eq!(third.updated_records, 1);
    assert_eq!(k.archive_record(id).unwrap().revision, 2);
    *r.page.lock().unwrap() = page(3, "", "recalled");
    let fourth = pull(&k, s, &c, Some(third.request.request_id)).await;
    assert_eq!(fourth.withdrawn_records, 1);
    assert!(k.archive_record(id).unwrap().text.is_none());
    *r.page.lock().unwrap() = page(4, "must never resurrect", "active");
    let fifth = pull(&k, s, &c, Some(fourth.request.request_id)).await;
    assert_eq!(fifth.new_records, 0);
    assert!(k.archive_record(id).unwrap().text.is_none());
    let calls = r.calls.load(Ordering::SeqCst);
    let same = start_config(
        k.clone(),
        s,
        fifth.request.clone(),
        Redactor::new(vec![]),
        c.clone(),
    )
    .await
    .unwrap();
    assert_eq!(same.state, ArchiveReadState::Completed);
    assert_eq!(r.calls.load(Ordering::SeqCst), calls);
    assert!(k.list_sessions().unwrap().is_empty());
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn bounded_coverage_survives_bridge_without_liveness_or_legacy_claims() {
    let (_dir, k, s, c, r, server) = fixture().await;
    r.page.lock().unwrap()["coverage"] = json!({"observed_at_ms":2000,"bounded_history":true,"source_epochs_untracked":true,
        "events":[{"sequence":7,"kind":"disconnected_gap","recorded_at_ms":1800}]});
    let first = pull(&k, s, &c, None).await;
    assert_eq!(first.state, ArchiveReadState::Completed);
    let state = k.knowledge_state(s).unwrap().unwrap();
    let coverage = state.coverage.unwrap();
    assert_eq!(
        coverage.events[0].kind,
        ArchiveCoverageKind::DisconnectedGap
    );
    assert_eq!(coverage.events[0].recorded_at_ms, 1800);
    assert!(coverage.source_epochs_untracked);
    let status = k.archive_source_status(s).unwrap();
    assert_eq!(status.receiving_now, None);
    r.page
        .lock()
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("coverage");
    let second = pull(&k, s, &c, Some(first.request.request_id)).await;
    assert_eq!(second.state, ArchiveReadState::Completed);
    assert!(k.knowledge_state(s).unwrap().unwrap().coverage.is_none());
    let mut bad = page(2, "must not import", "active");
    bad["coverage"] = json!({"observed_at_ms":1000,"bounded_history":true,"source_epochs_untracked":true,
        "events":[{"sequence":8,"kind":"disconnected_gap","recorded_at_ms":1800}]});
    *r.page.lock().unwrap() = bad;
    let failed = pull(&k, s, &c, Some(second.request.request_id)).await;
    assert_eq!(failed.state, ArchiveReadState::Failed);
    assert!(k.knowledge_state(s).unwrap().unwrap().coverage.is_none());
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn mismatched_page_and_cancelled_pull_never_publish_or_refresh_a_source() {
    let (_dir, k, s, c, r, server) = fixture().await;
    let denied = super::start_resolved(
        k.clone(),
        s,
        request(None),
        Redactor::new(vec![]),
        c.clone(),
        SecretValue(zeroize::Zeroizing::new("wrong-fixture-key".into())),
    )
    .await
    .unwrap();
    assert_eq!(
        settled(&k, denied.request.request_id)
            .await
            .provider_error_code,
        Some(403)
    );
    r.page.lock().unwrap()["store_id"] = json!("different-store");
    let failed = pull(&k, s, &c, None).await;
    assert_eq!(failed.state, ArchiveReadState::Failed);
    assert_eq!(k.archive_source(s).unwrap().total_records, 0);
    *r.page.lock().unwrap() = page(1, "bridge original", "active");
    let good = pull(&k, s, &c, None).await;
    let id = good.record_ids[0];
    r.gate.forget_permits(20);
    let j = start_config(
        k.clone(),
        s,
        request(Some(good.request.request_id)),
        Redactor::new(vec![]),
        c.clone(),
    )
    .await
    .unwrap();
    assert!(k.archive_record(id).unwrap().text.is_none());
    k.archive_read_fail(
        j.request.request_id,
        ArchiveReadState::Cancelled,
        ArchiveReadFailure::Interrupted,
        None,
    )
    .unwrap();
    assert_eq!(
        settled(&k, j.request.request_id).await.state,
        ArchiveReadState::Cancelled
    );
    assert_eq!(
        k.knowledge_state(s).unwrap().unwrap().last_read,
        good.request.request_id
    );
    r.gate.add_permits(20);
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn receipt_failure_rolls_back_projection_and_expired_or_remote_changed_sources_are_withheld()
{
    let (dir, k, s, c, r, server) = fixture().await;
    let db = rusqlite::Connection::open(dir.path().join("state.db")).unwrap();
    db.execute_batch("CREATE TRIGGER reject_knowledge_receipt BEFORE UPDATE ON archive_reads WHEN json_extract(NEW.snapshot,'$.state')='completed' BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    let failed = pull(&k, s, &c, None).await;
    assert_eq!(failed.state, ArchiveReadState::Failed);
    assert_eq!(k.archive_source(s).unwrap().total_records, 0);
    assert!(k.knowledge_state(s).unwrap().is_none());
    assert_eq!(
        db.query_row("SELECT count(*) FROM knowledge_records", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    db.execute_batch("DROP TRIGGER reject_knowledge_receipt;")
        .unwrap();
    let first = pull(&k, s, &c, None).await;
    let id = first.record_ids[0];
    let deadline = k.archive_record(id).unwrap().retain_until_ms;
    db.execute(
        "UPDATE knowledge_sources SET snapshot=json_set(snapshot,'$.fresh_until_ms',0)",
        [],
    )
    .unwrap();
    assert_eq!(
        k.archive_record(id).unwrap().availability,
        ArchiveAvailability::SourceUnavailable
    );
    let refreshed = pull(&k, s, &c, Some(first.request.request_id)).await;
    assert!(k.archive_record(id).unwrap().text.is_some());
    assert_eq!(k.archive_record(id).unwrap().retain_until_ms, deadline);
    *r.page.lock().unwrap() = page(2, "", "revoked");
    r.page.lock().unwrap()["source"]["revision"] = json!(2);
    r.page.lock().unwrap()["source"]["enabled"] = json!(false);
    let stopped = pull(&k, s, &c, Some(refreshed.request.request_id)).await;
    assert!(stopped.withdrawn_records > 0);
    assert!(k.archive_record(id).unwrap().text.is_none());
    assert!(!k.knowledge_state(s).unwrap().unwrap().remote_source_enabled);
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn owner_correction_is_not_silently_overwritten_and_shutdown_closes_owned_http_read() {
    let (_dir, k, s, c, r, server) = fixture().await;
    let first = pull(&k, s, &c, None).await;
    let id = first.record_ids[0];
    k.archive_correct(
        id,
        CorrectArchiveRecord {
            expected_revision: 1,
            text: "owner corrected observation".into(),
        },
        Redactor::new(vec![]),
    )
    .unwrap();
    *r.page.lock().unwrap() = page(2, "different upstream projection", "active");
    let changed = pull(&k, s, &c, Some(first.request.request_id)).await;
    assert_eq!(changed.withdrawn_records, 1);
    assert!(k.archive_record(id).unwrap().text.is_none());
    r.gate.forget_permits(20);
    let j = start_config(
        k.clone(),
        s,
        request(Some(changed.request.request_id)),
        Redactor::new(vec![]),
        c.clone(),
    )
    .await
    .unwrap();
    k.shutdown().unwrap();
    drain(&k).await.unwrap();
    let state = k.archive_read_job(j.request.request_id).unwrap().state;
    assert!(matches!(
        state,
        ArchiveReadState::Cancelled | ArchiveReadState::Failed
    ));
    r.gate.add_permits(20);
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn source_cursor_mismatch_and_unseen_tombstone_cannot_create_or_resurrect_records() {
    let (_dir, k, s, c, r, server) = fixture().await;
    r.page.lock().unwrap()["source"]["peer"] = json!("9999");
    assert_eq!(pull(&k, s, &c, None).await.state, ArchiveReadState::Failed);
    *r.page.lock().unwrap() = page(1, "invalid sender metadata", "active");
    r.page.lock().unwrap()["items"][0]["message"]["sender"] = json!("unsafe\nmetadata");
    assert_eq!(pull(&k, s, &c, None).await.state, ArchiveReadState::Failed);
    *r.page.lock().unwrap() = page(1, "", "forgotten");
    let tombstone = pull(&k, s, &c, None).await;
    assert_eq!(tombstone.state, ArchiveReadState::Completed);
    assert_eq!(tombstone.new_records, 0);
    assert_eq!(tombstone.withdrawn_records, 1);
    *r.page.lock().unwrap() = page(2, "must not restore unseen withdrawn ID", "active");
    let next = pull(&k, s, &c, Some(tombstone.request.request_id)).await;
    assert_eq!(next.new_records, 0);
    assert_eq!(k.archive_source(s).unwrap().total_records, 0);
    server.abort();
    let _ = server.await;
}
