use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use serde::Deserialize;
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

use crate::{identity::sha256_hex, provider::ToolDefinition};

const MAX_OUTPUT_BYTES: usize = 64 * 1024;
const MAX_FILE_BYTES: u64 = 1024 * 1024;
const MAX_VISITED_FILES: usize = 2_000;
const DEFAULT_LIST_LIMIT: usize = 200;
const DEFAULT_SEARCH_LIMIT: usize = 50;
const DEFAULT_READ_LINES: usize = 200;
const MAX_DIFF_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResult {
    pub content: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingEdit {
    pub call: ToolCall,
    pub display_path: String,
    pub diff: String,
    pub action_hash: String,
    path: PathBuf,
    original_hash: String,
    replacement: String,
}

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("invalid tool arguments: {0}")]
    InvalidArguments(#[from] serde_json::Error),
    #[error("unknown tool: {0}")]
    UnknownTool(String),
    #[error("path must stay inside the workspace")]
    PathOutsideWorkspace,
    #[error("path is unavailable: {0}")]
    PathUnavailable(String),
    #[error("file is too large")]
    FileTooLarge,
    #[error("file is not UTF-8 text")]
    NotText,
    #[error("replace_text requires explicit approval")]
    ApprovalRequired,
    #[error("edit rejected by user")]
    ApprovalRejected,
    #[error("edit approval unavailable")]
    ApprovalUnavailable,
    #[error("old_text cannot be empty")]
    EmptyOldText,
    #[error("old_text must match exactly once, found {0} occurrences")]
    MatchCount(usize),
    #[error("replacement would exceed the file size limit")]
    ReplacementTooLarge,
    #[error("file changed after approval was requested")]
    FileChanged,
    #[error("tool execution failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("tool task failed")]
    TaskFailed,
}

#[derive(Debug, Clone)]
pub struct WorkspaceTools {
    root: PathBuf,
}

impl WorkspaceTools {
    pub fn new(root: impl AsRef<Path>) -> Result<Self, ToolError> {
        Ok(Self {
            root: fs::canonicalize(root)
                .map_err(|error| ToolError::PathUnavailable(format!("workspace root: {error}")))?,
        })
    }

    pub fn definitions() -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "list_files".to_string(),
                description: "List files under a workspace-relative directory. Read-only; use this before guessing paths.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Workspace-relative directory; defaults to ."},
                        "limit": {"type": "integer", "minimum": 1, "maximum": 500}
                    },
                    "additionalProperties": false
                }),
                strict: false,
            },
            ToolDefinition {
                name: "read_file".to_string(),
                description: "Read a bounded range from a UTF-8 text file inside the workspace.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"},
                        "start_line": {"type": "integer", "minimum": 1},
                        "max_lines": {"type": "integer", "minimum": 1, "maximum": 500}
                    },
                    "required": ["path"],
                    "additionalProperties": false
                }),
                strict: false,
            },
            ToolDefinition {
                name: "search_text".to_string(),
                description: "Search UTF-8 workspace files for a case-insensitive literal text query and return path:line matches.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "minLength": 1},
                        "path": {"type": "string", "description": "Workspace-relative file or directory; defaults to ."},
                        "limit": {"type": "integer", "minimum": 1, "maximum": 100}
                    },
                    "required": ["query"],
                    "additionalProperties": false
                }),
                strict: false,
            },
            ToolDefinition {
                name: "replace_text".to_string(),
                description: "Replace one exact, uniquely occurring text block in an existing UTF-8 workspace file. Requires one-shot user approval after a diff preview.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Workspace-relative existing file"},
                        "old_text": {"type": "string", "minLength": 1},
                        "new_text": {"type": "string"}
                    },
                    "required": ["path", "old_text", "new_text"],
                    "additionalProperties": false
                }),
                strict: false,
            },
        ]
    }

    pub async fn execute(&self, call: ToolCall) -> Result<ToolResult, ToolError> {
        let tools = self.clone();
        tokio::task::spawn_blocking(move || tools.execute_blocking(call))
            .await
            .map_err(|_| ToolError::TaskFailed)?
    }

    fn execute_blocking(&self, call: ToolCall) -> Result<ToolResult, ToolError> {
        match call.name.as_str() {
            "list_files" => self.list_files(serde_json::from_str(&call.arguments)?),
            "read_file" => self.read_file(serde_json::from_str(&call.arguments)?),
            "search_text" => self.search_text(serde_json::from_str(&call.arguments)?),
            "replace_text" => Err(ToolError::ApprovalRequired),
            _ => Err(ToolError::UnknownTool(call.name)),
        }
    }

    pub async fn prepare_replace(&self, call: ToolCall) -> Result<PendingEdit, ToolError> {
        let tools = self.clone();
        tokio::task::spawn_blocking(move || tools.prepare_replace_blocking(call))
            .await
            .map_err(|_| ToolError::TaskFailed)?
    }

    pub async fn apply_replace(&self, edit: PendingEdit) -> Result<ToolResult, ToolError> {
        let tools = self.clone();
        tokio::task::spawn_blocking(move || tools.apply_replace_blocking(edit))
            .await
            .map_err(|_| ToolError::TaskFailed)?
    }

    fn prepare_replace_blocking(&self, call: ToolCall) -> Result<PendingEdit, ToolError> {
        if call.name != "replace_text" {
            return Err(ToolError::UnknownTool(call.name));
        }
        let input: ReplaceTextInput = serde_json::from_str(&call.arguments)?;
        if input.old_text.is_empty() {
            return Err(ToolError::EmptyOldText);
        }
        let path = self.resolve(&input.path)?;
        let original = read_text_file(&path)?;
        let count = original.match_indices(&input.old_text).count();
        if count != 1 {
            return Err(ToolError::MatchCount(count));
        }
        let replacement = original.replacen(&input.old_text, &input.new_text, 1);
        if replacement.len() as u64 > MAX_FILE_BYTES {
            return Err(ToolError::ReplacementTooLarge);
        }
        let display_path = relative_display(&self.root, &path);
        let original_hash = sha256(&original);
        let action_hash = sha256(&format!(
            "replace_text\0{display_path}\0{}\0{original_hash}",
            call.arguments
        ));
        Ok(PendingEdit {
            diff: replacement_preview(&display_path, &input.old_text, &input.new_text),
            call,
            display_path,
            action_hash,
            path,
            original_hash,
            replacement,
        })
    }

    fn apply_replace_blocking(&self, edit: PendingEdit) -> Result<ToolResult, ToolError> {
        let path = self.resolve(&edit.display_path)?;
        if path != edit.path {
            return Err(ToolError::FileChanged);
        }
        let current = read_text_file(&path)?;
        if sha256(&current) != edit.original_hash {
            return Err(ToolError::FileChanged);
        }
        replace_file(&path, edit.replacement.as_bytes(), &edit.original_hash)?;
        Ok(ToolResult {
            content: format!("updated {}", edit.display_path),
            summary: format!("updated {}", edit.display_path),
        })
    }

    fn list_files(&self, input: ListFilesInput) -> Result<ToolResult, ToolError> {
        let path = self.resolve(input.path.as_deref().unwrap_or("."))?;
        if !path.is_dir() {
            return Err(ToolError::PathUnavailable("not a directory".to_string()));
        }
        let limit = input.limit.unwrap_or(DEFAULT_LIST_LIMIT).clamp(1, 500);
        let mut files = Vec::new();
        collect_files(&path, &self.root, &mut files, limit, &mut 0)?;
        files.sort();
        let truncated = files.len() == limit;
        let content = cap_output(files.join("\n"));
        Ok(ToolResult {
            summary: format!(
                "listed {} path{}{}",
                files.len(),
                if files.len() == 1 { "" } else { "s" },
                if truncated { " (limit reached)" } else { "" }
            ),
            content,
        })
    }

    fn read_file(&self, input: ReadFileInput) -> Result<ToolResult, ToolError> {
        let path = self.resolve(&input.path)?;
        let text = read_text_file(&path)?;
        let start = input.start_line.unwrap_or(1).max(1);
        let max_lines = input.max_lines.unwrap_or(DEFAULT_READ_LINES).clamp(1, 500);
        let selected = text
            .lines()
            .enumerate()
            .skip(start - 1)
            .take(max_lines)
            .map(|(index, line)| format!("{}: {line}", index + 1))
            .collect::<Vec<_>>();
        Ok(ToolResult {
            summary: format!("read {} line(s) from {}", selected.len(), input.path),
            content: cap_output(selected.join("\n")),
        })
    }

    fn search_text(&self, input: SearchTextInput) -> Result<ToolResult, ToolError> {
        if input.query.is_empty() {
            return Err(ToolError::PathUnavailable(
                "query cannot be empty".to_string(),
            ));
        }
        let path = self.resolve(input.path.as_deref().unwrap_or("."))?;
        let limit = input.limit.unwrap_or(DEFAULT_SEARCH_LIMIT).clamp(1, 100);
        let query = input.query.to_lowercase();
        let mut files = Vec::new();
        if path.is_file() {
            files.push(path);
        } else if path.is_dir() {
            collect_file_paths(&path, &self.root, &mut files, MAX_VISITED_FILES)?;
        } else {
            return Err(ToolError::PathUnavailable(
                "not a file or directory".to_string(),
            ));
        }
        files.sort();
        let mut matches = Vec::new();
        for file in files {
            if matches.len() >= limit {
                break;
            }
            if fs::metadata(&file).is_ok_and(|metadata| metadata.len() > MAX_FILE_BYTES) {
                continue;
            }
            let Ok(text) = fs::read_to_string(&file) else {
                continue;
            };
            for (index, line) in text.lines().enumerate() {
                if line.to_lowercase().contains(&query) {
                    matches.push(format!(
                        "{}:{}: {}",
                        relative_display(&self.root, &file),
                        index + 1,
                        line.trim()
                    ));
                    if matches.len() >= limit {
                        break;
                    }
                }
            }
        }
        Ok(ToolResult {
            summary: format!("found {} match(es)", matches.len()),
            content: cap_output(matches.join("\n")),
        })
    }

    fn resolve(&self, relative: &str) -> Result<PathBuf, ToolError> {
        let relative = Path::new(relative);
        if relative.is_absolute()
            || relative.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(ToolError::PathOutsideWorkspace);
        }
        if relative.components().any(|component| match component {
            Component::Normal(name) => excluded_name(&name.to_string_lossy()),
            _ => false,
        }) {
            return Err(ToolError::PathUnavailable(
                "path is excluded by read policy".to_string(),
            ));
        }
        let path = fs::canonicalize(self.root.join(relative))
            .map_err(|error| ToolError::PathUnavailable(error.to_string()))?;
        if !path.starts_with(&self.root) {
            return Err(ToolError::PathOutsideWorkspace);
        }
        Ok(path)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListFilesInput {
    path: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadFileInput {
    path: String,
    start_line: Option<usize>,
    max_lines: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchTextInput {
    query: String,
    path: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplaceTextInput {
    path: String,
    old_text: String,
    new_text: String,
}

fn collect_files(
    directory: &Path,
    root: &Path,
    output: &mut Vec<String>,
    limit: usize,
    visited: &mut usize,
) -> Result<(), std::io::Error> {
    if output.len() >= limit || *visited >= MAX_VISITED_FILES {
        return Ok(());
    }
    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        if output.len() >= limit || *visited >= MAX_VISITED_FILES {
            break;
        }
        let file_type = entry.file_type()?;
        if file_type.is_symlink() || excluded_name(&entry.file_name().to_string_lossy()) {
            continue;
        }
        *visited += 1;
        let path = entry.path();
        if file_type.is_dir() {
            collect_files(&path, root, output, limit, visited)?;
        } else if file_type.is_file() {
            output.push(relative_display(root, &path));
        }
    }
    Ok(())
}

fn collect_file_paths(
    directory: &Path,
    root: &Path,
    output: &mut Vec<PathBuf>,
    limit: usize,
) -> Result<(), std::io::Error> {
    if output.len() >= limit {
        return Ok(());
    }
    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        if output.len() >= limit {
            break;
        }
        let file_type = entry.file_type()?;
        if file_type.is_symlink() || excluded_name(&entry.file_name().to_string_lossy()) {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            collect_file_paths(&path, root, output, limit)?;
        } else if file_type.is_file() && path.starts_with(root) {
            output.push(path);
        }
    }
    Ok(())
}

fn excluded_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        ".git"
            | ".lain"
            | "node_modules"
            | "target"
            | ".env"
            | ".npmrc"
            | ".pypirc"
            | "credentials"
            | "credentials.json"
            | "id_rsa"
            | "id_ed25519"
    ) || (lower.starts_with(".env.") && lower != ".env.example")
        || matches!(
            Path::new(&lower)
                .extension()
                .and_then(|value| value.to_str()),
            Some("pem" | "key" | "p12" | "pfx")
        )
}

fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn read_text_file(path: &Path) -> Result<String, ToolError> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(ToolError::PathUnavailable("not a file".to_string()));
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Err(ToolError::FileTooLarge);
    }
    fs::read_to_string(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::InvalidData {
            ToolError::NotText
        } else {
            ToolError::Io(error)
        }
    })
}

fn replacement_preview(path: &str, old_text: &str, new_text: &str) -> String {
    let mut preview = format!("--- {path}\n+++ {path}\n@@ exact replacement @@\n");
    for line in old_text.lines() {
        preview.push_str(&format!("-{line}\n"));
    }
    for line in new_text.lines() {
        preview.push_str(&format!("+{line}\n"));
    }
    cap_at(preview, MAX_DIFF_BYTES, "\n[diff truncated]")
}

fn sha256(value: &str) -> String {
    sha256_hex(value.as_bytes())
}

fn replace_file(path: &Path, content: &[u8], expected_hash: &str) -> Result<(), ToolError> {
    use std::io::Write;

    let parent = path
        .parent()
        .ok_or_else(|| ToolError::PathUnavailable("file has no parent".to_string()))?;
    let temp = parent.join(format!(".lain-edit-{}.tmp", Uuid::now_v7()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)?;
        file.write_all(content)?;
        file.sync_all()?;
        let permissions = fs::metadata(path)?.permissions();
        fs::set_permissions(&temp, permissions)?;
        if sha256(&read_text_file(path)?) != expected_hash {
            return Err(ToolError::FileChanged);
        }
        atomic_replace(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(not(windows))]
fn atomic_replace(source: &Path, destination: &Path) -> Result<(), ToolError> {
    fs::rename(source, destination).map_err(ToolError::Io)
}

#[cfg(windows)]
fn atomic_replace(source: &Path, destination: &Path) -> Result<(), ToolError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    // SAFETY: both pointers reference null-terminated UTF-16 buffers for the duration of the call.
    let succeeded = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if succeeded == 0 {
        Err(ToolError::Io(std::io::Error::last_os_error()))
    } else {
        Ok(())
    }
}

fn cap_output(output: String) -> String {
    cap_at(output, MAX_OUTPUT_BYTES, "\n[output truncated]")
}

fn cap_at(mut output: String, limit: usize, suffix: &str) -> String {
    if output.len() <= limit {
        return output;
    }
    let mut end = limit;
    while !output.is_char_boundary(end) {
        end -= 1;
    }
    output.truncate(end);
    output.push_str(suffix);
    output
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn read_only_tools_stay_in_workspace_and_return_bounded_text() {
        let fixture = TempDir::new().expect("fixture");
        fs::create_dir(fixture.path().join("src")).expect("directory");
        fs::write(
            fixture.path().join("src/main.rs"),
            "fn main() {}\n// needle\n",
        )
        .expect("source");
        fs::write(fixture.path().join(".env"), "SECRET=value\n").expect("secret");
        let tools = WorkspaceTools::new(fixture.path()).expect("tools");

        let listed = tools
            .execute(ToolCall {
                id: "1".to_string(),
                name: "list_files".to_string(),
                arguments: r#"{"path":"."}"#.to_string(),
            })
            .await
            .expect("list");
        assert_eq!(listed.content, "src/main.rs");

        let searched = tools
            .execute(ToolCall {
                id: "2".to_string(),
                name: "search_text".to_string(),
                arguments: r#"{"query":"NEEDLE"}"#.to_string(),
            })
            .await
            .expect("search");
        assert_eq!(searched.content, "src/main.rs:2: // needle");

        let outside = tools
            .execute(ToolCall {
                id: "3".to_string(),
                name: "read_file".to_string(),
                arguments: r#"{"path":"../outside"}"#.to_string(),
            })
            .await;
        assert!(matches!(outside, Err(ToolError::PathOutsideWorkspace)));
        let secret = tools
            .execute(ToolCall {
                id: "4".to_string(),
                name: "read_file".to_string(),
                arguments: r#"{"path":".env"}"#.to_string(),
            })
            .await;
        assert!(matches!(secret, Err(ToolError::PathUnavailable(_))));
    }

    #[tokio::test]
    async fn replace_requires_preview_and_rejects_stale_files() {
        let fixture = TempDir::new().expect("fixture");
        let path = fixture.path().join("note.txt");
        fs::write(&path, "before\n").expect("source");
        let tools = WorkspaceTools::new(fixture.path()).expect("tools");
        let call = ToolCall {
            id: "edit-1".to_string(),
            name: "replace_text".to_string(),
            arguments: r#"{"path":"note.txt","old_text":"before","new_text":"after"}"#.to_string(),
        };

        assert!(matches!(
            tools.execute(call.clone()).await,
            Err(ToolError::ApprovalRequired)
        ));
        let edit = tools.prepare_replace(call).await.expect("preview");
        assert!(edit.diff.contains("-before\n+after"));
        assert_eq!(fs::read_to_string(&path).expect("unchanged"), "before\n");
        tools.apply_replace(edit).await.expect("approved edit");
        assert_eq!(fs::read_to_string(&path).expect("updated"), "after\n");

        let stale = tools
            .prepare_replace(ToolCall {
                id: "edit-2".to_string(),
                name: "replace_text".to_string(),
                arguments: r#"{"path":"note.txt","old_text":"after","new_text":"final"}"#
                    .to_string(),
            })
            .await
            .expect("second preview");
        fs::write(&path, "changed elsewhere\n").expect("race fixture");
        assert!(matches!(
            tools.apply_replace(stale).await,
            Err(ToolError::FileChanged)
        ));
        assert_eq!(
            fs::read_to_string(&path).expect("external change kept"),
            "changed elsewhere\n"
        );
    }
}
