# Frequently Asked Questions (FAQ)

## General Questions

### What is WsForge?

WsForge is a high-performance WebSocket framework for Rust that provides a type-safe, ergonomic API for building real-time applications. It's built on top of `tokio-tungstenite` and inspired by modern web frameworks like Axum.

### Why should I use WsForge instead of tokio-tungstenite directly?

WsForge provides:
- **Higher-level abstractions**: Type-safe extractors, handlers, and routing
- **Less boilerplate**: Simple API reduces repetitive code
- **Built-in features**: Connection management, broadcasting, static file serving
- **Better DX**: Intuitive API similar to popular Rust web frameworks
- **Production-ready**: Comprehensive error handling and lifecycle hooks

### Is WsForge production-ready?

Yes! WsForge is designed for production use with:
- Comprehensive error handling
- Lock-free concurrent connection management
- Extensive testing
- Performance optimizations
- Security features (path traversal prevention, input validation)

### What are the minimum requirements?

- Rust 1.70 or later
- tokio runtime
- Basic understanding of async/await

## Installation & Setup

### How do I add WsForge to my project?

Add to your `Cargo.toml`:

```
[dependencies]
wsforge = "0.1.0"
tokio = { version = "1.40", features = ["full"] }
```

### Do I need any system dependencies?

On Linux, you may need OpenSSL development libraries:

```
# Debian/Ubuntu
sudo apt-get install libssl-dev pkg-config

# Fedora
sudo dnf install openssl-devel
```

macOS and Windows don't require additional dependencies.

### Why do I get compilation errors about features?

If you see errors about missing features, ensure your `Cargo.toml` has the correct tokio features:

```
tokio = { version = "1.40", features = ["full"] }
```

Or more selectively:
```
tokio = { version = "1.40", features = ["rt-multi-thread", "net", "sync", "macros"] }
```

## Usage Questions

### How do I create a simple echo server?

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

### How do I broadcast messages to all connections?

Use the `ConnectionManager` from state:

```
async fn broadcast_handler(
    msg: Message,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<()> {
    manager.broadcast(msg);
    Ok(())
}
```

### How do I parse JSON messages?

Use the `Json` extractor:

```
use serde::Deserialize;

#[derive(Deserialize)]
struct ChatMessage {
    username: String,
    text: String,
}

async fn handler(Json(msg): Json<ChatMessage>) -> Result<String> {
    Ok(format!("{}: {}", msg.username, msg.text))
}
```

### How do I send messages to specific clients?

Get the connection from the manager:

```
async fn send_to_user(
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<()> {
    if let Some(conn) = manager.get(&"conn_123".to_string()) {
        conn.send_text("Hello, user!")?;
    }
    Ok(())
}
```

### Can I serve static files and WebSocket on the same port?

Yes! Use `serve_static`:

```
Router::new()
    .serve_static("public")  // Serves HTML/CSS/JS from ./public
    .default_handler(handler(ws_handler))
```

### How do I handle connection lifecycle events?

Use `on_connect` and `on_disconnect`:

```
Router::new()
    .on_connect(|manager, conn_id| {
        println!("User {} joined", conn_id);
    })
    .on_disconnect(|manager, conn_id| {
        println!("User {} left", conn_id);
    })
```

### How do I share state across handlers?

Use `with_state` on the router:

```
struct AppConfig {
    max_connections: usize,
}

async fn handler(State(config): State<Arc<AppConfig>>) -> Result<String> {
    Ok(format!("Max: {}", config.max_connections))
}

#[tokio::main]
async fn main() -> Result<()> {
    Router::new()
        .with_state(Arc::new(AppConfig { max_connections: 100 }))
        .default_handler(handler(handler))
        .listen("127.0.0.1:8080")
        .await
}
```

## Common Errors

### "connection not found" error

This occurs when trying to send to a disconnected client. Always check if connection exists:

```
if let Some(conn) = manager.get(&conn_id) {
    conn.send(msg)?;
}
```

### "Broadcasting to 0 connections"

This happens when connections aren't being stored properly. Ensure you're using the router's connection manager in state.

### Messages not appearing in browser

Check your JavaScript client:
- Ensure WebSocket URL is correct (`ws://` or `wss://`)
- Verify connection is established before sending
- Check browser console for errors
- Confirm message format matches expected JSON structure

### Compilation error: "feature macros not found"

Add the `macros` feature to `wsforge/Cargo.toml`:

```
[features]
default = ["macros"]
macros = ["wsforge-macros"]
```

### "Address already in use" error

The port is occupied. Either:
- Change the port: `router.listen("127.0.0.1:8081")`
- Kill the process using the port
- Wait a few seconds if you just stopped the server

## Performance

### How many connections can WsForge handle?

WsForge can handle thousands of concurrent connections efficiently. The exact number depends on:
- Available system resources (RAM, CPU)
- Message throughput
- Handler complexity
- Network bandwidth

Benchmarks show 47K+ requests/second for simple echo operations.

### How can I improve performance?

1. **Use binary messages** for large data
2. **Batch operations** when possible
3. **Avoid blocking operations** in handlers
4. **Use connection pooling** for databases
5. **Profile your code** with `cargo flamegraph`

### Does WsForge use multiple threads?

Yes, when using tokio's multi-threaded runtime (default with `features = ["full"]`). Each connection is handled asynchronously on the tokio thread pool.

### What's the memory footprint per connection?

Each connection requires minimal memory:
- Connection struct: ~100 bytes
- Channel buffers: Depends on message queue depth
- Total: Typically 1-2 KB per idle connection

## Comparison with Other Frameworks

### How does WsForge compare to raw tokio-tungstenite?

WsForge provides higher-level abstractions while maintaining similar performance. You get routing, extractors, and state management without writing boilerplate.

### WsForge vs Actix WebSocket?

- **WsForge**: Focused solely on WebSocket with simpler API
- **Actix**: Full web framework with WebSocket support
- **Performance**: Both are very fast, WsForge has less overhead
- **Learning curve**: WsForge is simpler for WebSocket-only apps

### WsForge vs Warp WebSocket?

- **WsForge**: Specialized for WebSocket with built-in features
- **Warp**: HTTP-first framework with WebSocket support
- **API style**: WsForge is more similar to Axum, Warp uses filters
- **Use case**: Choose WsForge for WebSocket-heavy applications

## Advanced Topics

### Can I implement custom middleware?

Yes! Use the extensions system:

```
async fn auth_middleware(msg: Message, extensions: &Extensions) -> Result<()> {
    let token = extract_token(&msg)?;
    extensions.insert("user_id", validate_token(token)?);
    Ok(())
}
```

### How do I implement rate limiting?

Store rate limit data in state:

```
use tokio::sync::RwLock;
use std::collections::HashMap;

struct RateLimiter {
    limits: RwLock<HashMap<String, u32>>,
}

async fn rate_limited_handler(
    conn: Connection,
    State(limiter): State<Arc<RateLimiter>>,
) -> Result<String> {
    let mut limits = limiter.limits.write().await;
    let count = limits.entry(conn.id().clone()).or_insert(0);

    if *count > 100 {
        return Err(Error::custom("Rate limit exceeded"));
    }

    *count += 1;
    Ok("Processed".to_string())
}
```

### Can I use WsForge with WASM?

The core WebSocket functionality works, but some features (like serving static files) are server-only. For WASM clients, use `web-sys` WebSocket API directly.

### How do I implement authentication?

Check authentication on connect:

```
Router::new()
    .on_connect(|manager, conn_id| {
        // Validate auth token from initial message
        // Store user info in extensions or state
    })
```

Or validate in handlers:

```
async fn protected_handler(msg: Message) -> Result<String> {
    let token = msg.as_text()
        .ok_or(Error::custom("No token"))?;

    validate_token(token)?;
    Ok("Authenticated".to_string())
}
```

### How do I implement rooms/channels?

Use a custom room manager in state:

```
struct RoomManager {
    rooms: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

async fn join_room(
    conn: Connection,
    State(rooms): State<Arc<RoomManager>>,
) -> Result<()> {
    let mut room_map = rooms.rooms.write().await;
    room_map.entry("general".to_string())
        .or_insert_with(Vec::new)
        .push(conn.id().clone());
    Ok(())
}
```

### Can I use WsForge with databases?

Yes! Add the database pool to state:

```
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<()> {
    let pool = PgPool::connect("postgres://...").await?;

    Router::new()
        .with_state(Arc::new(pool))
        .default_handler(handler(db_handler))
        .listen("127.0.0.1:8080")
        .await
}

async fn db_handler(State(pool): State<Arc<PgPool>>) -> Result<String> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&*pool)
        .await?;
    Ok(format!("Users: {}", row.0))
}
```

## Testing

### How do I test my handlers?

Handlers are just async functions, so test them directly:

```
#[tokio::test]
async fn test_echo_handler() {
    let msg = Message::text("hello");
    let result = echo_handler(msg).await.unwrap();
    assert_eq!(result.as_text(), Some("hello"));
}
```

### How do I test with a real WebSocket connection?

Use `tokio-tungstenite` client in tests:

```
#[tokio::test]
async fn test_server() {
    // Start server in background
    tokio::spawn(async {
        Router::new()
            .default_handler(handler(echo))
            .listen("127.0.0.1:8081")
            .await
    });

    // Connect client
    let (mut client, _) = tokio_tungstenite::connect_async("ws://127.0.0.1:8081")
        .await
        .unwrap();

    // Test communication
    client.send(Message::text("test")).await.unwrap();
}
```

## Troubleshooting

### Server starts but I can't connect

1. Check firewall settings
2. Verify the correct URL (`ws://` not `http://`)
3. Ensure server is listening on correct interface (`0.0.0.0` for all)
4. Check if port is accessible

### Messages are delayed

1. Check for blocking operations in handlers
2. Verify network latency
3. Look for backpressure in message queues
4. Profile your application

### Memory usage keeps growing

1. Check for connection leaks (connections not being removed)
2. Verify message queues aren't growing unbounded
3. Look for circular references in state
4. Use memory profiling tools

## Getting Help

### Where can I get help?

- **GitHub Issues**: Report bugs and request features
- **GitHub Discussions**: Ask questions and discuss ideas
- **YouTube**: [@AarambhDevHub](https://youtube.com/@AarambhDevHub) for tutorials
- **Documentation**: Check other docs files

### How do I report a bug?

Create an issue with:
- Clear description
- Minimal reproducible example
- Expected vs actual behavior
- Environment details (OS, Rust version, WsForge version)

### How can I contribute?

See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines on:
- Reporting bugs
- Suggesting features
- Submitting pull requests
- Improving documentation

## License & Credits

### What license is WsForge under?

MIT License - see [LICENSE](../LICENSE) for details.

### Who created WsForge?

WsForge is created and maintained by [Aarambh Dev Hub](https://github.com/AarambhDevHub).

### Can I use WsForge in commercial projects?

Yes! The MIT license allows commercial use.

---

**Still have questions?** Open an issue on [GitHub](https://github.com/aarambhdevhub/wsforge/issues) or check out the [troubleshooting guide](troubleshooting.md).
