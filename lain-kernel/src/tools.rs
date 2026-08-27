use std::{
    env, fs,
    path::{Component, Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use serde::Deserialize;
use serde_json::json;
use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    identity::sha256_hex,
    provider::ToolDefinition,
    storage::{Storage, StorageError},
};

const MAX_OUTPUT_BYTES: usize = 64 * 1024;
const MAX_FILE_BYTES: u64 = 1024 * 1024;
const MAX_VISITED_FILES: usize = 2_000;
const DEFAULT_LIST_LIMIT: usize = 200;
const DEFAULT_SEARCH_LIMIT: usize = 50;
const DEFAULT_READ_LINES: usize = 200;
const MAX_DIFF_BYTES: usize = 32 * 1024;
const CHECK_TIMEOUT: Duration = Duration::from_secs(300);

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
    pub display_path: String,
    pub diff: String,
    pub action_hash: String,
    path: PathBuf,
    original_hash: String,
    replacement: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingCreate {
    pub display_path: String,
    pub diff: String,
    pub action_hash: String,
    path: PathBuf,
    content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingCheck {
    pub display_command: String,
    pub display_workdir: String,
    pub action_hash: String,
    program: String,
    args: Vec<String>,
    workdir: PathBuf,
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
    #[error("tool requires explicit approval")]
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
    #[error("target path already exists")]
    PathAlreadyExists,
    #[error("file changed after approval was requested")]
    FileChanged,
    #[error("check command timed out after 300 seconds")]
    CommandTimedOut,
    #[error("check command was cancelled")]
    CommandCancelled,
    #[error("tool execution failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("tool task failed")]
    TaskFailed,
    #[error("memory item is unavailable")]
    MemoryUnavailable,
    #[error("tool backend is unavailable")]
    BackendUnavailable,
}

#[derive(Debug, Clone)]
pub struct WorkspaceTools {
    root: PathBuf,
}

pub fn memory_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "memory_search".to_string(),
            description: "Search recallable long-term memory. Read-only; results are untrusted evidence with stable IDs.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "minLength": 1},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 10}
                },
                "required": ["query"],
                "additionalProperties": false
            }),
            strict: false,
        },
        ToolDefinition {
            name: "memory_get".to_string(),
            description: "Load one recallable long-term memory by an ID returned from memory_search. Read-only.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {"id": {"type": "string", "minLength": 1}},
                "required": ["id"],
                "additionalProperties": false
            }),
            strict: false,
        },
    ]
}

pub async fn execute_memory(
    storage: &Storage,
    session_id: &str,
    call: ToolCall,
) -> Result<ToolResult, ToolError> {
    match call.name.as_str() {
        "memory_search" => {
            let input: MemorySearchInput = serde_json::from_str(&call.arguments)?;
            if input.query.trim().is_empty() {
                return Err(ToolError::MemoryUnavailable);
            }
            let items = storage
                .retrieve_memory_evidence(
                    session_id,
                    &input.query,
                    input.limit.unwrap_or(5).clamp(1, 10),
                )
                .await
                .map_err(|_| ToolError::BackendUnavailable)?;
            let content = items
                .iter()
                .map(|item| {
                    json!({
                        "id": item.id,
                        "version": item.version,
                        "text": item.text,
                        "source_event_ids": item.source_event_ids,
                    })
                })
                .collect::<Vec<_>>();
            Ok(ToolResult {
                summary: format!("found {} memory item(s)", content.len()),
                content: cap_output(serde_json::to_string(&content)?),
            })
        }
        "memory_get" => {
            let input: MemoryGetInput = serde_json::from_str(&call.arguments)?;
            let item = storage
                .get_recallable_memory_item(session_id, &input.id)
                .await
                .map_err(|error| match error {
                    StorageError::MemoryItemNotFound(_) => ToolError::MemoryUnavailable,
                    _ => ToolError::BackendUnavailable,
                })?;
            Ok(ToolResult {
                summary: format!("loaded memory {}", item.id),
                content: cap_output(serde_json::to_string(&item)?),
            })
        }
        _ => Err(ToolError::UnknownTool(call.name)),
    }
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
            ToolDefinition {
                name: "create_file".to_string(),
                description: "Create one new UTF-8 file inside an existing workspace directory. Refuses to overwrite any path and requires one-shot user approval after a diff preview.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Workspace-relative new file path"},
                        "content": {"type": "string"}
                    },
                    "required": ["path", "content"],
                    "additionalProperties": false
                }),
                strict: false,
            },
            ToolDefinition {
                name: "run_check".to_string(),
                description: "Run one predefined local project check after one-shot user approval. No free-form command or arguments are accepted.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "check": {
                            "type": "string",
                            "enum": [
                                "cargo_check", "cargo_test", "cargo_clippy", "cargo_fmt_check",
                                "node_test", "npm_test", "npm_lint", "npm_build"
                            ]
                        },
                        "path": {"type": "string", "description": "Workspace-relative project directory; defaults to ."}
                    },
                    "required": ["check"],
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
            "replace_text" | "create_file" | "run_check" => Err(ToolError::ApprovalRequired),
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
            display_path,
            action_hash,
            path,
            original_hash,
            replacement,
        })
    }

    pub async fn prepare_check(&self, call: ToolCall) -> Result<PendingCheck, ToolError> {
        let tools = self.clone();
        tokio::task::spawn_blocking(move || tools.prepare_check_blocking(call))
            .await
            .map_err(|_| ToolError::TaskFailed)?
    }

    pub async fn run_check(
        &self,
        check: PendingCheck,
        cancellation: CancellationToken,
    ) -> Result<ToolResult, ToolError> {
        let workdir = self.resolve(&check.display_workdir)?;
        if workdir != check.workdir {
            return Err(ToolError::FileChanged);
        }
        run_bounded_command(check, cancellation).await
    }

    fn prepare_check_blocking(&self, call: ToolCall) -> Result<PendingCheck, ToolError> {
        if call.name != "run_check" {
            return Err(ToolError::UnknownTool(call.name));
        }
        let input: RunCheckInput = serde_json::from_str(&call.arguments)?;
        let workdir = self.resolve(input.path.as_deref().unwrap_or("."))?;
        if !workdir.is_dir() {
            return Err(ToolError::PathUnavailable("not a directory".to_string()));
        }
        let (program, args, required_file, npm_script) = input.check.command();
        let manifest = workdir.join(required_file);
        if !manifest.is_file() {
            return Err(ToolError::PathUnavailable(format!(
                "{} not found in project directory",
                required_file
            )));
        }
        if let Some(script) = npm_script {
            let package: serde_json::Value = serde_json::from_str(&read_text_file(&manifest)?)?;
            if package
                .get("scripts")
                .and_then(|scripts| scripts.get(script))
                .and_then(serde_json::Value::as_str)
                .is_none()
            {
                return Err(ToolError::PathUnavailable(format!(
                    "npm script {script} not found"
                )));
            }
        }
        let display_workdir = match relative_display(&self.root, &workdir) {
            value if value.is_empty() => ".".to_string(),
            value => value,
        };
        let display_command = std::iter::once(program)
            .chain(args.iter().copied())
            .collect::<Vec<_>>()
            .join(" ");
        Ok(PendingCheck {
            action_hash: sha256(&format!("run_check\0{display_workdir}\0{display_command}")),
            display_command,
            display_workdir,
            program: platform_program(program),
            args: args.into_iter().map(str::to_string).collect(),
            workdir,
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

    pub async fn prepare_create(&self, call: ToolCall) -> Result<PendingCreate, ToolError> {
        let tools = self.clone();
        tokio::task::spawn_blocking(move || tools.prepare_create_blocking(call))
            .await
            .map_err(|_| ToolError::TaskFailed)?
    }

    pub async fn apply_create(&self, create: PendingCreate) -> Result<ToolResult, ToolError> {
        let tools = self.clone();
        tokio::task::spawn_blocking(move || tools.apply_create_blocking(create))
            .await
            .map_err(|_| ToolError::TaskFailed)?
    }

    fn prepare_create_blocking(&self, call: ToolCall) -> Result<PendingCreate, ToolError> {
        if call.name != "create_file" {
            return Err(ToolError::UnknownTool(call.name));
        }
        let input: CreateFileInput = serde_json::from_str(&call.arguments)?;
        if input.content.len() as u64 > MAX_FILE_BYTES {
            return Err(ToolError::ReplacementTooLarge);
        }
        let path = self.resolve_new_file(&input.path)?;
        let display_path = relative_display(&self.root, &path);
        let content_hash = sha256(&input.content);
        Ok(PendingCreate {
            diff: creation_preview(&display_path, &input.content),
            action_hash: sha256(&format!("create_file\0{display_path}\0{content_hash}")),
            display_path,
            path,
            content: input.content,
        })
    }

    fn apply_create_blocking(&self, create: PendingCreate) -> Result<ToolResult, ToolError> {
        let path = self.resolve_new_file(&create.display_path)?;
        if path != create.path {
            return Err(ToolError::FileChanged);
        }
        create_file_atomic(&path, create.content.as_bytes())?;
        Ok(ToolResult {
            content: format!("created {}", create.display_path),
            summary: format!("created {}", create.display_path),
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
                "path is excluded by workspace policy".to_string(),
            ));
        }
        let path = fs::canonicalize(self.root.join(relative))
            .map_err(|error| ToolError::PathUnavailable(error.to_string()))?;
        if !path.starts_with(&self.root) {
            return Err(ToolError::PathOutsideWorkspace);
        }
        Ok(path)
    }

    fn resolve_new_file(&self, relative: &str) -> Result<PathBuf, ToolError> {
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
                "path is excluded by workspace policy".to_string(),
            ));
        }
        let file_name = match relative.components().next_back() {
            Some(Component::Normal(name)) if valid_new_file_name(name) => name,
            _ => {
                return Err(ToolError::PathUnavailable(
                    "path must name a regular file".to_string(),
                ));
            }
        };
        let parent_relative = relative.parent().unwrap_or_else(|| Path::new("."));
        let parent = self.resolve(&parent_relative.to_string_lossy())?;
        if !parent.is_dir() {
            return Err(ToolError::PathUnavailable(
                "parent is not a directory".to_string(),
            ));
        }
        let path = parent.join(file_name);
        match fs::symlink_metadata(&path) {
            Ok(_) => Err(ToolError::PathAlreadyExists),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(path),
            Err(error) => Err(ToolError::Io(error)),
        }
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
struct MemorySearchInput {
    query: String,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MemoryGetInput {
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplaceTextInput {
    path: String,
    old_text: String,
    new_text: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateFileInput {
    path: String,
    content: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RunCheckInput {
    check: CheckKind,
    path: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CheckKind {
    CargoCheck,
    CargoTest,
    CargoClippy,
    CargoFmtCheck,
    NodeTest,
    NpmTest,
    NpmLint,
    NpmBuild,
}

impl CheckKind {
    fn command(
        self,
    ) -> (
        &'static str,
        Vec<&'static str>,
        &'static str,
        Option<&'static str>,
    ) {
        match self {
            Self::CargoCheck => (
                "cargo",
                vec!["check", "--all-targets", "--all-features"],
                "Cargo.toml",
                None,
            ),
            Self::CargoTest => (
                "cargo",
                vec!["test", "--all-targets", "--all-features"],
                "Cargo.toml",
                None,
            ),
            Self::CargoClippy => (
                "cargo",
                vec![
                    "clippy",
                    "--all-targets",
                    "--all-features",
                    "--",
                    "-D",
                    "warnings",
                ],
                "Cargo.toml",
                None,
            ),
            Self::CargoFmtCheck => (
                "cargo",
                vec!["fmt", "--all", "--", "--check"],
                "Cargo.toml",
                None,
            ),
            Self::NodeTest => ("node", vec!["--test"], "package.json", None),
            Self::NpmTest => ("npm", vec!["test"], "package.json", Some("test")),
            Self::NpmLint => ("npm", vec!["run", "lint"], "package.json", Some("lint")),
            Self::NpmBuild => ("npm", vec!["run", "build"], "package.json", Some("build")),
        }
    }
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

fn valid_new_file_name(name: &std::ffi::OsStr) -> bool {
    let name = name.to_string_lossy();
    if name.is_empty() {
        return false;
    }
    if !cfg!(windows) {
        return true;
    }
    if name.contains(':') || name.ends_with(['.', ' ']) {
        return false;
    }
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    !(matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CLOCK$")
        || (stem.len() == 4
            && matches!(&stem[..3], "COM" | "LPT")
            && matches!(stem.as_bytes()[3], b'1'..=b'9')))
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

fn creation_preview(path: &str, content: &str) -> String {
    let mut preview = format!("--- /dev/null\n+++ {path}\n@@ new file @@\n");
    for line in content.lines() {
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

fn create_file_atomic(path: &Path, content: &[u8]) -> Result<(), ToolError> {
    use std::io::Write;

    let parent = path
        .parent()
        .ok_or_else(|| ToolError::PathUnavailable("file has no parent".to_string()))?;
    let temp = parent.join(format!(".lain-create-{}.tmp", Uuid::now_v7()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)?;
        file.write_all(content)?;
        file.sync_all()?;
        drop(file);
        match fs::hard_link(&temp, path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                Err(ToolError::PathAlreadyExists)
            }
            Err(error) => Err(ToolError::Io(error)),
        }
    })();
    let _ = fs::remove_file(&temp);
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

fn platform_program(program: &str) -> String {
    if cfg!(windows) && program == "npm" {
        "npm.cmd".to_string()
    } else {
        program.to_string()
    }
}

async fn run_bounded_command(
    check: PendingCheck,
    cancellation: CancellationToken,
) -> Result<ToolResult, ToolError> {
    let mut command = Command::new(&check.program);
    command
        .args(&check.args)
        .current_dir(&check.workdir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .env("CI", "1")
        .env("NO_COLOR", "1")
        .env("CARGO_TERM_COLOR", "never")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("npm_config_audit", "false")
        .env("npm_config_fund", "false")
        .env("npm_config_progress", "false");
    for (name, _) in env::vars_os() {
        if sensitive_env_name(&name.to_string_lossy()) {
            command.env_remove(name);
        }
    }

    let mut child = command.spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ToolError::PathUnavailable("stdout unavailable".to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| ToolError::PathUnavailable("stderr unavailable".to_string()))?;
    let stdout_task = tokio::spawn(read_bounded(stdout));
    let stderr_task = tokio::spawn(read_bounded(stderr));
    enum Completion {
        Status(std::io::Result<std::process::ExitStatus>),
        Cancelled,
        TimedOut,
    }
    let completion = {
        let wait = child.wait();
        tokio::pin!(wait);
        tokio::select! {
            status = &mut wait => Completion::Status(status),
            () = cancellation.cancelled() => Completion::Cancelled,
            () = tokio::time::sleep(CHECK_TIMEOUT) => Completion::TimedOut,
        }
    };
    let status = match completion {
        Completion::Status(status) => status?,
        Completion::Cancelled | Completion::TimedOut => {
            // ponytail: direct-child kill is sufficient for preset foreground checks; add a
            // process-tree backend before exposing arbitrary commands or background jobs.
            let _ = child.kill().await;
            let _ = child.wait().await;
            stdout_task.abort();
            stderr_task.abort();
            return Err(if matches!(completion, Completion::TimedOut) {
                ToolError::CommandTimedOut
            } else {
                ToolError::CommandCancelled
            });
        }
    };
    let stdout = stdout_task.await.map_err(|_| ToolError::TaskFailed)??;
    let stderr = stderr_task.await.map_err(|_| ToolError::TaskFailed)??;
    let exit = status
        .code()
        .map_or_else(|| "signal".to_string(), |code| code.to_string());
    let mut content = format!("[exit code: {exit}]");
    if !stdout.is_empty() {
        content.push_str("\n[stdout]\n");
        content.push_str(&String::from_utf8_lossy(&stdout));
    }
    if !stderr.is_empty() {
        content.push_str("\n[stderr]\n");
        content.push_str(&String::from_utf8_lossy(&stderr));
    }
    Ok(ToolResult {
        summary: format!("{} exited {exit}", check.display_command),
        content: cap_output(content),
    })
}

async fn read_bounded(mut reader: impl AsyncRead + Unpin) -> std::io::Result<Vec<u8>> {
    let mut kept = Vec::new();
    let mut chunk = [0_u8; 8 * 1024];
    loop {
        let read = reader.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        let remaining = MAX_OUTPUT_BYTES.saturating_sub(kept.len());
        kept.extend_from_slice(&chunk[..read.min(remaining)]);
    }
    Ok(kept)
}

fn sensitive_env_name(name: &str) -> bool {
    let name = name.to_ascii_uppercase();
    matches!(
        name.as_str(),
        "API_KEY"
            | "TOKEN"
            | "SECRET"
            | "PASSWORD"
            | "CREDENTIAL"
            | "CREDENTIALS"
            | "DOCKER_AUTH_CONFIG"
            | "KUBECONFIG"
            | "SSH_AUTH_SOCK"
    ) || name.ends_with("_KEY")
        || name.contains("_TOKEN")
        || name.contains("_SECRET")
        || name.contains("_PASSWORD")
        || name.contains("_CREDENTIAL")
        || name.starts_with("DEEPSEEK_")
        || name.starts_with("OPENAI_")
        || name.starts_with("ANTHROPIC_")
        || name.starts_with("AWS_")
        || name.starts_with("AZURE_")
        || name.starts_with("GITHUB_")
        || name.starts_with("GH_")
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
    async fn memory_tools_search_and_fail_closed() {
        let storage = Storage::in_memory().await.expect("storage");
        let session_id = storage.scope().session_id.clone();
        let searched = execute_memory(
            &storage,
            &session_id,
            ToolCall {
                id: "memory-1".to_string(),
                name: "memory_search".to_string(),
                arguments: r#"{"query":"editor","limit":5}"#.to_string(),
            },
        )
        .await
        .expect("empty search");
        assert_eq!(searched.content, "[]");
        assert_eq!(
            memory_definitions()
                .into_iter()
                .map(|definition| definition.name)
                .collect::<Vec<_>>(),
            ["memory_search", "memory_get"]
        );
        assert!(matches!(
            execute_memory(
                &storage,
                &session_id,
                ToolCall {
                    id: "memory-2".to_string(),
                    name: "memory_get".to_string(),
                    arguments: r#"{"id":"missing"}"#.to_string(),
                },
            )
            .await,
            Err(ToolError::MemoryUnavailable)
        ));
    }

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

    #[tokio::test]
    async fn create_requires_preview_and_never_overwrites() {
        let fixture = TempDir::new().expect("fixture");
        fs::create_dir(fixture.path().join("src")).expect("directory");
        let tools = WorkspaceTools::new(fixture.path()).expect("tools");
        let call = ToolCall {
            id: "create-1".to_string(),
            name: "create_file".to_string(),
            arguments: r#"{"path":"src/new.rs","content":"fn created() {}\n"}"#.to_string(),
        };
        assert!(matches!(
            tools.execute(call.clone()).await,
            Err(ToolError::ApprovalRequired)
        ));
        let create = tools.prepare_create(call).await.expect("preview");
        assert!(create.diff.contains("+++ src/new.rs\n"));
        assert!(!fixture.path().join("src/new.rs").exists());
        tools.apply_create(create).await.expect("approved create");
        assert_eq!(
            fs::read_to_string(fixture.path().join("src/new.rs")).expect("created"),
            "fn created() {}\n"
        );

        let raced = tools
            .prepare_create(ToolCall {
                id: "create-2".to_string(),
                name: "create_file".to_string(),
                arguments: r#"{"path":"race.txt","content":"agent\n"}"#.to_string(),
            })
            .await
            .expect("race preview");
        fs::write(fixture.path().join("race.txt"), "external\n").expect("racer");
        assert!(matches!(
            tools.apply_create(raced).await,
            Err(ToolError::PathAlreadyExists)
        ));
        assert_eq!(
            fs::read_to_string(fixture.path().join("race.txt")).expect("external kept"),
            "external\n"
        );
        for path in ["../outside.txt", ".env"] {
            assert!(
                tools
                    .prepare_create(ToolCall {
                        id: format!("invalid-{path}"),
                        name: "create_file".to_string(),
                        arguments: serde_json::json!({"path": path, "content": "secret"})
                            .to_string(),
                    })
                    .await
                    .is_err()
            );
        }
        #[cfg(windows)]
        assert!(
            tools
                .prepare_create(ToolCall {
                    id: "reserved".to_string(),
                    name: "create_file".to_string(),
                    arguments: r#"{"path":"NUL.txt","content":"invalid"}"#.to_string(),
                })
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn checks_are_predefined_approved_and_bounded() {
        let fixture = TempDir::new().expect("fixture");
        fs::write(
            fixture.path().join("Cargo.toml"),
            "[package]\nname='fixture'\n",
        )
        .expect("manifest");
        let tools = WorkspaceTools::new(fixture.path()).expect("tools");
        let call = ToolCall {
            id: "check-1".to_string(),
            name: "run_check".to_string(),
            arguments: r#"{"check":"cargo_fmt_check"}"#.to_string(),
        };
        assert!(matches!(
            tools.execute(call.clone()).await,
            Err(ToolError::ApprovalRequired)
        ));
        let prepared = tools.prepare_check(call).await.expect("check preview");
        assert_eq!(prepared.display_command, "cargo fmt --all -- --check");
        assert_eq!(prepared.display_workdir, ".");

        let result = run_bounded_command(
            PendingCheck {
                display_command: "test-binary --list".to_string(),
                display_workdir: ".".to_string(),
                action_hash: "test".to_string(),
                program: std::env::current_exe()
                    .expect("test binary")
                    .to_string_lossy()
                    .into_owned(),
                args: vec!["--list".to_string()],
                workdir: fixture.path().to_path_buf(),
            },
            CancellationToken::new(),
        )
        .await
        .expect("bounded process");
        assert!(result.content.starts_with("[exit code: 0]"));
        assert!(sensitive_env_name("DEEPSEEK_API_KEY"));

        let cancellation = CancellationToken::new();
        cancellation.cancel();
        let cancelled = run_bounded_command(
            PendingCheck {
                display_command: "cancelled check".to_string(),
                display_workdir: ".".to_string(),
                action_hash: "test".to_string(),
                program: std::env::current_exe()
                    .expect("test binary")
                    .to_string_lossy()
                    .into_owned(),
                args: vec!["--list".to_string()],
                workdir: fixture.path().to_path_buf(),
            },
            cancellation,
        )
        .await;
        assert!(matches!(cancelled, Err(ToolError::CommandCancelled)));
    }
}
