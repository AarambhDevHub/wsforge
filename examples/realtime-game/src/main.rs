use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wsforge::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PlayerPosition {
    player_id: String,
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum GameMessage {
    #[serde(rename = "move")]
    Move { position: PlayerPosition },
    #[serde(rename = "shoot")]
    Shoot { target_id: String },
    #[serde(rename = "chat")]
    Chat { message: String },
}

async fn game_handler(
    Json(game_msg): Json<GameMessage>,
    conn: Connection,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<()> {
    match game_msg {
        GameMessage::Move { position } => {
            println!(
                "🎮 Player {} moved to ({}, {}, {})",
                position.player_id, position.x, position.y, position.z
            );

            let update = serde_json::json!({
                "type": "position_update",
                "player_id": position.player_id,
                "x": position.x,
                "y": position.y,
                "z": position.z,
            });

            let json = serde_json::to_string(&update).unwrap();
            manager.broadcast_except(conn.id(), Message::text(json));
        }
        GameMessage::Shoot { target_id } => {
            println!("💥 {} shot at {}", conn.id(), target_id);

            let event = serde_json::json!({
                "type": "shot_fired",
                "shooter_id": conn.id(),
                "target_id": target_id,
            });

            let json = serde_json::to_string(&event).unwrap();
            manager.broadcast(Message::text(json));
        }
        GameMessage::Chat { message } => {
            println!("💬 {}: {}", conn.id(), message);

            let chat = serde_json::json!({
                "type": "chat",
                "player_id": conn.id(),
                "message": message,
            });

            let json = serde_json::to_string(&chat).unwrap();
            manager.broadcast(Message::text(json));
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let manager = Arc::new(ConnectionManager::new());

    let router = Router::new()
        .with_state(manager.clone())
        .route("/game", handler(game_handler))
        .on_connect(move |manager, conn_id| {
            println!("🎮 Player joined: {}", conn_id);
            let spawn = serde_json::json!({
                "type": "player_joined",
                "player_id": conn_id,
            });
            if let Ok(json) = serde_json::to_string(&spawn) {
                manager.broadcast(Message::text(json));
            }
        })
        .on_disconnect(|_manager, conn_id| {
            println!("👋 Player left: {}", conn_id);
        });

    println!("🎮 Real-time game server running on ws://127.0.0.1:9001");
    router.listen("127.0.0.1:9001").await?;

    Ok(())
}
