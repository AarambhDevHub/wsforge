# WsForge Macros

Procedural macros for the WsForge WebSocket framework.

## Overview

This crate provides compile-time code generation macros that reduce boilerplate and improve developer experience when building WebSocket applications with WsForge.

## Available Macros

### `#[websocket_handler]`

Transform async functions into WebSocket handlers.

```rust
use wsforge_macros::websocket_handler;

#[websocket_handler]
async fn my_handler(msg: Message) -> Result<String> {
    Ok("response".to_string())
}
```

### `#[derive(WebSocketMessage)]`

Auto-implement message conversion methods.

```rust
use wsforge_macros::WebSocketMessage;
use serde::{Deserialize, Serialize};

#[derive(WebSocketMessage, Serialize, Deserialize)]
struct ChatMessage {
    username: String,
    text: String,
}

// Generates:
// - into_message(&self) -> Message
// - from_message(msg: Message) -> Result<Self, Error>
```

### `#[derive(WebSocketHandler)]`

Implement the Handler trait for custom types.

```rust
use wsforge_macros::WebSocketHandler;

#[derive(WebSocketHandler)]
struct MyHandler;

impl MyHandler {
    async fn handle(
        &self,
        message: Message,
        conn: Connection,
        state: AppState,
        extensions: Extensions,
    ) -> Result<Option<Message>> {
        // Your logic here
    }
}
```

### `routes!()`

Create a new Router instance.

```rust
use wsforge_macros::routes;

let router = routes!()
    .default_handler(handler(my_handler));
```

## Installation

This crate is typically used as a dependency of the main `wsforge` crate:

```
[dependencies]
wsforge = { version = "0.1.0", features = ["macros"] }
```

Or directly:

```
[dependencies]
wsforge-macros = "0.1.0"
```

## Documentation

For complete documentation, see:
- [WsForge Documentation](https://docs.rs/wsforge)
- [API Reference](https://docs.rs/wsforge-macros)
- [Examples](../examples)

## Requirements

- Rust 1.70 or later
- Part of the WsForge workspace

## License

MIT License - see [LICENSE](../LICENSE) for details.

## Links

- **Main Crate**: [wsforge](../wsforge)
- **Core Library**: [wsforge-core](../wsforge-core)
- **GitHub**: [aarambhdevhub/wsforge](https://github.com/aarambhdevhub/wsforge)
