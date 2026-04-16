mod adapter;
mod config;

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
use serde::Serialize;

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

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
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
                msgs.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: reply.clone(),
                });
                let msg_count = msgs.len();
                if msg_count > 10 {
                    let new_msgs = msgs.split_off(msg_count - 10);
                    msgs = new_msgs;
                }
                let _ = socket.send(Message::Text(reply.into())).await;
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
