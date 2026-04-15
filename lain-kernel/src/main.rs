mod adapter;

use adapter::llm::{ChatMessage, LlmClient};
use axum::{
    Json, Router,
    extract::{
        WebSocketUpgrade,
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

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    let client = LlmClient::new();

    while let Some(Ok(Message::Text(text))) = socket.recv().await {
        let msg = ChatMessage {
            role: "user".to_string(),
            content: text.to_string(),
        };
        match client.chat(vec![msg]).await {
            Ok(reply) => {
                let _ = socket.send(Message::Text(reply.into())).await;
            }
            Err(e) => {
                let error_msg = format!("Lain 想不通...{}", e);
                let _ = socket.send(Message::Text(error_msg.into())).await;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let app = Router::new()
        .route("/api/health", get(health))
        .route("/ws", get(ws_handler));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
