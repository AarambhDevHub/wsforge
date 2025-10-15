# Troubleshooting Guide

This guide covers common issues you might encounter while using WsForge and how to resolve them.

## Table of Contents

- [Connection Issues](#connection-issues)
- [Message Handling](#message-handling)
- [Broadcasting Problems](#broadcasting-problems)
- [State Management](#state-management)
- [Compilation Errors](#compilation-errors)
- [Runtime Errors](#runtime-errors)
- [Performance Issues](#performance-issues)
- [Deployment Problems](#deployment-problems)

---

## Connection Issues

### WebSocket Connection Fails to Establish

**Symptoms:**
- Client receives connection refused
- Handshake fails
- Connection drops immediately

**Solutions:**

1. **Check if server is running:**
```
netstat -an | grep 8080
# or
lsof -i :8080
```

2. **Verify bind address:**
```
// ❌ Wrong - only localhost
router.listen("127.0.0.1:8080").await?;

// ✅ Correct - all interfaces
router.listen("0.0.0.0:8080").await?;
```

3. **Check firewall settings:**
```
# Linux
sudo ufw allow 8080

# Check if port is blocked
telnet localhost 8080
```

4. **Verify WebSocket URL:**
```
// ❌ Wrong
const ws = new WebSocket("http://localhost:8080");

// ✅ Correct
const ws = new WebSocket("ws://localhost:8080");

// For HTTPS
const ws = new WebSocket("wss://localhost:8443");
```

### Connection Drops Randomly

**Symptoms:**
- Connection closes unexpectedly
- "Connection reset" errors

**Solutions:**

1. **Enable keep-alive:**
```
async fn ping_handler() -> Result<Message> {
    Ok(Message::ping(vec![]))
}

// Send pings periodically
tokio::spawn(async move {
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;
        manager.broadcast(Message::ping(vec![]));
    }
});
```

2. **Check connection timeout:**
```
// Set timeout in connection handling
tokio::time::timeout(
    Duration::from_secs(60),
    handle_websocket(...)
).await;
```

3. **Monitor logs for errors:**
```
RUST_LOG=wsforge=debug cargo run
```

### "Address already in use" Error

**Symptoms:**
```
Error: Address already in use (os error 98)
```

**Solutions:**

1. **Kill existing process:**
```
# Find process using port
lsof -ti:8080

# Kill it
kill -9 $(lsof -ti:8080)
```

2. **Use different port:**
```
router.listen("127.0.0.1:8081").await?;
```

3. **Enable SO_REUSEADDR:**
```
use tokio::net::TcpListener;
use socket2::{Socket, Domain, Type};

let socket = Socket::new(Domain::IPV4, Type::STREAM, None)?;
socket.set_reuse_address(true)?;
socket.bind(&addr.into())?;
socket.listen(128)?;

let listener = TcpListener::from_std(socket.into())?;
```

---

## Message Handling

### Messages Not Received in Browser

**Symptoms:**
- Server logs show message sent
- Browser doesn't receive anything

**Solutions:**

1. **Check message format:**
```
// ❌ Wrong - broadcasting to 0 connections
async fn handler(msg: Message, State(manager): State<Arc<ConnectionManager>>) -> Result<()> {
    let wrong_manager = Arc::new(ConnectionManager::new()); // New instance!
    wrong_manager.broadcast(msg); // Broadcasts to nothing
    Ok(())
}

// ✅ Correct - use the router's manager
async fn handler(msg: Message, State(manager): State<Arc<ConnectionManager>>) -> Result<()> {
    manager.broadcast(msg); // Broadcasts to actual connections
    Ok(())
}
```

2. **Verify WebSocket event listeners:**
```
const ws = new WebSocket("ws://localhost:8080");

ws.onmessage = (event) => {
    console.log("Received:", event.data);
};

ws.onerror = (error) => {
    console.error("WebSocket error:", error);
};
```

3. **Check if connection is in manager:**
```
async fn handler(conn: Connection, State(manager): State<Arc<ConnectionManager>>) -> Result<()> {
    println!("Total connections: {}", manager.count());
    println!("Connection {} exists: {}", conn.id(), manager.get(conn.id()).is_some());
    Ok(())
}
```

### JSON Parsing Fails

**Symptoms:**
- `JSON error: expected value at line 1 column 1`
- Messages not deserializing

**Solutions:**

1. **Validate JSON format:**
```
async fn handler(msg: Message) -> Result<String> {
    // Add debug logging
    if let Some(text) = msg.as_text() {
        println!("Received text: {}", text);
    }

    // Try parsing
    let data: serde_json::Value = msg.json()
        .map_err(|e| {
            eprintln!("JSON parse error: {}", e);
            Error::custom(format!("Invalid JSON: {}", e))
        })?;

    Ok(format!("Parsed: {:?}", data))
}
```

2. **Check struct fields match:**
```
#[derive(Deserialize)]
struct ChatMessage {
    username: String,
    text: String,
}

// Client must send:
// {"username": "Alice", "text": "Hello"}
// NOT: {"user": "Alice", "message": "Hello"}
```

3. **Handle optional fields:**
```
#[derive(Deserialize)]
struct Request {
    action: String,
    #[serde(default)]
    data: Option<serde_json::Value>,
}
```

### Binary Messages Not Working

**Symptoms:**
- Binary data corrupted
- Errors when sending bytes

**Solutions:**

```
// ✅ Correct binary handling
async fn binary_handler(Data(bytes): Data) -> Result<Vec<u8>> {
    println!("Received {} bytes", bytes.len());
    Ok(bytes) // Echo back
}

// ✅ Send binary from handler
async fn send_binary() -> Result<Vec<u8>> {
    Ok(vec![0x01, 0x02, 0x03, 0x04])
}
```

---

## Broadcasting Problems

### Broadcast Sends to 0 Connections

**Symptoms:**
- Logs show "Broadcasting to 0 connections"
- Connection count is correct but broadcast fails

**Solutions:**

1. **Ensure same manager instance:**
```
// ❌ Wrong - creates new manager
let router = Router::new();
let separate_manager = Arc::new(ConnectionManager::new()); // DON'T DO THIS

// ✅ Correct - use router's manager
let router = Router::new();
let manager = router.connection_manager(); // Use this in state
let router = router.with_state(manager);
```

2. **Verify state insertion:**
```
#[tokio::main]
async fn main() -> Result<()> {
    let router = Router::new()
        .default_handler(handler(my_handler));

    // Manager is automatically added to state by router.listen()
    router.listen("127.0.0.1:8080").await?;
    Ok(())
}
```

3. **Check connection is added before broadcast:**
```
// The framework handles this automatically, but if you're debugging:
async fn on_connect_debug(manager: &Arc<ConnectionManager>, conn_id: ConnectionId) {
    println!("Before: {} connections", manager.count());
    // Connection should already be in manager here
    tokio::time::sleep(Duration::from_millis(100)).await;
    println!("After: {} connections", manager.count());
}
```

### Messages Only Received by Sender

**Symptoms:**
- Sender receives message
- Other clients don't

**Solutions:**

```
// ❌ Wrong - only sends to current connection
async fn handler(msg: Message, conn: Connection) -> Result<Message> {
    conn.send(msg.clone())?; // Only sender gets it
    Ok(msg)
}

// ✅ Correct - broadcast to all except sender
async fn handler(
    msg: Message,
    conn: Connection,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<()> {
    manager.broadcast_except(conn.id(), msg);
    Ok(())
}
```

---

## State Management

### "State not found" Error

**Symptoms:**
```
Error: Extractor error: State not found
```

**Solutions:**

1. **Add state to router:**
```
#[tokio::main]
async fn main() -> Result<()> {
    let db = Arc::new(Database::new());

    let router = Router::new()
        .with_state(db) // ← Add this!
        .default_handler(handler(my_handler));

    router.listen("127.0.0.1:8080").await?;
    Ok(())
}
```

2. **Check state type matches:**
```
// ❌ Wrong - type mismatch
.with_state(Database::new()) // Not Arc!

async fn handler(State(db): State<Arc<Database>>) -> Result<()> {
    // ...
}

// ✅ Correct - types match
.with_state(Arc::new(Database::new()))

async fn handler(State(db): State<Arc<Database>>) -> Result<()> {
    // ...
}
```

### Multiple State Types

**Solution:**
```
struct Config { port: u16 }
struct Database { /* ... */ }

#[tokio::main]
async fn main() -> Result<()> {
    let router = Router::new()
        .with_state(Arc::new(Config { port: 8080 }))
        .with_state(Arc::new(Database::new()))
        .default_handler(handler(my_handler));

    router.listen("127.0.0.1:8080").await?;
    Ok(())
}

async fn my_handler(
    State(config): State<Arc<Config>>,
    State(db): State<Arc<Database>>,
) -> Result<String> {
    Ok(format!("Port: {}", config.port))
}
```

---

## Compilation Errors

### "trait FromMessage is not implemented"

**Error:**
```
the trait `FromMessage` is not implemented for `MyStruct`
```

**Solutions:**

1. **Derive or implement FromMessage:**
```
use serde::Deserialize;

#[derive(Deserialize)]
struct MyStruct {
    field: String,
}

// Use Json extractor
async fn handler(Json(data): Json<MyStruct>) -> Result<String> {
    Ok(data.field)
}
```

### "cannot infer type" Errors

**Error:**
```
cannot infer type for type parameter `T`
```

**Solutions:**

```
// ❌ Wrong - ambiguous
let data = msg.json()?;

// ✅ Correct - explicit type
let data: MyStruct = msg.json()?;

// Or use turbofish
let data = msg.json::<MyStruct>()?;
```

### "future cannot be sent between threads safely"

**Error:**
```
future cannot be sent between threads safely
```

**Solutions:**

```
// ❌ Wrong - Rc is not Send
use std::rc::Rc;
let data = Rc::new(MyData::new());

// ✅ Correct - Arc is Send + Sync
use std::sync::Arc;
let data = Arc::new(MyData::new());
```

### Lifetime Issues

**Solutions:**

```
// ❌ Wrong - lifetime issues
async fn handler(msg: &Message) -> Result<String> {
    // ...
}

// ✅ Correct - owned value
async fn handler(msg: Message) -> Result<String> {
    // ...
}
```

---

## Runtime Errors

### Panic: "already borrowed: BorrowMutError"

**Solutions:**

```
// ❌ Wrong - RefCell is not Send
use std::cell::RefCell;
let state = RefCell::new(data);

// ✅ Correct - Use Arc<RwLock<T>>
use tokio::sync::RwLock;
let state = Arc::new(RwLock::new(data));

async fn handler(State(state): State<Arc<RwLock<MyData>>>) -> Result<()> {
    let mut data = state.write().await;
    data.modify();
    Ok(())
}
```

### "channel closed" Errors

**Symptoms:**
```
Error: Failed to send message: channel closed
```

**Solutions:**

1. **Check connection is still active:**
```
async fn safe_send(conn: &Connection, msg: Message) -> Result<()> {
    match conn.send(msg) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Failed to send to {}: {}", conn.id(), e);
            // Connection likely closed, this is normal
            Ok(()) // Don't propagate error
        }
    }
}
```

### Task Panics

**Solutions:**

```
// ✅ Always handle panics in spawned tasks
tokio::spawn(async move {
    if let Err(e) = risky_operation().await {
        eprintln!("Task error: {}", e);
    }
});

// ✅ Or use catch_unwind for critical tasks
use std::panic::catch_unwind;
tokio::spawn(async move {
    let result = catch_unwind(|| {
        // risky code
    });
    if let Err(e) = result {
        eprintln!("Task panicked: {:?}", e);
    }
});
```

---

## Performance Issues

### High Latency

**Solutions:**

1. **Enable logging to find bottlenecks:**
```
RUST_LOG=wsforge=trace cargo run
```

2. **Profile with cargo-flamegraph:**
```
cargo install flamegraph
cargo flamegraph --bin your-server
```

3. **Check for blocking operations:**
```
// ❌ Wrong - blocks async runtime
use std::thread::sleep;
sleep(Duration::from_secs(1));

// ✅ Correct - async sleep
tokio::time::sleep(Duration::from_secs(1)).await;
```

### Memory Leaks

**Solutions:**

1. **Check for Arc cycles:**
```
// Be careful with Arc<T> containing Arc<T>
// Use Weak for parent references
```

2. **Monitor connection cleanup:**
```
router.on_disconnect(|manager, conn_id| {
    println!("Disconnected: {} (Remaining: {})", conn_id, manager.count());
    // Verify count decreases
});
```

3. **Use valgrind or heaptrack:**
```
valgrind --leak-check=full ./target/release/your-server
```

### High CPU Usage

**Solutions:**

1. **Avoid busy loops:**
```
// ❌ Wrong
loop {
    if condition {
        break;
    }
}

// ✅ Correct
loop {
    if condition {
        break;
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
}
```

2. **Optimize broadcast operations:**
```
// Batch broadcasts if possible
async fn batch_broadcast(manager: &ConnectionManager, messages: Vec<Message>) {
    for msg in messages {
        manager.broadcast(msg);
    }
}
```

---

## Deployment Problems

### Static Files Not Found

**Solutions:**

```
// ✅ Use relative path from binary location
let router = Router::new()
    .serve_static("./public") // Same directory as binary
    .default_handler(handler(ws_handler));

// Or use absolute path
.serve_static("/var/www/myapp/public")
```

### Environment-Specific Issues

**Solutions:**

```
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("{}:{}", host, port);

    println!("Starting server on {}", addr);

    let router = Router::new()
        .default_handler(handler(echo));

    router.listen(&addr).await?;
    Ok(())
}
```

### Docker Issues

**Solutions:**

1. **Bind to 0.0.0.0:**
```
// In Docker, bind to all interfaces
router.listen("0.0.0.0:8080").await?;
```

2. **Dockerfile:**
```
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3
COPY --from=builder /app/target/release/your-server /usr/local/bin/
EXPOSE 8080
CMD ["your-server"]
```

3. **docker-compose.yml:**
```
version: '3'
services:
  websocket:
    build: .
    ports:
      - "8080:8080"
    environment:
      - RUST_LOG=info
```

---

## Debugging Tips

### Enable Detailed Logging

```
# Debug level
RUST_LOG=wsforge=debug cargo run

# Trace level (very verbose)
RUST_LOG=wsforge=trace cargo run

# Multiple modules
RUST_LOG=wsforge=debug,tokio=info cargo run
```

### Use dbg! Macro

```
async fn handler(msg: Message) -> Result<String> {
    dbg!(&msg); // Prints debug info
    dbg!(msg.as_text());
    Ok("done".to_string())
}
```

### Network Debugging Tools

```
# Monitor WebSocket traffic
websocat -v ws://localhost:8080

# Use tcpdump
sudo tcpdump -i lo -A 'port 8080'

# Browser DevTools
# Open Chrome DevTools → Network → WS tab
```

### Test with Simple Clients

```
# Python test client
import asyncio
import websockets

async def test():
    async with websockets.connect('ws://localhost:8080') as ws:
        await ws.send("Hello")
        response = await ws.recv()
        print(f"Received: {response}")

asyncio.run(test())
```

---

## Getting Help

If you're still stuck:

1. **Check GitHub Issues**: [github.com/aarambhdevhub/wsforge/issues](https://github.com/aarambhdevhub/wsforge/issues)
2. **Create a Minimal Example**: Reproduce in smallest possible code
3. **Provide Details**:
   - Rust version (`rustc --version`)
   - WsForge version
   - Operating system
   - Full error message
   - Minimal reproduction code

4. **Join Community**:
   - GitHub Discussions
   - YouTube: [@AarambhDevHub](https://youtube.com/@AarambhDevHub)

---

**Still having issues?** Open an issue with:
- Clear title
- Steps to reproduce
- Expected vs actual behavior
- Environment details
- Code example

We're here to help! 🚀
