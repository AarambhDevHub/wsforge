//! Routing and server management for WebSocket connections.
//!
//! This module provides the core routing infrastructure for WsForge, allowing you to
//! define handlers for different message patterns, manage application state, and configure
//! server behavior. The router handles both WebSocket connections and static file serving
//! on the same port.
//!
//! # Overview
//!
//! The [`Router`] is the main entry point for building a WebSocket server. It provides
//! a builder-style API for:
//! - Registering message handlers for specific routes
//! - Managing shared application state
//! - Serving static files (HTML, CSS, JS)
//! - Configuring connection lifecycle callbacks
//! - Managing WebSocket connections
//!
//! # Architecture
//!
//! ```
//! ┌─────────────────┐
//! │  TCP Listener   │
//! └────────┬────────┘
//!          │
//!          ├──→ HTTP Request → Static File Handler
//!          │
//!          └──→ WebSocket Upgrade → Connection Manager
//!                                    │
//!                                    ├──→ on_connect callback
//!                                    │
//!                                    ├──→ Message Router → Handler
//!                                    │
//!                                    └──→ on_disconnect callback
//! ```
//!
//! # Examples
//!
//! ## Simple Echo Server
//!
//! ```
//! use wsforge::prelude::*;
//!
//! async fn echo(msg: Message) -> Result<Message> {
//!     Ok(msg)
//! }
//!
//! # async fn example() -> Result<()> {
//! let router = Router::new()
//!     .default_handler(handler(echo));
//!
//! router.listen("127.0.0.1:8080").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Multiple Routes
//!
//! ```
//! use wsforge::prelude::*;
//!
//! async fn echo(msg: Message) -> Result<Message> {
//!     Ok(msg)
//! }
//!
//! async fn stats(State(manager): State<Arc<ConnectionManager>>) -> Result<String> {
//!     Ok(format!("Active connections: {}", manager.count()))
//! }
//!
//! # async fn example() -> Result<()> {
//! # use std::sync::Arc;
//! let router = Router::new()
//!     .route("/echo", handler(echo))
//!     .route("/stats", handler(stats))
//!     .default_handler(handler(echo));
//!
//! router.listen("127.0.0.1:8080").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## With State and Callbacks
//!
//! ```
//! use wsforge::prelude::*;
//! use std::sync::Arc;
//!
//! async fn broadcast(msg: Message, State(manager): State<Arc<ConnectionManager>>) -> Result<()> {
//!     manager.broadcast(msg);
//!     Ok(())
//! }
//!
//! # async fn example() -> Result<()> {
//! let router = Router::new()
//!     .default_handler(handler(broadcast))
//!     .on_connect(|manager, conn_id| {
//!         println!("✅ User {} connected (Total: {})", conn_id, manager.count());
//!     })
//!     .on_disconnect(|manager, conn_id| {
//!         println!("❌ User {} disconnected", conn_id);
//!     });
//!
//! router.listen("127.0.0.1:8080").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Hybrid HTTP/WebSocket Server
//!
//! ```
//! use wsforge::prelude::*;
//!
//! async fn ws_handler(msg: Message) -> Result<Message> {
//!     Ok(msg)
//! }
//!
//! # async fn example() -> Result<()> {
//! let router = Router::new()
//!     .serve_static("public")  // Serve HTML/CSS/JS from 'public' folder
//!     .default_handler(handler(ws_handler));
//!
//! // Handles both HTTP (for files) and WebSocket on same port
//! router.listen("127.0.0.1:8080").await?;
//! # Ok(())
//! # }
//! ```

use crate::connection::{ConnectionId, ConnectionManager, handle_websocket};
use crate::error::{Error, Result};
use crate::extractor::Extensions;
use crate::handler::Handler;
use crate::message::Message;
use crate::state::AppState;
use dashmap::DashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use tracing::{error, info};

/// Represents a single route with its path and handler.
///
/// Routes map message patterns (paths) to handler functions.
/// This is typically used internally by the [`Router`].
///
/// # Examples
///
/// ```
/// use wsforge::prelude::*;
///
/// async fn my_handler() -> Result<String> {
///     Ok("response".to_string())
/// }
///
/// # fn example() {
/// let route = Route {
///     path: "/api/message".to_string(),
///     handler: handler(my_handler),
/// };
/// # }
/// ```
pub struct Route {
    /// The route path (e.g., "/chat", "/api/users")
    pub path: String,
    /// The handler for this route
    pub handler: Arc<dyn Handler>,
}

/// The main router for WebSocket servers.
///
/// `Router` is the central component that manages routing, state, connections,
/// and server lifecycle. It uses a builder pattern for configuration and
/// supports both WebSocket and HTTP static file serving on the same port.
///
/// # Thread Safety
///
/// Router is thread-safe and can be cloned cheaply (uses `Arc` internally).
/// All connections share the same router instance.
///
/// # Lifecycle
///
/// 1. Create router with `Router::new()`
/// 2. Configure routes, state, handlers, callbacks
/// 3. Call `listen()` to start the server
/// 4. Router handles incoming connections automatically
///
/// # Examples
///
/// ## Basic Setup
///
/// ```
/// use wsforge::prelude::*;
///
/// async fn handler(msg: Message) -> Result<String> {
///     Ok("received".to_string())
/// }
///
/// # async fn example() -> Result<()> {
/// let router = Router::new()
///     .default_handler(handler(handler));
///
/// router.listen("0.0.0.0:8080").await?;
/// # Ok(())
/// # }
/// ```
///
/// ## With Shared State
///
/// ```
/// use wsforge::prelude::*;
/// use std::sync::Arc;
///
/// struct AppConfig {
///     max_connections: usize,
/// }
///
/// async fn handler(State(config): State<Arc<AppConfig>>) -> Result<String> {
///     Ok(format!("Max connections: {}", config.max_connections))
/// }
///
/// # async fn example() -> Result<()> {
/// let config = Arc::new(AppConfig { max_connections: 100 });
///
/// let router = Router::new()
///     .with_state(config)
///     .default_handler(handler(handler));
///
/// router.listen("127.0.0.1:8080").await?;
/// # Ok(())
/// # }
/// ```
pub struct Router {
    routes: Arc<DashMap<String, Arc<dyn Handler>>>,
    state: AppState,
    connection_manager: Arc<ConnectionManager>,
    on_connect: Option<Arc<dyn Fn(&Arc<ConnectionManager>, ConnectionId) + Send + Sync>>,
    on_disconnect: Option<Arc<dyn Fn(&Arc<ConnectionManager>, ConnectionId) + Send + Sync>>,
    default_handler: Option<Arc<dyn Handler>>,
    static_handler: Option<crate::static_files::StaticFileHandler>,
}

impl Router {
    /// Creates a new empty router.
    ///
    /// The router starts with no routes, no state, and no handlers.
    /// Use the builder methods to configure it.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// let router = Router::new();
    /// ```
    pub fn new() -> Self {
        Self {
            routes: Arc::new(DashMap::new()),
            state: AppState::new(),
            connection_manager: Arc::new(ConnectionManager::new()),
            on_connect: None,
            on_disconnect: None,
            default_handler: None,
            static_handler: None,
        }
    }

    /// Registers a handler for a specific route.
    ///
    /// Routes are matched against the beginning of incoming messages.
    /// For example, a message like `/chat hello` would match route `/chat`.
    ///
    /// # Arguments
    ///
    /// * `path` - The route path (e.g., "/chat", "/api/users")
    /// * `handler` - The handler function wrapped with `handler()`
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// async fn chat_handler(msg: Message) -> Result<String> {
    ///     Ok("chat response".to_string())
    /// }
    ///
    /// async fn api_handler(msg: Message) -> Result<String> {
    ///     Ok("api response".to_string())
    /// }
    ///
    /// # fn example() {
    /// let router = Router::new()
    ///     .route("/chat", handler(chat_handler))
    ///     .route("/api", handler(api_handler));
    /// # }
    /// ```
    pub fn route(self, path: impl Into<String>, handler: Arc<dyn Handler>) -> Self {
        self.routes.insert(path.into(), handler);
        self
    }

    /// Adds shared state to the router.
    ///
    /// State is shared across all connections and can be extracted in handlers
    /// using the [`State`] extractor. Any type that is `Send + Sync + 'static`
    /// can be used as state.
    ///
    /// # Type Safety
    ///
    /// Multiple different types can be added as state. Each type is stored
    /// separately and retrieved by type.
    ///
    /// # Arguments
    ///
    /// * `data` - The state data to share
    ///
    /// # Examples
    ///
    /// ## Single State
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use std::sync::Arc;
    ///
    /// struct Database {
    ///     // database fields
    /// }
    ///
    /// async fn handler(State(db): State<Arc<Database>>) -> Result<String> {
    ///     Ok("query result".to_string())
    /// }
    ///
    /// # fn example() {
    /// let db = Arc::new(Database {});
    ///
    /// let router = Router::new()
    ///     .with_state(db)
    ///     .default_handler(handler(handler));
    /// # }
    /// ```
    ///
    /// ## Multiple States
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use std::sync::Arc;
    ///
    /// struct Config {
    ///     port: u16,
    /// }
    ///
    /// struct Database {
    ///     // database fields
    /// }
    ///
    /// # fn example() {
    /// let router = Router::new()
    ///     .with_state(Arc::new(Config { port: 8080 }))
    ///     .with_state(Arc::new(Database {}));
    /// # }
    /// ```
    pub fn with_state<T: Send + Sync + 'static>(self, data: Arc<T>) -> Self {
        self.state.insert(data);
        self
    }

    /// Sets a callback to be called when a new connection is established.
    ///
    /// The callback receives a reference to the connection manager and the
    /// connection ID. This is useful for logging, sending welcome messages,
    /// or updating user lists.
    ///
    /// # Arguments
    ///
    /// * `f` - Callback function with signature `Fn(&Arc<ConnectionManager>, ConnectionId)`
    ///
    /// # Examples
    ///
    /// ## Simple Logging
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example() {
    /// let router = Router::new()
    ///     .on_connect(|manager, conn_id| {
    ///         println!("New connection: {} (Total: {})", conn_id, manager.count());
    ///     });
    /// # }
    /// ```
    ///
    /// ## Send Welcome Message
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example() {
    /// let router = Router::new()
    ///     .on_connect(|manager, conn_id| {
    ///         if let Some(conn) = manager.get(&conn_id) {
    ///             let _ = conn.send_text("Welcome to the server!");
    ///         }
    ///     });
    /// # }
    /// ```
    ///
    /// ## Broadcast Join Notification
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example() {
    /// let router = Router::new()
    ///     .on_connect(|manager, conn_id| {
    ///         let msg = format!("User {} joined", conn_id);
    ///         manager.broadcast(Message::text(msg));
    ///     });
    /// # }
    /// ```
    pub fn on_connect<F>(mut self, f: F) -> Self
    where
        F: Fn(&Arc<ConnectionManager>, ConnectionId) + Send + Sync + 'static,
    {
        self.on_connect = Some(Arc::new(f));
        self
    }

    /// Sets a callback to be called when a connection is closed.
    ///
    /// The callback receives a reference to the connection manager and the
    /// connection ID. Note that the connection is already removed from the
    /// manager when this is called.
    ///
    /// # Arguments
    ///
    /// * `f` - Callback function with signature `Fn(&Arc<ConnectionManager>, ConnectionId)`
    ///
    /// # Examples
    ///
    /// ## Logging Disconnections
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example() {
    /// let router = Router::new()
    ///     .on_disconnect(|manager, conn_id| {
    ///         println!("Connection {} closed (Remaining: {})", conn_id, manager.count());
    ///     });
    /// # }
    /// ```
    ///
    /// ## Broadcast Leave Notification
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example() {
    /// let router = Router::new()
    ///     .on_disconnect(|manager, conn_id| {
    ///         let msg = format!("User {} left", conn_id);
    ///         manager.broadcast(Message::text(msg));
    ///     });
    /// # }
    /// ```
    pub fn on_disconnect<F>(mut self, f: F) -> Self
    where
        F: Fn(&Arc<ConnectionManager>, ConnectionId) + Send + Sync + 'static,
    {
        self.on_disconnect = Some(Arc::new(f));
        self
    }

    /// Sets the default handler for messages that don't match any route.
    ///
    /// This handler is called when no route matches the incoming message.
    /// Use this for catch-all behavior or when you don't need routing.
    ///
    /// # Arguments
    ///
    /// * `handler` - The default handler wrapped with `handler()`
    ///
    /// # Examples
    ///
    /// ## Echo Server
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// async fn echo(msg: Message) -> Result<Message> {
    ///     Ok(msg)
    /// }
    ///
    /// # fn example() {
    /// let router = Router::new()
    ///     .default_handler(handler(echo));
    /// # }
    /// ```
    ///
    /// ## Error Handler
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// async fn not_found() -> Result<String> {
    ///     Ok("Unknown command".to_string())
    /// }
    ///
    /// # fn example() {
    /// let router = Router::new()
    ///     .route("/known", handler(not_found))
    ///     .default_handler(handler(not_found));
    /// # }
    /// ```
    pub fn default_handler(mut self, handler: Arc<dyn Handler>) -> Self {
        self.default_handler = Some(handler);
        self
    }

    /// Enables static file serving from a directory.
    ///
    /// When enabled, the router will serve static files (HTML, CSS, JavaScript, images)
    /// from the specified directory for HTTP requests, while still handling WebSocket
    /// connections on the same port.
    ///
    /// # Path Resolution
    ///
    /// - Requests to `/` serve `index.html` from the directory
    /// - Other requests map directly to files (e.g., `/style.css` → `directory/style.css`)
    /// - MIME types are automatically detected
    ///
    /// # Security
    ///
    /// Path traversal attempts (e.g., `../../etc/passwd`) are automatically blocked.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the directory containing static files
    ///
    /// # Examples
    ///
    /// ## Serve Static Files
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// async fn ws_handler(msg: Message) -> Result<Message> {
    ///     Ok(msg)
    /// }
    ///
    /// # fn example() {
    /// let router = Router::new()
    ///     .serve_static("public")  // Serve files from ./public
    ///     .default_handler(handler(ws_handler));
    ///
    /// // Now you can access:
    /// // http://localhost:8080/          -> public/index.html
    /// // http://localhost:8080/app.js    -> public/app.js
    /// // ws://localhost:8080             -> WebSocket handler
    /// # }
    /// ```
    ///
    /// ## Web Chat Application
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// async fn chat_handler(msg: Message, State(manager): State<Arc<ConnectionManager>>) -> Result<()> {
    ///     manager.broadcast(msg);
    ///     Ok(())
    /// }
    ///
    /// # fn example() {
    /// # use std::sync::Arc;
    /// let router = Router::new()
    ///     .serve_static("chat-ui")  // HTML/CSS/JS for chat interface
    ///     .default_handler(handler(chat_handler));
    /// # }
    /// ```
    pub fn serve_static(mut self, path: impl Into<PathBuf>) -> Self {
        self.static_handler = Some(crate::static_files::StaticFileHandler::new(path.into()));
        self
    }

    /// Returns a reference to the connection manager.
    ///
    /// The connection manager is automatically created with the router.
    /// Use this to get access to it for storing in state or elsewhere.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example() {
    /// let router = Router::new();
    /// let manager = router.connection_manager();
    ///
    /// // Now you can use the manager
    /// println!("Active connections: {}", manager.count());
    /// # }
    /// ```
    pub fn connection_manager(&self) -> Arc<ConnectionManager> {
        self.connection_manager.clone()
    }

    /// Starts the WebSocket server and listens for connections.
    ///
    /// This method consumes the router and starts the server loop. It will
    /// run indefinitely until the process is terminated or an error occurs.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to bind to (e.g., "127.0.0.1:8080", "0.0.0.0:3000")
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The address format is invalid
    /// - The port is already in use
    /// - Permission is denied (e.g., ports < 1024 on Unix)
    ///
    /// # Examples
    ///
    /// ## Basic Usage
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # async fn example() -> Result<()> {
    /// let router = Router::new();
    /// router.listen("127.0.0.1:8080").await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## All Interfaces
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # async fn example() -> Result<()> {
    /// let router = Router::new();
    /// router.listen("0.0.0.0:8080").await?;  // Accept connections from anywhere
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## With Error Handling
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # async fn example() {
    /// let router = Router::new();
    ///
    /// match router.listen("127.0.0.1:8080").await {
    ///     Ok(_) => println!("Server stopped"),
    ///     Err(e) => eprintln!("Server error: {}", e),
    /// }
    /// # }
    /// ```
    pub async fn listen(self, addr: impl AsRef<str>) -> Result<()> {
        let addr: SocketAddr = addr
            .as_ref()
            .parse()
            .map_err(|e| Error::custom(format!("Invalid address: {}", e)))?;

        // Insert connection manager into state BEFORE wrapping in Arc
        self.state.insert(self.connection_manager.clone());

        let listener = TcpListener::bind(addr).await?;
        info!("WebSocket server listening on {}", addr);

        let router = Arc::new(self);

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            let router = router.clone();

            tokio::spawn(async move {
                if let Err(e) = router.handle_connection(stream, peer_addr).await {
                    error!("Connection error: {}", e);
                }
            });
        }
    }

    async fn handle_connection(&self, stream: TcpStream, peer_addr: SocketAddr) -> Result<()> {
        let mut buffer = [0u8; 1024];

        let n = tokio::time::timeout(std::time::Duration::from_secs(5), stream.peek(&mut buffer))
            .await
            .map_err(|_| Error::custom("Connection timeout"))?
            .map_err(|e| Error::custom(format!("Failed to read: {}", e)))?;

        let header = String::from_utf8_lossy(&buffer[..n]);

        if header.contains("Upgrade: websocket") || header.contains("upgrade: websocket") {
            self.handle_websocket_connection(stream, peer_addr).await
        } else if let Some(ref static_handler) = self.static_handler {
            self.handle_http_request(stream, static_handler, &header)
                .await
        } else {
            Err(Error::custom("No handler for HTTP requests"))
        }
    }

    async fn handle_http_request(
        &self,
        mut stream: TcpStream,
        static_handler: &crate::static_files::StaticFileHandler,
        header: &str,
    ) -> Result<()> {
        use crate::static_files::http_response;
        use tokio::io::AsyncWriteExt;

        let path = header
            .lines()
            .next()
            .and_then(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && (parts[0] == "GET" || parts[0] == "HEAD") {
                    Some(parts[1])
                } else {
                    None
                }
            })
            .unwrap_or("/");

        let response = match static_handler.serve(path).await {
            Ok((content, mime_type)) => {
                info!("Served: {} ({} bytes)", path, content.len());
                http_response(200, &mime_type, content)
            }
            Err(e) => {
                tracing::warn!("File not found: {} - {}", path, e);
                let html = b"<html><body><h1>404 Not Found</h1></body></html>".to_vec();
                http_response(404, "text/html", html)
            }
        };

        stream.write_all(&response).await?;
        stream.flush().await?;
        Ok(())
    }

    async fn handle_websocket_connection(
        &self,
        stream: TcpStream,
        peer_addr: SocketAddr,
    ) -> Result<()> {
        let ws_stream = accept_async(stream).await?;
        let conn_id = Self::generate_connection_id();

        let router = self.clone();
        let manager = self.connection_manager.clone();

        let on_message = Arc::new(move |conn_id: ConnectionId, message: Message| {
            let router = router.clone();
            tokio::spawn(async move {
                if let Err(e) = router.handle_message(conn_id, message).await {
                    error!("Message handling error: {}", e);
                }
            });
        });

        let manager_ref = manager.clone();
        let on_connect = self
            .on_connect
            .clone()
            .map(move |cb| {
                let manager = manager_ref.clone();
                Arc::new(move |conn_id: ConnectionId| {
                    cb(&manager, conn_id);
                }) as Arc<dyn Fn(ConnectionId) + Send + Sync>
            })
            .unwrap_or_else(|| {
                Arc::new(|conn_id: ConnectionId| {
                    info!("Client connected: {}", conn_id);
                })
            });

        let manager_ref = manager.clone();
        let on_disconnect = self
            .on_disconnect
            .clone()
            .map(move |cb| {
                let manager = manager_ref.clone();
                Arc::new(move |conn_id: ConnectionId| {
                    cb(&manager, conn_id);
                }) as Arc<dyn Fn(ConnectionId) + Send + Sync>
            })
            .unwrap_or_else(|| {
                Arc::new(|conn_id: ConnectionId| {
                    info!("Client disconnected: {}", conn_id);
                })
            });

        handle_websocket(
            ws_stream,
            conn_id,
            peer_addr,
            manager,
            on_message,
            on_connect,
            on_disconnect,
        )
        .await;

        Ok(())
    }

    async fn handle_message(&self, conn_id: ConnectionId, message: Message) -> Result<()> {
        let conn = self
            .connection_manager
            .get(&conn_id)
            .ok_or_else(|| Error::ConnectionNotFound(conn_id.clone()))?;

        let extensions = Extensions::new();

        let handler = if let Some(text) = message.as_text() {
            if text.starts_with('/') {
                if let Some((route, _)) = text.split_once(' ') {
                    self.routes.get(route).map(|h| h.value().clone())
                } else {
                    self.routes.get(text).map(|h| h.value().clone())
                }
            } else {
                None
            }
        } else {
            None
        };

        let handler = handler.or_else(|| self.default_handler.clone());

        if let Some(handler) = handler {
            match handler
                .call(message, conn.clone(), self.state.clone(), extensions)
                .await
            {
                Ok(Some(response)) => {
                    if let Err(e) = conn.send(response) {
                        error!("Failed to send response to {}: {}", conn_id, e);
                    }
                }
                Ok(None) => {
                    tracing::debug!("Handler processed message without response");
                }
                Err(e) => {
                    error!("Handler error for {}: {}", conn_id, e);
                }
            }
        } else {
            tracing::warn!("No handler found for message from {}", conn_id);
        }

        Ok(())
    }

    fn generate_connection_id() -> ConnectionId {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        format!("conn_{}", COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

impl Clone for Router {
    fn clone(&self) -> Self {
        Self {
            routes: self.routes.clone(),
            state: self.state.clone(),
            connection_manager: self.connection_manager.clone(),
            on_connect: self.on_connect.clone(),
            on_disconnect: self.on_disconnect.clone(),
            default_handler: self.default_handler.clone(),
            static_handler: self.static_handler.clone(),
        }
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}
