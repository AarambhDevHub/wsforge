# WsForge API Reference

Complete reference for all public APIs in the WsForge WebSocket framework.

## Table of Contents

- [Core Types](#core-types)
- [Router](#router)
- [Connection & ConnectionManager](#connection--connectionmanager)
- [Message & MessageType](#message--messagetype)
- [Handlers](#handlers)
- [Extractors](#extractors)
- [State Management](#state-management)
- [Error Types](#error-types)
- [Static Files](#static-files)
- [Macros](#macros)

---

## Core Types

### `Result<T>`

Type alias for `std::result::Result<T, Error>`.

```
pub type Result<T> = std::result::Result<T, Error>;
```

### `ConnectionId`

Type alias for connection identifiers.

```
pub type ConnectionId = String;
```

---

## Router

The main entry point for building WebSocket servers.

### `Router::new() -> Self`

Creates a new empty router.

```
let router = Router::new();
```

### `Router::route(self, path: impl Into<String>, handler: Arc<dyn Handler>) -> Self`

Registers a handler for a specific route pattern.

**Parameters:**
- `path` - Route path (e.g., "/chat", "/api")
- `handler` - Handler wrapped with `handler()`

**Example:**
```
async fn chat_handler(msg: Message) -> Result<String> {
    Ok("chat response".to_string())
}

let router = Router::new()
    .route("/chat", handler(chat_handler));
```

### `Router::with_state<T: Send + Sync + 'static>(self, data: T) -> Self`

Adds shared application state.

**Parameters:**
- `data` - State data (must be `Send + Sync + 'static`)

**Example:**
```
struct Database { /* ... */ }

let router = Router::new()
    .with_state(Arc::new(Database::new()));
```

### `Router::default_handler(self, handler: Arc<dyn Handler>) -> Self`

Sets the default handler for messages that don't match any route.

**Example:**
```
let router = Router::new()
    .default_handler(handler(my_handler));
```

### `Router::serve_static(self, path: impl Into<PathBuf>) -> Self`

Enables static file serving from a directory.

**Parameters:**
- `path` - Directory path containing static files

**Example:**
```
let router = Router::new()
    .serve_static("public");
```

### `Router::on_connect<F>(self, f: F) -> Self`

Sets callback for when connections are established.

**Signature:**
```
where F: Fn(&Arc<ConnectionManager>, ConnectionId) + Send + Sync + 'static
```

**Example:**
```
let router = Router::new()
    .on_connect(|manager, conn_id| {
        println!("Connected: {}", conn_id);
    });
```

### `Router::on_disconnect<F>(self, f: F) -> Self`

Sets callback for when connections are closed.

**Example:**
```
let router = Router::new()
    .on_disconnect(|manager, conn_id| {
        println!("Disconnected: {}", conn_id);
    });
```

### `Router::connection_manager(&self) -> Arc<ConnectionManager>`

Returns a reference to the connection manager.

**Example:**
```
let manager = router.connection_manager();
println!("Active: {}", manager.count());
```

### `Router::listen(self, addr: impl AsRef<str>) -> Result<()>`

Starts the WebSocket server (async).

**Parameters:**
- `addr` - Bind address (e.g., "127.0.0.1:8080")

**Example:**
```
router.listen("0.0.0.0:8080").await?;
```

---

## Connection & ConnectionManager

### Connection

Represents an active WebSocket connection.

#### `Connection::new(id: ConnectionId, addr: SocketAddr, sender: UnboundedSender<Message>) -> Self`

Creates a new connection (typically internal use).

#### `Connection::send(&self, message: Message) -> Result<()>`

Sends a message to the client.

**Example:**
```
conn.send(Message::text("Hello!"))?;
```

#### `Connection::send_text(&self, text: impl Into<String>) -> Result<()>`

Sends a text message.

**Example:**
```
conn.send_text("Hello, client!")?;
```

#### `Connection::send_binary(&self, data: Vec<u8>) -> Result<()>`

Sends binary data.

**Example:**
```
conn.send_binary(vec![0x01, 0x02, 0x03])?;
```

#### `Connection::send_json<T: Serialize>(&self, data: &T) -> Result<()>`

Serializes and sends JSON data.

**Example:**
```
#[derive(Serialize)]
struct Response { status: String }

conn.send_json(&Response { status: "ok".to_string() })?;
```

#### `Connection::id(&self) -> &ConnectionId`

Returns the connection ID.

#### `Connection::info(&self) -> &ConnectionInfo`

Returns connection metadata.

### ConnectionManager

Manages all active connections.

#### `ConnectionManager::new() -> Self`

Creates a new connection manager.

#### `ConnectionManager::add(&self, conn: Connection) -> usize`

Adds a connection and returns total count.

#### `ConnectionManager::remove(&self, id: &ConnectionId) -> Option<Connection>`

Removes a connection by ID.

#### `ConnectionManager::get(&self, id: &ConnectionId) -> Option<Connection>`

Retrieves a connection by ID.

**Example:**
```
if let Some(conn) = manager.get(&conn_id) {
    conn.send_text("message")?;
}
```

#### `ConnectionManager::broadcast(&self, message: Message)`

Broadcasts message to all connections.

**Example:**
```
manager.broadcast(Message::text("Announcement!"));
```

#### `ConnectionManager::broadcast_except(&self, except_id: &ConnectionId, message: Message)`

Broadcasts to all except one connection.

**Example:**
```
manager.broadcast_except(&sender_id, msg);
```

#### `ConnectionManager::broadcast_to(&self, ids: &[ConnectionId], message: Message)`

Broadcasts to specific connections.

**Example:**
```
let room_members = vec!["conn_1".to_string(), "conn_2".to_string()];
manager.broadcast_to(&room_members, msg);
```

#### `ConnectionManager::count(&self) -> usize`

Returns the number of active connections.

#### `ConnectionManager::all_ids(&self) -> Vec<ConnectionId>`

Returns all connection IDs.

#### `ConnectionManager::all_connections(&self) -> Vec<Connection>`

Returns clones of all connections.

---

## Message & MessageType

### Message

WebSocket message type.

#### `Message::text(text: impl Into<String>) -> Self`

Creates a text message.

```
let msg = Message::text("Hello");
```

#### `Message::binary(data: Vec<u8>) -> Self`

Creates a binary message.

```
let msg = Message::binary(vec!);[11][12]
```

#### `Message::ping(data: Vec<u8>) -> Self`

Creates a ping frame.

#### `Message::pong(data: Vec<u8>) -> Self`

Creates a pong frame.

#### `Message::close() -> Self`

Creates a close frame.

#### `Message::is_text(&self) -> bool`

Checks if message is text.

#### `Message::is_binary(&self) -> bool`

Checks if message is binary.

#### `Message::is_close(&self) -> bool`

Checks if message is close frame.

#### `Message::as_text(&self) -> Option<&str>`

Returns text content if text message.

```
if let Some(text) = msg.as_text() {
    println!("Text: {}", text);
}
```

#### `Message::as_bytes(&self) -> &[u8]`

Returns raw bytes (works for all types).

#### `Message::json<T: DeserializeOwned>(&self) -> Result<T>`

Deserializes message as JSON.

```
#[derive(Deserialize)]
struct Data { name: String }

let data: Data = msg.json()?;
```

### MessageType

Message type enum.

```
pub enum MessageType {
    Text,
    Binary,
    Ping,
    Pong,
    Close,
}
```

---

## Handlers

### `handler<F, T>(f: F) -> Arc<dyn Handler>`

Converts an async function into a handler.

**Example:**
```
async fn my_handler(msg: Message) -> Result<String> {
    Ok("response".to_string())
}

let h = handler(my_handler);
```

### Handler Trait

```
#[async_trait]
pub trait Handler: Send + Sync + 'static {
    async fn call(
        &self,
        message: Message,
        conn: Connection,
        state: AppState,
        extensions: Extensions,
    ) -> Result<Option<Message>>;
}
```

### IntoResponse Trait

Types that can be returned from handlers.

**Implementations:**
- `()` - No response
- `String` - Text message
- `&str` - Text message
- `Message` - Raw message
- `Vec<u8>` - Binary message
- `JsonResponse<T>` - JSON response
- `Result<T>` - Automatic error handling

### JsonResponse<T>

Wrapper for JSON responses.

```
#[derive(Serialize)]
struct Response { status: String }

async fn handler() -> Result<JsonResponse<Response>> {
    Ok(JsonResponse(Response { status: "ok".to_string() }))
}
```

---

## Extractors

### Json<T>

Extracts and deserializes JSON from messages.

```
#[derive(Deserialize)]
struct Request { name: String }

async fn handler(Json(req): Json<Request>) -> Result<String> {
    Ok(format!("Hello, {}", req.name))
}
```

### State<T>

Extracts shared application state.

```
async fn handler(State(db): State<Arc<Database>>) -> Result<String> {
    // Use database
    Ok("result".to_string())
}
```

### Connection

Extracts the active connection.

```
async fn handler(conn: Connection) -> Result<()> {
    println!("From: {}", conn.id());
    Ok(())
}
```

### ConnectInfo

Extracts connection metadata.

```
async fn handler(ConnectInfo(info): ConnectInfo) -> Result<String> {
    Ok(format!("Connected from: {}", info.addr))
}
```

### Data

Extracts raw binary data.

```
async fn handler(Data(bytes): Data) -> Result<String> {
    Ok(format!("Received {} bytes", bytes.len()))
}
```

### Message

Extracts the raw message.

```
async fn handler(msg: Message) -> Result<Message> {
    Ok(msg)
}
```

---

## State Management

### AppState

Type-safe container for shared state.

#### `AppState::new() -> Self`

Creates empty state.

#### `AppState::insert<T: Send + Sync + 'static>(&self, value: Arc<T>)`

Inserts state data.

```
let state = AppState::new();
state.insert(Arc::new(Database::new()));
```

#### `AppState::get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>>`

Retrieves state data.

```
if let Some(db) = state.get::<Database>() {
    // Use database
}
```

### Extensions

Request-scoped data container.

#### `Extensions::new() -> Self`

Creates empty extensions.

#### `Extensions::insert<T>(&self, key: impl Into<String>, value: T)`

Inserts extension data.

```
extensions.insert("user_id", 123_u64);
```

#### `Extensions::get<T>(&self, key: &str) -> Option<Arc<T>>`

Retrieves extension data.

---

## Error Types

### Error

Main error enum.

```
pub enum Error {
    WebSocket(tokio_tungstenite::tungstenite::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
    ConnectionNotFound(String),
    RouteNotFound(String),
    InvalidMessage,
    Handler(String),
    Extractor(String),
    Custom(String),
}
```

#### `Error::custom<T: Display>(msg: T) -> Self`

Creates custom error.

```
return Err(Error::custom("Something went wrong"));
```

#### `Error::handler<T: Display>(msg: T) -> Self`

Creates handler error.

#### `Error::extractor<T: Display>(msg: T) -> Self`

Creates extractor error.

---

## Static Files

### StaticFileHandler

Serves static files.

#### `StaticFileHandler::new(root: impl Into<PathBuf>) -> Self`

Creates handler for directory.

```
let handler = StaticFileHandler::new("public");
```

#### `StaticFileHandler::serve(&self, path: &str) -> Result<(Vec<u8>, String)>`

Serves a file (returns content and MIME type).

---

## Macros

### `#[websocket_handler]`

Attribute macro for handler functions.

```
#[websocket_handler]
async fn my_handler(msg: Message) -> Result<String> {
    Ok("response".to_string())
}
```

### `#[derive(WebSocketMessage)]`

Derives message conversion methods.

```
#[derive(WebSocketMessage, Serialize, Deserialize)]
struct ChatMsg {
    text: String,
}
```

### `#[derive(WebSocketHandler)]`

Derives Handler trait implementation.

```
#[derive(WebSocketHandler)]
struct MyHandler;
```

---

## Type Signatures Quick Reference

```
// Handler signatures
async fn handler1() -> Result<String>
async fn handler2(msg: Message) -> Result<Message>
async fn handler3(Json(data): Json<T>) -> Result<JsonResponse<R>>
async fn handler4(msg: Message, conn: Connection) -> Result<()>
async fn handler5(State(s): State<Arc<T>>) -> Result<String>

// Multiple extractors
async fn handler6(
    Json(data): Json<Request>,
    conn: Connection,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<JsonResponse<Response>>
```

---

## Version Information

**Current Version:** 0.1.0
**Minimum Rust Version:** 1.70
**Documentation:** https://docs.rs/wsforge
**Repository:** https://github.com/aarambhdevhub/wsforge
