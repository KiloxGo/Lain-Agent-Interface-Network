mod adapter;
mod config;
mod memory;

use adapter::llm::{ChatMessage, LlmClient};
use axum::{
    Json, Router,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
    routing::get,
};
use serde::{Serialize, de::value};

#[derive(Serialize)]
struct HealthResponse {
    status: String,
}

#[derive(Clone)]
struct AppState {
    system_prompt: String,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let client = LlmClient::new(&state.system_prompt);
    let mut msgs = Vec::new();

    while let Some(Ok(Message::Text(text))) = socket.recv().await {
        msgs.push(ChatMessage {
            role: "user".to_string(),
            content: text.to_string(),
        });

        match client.chat(msgs.clone()).await {
            Ok(reply) => {
                let memories = memory::extract_memory(&reply);
                for (key, value) in memories {
                    let _ = memory::update_profile(&key, &value).await;
                }

                let clean_reply = memory::strip_memory_tags(&reply);

                msgs.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: clean_reply.clone(),
                });
                let msg_count = msgs.len();
                if msg_count > 10 {
                    let new_msgs = msgs.split_off(msg_count - 10);
                    msgs = new_msgs;
                }

                let _ = socket.send(Message::Text(clean_reply.into())).await;

                let user_text = text.to_string();
                let lain_text = reply.clone();
                tokio::spawn(async move {
                    if let Err(e) = crate::memory::log_exchange(&user_text, &lain_text).await {
                        eprintln!("记忆写入失败:{}", e);
                    }
                });
            }
            Err(e) => {
                let error_msg = format!("...Noise...{}", e);
                let _ = socket.send(Message::Text(error_msg.into())).await;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let config = config::Config::load();
    let state = AppState {
        system_prompt: config.system_prompt,
    };

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/ws", get(ws_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
