//! WebSocket connection management and message handling.
//!
//! This module provides the core functionality for managing WebSocket connections,
//! including connection lifecycle, message routing, and broadcasting capabilities.
//!
//! # Overview
//!
//! The connection module consists of three main components:
//!
//! - [`Connection`]: Represents an individual WebSocket connection
//! - [`ConnectionManager`]: Manages multiple connections with thread-safe operations
//! - [`handle_websocket`]: Async function that handles the WebSocket lifecycle
//!
//! # Architecture
//!
//! Each WebSocket connection runs two concurrent tasks:
//! - **Read task**: Receives messages from the client
//! - **Write task**: Sends messages to the client via an unbounded channel
//!
//! This architecture ensures that slow clients don't block message processing.
//!
//! # Examples
//!
//! ## Creating and Using a ConnectionManager
//!
//! ```
//! use wsforge::prelude::*;
//! use std::sync::Arc;
//!
//! let manager = Arc::new(ConnectionManager::new());
//!
//! // Check connection count
//! println!("Active connections: {}", manager.count());
//!
//! // Broadcast a message to all connections
//! manager.broadcast(Message::text("Hello everyone!"));
//! ```
//!
//! ## Broadcasting Messages
//!
//! ```
//! use wsforge::prelude::*;
//! use std::sync::Arc;
//!
//! # let manager = Arc::new(ConnectionManager::new());
//! # let conn_id = "conn_0".to_string();
//! // Broadcast to all connections
//! manager.broadcast(Message::text("System announcement"));
//!
//! // Broadcast to all except one
//! manager.broadcast_except(&conn_id, Message::text("User joined"));
//!
//! // Broadcast to specific connections
//! let target_ids = vec!["conn_1".to_string(), "conn_2".to_string()];
//! manager.broadcast_to(&target_ids, Message::text("Private message"));
//! ```

use crate::error::{Error, Result};
use crate::message::Message;
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::WebSocketStream;
use tracing::{debug, error, info, warn};

/// A unique identifier for a WebSocket connection.
///
/// Connection IDs are automatically generated and guaranteed to be unique
/// within the lifetime of the application.
pub type ConnectionId = String;

/// Metadata about a WebSocket connection.
///
/// Contains information about when the connection was established,
/// the client's address, and optional protocol information.
///
/// # Examples
///
/// ```
/// use wsforge::connection::ConnectionInfo;
/// use std::net::SocketAddr;
///
/// let info = ConnectionInfo {
///     id: "conn_0".to_string(),
///     addr: "127.0.0.1:8080".parse().unwrap(),
///     connected_at: 1634567890,
///     protocol: Some("websocket".to_string()),
/// };
///
/// println!("Connection {} from {}", info.id, info.addr);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    /// Unique identifier for this connection
    pub id: ConnectionId,
    /// Socket address of the connected client
    pub addr: SocketAddr,
    /// Unix timestamp when the connection was established
    pub connected_at: u64,
    /// Optional protocol information (e.g., "websocket", "wss")
    pub protocol: Option<String>,
}

/// Represents an active WebSocket connection.
///
/// A `Connection` provides methods to send messages to the connected client.
/// Messages are sent asynchronously through an unbounded channel, ensuring
/// that slow clients don't block the server.
///
/// # Thread Safety
///
/// `Connection` is cheaply cloneable (uses `Arc` internally) and can be
/// safely shared across threads.
///
/// # Examples
///
/// ## Sending Text Messages
///
/// ```
/// use wsforge::prelude::*;
///
/// # async fn example(conn: Connection) -> Result<()> {
/// // Send a text message
/// conn.send_text("Hello, client!")?;
///
/// // Send JSON data
/// #[derive(serde::Serialize)]
/// struct Response {
///     status: String,
///     data: i32,
/// }
///
/// conn.send_json(&Response {
///     status: "ok".to_string(),
///     data: 42,
/// })?;
/// # Ok(())
/// # }
/// ```
///
/// ## Sending Binary Data
///
/// ```
/// use wsforge::prelude::*;
///
/// # async fn example(conn: Connection) -> Result<()> {
/// let data = vec![0x01, 0x02, 0x03, 0x04];
/// conn.send_binary(data)?;
/// # Ok(())
/// # }
/// ```
pub struct Connection {
    /// Unique identifier for this connection
    pub id: ConnectionId,
    /// Connection metadata
    pub info: ConnectionInfo,
    /// Channel sender for outgoing messages
    sender: mpsc::UnboundedSender<Message>,
}

impl Connection {
    /// Creates a new `Connection` instance.
    ///
    /// This is typically called internally by the framework when a new
    /// WebSocket connection is established.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the connection
    /// * `addr` - Socket address of the client
    /// * `sender` - Channel sender for outgoing messages
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::connection::Connection;
    /// use tokio::sync::mpsc;
    /// use std::net::SocketAddr;
    ///
    /// let (tx, rx) = mpsc::unbounded_channel();
    /// let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    /// let conn = Connection::new("conn_0".to_string(), addr, tx);
    ///
    /// assert_eq!(conn.id(), "conn_0");
    /// ```
    pub fn new(id: ConnectionId, addr: SocketAddr, sender: mpsc::UnboundedSender<Message>) -> Self {
        let info = ConnectionInfo {
            id: id.clone(),
            addr,
            connected_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            protocol: None,
        };

        Self { id, info, sender }
    }

    /// Sends a message to the connected client.
    ///
    /// Messages are queued in an unbounded channel and sent asynchronously.
    /// This method returns immediately without waiting for the message to be sent.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection has been closed and the channel
    /// receiver has been dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # async fn example(conn: Connection) -> Result<()> {
    /// let msg = Message::text("Hello!");
    /// conn.send(msg)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn send(&self, message: Message) -> Result<()> {
        self.sender
            .send(message)
            .map_err(|e| Error::custom(format!("Failed to send message: {}", e)))
    }

    /// Sends a text message to the connected client.
    ///
    /// This is a convenience method that creates a text [`Message`] and sends it.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection has been closed.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # async fn example(conn: Connection) -> Result<()> {
    /// conn.send_text("Welcome to the chat!")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn send_text(&self, text: impl Into<String>) -> Result<()> {
        self.send(Message::text(text.into()))
    }

    /// Sends binary data to the connected client.
    ///
    /// This is a convenience method that creates a binary [`Message`] and sends it.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection has been closed.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # async fn example(conn: Connection) -> Result<()> {
    /// let data = vec![0xFF, 0xFE, 0xFD];
    /// conn.send_binary(data)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn send_binary(&self, data: Vec<u8>) -> Result<()> {
        self.send(Message::binary(data))
    }

    /// Serializes data to JSON and sends it as a text message.
    ///
    /// This is a convenience method for sending structured data. The data
    /// is serialized using `serde_json` and sent as a text message.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Serialization fails
    /// - The connection has been closed
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct GameState {
    ///     score: u32,
    ///     level: u8,
    /// }
    ///
    /// # async fn example(conn: Connection) -> Result<()> {
    /// let state = GameState { score: 1000, level: 5 };
    /// conn.send_json(&state)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn send_json<T: Serialize>(&self, data: &T) -> Result<()> {
        let json = serde_json::to_string(data)?;
        self.send_text(json)
    }

    /// Returns the unique identifier for this connection.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example(conn: Connection) {
    /// println!("Connection ID: {}", conn.id());
    /// # }
    /// ```
    pub fn id(&self) -> &ConnectionId {
        &self.id
    }

    /// Returns the connection metadata.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example(conn: Connection) {
    /// let info = conn.info();
    /// println!("Client address: {}", info.addr);
    /// println!("Connected at: {}", info.connected_at);
    /// # }
    /// ```
    pub fn info(&self) -> &ConnectionInfo {
        &self.info
    }
}

/// Manages a collection of active WebSocket connections.
///
/// `ConnectionManager` provides thread-safe operations for managing connections,
/// including adding, removing, and broadcasting messages. It uses [`DashMap`]
/// internally for lock-free concurrent access.
///
/// # Thread Safety
///
/// All operations are thread-safe and can be called from multiple threads
/// concurrently without additional synchronization.
///
/// # Examples
///
/// ## Basic Usage
///
/// ```
/// use wsforge::prelude::*;
/// use std::sync::Arc;
///
/// let manager = Arc::new(ConnectionManager::new());
///
/// // Get connection count
/// let count = manager.count();
/// println!("Active connections: {}", count);
///
/// // Get all connection IDs
/// let ids = manager.all_ids();
/// for id in ids {
///     println!("Connection: {}", id);
/// }
/// ```
///
/// ## Broadcasting
///
/// ```
/// use wsforge::prelude::*;
/// use std::sync::Arc;
///
/// # let manager = Arc::new(ConnectionManager::new());
/// // Broadcast system announcement
/// manager.broadcast(Message::text("Server maintenance in 5 minutes"));
///
/// // Notify everyone except the sender
/// let sender_id = "conn_42";
/// manager.broadcast_except(&sender_id.to_string(),
///     Message::text("New user joined the chat"));
/// ```
pub struct ConnectionManager {
    /// Thread-safe map of active connections
    connections: Arc<DashMap<ConnectionId, Connection>>,
}

impl ConnectionManager {
    /// Creates a new empty `ConnectionManager`.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use std::sync::Arc;
    ///
    /// let manager = Arc::new(ConnectionManager::new());
    /// assert_eq!(manager.count(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
        }
    }

    /// Adds a connection to the manager.
    ///
    /// Returns the total number of connections after adding.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use tokio::sync::mpsc;
    /// use std::net::SocketAddr;
    ///
    /// # fn example() {
    /// let manager = ConnectionManager::new();
    /// let (tx, rx) = mpsc::unbounded_channel();
    /// let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    /// let conn = Connection::new("conn_0".to_string(), addr, tx);
    ///
    /// let count = manager.add(conn);
    /// assert_eq!(count, 1);
    /// # }
    /// ```
    pub fn add(&self, conn: Connection) -> usize {
        let id = conn.id.clone();
        self.connections.insert(id.clone(), conn);
        let count = self.connections.len();
        info!("Added connection: {} (Total: {})", id, count);
        count
    }

    /// Removes a connection from the manager.
    ///
    /// Returns the removed connection if it existed, or `None` if not found.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example(manager: &ConnectionManager) {
    /// let conn_id = "conn_42".to_string();
    /// if let Some(conn) = manager.remove(&conn_id) {
    ///     println!("Removed connection: {}", conn.id());
    /// }
    /// # }
    /// ```
    pub fn remove(&self, id: &ConnectionId) -> Option<Connection> {
        let result = self.connections.remove(id).map(|(_, conn)| conn);
        let count = self.connections.len();
        info!("Removed connection: {} (Total: {})", id, count);
        result
    }

    /// Retrieves a connection by its ID.
    ///
    /// Returns a clone of the connection if found, or `None` if not found.
    ///
    /// # Performance
    ///
    /// This operation is O(1) and does not block other concurrent operations.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # async fn example(manager: &ConnectionManager) -> Result<()> {
    /// let conn_id = "conn_0".to_string();
    /// if let Some(conn) = manager.get(&conn_id) {
    ///     conn.send_text("Hello!")?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get(&self, id: &ConnectionId) -> Option<Connection> {
        self.connections.get(id).map(|entry| entry.value().clone())
    }

    /// Broadcasts a message to all active connections.
    ///
    /// This method iterates through all connections and sends the message
    /// to each one. Failed sends are logged but do not stop the broadcast.
    ///
    /// # Performance
    ///
    /// Broadcasts are performed synchronously but send operations are async,
    /// so messages are queued immediately and sent in the background.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example(manager: &ConnectionManager) {
    /// manager.broadcast(Message::text("Server announcement!"));
    /// # }
    /// ```
    pub fn broadcast(&self, message: Message) {
        let count = self.connections.len();
        debug!("Broadcasting message to {} connections", count);

        let mut success = 0;
        let mut failed = 0;

        for entry in self.connections.iter() {
            match entry.value().send(message.clone()) {
                Ok(_) => {
                    success += 1;
                    debug!("✅ Broadcast sent to {}", entry.key());
                }
                Err(e) => {
                    failed += 1;
                    error!("❌ Failed to broadcast to {}: {}", entry.key(), e);
                }
            }
        }

        info!(
            "Broadcast complete: {} success, {} failed out of {} total",
            success, failed, count
        );
    }

    /// Broadcasts a message to all connections except one.
    ///
    /// This is useful for notifying all users about an action taken by one user,
    /// without sending the notification back to the actor.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example(manager: &ConnectionManager) {
    /// let sender_id = "conn_42".to_string();
    /// manager.broadcast_except(&sender_id,
    ///     Message::text("User 42 sent a message"));
    /// # }
    /// ```
    pub fn broadcast_except(&self, except_id: &ConnectionId, message: Message) {
        debug!(
            "Broadcasting message to {} connections (except {})",
            self.connections.len() - 1,
            except_id
        );
        for entry in self.connections.iter() {
            if entry.key() != except_id {
                if let Err(e) = entry.value().send(message.clone()) {
                    error!("Failed to broadcast to {}: {}", entry.key(), e);
                }
            }
        }
    }

    /// Broadcasts a message to specific connections.
    ///
    /// Only connections whose IDs are in the provided list will receive the message.
    /// Non-existent connection IDs are silently ignored.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example(manager: &ConnectionManager) {
    /// let vip_users = vec![
    ///     "conn_1".to_string(),
    ///     "conn_5".to_string(),
    ///     "conn_10".to_string(),
    /// ];
    /// manager.broadcast_to(&vip_users, Message::text("VIP announcement"));
    /// # }
    /// ```
    pub fn broadcast_to(&self, ids: &[ConnectionId], message: Message) {
        for id in ids {
            if let Some(conn) = self.get(id) {
                if let Err(e) = conn.send(message.clone()) {
                    error!("Failed to send to {}: {}", id, e);
                }
            }
        }
    }

    /// Returns the number of active connections.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example(manager: &ConnectionManager) {
    /// let count = manager.count();
    /// println!("Active connections: {}", count);
    /// # }
    /// ```
    pub fn count(&self) -> usize {
        self.connections.len()
    }

    /// Returns a list of all connection IDs.
    ///
    /// The order of IDs is not guaranteed.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example(manager: &ConnectionManager) {
    /// for id in manager.all_ids() {
    ///     println!("Connection ID: {}", id);
    /// }
    /// # }
    /// ```
    pub fn all_ids(&self) -> Vec<ConnectionId> {
        self.connections.iter().map(|e| e.key().clone()).collect()
    }

    /// Returns clones of all active connections.
    ///
    /// This is useful for batch operations on all connections.
    ///
    /// # Performance
    ///
    /// Since connections are lightweight (they contain Arc internally),
    /// cloning is cheap.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # async fn example(manager: &ConnectionManager) -> Result<()> {
    /// for conn in manager.all_connections() {
    ///     conn.send_text("Shutdown in 1 minute")?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn all_connections(&self) -> Vec<Connection> {
        self.connections.iter().map(|e| e.value().clone()).collect()
    }
}

impl Clone for Connection {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            info: self.info.clone(),
            sender: self.sender.clone(),
        }
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Handles the lifecycle of a WebSocket connection.
///
/// This function manages the entire lifecycle of a WebSocket connection from
/// establishment to termination. It spawns two concurrent tasks:
/// - A read task that receives messages from the client
/// - A write task that sends messages to the client
///
/// # Architecture
///
/// The function uses a split WebSocket stream and an unbounded channel to
/// decouple reading and writing operations. This ensures that slow clients
/// don't block message processing and allows for efficient broadcasting.
///
/// # Lifecycle Events
///
/// 1. Connection is added to the manager
/// 2. `on_connect` callback is invoked
/// 3. Read and write tasks run concurrently
/// 4. When either task completes, both are terminated
/// 5. Connection is removed from the manager
/// 6. `on_disconnect` callback is invoked
///
/// # Arguments
///
/// * `stream` - The WebSocket stream
/// * `conn_id` - Unique identifier for this connection
/// * `peer_addr` - Socket address of the connected client
/// * `manager` - Shared connection manager
/// * `on_message` - Callback invoked when a message is received
/// * `on_connect` - Callback invoked when the connection is established
/// * `on_disconnect` - Callback invoked when the connection is closed
///
/// # Examples
///
/// This function is typically called by the router and not directly by users.
/// However, for custom implementations:
///
/// ```
/// use wsforge::prelude::*;
/// use std::sync::Arc;
/// use tokio_tungstenite::accept_async;
///
/// # async fn example(stream: tokio::net::TcpStream, peer_addr: std::net::SocketAddr) -> Result<()> {
/// let ws_stream = accept_async(stream).await?;
/// let conn_id = "conn_0".to_string();
/// let manager = Arc::new(ConnectionManager::new());
///
/// let on_message = Arc::new(|id: ConnectionId, msg: Message| {
///     println!("Received from {}: {:?}", id, msg);
/// });
///
/// let on_connect = Arc::new(|id: ConnectionId| {
///     println!("Connected: {}", id);
/// });
///
/// let on_disconnect = Arc::new(|id: ConnectionId| {
///     println!("Disconnected: {}", id);
/// });
///
/// handle_websocket(
///     ws_stream,
///     conn_id,
///     peer_addr,
///     manager,
///     on_message,
///     on_connect,
///     on_disconnect,
/// ).await;
/// # Ok(())
/// # }
/// ```
pub async fn handle_websocket(
    stream: WebSocketStream<TcpStream>,
    conn_id: ConnectionId,
    peer_addr: SocketAddr,
    manager: Arc<ConnectionManager>,
    on_message: Arc<dyn Fn(ConnectionId, Message) + Send + Sync>,
    on_connect: Arc<dyn Fn(ConnectionId) + Send + Sync>,
    on_disconnect: Arc<dyn Fn(ConnectionId) + Send + Sync>,
) {
    info!(
        "WebSocket connection established: {} from {}",
        conn_id, peer_addr
    );

    let (mut ws_sender, mut ws_receiver) = stream.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    // Create connection with actual peer address
    let conn = Connection::new(conn_id.clone(), peer_addr, tx);

    // Add connection to manager and get the count
    let _count = manager.add(conn);

    // Verify connection is actually in the map
    let verify_count = manager.count();
    debug!(
        "Connection {} added. Verified count: {}",
        conn_id, verify_count
    );

    // NOW call on_connect AFTER we've verified the connection is added
    on_connect(conn_id.clone());

    // Write task - sends messages to WebSocket
    let conn_id_write = conn_id.clone();
    let write_task = tokio::spawn(async move {
        debug!("Write task started for {}", conn_id_write);

        while let Some(message) = rx.recv().await {
            debug!("📤 Sending message to {}", conn_id_write);

            let msg = message.into_tungstenite();
            if let Err(e) = ws_sender.send(msg).await {
                error!("Failed to send message to {}: {}", conn_id_write, e);
                break;
            }

            debug!("✅ Message sent to {}", conn_id_write);
        }

        info!("Write task ended for {}", conn_id_write);
    });

    // Read task - receives messages from WebSocket
    let conn_id_read = conn_id.clone();
    let read_task = tokio::spawn(async move {
        debug!("Read task started for {}", conn_id_read);

        while let Some(result) = ws_receiver.next().await {
            match result {
                Ok(msg) => {
                    if msg.is_close() {
                        info!("Close message received from {}", conn_id_read);
                        break;
                    }
                    debug!("📨 Received message from {}", conn_id_read);
                    let message = Message::from_tungstenite(msg);
                    on_message(conn_id_read.clone(), message);
                }
                Err(e) => {
                    warn!("WebSocket error for {}: {}", conn_id_read, e);
                    break;
                }
            }
        }
        debug!("Read task ended for {}", conn_id_read);
    });

    // Wait for either task to complete
    tokio::select! {
        _ = write_task => {
            debug!("Write task finished first for {}", conn_id);
        },
        _ = read_task => {
            debug!("Read task finished first for {}", conn_id);
        },
    }

    // Remove connection and call disconnect
    manager.remove(&conn_id);
    on_disconnect(conn_id);
}
