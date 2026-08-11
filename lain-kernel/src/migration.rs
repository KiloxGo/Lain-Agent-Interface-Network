use std::path::Path;

use serde::Serialize;

use crate::storage::{SessionSnapshot, Storage, StorageError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyExchange {
    pub source: String,
    pub index: usize,
    pub heading: String,
    pub user: String,
    pub assistant: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportReport {
    pub parsed: usize,
    pub imported: usize,
    pub already_present: usize,
}

pub fn parse_legacy_markdown(source: &str, content: &str) -> Vec<LegacyExchange> {
    content
        .split("\n---")
        .enumerate()
        .filter_map(|(index, block)| parse_exchange(source, index, block))
        .collect()
}

pub async fn read_legacy_directory(path: &Path) -> std::io::Result<Vec<LegacyExchange>> {
    let mut entries = tokio::fs::read_dir(path).await?;
    let mut files = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if entry.file_type().await?.is_file()
            && path.extension().and_then(|value| value.to_str()) == Some("md")
        {
            files.push(path);
        }
    }
    files.sort();
    let mut exchanges = Vec::new();
    for path in files {
        let content = tokio::fs::read_to_string(&path).await?;
        let source = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("unknown.md");
        exchanges.extend(parse_legacy_markdown(source, &content));
    }
    Ok(exchanges)
}

pub async fn import_exchanges(
    storage: &Storage,
    exchanges: &[LegacyExchange],
) -> Result<ImportReport, StorageError> {
    let mut imported = 0;
    for exchange in exchanges {
        if storage
            .import_completed_exchange(
                &exchange.source,
                exchange.index,
                &exchange.user,
                &exchange.assistant,
            )
            .await?
        {
            imported += 1;
        }
    }
    Ok(ImportReport {
        parsed: exchanges.len(),
        imported,
        already_present: exchanges.len().saturating_sub(imported),
    })
}

pub fn export_snapshot_markdown(snapshot: &SessionSnapshot) -> String {
    let mut output = format!(
        "# Lain Session Export\n\nSession: {}\nLast event seq: {}\n\n",
        snapshot.session_id, snapshot.last_seq
    );
    for pair in snapshot.messages.chunks(2) {
        let Some(user) = pair.iter().find(|message| message.role == "user") else {
            continue;
        };
        let assistant = pair.iter().find(|message| message.role == "assistant");
        output.push_str(&format!(
            "## {}\n**User**: {}\n\n",
            user.created_at, user.content
        ));
        if let Some(assistant) = assistant {
            output.push_str(&format!("**Lain**: {}\n\n", assistant.content));
        }
        output.push_str("---\n\n");
    }
    output
}

fn parse_exchange(source: &str, index: usize, block: &str) -> Option<LegacyExchange> {
    let block = block.trim().trim_end_matches('-').trim();
    let user_marker = "**User**:";
    let assistant_marker = "**Lain**:";
    let user_start = block.find(user_marker)? + user_marker.len();
    let assistant_start = block[user_start..].find(assistant_marker)? + user_start;
    let user = block[user_start..assistant_start].trim();
    let assistant = block[assistant_start + assistant_marker.len()..].trim();
    if user.is_empty() || assistant.is_empty() {
        return None;
    }
    Some(LegacyExchange {
        source: source.to_string(),
        index,
        heading: block.lines().next().unwrap_or_default().trim().to_string(),
        user: user.to_string(),
        assistant: assistant.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_current_legacy_markdown_format() {
        let content = "## 12:34:56\n**User**: hello\n\n**Lain**: world\n\n---\n\n";
        let parsed = parse_legacy_markdown("2026-08-11.md", content);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].user, "hello");
        assert_eq!(parsed[0].assistant, "world");
    }

    #[test]
    fn export_remains_human_readable_and_reimportable() {
        let snapshot = SessionSnapshot {
            session_id: "session-1".to_string(),
            last_seq: 5,
            messages: vec![
                crate::storage::MessageRecord {
                    id: "message-1".to_string(),
                    session_id: "session-1".to_string(),
                    run_id: Some("run-1".to_string()),
                    role: "user".to_string(),
                    content: "hello".to_string(),
                    status: "complete".to_string(),
                    created_at: "2026-08-11T00:00:00Z".to_string(),
                    client_message_id: Some("client-1".to_string()),
                },
                crate::storage::MessageRecord {
                    id: "message-2".to_string(),
                    session_id: "session-1".to_string(),
                    run_id: Some("run-1".to_string()),
                    role: "assistant".to_string(),
                    content: "world".to_string(),
                    status: "complete".to_string(),
                    created_at: "2026-08-11T00:00:01Z".to_string(),
                    client_message_id: None,
                },
            ],
            active_run: None,
        };
        let markdown = export_snapshot_markdown(&snapshot);
        assert!(markdown.contains("**User**: hello"));
        assert!(markdown.contains("**Lain**: world"));
        assert_eq!(parse_legacy_markdown("export.md", &markdown).len(), 1);
    }
}
