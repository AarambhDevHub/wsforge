# WsForge Documentation

Welcome to the WsForge documentation! This guide will help you understand and use WsForge effectively.

## 📚 Documentation Structure

This documentation is organized into focused guides:

### Getting Started
- **[Installation](installation.md)** - Setup and dependencies
- **[Getting Started](getting-started.md)** - Your first WebSocket server
- **[Quick Start](quick-start.md)** - Common patterns and examples

### Core Concepts
- **[Handlers](handlers.md)** - Writing message handlers
- **[Extractors](extractors.md)** - Type-safe data extraction
- **[Routing](routing.md)** - Message routing patterns
- **[State Management](state-management.md)** - Shared application state
- **[Broadcasting](broadcasting.md)** - Sending messages to multiple clients

### Advanced Topics
- **[Static Files](static-files.md)** - Serving HTML/CSS/JS
- **[Error Handling](error-handling.md)** - Managing errors effectively
- **[Testing](testing.md)** - Writing tests for handlers
- **[Performance](performance.md)** - Optimization techniques
- **[Deployment](deployment.md)** - Production deployment

### Reference
- **[API Reference](api-reference.md)** - Complete API documentation
- **[Examples](examples.md)** - Real-world examples
- **[FAQ](faq.md)** - Frequently asked questions
- **[Troubleshooting](troubleshooting.md)** - Common issues and solutions
- **[Migration Guide](migration-guide.md)** - Upgrading between versions

## 🏗️ System Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      Client Applications                        │
│   (Browser, Mobile App, Desktop App, CLI, IoT Device)           │
└───────────────────────────┬─────────────────────────────────────┘
                            │
                            │ WebSocket Connection
                            │ (ws:// or wss://)
                            │
┌───────────────────────────▼─────────────────────────────────────┐
│                       WsForge Server                            │
│                                                                 │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                    Router Layer                            │ │
│  │  -  Route Matching                                         │ │
│  │  -  Connection Management                                  │ │
│  │  -  Lifecycle Hooks (on_connect, on_disconnect)            │ │
│  └────────────────┬──────────────────┬────────────────────────┘ │
│                   │                  │                          │
│  ┌────────────────▼──────┐  ┌───────▼────────┐                 │
│  │  Static File Handler  │  │  Message Router │                 │
│  │  -  Serve HTML/CSS/JS  │  │  -  Route to     │               │
│  │  -  MIME detection     │  │    Handlers     │                │
│  │  -  Path validation    │  │  -  Extract data │               │
│  └───────────────────────┘  └────────┬────────┘                 │
│                                       │                         │
│                      ┌────────────────▼──────────────┐          │
│                      │      Handler Layer            │          │
│                      │  -  Process Messages           │          │
│                      │  -  Execute Business Logic     │          │
│                      │  -  Return Responses           │          │
│                      └────────┬──────────────────────┘          │
│                               │                                  │
│  ┌────────────────────────────▼──────────────────────────────┐  │
│  │                  State & Extractors                        │  │
│  │  -  Application State (Database, Config, Cache)            │  │
│  │  -  Connection Manager (Active WebSocket Connections)      │  │
│  │  -  JSON Parser, Connection Info, Custom Extractors        │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                   │
└───────────────────────────────────────────────────────────────────┘
                            │
                            ▼
               ┌────────────────────────┐
               │   External Services    │
               │  -  Database            │
               │  -  Redis Cache         │
               │  -  Message Queue       │
               │  -  APIs                │
               └────────────────────────┘
```

### Component Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    WsForge Framework                         │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │   wsforge    │  │ wsforge-core │  │ wsforge-macros  │  │
│  │  (main crate)│─▶│  (core lib)  │  │  (proc macros)  │  │
│  │   Prelude    │  │   Router     │  │  #[derive(...)] │  │
│  │   Re-exports │  │   Handler    │  │  Convenience    │  │
│  └──────────────┘  │   Message    │  └─────────────────┘  │
│                     │   Connection │                        │
│                     │   State      │                        │
│                     │   Extractor  │                        │
│                     └──────────────┘                        │
│                            │                                 │
│                            ▼                                 │
│                   ┌────────────────┐                        │
│                   │  tokio-tungstenite                      │
│                   │  (WebSocket Protocol)                   │
│                   └────────────────┘                        │
└─────────────────────────────────────────────────────────────┘
```

### Request Flow

```
1. Client Connection
   │
   ├─▶ TCP Socket established
   │
   ├─▶ WebSocket Upgrade handshake
   │
   └─▶ Connection added to ConnectionManager
       │
       ├─▶ on_connect callback invoked
       └─▶ Connection ID assigned (conn_0, conn_1, ...)

2. Message Reception
   │
   ├─▶ Raw WebSocket frame received
   │
   ├─▶ Frame parsed into Message
   │   (Text, Binary, Ping, Pong, Close)
   │
   ├─▶ Router matches message to handler
   │
   └─▶ Handler execution
       │
       ├─▶ Extractors run (Json, State, Connection, etc.)
       │
       ├─▶ Handler function executes
       │
       └─▶ Response generated (if any)
           │
           └─▶ Response sent back to client

3. Message Broadcasting
   │
   ├─▶ Handler calls manager.broadcast()
   │
   ├─▶ ConnectionManager iterates active connections
   │
   └─▶ Message queued for each connection
       │
       └─▶ Async write tasks send to clients

4. Disconnection
   │
   ├─▶ Connection closed (client or server)
   │
   ├─▶ Connection removed from manager
   │
   └─▶ on_disconnect callback invoked
```

### Connection Manager Architecture

```
┌───────────────────────────────────────────────────────────┐
│            ConnectionManager (Arc<DashMap>)               │
│                                                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │   conn_0     │  │   conn_1     │  │   conn_N     │   │
│  │              │  │              │  │              │   │
│  │  ┌────────┐  │  │  ┌────────┐  │  │  ┌────────┐  │   │
│  │  │ Sender │  │  │  │ Sender │  │  │  │ Sender │  │   │
│  │  │ Channel│  │  │  │ Channel│  │  │  │ Channel│  │   │
│  │  └────┬───┘  │  │  └────┬───┘  │  │  └────┬───┘  │   │
│  │       │      │  │       │      │  │       │      │   │
│  │  ┌────▼───┐  │  │  ┌────▼───┐  │  │  ┌────▼───┐  │   │
│  │  │  Read  │  │  │  │  Read  │  │  │  │  Read  │  │   │
│  │  │  Task  │  │  │  │  Task  │  │  │  │  Task  │  │   │
│  │  └────┬───┘  │  │  └────┬───┘  │  │  └────┬───┘  │   │
│  │       │      │  │       │      │  │       │      │   │
│  │  ┌────▼───┐  │  │  ┌────▼───┐  │  │  ┌────▼───┐  │   │
│  │  │ Write  │  │  │  │ Write  │  │  │  │ Write  │  │   │
│  │  │  Task  │  │  │  │  Task  │  │  │  │  Task  │  │   │
│  │  └────┬───┘  │  │  └────┬───┘  │  │  └────┬───┘  │   │
│  │       │      │  │       │      │  │       │      │   │
│  │  ┌────▼───┐  │  │  ┌────▼───┐  │  │  ┌────▼───┐  │   │
│  │  │WebSocket│ │  │  │WebSocket│ │  │  │WebSocket│ │   │
│  │  │ Stream │  │  │  │ Stream │  │  │  │ Stream │  │   │
│  │  └────────┘  │  │  └────────┘  │  │  └────────┘  │   │
│  └──────────────┘  └──────────────┘  └──────────────┘   │
│                                                            │
│  Lock-free concurrent access via DashMap                  │
└───────────────────────────────────────────────────────────┘
```

### Handler Execution Flow

```
Request Message
      │
      ▼
┌──────────────────┐
│  FromMessage     │
│  Trait Execution │
│                  │
│  Extractors run: │
│  -  Json<T>       │
│  -  State<T>      │
│  -  Connection    │
│  -  ConnectInfo   │
│  -  Custom        │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│  Handler         │
│  Function        │
│  Execution       │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│  IntoResponse    │
│  Trait           │
│                  │
│  Convert:        │
│  -  String → Text │
│  -  Message → Raw │
│  -  () → None     │
│  -  Result<T>     │
└────────┬─────────┘
         │
         ▼
  Response (Option<Message>)
```

## 🚀 Quick Start

### Minimal Example

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

### Feature Overview

| Feature | Description | Documentation |
|---------|-------------|---------------|
| 🎯 **Handlers** | Async functions to process messages | [handlers.md](handlers.md) |
| 🔧 **Extractors** | Type-safe data extraction | [extractors.md](extractors.md) |
| 📡 **Broadcasting** | Send to multiple clients | [broadcasting.md](broadcasting.md) |
| 🗺️ **Routing** | Route messages to handlers | [routing.md](routing.md) |
| 💾 **State** | Shared application data | [state-management.md](state-management.md) |
| 📁 **Static Files** | Serve HTML/CSS/JS | [static-files.md](static-files.md) |

## 🎓 Learning Path

### Beginner
1. [Installation](installation.md) - Set up your environment
2. [Getting Started](getting-started.md) - First server
3. [Quick Start](quick-start.md) - Common patterns
4. [Handlers](handlers.md) - Write handlers
5. [Examples](examples.md) - Real examples

### Intermediate
1. [Extractors](extractors.md) - Advanced extraction
2. [State Management](state-management.md) - Shared data
3. [Broadcasting](broadcasting.md) - Multi-client messaging
4. [Routing](routing.md) - Complex routing
5. [Error Handling](error-handling.md) - Proper errors

### Advanced
1. [Static Files](static-files.md) - Hybrid servers
2. [Testing](testing.md) - Test your code
3. [Performance](performance.md) - Optimization
4. [Deployment](deployment.md) - Production setup

## 🔗 External Resources

- **[GitHub Repository](https://github.com/aarambhdevhub/wsforge)** - Source code
- **[API Docs](https://docs.rs/wsforge)** - Rust documentation
- **[Examples](https://github.com/aarambhdevhub/wsforge/tree/main/examples)** - Full examples
- **[YouTube Tutorials](https://youtube.com/@AarambhDevHub)** - Video guides

## 💬 Community

- **Issues**: [GitHub Issues](https://github.com/aarambhdevhub/wsforge/issues)
- **Discussions**: [GitHub Discussions](https://github.com/aarambhdevhub/wsforge/discussions)
- **Contributing**: [CONTRIBUTING.md](../CONTRIBUTING.md)

## 📄 License

WsForge is licensed under the MIT License. See [LICENSE](../LICENSE) for details.

---

**Ready to start?** Begin with [Installation](installation.md) →
