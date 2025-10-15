//! Handler traits and implementations for WebSocket message processing.
//!
//! This module provides the foundation for handling WebSocket messages with a flexible,
//! type-safe interface. Handlers can accept multiple extractors and return various response types,
//! making it easy to build complex message processing logic.
//!
//! # Overview
//!
//! The handler system consists of three main components:
//! - [`Handler`] trait: Core trait that all handlers implement
//! - [`IntoResponse`] trait: Converts handler return values into messages
//! - [`HandlerService`]: Wrapper that bridges async functions to the Handler trait
//!
//! # Handler Signatures
//!
//! Handlers can have various signatures with different combinations of extractors:
//!
//! ```
//! use wsforge::prelude::*;
//! use std::sync::Arc;
//!
//! // No extractors
//! async fn simple() -> Result<String> {
//!     Ok("Hello!".to_string())
//! }
//!
//! // Single extractor
//! async fn with_message(msg: Message) -> Result<String> {
//!     Ok("Received".to_string())
//! }
//!
//! // Multiple extractors
//! async fn complex(
//!     Json(data): Json<serde_json::Value>,
//!     conn: Connection,
//!     State(manager): State<Arc<ConnectionManager>>,
//! ) -> Result<()> {
//!     Ok(())
//! }
//! ```
//!
//! # Return Types
//!
//! Handlers can return various types that implement [`IntoResponse`]:
//!
//! | Return Type | Description | Example |
//! |-------------|-------------|---------|
//! | `()` | No response sent | `async fn handler() -> Result<()>` |
//! | `String` | Text message | `async fn handler() -> Result<String>` |
//! | `&str` | Text message | `async fn handler() -> Result<&str>` |
//! | `Message` | Raw message | `async fn handler() -> Result<Message>` |
//! | `Vec<u8>` | Binary message | `async fn handler() -> Result<Vec<u8>>` |
//! | `JsonResponse<T>` | JSON response | `async fn handler() -> Result<JsonResponse<T>>` |
//! | `Result<T>` | Automatic error handling | Any of above wrapped in `Result` |
//!
//! # Examples
//!
//! ## Echo Handler
//!
//! ```
//! use wsforge::prelude::*;
//!
//! async fn echo_handler(msg: Message) -> Result<Message> {
//!     Ok(msg)
//! }
//! ```
//!
//! ## JSON Processing
//!
//! ```
//! use wsforge::prelude::*;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Deserialize)]
//! struct Request {
//!     action: String,
//! }
//!
//! #[derive(Serialize)]
//! struct Response {
//!     status: String,
//!     result: String,
//! }
//!
//! async fn json_handler(Json(req): Json<Request>) -> Result<JsonResponse<Response>> {
//!     Ok(JsonResponse(Response {
//!         status: "success".to_string(),
//!         result: format!("Executed: {}", req.action),
//!     }))
//! }
//! ```
//!
//! ## Broadcasting
//!
//! ```
//! use wsforge::prelude::*;
//! use std::sync::Arc;
//!
//! async fn broadcast_handler(
//!     msg: Message,
//!     conn: Connection,
//!     State(manager): State<Arc<ConnectionManager>>,
//! ) -> Result<()> {
//!     // Broadcast to everyone except sender
//!     manager.broadcast_except(conn.id(), msg);
//!     Ok(())
//! }
//! ```

use crate::connection::Connection;
use crate::error::Result;
use crate::extractor::{Extensions, FromMessage};
use crate::message::Message;
use crate::state::AppState;
use async_trait::async_trait;
use serde::Serialize;
use std::marker::PhantomData;
use std::sync::Arc;

/// Trait for converting handler return values into WebSocket messages.
///
/// This trait allows handlers to return various types that are automatically
/// converted to messages or no response. The framework handles the conversion
/// transparently.
///
/// # Automatic Implementations
///
/// The trait is implemented for common return types:
/// - `()` - No response is sent
/// - `String` - Sent as text message
/// - `&str` - Sent as text message
/// - `Message` - Sent as-is
/// - `Vec<u8>` - Sent as binary message
/// - `Result<T>` - Automatically handles errors
///
/// # Examples
///
/// ## Implementing for Custom Types
///
/// ```
/// use wsforge::prelude::*;
/// use async_trait::async_trait;
///
/// struct CustomResponse {
///     code: u32,
///     data: String,
/// }
///
/// #[async_trait]
/// impl IntoResponse for CustomResponse {
///     async fn into_response(self) -> Result<Option<Message>> {
///         let text = format!("{}:{}", self.code, self.data);
///         Ok(Some(Message::text(text)))
///     }
/// }
///
/// async fn handler() -> Result<CustomResponse> {
///     Ok(CustomResponse {
///         code: 200,
///         data: "Success".to_string(),
///     })
/// }
/// ```
#[async_trait]
pub trait IntoResponse: Send {
    /// Converts this value into an optional message.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(message))` - A message to send back to the client
    /// - `Ok(None)` - No response should be sent
    /// - `Err(error)` - An error occurred during conversion
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # async fn example() -> Result<()> {
    /// let response = "Hello".to_string();
    /// let message = response.into_response().await?;
    /// assert!(message.is_some());
    /// # Ok(())
    /// # }
    /// ```
    async fn into_response(self) -> Result<Option<Message>>;
}

/// Response that sends nothing back to the client.
///
/// Use this when the handler only performs side effects and doesn't need
/// to send a response.
///
/// # Examples
///
/// ```
/// use wsforge::prelude::*;
/// use std::sync::Arc;
///
/// async fn log_handler(
///     msg: Message,
///     State(manager): State<Arc<ConnectionManager>>,
/// ) -> Result<()> {
///     println!("Received message, {} connections active", manager.count());
///     Ok(())
/// }
/// ```
#[async_trait]
impl IntoResponse for () {
    async fn into_response(self) -> Result<Option<Message>> {
        Ok(None)
    }
}

/// Response that sends the message as-is.
///
/// Use this when you have full control over the message construction.
///
/// # Examples
///
/// ```
/// use wsforge::prelude::*;
///
/// async fn custom_handler() -> Result<Message> {
///     Ok(Message::text("Custom response"))
/// }
/// ```
#[async_trait]
impl IntoResponse for Message {
    async fn into_response(self) -> Result<Option<Message>> {
        Ok(Some(self))
    }
}

/// Response that sends a string as a text message.
///
/// # Examples
///
/// ```
/// use wsforge::prelude::*;
///
/// async fn greeting_handler() -> Result<String> {
///     Ok(format!("Hello at {}", chrono::Utc::now()))
/// }
/// ```
#[async_trait]
impl IntoResponse for String {
    async fn into_response(self) -> Result<Option<Message>> {
        Ok(Some(Message::text(self)))
    }
}

/// Response that sends a string slice as a text message.
///
/// # Examples
///
/// ```
/// use wsforge::prelude::*;
///
/// async fn static_handler() -> Result<&'static str> {
///     Ok("Static response")
/// }
/// ```
#[async_trait]
impl IntoResponse for &str {
    async fn into_response(self) -> Result<Option<Message>> {
        Ok(Some(Message::text(self.to_string())))
    }
}

/// Response that sends binary data.
///
/// # Examples
///
/// ```
/// use wsforge::prelude::*;
///
/// async fn binary_handler() -> Result<Vec<u8>> {
///     Ok(vec![0x01, 0x02, 0x03, 0x04])
/// }
/// ```
#[async_trait]
impl IntoResponse for Vec<u8> {
    async fn into_response(self) -> Result<Option<Message>> {
        Ok(Some(Message::binary(self)))
    }
}

/// Automatic error handling for handler results.
///
/// When a handler returns `Result<T>`, errors are automatically converted
/// to error messages sent back to the client.
///
/// # Examples
///
/// ```
/// use wsforge::prelude::*;
///
/// async fn validated_handler(msg: Message) -> Result<String> {
///     let text = msg.as_text()
///         .ok_or_else(|| Error::custom("Message must be text"))?;
///
///     if text.is_empty() {
///         return Err(Error::custom("Message cannot be empty"));
///     }
///
///     Ok(format!("Processed: {}", text))
/// }
/// ```
#[async_trait]
impl<T: IntoResponse> IntoResponse for Result<T> {
    async fn into_response(self) -> Result<Option<Message>> {
        match self {
            Ok(resp) => resp.into_response().await,
            Err(e) => Ok(Some(Message::text(format!("Error: {}", e)))),
        }
    }
}

/// JSON response wrapper.
///
/// Automatically serializes data to JSON and sends it as a text message.
/// The type must implement `serde::Serialize`.
///
/// # Examples
///
/// ## Simple Response
///
/// ```
/// use wsforge::prelude::*;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct UserInfo {
///     id: u64,
///     name: String,
/// }
///
/// async fn user_handler() -> Result<JsonResponse<UserInfo>> {
///     Ok(JsonResponse(UserInfo {
///         id: 123,
///         name: "Alice".to_string(),
///     }))
/// }
/// ```
///
/// ## Dynamic JSON
///
/// ```
/// use wsforge::prelude::*;
///
/// async fn stats_handler() -> Result<JsonResponse<serde_json::Value>> {
///     let stats = serde_json::json!({
///         "users": 42,
///         "messages": 1337,
///         "uptime": 86400,
///     });
///     Ok(JsonResponse(stats))
/// }
/// ```
pub struct JsonResponse<T: Serialize>(pub T);

#[async_trait]
impl<T: Serialize + Send> IntoResponse for JsonResponse<T> {
    async fn into_response(self) -> Result<Option<Message>> {
        let json = serde_json::to_string(&self.0)?;
        Ok(Some(Message::text(json)))
    }
}

/// Core trait for message handlers.
///
/// This trait is automatically implemented for async functions that match
/// the required signature. You typically don't implement this trait directly;
/// instead, use the [`handler()`] function to wrap your async functions.
///
/// # Automatic Implementation
///
/// Any async function with up to 8 extractor parameters that returns
/// `impl IntoResponse` automatically implements this trait.
///
/// # Examples
///
/// ```
/// use wsforge::prelude::*;
///
/// // This function automatically implements Handler
/// async fn my_handler(msg: Message, conn: Connection) -> Result<String> {
///     Ok(format!("Received from {}", conn.id()))
/// }
///
/// # fn example() {
/// // Wrap it in a HandlerService
/// let handler = handler(my_handler);
/// # }
/// ```
#[async_trait]
pub trait Handler: Send + Sync + 'static {
    /// Processes a message and returns an optional response.
    ///
    /// This method is called by the framework when a message is received.
    /// It extracts the required data and executes the handler logic.
    ///
    /// # Arguments
    ///
    /// * `message` - The received WebSocket message
    /// * `conn` - The connection that sent the message
    /// * `state` - The application state
    /// * `extensions` - Request-scoped extension data
    ///
    /// # Returns
    ///
    /// - `Ok(Some(message))` - Send this message back to the client
    /// - `Ok(None)` - Don't send any response
    /// - `Err(error)` - An error occurred during processing
    async fn call(
        &self,
        message: Message,
        conn: Connection,
        state: AppState,
        extensions: Extensions,
    ) -> Result<Option<Message>>;
}

/// Service wrapper for handler functions.
///
/// This struct wraps async functions and implements the [`Handler`] trait.
/// It uses a phantom type parameter to distinguish between different handler signatures.
///
/// You typically don't construct this directly; use the [`handler()`] function instead.
///
/// # Type Parameters
///
/// * `F` - The function type
/// * `T` - Phantom type representing the extractor tuple
pub struct HandlerService<F, T> {
    handler: F,
    _marker: PhantomData<fn() -> T>,
}

impl<F, T> HandlerService<F, T> {
    /// Creates a new `HandlerService` wrapping the given function.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// async fn my_handler() -> Result<String> {
    ///     Ok("Hello".to_string())
    /// }
    ///
    /// let service = HandlerService::new(my_handler);
    /// ```
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            _marker: PhantomData,
        }
    }
}

// Macro to implement Handler trait for various argument counts
macro_rules! impl_handler {
    (
        $($ty:ident),*
    ) => {
        #[allow(non_snake_case)]
        #[async_trait]
        impl<F, Fut, Res, $($ty,)*> Handler for HandlerService<F, ($($ty,)*)>
        where
            F: Fn($($ty,)*) -> Fut + Send + Sync + 'static,
            Fut: std::future::Future<Output = Res> + Send + 'static,
            Res: IntoResponse,
            $( $ty: FromMessage + Send + 'static, )*
        {
            async fn call(
                &self,
                message: Message,
                conn: Connection,
                state: AppState,
                extensions: Extensions,
            ) -> Result<Option<Message>> {
                $(
                    let $ty = $ty::from_message(&message, &conn, &state, &extensions).await?;
                )*

                let response = (self.handler)($($ty,)*).await;
                response.into_response().await
            }
        }

        impl<F, Fut, Res, $($ty,)*> IntoHandler<($($ty,)*)> for F
        where
            F: Fn($($ty,)*) -> Fut + Send + Sync + 'static,
            Fut: std::future::Future<Output = Res> + Send + 'static,
            Res: IntoResponse,
            $( $ty: FromMessage + Send + 'static, )*
        {
            type Handler = HandlerService<F, ($($ty,)*)>;

            fn into_handler(self) -> Self::Handler {
                HandlerService::new(self)
            }
        }
    };
}

/// Helper trait for converting functions into handlers.
///
/// This trait is automatically implemented for async functions and is used
/// internally by the [`handler()`] function.
///
/// # Type Parameters
///
/// * `T` - Tuple representing the extractor types
pub trait IntoHandler<T> {
    /// The resulting handler type.
    type Handler: Handler;

    /// Converts this function into a handler.
    fn into_handler(self) -> Self::Handler;
}

// Implement for 0 to 8 arguments
impl_handler!();
impl_handler!(T1);
impl_handler!(T1, T2);
impl_handler!(T1, T2, T3);
impl_handler!(T1, T2, T3, T4);
impl_handler!(T1, T2, T3, T4, T5);
impl_handler!(T1, T2, T3, T4, T5, T6);
impl_handler!(T1, T2, T3, T4, T5, T6, T7);
impl_handler!(T1, T2, T3, T4, T5, T6, T7, T8);

/// Converts an async function into a handler.
///
/// This is the main function you use to create handlers from async functions.
/// It automatically detects the function signature and creates the appropriate
/// handler implementation.
///
/// # Type Inference
///
/// The function uses type inference to determine the extractor types. You don't
/// need to specify any type parameters explicitly.
///
/// # Examples
///
/// ## Simple Handler
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
/// ## Handler with State
///
/// ```
/// use wsforge::prelude::*;
/// use std::sync::Arc;
///
/// async fn broadcast(
///     msg: Message,
///     State(manager): State<Arc<ConnectionManager>>,
/// ) -> Result<()> {
///     manager.broadcast(msg);
///     Ok(())
/// }
///
/// # fn example() {
/// let router = Router::new()
///     .default_handler(handler(broadcast));
/// # }
/// ```
///
/// ## Handler with Multiple Extractors
///
/// ```
/// use wsforge::prelude::*;
/// use serde::Deserialize;
/// use std::sync::Arc;
///
/// #[derive(Deserialize)]
/// struct GameMove {
///     player_id: u64,
///     action: String,
/// }
///
/// async fn process_move(
///     Json(game_move): Json<GameMove>,
///     conn: Connection,
///     State(manager): State<Arc<ConnectionManager>>,
/// ) -> Result<String> {
///     println!("Player {} from {} made move: {}",
///         game_move.player_id, conn.id(), game_move.action);
///
///     manager.broadcast_except(conn.id(),
///         Message::text(format!("Player {} moved", game_move.player_id)));
///
///     Ok("Move processed".to_string())
/// }
///
/// # fn example() {
/// let router = Router::new()
///     .route("/game/move", handler(process_move));
/// # }
/// ```
///
/// ## Handler Returning JSON
///
/// ```
/// use wsforge::prelude::*;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Stats {
///     users: u32,
///     messages: u64,
/// }
///
/// async fn get_stats() -> Result<JsonResponse<Stats>> {
///     Ok(JsonResponse(Stats {
///         users: 42,
///         messages: 1337,
///     }))
/// }
///
/// # fn example() {
/// let router = Router::new()
///     .route("/stats", handler(get_stats));
/// # }
/// ```
pub fn handler<F, T>(f: F) -> Arc<dyn Handler>
where
    F: IntoHandler<T>,
{
    Arc::new(f.into_handler())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_string_into_response() {
        let response = "test".to_string();
        let result = response.into_response().await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_unit_into_response() {
        let response = ();
        let result = response.into_response().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_json_response() {
        use serde_json::json;

        let data = json!({"key": "value"});
        let response = JsonResponse(data);
        let result = response.into_response().await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_handler_creation() {
        async fn test_handler() -> Result<String> {
            Ok("test".to_string())
        }

        let _handler = handler(test_handler);
    }
}
