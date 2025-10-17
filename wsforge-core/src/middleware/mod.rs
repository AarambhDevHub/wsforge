//! Middleware system for request/response processing.
//!
//! This module provides a flexible middleware chain system that allows you to intercept
//! and process WebSocket messages before they reach handlers. Middleware can modify messages,
//! perform authentication, logging, rate limiting, and more.
//!
//! # Overview
//!
//! The middleware system is built around three core types:
//! - [`Middleware`] - Trait that all middleware must implement
//! - [`MiddlewareChain`] - Container that holds and executes middleware in order
//! - [`Next`] - Represents the next step in the middleware chain
//!
//! # Architecture
//!
//! ```
//! Message → Middleware 1 → Middleware 2 → ... → Handler → Response
//!              ↓              ↓                      ↓
//!           Next::run     Next::run            Handler::call
//! ```
//!
//! Each middleware can:
//! - Inspect the incoming message
//! - Modify the message before passing it forward
//! - Short-circuit the chain by not calling `next.run()`
//! - Modify the response after calling `next.run()`
//! - Handle errors and transform responses
//!
//! # Examples
//!
//! ## Using Built-in Logger Middleware
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
//!     .layer(LoggerMiddleware::new())
//!     .default_handler(handler(echo));
//!
//! router.listen("127.0.0.1:8080").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Creating Custom Middleware
//!
//! ```
//! use wsforge::prelude::*;
//! use async_trait::async_trait;
//!
//! struct AuthMiddleware {
//!     secret: String,
//! }
//!
//! #[async_trait]
//! impl Middleware for AuthMiddleware {
//!     async fn handle(
//!         &self,
//!         message: Message,
//!         conn: Connection,
//!         state: AppState,
//!         extensions: Extensions,
//!         mut next: Next,
//!     ) -> Result<Option<Message>> {
//!         // Check for auth token in message
//!         if let Some(text) = message.as_text() {
//!             if !text.contains(&self.secret) {
//!                 return Err(Error::custom("Unauthorized"));
//!             }
//!         }
//!
//!         // Continue to next middleware/handler
//!         next.run(message, conn, state, extensions).await
//!     }
//! }
//! ```
//!
//! ## Function-based Middleware
//!
//! ```
//! use wsforge::prelude::*;
//!
//! # async fn example() {
//! let logging_middleware = from_fn(|msg, conn, state, ext, mut next| async move {
//!     println!("Before handler: {:?}", msg.as_text());
//!     let response = next.run(msg, conn, state, ext).await?;
//!     println!("After handler");
//!     Ok(response)
//! });
//!
//! // Use in router
//! // router.layer(logging_middleware);
//! # }
//! ```
//!
//! ## Chaining Multiple Middleware
//!
//! ```
//! use wsforge::prelude::*;
//!
//! # async fn example() -> Result<()> {
//! let router = Router::new()
//!     .layer(LoggerMiddleware::new())
//!     .layer(auth_middleware())
//!     .layer(rate_limit_middleware())
//!     .default_handler(handler(my_handler));
//! # Ok(())
//! # }
//! # async fn my_handler() -> Result<String> { Ok("".to_string()) }
//! # fn auth_middleware() -> Arc<dyn Middleware> { unimplemented!() }
//! # fn rate_limit_middleware() -> Arc<dyn Middleware> { unimplemented!() }
//! ```

pub mod logger;

pub use logger::LoggerMiddleware;

use crate::connection::Connection;
use crate::error::Result;
use crate::extractor::Extensions;
use crate::message::Message;
use crate::state::AppState;
use async_trait::async_trait;
use std::sync::Arc;

/// Represents the next middleware or handler in the chain.
///
/// `Next` is used to pass control to the next step in the middleware pipeline.
/// When a middleware calls `next.run()`, it invokes the next middleware or,
/// if there are no more middleware, the final handler.
///
/// # Examples
///
/// ```
/// use wsforge::prelude::*;
/// use async_trait::async_trait;
///
/// struct MyMiddleware;
///
/// #[async_trait]
/// impl Middleware for MyMiddleware {
///     async fn handle(
///         &self,
///         message: Message,
///         conn: Connection,
///         state: AppState,
///         extensions: Extensions,
///         mut next: Next,
///     ) -> Result<Option<Message>> {
///         println!("Before next");
///
///         // Call the next middleware/handler
///         let response = next.run(message, conn, state, extensions).await?;
///
///         println!("After next");
///         Ok(response)
///     }
/// }
/// ```
pub struct Next {
    chain: Arc<MiddlewareChain>,
    index: usize,
}

impl Next {
    /// Creates a new `Next` instance.
    ///
    /// # Arguments
    ///
    /// * `chain` - The middleware chain to execute
    /// * `index` - Current position in the chain
    pub fn new(chain: Arc<MiddlewareChain>, index: usize) -> Self {
        Self { chain, index }
    }

    /// Call the next middleware in the chain.
    ///
    /// This method executes the next middleware in the sequence. If all middleware
    /// have been executed, it calls the final handler.
    ///
    /// # Arguments
    ///
    /// * `message` - The WebSocket message being processed
    /// * `conn` - The connection that sent the message
    /// * `state` - Application state
    /// * `extensions` - Request-scoped extension data
    ///
    /// # Returns
    ///
    /// Returns the response from the next middleware or handler, or `None` if
    /// no response should be sent.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use async_trait::async_trait;
    ///
    /// struct TimingMiddleware;
    ///
    /// #[async_trait]
    /// impl Middleware for TimingMiddleware {
    ///     async fn handle(
    ///         &self,
    ///         message: Message,
    ///         conn: Connection,
    ///         state: AppState,
    ///         extensions: Extensions,
    ///         mut next: Next,
    ///     ) -> Result<Option<Message>> {
    ///         let start = std::time::Instant::now();
    ///
    ///         let response = next.run(message, conn, state, extensions).await?;
    ///
    ///         let duration = start.elapsed();
    ///         println!("Request took: {:?}", duration);
    ///
    ///         Ok(response)
    ///     }
    /// }
    /// ```
    pub async fn run(
        mut self,
        message: Message,
        conn: Connection,
        state: AppState,
        extensions: Extensions,
    ) -> Result<Option<Message>> {
        if self.index < self.chain.middlewares.len() {
            let middleware = self.chain.middlewares[self.index].clone();
            self.index += 1;
            middleware
                .handle(message, conn, state, extensions, self)
                .await
        } else if let Some(ref handler) = self.chain.handler {
            handler.call(message, conn, state, extensions).await
        } else {
            Ok(None)
        }
    }
}

/// Middleware trait that all middleware must implement.
///
/// Middleware can intercept messages before they reach handlers, perform
/// transformations, add metadata to extensions, or short-circuit the request.
///
/// # Implementation Guidelines
///
/// - **Always call `next.run()`** unless you want to short-circuit
/// - **Use extensions** to pass data to handlers or other middleware
/// - **Handle errors gracefully** and provide clear error messages
/// - **Be mindful of performance** - middleware runs on every message
///
/// # Examples
///
/// ## Authentication Middleware
///
/// ```
/// use wsforge::prelude::*;
/// use async_trait::async_trait;
///
/// struct AuthMiddleware {
///     required_token: String,
/// }
///
/// #[async_trait]
/// impl Middleware for AuthMiddleware {
///     async fn handle(
///         &self,
///         message: Message,
///         conn: Connection,
///         state: AppState,
///         extensions: Extensions,
///         mut next: Next,
///     ) -> Result<Option<Message>> {
///         if let Some(text) = message.as_text() {
///             if let Some(token) = text.strip_prefix("TOKEN:") {
///                 if token == self.required_token {
///                     extensions.insert("authenticated", true);
///                     return next.run(message, conn, state, extensions).await;
///                 }
///             }
///         }
///
///         Err(Error::custom("Unauthorized"))
///     }
/// }
/// ```
///
/// ## Rate Limiting Middleware
///
/// ```
/// use wsforge::prelude::*;
/// use async_trait::async_trait;
/// use std::sync::Arc;
/// use tokio::sync::RwLock;
/// use std::collections::HashMap;
///
/// struct RateLimitMiddleware {
///     limits: Arc<RwLock<HashMap<String, u32>>>,
///     max_requests: u32,
/// }
///
/// #[async_trait]
/// impl Middleware for RateLimitMiddleware {
///     async fn handle(
///         &self,
///         message: Message,
///         conn: Connection,
///         state: AppState,
///         extensions: Extensions,
///         mut next: Next,
///     ) -> Result<Option<Message>> {
///         let conn_id = conn.id();
///         let mut limits = self.limits.write().await;
///         let count = limits.entry(conn_id.clone()).or_insert(0);
///
///         if *count >= self.max_requests {
///             return Err(Error::custom("Rate limit exceeded"));
///         }
///
///         *count += 1;
///         drop(limits);
///
///         next.run(message, conn, state, extensions).await
///     }
/// }
/// ```
///
/// ## Request ID Middleware
///
/// ```
/// use wsforge::prelude::*;
/// use async_trait::async_trait;
///
/// struct RequestIdMiddleware;
///
/// #[async_trait]
/// impl Middleware for RequestIdMiddleware {
///     async fn handle(
///         &self,
///         message: Message,
///         conn: Connection,
///         state: AppState,
///         extensions: Extensions,
///         mut next: Next,
///     ) -> Result<Option<Message>> {
///         use std::sync::atomic::{AtomicU64, Ordering};
///         static COUNTER: AtomicU64 = AtomicU64::new(0);
///
///         let request_id = COUNTER.fetch_add(1, Ordering::SeqCst);
///         extensions.insert("request_id", request_id);
///
///         next.run(message, conn, state, extensions).await
///     }
/// }
/// ```
#[async_trait]
pub trait Middleware: Send + Sync + 'static {
    /// Handle a message and optionally pass it to the next middleware.
    ///
    /// # Arguments
    ///
    /// * `message` - The incoming WebSocket message
    /// * `conn` - The connection that sent the message
    /// * `state` - Application state
    /// * `extensions` - Request-scoped extension data
    /// * `next` - The next step in the middleware chain
    ///
    /// # Returns
    ///
    /// Returns an optional message to send back to the client, or an error.
    async fn handle(
        &self,
        message: Message,
        conn: Connection,
        state: AppState,
        extensions: Extensions,
        next: Next,
    ) -> Result<Option<Message>>;
}

/// Middleware chain holds all middlewares and the final handler.
///
/// The chain executes middleware in the order they were added, and finally
/// calls the handler if all middleware pass control forward.
///
/// # Examples
///
/// ```
/// use wsforge::prelude::*;
///
/// # fn example() {
/// let mut chain = MiddlewareChain::new();
///
/// // Add middleware
/// chain.layer(LoggerMiddleware::new());
///
/// // Set final handler
/// chain.handler(handler(my_handler));
/// # }
/// # async fn my_handler() -> Result<String> { Ok("".to_string()) }
/// ```
#[derive(Clone)]
pub struct MiddlewareChain {
    /// All middleware in the chain, executed in order
    pub middlewares: Vec<Arc<dyn Middleware>>,
    /// The final handler to call after all middleware
    pub handler: Option<Arc<dyn crate::handler::Handler>>,
}

impl MiddlewareChain {
    /// Creates a new empty middleware chain.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// let chain = MiddlewareChain::new();
    /// ```
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
            handler: None,
        }
    }

    /// Add a middleware to the chain.
    ///
    /// Middleware are executed in the order they are added.
    ///
    /// # Arguments
    ///
    /// * `middleware` - The middleware to add
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example() {
    /// let mut chain = MiddlewareChain::new();
    ///
    /// chain.layer(LoggerMiddleware::new());
    /// # }
    /// ```
    pub fn layer(mut self, middleware: Arc<dyn Middleware>) -> Self {
        self.middlewares.push(middleware);
        self
    }

    /// Set the final handler for the chain.
    ///
    /// The handler is called after all middleware have been executed.
    ///
    /// # Arguments
    ///
    /// * `handler` - The handler to call
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// async fn my_handler(msg: Message) -> Result<String> {
    ///     Ok("response".to_string())
    /// }
    ///
    /// # fn example() {
    /// let mut chain = MiddlewareChain::new();
    /// chain.handler(handler(my_handler));
    /// # }
    /// ```
    pub fn handler(mut self, handler: Arc<dyn crate::handler::Handler>) -> Self {
        self.handler = Some(handler);
        self
    }

    /// Execute the middleware chain.
    ///
    /// This runs all middleware in order, then calls the handler if present.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to process
    /// * `conn` - The connection
    /// * `state` - Application state
    /// * `extensions` - Extension data
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # async fn example(chain: MiddlewareChain, msg: Message, conn: Connection) -> Result<()> {
    /// let state = AppState::new();
    /// let extensions = Extensions::new();
    ///
    /// let response = chain.execute(msg, conn, state, extensions).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute(
        &self,
        message: Message,
        conn: Connection,
        state: AppState,
        extensions: Extensions,
    ) -> Result<Option<Message>> {
        let next = Next::new(Arc::new(self.clone()), 0);
        next.run(message, conn, state, extensions).await
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Function-based Middleware
///
/// Helper to create middleware from async functions without implementing
/// the full `Middleware` trait.
pub struct FnMiddleware<F> {
    func: F,
}

impl<F> FnMiddleware<F> {
    /// Creates a new function-based middleware.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// # fn example() {
    /// let middleware = FnMiddleware::new(|msg, conn, state, ext, mut next| async move {
    ///     println!("Before handler");
    ///     let response = next.run(msg, conn, state, ext).await?;
    ///     println!("After handler");
    ///     Ok(response)
    /// });
    /// # }
    /// ```
    pub fn new(func: F) -> Arc<Self> {
        Arc::new(Self { func })
    }
}

#[async_trait]
impl<F, Fut> Middleware for FnMiddleware<F>
where
    F: Fn(Message, Connection, AppState, Extensions, Next) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<Option<Message>>> + Send + 'static,
{
    async fn handle(
        &self,
        message: Message,
        conn: Connection,
        state: AppState,
        extensions: Extensions,
        next: Next,
    ) -> Result<Option<Message>> {
        (self.func)(message, conn, state, extensions, next).await
    }
}

/// Helper function to create middleware from async functions.
///
/// This is a convenience function that wraps an async function in a middleware.
///
/// # Arguments
///
/// * `f` - Async function with signature matching middleware requirements
///
/// # Examples
///
/// ## Simple Logging
///
/// ```
/// use wsforge::prelude::*;
///
/// # fn example() {
/// let logging = from_fn(|msg, conn, state, ext, mut next| async move {
///     println!("Processing message from {}", conn.id());
///     next.run(msg, conn, state, ext).await
/// });
/// # }
/// ```
///
/// ## With State Access
///
/// ```
/// use wsforge::prelude::*;
/// use std::sync::Arc;
///
/// # fn example() {
/// let counter = from_fn(|msg, conn, state, ext, mut next| async move {
///     // Access state
///     if let Some(counter) = state.get::<Arc<std::sync::atomic::AtomicU64>>() {
///         counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
///     }
///     next.run(msg, conn, state, ext).await
/// });
/// # }
/// ```
pub fn from_fn<F, Fut>(f: F) -> Arc<FnMiddleware<F>>
where
    F: Fn(Message, Connection, AppState, Extensions, Next) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<Option<Message>>> + Send + 'static,
{
    FnMiddleware::new(f)
}
