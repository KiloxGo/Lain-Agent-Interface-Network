//! One output coordinator for transient previews and authoritative Actor records.
use crate::tui::safe_text;
use anyhow::Result;
use lain_wire::{
    ActionId, ActionState, ActorSnapshot, ActorTurnState, MessageId, MessageRole, PreviewEvent,
    PreviewId, PreviewUpdate,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    time::Instant,
};

struct Preview {
    id: PreviewId,
    message: MessageId,
    sequence: u64,
    text: String,
    open: bool,
    finished: bool,
}
pub struct Renderer<W: Write> {
    output: W,
    turn: Option<lain_wire::ActorTurnId>,
    messages: BTreeSet<MessageId>,
    actions: BTreeMap<ActionId, ActionState>,
    reviewed_memories: BTreeSet<ActionId>,
    retired: BTreeSet<PreviewId>,
    current: Option<Preview>,
    error: Option<String>,
    pub first_visible: Option<Instant>,
}
impl Renderer<Vec<u8>> {
    pub(crate) fn drain_text(&mut self) -> String {
        String::from_utf8(std::mem::take(&mut self.output)).expect("renderer writes UTF-8")
    }
}
impl<W: Write> Renderer<W> {
    pub fn new(output: W) -> Self {
        Self {
            output,
            turn: None,
            messages: BTreeSet::new(),
            actions: BTreeMap::new(),
            reviewed_memories: BTreeSet::new(),
            retired: BTreeSet::new(),
            current: None,
            error: None,
            first_visible: None,
        }
    }
    fn text(&mut self, text: &str, start: bool) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        if start {
            write!(self.output, "Lain> ")?;
        }
        write!(self.output, "{}", safe_text(text).replace('\n', "\n      "))?;
        self.output.flush()?;
        self.first_visible.get_or_insert_with(Instant::now);
        Ok(())
    }
    fn close(&mut self, preview: &mut Preview) -> Result<()> {
        if preview.open {
            writeln!(self.output)?;
            preview.open = false;
        }
        Ok(())
    }
    pub fn interrupted(&mut self, reason: &str) -> Result<()> {
        if let Some(mut preview) = self.current.take() {
            self.close(&mut preview)?;
            if !preview.text.is_empty() {
                writeln!(
                    self.output,
                    "Preview> {}; completed message not confirmed.",
                    safe_text(reason)
                )?;
            }
            self.retired.insert(preview.id);
        }
        self.output.flush()?;
        Ok(())
    }
    fn begin_turn(&mut self, id: lain_wire::ActorTurnId) -> Result<()> {
        if self.turn != Some(id) {
            self.interrupted("next Actor turn")?;
            self.turn = Some(id);
            self.messages.clear();
            self.actions.clear();
            self.reviewed_memories.clear();
            self.retired.clear();
            self.error = None;
            self.first_visible = None;
        }
        Ok(())
    }
    pub fn snapshot(&mut self, turn: &ActorSnapshot) -> Result<()> {
        self.begin_turn(turn.id)?;
        for receipt in &turn.reviewed_memory_receipts {
            if self.reviewed_memories.insert(receipt.action_id) {
                self.interrupted("committed memory receipt")?;
                writeln!(
                    self.output,
                    "System> 已保存来源事项 · 仅当前会话 · 历史操作回执\n      Memory: {} · recorded_at_ms: {}\n      回执不表示正文仍可用，也不授权再次保存。",
                    receipt.memory_id, receipt.recorded_at_ms
                )?;
            }
        }
        for r in &turn.memory_change_receipts {
            if self.reviewed_memories.insert(r.action_id) {
                let label = if r.operation == lain_wire::memory::MemoryOperation::Correct {
                    "已更正保存事项"
                } else {
                    "已忘记来源事项"
                };
                self.interrupted("committed memory change")?;
                writeln!(
                    self.output,
                    "System> {} · 仅当前会话\n      Memory: {} · recorded_at_ms: {}\n      历史操作回执；不会恢复或自动重放旧正文。",
                    label, r.memory_id, r.recorded_at_ms
                )?;
            }
        }
        for message in &turn.messages {
            if message.role != MessageRole::Assistant || !self.messages.insert(message.id) {
                continue;
            }
            // The Kernel's durable receipt body remains unchanged in JSON and
            // history. Human mode presents it once through the action below.
            if turn.actions.iter().any(|a| {
                let prefix = match a.state {
                    ActionState::Executed => "Kernel action accepted: ",
                    ActionState::Unknown => {
                        "Kernel action did not complete reliably; do not replay automatically: "
                    }
                    _ => return false,
                };
                serde_json::to_string(&a.result)
                    .is_ok_and(|result| message.content == format!("{prefix}{result}"))
            }) {
                continue;
            }
            if self
                .current
                .as_ref()
                .is_some_and(|p| p.message == message.id)
            {
                let mut preview = self.current.take().unwrap();
                if let Some(tail) = message.content.strip_prefix(&preview.text) {
                    // A checkpoint/final snapshot can arrive before all live deltas.
                    self.text(tail, !preview.open)?;
                    if preview.open || !tail.is_empty() {
                        writeln!(self.output)?;
                    }
                } else {
                    self.close(&mut preview)?;
                    writeln!(self.output, "Preview> superseded by the completed message.")?;
                    self.text(&message.content, true)?;
                    writeln!(self.output)?;
                }
                self.retired.insert(preview.id);
            } else {
                self.interrupted("superseded")?;
                self.text(&message.content, true)?;
                writeln!(self.output)?;
            }
        }
        for action in &turn.actions {
            if self.actions.insert(action.id, action.state) != Some(action.state) {
                self.interrupted("tool state changed")?;
                self.action(turn, action)?;
            }
        }
        if let Some(error) = &turn.error
            && self.error.as_ref() != Some(error)
        {
            self.interrupted("generation ended")?;
            let display = if error
                == "Inference or tool execution did not finish reliably; inspect recorded actions before retrying. No mutation was replayed."
            {
                "本轮模型请求或工具执行未可靠完成。请先查看已有操作回执，再决定是否重试；没有自动重放操作。"
            } else {
                error
            };
            writeln!(self.output, "System> {}", safe_text(display))?;
            self.error = Some(error.clone());
        }
        if turn.state != ActorTurnState::Running {
            self.interrupted("generation ended")?;
        }
        self.output.flush()?;
        Ok(())
    }
    fn action(&mut self, turn: &ActorSnapshot, action: &lain_wire::ActorAction) -> Result<()> {
        let tool = serde_json::to_value(&action.tool)?;
        let name = tool["name"].as_str().unwrap_or("tool");
        let state = match action.state {
            ActionState::Proposed => "awaiting confirmation",
            ActionState::Executing => "approved; submitting operation",
            ActionState::Executed => "operation accepted",
            ActionState::Rejected => "declined; no operation executed",
            ActionState::Failed if missing_memory_read(action) => {
                "not found or not visible (state: failed)"
            }
            ActionState::Failed => "operation failed",
            ActionState::Unknown => "outcome unknown; inspect before retrying",
        };
        writeln!(self.output, "Tool> {name} · {state}")?;
        if action.state == ActionState::Proposed {
            if matches!(action.tool, lain_wire::ToolCall::MemorySelectionChange(_)) {
                writeln!(
                    self.output,
                    "{}",
                    crate::memory::selection_review(turn, action)?
                )?;
            }
            if matches!(action.tool, lain_wire::ToolCall::BriefingRemember(_)) {
                writeln!(
                    self.output,
                    "{}",
                    crate::memory::briefing_review(turn, action)?
                )?;
            }
            writeln!(
                self.output,
                "      Action: {}\n      Exact arguments: {}",
                action.id,
                safe_text(&serde_json::to_string_pretty(&tool["arguments"])?)
            )?;
        } else if let Some(result) = &action.result {
            let outcome = result.get("outcome").unwrap_or(result);
            let partial = result.get("partial");
            if let Some(id) = outcome
                .get("work_session_id")
                .or_else(|| partial.and_then(|v| v.get("work_session_id")))
                .and_then(|v| v.as_str())
            {
                writeln!(self.output, "      WorkSession: {}", safe_text(id))?;
            }
            if let Some(id) = outcome
                .get("turn_id")
                .or_else(|| outcome.get("id"))
                .and_then(|v| v.as_str())
            {
                writeln!(
                    self.output,
                    "      {}: {}",
                    if name == "memory.propose" {
                        "MemoryCandidate"
                    } else {
                        "Turn"
                    },
                    safe_text(id)
                )?;
            }
            if let Some(id) = outcome.get("memory_id").and_then(|v| v.as_str()) {
                writeln!(self.output, "      Saved Memory: {}", safe_text(id))?;
            }
            if name == "work.create" && action.state == ActionState::Executed {
                writeln!(
                    self.output,
                    "      Task submitted; worker results are delivered separately."
                )?;
            } else if name == "work.send" && action.state == ActionState::Executed {
                writeln!(
                    self.output,
                    "      Follow-up submitted; worker results are delivered separately."
                )?;
            } else if let Some(error) = outcome.get("error").and_then(|v| v.as_str()) {
                writeln!(self.output, "      {}", safe_text(error))?;
            } else if let Some(entries) = outcome
                .get("sessions")
                .or_else(|| outcome.get("profiles"))
                .or_else(|| outcome.get("events"))
                .and_then(|v| v.as_array())
            {
                writeln!(
                    self.output,
                    "      {} entries; receipt stored in Kernel history.",
                    entries.len()
                )?;
            } else {
                if let Some(records) = outcome
                    .get("page")
                    .and_then(|p| p.get("records"))
                    .and_then(|r| r.as_array())
                {
                    writeln!(
                        self.output,
                        "      本次查询匹配 {} 条；结果仅覆盖指定筛选与分页范围。",
                        records.len()
                    )?;
                }
                writeln!(self.output, "      Receipt stored in Kernel history.")?;
            }
        }
        Ok(())
    }
    pub fn notice(&mut self, notice: &lain_wire::notices::ConversationNotice) -> Result<()> {
        use lain_wire::notices::{NoticeOutcome, WorkerOutcome};
        self.suspend_preview()?;
        let state = match notice.outcome {
            NoticeOutcome::Worker {
                state: WorkerOutcome::Completed,
            } => "turn completed",
            NoticeOutcome::Worker {
                state: WorkerOutcome::Failed,
            } => "turn failed",
            NoticeOutcome::Worker {
                state: WorkerOutcome::Cancelled,
            } => "turn cancelled",
            NoticeOutcome::Worker {
                state: WorkerOutcome::Unknown,
            } => "state unknown; outcome is unverified",
            NoticeOutcome::SubmissionUnknown => "submission unknown; inspect before retrying",
        };
        writeln!(
            self.output,
            "Worker> {state}\n        WorkSession: {}",
            notice.work_session_id
        )?;
        if let Some(id) = notice.worker_turn_id {
            writeln!(self.output, "        Turn: {id}")?;
        }
        if let Some(text) = &notice.evidence.final_assistant_message {
            writeln!(
                self.output,
                "        Worker report:\n        {}",
                safe_text(text).replace('\n', "\n        ")
            )?;
        } else {
            writeln!(
                self.output,
                "        No final assistant report was available in the correlated history."
            )?;
        }
        if notice.evidence.failed_command_count != 0 {
            writeln!(
                self.output,
                "        {} command(s) returned nonzero status; completion does not mean all checks passed.",
                notice.evidence.failed_command_count
            )?;
            for command in &notice.evidence.failed_commands {
                writeln!(
                    self.output,
                    "        exit {:?}: {}",
                    command.exit_code,
                    safe_text(&command.command)
                )?;
            }
        }
        for error in &notice.evidence.errors {
            writeln!(
                self.output,
                "        {}: {}",
                safe_text(&error.kind),
                safe_text(&error.message)
            )?;
        }
        for path in &notice.evidence.changed_files {
            writeln!(self.output, "        Changed: {}", safe_text(path))?;
        }
        if notice.evidence.truncated {
            writeln!(self.output, "        [result excerpt truncated]")?;
        }
        writeln!(
            self.output,
            "        Full history: lain work history {}\n        Notice: {}",
            notice.work_session_id, notice.id
        )?;
        self.output.flush()?;
        Ok(())
    }
    pub fn notice_offline(&mut self) -> Result<()> {
        self.suspend_preview()?;
        writeln!(
            self.output,
            "Notice> Connection interrupted; reconnecting. Results remain stored in the Kernel."
        )?;
        self.output.flush()?;
        Ok(())
    }
    fn suspend_preview(&mut self) -> Result<()> {
        // A worker notice does not invalidate an unrelated Actor preview. Close
        // its current line, retaining identity/text for subsequent live deltas
        // and final-message reconciliation.
        if let Some(mut preview) = self.current.take() {
            self.close(&mut preview)?;
            self.current = Some(preview);
        }
        Ok(())
    }
    pub(crate) fn preview_checkpoint(&self) -> Option<(PreviewId, MessageId, u64)> {
        self.current.as_ref().map(|p| (p.id, p.message, p.sequence))
    }
    pub fn preview(&mut self, event: PreviewEvent, turn: &ActorSnapshot) -> Result<()> {
        self.begin_turn(turn.id)?;
        if turn.id != event.turn_id
            || turn.state != ActorTurnState::Running
            || self.messages.contains(&event.message_id)
            || self.retired.contains(&event.preview_id)
        {
            return Ok(());
        }
        if let PreviewUpdate::Started { text } = event.update {
            if text.len() > 16384 {
                return Ok(());
            }
            if self
                .current
                .as_ref()
                .is_some_and(|p| p.id == event.preview_id)
            {
                let mut p = self.current.take().unwrap();
                if p.message == event.message_id && event.sequence >= p.sequence && !p.finished {
                    if let Some(tail) = text.strip_prefix(&p.text) {
                        self.text(tail, !p.open)?;
                        p.open |= !tail.is_empty();
                        p.text = text;
                        p.sequence = event.sequence;
                    } else {
                        self.current = Some(p);
                        self.interrupted("inconsistent checkpoint")?;
                        return Ok(());
                    }
                }
                self.current = Some(p);
            } else {
                self.interrupted("new generation")?;
                self.text(&text, true)?;
                self.current = Some(Preview {
                    id: event.preview_id,
                    message: event.message_id,
                    sequence: event.sequence,
                    open: !text.is_empty(),
                    text,
                    finished: false,
                });
            }
            return Ok(());
        }
        let Some(mut p) = self.current.take() else {
            return Ok(());
        };
        if p.id != event.preview_id
            || p.message != event.message_id
            || event.sequence <= p.sequence
            || p.finished
        {
            self.current = Some(p);
            return Ok(());
        }
        if event.sequence != p.sequence + 1 {
            self.current = Some(p);
            self.interrupted("sequence gap; awaiting authoritative snapshot")?;
            return Ok(());
        }
        p.sequence = event.sequence;
        match event.update {
            PreviewUpdate::Delta { text } if p.text.len().saturating_add(text.len()) <= 16384 => {
                self.text(&text, !p.open)?;
                p.open |= !text.is_empty();
                p.text.push_str(&text);
            }
            PreviewUpdate::Finished => {
                self.close(&mut p)?;
                p.finished = true;
            }
            _ => {
                self.current = Some(p);
                self.interrupted("discarded")?;
                return Ok(());
            }
        }
        self.current = Some(p);
        Ok(())
    }
}

// Presentation only: NotFound does not establish deletion or its cause. Keep the
// authoritative Failed state and original result intact, including diagnostics.
pub(crate) fn missing_memory_read(action: &lain_wire::ActorAction) -> bool {
    action.state == ActionState::Failed
        && matches!(action.tool, lain_wire::ToolCall::MemoryInspect { .. })
        && action.result.as_ref().is_some_and(|result| {
            result.get("success") == Some(&serde_json::Value::Bool(false))
                && result.get("error").and_then(serde_json::Value::as_str) == Some("not_found")
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lain_wire::*;
    #[test]
    fn reviewed_memory_receipt_remains_readable_once_without_source_text_or_action_ack() {
        let mut turn = running();
        turn.availability = Some(ActorAvailability::SourceUnavailable);
        turn.error = Some("Source unavailable; previous content withheld.".into());
        let action = ActionId::new();
        turn.reviewed_memory_receipts
            .push(memory::ReviewedMemoryReceipt {
                action_id: action,
                actor_turn_id: turn.id,
                conversation_id: turn.conversation_id,
                candidate_id: CandidateId(action.0),
                memory_id: MemoryId::new(),
                recorded_at_ms: 1234567,
            });
        turn.validate().unwrap();
        let mut renderer = Renderer::new(Vec::new());
        renderer.snapshot(&turn).unwrap();
        renderer.snapshot(&turn).unwrap();
        let text = String::from_utf8(renderer.output).unwrap();
        assert_eq!(text.matches("已保存来源事项").count(), 1);
        assert!(text.contains(&turn.reviewed_memory_receipts[0].memory_id.to_string()));
        turn.reviewed_memory_receipts[0].conversation_id = ConversationId::new();
        assert!(turn.validate().is_err());
    }
    #[test]
    fn missing_memory_lookup_does_not_claim_failure_recovery_or_forgetting() {
        let mut turn = running();
        turn.actions.push(ActorAction {
            memory_review: None,
            memory_change_review: None,
            id: ActionId::new(),
            call_id: "lookup".into(),
            tool: ToolCall::MemoryInspect {
                memory_id: MemoryId::new(),
                scope: memory::MemoryScope::Global,
            },
            state: ActionState::Failed,
            result: Some(serde_json::json!({"success":false,"error":"not_found"})),
        });
        let render = |turn: &ActorSnapshot| {
            let mut renderer = Renderer::new(Vec::new());
            renderer.snapshot(turn).unwrap();
            String::from_utf8(renderer.output).unwrap()
        };
        let text = render(&turn);
        assert!(text.contains("not found or not visible (state: failed)"));
        assert!(text.contains("not_found") && !text.contains("operation accepted"));
        assert!(!text.contains("forgotten"));

        for result in [
            serde_json::json!({"success":false,"error":"forbidden"}),
            serde_json::json!({"success":false,"error":"transport"}),
            serde_json::json!({"success":true,"error":"not_found"}),
            serde_json::json!({"error":"not_found"}),
            serde_json::json!({"outcome":{"success":false,"error":"not_found"}}),
        ] {
            turn.actions[0].result = Some(result);
            assert!(render(&turn).contains("operation failed"));
        }
        turn.actions[0].result = Some(serde_json::json!({"success":false,"error":"not_found"}));
        turn.actions[0].tool = ToolCall::WorkCancel {
            session_id: WorkSessionId::new(),
        };
        assert!(render(&turn).contains("operation failed"));
        turn.actions[0].tool = ToolCall::WorkInspect {
            session_id: WorkSessionId::new(),
        };
        assert!(render(&turn).contains("operation failed"));
        turn.actions[0].tool = ToolCall::MemoryInspect {
            memory_id: MemoryId::new(),
            scope: memory::MemoryScope::Global,
        };
        turn.actions[0].state = ActionState::Unknown;
        assert!(render(&turn).contains("outcome unknown; inspect before retrying"));
    }

    #[test]
    fn runtime_failure_is_system_output_not_lain_speech() {
        let mut turn = running();
        turn.state = ActorTurnState::Unknown;
        turn.error = Some("Inference or tool execution did not finish reliably; inspect recorded actions before retrying. No mutation was replayed.".into());
        let mut renderer = Renderer::new(Vec::new());
        renderer.snapshot(&turn).unwrap();
        let text = String::from_utf8(renderer.output).unwrap();
        assert!(text.contains("System>"));
        assert!(text.contains("回执"));
        assert!(!text.contains("Lain>") && !text.contains("Inference or tool execution"));
    }
    fn running() -> ActorSnapshot {
        ActorSnapshot {
            availability: None,
            effect_receipts: vec![],
            reviewed_memory_receipts: vec![],
            memory_change_receipts: vec![],
            reading_activity: None,
            briefing_activity: None,
            sharing_activity: None,
            id: ActorTurnId::new(),
            conversation_id: ConversationId::new(),
            request_id: RequestId::new(),
            sequence: 1,
            revision: 1,
            state: ActorTurnState::Running,
            messages: vec![],
            actions: vec![],
            error: None,
            model_requests: 1,
            usage_tokens: 0,
            timing: ActorTiming::default(),
            channel_memory: None,
            channel_source: None,
            private_chat: None,
        }
    }
    #[test]
    fn notice_interleaves_without_retiring_an_unrelated_actor_preview() {
        use lain_wire::notices::{
            ConversationNotice, NoticeOutcome, ResultEvidence, WorkerOutcome,
        };
        let mut turn = running();
        let preview = PreviewId::new();
        let message = MessageId::new();
        let mut r = Renderer::new(Vec::new());
        r.snapshot(&turn).unwrap();
        r.preview(
            event(
                &turn,
                preview,
                message,
                0,
                PreviewUpdate::Started {
                    text: "开头".into(),
                },
            ),
            &turn,
        )
        .unwrap();
        r.notice(&ConversationNotice {
            id: NoticeId::new(),
            cursor: 1,
            conversation_id: turn.conversation_id,
            actor_turn_id: ActorTurnId::new(),
            action_id: ActionId::new(),
            work_session_id: WorkSessionId::new(),
            worker_turn_id: Some(WorkerTurnId::new()),
            timestamp_ms: 1,
            source_history_order: Some(2),
            outcome: NoticeOutcome::Worker {
                state: WorkerOutcome::Completed,
            },
            evidence: ResultEvidence {
                final_assistant_message: Some("独立 worker 报告".into()),
                ..Default::default()
            },
            acknowledged: false,
        })
        .unwrap();
        r.preview(
            event(
                &turn,
                preview,
                message,
                1,
                PreviewUpdate::Delta {
                    text: "结尾".into(),
                },
            ),
            &turn,
        )
        .unwrap();
        turn.state = ActorTurnState::Completed;
        turn.messages.push(ActorMessage {
            id: message,
            conversation_id: turn.conversation_id,
            role: MessageRole::Assistant,
            content: "开头结尾".into(),
            timestamp_ms: 1,
            status: MessageStatus::Completed,
        });
        r.snapshot(&turn).unwrap();
        let text = String::from_utf8(r.output).unwrap();
        assert_eq!(text.matches("开头").count(), 1);
        assert_eq!(text.matches("结尾").count(), 1);
        assert!(text.contains("独立 worker 报告"));
        assert!(!text.contains("not confirmed"));
    }
    #[test]
    fn turn_scoped_rendering_resets_latency_errors_and_dedup_memory() {
        let mut r = Renderer::new(Vec::new());
        for _ in 0..2 {
            let mut turn = running();
            turn.state = ActorTurnState::Failed;
            turn.error = Some("same failure".into());
            r.snapshot(&turn).unwrap();
            r.snapshot(&turn).unwrap();
            assert!(r.first_visible.is_none());
        }
        assert_eq!(
            String::from_utf8(r.output)
                .unwrap()
                .matches("same failure")
                .count(),
            2
        );
    }
    #[test]
    fn human_receipts_keep_exact_proposal_and_do_not_repeat_durable_json_body() {
        let mut turn = running();
        turn.state = ActorTurnState::PendingConfirmation;
        let task = "只读检查；不要修改文件。保留原样。";
        let sid = WorkSessionId::new();
        turn.actions.push(ActorAction {
            memory_review: None,
            memory_change_review: None,
            id: ActionId::new(),
            call_id: "fixture".into(),
            tool: ToolCall::WorkCreate {
                workspace: "/directory with spaces/project".into(),
                profile_id: "codex/local".into(),
                sandbox: Sandbox::ReadOnly,
                task: task.into(),
            },
            state: ActionState::Proposed,
            result: None,
        });
        let mut renderer = Renderer::new(Vec::new());
        renderer.snapshot(&turn).unwrap();
        turn.actions[0].state = ActionState::Executing;
        turn.state = ActorTurnState::Running;
        renderer.snapshot(&turn).unwrap();
        turn.actions[0].state = ActionState::Executed;
        turn.actions[0].result = Some(
            serde_json::json!({"outcome":{"work_session_id":sid,"turn_id":WorkerTurnId::new(),"stage":"submitted","worker_completion":"not_observed"},"partial":null}),
        );
        turn.state = ActorTurnState::Completed;
        turn.messages.push(ActorMessage {
            id: MessageId::new(),
            conversation_id: turn.conversation_id,
            role: MessageRole::Assistant,
            content: format!(
                "Kernel action accepted: {}",
                serde_json::to_string(&turn.actions[0].result).unwrap()
            ),
            timestamp_ms: 1,
            status: MessageStatus::Completed,
        });
        let before = serde_json::to_value(&turn).unwrap();
        renderer.snapshot(&turn).unwrap();
        renderer.snapshot(&turn).unwrap();
        let text = String::from_utf8(renderer.output).unwrap();
        assert!(text.contains(task));
        assert!(text.contains("/directory with spaces/project"));
        assert_eq!(text.matches("Exact arguments:").count(), 1);
        assert_eq!(text.matches("operation accepted").count(), 1);
        assert!(text.contains(&sid.to_string()));
        assert!(!text.contains("worker_completion"));
        assert_eq!(serde_json::to_value(turn).unwrap(), before);
    }
    fn event(
        turn: &ActorSnapshot,
        preview: PreviewId,
        message: MessageId,
        sequence: u64,
        update: PreviewUpdate,
    ) -> PreviewEvent {
        PreviewEvent {
            turn_id: turn.id,
            preview_id: preview,
            message_id: message,
            sequence,
            update,
        }
    }
    #[test]
    fn unicode_controls_checkpoint_overlap_and_final_are_rendered_once() {
        let mut turn = running();
        let preview = PreviewId::new();
        let message = MessageId::new();
        let mut r = Renderer::new(Vec::new());
        r.preview(
            event(
                &turn,
                preview,
                message,
                0,
                PreviewUpdate::Started {
                    text: String::new(),
                },
            ),
            &turn,
        )
        .unwrap();
        r.preview(
            event(
                &turn,
                preview,
                message,
                1,
                PreviewUpdate::Delta {
                    text: "你好\n".into(),
                },
            ),
            &turn,
        )
        .unwrap();
        r.preview(
            event(
                &turn,
                preview,
                message,
                1,
                PreviewUpdate::Delta {
                    text: "duplicate".into(),
                },
            ),
            &turn,
        )
        .unwrap();
        r.preview(
            event(
                &turn,
                preview,
                message,
                2,
                PreviewUpdate::Started {
                    text: "你好\n世界\u{1b}".into(),
                },
            ),
            &turn,
        )
        .unwrap();
        r.preview(
            event(&turn, preview, message, 3, PreviewUpdate::Finished),
            &turn,
        )
        .unwrap();
        turn.state = ActorTurnState::Completed;
        turn.messages.push(ActorMessage {
            id: message,
            conversation_id: turn.conversation_id,
            role: MessageRole::Assistant,
            content: "你好\n世界\u{1b}".into(),
            timestamp_ms: 0,
            status: MessageStatus::Completed,
        });
        r.snapshot(&turn).unwrap();
        r.snapshot(&turn).unwrap();
        r.preview(
            event(
                &turn,
                PreviewId::new(),
                MessageId::new(),
                0,
                PreviewUpdate::Started {
                    text: "late".into(),
                },
            ),
            &turn,
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(r.output).unwrap(),
            "Lain> 你好\n      世界\\u{1b}\n"
        );
    }
    #[test]
    fn wrong_generation_gap_and_failure_never_become_completed_prose() {
        let mut turn = running();
        let preview = PreviewId::new();
        let message = MessageId::new();
        let mut r = Renderer::new(Vec::new());
        let other = running();
        r.preview(
            event(
                &other,
                preview,
                message,
                0,
                PreviewUpdate::Started {
                    text: "wrong".into(),
                },
            ),
            &turn,
        )
        .unwrap();
        r.preview(
            event(
                &turn,
                preview,
                message,
                0,
                PreviewUpdate::Started {
                    text: "partial".into(),
                },
            ),
            &turn,
        )
        .unwrap();
        r.preview(
            event(
                &turn,
                preview,
                message,
                2,
                PreviewUpdate::Delta { text: "gap".into() },
            ),
            &turn,
        )
        .unwrap();
        r.preview(
            event(
                &turn,
                preview,
                message,
                1,
                PreviewUpdate::Delta {
                    text: "stale".into(),
                },
            ),
            &turn,
        )
        .unwrap();
        turn.state = ActorTurnState::Unknown;
        r.snapshot(&turn).unwrap();
        let text = String::from_utf8(r.output).unwrap();
        assert!(text.contains("completed message not confirmed"));
        assert!(
            !text.contains("wrong")
                && !text.contains("Lain> partialgap")
                && !text.contains("stale")
        );
        assert!(r.messages.is_empty());
    }
}
