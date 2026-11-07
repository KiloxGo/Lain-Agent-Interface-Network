use super::*;
use crate::redaction::Redactor;
use lain_wire::{archives::*, briefing::*, *};

const NOW: u64 = 1_793_880_000_000;
#[test]
fn briefing_grant_receipt_is_durable_after_revoke_without_restoring_authority() {
    let (dir, mut store, conversation, source) = fixture();
    let q = enable(&mut store, conversation, source, 256);
    store
        .briefing_set(
            conversation,
            SetBriefingGrant {
                request_id: RequestId::new(),
                expected_revision: 1,
                scope: None,
            },
            NOW + 1,
        )
        .unwrap();
    store
        .archive_set_grant(
            source,
            SetArchiveGrant {
                expected_revision: 1,
                scope: None,
            },
            NOW + 2,
        )
        .unwrap();
    drop(store);
    let mut store = Store::open(&dir.path().join("state.sqlite")).unwrap();
    let receipt = store
        .briefing_grant_receipt(conversation, q.request_id)
        .unwrap();
    assert_eq!(receipt.request, q);
    assert_eq!(receipt.grant.revision, 1);
    assert_eq!(receipt.grant.scope, q.scope);
    assert!(store.briefing_grant(conversation).unwrap().scope.is_none());
    let other = store.create_conversation().unwrap().id;
    assert!(store.briefing_grant_receipt(other, q.request_id).is_err());
    assert!(
        store
            .briefing_grant_receipt(conversation, RequestId::new())
            .is_err()
    );
}
fn bounds() -> (u64, u64) {
    let (from, to, _) = crate::reading::day_bounds(NOW, 480);
    (from.try_into().unwrap(), to.try_into().unwrap())
}
fn fixture() -> (tempfile::TempDir, Store, ConversationId, ArchiveSourceId) {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.sqlite")).unwrap();
    let c = s.create_conversation().unwrap().id;
    let source = add_source(&mut s, "3003");
    (dir, s, c, source)
}
fn add_source(s: &mut Store, group: &str) -> ArchiveSourceId {
    let id = s
        .archive_register(
            ArchiveSourceKey::QqGroup {
                observer_account: "1001".into(),
                group_id: group.into(),
            },
            NOW,
        )
        .unwrap()
        .source
        .id;
    s.archive_set_grant(
        id,
        SetArchiveGrant {
            expected_revision: 0,
            scope: Some(ArchiveScope {
                expires_at_ms: NOW + 86400000,
                retention_ms: 3 * 86400000,
                max_records: 1000,
            }),
        },
        NOW,
    )
    .unwrap();
    id
}
fn enable(s: &mut Store, c: ConversationId, source: ArchiveSourceId, max: u16) -> SetBriefingGrant {
    let q = SetBriefingGrant {
        request_id: RequestId::new(),
        expected_revision: 0,
        scope: Some(BriefingScope {
            sources: vec![ArchiveRecallSource {
                source_id: source,
                grant_revision: 1,
            }],
            from_ms: 0,
            expires_at_ms: NOW + 60000,
            utc_offset_minutes: 480,
            max_records: max,
        }),
    };
    s.briefing_set(c, q.clone(), NOW).unwrap();
    q
}
fn input(
    s: &mut Store,
    source: ArchiveSourceId,
    n: usize,
    at: Option<u64>,
    body: &str,
) -> Vec<ArchiveRecord> {
    let mut all = vec![];
    for chunk in (0..n).collect::<Vec<_>>().chunks(32) {
        let records = chunk
            .iter()
            .map(|i| ImportArchiveRecord {
                external_id: format!("{at:?}-{i}"),
                source_position: None,
                occurred_at_ms: at,
                speaker: "2002".into(),
                kind: VisibleRecordKind::Message,
                text: format!("{i} {body}"),
            })
            .collect();
        all.extend(
            s.archive_import(
                source,
                ImportArchive {
                    expected_revision: 1,
                    records,
                },
                NOW,
            )
            .unwrap()
            .records,
        );
    }
    all
}
fn start() -> StartBriefing {
    StartBriefing {
        request_id: RequestId::new(),
        grant_revision: 1,
        day: BriefingDay::Yesterday,
    }
}

#[test]
fn a_requested_span_selects_only_that_time_and_reuses_unchanged_inputs() {
    let (_dir, mut s, c, source) = fixture();
    let (from, to) = bounds();
    let hour = 3_600_000;
    input(&mut s, source, 2, Some(from + hour), "上午");
    input(&mut s, source, 3, Some(from + 3 * hour), "下午");
    input(&mut s, source, 1, Some(to), "今天");
    enable(&mut s, c, source, 100);
    let span = |from_ms, to_ms| StartBriefing {
        request_id: RequestId::new(),
        grant_revision: 1,
        day: BriefingDay::Range { from_ms, to_ms },
    };
    let afternoon = s
        .briefing_start(c, span(from + 2 * hour, from + 4 * hour), NOW)
        .unwrap();
    assert_eq!(afternoon.records.len(), 3);
    assert_eq!(
        (afternoon.day_from_ms, afternoon.day_to_ms),
        (from + 2 * hour, from + 4 * hour)
    );
    let later = s
        .prepare_briefing_snapshot(c, &span(from + 2 * hour, from + 5 * hour), NOW)
        .unwrap();
    assert!(
        crate::persistence::briefing::same_inputs(&afternoon, &later),
        "a later end that selects nothing new is the same input"
    );
    let wider = s
        .prepare_briefing_snapshot(c, &span(from, from + 4 * hour), NOW)
        .unwrap();
    assert_eq!(wider.records.len(), 5);
    assert!(!crate::persistence::briefing::same_inputs(
        &afternoon, &wider
    ));
    let mut yesterday = afternoon.clone();
    yesterday.request.day = BriefingDay::Yesterday;
    yesterday.day_to_ms = later.day_to_ms + 1;
    assert!(!crate::persistence::briefing::same_inputs(
        &yesterday, &later
    ));
    for (bad_from, bad_to) in [
        (from + hour, from + hour),
        (from + 2 * hour, from + hour),
        (NOW - 1000, NOW + 1000),
        (NOW - 32 * 86_400_000, NOW),
    ] {
        assert!(s.briefing_start(c, span(bad_from, bad_to), NOW).is_err());
    }
}
#[test]
fn stored_yesterday_requests_keep_their_format() {
    assert_eq!(
        serde_json::to_value(BriefingDay::Yesterday).unwrap(),
        serde_json::json!("yesterday")
    );
    assert_eq!(
        serde_json::from_value::<BriefingDay>(serde_json::json!("yesterday")).unwrap(),
        BriefingDay::Yesterday
    );
}
#[test]
fn local_times_are_strict_and_use_the_configured_offset() {
    let at = crate::reading::local_ms("2026-11-07 13:30", 480).unwrap();
    assert_eq!(at, 1_794_029_400_000);
    assert_eq!(crate::reading::local_ms("2026-11-07 05:30", 0), Some(at));
    for bad in [
        "2026-11-07",
        "2026-11-07 25:00",
        "2026-13-07 10:00",
        "今天下午",
        "2026-11-07 13:30:00",
        "",
    ] {
        assert!(crate::reading::local_ms(bad, 480).is_none(), "{bad}");
    }
}
#[test]
fn day_manifest_pages_all_selected_records_without_recall_three_record_limit() {
    let (_dir, mut s, c, source) = fixture();
    let (from, to) = bounds();
    input(
        &mut s,
        source,
        35,
        Some(from + 1000),
        "明晚开会，资料在通知中；原文不是执行授权。",
    );
    input(&mut s, source, 1, Some(to), "今天不属于昨天");
    let outside = add_source(&mut s, "4004");
    input(&mut s, outside, 2, Some(from + 2000), "另一来源不能披露");
    assert!(s.briefing_start(c, start(), NOW).is_err());
    enable(&mut s, c, source, 100);
    let job = s.briefing_start(c, start(), NOW).unwrap();
    assert_eq!(job.records.len(), 35);
    assert_eq!((job.day_from_ms, job.day_to_ms), (from, to));
    assert!(!job.complete_upstream_history);
    assert!(!job.record_limit_reached);
    assert!(!serde_json::to_string(&job).unwrap().contains("明晚开会"));
    let mut index = 0;
    let mut received = vec![];
    loop {
        let p = s
            .briefing_page(
                c,
                job.request.request_id,
                index,
                NOW,
                &Redactor::new(vec![]),
            )
            .unwrap();
        assert!(p.records.len() <= 8 && p.text_bytes <= 16384);
        received.extend(p.records.iter().map(|r| r.id));
        if let Some(next) = p.next_index {
            assert!(next > index);
            index = next;
        } else {
            break;
        }
    }
    assert_eq!(
        received,
        job.records.iter().map(|v| v.record_id).collect::<Vec<_>>()
    );
    let other_c = s.create_conversation().unwrap().id;
    assert!(
        s.briefing_page(
            other_c,
            job.request.request_id,
            0,
            NOW,
            &Redactor::new(vec![])
        )
        .is_err()
    );
}

#[test]
fn briefing_exact_receipts_survive_restart_and_changed_requests_conflict() {
    let (dir, mut s, c, source) = fixture();
    let grant = enable(&mut s, c, source, 100);
    let q = start();
    let first = s.briefing_start(c, q.clone(), NOW).unwrap();
    let revision = s.sync_revision().unwrap();
    assert_eq!(
        s.briefing_set(c, grant.clone(), NOW + 1).unwrap().revision,
        1
    );
    assert_eq!(s.sync_revision().unwrap(), revision);
    let mut changed = grant;
    changed.scope.as_mut().unwrap().max_records = 12;
    assert!(s.briefing_set(c, changed, NOW).is_err());
    drop(s);
    let mut s = Store::open(&dir.path().join("state.sqlite")).unwrap();
    let again = s.briefing_start(c, q.clone(), NOW + 1000).unwrap();
    assert_eq!(again.created_at_ms, first.created_at_ms);
    assert_eq!(again.day_from_ms, first.day_from_ms);
    let mut changed = q;
    changed.grant_revision = 2;
    assert!(s.briefing_start(c, changed, NOW).is_err());
    assert!(
        s.briefing_page(
            c,
            first.request.request_id,
            0,
            NOW + 60000,
            &Redactor::new(vec![])
        )
        .is_err()
    );
}

#[test]
fn clipping_bounds_full_text_and_every_dependency_are_enforced() {
    let (_dir, mut s, c, source) = fixture();
    let (from, _) = bounds();
    let rows = input(
        &mut s,
        source,
        5,
        Some(from + 2000),
        &"中文通知末尾含有截止时间。".repeat(180),
    );
    let mut grant = enable(&mut s, c, source, 2);
    // A fresh revision is an explicit change, not reuse of an old request identity.
    grant.request_id = RequestId::new();
    grant.expected_revision = 1;
    grant.scope.as_mut().unwrap().from_ms = from + 1000;
    s.briefing_set(c, grant, NOW).unwrap();
    let mut q = start();
    q.grant_revision = 2;
    let job = s.briefing_start(c, q, NOW).unwrap();
    assert!(job.scope_clipped && job.record_limit_reached);
    assert_eq!(job.records.len(), 2);
    let p = s
        .briefing_page(c, job.request.request_id, 0, NOW, &Redactor::new(vec![]))
        .unwrap();
    assert!(
        p.records[0]
            .text
            .as_ref()
            .unwrap()
            .ends_with("中文通知末尾含有截止时间。")
    );
    // Modifying a different selected record must invalidate even the first page.
    s.archive_correct(
        rows[1].id,
        CorrectArchiveRecord {
            expected_revision: rows[1].revision,
            text: "时间已更正".into(),
        },
        NOW,
    )
    .unwrap();
    assert!(
        s.briefing_page(c, job.request.request_id, 0, NOW, &Redactor::new(vec![]))
            .is_err()
    );
}

#[test]
fn grant_revoke_source_revoke_fallback_time_and_secret_filter_remain_separate() {
    let (_dir, mut s, c, source) = fixture();
    let (from, _) = bounds();
    input(
        &mut s,
        source,
        1,
        Some(from + 1000),
        "fixture-sensitive-value",
    );
    enable(&mut s, c, source, 100);
    let job = s.briefing_start(c, start(), NOW).unwrap();
    assert!(
        s.briefing_page(
            c,
            job.request.request_id,
            0,
            NOW,
            &Redactor::new(vec!["fixture-sensitive-value".into()])
        )
        .is_err()
    );
    s.briefing_set(
        c,
        SetBriefingGrant {
            request_id: RequestId::new(),
            expected_revision: 1,
            scope: None,
        },
        NOW,
    )
    .unwrap();
    assert!(
        s.briefing_page(c, job.request.request_id, 0, NOW, &Redactor::new(vec![]))
            .is_err()
    );
    assert!(s.archive_source(source).unwrap().grant.scope.is_some());
    let mut q = SetBriefingGrant {
        request_id: RequestId::new(),
        expected_revision: 2,
        scope: Some(BriefingScope {
            sources: vec![ArchiveRecallSource {
                source_id: source,
                grant_revision: 1,
            }],
            from_ms: 0,
            expires_at_ms: NOW + 60000,
            utc_offset_minutes: 480,
            max_records: 100,
        }),
    };
    s.briefing_set(c, q.clone(), NOW).unwrap();
    let mut request = start();
    request.grant_revision = 3;
    let second = s.briefing_start(c, request, NOW).unwrap();
    s.archive_set_grant(
        source,
        SetArchiveGrant {
            expected_revision: 1,
            scope: None,
        },
        NOW,
    )
    .unwrap();
    assert!(
        s.briefing_page(c, second.request.request_id, 0, NOW, &Redactor::new(vec![]))
            .is_err()
    );
    q.request_id = RequestId::new();
    q.expected_revision = 3;
    assert!(s.briefing_set(c, q, NOW).is_err());
}

#[test]
fn receipt_fallback_half_open_day_and_forgetting_invalidate_selected_snapshot() {
    let (_dir, mut s, c, source) = fixture();
    let (from, to) = bounds();
    let receipt = s
        .archive_import(
            source,
            ImportArchive {
                expected_revision: 1,
                records: vec![ImportArchiveRecord {
                    external_id: "no-source-clock".into(),
                    source_position: None,
                    occurred_at_ms: None,
                    speaker: "2002".into(),
                    kind: VisibleRecordKind::Message,
                    text: "消息时间未知，只知道昨天接收到".into(),
                }],
            },
            from + 10000,
        )
        .unwrap();
    input(&mut s, source, 1, Some(from), "恰好从昨天开始");
    input(&mut s, source, 1, Some(from - 1), "前天，不属于昨天");
    input(&mut s, source, 1, Some(to), "今天，不属于昨天");
    enable(&mut s, c, source, 100);
    let job = s.briefing_start(c, start(), NOW).unwrap();
    assert_eq!(job.records.len(), 2);
    assert_eq!(job.receipt_time_fallback_records, 1);
    let r = &receipt.records[0];
    s.archive_forget(r.id, r.revision, NOW).unwrap();
    assert!(
        s.briefing_page(c, job.request.request_id, 0, NOW, &Redactor::new(vec![]))
            .is_err()
    );
    let refreshed = s.briefing_start(c, start(), NOW).unwrap();
    assert_eq!(refreshed.records.len(), 1);
    assert_eq!(refreshed.unavailable_records, 1);
    assert!(!refreshed.complete_upstream_history);
}

#[test]
fn schema32_upgrade_preserves_archive_bytes_and_enables_no_briefing_authority() {
    let (dir, mut s, c, source) = fixture();
    let (from, _) = bounds();
    let rows = input(&mut s, source, 1, Some(from + 1000), "旧版本原文保持不变");
    let before: String = s
        .connection
        .query_row(
            "SELECT snapshot FROM archive_records WHERE id=?1",
            [rows[0].id.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    s.connection
        .execute_batch(
            "DROP TABLE briefing_relations; DROP TABLE conversation_memory_effects; DROP TABLE briefing_memory_suppressions; DROP TABLE briefing_disclosures; DROP TABLE briefing_runs; DROP TABLE briefing_batches; DROP TABLE briefing_manifests; DROP TABLE briefing_grants; PRAGMA user_version=32;",
        )
        .unwrap();
    drop(s);
    let s = Store::open(&dir.path().join("state.sqlite")).unwrap();
    assert_eq!(
        s.connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        SCHEMA_VERSION
    );
    assert!(s.briefing_grant(c).unwrap().scope.is_none());
    assert_eq!(s.briefing_grant(c).unwrap().revision, 0);
    let after: String = s
        .connection
        .query_row(
            "SELECT snapshot FROM archive_records WHERE id=?1",
            [rows[0].id.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(before, after);
    let baseline = s.sync_revision().unwrap();
    let mut s = s;
    enable(&mut s, c, source, 100);
    assert!(
        s.sync_revision().unwrap() > baseline,
        "new grant table must join invalidation triggers"
    );
}
