//! WebSocket message types and utilities.
//!
//! This module provides the core message abstraction used throughout WsForge.
//! It wraps the underlying WebSocket message types from `tokio-tungstenite` and
//! provides convenient methods for creating, inspecting, and parsing messages.
//!
//! # Overview
//!
//! WebSocket messages can be one of several types:
//! - **Text**: UTF-8 encoded string data
//! - **Binary**: Raw byte data
//! - **Ping/Pong**: Control frames for keep-alive
//! - **Close**: Connection termination frames
//!
//! The [`Message`] type provides a unified interface for working with all these types,
//! with automatic conversion between WsForge's message type and the underlying
//! `tungstenite` message type.
//!
//! # Message Types
//!
//! | Type | Description | Use Case |
//! |------|-------------|----------|
//! | [`MessageType::Text`] | UTF-8 text | JSON, commands, chat messages |
//! | [`MessageType::Binary`] | Raw bytes | Images, files, protocol buffers |
//! | [`MessageType::Ping`] | Keep-alive request | Connection health checks |
//! | [`MessageType::Pong`] | Keep-alive response | Responding to pings |
//! | [`MessageType::Close`] | Connection close | Graceful disconnection |
//!
//! # Examples
//!
//! ## Creating Messages
//!
//! ```
//! use wsforge::prelude::*;
//!
//! // Text message
//! let text_msg = Message::text("Hello, WebSocket!");
//!
//! // Binary message
//! let binary_msg = Message::binary(vec![0x01, 0x02, 0x03]);
//!
//! // JSON message (text)
//! let json_msg = Message::text(r#"{"type":"greeting","text":"hello"}"#);
//! ```
//!
//! ## Inspecting Messages
//!
//! ```
//! use wsforge::prelude::*;
//!
//! # fn example(msg: Message) {
//! if msg.is_text() {
//!     println!("Text: {}", msg.as_text().unwrap());
//! } else if msg.is_binary() {
//!     println!("Binary: {} bytes", msg.as_bytes().len());
//! }
//!
//! // Get the message type
//! match msg.message_type() {
//!     MessageType::Text => println!("Received text"),
//!     MessageType::Binary => println!("Received binary"),
//!     _ => println!("Received control frame"),
//! }
//! # }
//! ```
//!
//! ## Parsing JSON
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
//! # fn example(msg: Message) -> Result<()> {
//! // Parse JSON from message
//! let chat: ChatMessage = msg.json()?;
//! println!("{} says: {}", chat.username, chat.text);
//! # Ok(())
//! # }
//! ```

use crate::error::Result;
use serde::de::DeserializeOwned;
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;

/// Represents the type of a WebSocket message.
///
/// This enum categorizes messages into their protocol-defined types.
/// Most application logic will work with text and binary messages,
/// while control frames (Ping, Pong, Close) are typically handled
/// by the framework automatically.
///
/// # Examples
///
/// ```
/// use wsforge::prelude::*;
///
/// # fn example(msg: Message) {
/// match msg.message_type() {
///     MessageType::Text => {
///         println!("Processing text message");
///     }
///     MessageType::Binary => {
///         println!("Processing binary data");
///     }
///     MessageType::Ping => {
///         println!("Received ping (will auto-respond with pong)");
///     }
///     MessageType::Pong => {
///         println!("Received pong");
///     }
///     MessageType::Close => {
///         println!("Client disconnecting");
///     }
/// }
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    /// Text message containing UTF-8 encoded string data.
    ///
    /// This is the most common message type for:
    /// - JSON data
    /// - Plain text chat messages
    /// - Commands and instructions
    /// - XML or other text-based formats
    Text,

    /// Binary message containing raw byte data.
    ///
    /// Use this for:
    /// - Images, audio, video files
    /// - Protocol buffers, MessagePack
    /// - Custom binary protocols
    /// - Large data transfers
    Binary,

    /// Ping frame for connection keep-alive.
    ///
    /// Servers send pings to check if the connection is still active.
    /// Clients should respond with a Pong frame (handled automatically).
    Ping,

    /// Pong frame responding to a Ping.
    ///
    /// This is automatically sent in response to Ping frames.
    /// You rarely need to create these manually.
    Pong,

    /// Close frame indicating connection termination.
    ///
    /// Sent when either side wants to close the connection gracefully.
    /// Contains optional close code and reason.
    Close,
}

/// A WebSocket message.
///
/// This is the main type for working with WebSocket messages in WsForge.
/// It wraps the raw message data and provides convenient methods for
/// creating, inspecting, and converting messages.
///
/// # Thread Safety
///
/// `Message` is cheaply cloneable and can be safely shared across threads.
///
/// # Examples
///
/// ## Creating Messages
///
/// ```
/// use wsforge::prelude::*;
///
/// // Create text message
/// let greeting = Message::text("Hello!");
///
/// // Create binary message
/// let data = Message::binary(vec!);[1][2][3][4]
/// ```
///
/// ## Working with Content
///
/// ```
/// use wsforge::prelude::*;
///
/// # fn example(msg: Message) {
/// // Check message type
/// if msg.is_text() {
///     // Get text content
///     if let Some(text) = msg.as_text() {
///         println!("Message: {}", text);
///     }
/// }
///
/// // Get raw bytes (works for both text and binary)
/// let bytes = msg.as_bytes();
/// println!("Size: {} bytes", bytes.len());
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Message {
    /// The raw message data as bytes.
    ///
    /// For text messages, this contains UTF-8 encoded text.
    /// For binary messages, this contains raw bytes.
    pub data: Vec<u8>,

    /// The type of this message.
    pub msg_type: MessageType,
}

impl Message {
    /// Creates a new text message.
    ///
    /// The string is converted to UTF-8 bytes and stored as a text message.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content (any type convertible to `String`)
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// // From &str
    /// let msg1 = Message::text("Hello");
    ///
    /// // From String
    /// let msg2 = Message::text(String::from("World"));
    ///
    /// // From format!
    /// let msg3 = Message::text(format!("User {} joined", 42));
    /// ```
    pub fn text(text: impl Into<String>) -> Self {
        let string = text.into();
        Self {
            data: string.into_bytes(),
            msg_type: MessageType::Text,
        }
    }

    /// Creates a new binary message.
    ///
    /// The bytes are stored as-is without any encoding or processing.
    ///
    /// # Arguments
    ///
    /// * `data` - The binary data
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// // From Vec<u8>
    /// let msg1 = Message::binary(vec![0x01, 0x02, 0x03]);
    ///
    /// // From byte slice
    /// let bytes: &[u8] = &[0xFF, 0xFE, 0xFD];
    /// let msg2 = Message::binary(bytes.to_vec());
    ///
    /// // From array
    /// let msg3 = Message::binary(.to_vec());[2][3][4][1]
    /// ```
    pub fn binary(data: Vec<u8>) -> Self {
        Self {
            data,
            msg_type: MessageType::Binary,
        }
    }

    /// Creates a ping message.
    ///
    /// Ping messages are used for connection keep-alive. The client
    /// should respond with a pong message (handled automatically by
    /// most WebSocket implementations).
    ///
    /// # Arguments
    ///
    /// * `data` - Optional payload data (usually empty)
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// // Simple ping
    /// let ping = Message::ping(vec![]);
    ///
    /// // Ping with payload
    /// let ping_with_data = Message::ping(b"timestamp:1234567890".to_vec());
    /// ```
    pub fn ping(data: Vec<u8>) -> Self {
        Self {
            data,
            msg_type: MessageType::Ping,
        }
    }

    /// Creates a pong message.
    ///
    /// Pong messages are sent in response to ping messages. This is
    /// typically handled automatically by the framework.
    ///
    /// # Arguments
    ///
    /// * `data` - Payload data (should match the ping payload)
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example(ping_msg: Message) {
    /// // Respond to a ping
    /// if ping_msg.is_ping() {
    ///     let pong = Message::pong(ping_msg.data.clone());
    ///     // Send pong back...
    /// }
    /// # }
    /// ```
    pub fn pong(data: Vec<u8>) -> Self {
        Self {
            data,
            msg_type: MessageType::Pong,
        }
    }

    /// Creates a close message.
    ///
    /// Close messages signal graceful connection termination.
    /// The connection will close after this message is sent/received.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// // Simple close
    /// let close = Message::close();
    /// ```
    pub fn close() -> Self {
        Self {
            data: Vec::new(),
            msg_type: MessageType::Close,
        }
    }

    /// Converts this message to a `tungstenite` message.
    ///
    /// This is used internally by the framework to convert between
    /// WsForge's message type and the underlying WebSocket library.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example() {
    /// let msg = Message::text("Hello");
    /// let tungstenite_msg = msg.into_tungstenite();
    /// # }
    /// ```
    pub fn into_tungstenite(self) -> TungsteniteMessage {
        match self.msg_type {
            MessageType::Text => {
                TungsteniteMessage::Text(String::from_utf8_lossy(&self.data).to_string())
            }
            MessageType::Binary => TungsteniteMessage::Binary(self.data),
            MessageType::Ping => TungsteniteMessage::Ping(self.data),
            MessageType::Pong => TungsteniteMessage::Pong(self.data),
            MessageType::Close => TungsteniteMessage::Close(None),
        }
    }

    /// Creates a message from a `tungstenite` message.
    ///
    /// This is used internally by the framework to convert incoming
    /// WebSocket messages to WsForge's message type.
    ///
    /// # Arguments
    ///
    /// * `msg` - A tungstenite WebSocket message
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
    ///
    /// # fn example() {
    /// let tung_msg = TungsteniteMessage::Text("Hello".to_string());
    /// let msg = Message::from_tungstenite(tung_msg);
    /// assert!(msg.is_text());
    /// # }
    /// ```
    pub fn from_tungstenite(msg: TungsteniteMessage) -> Self {
        match msg {
            TungsteniteMessage::Text(text) => Self::text(text),
            TungsteniteMessage::Binary(data) => Self::binary(data),
            TungsteniteMessage::Ping(data) => Self::ping(data),
            TungsteniteMessage::Pong(data) => Self::pong(data),
            TungsteniteMessage::Close(_) => Self::close(),
            TungsteniteMessage::Frame(_) => Self::binary(vec![]),
        }
    }

    /// Returns the type of this message.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example() {
    /// let msg = Message::text("Hello");
    /// assert_eq!(msg.message_type(), MessageType::Text);
    ///
    /// let binary = Message::binary(vec!);[3][1][2]
    /// assert_eq!(binary.message_type(), MessageType::Binary);
    /// # }
    /// ```
    pub fn message_type(&self) -> MessageType {
        self.msg_type
    }

    /// Checks if this is a text message.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example(msg: Message) {
    /// if msg.is_text() {
    ///     println!("Processing text: {}", msg.as_text().unwrap());
    /// }
    /// # }
    /// ```
    pub fn is_text(&self) -> bool {
        self.msg_type == MessageType::Text
    }

    /// Checks if this is a binary message.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example(msg: Message) {
    /// if msg.is_binary() {
    ///     let bytes = msg.as_bytes();
    ///     println!("Processing {} bytes of binary data", bytes.len());
    /// }
    /// # }
    /// ```
    pub fn is_binary(&self) -> bool {
        self.msg_type == MessageType::Binary
    }

    /// Checks if this is a ping message.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example(msg: Message) {
    /// if msg.is_ping() {
    ///     // Framework usually auto-responds with pong
    ///     println!("Received ping");
    /// }
    /// # }
    /// ```
    pub fn is_ping(&self) -> bool {
        self.msg_type == MessageType::Ping
    }

    /// Checks if this is a pong message.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example(msg: Message) {
    /// if msg.is_pong() {
    ///     println!("Connection is alive");
    /// }
    /// # }
    /// ```
    pub fn is_pong(&self) -> bool {
        self.msg_type == MessageType::Pong
    }

    /// Checks if this is a close message.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example(msg: Message) {
    /// if msg.is_close() {
    ///     println!("Client is disconnecting");
    /// }
    /// # }
    /// ```
    pub fn is_close(&self) -> bool {
        self.msg_type == MessageType::Close
    }

    /// Returns the message content as a string slice, if it's a text message.
    ///
    /// Returns `None` if the message is not text or if the data is not valid UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example(msg: Message) {
    /// if let Some(text) = msg.as_text() {
    ///     println!("Received: {}", text);
    /// }
    /// # }
    /// ```
    pub fn as_text(&self) -> Option<&str> {
        if self.is_text() {
            std::str::from_utf8(&self.data).ok()
        } else {
            None
        }
    }

    /// Returns the message content as a byte slice.
    ///
    /// This works for all message types, returning the raw underlying data.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example(msg: Message) {
    /// let bytes = msg.as_bytes();
    /// println!("Message size: {} bytes", bytes.len());
    ///
    /// // Works for text messages too
    /// let text_msg = Message::text("Hello");
    /// assert_eq!(text_msg.as_bytes(), b"Hello");
    /// # }
    /// ```
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Deserializes the message content as JSON.
    ///
    /// This is a convenience method for parsing JSON from text messages.
    /// The type must implement `serde::Deserialize`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The message is not text
    /// - The JSON is malformed
    /// - The JSON doesn't match the expected type
    ///
    /// # Examples
    ///
    /// ## Simple Types
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example() -> Result<()> {
    /// let msg = Message::text(r#"{"name":"Alice","age":30}"#);
    ///
    /// let value: serde_json::Value = msg.json()?;
    /// assert_eq!(value["name"], "Alice");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Struct Deserialization
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct User {
    ///     name: String,
    ///     age: u32,
    /// }
    ///
    /// # fn example() -> Result<()> {
    /// let msg = Message::text(r#"{"name":"Alice","age":30}"#);
    /// let user: User = msg.json()?;
    /// assert_eq!(user.name, "Alice");
    /// # Ok(())
    /// # }
    /// ```
    pub fn json<T: DeserializeOwned>(&self) -> Result<T> {
        let text = self
            .as_text()
            .ok_or_else(|| crate::error::Error::InvalidMessage)?;
        Ok(serde_json::from_str(text)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_message() {
        let msg = Message::text("Hello, World!");
        assert!(msg.is_text());
        assert_eq!(msg.as_text(), Some("Hello, World!"));
        assert_eq!(msg.message_type(), MessageType::Text);
    }

    #[test]
    fn test_binary_message() {
        let data = vec![1, 2, 3, 4, 5];
        let msg = Message::binary(data.clone());
        assert!(msg.is_binary());
        assert_eq!(msg.as_bytes(), &data[..]);
        assert_eq!(msg.message_type(), MessageType::Binary);
    }

    #[test]
    fn test_ping_message() {
        let msg = Message::ping(vec![]);
        assert!(msg.is_ping());
        assert_eq!(msg.message_type(), MessageType::Ping);
    }

    #[test]
    fn test_pong_message() {
        let msg = Message::pong(vec![]);
        assert!(msg.is_pong());
        assert_eq!(msg.message_type(), MessageType::Pong);
    }

    #[test]
    fn test_close_message() {
        let msg = Message::close();
        assert!(msg.is_close());
        assert_eq!(msg.message_type(), MessageType::Close);
    }

    #[test]
    fn test_json_parsing() {
        let msg = Message::text(r#"{"key":"value","number":42}"#);
        let json: serde_json::Value = msg.json().unwrap();
        assert_eq!(json["key"], "value");
        assert_eq!(json["number"], 42);
    }

    #[test]
    fn test_as_bytes() {
        let text_msg = Message::text("Hello");
        assert_eq!(text_msg.as_bytes(), b"Hello");

        let binary_msg = Message::binary(vec![1, 2, 3]);
        assert_eq!(binary_msg.as_bytes(), &[1, 2, 3]);
    }

    #[test]
    fn test_tungstenite_conversion() {
        let msg = Message::text("test");
        let tung_msg = msg.clone().into_tungstenite();
        let back = Message::from_tungstenite(tung_msg);
        assert_eq!(back.as_text(), msg.as_text());
    }
}
