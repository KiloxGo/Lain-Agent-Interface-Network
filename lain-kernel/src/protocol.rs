use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    domain::event::{EventEnvelope, PROTOCOL_VERSION},
    storage::{RunRecord, SessionSnapshot, StorageScope},
};

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type")]
pub enum ClientCommand {
    #[serde(rename = "client.hello")]
    Hello { versions: Vec<u16> },
    #[serde(rename = "session.subscribe")]
    SessionSubscribe {
        session_id: String,
        #[serde(default)]
        after_seq: i64,
    },
    #[serde(rename = "session.unsubscribe")]
    SessionUnsubscribe { session_id: String },
    #[serde(rename = "message.create")]
    MessageCreate {
        session_id: String,
        client_message_id: String,
        text: String,
    },
    #[serde(rename = "run.cancel")]
    RunCancel { run_id: String },
    #[serde(rename = "tool.approval.respond")]
    ToolApprovalRespond { approval_id: String, approved: bool },
    #[serde(rename = "client.ping")]
    Ping { nonce: Option<String> },
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ServerHello<'a> {
    pub v: u16,
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub connection_id: &'a str,
    pub selected_version: u16,
    pub heartbeat_interval_ms: u64,
    pub scope: &'a StorageScope,
}

impl<'a> ServerHello<'a> {
    pub fn new(connection_id: &'a str, scope: &'a StorageScope) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            event_type: "server.hello",
            connection_id,
            selected_version: PROTOCOL_VERSION,
            heartbeat_interval_ms: 20_000,
            scope,
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SnapshotEvent<'a> {
    pub v: u16,
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub snapshot: &'a SessionSnapshot,
}

impl<'a> SnapshotEvent<'a> {
    pub fn new(snapshot: &'a SessionSnapshot) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            event_type: "session.snapshot",
            snapshot,
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct PongEvent<'a> {
    pub v: u16,
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub nonce: Option<&'a str>,
}

impl<'a> PongEvent<'a> {
    pub fn new(nonce: Option<&'a str>) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            event_type: "server.pong",
            nonce,
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ErrorEvent<'a> {
    pub v: u16,
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub code: &'a str,
    pub recoverable: bool,
}

impl<'a> ErrorEvent<'a> {
    pub fn new(code: &'a str, recoverable: bool) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            event_type: "error",
            code,
            recoverable,
        }
    }
}

pub fn transient_event(
    scope: &StorageScope,
    session_id: &str,
    run: &RunRecord,
    event_type: &str,
    payload: Value,
) -> EventEnvelope {
    EventEnvelope {
        v: PROTOCOL_VERSION,
        event_id: Uuid::now_v7().to_string(),
        instance_id: scope.instance_id.clone(),
        session_id: Some(session_id.to_string()),
        run_id: Some(run.id.clone()),
        seq: None,
        event_type: event_type.to_string(),
        ts: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        payload,
    }
}
