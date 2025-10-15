use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wsforge::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChatMessage {
    username: String,
    message: String,
    timestamp: u64,
    msg_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct StatsMessage {
    r#type: String,
    count: usize,
}

async fn chat_handler(msg: Message, State(manager): State<Arc<ConnectionManager>>) -> Result<()> {
    if let Ok(chat_msg) = msg.json::<ChatMessage>() {
        println!(
            "💬 {} says: {} [Broadcasting to {} users]",
            chat_msg.username,
            chat_msg.message,
            manager.count()
        );

        let broadcast_msg = ChatMessage {
            username: chat_msg.username,
            message: chat_msg.message,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            msg_type: "user".to_string(),
        };

        let json = serde_json::to_string(&broadcast_msg).unwrap();
        manager.broadcast(Message::text(json));
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let router = Router::new()
        .serve_static("examples/chat-web/static")
        .default_handler(handler(chat_handler))
        .on_connect(|manager, conn_id| {
            let count = manager.count();
            println!("✅ User joined: {} (Total: {})", conn_id, count);

            let join_msg = ChatMessage {
                username: "System".to_string(),
                message: format!("User {} joined", conn_id),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                msg_type: "join".to_string(),
            };

            if let Ok(json) = serde_json::to_string(&join_msg) {
                manager.broadcast(Message::text(json));
            }

            let stats = StatsMessage {
                r#type: "stats".to_string(),
                count,
            };

            if let Ok(json) = serde_json::to_string(&stats) {
                manager.broadcast(Message::text(json));
            }
        })
        .on_disconnect(|manager, conn_id| {
            let count = manager.count();
            println!("❌ User left: {} (Remaining: {})", conn_id, count);

            let leave_msg = ChatMessage {
                username: "System".to_string(),
                message: format!("User {} left", conn_id),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                msg_type: "leave".to_string(),
            };

            if let Ok(json) = serde_json::to_string(&leave_msg) {
                manager.broadcast(Message::text(json));
            }

            let stats = StatsMessage {
                r#type: "stats".to_string(),
                count,
            };

            if let Ok(json) = serde_json::to_string(&stats) {
                manager.broadcast(Message::text(json));
            }
        });

    println!("🚀 Chat server: http://127.0.0.1:8080");
    router.listen("127.0.0.1:8080").await?;

    Ok(())
}
