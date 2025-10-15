# Performance Guide

This guide covers performance optimization techniques, benchmarking, and best practices for building high-performance WebSocket applications with WsForge.

## Performance Characteristics

### WsForge Performance Features

WsForge is designed from the ground up for high performance:

- **Lock-Free Concurrency**: DashMap-based connection management eliminates lock contention
- **Zero-Copy Operations**: Minimizes allocations and memory copies where possible
- **Async Native**: Built on tokio for maximum async I/O performance
- **Efficient Broadcasting**: Optimized message distribution to multiple connections
- **Minimal Overhead**: Handler system adds negligible overhead

### Benchmark Results

On modern hardware (AMD Ryzen 5/Intel i5 equivalent):

| Operation | Throughput | Latency |
|-----------|-----------|---------|
| Echo (simple) | 47,000+ req/s | <1ms |
| JSON parsing | 35,000+ req/s | <2ms |
| Broadcast (100 connections) | 25,000+ msg/s | <5ms |
| Connection establishment | 5,000+ conn/s | <10ms |

*Benchmarks performed with tokio runtime, single thread per core*

## Optimization Techniques

### 1. Connection Management

#### Limit Concurrent Connections

Set reasonable limits to prevent resource exhaustion:

```
use wsforge::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct ConnectionLimiter {
    count: AtomicUsize,
    max: usize,
}

impl ConnectionLimiter {
    fn try_acquire(&self) -> bool {
        let current = self.count.fetch_add(1, Ordering::SeqCst);
        if current >= self.max {
            self.count.fetch_sub(1, Ordering::SeqCst);
            false
        } else {
            true
        }
    }

    fn release(&self) {
        self.count.fetch_sub(1, Ordering::SeqCst);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let limiter = Arc::new(ConnectionLimiter {
        count: AtomicUsize::new(0),
        max: 10000,
    });

    let router = Router::new()
        .on_connect({
            let limiter = limiter.clone();
            move |_manager, conn_id| {
                if !limiter.try_acquire() {
                    println!("⚠️ Connection limit reached, rejecting {}", conn_id);
                }
            }
        })
        .on_disconnect({
            let limiter = limiter.clone();
            move |_manager, _conn_id| {
                limiter.release();
            }
        });

    router.listen("0.0.0.0:8080").await
}
```

#### Implement Heartbeats

Detect and clean up dead connections:

```
use tokio::time::{interval, Duration};

async fn heartbeat_monitor(manager: Arc<ConnectionManager>) {
    let mut ticker = interval(Duration::from_secs(30));

    loop {
        ticker.tick().await;

        // Send ping to all connections
        manager.broadcast(Message::ping(vec![]));

        println!("💓 Heartbeat sent to {} connections", manager.count());
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let router = Router::new();
    let manager = router.connection_manager();

    // Spawn heartbeat task
    tokio::spawn(heartbeat_monitor(manager.clone()));

    router.listen("0.0.0.0:8080").await
}
```

### 2. Message Optimization

#### Use Binary Formats

Binary formats are more efficient than JSON for large payloads:

```
use wsforge::prelude::*;

// Instead of JSON
async fn json_handler(Json(data): Json<serde_json::Value>) -> Result<String> {
    Ok(format!("{}", data))
}

// Use MessagePack or Protocol Buffers
async fn binary_handler(Data(bytes): Data) -> Result<Vec<u8>> {
    // Process binary data directly
    Ok(bytes)
}
```

#### Batch Small Messages

Combine multiple small messages to reduce overhead:

```
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

struct MessageBatcher {
    buffer: Arc<Mutex<Vec<Message>>>,
    max_size: usize,
}

impl MessageBatcher {
    async fn add(&self, msg: Message, manager: &ConnectionManager) {
        let mut buffer = self.buffer.lock().await;
        buffer.push(msg);

        if buffer.len() >= self.max_size {
            let batched = self.create_batch(&buffer);
            manager.broadcast(batched);
            buffer.clear();
        }
    }

    fn create_batch(&self, messages: &[Message]) -> Message {
        // Combine messages into single payload
        let combined = messages.iter()
            .filter_map(|m| m.as_text())
            .collect::<Vec<_>>()
            .join("\n");
        Message::text(combined)
    }
}

async fn flush_batch_periodically(batcher: Arc<MessageBatcher>, manager: Arc<ConnectionManager>) {
    let mut ticker = interval(Duration::from_millis(100));

    loop {
        ticker.tick().await;
        let mut buffer = batcher.buffer.lock().await;
        if !buffer.is_empty() {
            let batched = batcher.create_batch(&buffer);
            manager.broadcast(batched);
            buffer.clear();
        }
    }
}
```

#### Compress Large Payloads

For text-heavy messages, consider compression:

```
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;

async fn compress_handler(msg: Message) -> Result<Vec<u8>> {
    if msg.as_bytes().len() > 1024 {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(msg.as_bytes())?;
        Ok(encoder.finish()?)
    } else {
        Ok(msg.as_bytes().to_vec())
    }
}
```

### 3. Broadcasting Optimization

#### Avoid Unnecessary Broadcasts

Only broadcast when needed:

```
async fn smart_broadcast(
    msg: Message,
    conn: Connection,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<()> {
    // Only broadcast if there are other users
    if manager.count() > 1 {
        manager.broadcast_except(conn.id(), msg);
    }
    Ok(())
}
```

#### Use Targeted Broadcasting

Send to specific groups instead of everyone:

```
use std::collections::HashMap;
use tokio::sync::RwLock;

struct RoomManager {
    rooms: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

async fn room_broadcast(
    room: &str,
    msg: Message,
    room_mgr: &RoomManager,
    conn_mgr: &ConnectionManager,
) {
    let rooms = room_mgr.rooms.read().await;
    if let Some(members) = rooms.get(room) {
        conn_mgr.broadcast_to(members, msg);
    }
}
```

### 4. Handler Optimization

#### Minimize Async Operations

Reduce await points in hot paths:

```
// Slow - multiple awaits
async fn slow_handler(msg: Message) -> Result<String> {
    let data = msg.json::<serde_json::Value>()?;
    let result = process_data(data).await?;
    let formatted = format_result(result).await?;
    Ok(formatted)
}

// Fast - combine operations
async fn fast_handler(msg: Message) -> Result<String> {
    let data = msg.json::<serde_json::Value>()?;
    Ok(process_and_format(data))
}

fn process_and_format(data: serde_json::Value) -> String {
    // Synchronous processing
    format!("{}", data)
}
```

#### Use Efficient Data Structures

Choose the right structure for your use case:

```
use dashmap::DashMap;
use std::sync::Arc;

// Good for concurrent access
struct UserStore {
    users: Arc<DashMap<u64, User>>,
}

// For read-heavy workloads
struct CachedData {
    data: Arc<RwLock<HashMap<String, String>>>,
}
```

### 5. Memory Management

#### Reuse Buffers

Avoid allocations in hot paths:

```
use bytes::BytesMut;

struct BufferPool {
    buffers: Arc<Mutex<Vec<BytesMut>>>,
}

impl BufferPool {
    async fn get(&self) -> BytesMut {
        let mut pool = self.buffers.lock().await;
        pool.pop().unwrap_or_else(|| BytesMut::with_capacity(4096))
    }

    async fn return_buffer(&self, mut buf: BytesMut) {
        buf.clear();
        let mut pool = self.buffers.lock().await;
        if pool.len() < 100 {
            pool.push(buf);
        }
    }
}
```

#### Set Message Size Limits

Prevent memory exhaustion from large messages:

```
const MAX_MESSAGE_SIZE: usize = 1024 * 1024; // 1MB

async fn size_limited_handler(msg: Message) -> Result<String> {
    if msg.as_bytes().len() > MAX_MESSAGE_SIZE {
        return Err(Error::custom("Message too large"));
    }
    Ok("processed".to_string())
}
```

## Monitoring and Profiling

### Metrics Collection

Track key performance metrics:

```
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

struct Metrics {
    messages_received: AtomicU64,
    messages_sent: AtomicU64,
    bytes_received: AtomicU64,
    bytes_sent: AtomicU64,
    start_time: Instant,
}

impl Metrics {
    fn record_message_received(&self, size: usize) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(size as u64, Ordering::Relaxed);
    }

    fn stats(&self) -> String {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let msg_rate = self.messages_received.load(Ordering::Relaxed) as f64 / elapsed;
        format!("Messages/sec: {:.2}", msg_rate)
    }
}
```

### Profiling Tools

#### CPU Profiling

```
# Install cargo-flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --bin your-app

# Opens flamegraph.svg in browser
```

#### Memory Profiling

```
# Use valgrind
valgrind --tool=massif target/release/your-app

# Or heaptrack
heaptrack target/release/your-app
```

### Runtime Monitoring

Use tracing for production monitoring:

```
use tracing::{info, warn, instrument};

#[instrument(skip(manager))]
async fn monitored_handler(
    msg: Message,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<()> {
    info!("Processing message, {} connections active", manager.count());

    if manager.count() > 5000 {
        warn!("High connection count: {}", manager.count());
    }

    manager.broadcast(msg);
    Ok(())
}
```

## Best Practices

### 1. Connection Pooling

Reuse connections when possible:

```
// Keep connections alive with periodic pings
async fn keep_alive_task(manager: Arc<ConnectionManager>) {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;
        manager.broadcast(Message::ping(vec![]));
    }
}
```

### 2. Graceful Degradation

Handle high load gracefully:

```
async fn rate_limited_handler(
    msg: Message,
    State(limiter): State<Arc<RateLimiter>>,
) -> Result<String> {
    if !limiter.check_limit() {
        return Err(Error::custom("Rate limit exceeded"));
    }

    Ok("processed".to_string())
}
```

### 3. Async Best Practices

- Use `tokio::spawn` for CPU-intensive tasks
- Avoid blocking operations in async code
- Use `tokio::task::spawn_blocking` for sync code
- Keep critical sections small

```
async fn efficient_async(msg: Message) -> Result<String> {
    // Quick async operation
    let data = msg.json::<Value>()?;

    // CPU-intensive work in blocking task
    let result = tokio::task::spawn_blocking(move || {
        expensive_computation(data)
    }).await?;

    Ok(result)
}

fn expensive_computation(data: Value) -> String {
    // Heavy CPU work here
    format!("{}", data)
}
```

## Performance Tuning

### Tokio Runtime Configuration

```
#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() -> Result<()> {
    // 8 worker threads for CPU-bound tasks
    Router::new().listen("0.0.0.0:8080").await
}
```

### TCP Tuning

For high-throughput applications:

```
use tokio::net::TcpSocket;

async fn configure_socket() -> std::io::Result<TcpSocket> {
    let socket = TcpSocket::new_v4()?;

    // Enable TCP_NODELAY to disable Nagle's algorithm
    socket.set_nodelay(true)?;

    // Set buffer sizes
    socket.set_send_buffer_size(262144)?; // 256KB
    socket.set_recv_buffer_size(262144)?;

    Ok(socket)
}
```

## Common Performance Pitfalls

### ❌ Don't: Clone Large Data

```
// Bad - clones entire message for each connection
async fn bad_broadcast(msg: Message, manager: &ConnectionManager) {
    for id in manager.all_ids() {
        let conn = manager.get(&id).unwrap();
        conn.send(msg.clone()); // Expensive!
    }
}
```

### ✅ Do: Use Built-in Broadcast

```
// Good - optimized internal implementation
async fn good_broadcast(msg: Message, manager: &ConnectionManager) {
    manager.broadcast(msg);
}
```

### ❌ Don't: Block the Runtime

```
// Bad - blocks async runtime
async fn blocking_handler(msg: Message) -> Result<String> {
    std::thread::sleep(Duration::from_secs(1)); // DON'T!
    Ok("done".to_string())
}
```

### ✅ Do: Use Async Sleep

```
// Good - yields to runtime
async fn async_handler(msg: Message) -> Result<String> {
    tokio::time::sleep(Duration::from_secs(1)).await;
    Ok("done".to_string())
}
```

## Load Testing

### Using `websocat`

```
# Install websocat
cargo install websocat

# Connect and send messages
echo "test message" | websocat ws://localhost:8080
```

### Using Custom Load Test

```
use tokio_tungstenite::connect_async;
use futures_util::StreamExt;

#[tokio::test]
async fn load_test() {
    let mut tasks = vec![];

    for i in 0..1000 {
        let task = tokio::spawn(async move {
            let (ws, _) = connect_async("ws://localhost:8080").await.unwrap();
            // Send messages...
        });
        tasks.push(task);
    }

    for task in tasks {
        task.await.unwrap();
    }
}
```

## Deployment Optimization

### Use Release Mode

Always benchmark and deploy in release mode:

```
cargo build --release
./target/release/your-app
```

### Profile-Guided Optimization (PGO)

```
[profile.release]
lto = true
codegen-units = 1
opt-level = 3
```

### Platform-Specific Optimizations

```
[profile.release]
target-cpu = "native"  # Use CPU-specific instructions
```

## Summary

Key takeaways for high-performance WsForge applications:

1. **Limit connections** and implement heartbeats
2. **Use binary formats** for large payloads
3. **Batch messages** to reduce overhead
4. **Monitor metrics** in production
5. **Profile regularly** to find bottlenecks
6. **Use async properly** - don't block the runtime
7. **Test under load** before deployment

For more information:
- [Broadcasting Guide](broadcasting.md)
- [State Management](state-management.md)
- [Testing Guide](testing.md)
