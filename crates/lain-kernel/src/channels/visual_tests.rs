use super::*;
use lain_wire::{MediaId, media::*};
fn picture() -> (Input, MediaId) {
    let id = MediaId::new();
    let mut i = input("visual", "look at this");
    i.segments = vec![
        InputSegment::Text {
            text: i.text.clone(),
        },
        InputSegment::Visual {
            media_id: id,
            visual_kind: VisualKind::Picture,
        },
    ];
    (i, id)
}
fn pixels() -> media_image::PreparedImage {
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(3, 2, image::Rgb([240, 10, 20])))
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    media_image::prepare(bytes.get_ref()).unwrap()
}
#[tokio::test]
async fn current_pixels_and_following_text_share_one_request_and_never_enter_actor_history() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.db"));
    let (s, m) = service(&k, 4);
    let channel = fixture_channel(true, SendOutcome::Confirmed("reply".into()));
    let task = tokio::spawn(s.clone().run(channel.clone()));
    let (i, id) = picture();
    s.receive(i.clone()).await.unwrap();
    s.receive(input("following", "and this is the question"))
        .await
        .unwrap();
    assert_eq!(m.calls.load(Ordering::SeqCst), 0);
    k.channel_media_finish(id, Ok(pixels())).unwrap();
    eventually(|| channel.sent.lock().unwrap().len() == 1).await;
    assert_eq!(m.calls.load(Ordering::SeqCst), 1);
    {
        let requests = m.requests.lock().unwrap();
        let r = &requests[0];
        assert_eq!(r.images.len(), 1);
        assert!(r.images[0].jpeg.starts_with(&[0xff, 0xd8]));
        let text = serde_json::to_string(&r.input).unwrap();
        assert!(text.contains("look at this") && text.contains("and this is the question"));
        assert!(!text.contains("data:image"));
    }
    let batch = k.channel_snapshot(s.id()).unwrap().batches.remove(0);
    let turn = k.actor_turn(batch.actor_turn_id.unwrap()).unwrap();
    assert_eq!(turn.private_chat.as_ref().unwrap().media_versions.len(), 1);
    assert!(!serde_json::to_string(&turn).unwrap().contains("base64"));
    // Subsequent turns receive recent pixels and the dependency of the delivered reply.
    s.receive(input("follow-up", "what about its color?"))
        .await
        .unwrap();
    eventually(|| m.calls.load(Ordering::SeqCst) == 2).await;
    assert_eq!(m.requests.lock().unwrap()[1].images.len(), 1);
    k.shutdown().unwrap();
    task.await.unwrap().unwrap();
}
#[tokio::test]
async fn withdrawing_a_picture_during_inference_discards_the_visual_answer_and_history() {
    let dir = tempfile::tempdir().unwrap();
    let k = kernel(&dir.path().join("state.db"));
    let (s, m) = service(&k, 0);
    let channel = fixture_channel(true, SendOutcome::Confirmed("reply".into()));
    let task = tokio::spawn(s.clone().run(channel.clone()));
    let (i, id) = picture();
    s.receive(i).await.unwrap();
    k.channel_media_finish(id, Ok(pixels())).unwrap();
    eventually(|| m.calls.load(Ordering::SeqCst) == 1).await;
    k.channel_media_withdraw(s.id(), "visual").unwrap();
    m.gate.add_permits(1);
    eventually(|| !channel.sent.lock().unwrap().is_empty()).await;
    assert!(
        channel
            .sent
            .lock()
            .unwrap()
            .iter()
            .all(|d| !d.text.contains("刚才的话我听到了"))
    );
    s.receive(input("after-withdrawal", "what can you still see?"))
        .await
        .unwrap();
    eventually(|| m.calls.load(Ordering::SeqCst) == 2).await;
    {
        let r = m.requests.lock().unwrap();
        assert!(r[1].images.is_empty());
        assert!(
            !serde_json::to_string(&r[1].input)
                .unwrap()
                .contains("刚才的话我听到了")
        );
    }
    k.shutdown().unwrap();
    m.gate.add_permits(1);
    task.await.unwrap().unwrap();
}
