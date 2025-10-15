//! # WsForge - High-Performance WebSocket Framework for Rust
//!
//! WsForge is a complete, production-ready WebSocket framework that combines exceptional
//! performance with an intuitive, type-safe API. Built on `tokio-tungstenite`, it provides
//! everything needed to build real-time applications, from simple echo servers to complex
//! multiplayer games and chat platforms.
//!
//! ## Overview
//!
//! WsForge brings together the best practices from modern web frameworks like Axum and
//! combines them with the performance of Rust's async ecosystem. Whether you're building
//! a chat application, real-time dashboard, collaborative editor, or multiplayer game,
//! WsForge provides the tools you need.
//!
//! ## 🌟 Key Features
//!
//! - **🚀 High Performance**: Built on tokio-tungstenite with zero-copy optimizations
//! - **🔧 Type-Safe Extractors**: Automatic extraction of JSON, State, Connection info
//! - **🎯 Flexible Handlers**: Return String, Message, Result, JsonResponse, or ()
//! - **📡 Broadcasting**: Built-in broadcast, broadcast_except, and targeted messaging
//! - **⚡ Concurrent**: Lock-free connection management using DashMap
//! - **🔄 Lifecycle Hooks**: on_connect and on_disconnect callbacks
//! - **🌐 Hybrid Server**: Serve static files and WebSocket on the same port
//! - **🛡️ Type Safety**: Compile-time guarantees prevent common errors
//! - **🎨 Developer Friendly**: Intuitive API similar to popular Rust web frameworks
//! - **📦 Batteries Included**: Macros, examples, and documentation
//!
//! ## Quick Start
//!
//! Add WsForge to your `Cargo.toml`:
//!
//! ```
//! [dependencies]
//! wsforge = "0.1.0"
//! tokio = { version = "1.40", features = ["full"] }
//! serde = { version = "1.0", features = ["derive"] }
//! serde_json = "1.0"
//! ```
//!
//! ### Echo Server Example
//!
//! Create a simple echo server in just a few lines:
//!
//! ```
//! use wsforge::prelude::*;
//!
//! async fn echo(msg: Message) -> Result<Message> {
//!     Ok(msg)
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let router = Router::new()
//!         .default_handler(handler(echo));
//!
//!     println!("WebSocket server running on ws://127.0.0.1:8080");
//!     router.listen("127.0.0.1:8080").await?;
//!     Ok(())
//! }
//! ```
//!
//! ### Chat Server Example
//!
//! Build a real-time chat server with broadcasting:
//!
//! ```
//! use wsforge::prelude::*;
//! use std::sync::Arc;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Deserialize, Serialize)]
//! struct ChatMessage {
//!     username: String,
//!     text: String,
//! }
//!
//! async fn chat_handler(
//!     Json(msg): Json<ChatMessage>,
//!     conn: Connection,
//!     State(manager): State<Arc<ConnectionManager>>,
//! ) -> Result<()> {
//!     println!("{}: {}", msg.username, msg.text);
//!
//!     // Broadcast to everyone except sender
//!     let response = serde_json::to_string(&msg)?;
//!     manager.broadcast_except(conn.id(), Message::text(response));
//!
//!     Ok(())
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let router = Router::new()
//!         .default_handler(handler(chat_handler))
//!         .on_connect(|manager, conn_id| {
//!             println!("✅ {} joined (Total: {})", conn_id, manager.count());
//!         })
//!         .on_disconnect(|manager, conn_id| {
//!             println!("❌ {} left (Total: {})", conn_id, manager.count());
//!         });
//!
//!     router.listen("127.0.0.1:8080").await?;
//!     Ok(())
//! }
//! ```
//!
//! ### Web Application Example
//!
//! Create a hybrid server that serves static files and handles WebSocket connections:
//!
//! ```
//! use wsforge::prelude::*;
//! use std::sync::Arc;
//!
//! async fn ws_handler(
//!     msg: Message,
//!     State(manager): State<Arc<ConnectionManager>>,
//! ) -> Result<()> {
//!     manager.broadcast(msg);
//!     Ok(())
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let router = Router::new()
//!         .serve_static("public")  // Serve HTML/CSS/JS from ./public
//!         .default_handler(handler(ws_handler));
//!
//!     println!("Server running on http://127.0.0.1:8080");
//!     router.listen("127.0.0.1:8080").await?;
//!     Ok(())
//! }
//! ```
//!
//! ## 📚 Core Concepts
//!
//! ### Handlers
//!
//! Handlers are async functions that process WebSocket messages. They can extract
//! data using type-safe extractors and return various response types:
//!
//! ```
//! use wsforge::prelude::*;
//! use std::sync::Arc;
//!
//! // Simple handler
//! async fn simple() -> Result<String> {
//!     Ok("Hello!".to_string())
//! }
//!
//! // Handler with extractors
//! async fn with_extractors(
//!     msg: Message,
//!     conn: Connection,
//!     State(manager): State<Arc<ConnectionManager>>,
//! ) -> Result<()> {
//!     println!("Received from {}: {:?}", conn.id(), msg);
//!     Ok(())
//! }
//! ```
//!
//! ### Extractors
//!
//! Extractors automatically parse and validate data from messages and context:
//!
//! | Extractor | Description |
//! |-----------|-------------|
//! | `Message` | Raw WebSocket message |
//! | `Json<T>` | Deserialize JSON automatically |
//! | `Connection` | Access to the active connection |
//! | `State<T>` | Shared application state |
//! | `ConnectInfo` | Connection metadata |
//! | `Data` | Raw binary data |
//!
//! ### Broadcasting
//!
//! Send messages to multiple connections efficiently:
//!
//! ```
//! use wsforge::prelude::*;
//! use std::sync::Arc;
//!
//! async fn broadcast_example(
//!     msg: Message,
//!     conn: Connection,
//!     State(manager): State<Arc<ConnectionManager>>,
//! ) -> Result<()> {
//!     // Broadcast to all
//!     manager.broadcast(msg.clone());
//!
//!     // Broadcast except sender
//!     manager.broadcast_except(conn.id(), msg.clone());
//!
//!     // Broadcast to specific connections
//!     let targets = vec!["conn_1".to_string(), "conn_2".to_string()];
//!     manager.broadcast_to(&targets, msg);
//!
//!     Ok(())
//! }
//! ```
//!
//! ### State Management
//!
//! Share data across all connections:
//!
//! ```
//! use wsforge::prelude::*;
//! use std::sync::Arc;
//!
//! struct Database {
//!     // Database connection pool
//! }
//!
//! struct Config {
//!     max_connections: usize,
//! }
//!
//! async fn handler(
//!     State(db): State<Arc<Database>>,
//!     State(config): State<Arc<Config>>,
//! ) -> Result<String> {
//!     Ok(format!("Max connections: {}", config.max_connections))
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let router = Router::new()
//!         .with_state(Arc::new(Database {}))
//!         .with_state(Arc::new(Config { max_connections: 100 }))
//!         .default_handler(handler(handler));
//!
//!     router.listen("127.0.0.1:8080").await?;
//!     Ok(())
//! }
//! ```
//!
//! ## 🎮 Complete Examples
//!
//! ### Real-Time Game Server
//!
//! ```
//! use wsforge::prelude::*;
//! use std::sync::Arc;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Deserialize, Serialize)]
//! struct GameMove {
//!     player_id: u64,
//!     x: f32,
//!     y: f32,
//! }
//!
//! async fn game_handler(
//!     Json(game_move): Json<GameMove>,
//!     conn: Connection,
//!     State(manager): State<Arc<ConnectionManager>>,
//! ) -> Result<()> {
//!     // Broadcast move to all other players
//!     let json = serde_json::to_string(&game_move)?;
//!     manager.broadcast_except(conn.id(), Message::text(json));
//!     Ok(())
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let router = Router::new()
//!         .default_handler(handler(game_handler))
//!         .on_connect(|manager, conn_id| {
//!             println!("🎮 Player {} joined", conn_id);
//!         });
//!
//!     router.listen("0.0.0.0:8080").await?;
//!     Ok(())
//! }
//! ```
//!
//! ### Multi-Room Chat
//!
//! ```
//! use wsforge::prelude::*;
//! use std::sync::Arc;
//! use std::collections::HashMap;
//! use tokio::sync::RwLock;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Clone)]
//! struct RoomManager {
//!     rooms: Arc<RwLock<HashMap<String, Vec<String>>>>,
//! }
//!
//! #[derive(Deserialize)]
//! struct RoomMessage {
//!     room: String,
//!     text: String,
//! }
//!
//! async fn room_handler(
//!     Json(msg): Json<RoomMessage>,
//!     conn: Connection,
//!     State(room_mgr): State<Arc<RoomManager>>,
//!     State(conn_mgr): State<Arc<ConnectionManager>>,
//! ) -> Result<()> {
//!     // Get room members and broadcast
//!     let rooms = room_mgr.rooms.read().await;
//!     if let Some(members) = rooms.get(&msg.room) {
//!         let json = serde_json::to_string(&msg)?;
//!         conn_mgr.broadcast_to(members, Message::text(json));
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## 🔧 Advanced Features
//!
//! ### Custom Middleware
//!
//! ```
//! use wsforge::prelude::*;
//!
//! async fn auth_middleware(
//!     msg: Message,
//!     extensions: &Extensions,
//! ) -> Result<()> {
//!     // Verify authentication token
//!     let token = msg.as_text().ok_or(Error::custom("Invalid token"))?;
//!
//!     // Store user info in extensions
//!     extensions.insert("user_id", 123_u64);
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Graceful Shutdown
//!
//! ```
//! use wsforge::prelude::*;
//! use tokio::signal;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let router = Router::new();
//!
//!     tokio::select! {
//!         result = router.listen("127.0.0.1:8080") => {
//!             if let Err(e) = result {
//!                 eprintln!("Server error: {}", e);
//!             }
//!         }
//!         _ = signal::ctrl_c() => {
//!             println!("Shutting down gracefully...");
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## 📊 Performance
//!
//! WsForge is designed for high performance:
//!
//! - **Concurrent Connections**: Handles thousands of connections efficiently
//! - **Lock-Free**: DashMap-based connection management eliminates lock contention
//! - **Zero-Copy**: Minimizes allocations and copies where possible
//! - **Async Native**: Built on tokio for maximum async performance
//! - **Benchmarked**: 47K+ requests/second for simple echo operations
//!
//! ## 🛡️ Security
//!
//! - **Path Traversal Protection**: Static file handler prevents directory escapes
//! - **Type Safety**: Rust's type system prevents common errors
//! - **Input Validation**: JSON parsing with serde provides automatic validation
//! - **Connection Limits**: Easy to implement rate limiting and connection caps
//!
//! ## 📖 Documentation
//!
//! - [GitHub Repository](https://github.com/aarambhdevhub/wsforge)
//! - [API Documentation](https://docs.rs/wsforge)
//! - [Examples](https://github.com/aarambhdevhub/wsforge/tree/main/examples)
//! - [Tutorial Series](https://youtube.com/@AarambhDevHub)
//!
//! ## 🤝 Contributing
//!
//! Contributions are welcome! Please see our [Contributing Guide](https://github.com/aarambhdevhub/wsforge/blob/main/CONTRIBUTING.md).
//!
//! ## 📝 License
//!
//! Licensed under the MIT License. See [LICENSE](https://github.com/aarambhdevhub/wsforge/blob/main/LICENSE) for details.
//!
//! ## 🙏 Acknowledgments
//!
//! - Built with [tokio-tungstenite](https://github.com/snapview/tokio-tungstenite)
//! - Inspired by [Axum](https://github.com/tokio-rs/axum)
//! - Thanks to the Rust community
//!
//! ## 🔗 Links
//!
//! - **Author**: [Aarambh Dev Hub](https://github.com/AarambhDevHub)
//! - **YouTube**: [@AarambhDevHub](https://youtube.com/@AarambhDevHub)
//! - **Support**: [Issues](https://github.com/aarambhdevhub/wsforge/issues)

// Enable documentation features for docs.rs
#![cfg_attr(docsrs, feature(doc_cfg))]
// Deny missing docs to ensure comprehensive documentation
#![warn(missing_docs)]
// Enable additional documentation lint rules
#![warn(rustdoc::missing_crate_level_docs)]

// Re-export everything from wsforge-core
pub use wsforge_core::*;

// Re-export macros if enabled
#[cfg(feature = "macros")]
pub use wsforge_macros::*;

/// Prelude module for convenient imports.
///
/// Import this module to bring all commonly used types and traits into scope:
///
/// ```
/// use wsforge::prelude::*;
///
/// async fn handler(msg: Message) -> Result<String> {
///     Ok("response".to_string())
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let router = Router::new()
///         .default_handler(handler(handler));
///
///     router.listen("127.0.0.1:8080").await?;
///     Ok(())
/// }
/// ```
///
/// ## Included Types
///
/// - **Core**: `Router`, `Message`, `Connection`, `ConnectionManager`
/// - **Handlers**: `handler()`, `Handler`, `IntoResponse`, `JsonResponse`
/// - **Extractors**: `Json`, `State`, `ConnectInfo`, `Data`, `Extension`
/// - **Errors**: `Error`, `Result`
/// - **State**: `AppState`, `Extensions`
/// - **Types**: `MessageType`, `ConnectionId`
/// - **Static Files**: `StaticFileHandler`
pub mod prelude {
    pub use wsforge_core::prelude::*;

    #[cfg(feature = "macros")]
    pub use wsforge_macros::*;
}
