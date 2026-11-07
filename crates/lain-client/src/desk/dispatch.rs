//! Inspection of actual proposed task text, never a task rewriting pipeline.
use super::*;
use lain_wire::{ActorAction, ToolCall};

pub(super) fn review_text(turn: &ActorSnapshot, action: &ActorAction) -> Result<String> {
    if matches!(action.tool, ToolCall::MemorySelectionChange(_)) {
        return crate::memory::selection_review(turn, action);
    }
    if matches!(action.tool, ToolCall::BriefingRemember(_)) {
        return crate::memory::briefing_review(turn, action);
    }
    let payload = serde_json::to_string_pretty(&action.tool)?;
    let task = match &action.tool {
        ToolCall::DeviceQueue(args) => &args.task,
        ToolCall::DeviceSend(args) => &args.text,
        ToolCall::WorkCreate { task, .. } => task,
        ToolCall::WorkSend { text, .. } => text,
        _ => return Ok(safe_text(&payload)),
    };
    let originals: Vec<_> = turn
        .messages
        .iter()
        .filter(|m| m.role == MessageRole::User)
        .collect();
    ensure!(
        !originals.is_empty(),
        "Dispatch has no original user record to inspect"
    );
    ensure!(
        originals
            .iter()
            .all(|m| m.conversation_id == turn.conversation_id),
        "Original request belongs to another conversation"
    );
    let unchanged = originals.len() == 1 && originals[0].content == *task;
    let status = if unchanged { "Unchanged" } else { "Changed" };
    let mut text = format!(
        "Outbound task: {status}\nComparison: exact UTF-8 text; no automatic prompt compiler.\n"
    );
    if originals.len() != 1 {
        text.push_str(
            "Multiple original records remain separate; outbound is not one verbatim record.\n",
        );
    }
    if matches!(
        action.tool,
        ToolCall::DeviceQueue(_) | ToolCall::DeviceSend(_)
    ) {
        text.push_str(&format!("Dispatch RequestId: {}\n", action.id));
    }
    text.push_str(
        "\nOriginal user records (JSON strings preserve whitespace/control characters):\n",
    );
    for msg in &originals {
        text.push_str(&format!(
            "{}: {}\n",
            msg.id,
            serde_json::to_string(&msg.content)?
        ));
    }
    text.push_str(&format!(
        "\nActual outbound task (JSON string):\n{}\n",
        serde_json::to_string(task)?
    ));
    if !unchanged {
        // Full replacement diff, not a quadratic/minimal diff of untrusted large input.
        // Keep each source attributable and encode controls before terminal rendering.
        text.push_str("\nReplacement diff (- original record; + outbound task):\n");
        for msg in originals {
            text.push_str(&format!(
                "- {} {}\n",
                msg.id,
                serde_json::to_string(&msg.content)?
            ));
        }
        text.push_str(&format!("+ {}\n", serde_json::to_string(task)?));
    }
    text.push_str(&format!("\nComplete proposed tool payload:\n{payload}"));
    Ok(safe_text(&text))
}
