# Static Files

WsForge supports serving static files (HTML, CSS, JavaScript, images) alongside WebSocket connections on the same port, enabling you to build complete web applications with a single server.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Directory Structure](#directory-structure)
- [MIME Types](#mime-types)
- [Security](#security)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)

## Overview

The static file handler allows you to:

- Serve HTML, CSS, JavaScript, and other assets
- Run HTTP and WebSocket on the same port
- Automatically detect MIME types
- Serve index files for directory requests
- Prevent path traversal attacks

## Quick Start

### Basic Setup

```
use wsforge::prelude::*;

async fn ws_handler(msg: Message) -> Result<Message> {
    Ok(msg)
}

#[tokio::main]
async fn main() -> Result<()> {
    let router = Router::new()
        .serve_static("public")  // Serve files from ./public directory
        .default_handler(handler(ws_handler));

    router.listen("127.0.0.1:8080").await?;
    Ok(())
}
```

### Directory Structure

Create a `public` directory in your project root:

```
my-project/
├── Cargo.toml
├── src/
│   └── main.rs
└── public/
    ├── index.html
    ├── style.css
    ├── app.js
    └── assets/
        └── logo.png
```

### Accessing Files

Once configured, files are accessible via HTTP:

- `http://localhost:8080/` → `public/index.html`
- `http://localhost:8080/style.css` → `public/style.css`
- `http://localhost:8080/app.js` → `public/app.js`
- `http://localhost:8080/assets/logo.png` → `public/assets/logo.png`

WebSocket connections work on the same port:

- `ws://localhost:8080` → WebSocket handler

## Configuration

### Custom Static Directory

```
Router::new()
    .serve_static("/var/www/html")  // Absolute path
    .serve_static("dist")           // Relative path
    .serve_static("../shared/public") // Parent directory
```

### Custom Index File

The default index file is `index.html`. To customize:

```
use wsforge::prelude::*;

let handler = StaticFileHandler::new("public")
    .with_index("default.html");

// Note: Direct StaticFileHandler usage requires custom integration
```

### Path Configuration

All paths are resolved relative to the current working directory:

```
// From workspace root
.serve_static("examples/chat-web/static")

// From project root
.serve_static("public")

// Absolute path
.serve_static("/opt/myapp/www")
```

## Directory Structure

### Recommended Layout

```
public/
├── index.html          # Main page (served for /)
├── style.css           # Stylesheets
├── app.js              # JavaScript
├── favicon.ico         # Favicon
└── assets/             # Images, fonts, etc.
    ├── images/
    │   ├── logo.png
    │   └── banner.jpg
    ├── fonts/
    │   └── roboto.woff2
    └── data/
        └── config.json
```

### Single Page Application (SPA)

```
dist/
├── index.html          # Main SPA entry point
├── bundle.js           # Webpack/Vite output
├── styles.css          # Compiled styles
└── assets/
    └── ...
```

### Multi-Page Application

```
public/
├── index.html
├── about.html
├── contact.html
├── css/
│   ├── common.css
│   ├── home.css
│   └── about.css
└── js/
    ├── common.js
    └── websocket.js
```

## MIME Types

WsForge automatically detects MIME types based on file extensions:

| Extension | MIME Type | Description |
|-----------|-----------|-------------|
| `.html` | `text/html` | HTML documents |
| `.css` | `text/css` | Stylesheets |
| `.js` | `application/javascript` | JavaScript |
| `.json` | `application/json` | JSON data |
| `.png` | `image/png` | PNG images |
| `.jpg`, `.jpeg` | `image/jpeg` | JPEG images |
| `.gif` | `image/gif` | GIF images |
| `.svg` | `image/svg+xml` | SVG graphics |
| `.ico` | `image/x-icon` | Favicons |
| `.woff`, `.woff2` | `font/woff`, `font/woff2` | Web fonts |
| `.wasm` | `application/wasm` | WebAssembly |
| `.pdf` | `application/pdf` | PDF documents |
| `.zip` | `application/zip` | ZIP archives |
| `.txt` | `text/plain` | Text files |

### Custom MIME Types

MIME types are determined by the `mime_guess` crate. For custom types, ensure proper file extensions.

## Security

### Path Traversal Protection

WsForge automatically prevents path traversal attacks:

```
// These requests are blocked:
// http://localhost:8080/../../../etc/passwd
// http://localhost:8080/..%2F..%2Fetc%2Fpasswd
// http://localhost:8080/public/../private/secret.txt
```

The static file handler:
1. Canonicalizes all paths
2. Ensures requested files are within the root directory
3. Rejects any path escaping the root

### Best Practices

1. **Never serve sensitive directories**:
   ```
   // ❌ BAD - Don't do this!
   .serve_static(".")  // Exposes entire project
   .serve_static("/")  // Exposes entire filesystem

   // ✅ GOOD
   .serve_static("public")  // Only public directory
   ```

2. **Separate public and private files**:
   ```
   project/
   ├── public/          # Safe to serve
   │   └── ...
   ├── private/         # NOT served
   │   └── secrets.txt
   └── src/             # NOT served
       └── main.rs
   ```

3. **Use environment-specific paths**:
   ```
   let static_dir = if cfg!(debug_assertions) {
       "public"
   } else {
       "/var/www/production"
   };

   router.serve_static(static_dir);
   ```

4. **Validate file permissions**:
   ```
   # Set appropriate permissions
   chmod -R 644 public/**/*
   chmod 755 public/
   ```

## Examples

### Chat Application with UI

**Project Structure:**
```
chat-app/
├── src/main.rs
└── public/
    ├── index.html
    ├── style.css
    └── app.js
```

**src/main.rs:**
```
use wsforge::prelude::*;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct ChatMessage {
    username: String,
    text: String,
}

async fn chat_handler(
    Json(msg): Json<ChatMessage>,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<()> {
    let json = serde_json::to_string(&msg)?;
    manager.broadcast(Message::text(json));
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let router = Router::new()
        .serve_static("public")
        .default_handler(handler(chat_handler));

    println!("Chat app: http://127.0.0.1:8080");
    router.listen("127.0.0.1:8080").await?;
    Ok(())
}
```

**public/index.html:**
```
<!DOCTYPE html>
<html>
<head>
    <title>WebSocket Chat</title>
    <link rel="stylesheet" href="style.css">
</head>
<body>
    <div id="chat">
        <div id="messages"></div>
        <input type="text" id="username" placeholder="Your name">
        <input type="text" id="message" placeholder="Type a message...">
        <button id="send">Send</button>
    </div>
    <script src="app.js"></script>
</body>
</html>
```

**public/app.js:**
```
const ws = new WebSocket('ws://localhost:8080');

ws.onmessage = (event) => {
    const msg = JSON.parse(event.data);
    const div = document.createElement('div');
    div.textContent = `${msg.username}: ${msg.text}`;
    document.getElementById('messages').appendChild(div);
};

document.getElementById('send').onclick = () => {
    const username = document.getElementById('username').value;
    const text = document.getElementById('message').value;

    ws.send(JSON.stringify({ username, text }));
    document.getElementById('message').value = '';
};
```

### Real-Time Dashboard

```
use wsforge::prelude::*;
use std::sync::Arc;

async fn metrics_handler(
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<JsonResponse<serde_json::Value>> {
    let metrics = serde_json::json!({
        "connections": manager.count(),
        "timestamp": chrono::Utc::now().timestamp()
    });
    Ok(JsonResponse(metrics))
}

#[tokio::main]
async fn main() -> Result<()> {
    let router = Router::new()
        .serve_static("dashboard")  // Serve dashboard UI
        .route("/metrics", handler(metrics_handler))
        .default_handler(handler(ws_handler));

    router.listen("0.0.0.0:3000").await?;
    Ok(())
}
```

### Production Deployment

```
use wsforge::prelude::*;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let static_dir = env::var("STATIC_DIR")
        .unwrap_or_else(|_| "public".to_string());

    let router = Router::new()
        .serve_static(static_dir)
        .default_handler(handler(ws_handler));

    let addr = env::var("BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    println!("Server running on {}", addr);
    router.listen(addr).await?;
    Ok(())
}
```

## Troubleshooting

### Files Not Found (404)

**Problem**: Accessing files returns 404 errors.

**Solutions**:

1. **Check directory path**:
   ```
   // Verify the path exists
   use std::path::Path;
   assert!(Path::new("public").exists());
   ```

2. **Check working directory**:
   ```
   # Run from project root, not src/
   cargo run

   # Not from:
   cd src && cargo run  # ❌ Wrong!
   ```

3. **Use absolute paths for debugging**:
   ```
   let path = std::env::current_dir()?.join("public");
   println!("Serving from: {:?}", path);
   router.serve_static(path);
   ```

### Wrong MIME Type

**Problem**: Files served with incorrect Content-Type.

**Solutions**:

1. **Check file extension**:
   ```
   # Ensure correct extensions
   mv script.txt script.js
   mv page.txt page.html
   ```

2. **Verify file format**:
   ```
   file public/app.js  # Should show: ASCII text
   ```

### WebSocket Connection Fails

**Problem**: WebSocket handshake fails when static files are enabled.

**Solution**: Ensure WebSocket upgrade requests are properly detected:

```
// WsForge automatically handles this
// The router checks for "Upgrade: websocket" header
```

### Permission Denied

**Problem**: Cannot read files (Permission denied error).

**Solutions**:

1. **Check file permissions**:
   ```
   ls -la public/
   chmod 644 public/**/*
   ```

2. **Check directory permissions**:
   ```
   chmod 755 public/
   ```

3. **Run with appropriate user**:
   ```
   # Development
   cargo run

   # Production (as non-root user)
   sudo -u www-data ./target/release/myapp
   ```

### Large Files

**Problem**: Large files cause memory issues.

**Solution**: WsForge reads entire files into memory. For large files (>10MB), consider:

1. **Using a CDN** for large assets
2. **Implementing streaming** (custom handler)
3. **Using nginx** as a reverse proxy for static files

### Caching Issues

**Problem**: Browser caches old versions of files.

**Solutions**:

1. **During development - disable cache**:
   ```
   // In your HTML
   <script src="app.js?v=1"></script>
   ```

2. **Use cache-busting**:
   ```
   // Append version/hash to filenames
   // app.123abc.js instead of app.js
   ```

3. **Configure headers** (requires custom implementation):
   ```
   // Add cache headers to responses
   // Cache-Control: no-cache (development)
   // Cache-Control: max-age=31536000 (production)
   ```

## Advanced Topics

### Custom 404 Pages

To serve a custom 404 page, place `404.html` in your static directory and handle it in your WebSocket error handler.

### Compression

Static files are served uncompressed. For production:

1. Pre-compress files (gzip):
   ```
   find public -type f -name "*.js" -exec gzip -k {} \;
   ```

2. Use nginx/CDN for compression

### Hot Reload (Development)

For automatic reload during development, use `cargo-watch`:

```
cargo watch -x run
```

Or implement file watching in your application.

## See Also

- [Examples](examples.md) - Complete working examples
- [Deployment](deployment.md) - Production deployment guide
- [Routing](routing.md) - Message routing and handlers
