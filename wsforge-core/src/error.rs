//! Error types and result handling for WsForge.
//!
//! This module provides a unified error type that covers all possible error conditions
//! in the WsForge framework, from WebSocket protocol errors to application-level errors.
//!
//! # Overview
//!
//! The error handling in WsForge is designed to be:
//! - **Ergonomic**: Using `Result<T>` as a type alias for `std::result::Result<T, Error>`
//! - **Informative**: Each variant provides context about what went wrong
//! - **Composable**: Implements `From` traits for automatic error conversion
//! - **Debuggable**: All errors implement `Display` and `Debug` for easy troubleshooting
//!
//! # Error Categories
//!
//! Errors are organized into several categories:
//! - **Protocol Errors**: WebSocket and IO errors
//! - **Serialization Errors**: JSON parsing and serialization failures
//! - **Framework Errors**: Connection and routing issues
//! - **Application Errors**: Custom business logic errors
//!
//! # Examples
//!
//! ## Basic Error Handling
//!
//! ```
//! use wsforge::prelude::*;
//!
//! async fn handler(msg: Message) -> Result<String> {
//!     // Automatic error conversion from serde_json::Error
//!     let data: serde_json::Value = msg.json()?;
//!
//!     // Create custom error
//!     if data.is_null() {
//!         return Err(Error::custom("Data cannot be null"));
//!     }
//!
//!     Ok("Success".to_string())
//! }
//! ```
//!
//! ## Pattern Matching on Errors
//!
//! ```
//! use wsforge::prelude::*;
//!
//! # async fn example() {
//! # let result: Result<()> = Ok(());
//! match result {
//!     Ok(_) => println!("Success!"),
//!     Err(Error::ConnectionNotFound(id)) => {
//!         eprintln!("Connection {} not found", id);
//!     }
//!     Err(Error::Json(e)) => {
//!         eprintln!("JSON error: {}", e);
//!     }
//!     Err(e) => {
//!         eprintln!("Other error: {}", e);
//!     }
//! }
//! # }
//! ```
//!
//! ## Creating Custom Errors
//!
//! ```
//! use wsforge::prelude::*;
//!
//! async fn validate_user(username: &str) -> Result<()> {
//!     if username.is_empty() {
//!         return Err(Error::custom("Username cannot be empty"));
//!     }
//!
//!     if username.len() < 3 {
//!         return Err(Error::handler(
//!             format!("Username '{}' is too short (minimum 3 characters)", username)
//!         ));
//!     }
//!
//!     Ok(())
//! }
//! ```

use std::fmt;
use thiserror::Error;

/// The main error type for WsForge operations.
///
/// This enum represents all possible errors that can occur in the WsForge framework.
/// It uses the [`thiserror`](https://docs.rs/thiserror) crate to automatically
/// implement `std::error::Error` and provide good error messages.
///
/// # Variants
///
/// Each variant represents a specific category of error:
///
/// - [`WebSocket`](Error::WebSocket): WebSocket protocol errors from tungstenite
/// - [`Io`](Error::Io): I/O errors from file operations or network
/// - [`Json`](Error::Json): JSON serialization/deserialization errors
/// - [`ConnectionNotFound`](Error::ConnectionNotFound): Connection lookup failures
/// - [`RouteNotFound`](Error::RouteNotFound): Message routing failures
/// - [`InvalidMessage`](Error::InvalidMessage): Malformed message format
/// - [`Handler`](Error::Handler): Handler execution errors
/// - [`Extractor`](Error::Extractor): Type extraction errors
/// - [`Custom`](Error::Custom): Application-defined errors
///
/// # Examples
///
/// ## Handling Specific Error Types
///
/// ```
/// use wsforge::prelude::*;
///
/// # async fn example() {
/// # let result: Result<()> = Ok(());
/// match result {
///     Ok(_) => println!("Success"),
///     Err(Error::ConnectionNotFound(id)) => {
///         println!("Connection {} not found - may have disconnected", id);
///     }
///     Err(Error::Json(e)) => {
///         println!("Failed to parse JSON: {}", e);
///     }
///     Err(e) => {
///         println!("Unexpected error: {}", e);
///     }
/// }
/// # }
/// ```
///
/// ## Error Propagation with `?`
///
/// ```
/// use wsforge::prelude::*;
///
/// async fn process_message(msg: Message) -> Result<String> {
///     // JSON errors are automatically converted to Error::Json
///     let data: serde_json::Value = msg.json()?;
///
///     // Custom validation
///     let username = data["username"]
///         .as_str()
///         .ok_or_else(|| Error::custom("Missing username field"))?;
///
///     Ok(format!("Hello, {}", username))
/// }
/// ```
#[derive(Debug, Error)]
pub enum Error {
    /// WebSocket protocol error.
    ///
    /// This variant wraps errors from the `tokio-tungstenite` crate,
    /// which include protocol violations, connection issues, and
    /// framing errors.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example(err: tokio_tungstenite::tungstenite::Error) {
    /// let error = Error::from(err);
    /// match error {
    ///     Error::WebSocket(e) => println!("WebSocket error: {}", e),
    ///     _ => {}
    /// }
    /// # }
    /// ```
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    /// I/O error.
    ///
    /// This variant wraps standard I/O errors that can occur during
    /// file operations, network operations, or other system-level
    /// operations.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use std::io;
    ///
    /// # fn example(io_err: io::Error) {
    /// let error = Error::from(io_err);
    /// match error {
    ///     Error::Io(e) => println!("I/O error: {}", e),
    ///     _ => {}
    /// }
    /// # }
    /// ```
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization or deserialization error.
    ///
    /// This variant wraps errors from `serde_json`, which can occur when:
    /// - Parsing invalid JSON
    /// - Serializing data with unsupported types
    /// - Missing required fields during deserialization
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct User {
    ///     id: u32,
    ///     name: String,
    /// }
    ///
    /// async fn parse_user(msg: Message) -> Result<User> {
    ///     // If JSON is invalid or missing fields, Error::Json is returned
    ///     let user: User = msg.json()?;
    ///     Ok(user)
    /// }
    /// ```
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Connection not found error.
    ///
    /// This error occurs when attempting to send a message to a connection
    /// that no longer exists (usually because the client disconnected).
    ///
    /// The variant contains the ID of the connection that couldn't be found.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// async fn send_notification(
    ///     manager: &ConnectionManager,
    ///     user_id: &str,
    ///     msg: &str,
    /// ) -> Result<()> {
    ///     let conn = manager
    ///         .get(&user_id.to_string())
    ///         .ok_or_else(|| Error::ConnectionNotFound(user_id.to_string()))?;
    ///
    ///     conn.send_text(msg)?;
    ///     Ok(())
    /// }
    /// ```
    #[error("Connection not found: {0}")]
    ConnectionNotFound(String),

    /// Route not found error.
    ///
    /// This error occurs when a message is sent to a route that doesn't
    /// have a registered handler.
    ///
    /// The variant contains the route path that couldn't be found.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example() {
    /// let route = "/unknown/path";
    /// let error = Error::RouteNotFound(route.to_string());
    /// println!("{}", error); // "Route not found: /unknown/path"
    /// # }
    /// ```
    #[error("Route not found: {0}")]
    RouteNotFound(String),

    /// Invalid message format error.
    ///
    /// This error occurs when a message has an unexpected format or
    /// structure that the handler cannot process.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// async fn validate_message(msg: &Message) -> Result<()> {
    ///     if msg.is_binary() {
    ///         return Err(Error::InvalidMessage);
    ///     }
    ///
    ///     let text = msg.as_text()
    ///         .ok_or(Error::InvalidMessage)?;
    ///
    ///     if text.is_empty() {
    ///         return Err(Error::InvalidMessage);
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    #[error("Invalid message format")]
    InvalidMessage,

    /// Handler execution error.
    ///
    /// This error occurs when a message handler encounters an error
    /// during execution. The variant contains a description of what
    /// went wrong.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// async fn process_command(cmd: &str) -> Result<()> {
    ///     match cmd {
    ///         "start" => Ok(()),
    ///         "stop" => Ok(()),
    ///         unknown => Err(Error::handler(
    ///             format!("Unknown command: {}", unknown)
    ///         )),
    ///     }
    /// }
    /// ```
    #[error("Handler error: {0}")]
    Handler(String),

    /// Type extractor error.
    ///
    /// This error occurs when a type extractor fails to extract data
    /// from the message or connection context. Common causes include:
    /// - Missing required data in state
    /// - Type mismatches
    /// - Missing path or query parameters
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// async fn get_user_state(state: &AppState) -> Result<String> {
    ///     state
    ///         .get::<String>()
    ///         .ok_or_else(|| Error::extractor("User state not found"))
    ///         .map(|arc| (*arc).clone())
    /// }
    /// ```
    #[error("Extractor error: {0}")]
    Extractor(String),

    /// Custom application-defined error.
    ///
    /// This variant allows applications to create custom errors with
    /// arbitrary messages. Use this for application-specific error
    /// conditions that don't fit other categories.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// async fn check_rate_limit(user_id: &str) -> Result<()> {
    ///     let request_count = get_request_count(user_id);
    ///
    ///     if request_count > 100 {
    ///         return Err(Error::custom(
    ///             format!("Rate limit exceeded for user {}", user_id)
    ///         ));
    ///     }
    ///
    ///     Ok(())
    /// }
    ///
    /// # fn get_request_count(_: &str) -> u32 { 0 }
    /// ```
    #[error("Custom error: {0}")]
    Custom(String),
}

/// A type alias for `Result<T, Error>`.
///
/// This alias is provided for convenience and consistency across the codebase.
/// Most functions in WsForge return this type.
///
/// # Examples
///
/// ```
/// use wsforge::prelude::*;
///
/// async fn my_handler(msg: Message) -> Result<String> {
///     Ok("Success".to_string())
/// }
/// ```
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Creates a custom error with the given message.
    ///
    /// This is a convenience method for creating [`Error::Custom`] variants.
    /// Use this for application-specific errors that don't fit into other
    /// error categories.
    ///
    /// # Arguments
    ///
    /// * `msg` - Any type that implements `Display` (like `&str`, `String`, etc.)
    ///
    /// # Examples
    ///
    /// ## With String Literals
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example() -> Result<()> {
    /// if some_condition() {
    ///     return Err(Error::custom("Something went wrong"));
    /// }
    /// Ok(())
    /// # }
    /// # fn some_condition() -> bool { false }
    /// ```
    ///
    /// ## With Formatted Strings
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example(user_id: u32, max_age: u32) -> Result<()> {
    /// let age = get_user_age(user_id);
    /// if age > max_age {
    ///     return Err(Error::custom(
    ///         format!("User {} age {} exceeds maximum {}", user_id, age, max_age)
    ///     ));
    /// }
    /// Ok(())
    /// # }
    /// # fn get_user_age(_: u32) -> u32 { 25 }
    /// ```
    pub fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Custom(msg.to_string())
    }

    /// Creates a handler error with the given message.
    ///
    /// This is a convenience method for creating [`Error::Handler`] variants.
    /// Use this for errors that occur during handler execution.
    ///
    /// # Arguments
    ///
    /// * `msg` - Any type that implements `Display`
    ///
    /// # Examples
    ///
    /// ## Validation Errors
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// async fn validate_input(data: &str) -> Result<()> {
    ///     if data.is_empty() {
    ///         return Err(Error::handler("Input cannot be empty"));
    ///     }
    ///
    ///     if data.len() > 1000 {
    ///         return Err(Error::handler(
    ///             format!("Input too long: {} characters (max 1000)", data.len())
    ///         ));
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// ## Command Processing Errors
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// async fn execute_command(cmd: &str, args: &[&str]) -> Result<String> {
    ///     match cmd {
    ///         "echo" if args.is_empty() => {
    ///             Err(Error::handler("echo command requires arguments"))
    ///         }
    ///         "echo" => Ok(args.join(" ")),
    ///         unknown => Err(Error::handler(
    ///             format!("Unknown command: {}", unknown)
    ///         )),
    ///     }
    /// }
    /// ```
    pub fn handler<T: fmt::Display>(msg: T) -> Self {
        Error::Handler(msg.to_string())
    }

    /// Creates an extractor error with the given message.
    ///
    /// This is a convenience method for creating [`Error::Extractor`] variants.
    /// Use this when type extraction fails, such as when required data is
    /// missing from the request context.
    ///
    /// # Arguments
    ///
    /// * `msg` - Any type that implements `Display`
    ///
    /// # Examples
    ///
    /// ## Missing State Data
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use std::sync::Arc;
    ///
    /// struct DatabasePool;
    ///
    /// async fn get_database(state: &AppState) -> Result<Arc<DatabasePool>> {
    ///     state
    ///         .get::<DatabasePool>()
    ///         .ok_or_else(|| Error::extractor("Database pool not found in state"))
    /// }
    /// ```
    ///
    /// ## Missing Path Parameters
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// async fn extract_user_id(extensions: &Extensions) -> Result<u32> {
    ///     extensions
    ///         .get::<u32>("user_id")
    ///         .map(|arc| *arc)
    ///         .ok_or_else(|| Error::extractor("User ID not found in path"))
    /// }
    /// ```
    ///
    /// ## Type Mismatch
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// async fn parse_count(value: &str) -> Result<usize> {
    ///     value.parse::<usize>().map_err(|_| {
    ///         Error::extractor(format!(
    ///             "Failed to parse '{}' as count (expected positive integer)",
    ///             value
    ///         ))
    ///     })
    /// }
    /// ```
    pub fn extractor<T: fmt::Display>(msg: T) -> Self {
        Error::Extractor(msg.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_error() {
        let err = Error::custom("test error");
        assert!(matches!(err, Error::Custom(_)));
        assert_eq!(err.to_string(), "Custom error: test error");
    }

    #[test]
    fn test_handler_error() {
        let err = Error::handler("handler failed");
        assert!(matches!(err, Error::Handler(_)));
        assert_eq!(err.to_string(), "Handler error: handler failed");
    }

    #[test]
    fn test_extractor_error() {
        let err = Error::extractor("missing field");
        assert!(matches!(err, Error::Extractor(_)));
        assert_eq!(err.to_string(), "Extractor error: missing field");
    }

    #[test]
    fn test_connection_not_found() {
        let err = Error::ConnectionNotFound("conn_123".to_string());
        assert_eq!(err.to_string(), "Connection not found: conn_123");
    }

    #[test]
    fn test_route_not_found() {
        let err = Error::RouteNotFound("/api/users".to_string());
        assert_eq!(err.to_string(), "Route not found: /api/users");
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = Error::from(io_err);
        assert!(matches!(err, Error::Io(_)));
    }

    #[test]
    fn test_error_from_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let err = Error::from(json_err);
        assert!(matches!(err, Error::Json(_)));
    }

    #[test]
    fn test_result_type_alias() {
        fn returns_result() -> Result<String> {
            Ok("success".to_string())
        }

        assert!(returns_result().is_ok());
    }

    #[test]
    fn test_error_display_formatting() {
        let errors = vec![
            Error::custom("custom message"),
            Error::handler("handler message"),
            Error::extractor("extractor message"),
            Error::InvalidMessage,
        ];

        for err in errors {
            let display = format!("{}", err);
            assert!(!display.is_empty());
        }
    }
}
