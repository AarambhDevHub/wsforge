# Migration Guide

This guide helps you migrate to WsForge from other WebSocket frameworks and libraries.

## Table of Contents

- [Migrating from tokio-tungstenite](#migrating-from-tokio-tungstenite)
- [Migrating from warp](#migrating-from-warp)
- [Migrating from actix-web](#migrating-from-actix-web)
- [Migrating from Socket.io (Node.js)](#migrating-from-socketio-nodejs)
- [Migrating from ws (Node.js)](#migrating-from-ws-nodejs)
- [Migrating from Python websockets](#migrating-from-python-websockets)
- [Version Migration (0.x to 1.0)](#version-migration)
- [Breaking Changes](#breaking-changes)
- [Common Patterns](#common-patterns)

---

## Migrating from tokio-tungstenite

### Before (tokio-tungstenite)

```
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tokio::net::TcpListener;
use futures_util::{StreamExt, SinkExt};

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(async move {
            let ws_stream = accept_async(stream).await.unwrap();
            let (mut write, mut read) = ws_stream.split();

            while let Some(msg) = read.next().await {
                if let Ok(msg) = msg {
                    if msg.is_text() || msg.is_binary() {
                        write.send(msg).await.unwrap();
                    }
                }
            }
        });
    }
}
```

### After (WsForge)

```
use wsforge::prelude::*;

async fn echo(msg: Message) -> Result<Message> {
    Ok(msg)
}

#[tokio::main]
async fn main() -> Result<()> {
    Router::new()
        .default_handler(handler(echo))
        .listen("127.0.0.1:8080")
        .await
}
```

### Key Differences

1. **No Manual Connection Handling**: WsForge manages connections automatically
2. **Handler Functions**: Write simple async functions instead of managing streams
3. **Type Safety**: Built-in extractors and type-safe routing
4. **Broadcasting Built-in**: No need to implement connection tracking

### Connection Management

**Before:**
```
// Manual connection tracking
let connections = Arc::new(Mutex::new(HashMap::new()));
```

**After:**
```
// Automatic via ConnectionManager
State(manager): State<Arc<ConnectionManager>>
```

### Broadcasting

**Before:**
```
// Manual broadcasting
for (_, tx) in connections.lock().await.iter() {
    tx.send(msg.clone()).await.ok();
}
```

**After:**
```
// Built-in broadcasting
manager.broadcast(msg);
manager.broadcast_except(conn.id(), msg);
```

---

## Migrating from warp

### Before (warp)

```
use warp::Filter;

#[tokio::main]
async fn main() {
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| {
            ws.on_upgrade(|websocket| async {
                let (tx, mut rx) = websocket.split();
                while let Some(result) = rx.next().await {
                    if let Ok(msg) = result {
                        tx.send(msg).await.ok();
                    }
                }
            })
        });

    warp::serve(ws_route).run((, 8080)).await;[9][10]
}
```

### After (WsForge)

```
use wsforge::prelude::*;

async fn echo(msg: Message) -> Result<Message> {
    Ok(msg)
}

#[tokio::main]
async fn main() -> Result<()> {
    Router::new()
        .route("/ws", handler(echo))
        .listen("127.0.0.1:8080")
        .await
}
```

### Routing Migration

**Before:**
```
let route1 = warp::path("echo").and(warp::ws());
let route2 = warp::path("chat").and(warp::ws());
let routes = route1.or(route2);
```

**After:**
```
Router::new()
    .route("/echo", handler(echo_handler))
    .route("/chat", handler(chat_handler))
```

---

## Migrating from actix-web

### Before (actix-web)

```
use actix::{Actor, StreamHandler};
use actix_web::{web, App, HttpServer, HttpRequest, HttpResponse};
use actix_web_actors::ws;

struct MyWebSocket;

impl Actor for MyWebSocket {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MyWebSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(text)) => ctx.text(text),
            _ => (),
        }
    }
}

async fn ws_index(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, actix_web::Error> {
    ws::start(MyWebSocket {}, &req, stream)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().route("/ws", web::get().to(ws_index))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

### After (WsForge)

```
use wsforge::prelude::*;

async fn ws_handler(msg: Message) -> Result<Message> {
    if msg.is_text() {
        Ok(msg)
    } else {
        Ok(Message::text("Only text messages"))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    Router::new()
        .route("/ws", handler(ws_handler))
        .listen("127.0.0.1:8080")
        .await
}
```

### State Management Migration

**Before:**
```
struct AppState {
    counter: Arc<Mutex<i32>>,
}

let state = web::Data::new(AppState {
    counter: Arc::new(Mutex::new(0)),
});
```

**After:**
```
struct AppState {
    counter: Arc<RwLock<i32>>,
}

Router::new()
    .with_state(Arc::new(AppState {
        counter: Arc::new(RwLock::new(0)),
    }))
```

---

## Migrating from Socket.io (Node.js)

### Before (Socket.io)

```
const io = require('socket.io')(3000);

io.on('connection', (socket) => {
  console.log('User connected');

  socket.on('message', (msg) => {
    io.emit('message', msg); // Broadcast to all
  });

  socket.on('disconnect', () => {
    console.log('User disconnected');
  });
});
```

### After (WsForge)

```
use wsforge::prelude::*;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct ChatMessage {
    text: String,
}

async fn chat_handler(
    Json(msg): Json<ChatMessage>,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<()> {
    let json = serde_json::to_string(&msg)?;
    manager.broadcast(Message::text(json));
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    Router::new()
        .default_handler(handler(chat_handler))
        .on_connect(|manager, conn_id| {
            println!("User connected: {}", conn_id);
        })
        .on_disconnect(|manager, conn_id| {
            println!("User disconnected: {}", conn_id);
        })
        .listen("127.0.0.1:3000")
        .await
}
```

### Event System Migration

**Socket.io Events → WsForge Routes:**

```
// Socket.io
socket.on('chat message', handleChat);
socket.on('user typing', handleTyping);
```

```
// WsForge
#[derive(Deserialize)]
#[serde(tag = "type")]
enum Event {
    #[serde(rename = "chat message")]
    ChatMessage { text: String },
    #[serde(rename = "user typing")]
    UserTyping { user: String },
}

async fn event_handler(Json(event): Json<Event>) -> Result<()> {
    match event {
        Event::ChatMessage { text } => { /* handle chat */ }
        Event::UserTyping { user } => { /* handle typing */ }
    }
    Ok(())
}
```

---

## Migrating from ws (Node.js)

### Before (ws)

```
const WebSocket = require('ws');
const wss = new WebSocket.Server({ port: 8080 });

wss.on('connection', (ws) => {
  ws.on('message', (data) => {
    // Broadcast to all clients
    wss.clients.forEach((client) => {
      if (client.readyState === WebSocket.OPEN) {
        client.send(data);
      }
    });
  });
});
```

### After (WsForge)

```
use wsforge::prelude::*;
use std::sync::Arc;

async fn broadcast_handler(
    msg: Message,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<()> {
    manager.broadcast(msg);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    Router::new()
        .default_handler(handler(broadcast_handler))
        .listen("127.0.0.1:8080")
        .await
}
```

---

## Migrating from Python websockets

### Before (Python websockets)

```
import asyncio
import websockets

connected = set()

async def handler(websocket):
    connected.add(websocket)
    try:
        async for message in websocket:
            for conn in connected:
                await conn.send(message)
    finally:
        connected.remove(websocket)

async def main():
    async with websockets.serve(handler, "localhost", 8080):
        await asyncio.Future()

asyncio.run(main())
```

### After (WsForge)

```
use wsforge::prelude::*;
use std::sync::Arc;

async fn handler(
    msg: Message,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<()> {
    manager.broadcast(msg);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    Router::new()
        .default_handler(handler(handler))
        .listen("127.0.0.1:8080")
        .await
}
```

---

## Version Migration

### From 0.1.x to 1.0.0 (Future)

**Breaking Changes Expected:**

1. **Handler Signature Changes**
   - Old: `fn(Message) -> Result<Message>`
   - New: Async trait-based handlers (TBD)

2. **State Access**
   - Old: Manual Arc wrapping
   - New: Automatic state management

3. **Error Types**
   - Consolidated error types
   - Better error context

**Migration Steps:**

1. Update `Cargo.toml`:
   ```
   wsforge = "1.0"
   ```

2. Run migration tool:
   ```
   cargo fix --edition-idioms
   ```

3. Update handler signatures as needed

4. Test thoroughly

---

## Breaking Changes

### Current Version (0.1.x)

No breaking changes yet (initial release).

### Planned for 1.0

- Stabilized API
- Handler trait refinements
- Improved error handling
- Enhanced type safety

---

## Common Patterns

### Pattern: Room-based Broadcasting

**Old Way (Manual):**
```
// Track rooms manually
let rooms: HashMap<String, Vec<ConnectionId>> = HashMap::new();
```

**WsForge Way:**
```
struct RoomManager {
    rooms: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

async fn send_to_room(
    room: &str,
    msg: Message,
    State(room_mgr): State<Arc<RoomManager>>,
    State(conn_mgr): State<Arc<ConnectionManager>>,
) -> Result<()> {
    let rooms = room_mgr.rooms.read().await;
    if let Some(members) = rooms.get(room) {
        conn_mgr.broadcast_to(members, msg);
    }
    Ok(())
}
```

### Pattern: Authentication

**Old Way:**
```
// Manual token validation in connection handler
```

**WsForge Way:**
```
async fn auth_handler(
    msg: Message,
    extensions: &Extensions,
) -> Result<String> {
    let token = msg.as_text().ok_or(Error::custom("No token"))?;

    // Validate token
    let user_id = validate_token(token)?;

    // Store in extensions
    extensions.insert("user_id", user_id);

    Ok("Authenticated".to_string())
}
```

### Pattern: Rate Limiting

**Old Way:**
```
// Manual rate limit tracking per connection
```

**WsForge Way:**
```
struct RateLimiter {
    limits: Arc<RwLock<HashMap<String, (u32, Instant)>>>,
}

async fn rate_limited_handler(
    msg: Message,
    conn: Connection,
    State(limiter): State<Arc<RateLimiter>>,
) -> Result<String> {
    let mut limits = limiter.limits.write().await;
    let (count, last_reset) = limits
        .entry(conn.id().clone())
        .or_insert((0, Instant::now()));

    if last_reset.elapsed() > Duration::from_secs(60) {
        *count = 0;
        *last_reset = Instant::now();
    }

    if *count >= 100 {
        return Err(Error::custom("Rate limit exceeded"));
    }

    *count += 1;
    Ok("Processed".to_string())
}
```

---

## Need Help?

- **GitHub Issues**: Report migration problems
- **Discussions**: Ask migration questions
- **Examples**: Check `examples/` directory
- **Documentation**: Read full docs at docs.rs/wsforge

## Contributing

Found a migration pattern that should be documented? Submit a PR to add it to this guide!
