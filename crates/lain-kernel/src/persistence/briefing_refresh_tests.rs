use super::Store;
use crate::{
    actor::{
        domain::*,
        tools::{Empty, Tool},
    },
    domain::now_ms,
};
use lain_wire::{RequestId, archives::*, briefing::*};

const DAY_MS: u64 = 86_400_000;

struct Fixture {
    _dir: tempfile::TempDir,
    store: Store,
    conversation: ConversationId,
    source: lain_wire::ArchiveSourceId,
    now: u64,
    yesterday: u64,
    today: u64,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("briefing-refresh.sqlite")).unwrap();
        let conversation = store.create_conversation().unwrap().id;
        let now = now_ms().unwrap();
        // Keep this real-clock API test away from local midnight without assuming a date.
        let utc_minute = ((now / 60_000) % 1440) as i16;
        let offset = (720 - utc_minute + 720).rem_euclid(1440) - 720;
        let (yesterday, today, _) = crate::reading::day_bounds(now, offset);
        let source = add_source(&mut store, now, "3003");
        store
            .briefing_set(
                conversation,
                SetBriefingGrant {
                    request_id: RequestId::new(),
                    expected_revision: 0,
                    scope: Some(BriefingScope {
                        sources: vec![ArchiveRecallSource {
                            source_id: source,
                            grant_revision: 1,
                        }],
                        from_ms: 0,
                        expires_at_ms: now + DAY_MS,
                        utc_offset_minutes: offset,
                        max_records: 100,
                    }),
                },
                now,
            )
            .unwrap();
        Self {
            _dir: dir,
            store,
            conversation,
            source,
            now,
            yesterday: yesterday.try_into().unwrap(),
            today: today.try_into().unwrap(),
        }
    }

    fn request(&mut self) -> (ActorTurnId, ActionId, RequestId) {
        let (turn, action) = executing_request(&mut self.store, self.conversation);
        let briefing = self.store.prepare_actor_briefing(turn, action).unwrap();
        (turn, action, briefing)
    }
}

fn add_source(store: &mut Store, now: u64, group: &str) -> lain_wire::ArchiveSourceId {
    let source = store
        .archive_register(
            ArchiveSourceKey::QqGroup {
                observer_account: "1001".into(),
                group_id: group.into(),
            },
            now,
        )
        .unwrap()
        .source
        .id;
    store
        .archive_set_grant(
            source,
            SetArchiveGrant {
                expected_revision: 0,
                scope: Some(ArchiveScope {
                    expires_at_ms: now + 2 * DAY_MS,
                    retention_ms: 3 * DAY_MS,
                    max_records: 1000,
                }),
            },
            now,
        )
        .unwrap();
    source
}

fn import(
    store: &mut Store,
    source: lain_wire::ArchiveSourceId,
    now: u64,
    at: u64,
    external_id: &str,
) -> ArchiveRecord {
    store
        .archive_import(
            source,
            ImportArchive {
                expected_revision: 1,
                records: vec![ImportArchiveRecord {
                    external_id: external_id.into(),
                    source_position: None,
                    occurred_at_ms: Some(at),
                    speaker: "2002".into(),
                    kind: VisibleRecordKind::Message,
                    text: format!("synthetic notice {external_id}"),
                }],
            },
            now,
        )
        .unwrap()
        .records
        .remove(0)
}

fn executing_request(store: &mut Store, conversation: ConversationId) -> (ActorTurnId, ActionId) {
    executing_tool(store, conversation, Tool::BriefingYesterday(Empty {}))
}

fn executing_tool(
    store: &mut Store,
    conversation: ConversationId,
    tool: Tool,
) -> (ActorTurnId, ActionId) {
    let action = ActionId::new();
    let mut turn = ActorTurn {
        id: ActorTurnId::new(),
        conversation_id: conversation,
        request_id: uuid::Uuid::new_v4(),
        sequence: 0,
        revision: 0,
        state: TurnState::Running,
        messages: vec![],
        actions: vec![Action {
            id: action,
            call_id: "synthetic-briefing-yesterday".into(),
            tool,
            state: ActionState::Executing,
            result: None,
        }],
        error: None,
        model_requests: 0,
        usage_tokens: 0,
        timing: Default::default(),
        reading_activity: None,
        briefing_activity: None,
        sharing_activity: None,
        channel_memory: None,
        channel_source: None,
        private_chat: None,
    };
    turn.message(MessageRole::User, "昨天有什么重要消息？".into())
        .unwrap();
    store.save_actor_turn(&mut turn, true).unwrap();
    (turn.id, action)
}

#[test]
fn default_latest_read_requires_refresh_after_late_yesterday_input_without_starting_work() {
    let mut f = Fixture::new();
    let (_, _, original) = f.request();
    assert!(
        !f.store
            .begin_briefing_run(f.conversation, original)
            .unwrap()
    );
    let snapshot =
        serde_json::to_value(f.store.briefing_manifest(f.conversation, original).unwrap()).unwrap();
    import(&mut f.store, f.source, f.now, f.yesterday + 1000, "late");
    let (turn, action) = executing_tool(
        &mut f.store,
        f.conversation,
        Tool::BriefingRead(BriefingRead {
            briefing_id: None,
            after: 0,
            limit: 4,
            query: Some("late".into()),
        }),
    );
    let notice = f
        .store
        .briefing_actor_lookup(turn, action, &crate::redaction::Redactor::new(vec![]))
        .unwrap();
    assert_eq!(notice["refresh_required"], true);
    assert!(notice.get("items").is_none());
    let (turn, action) = executing_tool(
        &mut f.store,
        f.conversation,
        Tool::BriefingRead(BriefingRead {
            briefing_id: Some(original),
            after: 0,
            limit: 4,
            query: None,
        }),
    );
    let historical = f
        .store
        .briefing_actor_lookup(turn, action, &crate::redaction::Redactor::new(vec![]))
        .unwrap();
    assert!(historical["items"].as_array().unwrap().is_empty());
    assert_eq!(historical["briefing_id"], original.to_string());
    assert_eq!(
        serde_json::to_value(f.store.briefing_manifest(f.conversation, original).unwrap()).unwrap(),
        snapshot
    );
    for table in ["briefing_manifests", "briefing_runs"] {
        assert_eq!(
            f.store
                .connection
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            1,
            "a read notice must not create or replay work"
        );
    }
}

#[test]
fn briefing_phrase_lookup_schema_gate_preserves_legacy_actions_and_existing_rows() {
    let mut f = Fixture::new();
    let (turn, _, brief) = f.request();
    let snapshot =
        serde_json::to_value(f.store.briefing_manifest(f.conversation, brief).unwrap()).unwrap();
    // 040 changes only the reader gate, so a closed 039 fixture has the exact
    // same tables. No production database is downgraded or copied.
    f.store
        .connection
        .pragma_update(None, "user_version", 39)
        .unwrap();
    let path = f._dir.path().join("briefing-refresh.sqlite");
    drop(f.store);
    let mut store = Store::open(&path).unwrap();
    assert_eq!(
        store
            .connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        40
    );
    assert_eq!(
        serde_json::to_value(store.briefing_manifest(f.conversation, brief).unwrap()).unwrap(),
        snapshot
    );
    assert_eq!(
        store.actor_turn(turn).unwrap().conversation_id,
        f.conversation
    );
    let (read, _) = executing_tool(
        &mut store,
        f.conversation,
        Tool::BriefingRead(BriefingRead {
            briefing_id: Some(brief),
            after: 0,
            limit: 4,
            query: Some("图书馆".into()),
        }),
    );
    let serialized = serde_json::to_value(store.actor_turn(read).unwrap()).unwrap();
    assert_eq!(
        serialized["actions"][0]["tool"]["arguments"]["query"],
        "图书馆"
    );
    let old: BriefingRead =
        serde_json::from_str(r#"{"briefing_id":null,"after":0,"limit":4}"#).unwrap();
    assert!(old.query.is_none());
    assert!(serde_json::to_value(old).unwrap().get("query").is_none());
}

#[test]
fn default_latest_read_ignores_today_only_arrivals_and_keeps_source_revocation_gate() {
    let mut f = Fixture::new();
    let (_, _, original) = f.request();
    assert!(
        !f.store
            .begin_briefing_run(f.conversation, original)
            .unwrap()
    );
    import(&mut f.store, f.source, f.now, f.today + 1000, "today");
    let read = || {
        Tool::BriefingRead(BriefingRead {
            briefing_id: None,
            after: 0,
            limit: 4,
            query: None,
        })
    };
    let (turn, action) = executing_tool(&mut f.store, f.conversation, read());
    let value = f
        .store
        .briefing_actor_lookup(turn, action, &crate::redaction::Redactor::new(vec![]))
        .unwrap();
    assert_ne!(value["refresh_required"], true);
    assert_eq!(value["briefing_id"], original.to_string());
    f.store
        .archive_set_grant(
            f.source,
            SetArchiveGrant {
                expected_revision: 1,
                scope: None,
            },
            f.now,
        )
        .unwrap();
    let (turn, action) = executing_tool(&mut f.store, f.conversation, read());
    assert!(
        f.store
            .briefing_actor_lookup(turn, action, &crate::redaction::Redactor::new(vec![]))
            .is_err()
    );
}

#[test]
fn fresh_ordinary_request_includes_late_imported_yesterday_record_and_preserves_original_receipt() {
    let mut f = Fixture::new();
    let first_record = import(&mut f.store, f.source, f.now, f.yesterday + 1000, "first");
    let (original_turn, original_action, original_id) = f.request();
    assert_eq!(original_id, RequestId(original_action.0));
    let original = f
        .store
        .briefing_manifest(f.conversation, original_id)
        .unwrap();
    let original_snapshot = serde_json::to_value(&original).unwrap();
    assert_eq!(
        original
            .records
            .iter()
            .map(|r| r.record_id)
            .collect::<Vec<_>>(),
        vec![first_record.id]
    );

    // Receipt is today, while the source reports yesterday: this must enter a fresh selection.
    let late = import(
        &mut f.store,
        f.source,
        f.now + 1,
        f.yesterday + 2000,
        "late-yesterday",
    );
    let (_, fresh_action, fresh_id) = f.request();
    assert_ne!(
        fresh_id, original_id,
        "new ordinary requests must see newly imported yesterday inputs"
    );
    assert_eq!(fresh_id, RequestId(fresh_action.0));
    let fresh = f.store.briefing_manifest(f.conversation, fresh_id).unwrap();
    assert_eq!(
        fresh
            .records
            .iter()
            .map(|r| r.record_id)
            .collect::<Vec<_>>(),
        vec![first_record.id, late.id]
    );

    assert_eq!(
        f.store
            .prepare_actor_briefing(original_turn, original_action)
            .unwrap(),
        original_id,
        "an original prepare action must never be retargeted to the newer manifest"
    );
    let receipt = f
        .store
        .briefing_start(f.conversation, original.request.clone(), f.now + DAY_MS)
        .unwrap();
    assert_eq!(serde_json::to_value(receipt).unwrap(), original_snapshot);
    assert_eq!(
        serde_json::to_value(
            f.store
                .briefing_manifest(f.conversation, original_id)
                .unwrap()
        )
        .unwrap(),
        original_snapshot,
        "refresh must not mutate the old manifest, including its source snapshot"
    );
    assert_eq!(
        f.request().2,
        fresh_id,
        "unchanged refreshed inputs should reuse the fresh manifest"
    );
}

#[test]
fn ordinary_requests_reuse_yesterday_manifest_for_unchanged_today_and_unselected_source_inputs() {
    let mut f = Fixture::new();
    import(
        &mut f.store,
        f.source,
        f.now,
        f.yesterday + 1000,
        "yesterday",
    );
    let (_, _, original_id) = f.request();
    let original = serde_json::to_value(
        f.store
            .briefing_manifest(f.conversation, original_id)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        f.request().2,
        original_id,
        "unchanged inputs must not repeat processing"
    );

    import(&mut f.store, f.source, f.now + 1, f.today + 1000, "today");
    assert_eq!(
        f.request().2,
        original_id,
        "today's same-source message must not refresh yesterday"
    );

    let outside = add_source(&mut f.store, f.now, "4004");
    import(
        &mut f.store,
        outside,
        f.now + 2,
        f.yesterday + 2000,
        "unselected-source",
    );
    assert_eq!(
        f.request().2,
        original_id,
        "messages outside the granted source set must not refresh"
    );
    assert_eq!(
        serde_json::to_value(
            f.store
                .briefing_manifest(f.conversation, original_id)
                .unwrap()
        )
        .unwrap(),
        original
    );
}

#[test]
fn unchanged_unknown_run_is_reused_without_restarting_processing() {
    let mut f = Fixture::new();
    import(
        &mut f.store,
        f.source,
        f.now,
        f.yesterday + 1000,
        "yesterday",
    );
    let (_, _, original_id) = f.request();
    assert!(
        f.store
            .begin_briefing_run(f.conversation, original_id)
            .unwrap()
    );
    f.store
        .stop_briefing_run(f.conversation, original_id, BriefingRunStop::Interrupted)
        .unwrap();
    let original = f
        .store
        .briefing_run_status(f.conversation, original_id)
        .unwrap()
        .run;
    assert_eq!(original.phase, BriefingRunPhase::Unknown);

    assert_eq!(f.request().2, original_id);
    assert!(
        !f.store
            .begin_briefing_run(f.conversation, original_id)
            .unwrap()
    );
    let after = f
        .store
        .briefing_run_status(f.conversation, original_id)
        .unwrap()
        .run;
    assert_eq!(
        serde_json::to_value(after).unwrap(),
        serde_json::to_value(original).unwrap()
    );
}

#[test]
fn unavailable_source_blocks_new_prepare_and_preserves_historical_manifest() {
    let mut f = Fixture::new();
    import(
        &mut f.store,
        f.source,
        f.now,
        f.yesterday + 1000,
        "yesterday",
    );
    let (_, _, original_id) = f.request();
    let original = f
        .store
        .briefing_manifest(f.conversation, original_id)
        .unwrap();
    f.store
        .archive_set_grant(
            f.source,
            SetArchiveGrant {
                expected_revision: 1,
                scope: None,
            },
            f.now + 1,
        )
        .unwrap();

    let (turn, action) = executing_request(&mut f.store, f.conversation);
    assert!(f.store.prepare_actor_briefing(turn, action).is_err());
    assert!(
        f.store
            .briefing_manifest(f.conversation, RequestId(action.0))
            .is_err()
    );
    assert_eq!(
        serde_json::to_value(
            f.store
                .briefing_start(f.conversation, original.request.clone(), f.now + DAY_MS)
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(original).unwrap(),
        "historical metadata is not new authority to process unavailable sources"
    );
}

#[test]
fn reused_preparation_stays_bound_on_retry_after_late_yesterday_import() {
    let mut f = Fixture::new();
    let first = import(&mut f.store, f.source, f.now, f.yesterday + 1000, "first");
    let (_, _, original_id) = f.request();
    let original_snapshot = serde_json::to_value(
        f.store
            .briefing_manifest(f.conversation, original_id)
            .unwrap(),
    )
    .unwrap();
    let (reusing_turn, reusing_action, reused_id) = f.request();
    assert_eq!(reused_id, original_id);
    assert_ne!(reused_id, RequestId(reusing_action.0));
    assert_eq!(
        f.store.actor_turn(reusing_turn).unwrap().actions[0].state,
        ActionState::Executing,
        "the repeated preparation is an unfinished action, not a new request"
    );

    let late = import(
        &mut f.store,
        f.source,
        f.now + 1,
        f.yesterday + 2000,
        "late-yesterday",
    );
    assert_eq!(
        f.store
            .prepare_actor_briefing(reusing_turn, reusing_action)
            .unwrap(),
        original_id,
        "retrying a preparation that reused an older manifest must keep its original binding"
    );
    let manifest_count: i64 = f
        .store
        .connection
        .query_row(
            "SELECT count(*) FROM briefing_manifests WHERE conversation_id=?1",
            [f.conversation.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        manifest_count, 1,
        "the retry must not create another manifest"
    );
    assert!(
        f.store
            .briefing_manifest(f.conversation, RequestId(reusing_action.0))
            .is_err()
    );

    let (_, fresh_action, fresh_id) = f.request();
    assert_eq!(fresh_id, RequestId(fresh_action.0));
    assert_ne!(fresh_id, original_id);
    assert_eq!(
        f.store
            .briefing_manifest(f.conversation, fresh_id)
            .unwrap()
            .records
            .iter()
            .map(|record| record.record_id)
            .collect::<Vec<_>>(),
        vec![first.id, late.id],
        "a genuinely fresh ordinary request must include the late import"
    );
    assert_eq!(
        serde_json::to_value(
            f.store
                .briefing_manifest(f.conversation, original_id)
                .unwrap()
        )
        .unwrap(),
        original_snapshot,
        "neither the retry nor a later fresh request may mutate the original snapshot"
    );
}

#[test]
fn an_empty_yesterday_snapshot_refreshes_when_the_first_late_record_arrives() {
    let mut f = Fixture::new();
    let (_, _, empty) = f.request();
    assert!(
        f.store
            .briefing_manifest(f.conversation, empty)
            .unwrap()
            .records
            .is_empty()
    );
    let late = import(
        &mut f.store,
        f.source,
        f.now,
        f.yesterday + 1000,
        "first-late",
    );
    let (_, _, fresh) = f.request();
    assert_ne!(
        fresh, empty,
        "an earlier empty result must not hide later received messages"
    );
    assert_eq!(
        f.store
            .briefing_manifest(f.conversation, fresh)
            .unwrap()
            .records[0]
            .record_id,
        late.id
    );
    assert!(
        f.store
            .briefing_manifest(f.conversation, empty)
            .unwrap()
            .records
            .is_empty()
    );
}

#[test]
fn preparation_receipt_and_new_manifest_roll_back_together_if_actor_save_fails() {
    let mut f = Fixture::new();
    import(&mut f.store, f.source, f.now, f.yesterday + 1000, "first");
    let (turn, action) = executing_request(&mut f.store, f.conversation);
    f.store.connection.execute_batch(
        "CREATE TRIGGER reject_preparation BEFORE UPDATE ON lain_actor_turns BEGIN SELECT RAISE(ABORT,'synthetic receipt failure'); END;"
    ).unwrap();
    assert!(f.store.prepare_actor_briefing(turn, action).is_err());
    assert!(
        f.store
            .briefing_manifest(f.conversation, RequestId(action.0))
            .is_err()
    );
    assert!(
        f.store.actor_turn(turn).unwrap().actions[0]
            .result
            .is_none()
    );
    f.store
        .connection
        .execute_batch("DROP TRIGGER reject_preparation")
        .unwrap();
    assert_eq!(
        f.store.prepare_actor_briefing(turn, action).unwrap(),
        RequestId(action.0)
    );
    let saved = f.store.actor_turn(turn).unwrap();
    assert_eq!(
        saved.actions[0].state,
        ActionState::Executing,
        "preparation is not execution completion"
    );
    saved.validate().unwrap();
    let receipt = saved.actions[0].result.as_ref().unwrap();
    assert_eq!(receipt["kind"], "briefing_prepared");
    assert!(
        !receipt.to_string().contains("synthetic notice"),
        "receipt must carry no source body"
    );
}

#[test]
fn a_receipt_copied_from_another_action_cannot_retarget_preparation() {
    let mut f = Fixture::new();
    import(&mut f.store, f.source, f.now, f.yesterday + 1000, "first");
    let (first, _, original) = f.request();
    let receipt = f.store.actor_turn(first).unwrap().actions[0].result.clone();
    let (other, action) = executing_request(&mut f.store, f.conversation);
    let mut copied = f.store.actor_turn(other).unwrap();
    copied.actions[0].result = receipt;
    f.store.save_actor_turn(&mut copied, false).unwrap();
    assert!(f.store.prepare_actor_briefing(other, action).is_err());
    assert!(
        f.store
            .briefing_manifest(f.conversation, RequestId(action.0))
            .is_err()
    );
    assert_eq!(
        f.store
            .briefing_manifest(f.conversation, original)
            .unwrap()
            .records
            .len(),
        1
    );
}

#[test]
fn new_records_beyond_an_already_reported_selection_limit_do_not_repeat_processing() {
    let mut f = Fixture::new();
    let mut scope = f
        .store
        .briefing_grant(f.conversation)
        .unwrap()
        .scope
        .unwrap();
    scope.max_records = 1;
    f.store
        .briefing_set(
            f.conversation,
            SetBriefingGrant {
                request_id: RequestId::new(),
                expected_revision: 1,
                scope: Some(scope),
            },
            f.now,
        )
        .unwrap();
    let first = import(&mut f.store, f.source, f.now, f.yesterday + 1000, "first");
    import(
        &mut f.store,
        f.source,
        f.now,
        f.yesterday + 2000,
        "already-omitted",
    );
    let (_, _, original) = f.request();
    let before = f.store.briefing_manifest(f.conversation, original).unwrap();
    assert!(before.record_limit_reached);
    assert_eq!(before.records[0].record_id, first.id);
    import(
        &mut f.store,
        f.source,
        f.now,
        f.yesterday + 3000,
        "also-outside-limit",
    );
    assert_eq!(
        f.request().2,
        original,
        "unchanged bounded selection with known omissions stays frozen"
    );
}

#[test]
fn coverage_changes_matter_but_observation_and_cursor_churn_do_not_change_inputs() {
    let mut f = Fixture::new();
    let (_, _, original) = f.request();
    let mut before = f.store.briefing_manifest(f.conversation, original).unwrap();
    before.sources[0].status.upstream_snapshot = Some(KnowledgeArchiveState {
        source_id: f.source,
        local_grant_revision: 1,
        reader_revision: "a".repeat(64),
        store_id: uuid::Uuid::new_v4().to_string(),
        remote_source_revision: 1,
        remote_source_enabled: true,
        through: 10,
        last_read: RequestId::new(),
        observed_at_ms: f.now,
        fresh_until_ms: f.now + 60000,
        coverage: Some(ArchiveCoverage {
            observed_at_ms: f.now,
            bounded_history: true,
            source_epochs_untracked: true,
            events: vec![],
        }),
    });
    let mut after = before.clone();
    after.created_at_ms += 1000;
    after.sources[0].through_sequence += 1;
    after.sources[0].status.observed_at_ms += 1000;
    after.sources[0].status.latest_received_at_ms = Some(f.now + 1000);
    let upstream = after.sources[0].status.upstream_snapshot.as_mut().unwrap();
    upstream.observed_at_ms += 1000;
    upstream.fresh_until_ms += 1000;
    upstream.through += 1;
    upstream.last_read = RequestId::new();
    upstream.coverage.as_mut().unwrap().observed_at_ms += 1000;
    assert!(
        super::briefing::same_inputs(&before, &after),
        "new observation/cursor alone must not cause new model work"
    );
    after.sources[0]
        .status
        .upstream_snapshot
        .as_mut()
        .unwrap()
        .coverage
        .as_mut()
        .unwrap()
        .events
        .push(ArchiveCoverageEvent {
            sequence: 11,
            kind: ArchiveCoverageKind::DisconnectedGap,
            recorded_at_ms: f.now + 1000,
        });
    assert!(
        !super::briefing::same_inputs(&before, &after),
        "new known gaps must not reuse stale coverage"
    );
}

#[test]
fn fresh_queries_use_corrected_versions_then_exclude_forgotten_originals() {
    let mut f = Fixture::new();
    let record = import(&mut f.store, f.source, f.now, f.yesterday + 1000, "notice");
    let (_, _, first) = f.request();
    let original = f.store.briefing_manifest(f.conversation, first).unwrap();
    let corrected = f
        .store
        .archive_correct(
            record.id,
            CorrectArchiveRecord {
                expected_revision: record.revision,
                text: "更正：地点改为教学楼。".into(),
            },
            f.now + 1,
        )
        .unwrap();
    assert!(
        f.store
            .briefing_page(
                f.conversation,
                first,
                0,
                f.now + 1,
                &crate::redaction::Redactor::new(vec![])
            )
            .is_err()
    );
    let (_, _, second) = f.request();
    assert_ne!(second, first);
    let revised = f.store.briefing_manifest(f.conversation, second).unwrap();
    assert_eq!(revised.records[0].record_id, record.id);
    assert_eq!(revised.records[0].revision, corrected.revision);
    assert_eq!(
        f.store
            .briefing_page(
                f.conversation,
                second,
                0,
                f.now + 1,
                &crate::redaction::Redactor::new(vec![])
            )
            .unwrap()
            .records[0]
            .text
            .as_deref(),
        Some("更正：地点改为教学楼。")
    );
    f.store
        .archive_forget(record.id, corrected.revision, f.now + 2)
        .unwrap();
    let (_, _, third) = f.request();
    assert_ne!(third, second);
    let withdrawn = f.store.briefing_manifest(f.conversation, third).unwrap();
    assert!(withdrawn.records.is_empty());
    assert_eq!(withdrawn.unavailable_records, 1);
    assert!(!withdrawn.complete_upstream_history);
    assert!(
        f.store
            .briefing_page(
                f.conversation,
                second,
                0,
                f.now + 2,
                &crate::redaction::Redactor::new(vec![])
            )
            .is_err()
    );
    assert_eq!(
        serde_json::to_value(f.store.briefing_manifest(f.conversation, first).unwrap()).unwrap(),
        serde_json::to_value(original).unwrap(),
        "original metadata remains immutable, without restoring old body access"
    );
}
