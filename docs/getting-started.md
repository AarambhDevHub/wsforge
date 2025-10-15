# Getting Started with WsForge

Welcome to WsForge! This guide will help you build your first WebSocket server in just a few minutes.

## What is WsForge?

WsForge is a high-performance WebSocket framework for Rust that combines the power of `tokio-tungstenite` with an intuitive, type-safe API inspired by modern web frameworks like Axum. It's designed to make building real-time applications easy, fast, and enjoyable.

**Key Features:**
- 🚀 High performance with async/await
- 🔧 Type-safe extractors (JSON, State, Connection)
- 📡 Built-in broadcasting support
- 🌐 Hybrid HTTP/WebSocket server
- 🛡️ Compile-time safety guarantees

## Prerequisites

Before you begin, make sure you have:

- **Rust 1.70 or later** - [Install via rustup](https://rustup.rs/)
- **Basic knowledge of async/await in Rust** (helpful but not required)
- **A code editor** (VS Code with rust-analyzer recommended)

Check your Rust version:

```bash
rustc --version
```

## Creating Your First WebSocket Server

### Step 1: Create a New Project

Open your terminal and create a new Rust project:

```bash
cargo new my-websocket-server
cd my-websocket-server
```

### Step 2: Add Dependencies

Open `Cargo.toml` and add WsForge:

```toml
[package]
name = "my-websocket-server"
version = "0.1.0"
edition = "2021"

[dependencies]
wsforge = "0.1.0"
tokio = { version = "1.40", features = ["full"] }
```

### Step 3: Write Your First Echo Server

Replace the contents of `src/main.rs` with:

```rust
use wsforge::prelude::*;

// Handler function that echoes messages back
async fn echo_handler(msg: Message) -> Result<Message> {
    println!("📨 Received: {:?}", msg.as_text());
    Ok(msg)  // Echo the message back
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create a router with the echo handler
    let router = Router::new()
        .default_handler(handler(echo_handler));

    println!("🚀 WebSocket server running on ws://127.0.0.1:8080");
    println!("📡 Press Ctrl+C to stop");

    // Start listening for connections
    router.listen("127.0.0.1:8080").await?;
    Ok(())
}
```

### Step 4: Run Your Server

Start the server:

```bash
cargo run
```

You should see:
```bash
🚀 WebSocket server running on ws://127.0.0.1:8080
📡 Press Ctrl+C to stop
```

## Testing Your Server

### Option 1: Using Browser Console

Open your browser's developer console (F12) and paste:

```rust
const ws = new WebSocket('ws://localhost:8080');

ws.onopen = () => {
    console.log('✅ Connected!');
    ws.send('Hello, WsForge!');
};

ws.onmessage = (event) => {
    console.log('📨 Received:', event.data);
};

ws.onerror = (error) => {
    console.error('❌ Error:', error);
};
```

### Option 2: Using websocat (CLI Tool)

Install websocat:

```bash
# macOS
brew install websocat

# Linux
cargo install websocat

# Windows (via cargo)
cargo install websocat
```

Connect and send messages:

```bash
websocat ws://127.0.0.1:8080
```

Type any message and press Enter. You should see it echoed back!

### Option 3: Using an Online Tool

Visit [websocket.org/echo.html](https://websocket.org/echo.html) and connect to `ws://localhost:8080`.

## Understanding the Code

Let's break down what we just built:

```rust
use wsforge::prelude::*;
```
Imports all commonly used WsForge types.

```rust
async fn echo_handler(msg: Message) -> Result<Message> {
    Ok(msg)
}
```
A **handler function** that receives a `Message` and returns it unchanged. Handlers can extract various types and return different response types.

```rust
let router = Router::new()
    .default_handler(handler(echo_handler));
```
Creates a **Router** that directs all messages to our echo handler.

```rust
router.listen("127.0.0.1:8080").await?;
```
Starts the server on port 8080.

## Adding Lifecycle Callbacks

Let's enhance the server with connection tracking:

```rust
use wsforge::prelude::*;

async fn echo_handler(msg: Message) -> Result<Message> {
    Ok(msg)
}

#[tokio::main]
async fn main() -> Result<()> {
    let router = Router::new()
        .default_handler(handler(echo_handler))
        .on_connect(|manager, conn_id| {
            println!("✅ Client {} connected (Total: {})",
                conn_id, manager.count());
        })
        .on_disconnect(|manager, conn_id| {
            println!("❌ Client {} disconnected (Remaining: {})",
                conn_id, manager.count());
        });

    println!("🚀 WebSocket server running on ws://127.0.0.1:8080");
    router.listen("127.0.0.1:8080").await?;
    Ok(())
}
```

Now when clients connect or disconnect, you'll see logs!

## Building a Simple Chat Server

Let's create something more interactive:

```rust
use wsforge::prelude::*;
use std::sync::Arc;

async fn chat_handler(
    msg: Message,
    conn: Connection,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<()> {
    println!("💬 {} says: {:?}", conn.id(), msg.as_text());

    // Broadcast to everyone except the sender
    manager.broadcast_except(conn.id(), msg);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let router = Router::new()
        .default_handler(handler(chat_handler))
        .on_connect(|manager, conn_id| {
            println!("✅ {} joined (Total: {})", conn_id, manager.count());

            // Send welcome message to the new user
            if let Some(conn) = manager.get(&conn_id) {
                let _ = conn.send_text("Welcome to the chat!");
            }
        });

    println!("🚀 Chat server running on ws://127.0.0.1:8080");
    router.listen("127.0.0.1:8080").await?;
    Ok(())
}
```

**What's different?**
- Handler extracts `Connection` and `State<Arc<ConnectionManager>>`
- Uses `broadcast_except()` to send messages to all other clients
- Sends a welcome message to new users

## Common Patterns

### 1. JSON Message Handling

```rust
use wsforge::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Request {
    action: String,
    data: String,
}

#[derive(Serialize)]
struct Response {
    status: String,
    result: String,
}

async fn json_handler(Json(req): Json<Request>) -> Result<JsonResponse<Response>> {
    println!("Action: {}", req.action);

    Ok(JsonResponse(Response {
        status: "success".to_string(),
        result: format!("Processed: {}", req.data),
    }))
}
```

### 2. Multiple Routes

```rust
async fn echo(msg: Message) -> Result<Message> {
    Ok(msg)
}

async fn uppercase(msg: Message) -> Result<String> {
    let text = msg.as_text().unwrap_or("");
    Ok(text.to_uppercase())
}

#[tokio::main]
async fn main() -> Result<()> {
    let router = Router::new()
        .route("/echo", handler(echo))
        .route("/upper", handler(uppercase))
        .default_handler(handler(|_: Message| async {
            Ok("Unknown route".to_string())
        }));

    router.listen("127.0.0.1:8080").await?;
    Ok(())
}
```

Send messages like `/echo hello` or `/upper hello` to use different routes!

## Troubleshooting

### Port Already in Use

**Error:** `Address already in use`

**Solution:** Change the port or kill the process using port 8080:

```bash
# macOS/Linux
lsof -ti:8080 | xargs kill

# Windows
netstat -ano | findstr :8080
taskkill /PID <PID> /F
```

### Connection Refused

**Error:** `Connection refused`

**Solution:**
- Ensure the server is running
- Check firewall settings
- Verify you're using `ws://` not `wss://` for local testing

### Compilation Errors

**Error:** `cannot find type Message in this scope`

**Solution:** Make sure you have `use wsforge::prelude::*;` at the top of your file.

## Next Steps

Now that you have a working WebSocket server, explore more:

- **[Handlers Guide](handlers.md)** - Learn about different handler types
- **[Extractors](extractors.md)** - Deep dive into type extractors
- **[Broadcasting](broadcasting.md)** - Advanced message distribution
- **[State Management](state-management.md)** - Share data across connections
- **[Examples](examples.md)** - Complete example applications

## Learning Resources

- **GitHub Repository**: [github.com/aarambhdevhub/wsforge](https://github.com/aarambhdevhub/wsforge)
- **API Documentation**: Run `cargo doc --open` in your project
- **YouTube Tutorials**: [@AarambhDevHub](https://youtube.com/@AarambhDevHub)
- **Example Projects**: Check the `examples/` directory in the repository

## Need Help?

- **Issues**: [GitHub Issues](https://github.com/aarambhdevhub/wsforge/issues)
- **Questions**: [GitHub Discussions](https://github.com/aarambhdevhub/wsforge/discussions)
- **Documentation**: [docs.rs/wsforge](https://docs.rs/wsforge)

Happy coding with WsForge! 🚀
