use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use serde::Deserialize;
use serde_json::json;
use thiserror::Error;

use crate::provider::ToolDefinition;

const MAX_OUTPUT_BYTES: usize = 64 * 1024;
const MAX_FILE_BYTES: u64 = 1024 * 1024;
const MAX_VISITED_FILES: usize = 2_000;
const DEFAULT_LIST_LIMIT: usize = 200;
const DEFAULT_SEARCH_LIMIT: usize = 50;
const DEFAULT_READ_LINES: usize = 200;

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
            _ => Err(ToolError::UnknownTool(call.name)),
        }
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
        let metadata = fs::metadata(&path)?;
        if !metadata.is_file() {
            return Err(ToolError::PathUnavailable("not a file".to_string()));
        }
        if metadata.len() > MAX_FILE_BYTES {
            return Err(ToolError::FileTooLarge);
        }
        let text = fs::read_to_string(&path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::InvalidData {
                ToolError::NotText
            } else {
                ToolError::Io(error)
            }
        })?;
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

fn cap_output(mut output: String) -> String {
    if output.len() <= MAX_OUTPUT_BYTES {
        return output;
    }
    let mut end = MAX_OUTPUT_BYTES;
    while !output.is_char_boundary(end) {
        end -= 1;
    }
    output.truncate(end);
    output.push_str("\n[output truncated]");
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
}
