# Handlers Guide

Handlers are the heart of your WsForge application. They process incoming WebSocket messages and return responses.

## Table of Contents

- [What are Handlers?](#what-are-handlers)
- [Basic Handler](#basic-handler)
- [Handler Signatures](#handler-signatures)
- [Return Types](#return-types)
- [Using Extractors](#using-extractors)
- [Multiple Extractors](#multiple-extractors)
- [Error Handling](#error-handling)
- [Advanced Patterns](#advanced-patterns)
- [Best Practices](#best-practices)

## What are Handlers?

Handlers are async functions that:
- Receive WebSocket messages
- Process data using type-safe extractors
- Return responses or perform actions
- Handle errors gracefully

## Basic Handler

The simplest handler just returns a response:

```
use wsforge::prelude::*;

async fn simple_handler() -> Result<String> {
    Ok("Hello from handler!".to_string())
}
```

## Handler Signatures

Handlers are flexible and can have various signatures:

### No Parameters

```
async fn no_params() -> Result<String> {
    Ok("Static response".to_string())
}
```

### With Message

```
async fn with_message(msg: Message) -> Result<Message> {
    println!("Received: {:?}", msg);
    Ok(msg)
}
```

### With Connection

```
async fn with_connection(conn: Connection) -> Result<String> {
    Ok(format!("Your ID: {}", conn.id()))
}
```

### With State

```
use std::sync::Arc;

async fn with_state(State(manager): State<Arc<ConnectionManager>>) -> Result<String> {
    Ok(format!("Active connections: {}", manager.count()))
}
```

## Return Types

Handlers support multiple return types:

### Unit Type `()`

No response sent to client:

```
async fn log_only(msg: Message) -> Result<()> {
    println!("Logging: {:?}", msg);
    Ok(())
}
```

### String

Sent as text message:

```
async fn text_response() -> Result<String> {
    Ok("Text response".to_string())
}
```

### Message

Full control over message:

```
async fn message_response() -> Result<Message> {
    Ok(Message::text("Custom message"))
}
```

### Binary Data

```
async fn binary_response() -> Result<Vec<u8>> {
    Ok(vec![0x01, 0x02, 0x03, 0x04])
}
```

### JSON Response

```
use serde::Serialize;

#[derive(Serialize)]
struct Response {
    status: String,
    count: u32,
}

async fn json_response() -> Result<JsonResponse<Response>> {
    Ok(JsonResponse(Response {
        status: "ok".to_string(),
        count: 42,
    }))
}
```

## Using Extractors

Extractors automatically parse and validate data:

### JSON Extractor

```
use serde::Deserialize;

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

async fn login(Json(req): Json<LoginRequest>) -> Result<String> {
    println!("Login attempt: {}", req.username);
    Ok(format!("Welcome, {}!", req.username))
}
```

### Connection Info Extractor

```
async fn connection_info(ConnectInfo(info): ConnectInfo) -> Result<String> {
    Ok(format!(
        "Connected from {} at {}",
        info.addr,
        info.connected_at
    ))
}
```

### Data Extractor (Binary)

```
async fn binary_handler(Data(bytes): Data) -> Result<String> {
    Ok(format!("Received {} bytes", bytes.len()))
}
```

## Multiple Extractors

Combine multiple extractors in one handler:

```
use std::sync::Arc;
use serde::Deserialize;

#[derive(Deserialize)]
struct ChatMessage {
    text: String,
}

async fn chat_handler(
    Json(msg): Json<ChatMessage>,
    conn: Connection,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<()> {
    println!("{} says: {}", conn.id(), msg.text);

    // Broadcast to others
    let response = serde_json::to_string(&msg)?;
    manager.broadcast_except(conn.id(), Message::text(response));

    Ok(())
}
```

## Error Handling

### Basic Error Handling

```
async fn safe_handler(msg: Message) -> Result<String> {
    let text = msg.as_text()
        .ok_or_else(|| Error::custom("Message must be text"))?;

    if text.is_empty() {
        return Err(Error::custom("Empty message not allowed"));
    }

    Ok(format!("Processed: {}", text))
}
```

### With Custom Error Types

```
async fn validated_handler(Json(data): Json<serde_json::Value>) -> Result<String> {
    // Validate data
    if !data.is_object() {
        return Err(Error::custom("Data must be an object"));
    }

    let username = data.get("username")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::custom("Missing username field"))?;

    if username.len() < 3 {
        return Err(Error::custom("Username too short (min 3 characters)"));
    }

    Ok(format!("Valid user: {}", username))
}
```

### Graceful Error Recovery

```
async fn resilient_handler(msg: Message) -> Result<String> {
    match msg.json::<serde_json::Value>() {
        Ok(data) => {
            // Process JSON
            Ok(format!("Processed: {:?}", data))
        }
        Err(_) => {
            // Fallback to text
            Ok(format!("Text: {}", msg.as_text().unwrap_or("invalid")))
        }
    }
}
```

## Advanced Patterns

### Stateful Handler with Database

```
use std::sync::Arc;

struct Database {
    // Your database connection
}

async fn db_handler(
    Json(data): Json<serde_json::Value>,
    State(db): State<Arc<Database>>,
) -> Result<JsonResponse<serde_json::Value>> {
    // Query database
    // let result = db.query(...).await?;

    Ok(JsonResponse(serde_json::json!({
        "status": "success",
        "data": data
    })))
}
```

### Handler with Middleware Pattern

```
async fn auth_handler(
    msg: Message,
    extensions: &Extensions,
) -> Result<String> {
    // Check if user is authenticated (set by middleware)
    if let Some(user_id) = extensions.get::<u64>("user_id") {
        Ok(format!("Authenticated user: {}", user_id))
    } else {
        Err(Error::custom("Unauthorized"))
    }
}
```

### Command Pattern Handler

```
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "command")]
enum Command {
    Echo { text: String },
    Broadcast { message: String },
    Stats,
}

async fn command_handler(
    Json(cmd): Json<Command>,
    conn: Connection,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<String> {
    match cmd {
        Command::Echo { text } => {
            Ok(format!("Echo: {}", text))
        }
        Command::Broadcast { message } => {
            manager.broadcast(Message::text(message));
            Ok("Broadcasted".to_string())
        }
        Command::Stats => {
            Ok(format!("Active connections: {}", manager.count()))
        }
    }
}
```

### Rate-Limited Handler

```
use std::collections::HashMap;
use tokio::sync::RwLock;

struct RateLimiter {
    requests: RwLock<HashMap<String, u32>>,
}

async fn rate_limited_handler(
    msg: Message,
    conn: Connection,
    State(limiter): State<Arc<RateLimiter>>,
) -> Result<String> {
    let mut requests = limiter.requests.write().await;
    let count = requests.entry(conn.id().clone()).or_insert(0);

    *count += 1;

    if *count > 100 {
        return Err(Error::custom("Rate limit exceeded"));
    }

    Ok("Processed".to_string())
}
```

## Best Practices

### 1. Keep Handlers Focused

Each handler should do one thing well:

```
// Good - focused
async fn send_message(Json(msg): Json<ChatMessage>) -> Result<()> {
    // Only handles sending messages
    Ok(())
}

// Bad - doing too much
async fn handle_everything(msg: Message) -> Result<String> {
    // Handles messages, auth, logging, broadcasting...
    Ok("".to_string())
}
```

### 2. Validate Early

```
async fn validated_handler(Json(data): Json<UserData>) -> Result<String> {
    // Validate immediately
    if data.age < 18 {
        return Err(Error::custom("Must be 18+"));
    }

    // Process valid data
    Ok("Valid".to_string())
}
```

### 3. Use Proper Error Messages

```
async fn descriptive_errors(msg: Message) -> Result<String> {
    let text = msg.as_text()
        .ok_or_else(|| Error::custom("Expected text message, got binary"))?;

    if text.is_empty() {
        return Err(Error::custom("Message cannot be empty"));
    }

    Ok(text.to_string())
}
```

### 4. Don't Block the Handler

```
// Good - async operations
async fn async_handler(Json(data): Json<Request>) -> Result<String> {
    let result = async_database_call().await?;
    Ok(result)
}

// Bad - blocking operation
async fn blocking_handler() -> Result<String> {
    // std::thread::sleep(Duration::from_secs(5)); // DON'T DO THIS
    Ok("done".to_string())
}
```

### 5. Use Type Safety

```
use serde::Deserialize;

#[derive(Deserialize)]
struct TypedRequest {
    action: String,
    data: serde_json::Value,
}

// Good - type-safe
async fn typed_handler(Json(req): Json<TypedRequest>) -> Result<String> {
    Ok(req.action)
}

// Bad - stringly typed
async fn untyped_handler(msg: Message) -> Result<String> {
    let text = msg.as_text().unwrap();
    // Manual string parsing...
    Ok("".to_string())
}
```

## Registration

Register handlers with the router:

```
#[tokio::main]
async fn main() -> Result<()> {
    let router = Router::new()
        .route("/echo", handler(echo_handler))
        .route("/chat", handler(chat_handler))
        .default_handler(handler(fallback_handler));

    router.listen("127.0.0.1:8080").await?;
    Ok(())
}
```

## Testing Handlers

```
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_echo_handler() {
        let msg = Message::text("test");
        let result = echo_handler(msg).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_json_handler() {
        let json = serde_json::json!({"name": "Alice"});
        let msg = Message::text(serde_json::to_string(&json).unwrap());
        // Test your handler
    }
}
```

## Next Steps

- [Extractors Guide](extractors.md) - Learn about all available extractors
- [Broadcasting](broadcasting.md) - Send messages to multiple clients
- [State Management](state-management.md) - Share data across handlers
- [Error Handling](error-handling.md) - Advanced error patterns
