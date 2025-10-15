//! Static file serving for hybrid HTTP/WebSocket servers.
//!
//! This module provides functionality to serve static files (HTML, CSS, JavaScript, images, etc.)
//! alongside WebSocket connections on the same port. This allows you to build complete web
//! applications where the frontend UI and WebSocket backend share a single server.
//!
//! # Overview
//!
//! The static file handler:
//! - Serves files from a specified directory
//! - Automatically detects MIME types
//! - Handles index files (e.g., `index.html` for directory requests)
//! - Prevents path traversal attacks
//! - Supports percent-encoded URLs
//! - Returns proper HTTP responses with status codes
//!
//! # Security
//!
//! The handler includes built-in security features:
//! - **Path traversal prevention**: Attempts to access `../` or similar patterns are blocked
//! - **Canonical path validation**: All paths are canonicalized and checked against the root
//! - **Access control**: Only files within the configured root directory can be served
//!
//! # MIME Type Detection
//!
//! File types are automatically detected based on file extensions:
//!
//! | Extension | MIME Type | Use Case |
//! |-----------|-----------|----------|
//! | `.html` | `text/html` | Web pages |
//! | `.css` | `text/css` | Stylesheets |
//! | `.js` | `application/javascript` | JavaScript |
//! | `.json` | `application/json` | JSON data |
//! | `.png` | `image/png` | PNG images |
//! | `.jpg`, `.jpeg` | `image/jpeg` | JPEG images |
//! | `.svg` | `image/svg+xml` | SVG graphics |
//! | `.wasm` | `application/wasm` | WebAssembly |
//!
//! # Architecture
//!
//! ```
//! ┌─────────────────┐
//! │  HTTP Request   │
//! └────────┬────────┘
//!          │
//!          ├──→ Parse URL path
//!          │
//!          ├──→ Decode percent-encoding
//!          │
//!          ├──→ Validate against root directory
//!          │
//!          ├──→ Check if directory → serve index.html
//!          │
//!          ├──→ Read file contents
//!          │
//!          └──→ Return HTTP response with MIME type
//! ```
//!
//! # Examples
//!
//! ## Basic Static File Serving
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
//!     .serve_static("public")  // Serve files from ./public directory
//!     .default_handler(handler(ws_handler));
//!
//! // Now accessible:
//! // http://localhost:8080/          -> public/index.html
//! // http://localhost:8080/app.js    -> public/app.js
//! // http://localhost:8080/style.css -> public/style.css
//! // ws://localhost:8080             -> WebSocket handler
//!
//! router.listen("127.0.0.1:8080").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Web Chat Application
//!
//! ```
//! use wsforge::prelude::*;
//! use std::sync::Arc;
//!
//! async fn chat_handler(
//!     msg: Message,
//!     State(manager): State<Arc<ConnectionManager>>,
//! ) -> Result<()> {
//!     manager.broadcast(msg);
//!     Ok(())
//! }
//!
//! # async fn example() -> Result<()> {
//! let router = Router::new()
//!     .serve_static("chat-ui")  // Serve chat UI from ./chat-ui
//!     .default_handler(handler(chat_handler));
//!
//! // Directory structure:
//! // chat-ui/
//! //   ├── index.html    <- Main chat page
//! //   ├── app.js        <- WebSocket client logic
//! //   ├── style.css     <- Chat styling
//! //   └── assets/
//! //       └── logo.png  <- Static assets
//!
//! router.listen("127.0.0.1:8080").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Single Page Application
//!
//! ```
//! use wsforge::prelude::*;
//!
//! async fn api_handler(msg: Message) -> Result<Message> {
//!     // Handle API WebSocket messages
//!     Ok(msg)
//! }
//!
//! # async fn example() -> Result<()> {
//! let router = Router::new()
//!     .serve_static("dist")  // Serve built SPA from ./dist
//!     .default_handler(handler(api_handler));
//!
//! // Typical SPA structure:
//! // dist/
//! //   ├── index.html
//! //   ├── bundle.js
//! //   ├── styles.css
//! //   └── assets/
//!
//! router.listen("0.0.0.0:3000").await?;
//! # Ok(())
//! # }
//! ```

use crate::error::{Error, Result};
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tracing::{debug, warn};

/// Handler for serving static files from a directory.
///
/// `StaticFileHandler` provides secure, efficient static file serving with
/// automatic MIME type detection and directory index support.
///
/// # Security Features
///
/// - **Path traversal protection**: Prevents access to files outside the root directory
/// - **Canonicalization**: All paths are resolved to their canonical form
/// - **Access validation**: Only files under the configured root can be accessed
///
/// # Performance
///
/// - Files are read asynchronously using tokio's `AsyncReadExt`
/// - No buffering overhead for small files
/// - Efficient path resolution and validation
///
/// # Examples
///
/// ## Basic Usage
///
/// ```
/// use wsforge::static_files::StaticFileHandler;
/// use std::path::PathBuf;
///
/// # fn example() {
/// let handler = StaticFileHandler::new(PathBuf::from("public"));
///
/// // Serve files from ./public directory
/// // Handler will automatically serve index.html for directories
/// # }
/// ```
///
/// ## Custom Index File
///
/// ```
/// use wsforge::static_files::StaticFileHandler;
/// use std::path::PathBuf;
///
/// # fn example() {
/// let handler = StaticFileHandler::new(PathBuf::from("public"))
///     .with_index("default.html");
///
/// // Now directories will serve default.html instead of index.html
/// # }
/// ```
///
/// ## Serving Files
///
/// ```
/// use wsforge::static_files::StaticFileHandler;
/// use std::path::PathBuf;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let handler = StaticFileHandler::new(PathBuf::from("public"));
///
/// // Serve a specific file
/// let (content, mime_type) = handler.serve("/app.js").await?;
/// println!("Served {} bytes of {}", content.len(), mime_type);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct StaticFileHandler {
    /// The root directory from which files are served
    root: PathBuf,
    /// The default file to serve for directory requests (e.g., "index.html")
    index_file: String,
}

impl StaticFileHandler {
    /// Creates a new static file handler for the given root directory.
    ///
    /// The root directory path should be relative to the current working directory
    /// or an absolute path. Files will only be served from within this directory
    /// and its subdirectories.
    ///
    /// # Arguments
    ///
    /// * `root` - Path to the root directory containing static files
    ///
    /// # Default Configuration
    ///
    /// - Index file: `index.html`
    /// - MIME detection: Automatic based on file extension
    ///
    /// # Examples
    ///
    /// ## Relative Path
    ///
    /// ```
    /// use wsforge::static_files::StaticFileHandler;
    ///
    /// # fn example() {
    /// let handler = StaticFileHandler::new("public");
    /// // Serves files from ./public
    /// # }
    /// ```
    ///
    /// ## Absolute Path
    ///
    /// ```
    /// use wsforge::static_files::StaticFileHandler;
    /// use std::path::PathBuf;
    ///
    /// # fn example() {
    /// let handler = StaticFileHandler::new(PathBuf::from("/var/www/html"));
    /// // Serves files from /var/www/html
    /// # }
    /// ```
    ///
    /// ## With PathBuf
    ///
    /// ```
    /// use wsforge::static_files::StaticFileHandler;
    /// use std::path::PathBuf;
    ///
    /// # fn example() {
    /// let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    /// path.push("static");
    /// let handler = StaticFileHandler::new(path);
    /// # }
    /// ```
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            index_file: "index.html".to_string(),
        }
    }

    /// Sets the name of the index file to serve for directory requests.
    ///
    /// By default, this is `index.html`. Change this if your application uses
    /// a different default file name.
    ///
    /// # Arguments
    ///
    /// * `index` - The name of the index file
    ///
    /// # Examples
    ///
    /// ## Custom Index
    ///
    /// ```
    /// use wsforge::static_files::StaticFileHandler;
    ///
    /// # fn example() {
    /// let handler = StaticFileHandler::new("public")
    ///     .with_index("default.html");
    /// # }
    /// ```
    ///
    /// ## Home Page
    ///
    /// ```
    /// use wsforge::static_files::StaticFileHandler;
    ///
    /// # fn example() {
    /// let handler = StaticFileHandler::new("public")
    ///     .with_index("home.html");
    /// # }
    /// ```
    pub fn with_index(mut self, index: impl Into<String>) -> Self {
        self.index_file = index.into();
        self
    }

    /// Serves a file at the given path.
    ///
    /// This method:
    /// 1. Decodes percent-encoded URLs
    /// 2. Validates the path is within the root directory
    /// 3. Checks if the path is a directory (serves index file if so)
    /// 4. Reads the file contents
    /// 5. Detects and returns the MIME type
    ///
    /// # Arguments
    ///
    /// * `path` - The requested path (e.g., "/app.js", "/images/logo.png")
    ///
    /// # Returns
    ///
    /// Returns a tuple of `(content, mime_type)` where:
    /// - `content` is the raw file bytes
    /// - `mime_type` is the detected MIME type as a string
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The path is invalid or contains illegal characters
    /// - The path escapes the root directory (security violation)
    /// - The file does not exist
    /// - The file cannot be read (permissions, etc.)
    ///
    /// # Security
    ///
    /// This method prevents path traversal attacks by:
    /// - Canonicalizing both the requested path and root path
    /// - Ensuring the canonical file path starts with the canonical root path
    /// - Rejecting any path that would escape the root directory
    ///
    /// # Examples
    ///
    /// ## Basic File Serving
    ///
    /// ```
    /// use wsforge::static_files::StaticFileHandler;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let handler = StaticFileHandler::new("public");
    ///
    /// let (content, mime_type) = handler.serve("/app.js").await?;
    /// assert_eq!(mime_type, "application/javascript");
    /// println!("Served {} bytes", content.len());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Directory Request
    ///
    /// ```
    /// use wsforge::static_files::StaticFileHandler;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let handler = StaticFileHandler::new("public");
    ///
    /// // Request to "/" serves public/index.html
    /// let (content, mime_type) = handler.serve("/").await?;
    /// assert_eq!(mime_type, "text/html");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Error Handling
    ///
    /// ```
    /// use wsforge::static_files::StaticFileHandler;
    ///
    /// # async fn example() {
    /// let handler = StaticFileHandler::new("public");
    ///
    /// match handler.serve("/nonexistent.html").await {
    ///     Ok((content, mime_type)) => {
    ///         println!("Served {}", mime_type);
    ///     }
    ///     Err(e) => {
    ///         eprintln!("File not found: {}", e);
    ///         // Send 404 response
    ///     }
    /// }
    /// # }
    /// ```
    ///
    /// ## Percent-Encoded Paths
    ///
    /// ```
    /// use wsforge::static_files::StaticFileHandler;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let handler = StaticFileHandler::new("public");
    ///
    /// // Handles percent-encoded characters
    /// let (content, _) = handler.serve("/my%20file.html").await?;
    /// // Serves "public/my file.html"
    /// # Ok(())
    /// # }
    /// ```
    pub async fn serve(&self, path: &str) -> Result<(Vec<u8>, String)> {
        let mut file_path = self.root.clone();

        // Remove leading slash and decode percent-encoding
        let clean_path = path.trim_start_matches('/');
        let decoded = percent_encoding::percent_decode_str(clean_path)
            .decode_utf8()
            .map_err(|e| Error::custom(format!("Invalid path encoding: {}", e)))?;

        file_path.push(decoded.as_ref());

        // Security: prevent path traversal
        let canonical = tokio::fs::canonicalize(&file_path)
            .await
            .map_err(|_| Error::custom("File not found"))?;

        let root_canonical = tokio::fs::canonicalize(&self.root)
            .await
            .map_err(|e| Error::custom(format!("Invalid root directory: {}", e)))?;

        if !canonical.starts_with(&root_canonical) {
            warn!("Path traversal attempt: {:?}", path);
            return Err(Error::custom("Access denied"));
        }

        // If it's a directory, serve index.html
        if canonical.is_dir() {
            file_path.push(&self.index_file);
        }

        debug!("Serving file: {:?}", file_path);

        // Read file
        let mut file = File::open(&file_path)
            .await
            .map_err(|_| Error::custom("File not found"))?;

        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .await
            .map_err(|e| Error::custom(format!("Failed to read file: {}", e)))?;

        // Determine MIME type
        let mime_type = mime_guess::from_path(&file_path)
            .first_or_octet_stream()
            .to_string();

        Ok((contents, mime_type))
    }
}

/// Constructs an HTTP response with the given status, content type, and body.
///
/// This is a utility function for creating properly formatted HTTP/1.1 responses.
/// The response includes standard headers and follows HTTP protocol conventions.
///
/// # Arguments
///
/// * `status` - HTTP status code (e.g., 200, 404, 500)
/// * `content_type` - MIME type of the response body
/// * `body` - The response body as bytes
///
/// # Returns
///
/// Returns a complete HTTP response as a byte vector, ready to be written to a socket.
///
/// # Response Format
///
/// ```
/// HTTP/1.1 {status} {status_text}\r\n
/// Content-Type: {content_type}\r\n
/// Content-Length: {body_length}\r\n
/// Connection: close\r\n
/// \r\n
/// {body}
/// ```
///
/// # Status Codes
///
/// Common status codes:
/// - `200 OK`: Successful request
/// - `404 Not Found`: File not found
/// - `500 Internal Server Error`: Server error
///
/// # Examples
///
/// ## Success Response
///
/// ```
/// use wsforge::static_files::http_response;
///
/// # fn example() {
/// let html = b"<html><body>Hello!</body></html>".to_vec();
/// let response = http_response(200, "text/html", html);
///
/// // Response will be:
/// // HTTP/1.1 200 OK
/// // Content-Type: text/html
/// // Content-Length: 32
/// // Connection: close
/// //
/// // <html><body>Hello!</body></html>
/// # }
/// ```
///
/// ## 404 Not Found
///
/// ```
/// use wsforge::static_files::http_response;
///
/// # fn example() {
/// let html = b"<html><body><h1>404 Not Found</h1></body></html>".to_vec();
/// let response = http_response(404, "text/html", html);
/// # }
/// ```
///
/// ## JSON Response
///
/// ```
/// use wsforge::static_files::http_response;
///
/// # fn example() {
/// let json = br#"{"error":"Not found"}"#.to_vec();
/// let response = http_response(404, "application/json", json);
/// # }
/// ```
///
/// ## Binary Response
///
/// ```
/// use wsforge::static_files::http_response;
///
/// # fn example() {
/// let image_data = vec![0x89, 0x50, 0x4E, 0x47]; // PNG header
/// let response = http_response(200, "image/png", image_data);
/// # }
/// ```
///
/// ## Server Error
///
/// ```
/// use wsforge::static_files::http_response;
///
/// # fn example() {
/// let error_html = b"<html><body><h1>500 Internal Server Error</h1></body></html>".to_vec();
/// let response = http_response(500, "text/html", error_html);
/// # }
/// ```
pub fn http_response(status: u16, content_type: &str, body: Vec<u8>) -> Vec<u8> {
    let status_text = match status {
        200 => "OK",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Unknown",
    };

    let response = format!(
        "HTTP/1.1 {} {}\r\n\
         Content-Type: {}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n",
        status,
        status_text,
        content_type,
        body.len()
    );

    let mut result = response.into_bytes();
    result.extend_from_slice(&body);
    result
}
