<div align="center">

# 🔥 WsForge

### High-Performance WebSocket Framework for Rust

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![GitHub Stars](https://img.shields.io/github/stars/aarambhdevhub/wsforge?style=social)](https://github.com/aarambhdevhub/wsforge)

**Build real-time applications with ease and exceptional performance**

[Documentation](https://docs.rs/wsforge) | [Examples](./examples) | [API Reference](https://docs.rs/wsforge) | [YouTube](https://youtube.com/@AarambhDevHub)

</div>

---

## ✨ Features

- 🚀 **High Performance** - Built on tokio-tungstenite with zero-copy optimizations
- 🔧 **Type-Safe Extractors** - Automatic JSON, State, and Connection extraction
- 🎯 **Flexible Handlers** - Return String, Message, Result, JsonResponse, or ()
- 📡 **Broadcasting** - Built-in broadcast, broadcast_except, and targeted messaging
- ⚡ **Concurrent** - Lock-free connection management using DashMap
- 🔄 **Lifecycle Hooks** - on_connect and on_disconnect callbacks
- 🌐 **Hybrid Server** - Serve static files and WebSocket on the same port
- 🛡️ **Type Safety** - Compile-time guarantees prevent common errors
- 🎨 **Developer Friendly** - Intuitive API inspired by Axum
- 📦 **Batteries Included** - Macros, examples, and comprehensive documentation

## 🚀 Quick Start

### Installation

Add WsForge to your `Cargo.toml`:

```
[dependencies]
wsforge = "0.1.0"
tokio = { version = "1.40", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Echo Server Example

```rust
use wsforge::prelude::*;

async fn echo(msg: Message) -> Result<Message> {
    Ok(msg)
}

#[tokio::main]
async fn main() -> Result<()> {
    let router = Router::new()
        .default_handler(handler(echo));

    println!("🚀 WebSocket server running on ws://127.0.0.1:8080");
    router.listen("127.0.0.1:8080").await?;
    Ok(())
}
```

### Chat Server Example

```rust
use wsforge::prelude::*;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct ChatMessage {
    username: String,
    text: String,
}

async fn chat_handler(
    Json(msg): Json<ChatMessage>,
    conn: Connection,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<()> {
    println!("{}: {}", msg.username, msg.text);

    // Broadcast to everyone except sender
    let response = serde_json::to_string(&msg)?;
    manager.broadcast_except(conn.id(), Message::text(response));

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let router = Router::new()
        .default_handler(handler(chat_handler))
        .on_connect(|manager, conn_id| {
            println!("✅ {} joined (Total: {})", conn_id, manager.count());
        })
        .on_disconnect(|manager, conn_id| {
            println!("❌ {} left (Total: {})", conn_id, manager.count());
        });

    router.listen("127.0.0.1:8080").await?;
    Ok(())
}
```

## 📁 Project Structure

```
wsforge/
├── wsforge-core/          # Core framework implementation
│   ├── src/
│   │   ├── connection.rs  # Connection management
│   │   ├── message.rs     # Message types
│   │   ├── handler.rs     # Handler traits
│   │   ├── extractor.rs   # Type extractors
│   │   ├── router.rs      # Routing logic
│   │   ├── state.rs       # State management
│   │   ├── error.rs       # Error types
│   │   └── static_files.rs # Static file serving
│   └── Cargo.toml
├── wsforge-macros/        # Procedural macros
│   ├── src/
│   │   └── lib.rs
│   └── Cargo.toml
├── wsforge/               # Main crate (re-exports)
│   ├── src/
│   │   └── lib.rs
│   └── Cargo.toml
├── examples/              # Example applications
│   ├── echo/              # Simple echo server
│   ├── chat/              # CLI chat application
│   ├── chat-web/          # Web-based chat with UI
│   └── realtime-game/     # Real-time game example
├── docs/                  # Documentation
├── CONTRIBUTING.md        # Contribution guidelines
├── LICENSE               # MIT License
└── README.md             # This file
```

## 📖 Documentation

- **[Getting Started](docs/getting-started.md)** - Your first WebSocket server
- **[Installation Guide](docs/installation.md)** - Detailed setup instructions
- **[Handlers](docs/handlers.md)** - Writing handler functions
- **[Extractors](docs/extractors.md)** - Type-safe data extraction
- **[Broadcasting](docs/broadcasting.md)** - Sending messages to multiple clients
- **[State Management](docs/state-management.md)** - Sharing data across connections
- **[API Reference](docs/api-reference.md)** - Complete API documentation
- **[Troubleshooting](docs/troubleshooting.md)** - Common issues and solutions

## 🎮 Examples

### Run Examples

```
# Simple echo server
cargo run --example echo

# CLI chat application
cargo run --example chat

# Web chat with beautiful UI
cargo run --example chat-web
# Open http://127.0.0.1:8080 in your browser

# Real-time game server
cargo run --example realtime-game
```

### Example Features

**Web Chat** (`examples/chat-web`):
- Beautiful gradient UI with dark mode
- Real-time message broadcasting
- User join/leave notifications
- Live user count display
- Auto-reconnection on disconnect
- LocalStorage for username persistence

## 🎯 Key Concepts

### Handlers

Handlers are async functions that process WebSocket messages:

```rust
// Simple handler
async fn simple() -> Result<String> {
    Ok("Hello!".to_string())
}

// Handler with extractors
async fn complex(
    Json(data): Json<MyStruct>,
    conn: Connection,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<JsonResponse<Response>> {
    // Your logic here
}
```

### Extractors

Type-safe data extraction from messages and context:

| Extractor | Description |
|-----------|-------------|
| `Message` | Raw WebSocket message |
| `Json<T>` | Deserialize JSON automatically |
| `Connection` | Access to active connection |
| `State<T>` | Shared application state |
| `ConnectInfo` | Connection metadata |
| `Data` | Raw binary data |

### Broadcasting

Send messages to multiple connections efficiently:

```rust
// Broadcast to all
manager.broadcast(msg);

// Broadcast except sender
manager.broadcast_except(conn.id(), msg);

// Broadcast to specific connections
manager.broadcast_to(&["conn_1", "conn_2"], msg);
```

## 📊 Performance

WsForge is designed for high performance:

- **47K+ requests/second** for simple echo operations
- **Sub-millisecond latency** for message routing
- **Lock-free** connection management using DashMap
- **Zero-copy** message handling where possible
- **Linear scaling** with connection count

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Quick Contribution Steps

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Setup

```
# Clone the repository
git clone https://github.com/aarambhdevhub/wsforge.git
cd wsforge

# Build the project
cargo build --all

# Run tests
cargo test --all

# Run examples
cargo run --example echo
```

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [tokio-tungstenite](https://github.com/snapview/tokio-tungstenite)
- Inspired by [Axum](https://github.com/tokio-rs/axum)
- Thanks to the amazing Rust community

## 🔗 Links

- **GitHub**: [aarambhdevhub/wsforge](https://github.com/aarambhdevhub/wsforge)
- **Documentation**: [docs.rs/wsforge](https://docs.rs/wsforge)
- **Crates.io**: [crates.io/crates/wsforge](https://crates.io/crates/wsforge)
- **YouTube**: [@AarambhDevHub](https://youtube.com/@AarambhDevHub)
- **Issues**: [GitHub Issues](https://github.com/aarambhdevhub/wsforge/issues)

## 💬 Community & Support

- **Questions?** Open a [GitHub Discussion](https://github.com/aarambhdevhub/wsforge/discussions)
- **Bug Reports**: [GitHub Issues](https://github.com/aarambhdevhub/wsforge/issues)
- **YouTube Tutorials**: [@AarambhDevHub](https://youtube.com/@AarambhDevHub)

## ⭐ Show Your Support

If you find WsForge useful, please consider giving it a ⭐ on GitHub! It helps others discover the project.

---

<div align="center">

**Made with ❤️ by [Aarambh Dev Hub](https://github.com/AarambhDevHub)**

*Building the future of real-time applications in Rust*

</div>
