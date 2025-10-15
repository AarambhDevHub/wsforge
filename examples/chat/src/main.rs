use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wsforge::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChatMessage {
    username: String,
    message: String,
    timestamp: u64,
}

#[derive(Debug, Clone)]
struct ChatRoom {
    _name: String,
}

async fn chat_handler(
    Json(msg): Json<ChatMessage>,
    conn: Connection,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<()> {
    println!("💬 {} says: {}", msg.username, msg.message);

    let broadcast_msg = ChatMessage {
        username: msg.username.clone(),
        message: msg.message.clone(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    let json = serde_json::to_string(&broadcast_msg).unwrap();
    manager.broadcast_except(conn.id(), Message::text(json));

    Ok(())
}

async fn broadcast_handler(
    Json(msg): Json<ChatMessage>,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<String> {
    let json = serde_json::to_string(&msg).unwrap();
    manager.broadcast(Message::text(json));
    Ok("Broadcast sent".to_string())
}

async fn stats_handler(
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<JsonResponse<serde_json::Value>> {
    let stats = serde_json::json!({
        "total_connections": manager.count(),
        "connection_ids": manager.all_ids(),
    });
    Ok(JsonResponse(stats))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let manager = Arc::new(ConnectionManager::new());

    let router = Router::new()
        .with_state(manager.clone())
        .with_state(Arc::new(ChatRoom {
            _name: "General".to_string(),
        }))
        .route("/chat", handler(chat_handler))
        .route("/broadcast", handler(broadcast_handler))
        .route("/stats", handler(stats_handler))
        .on_connect(move |manager, conn_id| {
            println!("✅ User joined the chat: {}", conn_id);
            let welcome = ChatMessage {
                username: "System".to_string(),
                message: format!("User {} joined the chat", conn_id),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            if let Ok(json) = serde_json::to_string(&welcome) {
                manager.broadcast(Message::text(json));
            }
        })
        .on_disconnect(|_manager, conn_id| {
            println!("❌ User left the chat: {}", conn_id);
        });

    println!("💬 Chat server running on ws://127.0.0.1:9000");
    println!(
        "📊 Send JSON: {{ \"username\": \"Alice\", \"message\": \"Hello!\", \"timestamp\": 0 }}"
    );

    router.listen("127.0.0.1:9000").await?;

    Ok(())
}
