# Routing in WsForge

Routing in WsForge allows you to direct WebSocket messages to different handlers based on patterns in the message content. This guide covers everything you need to know about routing.

## Table of Contents

- [Overview](#overview)
- [Basic Routing](#basic-routing)
- [Route Matching](#route-matching)
- [Multiple Routes](#multiple-routes)
- [Default Handler](#default-handler)
- [Route Patterns](#route-patterns)
- [Best Practices](#best-practices)
- [Advanced Routing](#advanced-routing)

## Overview

WsForge routing works by examining incoming text messages and matching them against registered route patterns. When a message starts with a route prefix (e.g., `/chat`, `/api`), it's directed to the corresponding handler.

### Key Concepts

- **Routes**: Paths that messages are matched against (e.g., `/chat`, `/game`)
- **Handlers**: Functions that process messages for specific routes
- **Default Handler**: Fallback handler for messages that don't match any route
- **Router**: Central component that manages all routes and handlers

## Basic Routing

### Simple Route Registration

```
use wsforge::prelude::*;

async fn echo_handler(msg: Message) -> Result<String> {
    Ok(format!("Echo: {:?}", msg.as_text()))
}

async fn chat_handler(msg: Message) -> Result<String> {
    Ok("Chat message received".to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    let router = Router::new()
        .route("/echo", handler(echo_handler))
        .route("/chat", handler(chat_handler));

    router.listen("127.0.0.1:8080").await?;
    Ok(())
}
```

### How It Works

When a client sends:
- `/echo hello` → Routes to `echo_handler`
- `/chat Hey there` → Routes to `chat_handler`
- `random message` → No match, needs default handler

## Route Matching

### Prefix Matching

WsForge uses prefix matching for routes:

```
// Client sends: "/chat hello world"
// Matches route: "/chat"
// Handler receives the full message

async fn chat_handler(msg: Message) -> Result<()> {
    // msg contains: "/chat hello world"
    let text = msg.as_text().unwrap();

    // Parse the command and arguments
    if let Some((route, args)) = text.split_once(' ') {
        println!("Route: {}", route);      // "/chat"
        println!("Arguments: {}", args);    // "hello world"
    }

    Ok(())
}
```

### Route Order

Routes are checked in the order they're registered. More specific routes should be registered first:

```
let router = Router::new()
    .route("/api/users", handler(users_handler))     // More specific
    .route("/api", handler(api_handler))             // Less specific
    .default_handler(handler(default_handler));      // Fallback
```

## Multiple Routes

### Organizing Multiple Routes

```
use wsforge::prelude::*;
use std::sync::Arc;

// Echo route
async fn echo_handler(msg: Message) -> Result<Message> {
    Ok(msg)
}

// Chat route
async fn chat_handler(
    msg: Message,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<()> {
    manager.broadcast(msg);
    Ok(())
}

// Stats route
async fn stats_handler(
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<String> {
    Ok(format!("Active connections: {}", manager.count()))
}

// API route
async fn api_handler(msg: Message) -> Result<JsonResponse<serde_json::Value>> {
    let response = serde_json::json!({
        "status": "ok",
        "message": "API endpoint"
    });
    Ok(JsonResponse(response))
}

#[tokio::main]
async fn main() -> Result<()> {
    let router = Router::new()
        .route("/echo", handler(echo_handler))
        .route("/chat", handler(chat_handler))
        .route("/stats", handler(stats_handler))
        .route("/api", handler(api_handler))
        .default_handler(handler(echo_handler));  // Default to echo

    router.listen("127.0.0.1:8080").await?;
    Ok(())
}
```

### Route Testing

Send these messages from the client:

```
// JavaScript WebSocket client
const ws = new WebSocket('ws://127.0.0.1:8080');

ws.onopen = () => {
    ws.send('/echo test');           // → echo_handler
    ws.send('/chat hello');          // → chat_handler
    ws.send('/stats');               // → stats_handler
    ws.send('/api');                 // → api_handler
    ws.send('no route');             // → default_handler
};
```

## Default Handler

The default handler catches all messages that don't match any registered route.

### Setting a Default Handler

```
async fn default_handler(msg: Message) -> Result<String> {
    Ok(format!("Unknown command: {:?}", msg.as_text()))
}

let router = Router::new()
    .route("/known", handler(known_handler))
    .default_handler(handler(default_handler));
```

### Default Handler Use Cases

**1. Help Messages**

```
async fn help_handler(_msg: Message) -> Result<String> {
    Ok(r#"
Available commands:
  /echo <text>   - Echo back the text
  /chat <text>   - Send to all users
  /stats         - View server stats
  /help          - Show this message
    "#.to_string())
}

let router = Router::new()
    .route("/echo", handler(echo_handler))
    .route("/chat", handler(chat_handler))
    .route("/stats", handler(stats_handler))
    .default_handler(handler(help_handler));
```

**2. Error Messages**

```
async fn not_found(msg: Message) -> Result<JsonResponse<serde_json::Value>> {
    let error = serde_json::json!({
        "error": "Unknown command",
        "received": msg.as_text(),
        "help": "Send /help for available commands"
    });
    Ok(JsonResponse(error))
}
```

**3. Catch-All Processing**

```
async fn process_all(
    msg: Message,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<()> {
    // Process all messages regardless of route
    println!("Processing: {:?}", msg.as_text());
    manager.broadcast(msg);
    Ok(())
}
```

## Route Patterns

### Command Pattern

Parse commands and arguments:

```
async fn command_handler(msg: Message) -> Result<String> {
    let text = msg.as_text().unwrap_or("");

    // Split: "/command arg1 arg2 arg3"
    let parts: Vec<&str> = text.split_whitespace().collect();

    match parts.as_slice() {
        ["/command", ..] => {
            let args = &parts[1..];
            Ok(format!("Command with {} arguments", args.len()))
        }
        _ => Err(Error::custom("Invalid command format"))
    }
}
```

### JSON Route Pattern

Handle JSON payloads with routes:

```
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct CommandMessage {
    command: String,
    data: serde_json::Value,
}

async fn json_route_handler(Json(cmd): Json<CommandMessage>) -> Result<String> {
    match cmd.command.as_str() {
        "echo" => Ok(format!("Echo: {:?}", cmd.data)),
        "broadcast" => Ok("Broadcasting...".to_string()),
        _ => Err(Error::custom("Unknown command"))
    }
}

// Client sends:
// {"command": "echo", "data": "hello"}
```

### RESTful-Style Routes

Simulate RESTful patterns:

```
async fn users_handler(msg: Message) -> Result<String> {
    let text = msg.as_text().unwrap_or("");

    // Parse: "/users/123/profile"
    let parts: Vec<&str> = text.split('/').filter(|s| !s.is_empty()).collect();

    match parts.as_slice() {
        ["users"] => Ok("List all users".to_string()),
        ["users", id] => Ok(format!("Get user: {}", id)),
        ["users", id, "profile"] => Ok(format!("Get profile for: {}", id)),
        _ => Err(Error::custom("Invalid path"))
    }
}
```

## Best Practices

### 1. Use Clear Route Names

```
// Good
.route("/chat", handler(chat_handler))
.route("/game", handler(game_handler))
.route("/api", handler(api_handler))

// Avoid
.route("/c", handler(chat_handler))
.route("/g", handler(game_handler))
```

### 2. Document Your Routes

```
/// Handles chat messages
/// Route: /chat <message>
/// Example: /chat Hello everyone!
async fn chat_handler(msg: Message) -> Result<()> {
    // Implementation
    Ok(())
}
```

### 3. Validate Input Early

```
async fn validated_handler(msg: Message) -> Result<String> {
    let text = msg.as_text()
        .ok_or_else(|| Error::custom("Message must be text"))?;

    if text.len() > 1000 {
        return Err(Error::custom("Message too long"));
    }

    // Process valid message
    Ok("Processed".to_string())
}
```

### 4. Keep Routes Focused

Each route handler should handle one specific concern:

```
// Good - focused handlers
async fn send_message_handler(...) -> Result<()> { }
async fn list_messages_handler(...) -> Result<Vec<Message>> { }
async fn delete_message_handler(...) -> Result<()> { }

// Avoid - single handler doing everything
async fn message_handler(...) -> Result<()> { }
```

### 5. Use Type-Safe Extractors

```
use serde::Deserialize;

#[derive(Deserialize)]
struct ChatMessage {
    username: String,
    text: String,
}

// Type-safe - compiler checks types
async fn chat_handler(Json(msg): Json<ChatMessage>) -> Result<()> {
    println!("{}: {}", msg.username, msg.text);
    Ok(())
}
```

## Advanced Routing

### Dynamic Route Registration

Build routes dynamically:

```
fn create_router() -> Router {
    let mut router = Router::new();

    // Register routes from configuration
    let routes = vec![
        ("/echo", handler(echo_handler)),
        ("/chat", handler(chat_handler)),
        ("/game", handler(game_handler)),
    ];

    for (path, handler) in routes {
        router = router.route(path, handler);
    }

    router.default_handler(handler(default_handler))
}
```

### Route Middleware Pattern

Implement middleware-like behavior:

```
async fn auth_middleware(
    msg: Message,
    extensions: &Extensions,
) -> Result<()> {
    // Extract and validate auth token
    let text = msg.as_text().ok_or(Error::custom("Invalid message"))?;

    if !text.contains("token=") {
        return Err(Error::custom("Unauthorized"));
    }

    // Store auth info
    extensions.insert("authorized", true);
    Ok(())
}

async fn protected_handler(
    msg: Message,
    extensions: Extensions,
) -> Result<String> {
    // Check authorization
    let authorized = extensions.get::<bool>("authorized")
        .map(|v| *v)
        .unwrap_or(false);

    if !authorized {
        return Err(Error::custom("Forbidden"));
    }

    Ok("Access granted".to_string())
}
```

### Route Groups

Organize related routes:

```
// API routes
async fn api_v1_users() -> Result<String> { Ok("V1 Users".to_string()) }
async fn api_v1_posts() -> Result<String> { Ok("V1 Posts".to_string()) }
async fn api_v2_users() -> Result<String> { Ok("V2 Users".to_string()) }

let router = Router::new()
    // API V1
    .route("/api/v1/users", handler(api_v1_users))
    .route("/api/v1/posts", handler(api_v1_posts))
    // API V2
    .route("/api/v2/users", handler(api_v2_users));
```

## Complete Example

Here's a complete routing example with multiple patterns:

```
use wsforge::prelude::*;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct ChatMsg {
    username: String,
    text: String,
}

#[derive(Serialize)]
struct Response {
    status: String,
    message: String,
}

async fn echo(msg: Message) -> Result<Message> {
    Ok(msg)
}

async fn chat(
    Json(msg): Json<ChatMsg>,
    conn: Connection,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<()> {
    let response = serde_json::to_string(&msg)?;
    manager.broadcast_except(conn.id(), Message::text(response));
    Ok(())
}

async fn stats(
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<JsonResponse<serde_json::Value>> {
    let stats = serde_json::json!({
        "connections": manager.count(),
        "ids": manager.all_ids()
    });
    Ok(JsonResponse(stats))
}

async fn help(_msg: Message) -> Result<String> {
    Ok(r#"
Commands:
  /echo <text>     - Echo back
  /chat <json>     - Send to all
  /stats           - View stats
  /help            - This message
    "#.to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    let router = Router::new()
        .route("/echo", handler(echo))
        .route("/chat", handler(chat))
        .route("/stats", handler(stats))
        .route("/help", handler(help))
        .default_handler(handler(help));

    println!("Server: ws://127.0.0.1:8080");
    router.listen("127.0.0.1:8080").await?;
    Ok(())
}
```

## See Also

- [Handlers Guide](handlers.md) - Learn about handler functions
- [Extractors Guide](extractors.md) - Type-safe data extraction
- [Examples](examples.md) - More routing examples
- [API Reference](api-reference.md) - Complete API documentation
