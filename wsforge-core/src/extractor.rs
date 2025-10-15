//! Type-safe data extraction from WebSocket messages and context.
//!
//! This module provides a powerful type extraction system inspired by frameworks like Axum,
//! allowing handlers to declaratively specify what data they need. Extractors automatically
//! parse and validate data from messages, connection state, and application context.
//!
//! # Overview
//!
//! Extractors are types that implement the [`FromMessage`] trait. They can extract:
//! - **Message content**: JSON, binary data, text
//! - **Connection info**: Client address, connection ID, metadata
//! - **Application state**: Shared data like database pools, configuration
//! - **Route parameters**: Path and query parameters from routing
//! - **Custom extensions**: User-defined request-scoped data
//!
//! # Design Philosophy
//!
//! The extractor system follows these principles:
//! - **Type safety**: Extraction failures are caught at runtime with clear errors
//! - **Composability**: Multiple extractors can be used in a single handler
//! - **Zero cost**: Extraction happens only once per handler invocation
//! - **Flexibility**: Custom extractors can be easily implemented
//!
//! # Built-in Extractors
//!
//! | Extractor | Description | Example |
//! |-----------|-------------|---------|
//! | [`Json<T>`] | Deserialize JSON from message | `Json(user): Json<User>` |
//! | [`State<T>`] | Extract shared application state | `State(db): State<Arc<Database>>` |
//! | [`Connection`] | Get the active connection | `conn: Connection` |
//! | [`ConnectInfo`] | Get connection metadata | `ConnectInfo(info)` |
//! | [`Message`] | Get raw message | `msg: Message` |
//! | [`Data`] | Extract binary data | `Data(bytes)` |
//! | [`Path<T>`] | Extract path parameters | `Path(id): Path<UserId>` |
//! | [`Query<T>`] | Extract query parameters | `Query(params): Query<SearchParams>` |
//! | [`Extension<T>`] | Extract custom extensions | `Extension(auth): Extension<Auth>` |
//!
//! # Examples
//!
//! ## Simple JSON Extraction
//!
//! ```
//! use wsforge::prelude::*;
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! struct ChatMessage {
//!     username: String,
//!     text: String,
//! }
//!
//! async fn chat_handler(Json(msg): Json<ChatMessage>) -> Result<String> {
//!     println!("{} says: {}", msg.username, msg.text);
//!     Ok(format!("Message from {} received", msg.username))
//! }
//! ```
//!
//! ## Multiple Extractors
//!
//! ```
//! use wsforge::prelude::*;
//! use serde::Deserialize;
//! use std::sync::Arc;
//!
//! #[derive(Deserialize)]
//! struct GameMove {
//!     player: String,
//!     action: String,
//! }
//!
//! async fn game_handler(
//!     Json(game_move): Json<GameMove>,
//!     conn: Connection,
//!     State(manager): State<Arc<ConnectionManager>>,
//! ) -> Result<()> {
//!     println!("Player {} from connection {} made move: {}",
//!         game_move.player, conn.id(), game_move.action);
//!
//!     // Broadcast to other players
//!     manager.broadcast_except(conn.id(),
//!         Message::text(format!("{} moved", game_move.player)));
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Custom Extractors
//!
//! ```
//! use wsforge::prelude::*;
//! use async_trait::async_trait;
//!
//! // Custom extractor for authenticated users
//! struct AuthUser {
//!     user_id: u64,
//!     username: String,
//! }
//!
//! #[async_trait]
//! impl FromMessage for AuthUser {
//!     async fn from_message(
//!         message: &Message,
//!         conn: &Connection,
//!         state: &AppState,
//!         extensions: &Extensions,
//!     ) -> Result<Self> {
//!         // Extract authentication token from message
//!         let text = message.as_text()
//!             .ok_or_else(|| Error::extractor("Message must be text"))?;
//!
//!         // Validate and extract user info
//!         // (In production, verify JWT, session token, etc.)
//!         Ok(AuthUser {
//!             user_id: 123,
//!             username: "user".to_string(),
//!         })
//!     }
//! }
//!
//! async fn protected_handler(user: AuthUser) -> Result<String> {
//!     Ok(format!("Hello, {}!", user.username))
//! }
//! ```

use crate::connection::{Connection, ConnectionInfo};
use crate::error::{Error, Result};
use crate::message::Message;
use crate::state::AppState;
use async_trait::async_trait;
use dashmap::DashMap;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;

/// Trait for types that can be extracted from WebSocket messages and context.
///
/// This trait is the core of the extractor system. Types that implement `FromMessage`
/// can be used as handler parameters, and the framework will automatically extract
/// and validate the data before calling the handler.
///
/// # Implementation Guidelines
///
/// When implementing custom extractors:
/// 1. **Be specific**: Return clear error messages when extraction fails
/// 2. **Be efficient**: Avoid expensive operations if possible
/// 3. **Be safe**: Validate all extracted data
/// 4. **Document**: Explain what data is extracted and any requirements
///
/// # Examples
///
/// ## Simple Extractor
///
/// ```
/// use wsforge::prelude::*;
/// use async_trait::async_trait;
///
/// struct MessageLength(usize);
///
/// #[async_trait]
/// impl FromMessage for MessageLength {
///     async fn from_message(
///         message: &Message,
///         _conn: &Connection,
///         _state: &AppState,
///         _extensions: &Extensions,
///     ) -> Result<Self> {
///         let len = message.as_bytes().len();
///         Ok(MessageLength(len))
///     }
/// }
///
/// async fn handler(MessageLength(len): MessageLength) -> Result<String> {
///     Ok(format!("Message length: {}", len))
/// }
/// ```
///
/// ## Extractor with Validation
///
/// ```
/// use wsforge::prelude::*;
/// use async_trait::async_trait;
///
/// struct ValidatedText(String);
///
/// #[async_trait]
/// impl FromMessage for ValidatedText {
///     async fn from_message(
///         message: &Message,
///         _conn: &Connection,
///         _state: &AppState,
///         _extensions: &Extensions,
///     ) -> Result<Self> {
///         let text = message.as_text()
///             .ok_or_else(|| Error::extractor("Message must be text"))?;
///
///         if text.is_empty() {
///             return Err(Error::extractor("Text cannot be empty"));
///         }
///
///         if text.len() > 1000 {
///             return Err(Error::extractor("Text too long (max 1000 characters)"));
///         }
///
///         Ok(ValidatedText(text.to_string()))
///     }
/// }
/// ```
#[async_trait]
pub trait FromMessage: Sized {
    /// Extracts `Self` from the message and context.
    ///
    /// # Arguments
    ///
    /// * `message` - The WebSocket message being processed
    /// * `conn` - The connection that sent the message
    /// * `state` - The application state
    /// * `extensions` - Request-scoped extension data
    ///
    /// # Errors
    ///
    /// Returns an error if extraction fails. Common reasons include:
    /// - Required data is missing
    /// - Data format is invalid
    /// - Type mismatch
    /// - Validation failure
    async fn from_message(
        message: &Message,
        conn: &Connection,
        state: &AppState,
        extensions: &Extensions,
    ) -> Result<Self>;
}

/// Container for request-scoped extension data.
///
/// Extensions provide a way to pass arbitrary data through the request pipeline.
/// This is useful for middleware to attach data (like authentication info, request IDs)
/// that handlers can later extract.
///
/// # Thread Safety
///
/// Extensions are thread-safe and can be safely shared across tasks.
///
/// # Examples
///
/// ## Adding and Retrieving Data
///
/// ```
/// use wsforge::prelude::*;
///
/// # fn example() {
/// let extensions = Extensions::new();
///
/// // Add data
/// extensions.insert("request_id", "req_123");
/// extensions.insert("user_id", 42_u64);
///
/// // Retrieve data
/// if let Some(request_id) = extensions.get::<&str>("request_id") {
///     println!("Request ID: {}", request_id);
/// }
///
/// if let Some(user_id) = extensions.get::<u64>("user_id") {
///     println!("User ID: {}", user_id);
/// }
/// # }
/// ```
///
/// ## Use in Middleware
///
/// ```
/// use wsforge::prelude::*;
///
/// async fn auth_middleware(
///     msg: Message,
///     conn: Connection,
///     extensions: &Extensions,
/// ) -> Result<()> {
///     // Extract and validate auth token
///     let token = extract_token(&msg)?;
///     let user_id = validate_token(&token)?;
///
///     // Store for handler to use
///     extensions.insert("user_id", user_id);
///
///     Ok(())
/// }
///
/// # fn extract_token(_: &Message) -> Result<String> { Ok("token".to_string()) }
/// # fn validate_token(_: &str) -> Result<u64> { Ok(123) }
/// ```
#[derive(Clone)]
pub struct Extensions {
    data: Arc<DashMap<String, Arc<dyn std::any::Any + Send + Sync>>>,
}

impl Extensions {
    /// Creates a new empty `Extensions` container.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// let extensions = Extensions::new();
    /// ```
    pub fn new() -> Self {
        Self {
            data: Arc::new(DashMap::new()),
        }
    }

    /// Inserts a value into the extensions.
    ///
    /// The value is stored under the given key and can be retrieved later
    /// using the same key and type.
    ///
    /// # Arguments
    ///
    /// * `key` - A unique identifier for this value
    /// * `value` - The value to store (must be `Send + Sync + 'static`)
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example() {
    /// let extensions = Extensions::new();
    ///
    /// // Store different types
    /// extensions.insert("count", 42_u32);
    /// extensions.insert("name", "Alice".to_string());
    /// extensions.insert("active", true);
    /// # }
    /// ```
    pub fn insert<T: Send + Sync + 'static>(&self, key: impl Into<String>, value: T) {
        self.data.insert(key.into(), Arc::new(value));
    }

    /// Retrieves a value from the extensions.
    ///
    /// Returns `None` if the key doesn't exist or if the stored type doesn't
    /// match the requested type.
    ///
    /// # Type Safety
    ///
    /// The returned value must match the type that was originally inserted.
    /// Attempting to retrieve with a different type will return `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example() {
    /// let extensions = Extensions::new();
    /// extensions.insert("count", 42_u32);
    ///
    /// // Correct type - succeeds
    /// let count: Option<Arc<u32>> = extensions.get("count");
    /// assert_eq!(*count.unwrap(), 42);
    ///
    /// // Wrong type - returns None
    /// let wrong: Option<Arc<String>> = extensions.get("count");
    /// assert!(wrong.is_none());
    /// # }
    /// ```
    pub fn get<T: Send + Sync + 'static>(&self, key: &str) -> Option<Arc<T>> {
        self.data
            .get(key)
            .and_then(|arc| arc.value().clone().downcast::<T>().ok())
    }
}

impl Default for Extensions {
    fn default() -> Self {
        Self::new()
    }
}

/// Extractor for shared application state.
///
/// Use this to access data that's shared across all connections, such as:
/// - Database connection pools
/// - Configuration
/// - Caches
/// - Connection managers
///
/// # Type Parameter
///
/// The generic parameter `T` should be wrapped in `Arc` since state is shared.
///
/// # Examples
///
/// ## Accessing Connection Manager
///
/// ```
/// use wsforge::prelude::*;
/// use std::sync::Arc;
///
/// async fn broadcast_handler(
///     msg: Message,
///     State(manager): State<Arc<ConnectionManager>>,
/// ) -> Result<()> {
///     manager.broadcast(msg);
///     Ok(())
/// }
/// ```
///
/// ## Custom State Type
///
/// ```
/// use wsforge::prelude::*;
/// use std::sync::Arc;
///
/// struct AppConfig {
///     max_message_size: usize,
///     rate_limit: u32,
/// }
///
/// async fn handler(State(config): State<Arc<AppConfig>>) -> Result<String> {
///     Ok(format!("Max message size: {}", config.max_message_size))
/// }
/// ```
pub struct State<T>(pub Arc<T>);

#[async_trait]
impl<T: Send + Sync + 'static> FromMessage for State<T> {
    async fn from_message(
        _message: &Message,
        _conn: &Connection,
        state: &AppState,
        _extensions: &Extensions,
    ) -> Result<Self> {
        state
            .get::<T>()
            .ok_or_else(|| Error::extractor("State not found"))
            .map(State)
    }
}

/// Extractor for JSON data from messages.
///
/// Automatically deserializes the message content as JSON into the specified type.
/// The type must implement `serde::Deserialize`.
///
/// # Errors
///
/// Returns an error if:
/// - The message is not text
/// - The JSON is malformed
/// - Required fields are missing
/// - Type constraints are not satisfied
///
/// # Examples
///
/// ## Simple Struct
///
/// ```
/// use wsforge::prelude::*;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct LoginRequest {
///     username: String,
///     password: String,
/// }
///
/// async fn login_handler(Json(req): Json<LoginRequest>) -> Result<String> {
///     // Validate credentials
///     Ok(format!("Login attempt by {}", req.username))
/// }
/// ```
///
/// ## With Validation
///
/// ```
/// use wsforge::prelude::*;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct CreateUser {
///     #[serde(deserialize_with = "validate_username")]
///     username: String,
///     age: u8,
/// }
///
/// async fn create_user(Json(user): Json<CreateUser>) -> Result<String> {
///     Ok(format!("Creating user: {}", user.username))
/// }
///
/// # fn validate_username<'de, D>(_: D) -> std::result::Result<String, D::Error>
/// # where D: serde::Deserializer<'de> {
/// #     Ok("valid".to_string())
/// # }
/// ```
///
/// ## Nested Structures
///
/// ```
/// use wsforge::prelude::*;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct GameState {
///     player: Player,
///     score: u32,
/// }
///
/// #[derive(Deserialize)]
/// struct Player {
///     id: u64,
///     name: String,
/// }
///
/// async fn update_game(Json(state): Json<GameState>) -> Result<()> {
///     println!("Player {} score: {}", state.player.name, state.score);
///     Ok(())
/// }
/// ```
pub struct Json<T>(pub T);

#[async_trait]
impl<T: DeserializeOwned + Send> FromMessage for Json<T> {
    async fn from_message(
        message: &Message,
        _conn: &Connection,
        _state: &AppState,
        _extensions: &Extensions,
    ) -> Result<Self> {
        let data: T = message.json()?;
        Ok(Json(data))
    }
}

impl<T: Serialize> Json<T> {
    /// Converts this JSON extractor back into a message.
    ///
    /// This is useful for echoing back modified data or creating responses.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Deserialize, Serialize)]
    /// struct Echo {
    ///     text: String,
    /// }
    ///
    /// async fn echo_handler(Json(data): Json<Echo>) -> Result<Message> {
    ///     Json(data).into_message()
    /// }
    /// ```
    pub fn into_message(self) -> Result<Message> {
        let json = serde_json::to_string(&self.0)?;
        Ok(Message::text(json))
    }
}

/// Extractor for the active connection.
///
/// Provides access to the connection that sent the message, allowing you to:
/// - Get the connection ID
/// - Access connection metadata
/// - Send messages back to the specific client
///
/// # Examples
///
/// ## Sending Response
///
/// ```
/// use wsforge::prelude::*;
///
/// async fn handler(msg: Message, conn: Connection) -> Result<()> {
///     conn.send_text("Message received!")?;
///     Ok(())
/// }
/// ```
///
/// ## Using Connection Info
///
/// ```
/// use wsforge::prelude::*;
///
/// async fn handler(conn: Connection) -> Result<String> {
///     let info = conn.info();
///     Ok(format!("Client from {} connected at {}",
///         info.addr, info.connected_at))
/// }
/// ```
#[async_trait]
impl FromMessage for Connection {
    async fn from_message(
        _message: &Message,
        conn: &Connection,
        _state: &AppState,
        _extensions: &Extensions,
    ) -> Result<Self> {
        Ok(conn.clone())
    }
}

/// Extractor for connection metadata.
///
/// Provides detailed information about the connection, including:
/// - Connection ID
/// - Client socket address
/// - Connection timestamp
/// - Protocol information
///
/// # Examples
///
/// ## Logging Connection Info
///
/// ```
/// use wsforge::prelude::*;
///
/// async fn handler(ConnectInfo(info): ConnectInfo) -> Result<String> {
///     println!("Connection {} from {} at {}",
///         info.id, info.addr, info.connected_at);
///     Ok("Connected".to_string())
/// }
/// ```
pub struct ConnectInfo(pub ConnectionInfo);

#[async_trait]
impl FromMessage for ConnectInfo {
    async fn from_message(
        _message: &Message,
        conn: &Connection,
        _state: &AppState,
        _extensions: &Extensions,
    ) -> Result<Self> {
        Ok(ConnectInfo(conn.info.clone()))
    }
}

/// Extractor for the raw message.
///
/// Use this when you need access to the complete message without
/// automatic deserialization.
///
/// # Examples
///
/// ## Raw Message Processing
///
/// ```
/// use wsforge::prelude::*;
///
/// async fn handler(msg: Message) -> Result<String> {
///     if msg.is_text() {
///         Ok(format!("Text: {}", msg.as_text().unwrap()))
///     } else if msg.is_binary() {
///         Ok(format!("Binary: {} bytes", msg.as_bytes().len()))
///     } else {
///         Ok("Unknown message type".to_string())
///     }
/// }
/// ```
#[async_trait]
impl FromMessage for Message {
    async fn from_message(
        message: &Message,
        _conn: &Connection,
        _state: &AppState,
        _extensions: &Extensions,
    ) -> Result<Self> {
        Ok(message.clone())
    }
}

/// Extractor for raw binary data.
///
/// Extracts the message payload as raw bytes. Works with both text and binary messages.
///
/// # Examples
///
/// ## Processing Binary Data
///
/// ```
/// use wsforge::prelude::*;
///
/// async fn handler(Data(bytes): Data) -> Result<String> {
///     println!("Received {} bytes", bytes.len());
///     Ok(format!("Processed {} bytes", bytes.len()))
/// }
/// ```
pub struct Data(pub Vec<u8>);

#[async_trait]
impl FromMessage for Data {
    async fn from_message(
        message: &Message,
        _conn: &Connection,
        _state: &AppState,
        _extensions: &Extensions,
    ) -> Result<Self> {
        Ok(Data(message.data.clone()))
    }
}

/// Extractor for path parameters.
///
/// Extracts typed parameters from the request path. The type must implement
/// `serde::Deserialize` and be stored in extensions by routing middleware.
///
/// # Examples
///
/// ## Single Parameter
///
/// ```
/// use wsforge::prelude::*;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct UserId(u64);
///
/// async fn get_user(Path(UserId(id)): Path<UserId>) -> Result<String> {
///     Ok(format!("Getting user {}", id))
/// }
/// ```
///
/// ## Multiple Parameters
///
/// ```
/// use wsforge::prelude::*;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct RoomParams {
///     room_id: String,
///     user_id: u64,
/// }
///
/// async fn join_room(Path(params): Path<RoomParams>) -> Result<String> {
///     Ok(format!("User {} joining room {}", params.user_id, params.room_id))
/// }
/// ```
pub struct Path<T>(pub T);

#[async_trait]
impl<T: DeserializeOwned + Send + Sync + Clone + 'static> FromMessage for Path<T> {
    async fn from_message(
        _message: &Message,
        _conn: &Connection,
        _state: &AppState,
        extensions: &Extensions,
    ) -> Result<Self> {
        extensions
            .get::<T>("path_params")
            .ok_or_else(|| Error::extractor("Path parameters not found"))
            .map(|arc| Path((*arc).clone()))
    }
}

/// Extractor for query parameters.
///
/// Extracts typed parameters from the query string. The type must implement
/// `serde::Deserialize` and be stored in extensions during connection establishment.
///
/// # Examples
///
/// ## Search Parameters
///
/// ```
/// use wsforge::prelude::*;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct SearchQuery {
///     q: String,
///     limit: Option<u32>,
/// }
///
/// async fn search(Query(params): Query<SearchQuery>) -> Result<String> {
///     let limit = params.limit.unwrap_or(10);
///     Ok(format!("Searching for '{}' (limit: {})", params.q, limit))
/// }
/// ```
pub struct Query<T>(pub T);

#[async_trait]
impl<T: DeserializeOwned + Send + Sync + Clone + 'static> FromMessage for Query<T> {
    async fn from_message(
        _message: &Message,
        _conn: &Connection,
        _state: &AppState,
        extensions: &Extensions,
    ) -> Result<Self> {
        extensions
            .get::<T>("query_params")
            .ok_or_else(|| Error::extractor("Query parameters not found"))
            .map(|arc| Query((*arc).clone()))
    }
}

/// Extractor for custom extension data.
///
/// Retrieves data that was previously stored in extensions by middleware or other handlers.
///
/// # Examples
///
/// ## Authentication Data
///
/// ```
/// use wsforge::prelude::*;
/// use std::sync::Arc;
///
/// #[derive(Clone)]
/// struct AuthData {
///     user_id: u64,
///     role: String,
/// }
///
/// async fn protected_handler(Extension(auth): Extension<AuthData>) -> Result<String> {
///     Ok(format!("User {} with role {}", auth.user_id, auth.role))
/// }
/// ```
pub struct Extension<T>(pub Arc<T>);

#[async_trait]
impl<T: Send + Sync + Clone + 'static> FromMessage for Extension<T> {
    async fn from_message(
        _message: &Message,
        _conn: &Connection,
        _state: &AppState,
        extensions: &Extensions,
    ) -> Result<Self> {
        extensions
            .get::<T>(std::any::type_name::<T>())
            .ok_or_else(|| Error::extractor("Extension not found"))
            .map(Extension)
    }
}
