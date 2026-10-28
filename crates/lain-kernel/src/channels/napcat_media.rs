//! Receive-only NapCat visual normalization; locators remain transient/private.
use super::{
    domain::Input,
    napcat::{NapCatConfig, native_id},
};
use lain_wire::{
    MediaId,
    media::{InputSegment, VisualKind},
};
use serde_json::Value;
use std::{collections::BTreeMap, sync::LazyLock};
use zeroize::Zeroizing;

pub(super) struct MediaLocator {
    pub id: MediaId,
    pub position: u16,
    pub kind: VisualKind,
    pub file: Option<Zeroizing<String>>,
    pub url: Option<Zeroizing<String>>,
    pub flash: bool,
}
pub(super) struct ParsedPrivate {
    pub input: Input,
    pub media: Vec<MediaLocator>,
}
fn face_name(code: &str) -> Option<String> {
    // Factual ID/name mapping: installed NapCat 52230 qface-BZLvZXJ8.js, 2026-10-28.
    static FACES: LazyLock<BTreeMap<String, String>> = LazyLock::new(|| {
        serde_json::from_str(include_str!("qq_faces.json")).expect("validated face metadata")
    });
    FACES.get(code).cloned()
}

fn id(config: &NapCatConfig, message: &str, position: usize) -> MediaId {
    let name = serde_json::to_vec(&(
        &config.name,
        &config.self_id,
        &config.user_id,
        message,
        position,
    ))
    .expect("bounded media identity");
    MediaId(uuid::Uuid::new_v5(
        &uuid::Uuid::from_u128(0x3c1fe32a_989e_4f30_b3b6_d2e8271ef043),
        &name,
    ))
}
pub(super) fn parse_event(config: &NapCatConfig, event: &Value) -> Option<ParsedPrivate> {
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
        .is_some_and(|n| native_id(n).as_deref() != Some(config.user_id.as_str()))
    {
        return None;
    }
    let message = native_id(event.get("message_id")?)?;
    let parts = event.get("message")?.as_array()?;
    if parts.is_empty() || parts.len() > 64 {
        return None;
    }
    let mut text = String::new();
    let mut segments = Vec::new();
    let mut media = Vec::new();
    for (position, part) in parts.iter().enumerate() {
        let data = part.get("data")?;
        match part.get("type")?.as_str()? {
            "text" => {
                let part = data.get("text")?.as_str()?;
                text.push_str(part);
                segments.push(InputSegment::Text { text: part.into() });
            }
            "face" => {
                let code = native_id(data.get("id")?)?;
                if code.parse::<u32>().ok().is_none_or(|n| n > 10000) {
                    return None;
                }
                segments.push(InputSegment::Expression {
                    code: format!("qq-face:{code}"),
                    name: face_name(&code),
                });
            }
            "image" | "mface" => {
                if media.len() == 8 {
                    return None;
                }
                let sticker = part["type"] == "mface"
                    || data.get("emoji_id").is_some()
                    || data
                        .get("sub_type")
                        .and_then(|v| v.as_u64().or_else(|| v.as_str()?.parse().ok()))
                        .is_some_and(|n| n != 0);
                let kind = if sticker {
                    VisualKind::Sticker
                } else {
                    VisualKind::Picture
                };
                let media_id = id(config, &message, position);
                segments.push(InputSegment::Visual {
                    media_id,
                    visual_kind: kind,
                });
                let bounded = |value: &Value| {
                    value
                        .as_str()
                        .filter(|s| {
                            !s.is_empty() && s.len() <= 8192 && !s.chars().any(char::is_control)
                        })
                        .map(|s| Zeroizing::new(s.to_owned()))
                };
                let file = data.get("file").and_then(bounded);
                let url = data.get("url").and_then(bounded);
                media.push(MediaLocator {
                    id: media_id,
                    position: position.try_into().ok()?,
                    kind,
                    file,
                    url,
                    flash: data.get("type").and_then(Value::as_str) == Some("flash"),
                });
            }
            "reply" => segments.push(InputSegment::QuotedMessage {
                reference: native_id(data.get("id")?)?,
            }),
            _ => return None,
        }
        if text.len() > super::domain::MAX_TEXT {
            return None;
        }
    }
    if segments
        .iter()
        .all(|s| matches!(s, InputSegment::Text { .. }))
    {
        segments.clear();
    }
    let input = Input {
        account: config.self_id.clone(),
        peer: config.user_id.clone(),
        external_id: message,
        text,
        segments,
    };
    input.validate().ok()?;
    Some(ParsedPrivate { input, media })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn config() -> NapCatConfig {
        serde_json::from_value(json!({"name":"fixture","endpoint":"ws://127.0.0.1:3001","self_id":"1001","user_id":"2002","secret_ref":"secret://env/napcat","allow_reply":true})).unwrap()
    }
    fn event(parts: Value) -> Value {
        json!({"post_type":"message","message_type":"private","sub_type":"friend","self_id":1001,"user_id":2002,"message_id":123,"message":parts})
    }
    #[test]
    fn text_picture_sticker_face_and_quote_keep_order_without_locator_disclosure() {
        let c = config();
        let e = event(json!([
            {"type":"text","data":{"text":"看下"}},
            {"type":"image","data":{"file":"opaque-image.png","url":"https://multimedia.nt.qq.com.cn/image?rkey=transport-only-value"}},
            {"type":"face","data":{"id":"14"}},
            {"type":"text","data":{"text":"还有这个"}},
            {"type":"image","data":{"file":"sticker.gif","sub_type":1}},
            {"type":"reply","data":{"id":-321}}
        ]));
        let p = parse_event(&c, &e).unwrap();
        assert_eq!(p.input.text, "看下还有这个");
        assert_eq!(
            p.input.conversation_text(),
            "看下[图片 1][QQ表情：微笑]还有这个[表情包 2][引用消息，未读取引用正文]"
        );
        assert_eq!(p.media.len(), 2);
        assert_eq!(p.media[0].position, 1);
        assert_eq!(p.media[1].kind, VisualKind::Sticker);
        assert_eq!(
            p.media[0].file.as_deref().map(|s| s.as_str()),
            Some("opaque-image.png")
        );
        assert!(p.media[0].url.is_some());
        let stored = serde_json::to_string(&p.input).unwrap();
        for value in [
            "opaque-image.png",
            "multimedia.nt.qq.com.cn",
            "rkey",
            "transport-only-value",
            "sticker.gif",
        ] {
            assert!(!stored.contains(value));
        }
    }
    #[test]
    fn image_only_and_market_sticker_are_inputs_and_duplicate_ids_ignore_url_rotation() {
        let c = config();
        let mut e = event(
            json!([{"type":"image","data":{"file":"picture.png","url":"https://gchat.qpic.cn/one"}},{"type":"mface","data":{"url":"https://gxh.vip.qq.com/item/two.gif"}}]),
        );
        let p = parse_event(&c, &e).unwrap();
        assert!(p.input.text.is_empty());
        assert_eq!(p.media[1].kind, VisualKind::Sticker);
        e["message"][0]["data"]["url"] = json!("https://gchat.qpic.cn/rotated?key=value");
        let second = parse_event(&c, &e).unwrap();
        assert_eq!(p.input, second.input);
        assert_eq!(p.media[0].id, second.media[0].id);
    }
    #[test]
    fn faces_keep_known_names_and_unknown_codes_without_fabricating_pixels() {
        let p = parse_event(
            &config(),
            &event(json!([{"type":"face","data":{"id":0}},{"type":"face","data":{"id":9999}}])),
        )
        .unwrap();
        assert_eq!(
            p.input.conversation_text(),
            "[QQ表情：惊讶][QQ表情：qq-face:9999]"
        );
        assert!(p.media.is_empty());
    }
    #[test]
    fn identities_unsupported_content_and_visual_limits_still_fail_closed() {
        let c = config();
        let normal = event(json!([{"type":"image","data":{"file":"image.png"}}]));
        for field in ["self_id", "user_id"] {
            let mut e = normal.clone();
            e[field] = json!(3003);
            assert!(parse_event(&c, &e).is_none());
        }
        let mut e = normal.clone();
        e["group_id"] = json!(3003);
        assert!(parse_event(&c, &e).is_none());
        let mut e = normal.clone();
        e["sender"] = json!({"user_id":3003});
        assert!(parse_event(&c, &e).is_none());
        assert!(
            parse_event(
                &c,
                &event(json!([{"type":"video","data":{"url":"file:///private"}}]))
            )
            .is_none()
        );
        assert!(
            parse_event(
                &c,
                &event(Value::Array(
                    (0..9)
                        .map(|_| json!({"type":"image","data":{"file":"image.png"}}))
                        .collect()
                ))
            )
            .is_none()
        );
    }
    #[test]
    fn flash_and_missing_sources_remain_explicit_visual_jobs_not_invented_text() {
        let p = parse_event(
            &config(),
            &event(json!([{"type":"image","data":{"type":"flash"}}])),
        )
        .unwrap();
        assert!(p.media[0].flash && p.media[0].file.is_none() && p.media[0].url.is_none());
        assert!(p.input.text.is_empty());
    }
    #[test]
    fn legacy_plain_text_wire_is_unchanged_and_markers_are_not_user_quotes() {
        let p = parse_event(
            &config(),
            &event(json!([{"type":"text","data":{"text":"你好"}}])),
        )
        .unwrap();
        assert!(p.input.segments.is_empty());
        assert!(
            serde_json::to_value(&p.input)
                .unwrap()
                .get("segments")
                .is_none()
        );
        assert_eq!(p.input.text, p.input.conversation_text());
        let mut wrong = p.input;
        wrong.segments.push(InputSegment::Text {
            text: "another body".into(),
        });
        assert!(wrong.validate().is_err());
    }
}
