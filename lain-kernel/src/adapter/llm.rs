use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

use crate::memory;

pub struct LlmClient {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
    system_prompt: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChatMessage,
}

impl LlmClient {
    pub fn new(system_prompt: &str) -> Self {
        let api_key = env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY not set in .env");

        Self {
            client: Client::new(),
            api_key,
            base_url: "https://api.deepseek.com/v1".to_string(),
            model: "deepseek-chat".to_string(),
            system_prompt: system_prompt.to_string(),
        }
    }

    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String, reqwest::Error> {
        let profile = memory::load_profile().await.unwrap_or_default();

        let system_content = if profile.is_empty() {
            self.system_prompt.clone()
        } else {
            format!("{}\n\n##User Context\n{}", self.system_prompt, profile)
        };

        let mut all_messages = vec![ChatMessage {
            role: "system".to_string(),
            content: system_content,
        }];
        all_messages.extend(messages);

        let request = ChatRequest {
            model: self.model.clone(),
            messages: all_messages,
            stream: false,
        };
        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?
            .json::<ChatResponse>()
            .await?;

        Ok(response.choices[0].message.content.clone())
    }
}
