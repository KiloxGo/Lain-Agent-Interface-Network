use chrono::Local;
use regex::Regex;
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;

pub mod policy;
pub mod types;
pub mod worker;

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn configure_data_dir(path: PathBuf) -> std::io::Result<()> {
    if let Some(configured) = DATA_DIR.get() {
        if configured == &path {
            return Ok(());
        }
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "memory data directory already configured as {}",
                configured.display()
            ),
        ));
    }
    DATA_DIR.set(path).map_err(|path| {
        std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "failed to configure memory data directory: {}",
                path.display()
            ),
        )
    })
}

fn lain_dir() -> PathBuf {
    DATA_DIR.get().cloned().unwrap_or_else(|| {
        dirs::home_dir()
            .map(|home| home.join(".lain"))
            .or_else(|| std::env::current_dir().ok().map(|cwd| cwd.join(".lain")))
            .unwrap_or_else(|| PathBuf::from(".lain"))
    })
}

fn memory_dir() -> PathBuf {
    lain_dir().join("memory")
}

fn sessions_dir() -> PathBuf {
    memory_dir().join("sessions")
}

fn profile_path() -> PathBuf {
    memory_dir().join("user/profile.md")
}

fn index_path() -> PathBuf {
    memory_dir().join("MEMORY.md")
}

pub async fn log_exchange(user_msg: &str, lain_reply: &str) -> std::io::Result<()> {
    let mut path = sessions_dir();

    fs::create_dir_all(&path).await?;

    let now = Local::now();
    let date_str = now.format("%Y-%m-%d").to_string();
    let time_str = now.format("%H:%M:%S").to_string();

    path.push(format!("{}.md", date_str));

    let content = format!(
        "## {}\n**User**: {}\n\n**Lain**: {}\n\n---\n\n",
        time_str, user_msg, lain_reply
    );

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .await?;

    file.write_all(content.as_bytes()).await?;

    let _ = update_index().await;
    Ok(())
}

pub async fn update_profile(key: &str, value: &str) -> std::io::Result<()> {
    let path = profile_path();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }

    let mut content = if path.exists() {
        fs::read_to_string(&path).await?
    } else {
        "# User Profile\n\n".to_string()
    };

    let key_line = format!("- {}", key);
    if let Some(pos) = content.find(&key_line) {
        let line_start = pos;
        let line_end = content[line_start..]
            .find('\n')
            .map(|i| line_start + i)
            .unwrap_or(content.len());
        let new_line = format!("- {}: {}", key, value);
        content.replace_range(line_start..line_end, &new_line);
    } else {
        content.push_str(&format!("- {}: {}\n", key, value));
    }
    fs::write(&path, content).await?;
    let _ = update_index().await;
    Ok(())
}

pub async fn load_profile() -> std::io::Result<Option<String>> {
    match fs::read_to_string(profile_path()).await {
        Ok(profile) => Ok(Some(profile)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub async fn update_index() -> std::io::Result<()> {
    let profile_content = match fs::read_to_string(profile_path()).await {
        Ok(content) => content,
        Err(_) => "# User Profile\n\nNo profile yet.\n".to_string(),
    };

    let sessions_path = sessions_dir();
    let mut sessions: Vec<(String, u32)> = Vec::new();
    let mut total_exchanges = 0;

    if let Ok(mut entries) = fs::read_dir(&sessions_path).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(metadata) = entry.metadata().await
                && metadata.is_file()
                && let Some(name) = entry.file_name().to_str()
                && name.ends_with(".md")
            {
                let content = fs::read_to_string(entry.path()).await.unwrap_or_default();
                let exchanges = content.matches("---").count() as u32;
                sessions.push((name.to_string(), exchanges));
                total_exchanges += exchanges;
            }
        }
    }

    sessions.sort_by(|a, b| a.0.cmp(&b.0));

    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut index = format!("# Lain's Memory\n\nLast updated: {}\n\n", now);

    index.push_str("## User Profile\n");
    for line in profile_content.lines() {
        if line.starts_with("- ") {
            index.push_str(line);
            index.push('\n');
        }
    }
    index.push('\n');

    index.push_str("## Statistics\n");
    index.push_str(&format!("- Total sessions: {}\n", sessions.len()));
    index.push_str(&format!("- Total exchanges: {}\n", total_exchanges));
    index.push('\n');

    if !sessions.is_empty() {
        index.push_str("## Recent Sessions\n");
        for (name, count) in sessions.iter().rev().take(10) {
            let date = name.trim_end_matches(".md");
            index.push_str(&format!(
                "- [{}](sessions/{}) - {} exchanges\n",
                date, name, count
            ));
        }
    }

    fs::write(index_path(), index).await
}

pub async fn search_history(keyword: &str, limit: usize) -> Vec<String> {
    let sessions_path = sessions_dir();
    let mut results = Vec::new();

    if let Ok(mut entries) = fs::read_dir(&sessions_path).await {
        let mut files: Vec<_> = Vec::new();

        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(metadata) = entry.metadata().await
                && metadata.is_file()
            {
                files.push(entry.path());
            }
        }

        files.sort_by(|a, b| b.cmp(a));

        'outer: for path in files {
            if let Ok(content) = fs::read_to_string(&path).await {
                for block in content.split("---") {
                    if block.to_lowercase().contains(&keyword.to_lowercase()) {
                        let lines: Vec<&str> =
                            block.lines().filter(|l| !l.is_empty()).take(5).collect();

                        if !lines.is_empty() {
                            results.push(lines.join("\n"));
                            if results.len() >= limit {
                                break 'outer;
                            }
                        }
                    }
                }
            }
        }
    }
    results
}

pub async fn search_related(query: &str, limit: usize) -> Vec<String> {
    let results = search_history(query, limit * 3).await;

    if results.is_empty() {
        return results;
    }

    let stop_words = [
        "the", "and", "or", "a", "an", "in", "on", "at", "to", "of", "是", "的", "了",
    ];
    let query_lower = query.to_lowercase();
    let query_words: Vec<_> = query_lower
        .split_whitespace()
        .filter(|w| w.len() > 1 && !stop_words.contains(w))
        .collect();

    if query_words.is_empty() {
        return results.into_iter().take(limit).collect();
    }

    let mut scored: Vec<(usize, String)> = results
        .into_iter()
        .map(|r| {
            let content = r.to_lowercase();
            let score = query_words.iter().filter(|w| content.contains(*w)).count();
            (score, r)
        })
        .filter(|(s, _)| *s > 0)
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().take(limit).map(|(_, r)| r).collect()
}

pub fn extract_memory(text: &str) -> Vec<(String, String)> {
    let Ok(re) = Regex::new(r"(?s)<memory>(.*?)</memory>") else {
        return Vec::new();
    };
    let mut memories = Vec::new();

    for cap in re.captures_iter(text) {
        let content = cap[1].trim();
        if let Some(colon_pos) = content.find(':') {
            let key = content[..colon_pos].trim().to_string();
            let value = content[colon_pos + 1..].trim().to_string();
            memories.push((key, value));
        }
    }
    memories
}

pub fn extract_topic(text: &str) -> Option<String> {
    let re = Regex::new(r"(?s)<topic>(.*?)</topic>").ok()?;
    re.captures(text).map(|cap| cap[1].trim().to_string())
}

pub fn normalize_tag(tag: &str) -> String {
    tag.to_lowercase()
        .replace(|c: char| c.is_whitespace(), "_")
        .replace(|c: char| !c.is_alphanumeric() && c != '_', "")
        .trim_matches('_')
        .to_string()
}

pub async fn log_to_topic(
    raw_topic: &str,
    user_msg: &str,
    lain_reply: &str,
) -> std::io::Result<()> {
    let topic = normalize_tag(raw_topic);

    if topic.len() < 2 {
        return Ok(());
    }

    let stop_words = ["the", "and", "or", "a", "an", "in", "on", "at", "to"];
    if stop_words.contains(&topic.as_str()) {
        return Ok(());
    }

    let mut path = memory_dir();
    path.push("topics");
    fs::create_dir_all(&path).await?;
    path.push(format!("{}.md", topic));

    let now = Local::now().format("%H:%M:%S").to_string();
    let content = format!(
        "## {}\n**User**: {}\n\n**Lain**: {}\n\n---\n\n",
        now, user_msg, lain_reply
    );

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .await?;

    file.write_all(content.as_bytes()).await
}

pub fn strip_all_tags(text: &str) -> String {
    Regex::new(r"(?s)<(?:memory|topic)>.*?</(?:memory|topic)>")
        .map(|re| re.replace_all(text, "").to_string().trim().to_string())
        .unwrap_or_else(|_| text.trim().to_string())
}
