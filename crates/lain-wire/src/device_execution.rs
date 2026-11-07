//! Normalized device observations and explicit human decisions. No provider objects.
use crate::{DeviceExecutionId, RequestId, WorkSessionId, WorkerApprovalId, WorkerTurnId};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionPhase {
    Preparing,
    Running,
    WaitingForApproval,
    Completed,
    Cancelled,
    Failed,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceApprovalChoice {
    ApproveOnce,
    Deny,
    Cancel,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceApproval {
    pub id: WorkerApprovalId,
    pub kind: String,
    pub description: String,
    pub truncated: bool,
    pub choices: Vec<DeviceApprovalChoice>,
    pub fingerprint: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceWorkResult {
    /// Bounded original Worker output, not an LLM-generated evaluation of success.
    pub output_tail: String,
    pub command_failures: u32,
    /// Reported references only; no byte upload, verified hash or hidden file read.
    pub reported_paths: Vec<String>,
    pub paths_withheld: u32,
    pub history_truncated: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionEvidence {
    pub execution_id: DeviceExecutionId,
    pub session_id: Option<WorkSessionId>,
    pub turn_id: Option<WorkerTurnId>,
    pub phase: ExecutionPhase,
    pub pending_approvals: Vec<DeviceApproval>,
    /// Cumulative device observations, separate from immutable owner decisions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub approval_reports: Vec<DeviceApprovalReport>,
    pub result: Option<DeviceWorkResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceApprovalStage {
    Received,
    Applied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceApprovalReport {
    pub approval_id: WorkerApprovalId,
    pub decision: DecideDeviceApproval,
    pub stage: DeviceApprovalStage,
}
impl ExecutionEvidence {
    pub fn valid(&self) -> bool {
        !self.execution_id.0.is_nil()
            && self.session_id.is_none_or(|id| !id.0.is_nil())
            && self.turn_id.is_none_or(|id| !id.0.is_nil())
            && (!matches!(
                self.phase,
                ExecutionPhase::Running
                    | ExecutionPhase::WaitingForApproval
                    | ExecutionPhase::Completed
            ) || self.turn_id.is_some())
            && (self.phase != ExecutionPhase::Preparing || self.turn_id.is_none())
            && (self.phase != ExecutionPhase::Completed || self.result.is_some())
            && self.pending_approvals.len() <= 1
            && self.approval_reports.len() <= 256
            && (self.approval_reports.is_empty() || self.turn_id.is_some())
            && self.approval_reports.iter().enumerate().all(|(i, r)| {
                !r.approval_id.0.is_nil()
                    && !r.decision.request_id.0.is_nil()
                    && r.decision.fingerprint.len() == 64
                    && r.decision
                        .fingerprint
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit())
                    && !self.approval_reports[..i].iter().any(|old| {
                        old.approval_id == r.approval_id
                            || old.decision.request_id == r.decision.request_id
                    })
                    && !(r.stage == DeviceApprovalStage::Applied
                        && self.pending_approvals.iter().any(|a| a.id == r.approval_id))
            })
            && (self.turn_id.is_none() || self.session_id.is_some())
            && self.pending_approvals.iter().all(|a| {
                !a.id.0.is_nil()
                    && !a.kind.is_empty()
                    && a.kind.len() <= 64
                    && a.kind
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
                    && !a.description.trim().is_empty()
                    && a.description.len() <= 2048
                    && a.description
                        .chars()
                        .all(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
                    && !a.choices.is_empty()
                    && a.choices.len() <= 3
                    && !a
                        .choices
                        .iter()
                        .enumerate()
                        .any(|(i, c)| a.choices[..i].contains(c))
                    && (!a.truncated || !a.choices.contains(&DeviceApprovalChoice::ApproveOnce))
                    && a.fingerprint
                        == approval_fingerprint(self.execution_id, self.session_id, self.turn_id, a)
            })
            && (self.pending_approvals.is_empty()
                == (self.phase != ExecutionPhase::WaitingForApproval))
            && (self.result.is_none()
                || matches!(
                    self.phase,
                    ExecutionPhase::Completed
                        | ExecutionPhase::Failed
                        | ExecutionPhase::Cancelled
                        | ExecutionPhase::Unknown
                ))
            && self.result.as_ref().is_none_or(|r| {
                r.output_tail.len() <= 2048
                    && r.output_tail
                        .chars()
                        .all(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
                    && r.reported_paths.len() <= 8
                    && r.reported_paths.iter().all(|p| {
                        !p.is_empty()
                            && p.len() <= 512
                            && !p.chars().any(char::is_control)
                            && !p.starts_with('/')
                            && !p.contains('\\')
                            && !p.contains(':')
                            && p.split('/').all(|s| !s.is_empty() && s != ".." && s != ".")
                    })
            })
    }
}
pub fn approval_fingerprint(
    execution_id: DeviceExecutionId,
    session: Option<WorkSessionId>,
    turn: Option<WorkerTurnId>,
    a: &DeviceApproval,
) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"lain-device-approval-v1\0");
    h.update(
        serde_json::to_vec(&(
            execution_id,
            session,
            turn,
            a.id,
            &a.kind,
            &a.description,
            a.truncated,
            &a.choices,
        ))
        .expect("typed approval"),
    );
    format!("{:x}", h.finalize())
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecideDeviceApproval {
    pub request_id: RequestId,
    pub fingerprint: String,
    pub choice: DeviceApprovalChoice,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceApprovalDecision {
    pub approval_id: WorkerApprovalId,
    pub decision: DecideDeviceApproval,
    pub received_at_ms: u64,
}

/// Explicit protected local configuration; cloud commands cannot supply it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceHostPolicy {
    pub version: u8,
    pub codex_path: String,
    pub workspace_write: bool,
    pub offline_grace_seconds: u32,
}
impl DeviceHostPolicy {
    pub fn valid(&self) -> bool {
        self.version == 1 && !self.codex_path.is_empty() && self.offline_grace_seconds <= 60
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> ExecutionEvidence {
        let mut e = ExecutionEvidence {
            execution_id: DeviceExecutionId(uuid::Uuid::from_u128(1)),
            session_id: Some(WorkSessionId(uuid::Uuid::from_u128(2))),
            turn_id: Some(WorkerTurnId(uuid::Uuid::from_u128(3))),
            phase: ExecutionPhase::WaitingForApproval,
            pending_approvals: vec![DeviceApproval {
                id: WorkerApprovalId(uuid::Uuid::from_u128(4)),
                kind: "command_execution".into(),
                description: "核对 fixture".into(),
                truncated: false,
                choices: vec![
                    DeviceApprovalChoice::ApproveOnce,
                    DeviceApprovalChoice::Deny,
                ],
                fingerprint: String::new(),
            }],
            approval_reports: vec![],
            result: None,
        };
        e.pending_approvals[0].fingerprint = approval_fingerprint(
            e.execution_id,
            e.session_id,
            e.turn_id,
            &e.pending_approvals[0],
        );
        e
    }
    #[test]
    fn approval_fingerprint_has_a_stable_cross_language_encoding_and_binds_scope() {
        let e = fixture();
        assert!(e.valid());
        assert_eq!(
            e.pending_approvals[0].fingerprint,
            "7b5569e680e798d7c1f4a5e283f9672be2aa57153417dbedf3800d09e082194a"
        );
        for field in [
            "execution",
            "session",
            "turn",
            "id",
            "kind",
            "description",
            "truncated",
            "choices",
        ] {
            let mut changed = e.clone();
            match field {
                "execution" => changed.execution_id = DeviceExecutionId(uuid::Uuid::from_u128(10)),
                "session" => changed.session_id = Some(WorkSessionId(uuid::Uuid::from_u128(10))),
                "turn" => changed.turn_id = Some(WorkerTurnId(uuid::Uuid::from_u128(10))),
                "id" => {
                    changed.pending_approvals[0].id = WorkerApprovalId(uuid::Uuid::from_u128(10))
                }
                "kind" => changed.pending_approvals[0].kind = "file_change".into(),
                "description" => changed.pending_approvals[0].description.push('!'),
                "truncated" => changed.pending_approvals[0].truncated = true,
                _ => changed.pending_approvals[0].choices.reverse(),
            }
            assert!(!changed.valid(), "{field}");
        }
        let mut raw = serde_json::to_value(e).unwrap();
        raw["pending_approvals"][0]["provider_request"] = serde_json::json!({"approve":true});
        assert!(serde_json::from_value::<ExecutionEvidence>(raw).is_err());
    }
    #[test]
    fn permission_reports_are_bounded_and_legacy_evidence_stays_verbatim() {
        let mut e = fixture();
        let legacy = serde_json::to_value(&e).unwrap();
        assert!(legacy.get("approval_reports").is_none());
        assert_eq!(
            serde_json::from_value::<ExecutionEvidence>(legacy.clone()).unwrap(),
            e
        );
        let report = DeviceApprovalReport {
            approval_id: e.pending_approvals[0].id,
            decision: DecideDeviceApproval {
                request_id: RequestId::new(),
                fingerprint: e.pending_approvals[0].fingerprint.clone(),
                choice: DeviceApprovalChoice::ApproveOnce,
            },
            stage: DeviceApprovalStage::Received,
        };
        e.approval_reports = vec![report.clone()];
        assert!(e.valid());
        e.approval_reports[0].stage = DeviceApprovalStage::Applied;
        assert!(
            !e.valid(),
            "the same request cannot still be pending and already applied"
        );
        e.approval_reports = vec![report.clone(), report.clone()];
        assert!(!e.valid());
        e.pending_approvals.clear();
        e.phase = ExecutionPhase::Running;
        e.approval_reports = (0..256)
            .map(|_| DeviceApprovalReport {
                approval_id: WorkerApprovalId::new(),
                decision: DecideDeviceApproval {
                    request_id: RequestId::new(),
                    ..report.decision.clone()
                },
                ..report.clone()
            })
            .collect();
        assert!(e.valid());
        assert!(
            serde_json::to_vec(&e).unwrap().len() < 120 * 1024,
            "maximum evidence must fit the protected 128 KiB record bound"
        );
        e.approval_reports.push(report);
        assert!(!e.valid());
    }
    #[test]
    fn result_paths_are_only_bounded_relative_references() {
        let mut e = fixture();
        e.phase = ExecutionPhase::Completed;
        e.pending_approvals.clear();
        e.result = Some(DeviceWorkResult {
            output_tail: "done\nwith details".into(),
            command_failures: 0,
            reported_paths: vec!["reports/result.tex".into()],
            paths_withheld: 0,
            history_truncated: false,
        });
        assert!(e.valid());
        for path in [
            "../outside",
            "/absolute",
            "C:/private",
            "a/../b",
            "a\\b",
            "a//b",
            ".",
            "\u{1b}[31m",
        ] {
            let mut bad = e.clone();
            bad.result.as_mut().unwrap().reported_paths = vec![path.into()];
            assert!(!bad.valid(), "{path}");
        }
        let mut bad = e.clone();
        bad.result.as_mut().unwrap().output_tail = "中".repeat(683);
        assert!(!bad.valid());
        let mut bad = e.clone();
        bad.result.as_mut().unwrap().reported_paths = vec!["a".into(); 9];
        assert!(!bad.valid());
        let mut bad = e;
        bad.turn_id = None;
        assert!(!bad.valid());
    }
}
