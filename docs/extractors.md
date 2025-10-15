# Extractors

Extractors are WsForge's powerful type-safe mechanism for automatically parsing and validating data from WebSocket messages, connections, and application context.

## Table of Contents

- [What are Extractors?](#what-are-extractors)
- [Built-in Extractors](#built-in-extractors)
- [Using Extractors](#using-extractors)
- [Combining Extractors](#combining-extractors)
- [Custom Extractors](#custom-extractors)
- [Best Practices](#best-practices)

## What are Extractors?

Extractors automatically extract data from incoming WebSocket messages and convert them into strongly-typed Rust values. They work by implementing the `FromMessage` trait and are used as function parameters in handlers.

**Benefits:**
- ✅ Type safety at compile time
- ✅ Automatic validation
- ✅ Clean, declarative handler signatures
- ✅ Composable and reusable
- ✅ Error handling built-in

## Built-in Extractors

### Message

Extracts the raw WebSocket message.

**Signature:** `msg: Message`

**Use when:** You need access to the complete message without automatic parsing.

```
use wsforge::prelude::*;

async fn handler(msg: Message) -> Result<String> {
    if msg.is_text() {
        Ok(format!("Text: {}", msg.as_text().unwrap()))
    } else {
        Ok("Binary message".to_string())
    }
}
```

### Json<T>

Automatically deserializes JSON from text messages.

**Signature:** `Json(data): Json<T>`

**Requirements:** Type `T` must implement `serde::Deserialize`

```
use wsforge::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

async fn login_handler(Json(req): Json<LoginRequest>) -> Result<String> {
    println!("Login attempt: {}", req.username);
    Ok("Login successful".to_string())
}
```

**Error Handling:**
```
async fn safe_handler(Json(req): Json<LoginRequest>) -> Result<String> {
    // JSON parse errors are automatically converted to Error::Json
    // and returned to the client
    Ok(format!("Welcome, {}", req.username))
}
```

### Connection

Provides access to the active WebSocket connection.

**Signature:** `conn: Connection`

**Use for:**
- Getting connection ID
- Accessing connection metadata
- Sending responses directly

```
use wsforge::prelude::*;

async fn handler(msg: Message, conn: Connection) -> Result<()> {
    println!("Message from {}: {:?}", conn.id(), msg);

    // Send response directly
    conn.send_text("Message received!")?;

    Ok(())
}
```

**Connection Methods:**
```
// Get connection ID
let id = conn.id();

// Get connection info (address, timestamp, etc.)
let info = conn.info();

// Send messages
conn.send_text("Hello")?;
conn.send_binary(vec!)?;[4][10]
conn.send_json(&my_data)?;
```

### State<T>

Extracts shared application state.

**Signature:** `State(data): State<Arc<T>>`

**Requirements:** State must be added to router with `.with_state()`

```
use wsforge::prelude::*;
use std::sync::Arc;

struct Database {
    // database connection pool
}

async fn handler(State(db): State<Arc<Database>>) -> Result<String> {
    // Use database
    Ok("Query result".to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    let db = Arc::new(Database {});

    Router::new()
        .with_state(db)
        .default_handler(handler(handler))
        .listen("127.0.0.1:8080")
        .await
}
```

**Multiple State Types:**
```
struct Config { max_connections: usize }
struct Cache { /* ... */ }

async fn handler(
    State(config): State<Arc<Config>>,
    State(cache): State<Arc<Cache>>,
) -> Result<String> {
    Ok(format!("Max: {}", config.max_connections))
}
```

### ConnectInfo

Extracts connection metadata.

**Signature:** `ConnectInfo(info): ConnectInfo`

```
use wsforge::prelude::*;

async fn handler(ConnectInfo(info): ConnectInfo) -> Result<String> {
    Ok(format!(
        "Client from {} connected at {}",
        info.addr,
        info.connected_at
    ))
}
```

### Data

Extracts raw binary data from the message.

**Signature:** `Data(bytes): Data`

**Use for:** Binary protocols, file uploads, custom serialization

```
use wsforge::prelude::*;

async fn binary_handler(Data(bytes): Data) -> Result<String> {
    println!("Received {} bytes", bytes.len());
    Ok(format!("Processed {} bytes", bytes.len()))
}
```

### Path<T>

Extracts typed parameters from route paths.

**Signature:** `Path(params): Path<T>`

**Note:** Requires routing middleware to set path params in extensions.

```
use wsforge::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct RoomParams {
    room_id: String,
}

async fn join_room(Path(params): Path<RoomParams>) -> Result<String> {
    Ok(format!("Joining room: {}", params.room_id))
}
```

### Query<T>

Extracts query parameters from the connection URL.

**Signature:** `Query(params): Query<T>`

```
use wsforge::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct SearchParams {
    q: String,
    limit: Option<u32>,
}

async fn search(Query(params): Query<SearchParams>) -> Result<String> {
    let limit = params.limit.unwrap_or(10);
    Ok(format!("Searching for '{}' (limit: {})", params.q, limit))
}
```

### Extension<T>

Extracts custom data from extensions (middleware data).

**Signature:** `Extension(data): Extension<Arc<T>>`

```
use wsforge::prelude::*;

#[derive(Clone)]
struct AuthData {
    user_id: u64,
    role: String,
}

async fn protected_handler(Extension(auth): Extension<Arc<AuthData>>) -> Result<String> {
    Ok(format!("User {} with role {}", auth.user_id, auth.role))
}
```

## Using Extractors

### Order Doesn't Matter

Extractors can be in any order:

```
// Both are valid
async fn handler1(msg: Message, conn: Connection) -> Result<()> { }
async fn handler2(conn: Connection, msg: Message) -> Result<()> { }
```

### Optional Extractors

Some extractors are always available, others depend on setup:

**Always Available:**
- `Message`
- `Connection`
- `ConnectInfo`
- `Data`

**Requires Setup:**
- `State<T>` - Must call `.with_state()`
- `Json<T>` - Message must be valid JSON
- `Path<T>` - Requires routing setup
- `Query<T>` - Requires URL parsing
- `Extension<T>` - Must be set by middleware

## Combining Extractors

You can use up to 8 extractors in a single handler:

```
use wsforge::prelude::*;
use std::sync::Arc;
use serde::Deserialize;

#[derive(Deserialize)]
struct Request {
    action: String,
}

struct Database;
struct Cache;

async fn complex_handler(
    Json(req): Json<Request>,
    conn: Connection,
    State(db): State<Arc<Database>>,
    State(cache): State<Arc<Cache>>,
    ConnectInfo(info): ConnectInfo,
) -> Result<String> {
    println!(
        "Request from {} at {}: {}",
        info.addr,
        info.connected_at,
        req.action
    );

    Ok("Processed".to_string())
}
```

## Custom Extractors

Implement the `FromMessage` trait to create custom extractors:

```
use wsforge::prelude::*;
use async_trait::async_trait;

struct ValidatedText(String);

#[async_trait]
impl FromMessage for ValidatedText {
    async fn from_message(
        message: &Message,
        _conn: &Connection,
        _state: &AppState,
        _extensions: &Extensions,
    ) -> Result<Self> {
        let text = message.as_text()
            .ok_or_else(|| Error::extractor("Message must be text"))?;

        if text.is_empty() {
            return Err(Error::extractor("Text cannot be empty"));
        }

        if text.len() > 1000 {
            return Err(Error::extractor("Text too long (max 1000 chars)"));
        }

        Ok(ValidatedText(text.to_string()))
    }
}

// Use it in handlers
async fn handler(ValidatedText(text): ValidatedText) -> Result<String> {
    Ok(format!("Valid text: {}", text))
}
```

### Authentication Extractor

```
use wsforge::prelude::*;
use async_trait::async_trait;

struct AuthUser {
    user_id: u64,
    username: String,
}

#[async_trait]
impl FromMessage for AuthUser {
    async fn from_message(
        message: &Message,
        _conn: &Connection,
        state: &AppState,
        extensions: &Extensions,
    ) -> Result<Self> {
        // Extract auth token from message or extensions
        let token = message.as_text()
            .ok_or_else(|| Error::extractor("Missing auth token"))?;

        // Validate token (simplified example)
        if token.starts_with("Bearer ") {
            Ok(AuthUser {
                user_id: 123,
                username: "user".to_string(),
            })
        } else {
            Err(Error::custom("Invalid authentication"))
        }
    }
}

async fn protected(user: AuthUser) -> Result<String> {
    Ok(format!("Hello, {}!", user.username))
}
```

## Best Practices

### 1. Use Specific Types

Instead of accepting `Message` and parsing manually, use specific extractors:

```
// ❌ Manual parsing
async fn bad_handler(msg: Message) -> Result<String> {
    let text = msg.as_text().ok_or(...)?;
    let data: MyStruct = serde_json::from_str(text)?;
    // process
}

// ✅ Use Json extractor
async fn good_handler(Json(data): Json<MyStruct>) -> Result<String> {
    // data is already parsed and validated
}
```

### 2. Extract Only What You Need

Don't extract data you won't use:

```
// ❌ Unused extractors
async fn bad(msg: Message, conn: Connection, State(db): State<Arc<DB>>) -> Result<()> {
    // Only uses msg
    Ok(())
}

// ✅ Only extract what's needed
async fn good(msg: Message) -> Result<()> {
    Ok(())
}
```

### 3. Handle Extraction Errors

Extraction errors are automatically returned to the client. Make error messages helpful:

```
#[async_trait]
impl FromMessage for MyExtractor {
    async fn from_message(...) -> Result<Self> {
        // ❌ Vague error
        Err(Error::extractor("Invalid"))

        // ✅ Helpful error
        Err(Error::extractor("Username must be 3-20 characters"))
    }
}
```

### 4. Reuse Custom Extractors

Create extractors for common validation patterns:

```
struct NonEmptyString(String);
struct PositiveNumber(u32);
struct EmailAddress(String);

// Implement FromMessage for each
// Reuse across handlers
```

### 5. Document Your Extractors

Add documentation to custom extractors:

```
/// Extracts and validates a username from the message.
///
/// # Requirements
/// - Message must be text
/// - Length between 3-20 characters
/// - Alphanumeric characters only
///
/// # Errors
/// Returns error if validation fails
struct Username(String);
```

## Error Messages

When extractors fail, they return descriptive errors:

| Extractor | Error Example |
|-----------|---------------|
| `Json<T>` | "JSON error: missing field 'username'" |
| `State<T>` | "Extractor error: State not found" |
| `Path<T>` | "Extractor error: Path parameters not found" |
| Custom | Your custom error message |

## Performance

Extractors are designed for performance:

- ✅ Zero-cost abstractions - compile to efficient code
- ✅ No unnecessary allocations
- ✅ Lazy evaluation - only parse what you request
- ✅ Type checking at compile time prevents runtime errors

## See Also

- [Handlers Guide](handlers.md) - Learn about handler functions
- [State Management](state-management.md) - Working with shared state
- [Error Handling](error-handling.md) - Handling errors in extractors
- [API Reference](api-reference.md) - Complete extractor API
