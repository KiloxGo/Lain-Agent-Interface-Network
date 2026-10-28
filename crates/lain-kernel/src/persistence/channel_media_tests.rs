use super::*;
use crate::channels::{domain::*, media_image};
use lain_wire::{MediaId, media::*};

fn fixture() -> (tempfile::TempDir, Store, Binding, Input, MediaId) {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    let b = s
        .channel_bind(
            BindingSpec {
                name: "visual-fixture".into(),
                adapter: "fixture".into(),
                account: "1001".into(),
                peer: "2002".into(),
            },
            true,
        )
        .unwrap();
    let media = MediaId::new();
    let input = Input {
        account: "1001".into(),
        peer: "2002".into(),
        external_id: "1".into(),
        text: "看看这张图".into(),
        segments: vec![
            InputSegment::Text {
                text: "看看这张图".into(),
            },
            InputSegment::Visual {
                media_id: media,
                visual_kind: VisualKind::Picture,
            },
        ],
    };
    (dir, s, b, input, media)
}
fn prepared() -> media_image::PreparedImage {
    let mut raw = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
        12,
        8,
        image::Rgb([200, 20, 30]),
    ))
    .write_to(&mut raw, image::ImageFormat::Png)
    .unwrap();
    media_image::prepare(raw.get_ref()).unwrap()
}
#[test]
fn receipt_precedes_preparation_without_busy_polling_or_pixel_data_in_input_history() {
    let (_dir, mut s, b, input, media) = fixture();
    let incoming = s.channel_receive(b.id, input).unwrap();
    assert_eq!(s.channel_media(media).unwrap().state, MediaState::Pending);
    assert!(
        s.channel_next_batch(b.id, incoming.received_at_ms + MERGE_MS)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        s.channel_deadline(b.id, incoming.received_at_ms + MERGE_MS)
            .unwrap(),
        Some(incoming.received_at_ms + channel_media::PREPARE_MS)
    );
    let normalized = prepared();
    let bytes = normalized.frames[0].bytes.clone();
    let ready = s.channel_media_finish(media, Ok(normalized)).unwrap();
    assert_eq!(ready.state, MediaState::Ready);
    assert_eq!(ready.samples.len(), 1);
    assert_eq!(s.channel_snapshot(b.id).unwrap().media, vec![ready]);
    let batch = s
        .channel_next_batch(b.id, incoming.received_at_ms + MERGE_MS)
        .unwrap()
        .unwrap();
    assert_eq!(batch.text, "看看这张图[图片 1]");
    assert_eq!(
        s.channel_snapshot(b.id).unwrap().incoming[0].input.text,
        "看看这张图"
    );
    let stored: String = s
        .connection
        .query_row("SELECT snapshot FROM channel_incoming", [], |r| r.get(0))
        .unwrap();
    assert!(
        !stored.contains("base64")
            && !stored.contains("data:image")
            && !stored.contains("image_url")
    );
    let pixels: Vec<u8> = s
        .connection
        .query_row(
            "SELECT body FROM channel_media_frames WHERE media_id=?1",
            [media.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(pixels, bytes);
}
#[test]
fn duplicate_and_restart_do_not_repeat_pending_reads() {
    let (dir, mut s, b, input, media) = fixture();
    let first = s.channel_receive(b.id, input.clone()).unwrap();
    let same = s.channel_receive(b.id, input).unwrap();
    assert_eq!(first.id, same.id);
    assert_eq!(
        s.connection
            .query_row("SELECT count(*) FROM channel_media", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    drop(s);
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    s.recover_channels().unwrap();
    let interrupted = s.channel_media(media).unwrap();
    assert_eq!(interrupted.state, MediaState::Interrupted);
    assert_eq!(interrupted.failure, Some(MediaFailure::Disconnected));
    assert!(s.channel_media_finish(media, Ok(prepared())).is_err());
    assert!(
        s.channel_next_batch(b.id, first.received_at_ms + MERGE_MS)
            .unwrap()
            .is_some()
    );
}
#[test]
fn ready_pixels_survive_restart_with_original_expiry_and_withdrawal_wins() {
    let (dir, mut s, b, input, media) = fixture();
    let received = s.channel_receive(b.id, input).unwrap();
    s.channel_media_finish(media, Ok(prepared())).unwrap();
    let saved = s.channel_media(media).unwrap();
    drop(s);
    let mut s = Store::open(&dir.path().join("state.db")).unwrap();
    s.recover_channels().unwrap();
    assert_eq!(s.channel_media(media).unwrap(), saved);
    assert_eq!(
        s.connection
            .query_row("SELECT count(*) FROM channel_media_frames", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    s.channel_media_withdraw(b.id, "unrelated").unwrap();
    assert_eq!(s.channel_media(media).unwrap().state, MediaState::Ready);
    s.channel_media_withdraw(b.id, "1").unwrap();
    assert_eq!(s.channel_media(media).unwrap().state, MediaState::Withdrawn);
    assert_eq!(
        s.connection
            .query_row("SELECT count(*) FROM channel_media_frames", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    s.channel_next_batch(b.id, received.received_at_ms + REPLY_TTL_MS)
        .unwrap();
    assert_eq!(s.channel_media(media).unwrap().state, MediaState::Withdrawn);
}
#[test]
fn revoke_and_expiry_remove_frames_and_reject_late_results() {
    for revoke in [false, true] {
        let (_dir, mut s, b, input, media) = fixture();
        let received = s.channel_receive(b.id, input).unwrap();
        s.channel_media_finish(media, Ok(prepared())).unwrap();
        if revoke {
            s.channel_grant(b.id, false, 1).unwrap();
            assert_eq!(s.channel_media(media).unwrap().state, MediaState::Withdrawn);
        } else {
            s.channel_next_batch(b.id, received.received_at_ms + REPLY_TTL_MS)
                .unwrap();
            assert_eq!(s.channel_media(media).unwrap().state, MediaState::Expired);
        }
        assert_eq!(
            s.connection
                .query_row("SELECT count(*) FROM channel_media_frames", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert!(s.channel_media_finish(media, Ok(prepared())).is_err());
    }
}
#[test]
fn timeout_and_safe_http_failure_unblock_the_original_input_without_fabricated_pixels() {
    for timeout in [false, true] {
        let (_dir, mut s, b, input, media) = fixture();
        let incoming = s.channel_receive(b.id, input).unwrap();
        if timeout {
            s.channel_next_batch(b.id, incoming.received_at_ms + channel_media::PREPARE_MS)
                .unwrap();
        } else {
            s.channel_media_finish(media, Err(MediaFailure::HttpStatus { code: 404 }))
                .unwrap();
        }
        let m = s.channel_media(media).unwrap();
        assert_eq!(m.state, MediaState::Unavailable);
        assert!(m.samples.is_empty());
        assert_eq!(
            m.failure,
            Some(if timeout {
                MediaFailure::Timeout
            } else {
                MediaFailure::HttpStatus { code: 404 }
            })
        );
        let batch = s
            .channel_next_batch(b.id, incoming.received_at_ms + channel_media::PREPARE_MS)
            .unwrap()
            .unwrap();
        assert_eq!(batch.text, "看看这张图[图片 1]");
    }
}
#[test]
fn media_metadata_and_frame_bodies_commit_atomically() {
    let (_dir, mut s, b, input, media) = fixture();
    s.channel_receive(b.id, input).unwrap();
    s.connection.execute_batch("CREATE TRIGGER reject_ready BEFORE UPDATE ON channel_media WHEN NEW.state='ready' BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    assert!(s.channel_media_finish(media, Ok(prepared())).is_err());
    assert_eq!(s.channel_media(media).unwrap().state, MediaState::Pending);
    assert_eq!(
        s.connection
            .query_row("SELECT count(*) FROM channel_media_frames", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    s.connection
        .execute_batch("DROP TRIGGER reject_ready")
        .unwrap();
    assert_eq!(
        s.channel_media_finish(media, Ok(prepared())).unwrap().state,
        MediaState::Ready
    );
}
#[test]
fn schema23_upgrade_preserves_original_text_input_bytes_and_adds_no_image() {
    let (dir, mut s, b, mut input, _) = fixture();
    input.segments.clear();
    s.channel_receive(b.id, input).unwrap();
    let before: String = s
        .connection
        .query_row("SELECT snapshot FROM channel_incoming", [], |r| r.get(0))
        .unwrap();
    s.connection
        .execute_batch(
            "DROP TABLE channel_media_frames; DROP TABLE channel_media; PRAGMA user_version=23;",
        )
        .unwrap();
    drop(s);
    let s = Store::open(&dir.path().join("state.db")).unwrap();
    assert_eq!(
        s.connection
            .query_row("SELECT snapshot FROM channel_incoming", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        before
    );
    assert_eq!(
        s.connection
            .query_row("SELECT count(*) FROM channel_media", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert!(
        s.channel_snapshot(b.id).unwrap().incoming[0]
            .input
            .segments
            .is_empty()
    );
}
#[test]
fn visual_expiry_withholds_assistant_and_tool_bodies_without_rewriting_original_user_text() {
    use crate::actor::{domain::*, tools::Tool};
    let (_dir, mut s, b, input, media) = fixture();
    let received = s.channel_receive(b.id, input).unwrap();
    s.channel_media_finish(media, Ok(prepared())).unwrap();
    let batch = s
        .channel_next_batch(b.id, received.received_at_ms + MAX_MERGE_MS)
        .unwrap()
        .unwrap();
    let mut t = ActorTurn {
        id: ActorTurnId::new(),
        reading_activity: None,
        sharing_activity: None,
        conversation_id: b.conversation_id,
        request_id: batch.id.0,
        sequence: 0,
        revision: 0,
        state: TurnState::Running,
        messages: vec![],
        actions: vec![],
        error: None,
        model_requests: 1,
        usage_tokens: 1,
        timing: Default::default(),
        channel_memory: None,
        channel_source: None,
        private_chat: Some(s.private_chat_context(b.id, batch.id).unwrap()),
    };
    t.message(MessageRole::User, batch.text).unwrap();
    s.save_actor_turn(&mut t, true).unwrap();
    s.channel_link(batch.id, &t).unwrap();
    let visual = s.private_visual_context(t.id, true).unwrap();
    assert_eq!(visual.images.len(), 1);
    let mut t = s.actor_turn(t.id).unwrap();
    assert!(s.private_chat_memory_current(t.id).unwrap());
    t.message(
        MessageRole::Assistant,
        "private visual interpretation".into(),
    )
    .unwrap();
    t.actions.push(Action {
        id: lain_wire::ActionId::new(),
        call_id: "fixture".into(),
        tool: Tool::MemoryRecall(crate::memory::RecallMemory {
            text: "private visual interpretation".into(),
        }),
        state: ActionState::Executed,
        result: Some(serde_json::json!({"text":"private visual interpretation"})),
    });
    t.state = TurnState::Completed;
    t.private_chat.as_mut().unwrap().decision = lain_wire::chat::ChatDecision::Reply;
    s.save_actor_turn(&mut t, false).unwrap();
    s.channel_finish(batch.id, &t).unwrap();
    let d = s.channel_claim_delivery(b.id).unwrap().unwrap();
    s.channel_settle(d.id, SendOutcome::Confirmed("reply".into()))
        .unwrap();
    // A new turn's history can use a fresh interpretation, with inherited media dependencies.
    let next = s
        .channel_receive(
            b.id,
            Input {
                segments: vec![],
                account: "1001".into(),
                peer: "2002".into(),
                external_id: "next".into(),
                text: "continue".into(),
            },
        )
        .unwrap();
    let batch = s
        .channel_next_batch(b.id, next.received_at_ms + MAX_MERGE_MS)
        .unwrap()
        .unwrap();
    let mut current = t.clone();
    current.id = ActorTurnId::new();
    current.request_id = batch.id.0;
    current.sequence = 0;
    current.revision = 0;
    current.state = TurnState::Running;
    current.messages.clear();
    current.actions.clear();
    current.private_chat = Some(s.private_chat_context(b.id, batch.id).unwrap());
    current
        .message(MessageRole::User, "continue".into())
        .unwrap();
    s.save_actor_turn(&mut current, true).unwrap();
    s.channel_link(batch.id, &current).unwrap();
    assert!(
        s.private_chat_history(current.id)
            .unwrap()
            .iter()
            .find(|p| p.id == t.id)
            .unwrap()
            .messages
            .iter()
            .any(|m| m.role == MessageRole::Assistant)
    );
    s.private_visual_context(current.id, true).unwrap();
    assert_eq!(
        s.actor_turn(current.id)
            .unwrap()
            .private_chat
            .unwrap()
            .media_versions,
        t.private_chat.unwrap().media_versions
    );
    s.connection.execute("UPDATE channel_media SET expires_at_ms=0,snapshot=json_set(snapshot,'$.expires_at_ms',0)",[]).unwrap();
    assert!(!s.private_chat_memory_current(current.id).unwrap());
    let history = s.private_chat_history(current.id).unwrap();
    let old = history.iter().find(|p| p.id == t.id).unwrap();
    assert!(old.actions.is_empty());
    assert!(old.messages.iter().all(|m| m.role == MessageRole::User));
    assert!(old.messages[0].content.contains("[图片 1]"));
    assert!(
        !serde_json::to_string(old)
            .unwrap()
            .contains("private visual interpretation")
    );
}
