//! Ordinary proactive contact, separate from replies, work delivery and source rights.
use crate::{ActorTurnId, ChannelId, DeliveryId, ExperienceId, ShareId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuietWindow {
    pub from_minute: u16,
    pub to_minute: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShareScope {
    pub urls: Vec<String>,
    pub expires_at_ms: u64,
    pub utc_offset_minutes: i16,
    pub quiet_windows: Vec<QuietWindow>,
    pub cooldown_ms: u64,
    pub max_per_day: u16,
    pub max_send_attempts: u16,
    pub max_model_requests: u16,
    pub candidate_ttl_ms: u64,
    pub source_max_age_ms: u64,
    pub user_quiet_ms: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareGrant {
    pub channel_id: ChannelId,
    pub revision: u64,
    pub scope: Option<ShareScope>,
    pub actor_watermark: u64,
    pub changed_at_ms: u64,
}
impl ShareGrant {
    pub fn active(&self, now: u64) -> bool {
        self.revision > 0 && self.scope.as_ref().is_some_and(|s| now < s.expires_at_ms)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetShareGrant {
    pub expected_revision: u64,
    pub scope: Option<ShareScope>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attention {
    pub channel_id: ChannelId,
    pub revision: u64,
    pub do_not_disturb: bool,
    pub until_ms: Option<u64>,
    pub changed_at_ms: u64,
}
impl Attention {
    pub fn quiet(&self, now: u64) -> bool {
        self.revision == 0 || (self.do_not_disturb && self.until_ms.is_none_or(|t| now < t))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetAttention {
    pub expected_revision: u64,
    pub do_not_disturb: bool,
    pub until_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShareState {
    Prepared,
    Drafting,
    Ready,
    Skipped,
    Failed,
    Sending,
    Confirmed,
    NotSent,
    Rejected,
    Unknown,
    Expired,
    Revoked,
    Superseded,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareCandidate {
    pub id: ShareId,
    pub channel_id: ChannelId,
    pub experience_id: ExperienceId,
    pub experience_revision: u64,
    pub experience_fingerprint: String,
    pub grant_revision: u64,
    pub state: ShareState,
    pub actor_turn_id: Option<ActorTurnId>,
    pub input_watermark: u64,
    pub created_at_ms: u64,
    pub expires_at_ms: u64,
    pub reason: Option<String>,
    pub text: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShareChoice {
    Share,
    Skip,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShareDecision {
    pub decision: ShareChoice,
    pub reason: String,
    pub text: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShareDelivery {
    pub id: DeliveryId,
    pub candidate_id: ShareId,
    pub channel_id: ChannelId,
    pub grant_revision: u64,
    pub attention_revision: u64,
    pub input_watermark: u64,
    pub text: String,
    pub state: crate::source::DeliveryState,
    pub created_at_ms: u64,
    pub expires_at_ms: u64,
    pub receipt: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingSnapshot {
    pub model_requests_reserved: u64,
    pub send_attempts_reserved: u64,
    pub grant: ShareGrant,
    pub attention: Attention,
    pub candidates: Vec<ShareCandidate>,
    pub deliveries: Vec<ShareDelivery>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShareQuery {
    pub from_ms: u64,
    pub to_ms: u64,
    pub text: Option<String>,
    pub limit: u16,
}
impl ShareQuery {
    pub fn valid(&self) -> bool {
        self.from_ms < self.to_ms
            && self.to_ms <= i64::MAX as u64
            && self.to_ms - self.from_ms <= 31 * 86400000
            && (1..=3).contains(&self.limit)
            && self.text.as_ref().is_none_or(|s| {
                !s.trim().is_empty() && s.len() <= 256 && !s.chars().any(char::is_control)
            })
    }
}
/// Exact receipt and optional source version disclosed in a private model request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareVersion {
    pub delivery_id: DeliveryId,
    pub fingerprint: String,
    pub experience: Option<crate::reading::ExperienceVersion>,
}
