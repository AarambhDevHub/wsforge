//! # WsForge Core - High-Performance WebSocket Framework
//!
//! `wsforge-core` is the foundational library for the WsForge WebSocket framework.
//! It provides type-safe, ergonomic abstractions for building real-time WebSocket applications
//! with exceptional performance and developer experience.
//!
//! ## Overview
//!
//! WsForge Core combines the power of `tokio-tungstenite` with a flexible, type-safe API inspired
//! by modern web frameworks like Axum. It's designed for building production-ready WebSocket
//! servers that are both fast and maintainable.
//!
//! ## Key Features
//!
//! - 🚀 **High Performance**: Built on tokio-tungstenite with zero-copy optimizations
//! - 🔧 **Type-Safe Extractors**: Automatic extraction of JSON, State, Connection info
//! - 🎯 **Flexible Handlers**: Return various types - String, Message, Result, JsonResponse
//! - 📡 **Broadcasting**: Built-in broadcast, broadcast_except, and targeted messaging
//! - ⚡ **Concurrent**: Lock-free connection management with DashMap
//! - 🔄 **Lifecycle Hooks**: on_connect and on_disconnect callbacks
//! - 🌐 **Hybrid Server**: Serve static files and WebSocket on same port
//! - 🛡️ **Type Safety**: Compile-time guarantees for correctness
//!
//! ## Architecture
//!
//! ```
//! ┌──────────────────────────────────────────────────────────────┐
//! │                        Application                            │
//! │  ┌────────────┐  ┌──────────┐  ┌───────────────────────┐   │
//! │  │  Handlers  │  │  Router  │  │  State & Extractors   │   │
//! │  └────────────┘  └──────────┘  └───────────────────────┘   │
//! └──────────────────────────────────────────────────────────────┘
//!                              │
//! ┌──────────────────────────────────────────────────────────────┐
//! │                      WsForge Core                             │
//! │  ┌──────────────┐  ┌────────────────┐  ┌─────────────────┐ │
//! │  │  Connection  │  │     Message     │  │   Static Files  │ │
//! │  │   Manager    │  │     Router      │  │     Handler     │ │
//! │  └──────────────┘  └────────────────┘  └─────────────────┘ │
//! └──────────────────────────────────────────────────────────────┘
//!                              │
//! ┌──────────────────────────────────────────────────────────────┐
//! │                    tokio-tungstenite                          │
//! │                  (WebSocket Protocol)                         │
//! └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Module Structure
//!
//! - [`connection`]: WebSocket connection management and lifecycle
//! - [`message`]: Message types and parsing utilities
//! - [`handler`]: Handler trait and response types
//! - [`extractor`]: Type-safe data extraction from messages
//! - [`router`]: Routing and server management
//! - [`state`]: Shared application state container
//! - [`error`]: Error types and result handling
//! - [`static_files`]: Static file serving for hybrid servers
//!
//! ## Quick Start Examples
//!
//! ### Echo Server
//!
//! The simplest possible WebSocket server:
//!
//! ```
//! use wsforge_core::prelude::*;
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
//!     router.listen("127.0.0.1:8080").await?;
//!     Ok(())
//! }
//! ```
//!
//! ### Chat Server with Broadcasting
//!
//! A real-time chat application:
//!
//! ```
//! use wsforge_core::prelude::*;
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
//!             println!("✅ {} connected (Total: {})", conn_id, manager.count());
//!         })
//!         .on_disconnect(|manager, conn_id| {
//!             println!("❌ {} disconnected", conn_id);
//!         });
//!
//!     router.listen("127.0.0.1:8080").await?;
//!     Ok(())
//! }
//! ```
//!
//! ### Web Application with Static Files
//!
//! Hybrid HTTP/WebSocket server:
//!
//! ```
//! use wsforge_core::prelude::*;
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
//!         .serve_static("public")  // Serve HTML/CSS/JS
//!         .default_handler(handler(ws_handler));
//!
//!     // Handles both:
//!     // http://localhost:8080/        -> public/index.html
//!     // ws://localhost:8080           -> WebSocket handler
//!
//!     router.listen("0.0.0.0:8080").await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Handler Patterns
//!
//! ### Simple Handler
//!
//! ```
//! use wsforge_core::prelude::*;
//!
//! async fn simple_handler() -> Result<String> {
//!     Ok("Hello, WebSocket!".to_string())
//! }
//! ```
//!
//! ### With JSON Extraction
//!
//! ```
//! use wsforge_core::prelude::*;
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! struct Request {
//!     action: String,
//!     data: String,
//! }
//!
//! async fn json_handler(Json(req): Json<Request>) -> Result<String> {
//!     Ok(format!("Action: {}, Data: {}", req.action, req.data))
//! }
//! ```
//!
//! ### With State and Connection
//!
//! ```
//! use wsforge_core::prelude::*;
//! use std::sync::Arc;
//!
//! async fn stateful_handler(
//!     msg: Message,
//!     conn: Connection,
//!     State(manager): State<Arc<ConnectionManager>>,
//! ) -> Result<String> {
//!     Ok(format!(
//!         "Connection {} | {} total connections",
//!         conn.id(),
//!         manager.count()
//!     ))
//! }
//! ```
//!
//! ## Extractors
//!
//! WsForge provides powerful type-safe extractors:
//!
//! | Extractor | Description | Example |
//! |-----------|-------------|---------|
//! | `Message` | Raw message | `msg: Message` |
//! | `Json<T>` | JSON deserialization | `Json(data): Json<MyStruct>` |
//! | `Connection` | Active connection | `conn: Connection` |
//! | `State<T>` | Shared state | `State(db): State<Arc<Database>>` |
//! | `ConnectInfo` | Connection metadata | `ConnectInfo(info)` |
//! | `Data` | Raw bytes | `Data(bytes): Data` |
//!
//! ## Response Types
//!
//! Handlers can return various types:
//!
//! ```
//! use wsforge_core::prelude::*;
//!
//! // No response
//! async fn handler1() -> Result<()> {
//!     Ok(())
//! }
//!
//! // Text response
//! async fn handler2() -> Result<String> {
//!     Ok("response".to_string())
//! }
//!
//! // Raw message
//! async fn handler3() -> Result<Message> {
//!     Ok(Message::text("response"))
//! }
//!
//! // Binary response
//! async fn handler4() -> Result<Vec<u8>> {
//!     Ok(vec!)[1][2][3][4]
//! }
//!
//! // JSON response
//! async fn handler5() -> Result<JsonResponse<serde_json::Value>> {
//!     Ok(JsonResponse(serde_json::json!({"status": "ok"})))
//! }
//! ```
//!
//! ## Broadcasting Patterns
//!
//! ### Broadcast to All
//!
//! ```
//! use wsforge_core::prelude::*;
//! use std::sync::Arc;
//!
//! async fn broadcast_all(
//!     msg: Message,
//!     State(manager): State<Arc<ConnectionManager>>,
//! ) -> Result<()> {
//!     manager.broadcast(msg);
//!     Ok(())
//! }
//! ```
//!
//! ### Broadcast Except Sender
//!
//! ```
//! use wsforge_core::prelude::*;
//! use std::sync::Arc;
//!
//! async fn broadcast_others(
//!     msg: Message,
//!     conn: Connection,
//!     State(manager): State<Arc<ConnectionManager>>,
//! ) -> Result<()> {
//!     manager.broadcast_except(conn.id(), msg);
//!     Ok(())
//! }
//! ```
//!
//! ### Targeted Broadcasting
//!
//! ```
//! use wsforge_core::prelude::*;
//! use std::sync::Arc;
//!
//! async fn broadcast_to_room(
//!     msg: Message,
//!     State(manager): State<Arc<ConnectionManager>>,
//! ) -> Result<()> {
//!     let room_members = vec!["conn_1".to_string(), "conn_2".to_string()];
//!     manager.broadcast_to(&room_members, msg);
//!     Ok(())
//! }
//! ```
//!
//! ## Error Handling
//!
//! WsForge provides comprehensive error handling:
//!
//! ```
//! use wsforge_core::prelude::*;
//!
//! async fn safe_handler(msg: Message) -> Result<String> {
//!     // Parse JSON
//!     let data: serde_json::Value = msg.json()?;
//!
//!     // Validate
//!     if data.is_null() {
//!         return Err(Error::custom("Data cannot be null"));
//!     }
//!
//!     // Process and return
//!     Ok("processed".to_string())
//! }
//! ```
//!
//! ## Performance Characteristics
//!
//! - **Connection Management**: O(1) lock-free operations via DashMap
//! - **Message Routing**: O(1) handler lookup
//! - **Broadcasting**: O(n) where n is the number of connections
//! - **Memory**: Zero-copy message handling where possible
//! - **Concurrency**: Full async/await with tokio
//!
//! ## Testing
//!
//! WsForge handlers are easy to test:
//!
//! ```
//! use wsforge_core::prelude::*;
//!
//! async fn my_handler(msg: Message) -> Result<String> {
//!     Ok(format!("Echo: {}", msg.as_text().unwrap_or("")))
//! }
//!
//! #[tokio::test]
//! async fn test_handler() {
//!     let msg = Message::text("hello");
//!     let result = my_handler(msg).await.unwrap();
//!     assert_eq!(result, "Echo: hello");
//! }
//! ```
//!
//! ## Production Considerations
//!
//! ### Rate Limiting
//!
//! ```
//! use wsforge_core::prelude::*;
//! use std::sync::Arc;
//! use tokio::sync::RwLock;
//! use std::collections::HashMap;
//!
//! struct RateLimiter {
//!     limits: RwLock<HashMap<String, u32>>,
//! }
//!
//! async fn rate_limited_handler(
//!     msg: Message,
//!     conn: Connection,
//!     State(limiter): State<Arc<RateLimiter>>,
//! ) -> Result<String> {
//!     // Check rate limit
//!     // Process if allowed
//!     Ok("processed".to_string())
//! }
//! ```
//!
//! ### Graceful Shutdown
//!
//! ```
//! use wsforge_core::prelude::*;
//! use tokio::signal;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let router = Router::new();
//!
//!     tokio::select! {
//!         _ = router.listen("127.0.0.1:8080") => {},
//!         _ = signal::ctrl_c() => {
//!             println!("Shutting down gracefully...");
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Further Reading
//!
//! - [Connection Management](connection/index.html)
//! - [Message Handling](message/index.html)
//! - [Handler Guide](handler/index.html)
//! - [Extractor Reference](extractor/index.html)
//! - [Router Configuration](router/index.html)
//! - [State Management](state/index.html)

// Enable documentation features for docs.rs
#![cfg_attr(docsrs, feature(doc_cfg))]
// Deny missing docs to ensure comprehensive documentation
#![warn(missing_docs)]
// Enable additional documentation lint rules
#![warn(rustdoc::missing_crate_level_docs)]

pub mod connection;
pub mod error;
pub mod extractor;
pub mod handler;
pub mod message;
pub mod router;
pub mod state;
pub mod static_files;

pub use connection::{Connection, ConnectionId};
pub use error::{Error, Result};
pub use extractor::{ConnectInfo, Data, Extension, Extensions, Json, Path, Query, State};
pub use handler::{Handler, HandlerService, IntoResponse, JsonResponse, handler};
pub use message::{Message, MessageType};
pub use router::{Route, Router};
pub use state::AppState;
pub use static_files::StaticFileHandler;

/// Commonly used types and traits for WsForge applications.
///
/// This prelude module re-exports the most frequently used types, making it easier
/// to get started with WsForge. Import this module to bring all essential types
/// into scope with a single use statement.
///
/// # Examples
///
/// ```
/// use wsforge_core::prelude::*;
///
/// // Now you have access to:
/// // - Router, Message, Connection, ConnectionManager
/// // - handler(), Error, Result
/// // - Json, State, ConnectInfo
/// // - And more!
///
/// async fn my_handler(msg: Message) -> Result<String> {
///     Ok("Hello!".to_string())
/// }
///
/// # fn example() {
/// let router = Router::new()
///     .default_handler(handler(my_handler));
/// # }
/// ```
///
/// # Included Types
///
/// ## Core Types
/// - [`Router`]: Server router and configuration
/// - [`Message`]: WebSocket message type
/// - [`Connection`]: Active connection handle
/// - [`ConnectionManager`]: Manages all connections
/// - [`Error`], [`Result`]: Error handling
///
/// ## Extractors
/// - [`Json<T>`]: JSON deserialization
/// - [`State<T>`]: Shared state extraction
/// - [`ConnectInfo`]: Connection metadata
/// - [`Data`]: Raw byte extraction
/// - [`Extension<T>`]: Custom extensions
///
/// ## Handlers
/// - [`handler()`]: Convert functions to handlers
/// - [`JsonResponse<T>`]: JSON response type
/// - [`IntoResponse`]: Response conversion trait
///
/// ## State
/// - [`AppState`]: Application state container
/// - [`Extensions`]: Request-scoped data
///
/// ## Utilities
/// - [`MessageType`]: Message type enum
/// - [`StaticFileHandler`]: Static file serving
pub mod prelude {
    pub use crate::connection::{Connection, ConnectionId, ConnectionManager};
    pub use crate::error::{Error, Result};
    pub use crate::extractor::{
        ConnectInfo, Data, Extension, Extensions, Json, Path, Query, State,
    };
    pub use crate::handler::{Handler, HandlerService, IntoResponse, JsonResponse, handler};
    pub use crate::message::{Message, MessageType};
    pub use crate::router::{Route, Router};
    pub use crate::state::AppState;
    pub use crate::static_files::StaticFileHandler;
}
