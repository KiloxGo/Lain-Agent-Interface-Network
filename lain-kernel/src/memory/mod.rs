use chrono::Local;
use dirs::home_dir;
use regex::Regex;
use std::path::PathBuf;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;

fn session_dir() -> PathBuf {
    let mut path = home_dir().expect("无法获取 HOME 目录");

    path.push(".lain");
    path.push("memory");
    path.push("sessions");

    path
}

pub async fn log_exchange(user_msg: &str, lain_reply: &str) -> std::io::Result<()> {
    let mut path = session_dir();

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

    Ok(())
}

pub fn extract_memory(text: &str) -> Vec<(String, String)> {
    let re = Regex::new(r"<memory>(.*?)</memory>").unwrap();
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

pub fn strip_memory_tags(text: &str) -> String {
    let re = Regex::new(r"<memory>.*?</memory>").unwrap();
    re.replace_all(text, "").to_string()
}

fn profile_path() -> PathBuf {
    home_dir()
        .expect("无法获取 HOME 路径")
        .join(".lain/memory/user/profile.md")
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
    fs::write(&path, content).await
}

pub async fn load_profile() -> Option<String> {
    let path = profile_path();
    if path.exists() {
        fs::read_to_string(&path).await.ok()
    } else {
        None
    }
}
