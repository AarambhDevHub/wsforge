# Testing Guide

Comprehensive guide to testing WsForge WebSocket applications.

## Table of Contents

- [Testing Overview](#testing-overview)
- [Unit Testing](#unit-testing)
- [Integration Testing](#integration-testing)
- [Testing Handlers](#testing-handlers)
- [Testing with Mock Connections](#testing-with-mock-connections)
- [Testing Broadcast Functionality](#testing-broadcast-functionality)
- [Error Scenario Testing](#error-scenario-testing)
- [Performance Testing](#performance-testing)
- [Testing Best Practices](#testing-best-practices)
- [CI/CD Integration](#cicd-integration)

## Testing Overview

WsForge applications should be tested at multiple levels to ensure reliability and correctness.

### Testing Pyramid

```
        /\
       /  \     E2E Tests (Few)
      /____\
     /      \   Integration Tests (Some)
    /________\
   /          \  Unit Tests (Many)
  /____________\
```

### Test Types

- **Unit Tests**: Test individual functions and components
- **Integration Tests**: Test component interactions
- **End-to-End Tests**: Test full application workflows
- **Performance Tests**: Measure throughput and latency

## Unit Testing

### Testing Handlers

Test handler functions in isolation:

```
use wsforge::prelude::*;

async fn echo_handler(msg: Message) -> Result<Message> {
    Ok(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_echo_handler() {
        let msg = Message::text("hello");
        let result = echo_handler(msg.clone()).await.unwrap();

        assert_eq!(result.as_text(), Some("hello"));
        assert!(result.is_text());
    }

    #[tokio::test]
    async fn test_echo_handler_binary() {
        let data = vec!;[1][4][11]
        let msg = Message::binary(data.clone());
        let result = echo_handler(msg).await.unwrap();

        assert_eq!(result.as_bytes(), &data[..]);
        assert!(result.is_binary());
    }
}
```

### Testing JSON Handlers

Test handlers that use JSON extraction:

```
use wsforge::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq)]
struct Request {
    action: String,
    value: i32,
}

async fn json_handler(Json(req): Json<Request>) -> Result<String> {
    Ok(format!("Action: {}, Value: {}", req.action, req.value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_json_handler() {
        let request = Request {
            action: "increment".to_string(),
            value: 42,
        };

        let json = serde_json::to_string(&request).unwrap();
        let msg = Message::text(json);

        // To test Json extractor, we need to extract manually
        let parsed: Request = msg.json().unwrap();
        let result = json_handler(Json(parsed)).await.unwrap();

        assert_eq!(result, "Action: increment, Value: 42");
    }

    #[tokio::test]
    async fn test_json_handler_invalid() {
        let invalid_json = "not valid json";
        let msg = Message::text(invalid_json);

        let result: Result<Request, _> = msg.json();
        assert!(result.is_err());
    }
}
```

### Testing Message Types

Test message creation and manipulation:

```
#[cfg(test)]
mod message_tests {
    use wsforge::prelude::*;

    #[test]
    fn test_text_message() {
        let msg = Message::text("hello world");
        assert!(msg.is_text());
        assert_eq!(msg.as_text(), Some("hello world"));
        assert_eq!(msg.as_bytes(), b"hello world");
    }

    #[test]
    fn test_binary_message() {
        let data = vec![0xFF, 0xFE, 0xFD];
        let msg = Message::binary(data.clone());
        assert!(msg.is_binary());
        assert_eq!(msg.as_bytes(), &data[..]);
    }

    #[test]
    fn test_json_parsing() {
        #[derive(serde::Deserialize, PartialEq, Debug)]
        struct Data {
            value: i32,
        }

        let json = r#"{"value": 42}"#;
        let msg = Message::text(json);
        let data: Data = msg.json().unwrap();

        assert_eq!(data, Data { value: 42 });
    }
}
```

## Integration Testing

### Testing with Real Server

Create integration tests in `tests/` directory:

```
// tests/integration_test.rs
use wsforge::prelude::*;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use url::Url;

async fn start_test_server() -> String {
    let addr = "127.0.0.1:0"; // Random port

    tokio::spawn(async move {
        let router = Router::new()
            .default_handler(handler(|msg: Message| async move {
                Ok(msg)
            }));

        router.listen(addr).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    "ws://127.0.0.1:8080".to_string()
}

#[tokio::test]
async fn test_echo_integration() {
    let server_url = start_test_server().await;
    let url = Url::parse(&server_url).unwrap();

    let (mut ws_stream, _) = connect_async(url).await.unwrap();

    // Send message
    use futures_util::SinkExt;
    ws_stream.send(WsMessage::Text("test".into())).await.unwrap();

    // Receive response
    use futures_util::StreamExt;
    if let Some(msg) = ws_stream.next().await {
        let msg = msg.unwrap();
        assert_eq!(msg.into_text().unwrap(), "test");
    }
}
```

### Testing Connection Lifecycle

Test connection and disconnection callbacks:

```
#[tokio::test]
async fn test_connection_lifecycle() {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let connected = Arc::new(Mutex::new(false));
    let disconnected = Arc::new(Mutex::new(false));

    let connected_clone = connected.clone();
    let disconnected_clone = disconnected.clone();

    let router = Router::new()
        .on_connect(move |_manager, _conn_id| {
            let c = connected_clone.clone();
            tokio::spawn(async move {
                *c.lock().await = true;
            });
        })
        .on_disconnect(move |_manager, _conn_id| {
            let d = disconnected_clone.clone();
            tokio::spawn(async move {
                *d.lock().await = true;
            });
        });

    // Test logic here
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
}
```

## Testing with Mock Connections

### Creating Mock Connections

```
use wsforge::prelude::*;
use tokio::sync::mpsc;
use std::net::SocketAddr;

fn create_mock_connection(id: &str) -> Connection {
    let (tx, _rx) = mpsc::unbounded_channel();
    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    Connection::new(id.to_string(), addr, tx)
}

#[tokio::test]
async fn test_with_mock_connection() {
    let conn = create_mock_connection("test_conn");
    assert_eq!(conn.id(), "test_conn");

    // Test sending messages
    let result = conn.send_text("test message");
    assert!(result.is_ok());
}
```

### Testing State Extraction

```
use std::sync::Arc;

#[tokio::test]
async fn test_state_extraction() {
    let state = AppState::new();
    let counter = Arc::new(42_u32);
    state.insert(counter);

    // Test extraction
    let extracted: Arc<u32> = state.get().unwrap();
    assert_eq!(*extracted, 42);
}
```

## Testing Broadcast Functionality

### Testing Broadcast to All

```
use wsforge::prelude::*;
use std::sync::Arc;

#[tokio::test]
async fn test_broadcast_all() {
    let manager = Arc::new(ConnectionManager::new());

    // Create mock connections
    let (tx1, mut rx1) = tokio::sync::mpsc::unbounded_channel();
    let (tx2, mut rx2) = tokio::sync::mpsc::unbounded_channel();

    let addr: std::net::SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let conn1 = Connection::new("conn_1".to_string(), addr, tx1);
    let conn2 = Connection::new("conn_2".to_string(), addr, tx2);

    manager.add(conn1);
    manager.add(conn2);

    // Broadcast message
    let msg = Message::text("broadcast test");
    manager.broadcast(msg);

    // Verify both received
    tokio::time::timeout(
        tokio::time::Duration::from_millis(100),
        async {
            let msg1 = rx1.recv().await;
            let msg2 = rx2.recv().await;
            assert!(msg1.is_some());
            assert!(msg2.is_some());
        }
    ).await.unwrap();
}
```

### Testing Broadcast Except

```
#[tokio::test]
async fn test_broadcast_except() {
    let manager = Arc::new(ConnectionManager::new());

    let (tx1, mut rx1) = tokio::sync::mpsc::unbounded_channel();
    let (tx2, mut rx2) = tokio::sync::mpsc::unbounded_channel();

    let addr: std::net::SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let conn1 = Connection::new("conn_1".to_string(), addr, tx1);
    let conn2 = Connection::new("conn_2".to_string(), addr, tx2);

    manager.add(conn1);
    manager.add(conn2);

    // Broadcast except conn_1
    let msg = Message::text("broadcast except test");
    manager.broadcast_except(&"conn_1".to_string(), msg);

    // Verify only conn_2 received
    tokio::time::timeout(
        tokio::time::Duration::from_millis(100),
        async {
            assert!(rx1.try_recv().is_err()); // conn_1 should not receive
            assert!(rx2.recv().await.is_some()); // conn_2 should receive
        }
    ).await.unwrap();
}
```

## Error Scenario Testing

### Testing Error Handling

```
async fn failing_handler(_msg: Message) -> Result<String> {
    Err(Error::custom("Intentional error"))
}

#[tokio::test]
async fn test_error_handling() {
    let msg = Message::text("test");
    let result = failing_handler(msg).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Intentional error"));
}
```

### Testing Invalid JSON

```
#[tokio::test]
async fn test_invalid_json_handling() {
    let invalid = Message::text("not json");
    let result: Result<serde_json::Value, _> = invalid.json();

    assert!(result.is_err());
}
```

## Performance Testing

### Load Testing

```
#[tokio::test]
async fn test_concurrent_connections() {
    let manager = Arc::new(ConnectionManager::new());

    // Create 1000 connections
    for i in 0..1000 {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let addr: std::net::SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let conn = Connection::new(format!("conn_{}", i), addr, tx);
        manager.add(conn);
    }

    assert_eq!(manager.count(), 1000);
}
```

### Benchmarking

```
// benches/broadcast_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use wsforge::prelude::*;

fn broadcast_benchmark(c: &mut Criterion) {
    c.bench_function("broadcast_1000", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let manager = std::sync::Arc::new(ConnectionManager::new());

        // Setup
        for i in 0..1000 {
            let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
            let addr: std::net::SocketAddr = "127.0.0.1:8080".parse().unwrap();
            let conn = Connection::new(format!("conn_{}", i), addr, tx);
            manager.add(conn);
        }

        b.iter(|| {
            let msg = Message::text("benchmark");
            manager.broadcast(black_box(msg));
        });
    });
}

criterion_group!(benches, broadcast_benchmark);
criterion_main!(benches);
```

## Testing Best Practices

### 1. Use Descriptive Test Names

```
#[tokio::test]
async fn test_handler_returns_uppercase_text() {
    // Test implementation
}

#[tokio::test]
async fn test_handler_rejects_empty_messages() {
    // Test implementation
}
```

### 2. Test Edge Cases

```
#[tokio::test]
async fn test_empty_message() {
    let msg = Message::text("");
    // Test handling
}

#[tokio::test]
async fn test_very_large_message() {
    let large_text = "x".repeat(1_000_000);
    let msg = Message::text(large_text);
    // Test handling
}
```

### 3. Use Test Fixtures

```
struct TestFixture {
    manager: Arc<ConnectionManager>,
    connections: Vec<Connection>,
}

impl TestFixture {
    fn new(count: usize) -> Self {
        let manager = Arc::new(ConnectionManager::new());
        let mut connections = Vec::new();

        for i in 0..count {
            let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
            let addr: std::net::SocketAddr = "127.0.0.1:8080".parse().unwrap();
            let conn = Connection::new(format!("conn_{}", i), addr, tx);
            manager.add(conn.clone());
            connections.push(conn);
        }

        Self { manager, connections }
    }
}

#[tokio::test]
async fn test_with_fixture() {
    let fixture = TestFixture::new(10);
    assert_eq!(fixture.manager.count(), 10);
}
```

### 4. Clean Up Resources

```
#[tokio::test]
async fn test_cleanup() {
    let manager = Arc::new(ConnectionManager::new());

    // Setup
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let addr: std::net::SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let conn = Connection::new("test".to_string(), addr, tx);
    manager.add(conn);

    // Test
    assert_eq!(manager.count(), 1);

    // Cleanup
    manager.remove(&"test".to_string());
    assert_eq!(manager.count(), 0);
}
```

## CI/CD Integration

### GitHub Actions Example

```
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run tests
        run: cargo test --all --verbose

      - name: Run clippy
        run: cargo clippy --all -- -D warnings

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Generate coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out Xml

      - name: Upload coverage
        uses: codecov/codecov-action@v1
```

### Running Tests Locally

```
# Run all tests
cargo test --all

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Run integration tests only
cargo test --test '*'

# Run with coverage
cargo tarpaulin --all-features --workspace
```

## Continuous Testing

Use `cargo-watch` for continuous testing during development:

```
# Install cargo-watch
cargo install cargo-watch

# Watch and test
cargo watch -x test

# Watch, test, and run clippy
cargo watch -x test -x clippy
```

---

**Next Steps:**
- [Performance Guide](performance.md) - Optimize your application
- [Deployment Guide](deployment.md) - Deploy to production
- [Troubleshooting](troubleshooting.md) - Common issues and solutions
