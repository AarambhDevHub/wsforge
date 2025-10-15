# Examples

This guide provides practical examples for building various types of WebSocket applications with WsForge.

## Table of Contents

- [Echo Server](#echo-server)
- [Chat Application](#chat-application)
- [Real-Time Game Server](#real-time-game-server)
- [Web Application with UI](#web-application-with-ui)
- [Multi-Room Chat](#multi-room-chat)
- [Collaborative Editor](#collaborative-editor)
- [Real-Time Dashboard](#real-time-dashboard)
- [File Upload Server](#file-upload-server)
- [Notification System](#notification-system)
- [API Gateway](#api-gateway)

---

## Echo Server

The simplest WebSocket server that echoes back messages.

```
use wsforge::prelude::*;

async fn echo(msg: Message) -> Result<Message> {
    println!("Received: {:?}", msg.as_text());
    Ok(msg)
}

#[tokio::main]
async fn main() -> Result<()> {
    let router = Router::new()
        .default_handler(handler(echo));

    println!("🚀 Echo server: ws://127.0.0.1:8080");
    router.listen("127.0.0.1:8080").await?;
    Ok(())
}
```

**Test it:**
```
cargo run --example echo
```

Connect with WebSocket client and send messages - they'll echo back!

---

## Chat Application

Full-featured chat with broadcasting and user management.

```
use wsforge::prelude::*;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
struct ChatMessage {
    username: String,
    text: String,
    timestamp: u64,
}

#[derive(Serialize)]
struct SystemMessage {
    r#type: String,
    message: String,
    user_count: usize,
}

async fn chat_handler(
    Json(msg): Json<ChatMessage>,
    conn: Connection,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<()> {
    println!("{}: {}", msg.username, msg.text);

    // Broadcast to all users
    let json = serde_json::to_string(&msg)?;
    manager.broadcast(Message::text(json));

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let router = Router::new()
        .default_handler(handler(chat_handler))
        .on_connect(|manager, conn_id| {
            println!("✅ {} joined (Total: {})", conn_id, manager.count());

            // Send welcome message
            let welcome = SystemMessage {
                r#type: "system".to_string(),
                message: format!("User {} joined", conn_id),
                user_count: manager.count(),
            };

            if let Ok(json) = serde_json::to_string(&welcome) {
                manager.broadcast(Message::text(json));
            }
        })
        .on_disconnect(|manager, conn_id| {
            println!("❌ {} left (Total: {})", conn_id, manager.count());

            // Broadcast user left
            let goodbye = SystemMessage {
                r#type: "system".to_string(),
                message: format!("User {} left", conn_id),
                user_count: manager.count(),
            };

            if let Ok(json) = serde_json::to_string(&goodbye) {
                manager.broadcast(Message::text(json));
            }
        });

    println!("💬 Chat server: ws://127.0.0.1:8080");
    router.listen("127.0.0.1:8080").await?;
    Ok(())
}
```

**Message Format:**
```
{
  "username": "Alice",
  "text": "Hello everyone!",
  "timestamp": 1634567890
}
```

---

## Real-Time Game Server

Multiplayer game with position updates.

```
use wsforge::prelude::*;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
struct PlayerMove {
    player_id: String,
    x: f32,
    y: f32,
    action: String,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum GameMessage {
    #[serde(rename = "move")]
    Move { position: PlayerMove },
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
            println!("Player {} moved to ({}, {})",
                position.player_id, position.x, position.y);

            // Broadcast to other players
            let update = serde_json::json!({
                "type": "position_update",
                "player_id": position.player_id,
                "x": position.x,
                "y": position.y,
            });

            let json = serde_json::to_string(&update)?;
            manager.broadcast_except(conn.id(), Message::text(json));
        }
        GameMessage::Shoot { target_id } => {
            println!("Player shooting at {}", target_id);

            let shot = serde_json::json!({
                "type": "shot_fired",
                "shooter": conn.id(),
                "target": target_id,
            });

            manager.broadcast(Message::text(serde_json::to_string(&shot)?));
        }
        GameMessage::Chat { message } => {
            println!("Chat: {}", message);
            manager.broadcast_except(
                conn.id(),
                Message::text(serde_json::to_string(&serde_json::json!({
                    "type": "chat",
                    "message": message,
                }))?),
            );
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let router = Router::new()
        .default_handler(handler(game_handler))
        .on_connect(|manager, conn_id| {
            println!("🎮 Player {} joined", conn_id);
        });

    println!("🎮 Game server: ws://127.0.0.1:8080");
    router.listen("0.0.0.0:8080").await?;
    Ok(())
}
```

---

## Web Application with UI

Hybrid HTTP/WebSocket server serving static files.

**Directory Structure:**
```
chat-web/
├── src/
│   └── main.rs
└── static/
    ├── index.html
    ├── app.js
    └── style.css
```

**src/main.rs:**
```
use wsforge::prelude::*;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct Message {
    username: String,
    text: String,
}

async fn ws_handler(
    Json(msg): Json<Message>,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<()> {
    let json = serde_json::to_string(&msg)?;
    manager.broadcast(Message::text(json));
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let router = Router::new()
        .serve_static("static")  // Serve HTML/CSS/JS
        .default_handler(handler(ws_handler));

    println!("🌐 Server: http://127.0.0.1:8080");
    router.listen("127.0.0.1:8080").await?;
    Ok(())
}
```

**static/app.js:**
```
const ws = new WebSocket('ws://localhost:8080');

ws.onmessage = (event) => {
    const msg = JSON.parse(event.data);
    displayMessage(msg.username, msg.text);
};

function sendMessage(username, text) {
    ws.send(JSON.stringify({ username, text }));
}
```

---

## Multi-Room Chat

Chat with multiple rooms and room management.

```
use wsforge::prelude::*;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct RoomManager {
    rooms: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl RoomManager {
    fn new() -> Self {
        Self {
            rooms: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn join_room(&self, room: String, conn_id: String) {
        let mut rooms = self.rooms.write().await;
        rooms.entry(room).or_insert_with(Vec::new).push(conn_id);
    }

    async fn leave_room(&self, room: &str, conn_id: &str) {
        let mut rooms = self.rooms.write().await;
        if let Some(members) = rooms.get_mut(room) {
            members.retain(|id| id != conn_id);
        }
    }

    async fn get_room_members(&self, room: &str) -> Vec<String> {
        self.rooms.read().await
            .get(room)
            .cloned()
            .unwrap_or_default()
    }
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum RoomMessage {
    #[serde(rename = "join")]
    Join { room: String },
    #[serde(rename = "leave")]
    Leave { room: String },
    #[serde(rename = "message")]
    Message { room: String, text: String },
}

async fn room_handler(
    Json(msg): Json<RoomMessage>,
    conn: Connection,
    State(room_mgr): State<Arc<RoomManager>>,
    State(conn_mgr): State<Arc<ConnectionManager>>,
) -> Result<()> {
    match msg {
        RoomMessage::Join { room } => {
            room_mgr.join_room(room.clone(), conn.id().to_string()).await;
            println!("{} joined room {}", conn.id(), room);
        }
        RoomMessage::Leave { room } => {
            room_mgr.leave_room(&room, conn.id()).await;
            println!("{} left room {}", conn.id(), room);
        }
        RoomMessage::Message { room, text } => {
            let members = room_mgr.get_room_members(&room).await;
            let json = serde_json::to_string(&serde_json::json!({
                "room": room,
                "text": text,
                "from": conn.id(),
            }))?;

            conn_mgr.broadcast_to(&members, Message::text(json));
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let room_mgr = Arc::new(RoomManager::new());

    let router = Router::new()
        .with_state(room_mgr)
        .default_handler(handler(room_handler));

    println!("🏠 Multi-room chat: ws://127.0.0.1:8080");
    router.listen("127.0.0.1:8080").await?;
    Ok(())
}
```

---

## Collaborative Editor

Real-time collaborative document editing.

```
use wsforge::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct Document {
    content: Arc<RwLock<String>>,
}

impl Document {
    fn new() -> Self {
        Self {
            content: Arc::new(RwLock::new(String::new())),
        }
    }

    async fn apply_edit(&self, edit: &Edit) -> String {
        let mut content = self.content.write().await;
        content.replace_range(edit.start..edit.end, &edit.text);
        content.clone()
    }

    async fn get_content(&self) -> String {
        self.content.read().await.clone()
    }
}

#[derive(Deserialize, Serialize)]
struct Edit {
    start: usize,
    end: usize,
    text: String,
    user: String,
}

async fn edit_handler(
    Json(edit): Json<Edit>,
    conn: Connection,
    State(doc): State<Arc<Document>>,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<()> {
    // Apply edit
    let new_content = doc.apply_edit(&edit).await;

    // Broadcast to others
    let update = serde_json::to_string(&edit)?;
    manager.broadcast_except(conn.id(), Message::text(update));

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let document = Arc::new(Document::new());

    let router = Router::new()
        .with_state(document)
        .default_handler(handler(edit_handler));

    println!("📝 Collaborative editor: ws://127.0.0.1:8080");
    router.listen("127.0.0.1:8080").await?;
    Ok(())
}
```

---

## Real-Time Dashboard

Live metrics and monitoring dashboard.

```
use wsforge::prelude::*;
use std::sync::Arc;
use serde::Serialize;
use tokio::time::{interval, Duration};

#[derive(Serialize)]
struct Metrics {
    cpu: f32,
    memory: f32,
    connections: usize,
    timestamp: u64,
}

async fn metrics_broadcaster(manager: Arc<ConnectionManager>) {
    let mut ticker = interval(Duration::from_secs(1));

    loop {
        ticker.tick().await;

        let metrics = Metrics {
            cpu: rand::random::<f32>() * 100.0,
            memory: rand::random::<f32>() * 100.0,
            connections: manager.count(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        if let Ok(json) = serde_json::to_string(&metrics) {
            manager.broadcast(Message::text(json));
        }
    }
}

async fn dashboard_handler(_msg: Message) -> Result<()> {
    // Just keep connection alive
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let router = Router::new()
        .default_handler(handler(dashboard_handler))
        .serve_static("dashboard");

    let manager = router.connection_manager();

    // Spawn metrics broadcaster
    tokio::spawn(async move {
        metrics_broadcaster(manager).await;
    });

    println!("📊 Dashboard: http://127.0.0.1:8080");
    router.listen("127.0.0.1:8080").await?;
    Ok(())
}
```

---

## Running Examples

All examples are in the `examples/` directory:

```
# Echo server
cargo run --example echo

# Chat application
cargo run --example chat

# Web chat with UI
cargo run --example chat-web

# Real-time game
cargo run --example realtime-game
```

## Next Steps

- Check [Handlers](handlers.md) for handler patterns
- Learn about [Broadcasting](broadcasting.md)
- Explore [State Management](state-management.md)
