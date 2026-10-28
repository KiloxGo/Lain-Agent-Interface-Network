//! Normalized current-conversation visuals. No CDN URLs, device paths or pixels.
use crate::{ChannelId, IncomingId, MediaId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualKind {
    Picture,
    Sticker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputSegment {
    Text {
        text: String,
    },
    Visual {
        media_id: MediaId,
        visual_kind: VisualKind,
    },
    Expression {
        code: String,
        name: Option<String>,
    },
    /// Reference metadata only; no automatic history lookup or quoted body.
    QuotedMessage {
        reference: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaState {
    Pending,
    Ready,
    Unavailable,
    Interrupted,
    Expired,
    Withdrawn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MediaFailure {
    ImagesDisabled,
    ModelImagesUnavailable,
    SourceUnavailable,
    UnsafeLocation,
    TooLarge,
    InvalidImage,
    UnsupportedFormat,
    FlashNotRead,
    Capacity,
    Timeout,
    Disconnected,
    HttpStatus { code: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaFrame {
    pub frame_index: u32,
    pub at_ms: u64,
    pub width: u32,
    pub height: u32,
    pub sha256: String,
    pub encoded_bytes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationMedia {
    pub id: MediaId,
    pub channel_id: ChannelId,
    pub incoming_id: IncomingId,
    pub grant_revision: u64,
    pub segment_position: u16,
    pub visual_kind: VisualKind,
    pub state: MediaState,
    pub failure: Option<MediaFailure>,
    pub received_at_ms: u64,
    pub prepare_until_ms: u64,
    pub expires_at_ms: u64,
    pub original_width: Option<u32>,
    pub original_height: Option<u32>,
    pub decoded_frames: u32,
    pub animation_truncated: bool,
    pub samples: Vec<MediaFrame>,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaVersion {
    pub media_id: MediaId,
    pub revision: u64,
}
