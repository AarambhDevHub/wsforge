# Error Handling

WsForge provides comprehensive error handling mechanisms to help you build robust WebSocket applications.

## Table of Contents

- [Error Types](#error-types)
- [Handling Errors in Handlers](#handling-errors-in-handlers)
- [Custom Error Messages](#custom-error-messages)
- [Error Propagation](#error-propagation)
- [Connection Errors](#connection-errors)
- [Message Parsing Errors](#message-parsing-errors)
- [Handler Errors](#handler-errors)
- [Best Practices](#best-practices)
- [Error Recovery Strategies](#error-recovery-strategies)

## Error Types

WsForge uses a unified `Error` enum for all error conditions:

```
use wsforge::prelude::*;

pub enum Error {
    WebSocket(tokio_tungstenite::tungstenite::Error),  // WebSocket protocol errors
    Io(std::io::Error),                                // I/O errors
    Json(serde_json::Error),                           // JSON parsing errors
    ConnectionNotFound(String),                         // Connection lookup failures
    RouteNotFound(String),                             // Routing errors
    InvalidMessage,                                     // Message format errors
    Handler(String),                                    // Handler execution errors
    Extractor(String),                                  // Type extraction errors
    Custom(String),                                     // Application-specific errors
}
```

## Handling Errors in Handlers

### Basic Error Handling

Handlers return `Result<T>` to handle errors gracefully:

```
use wsforge::prelude::*;

async fn handler(msg: Message) -> Result<String> {
    // Parse JSON - automatically converts serde_json::Error to Error::Json
    let data: serde_json::Value = msg.json()?;

    // Validate
    if data.is_null() {
        return Err(Error::custom("Data cannot be null"));
    }

    Ok("Success".to_string())
}
```

### Pattern Matching on Errors

```
use wsforge::prelude::*;

async fn detailed_handler(msg: Message) -> Result<String> {
    match msg.json::<serde_json::Value>() {
        Ok(data) => {
            // Process data
            Ok(format!("Processed: {}", data))
        }
        Err(Error::Json(e)) => {
            // Handle JSON parsing error specifically
            Err(Error::custom(format!("Invalid JSON: {}", e)))
        }
        Err(e) => {
            // Handle other errors
            Err(e)
        }
    }
}
```

## Custom Error Messages

### Creating Custom Errors

Use helper methods to create specific error types:

```
use wsforge::prelude::*;

async fn validate_user(username: &str) -> Result<()> {
    // Custom validation error
    if username.is_empty() {
        return Err(Error::custom("Username cannot be empty"));
    }

    if username.len() < 3 {
        return Err(Error::custom(
            format!("Username '{}' too short (minimum 3 characters)", username)
        ));
    }

    // Handler-specific error
    if username.contains("admin") {
        return Err(Error::handler("Reserved username"));
    }

    // Extractor error (when data extraction fails)
    if username.contains(" ") {
        return Err(Error::extractor("Username cannot contain spaces"));
    }

    Ok(())
}
```

### Error Messages to Clients

Errors are automatically sent back to clients as text messages:

```
async fn handler(msg: Message) -> Result<String> {
    // This error will be sent as: "Error: Invalid input"
    Err(Error::custom("Invalid input"))
}
```

To send custom error formats:

```
use serde::Serialize;

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    code: u32,
}

async fn handler(msg: Message) -> Result<JsonResponse<ErrorResponse>> {
    // Validate
    if msg.as_text().is_none() {
        return Ok(JsonResponse(ErrorResponse {
            error: "Text message required".to_string(),
            code: 400,
        }));
    }

    Ok(JsonResponse(ErrorResponse {
        error: "Unknown error".to_string(),
        code: 500,
    }))
}
```

## Error Propagation

### Using the ? Operator

The `?` operator automatically converts errors:

```
use wsforge::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct Request {
    action: String,
    data: String,
}

async fn process_request(msg: Message) -> Result<String> {
    // ? automatically converts serde_json::Error to Error::Json
    let req: Request = msg.json()?;

    // ? propagates the error up
    let result = validate_action(&req.action)?;

    Ok(format!("Processed: {}", result))
}

fn validate_action(action: &str) -> Result<String> {
    match action {
        "start" | "stop" => Ok(action.to_string()),
        _ => Err(Error::custom(format!("Invalid action: {}", action))),
    }
}
```

### Chaining Operations

```
async fn complex_handler(msg: Message) -> Result<String> {
    let data: serde_json::Value = msg.json()
        .map_err(|e| Error::custom(format!("Failed to parse JSON: {}", e)))?;

    let username = data["username"]
        .as_str()
        .ok_or_else(|| Error::custom("Missing username field"))?;

    let age = data["age"]
        .as_u64()
        .ok_or_else(|| Error::custom("Invalid age field"))?;

    if age < 18 {
        return Err(Error::custom("Must be 18 or older"));
    }

    Ok(format!("User {} registered", username))
}
```

## Connection Errors

### Handling Connection Failures

```
use wsforge::prelude::*;
use std::sync::Arc;

async fn send_to_user(
    manager: &Arc<ConnectionManager>,
    user_id: &str,
    message: &str,
) -> Result<()> {
    // Get connection or return error
    let conn = manager.get(&user_id.to_string())
        .ok_or_else(|| Error::ConnectionNotFound(user_id.to_string()))?;

    // Try to send, handle send errors
    conn.send_text(message)
        .map_err(|e| Error::custom(format!("Failed to send to {}: {}", user_id, e)))?;

    Ok(())
}
```

### Graceful Connection Cleanup

```
#[tokio::main]
async fn main() -> Result<()> {
    let router = Router::new()
        .default_handler(handler(my_handler))
        .on_disconnect(|manager, conn_id| {
            println!("⚠️  Connection {} lost", conn_id);

            // Notify other connections
            let msg = format!("User {} disconnected", conn_id);
            manager.broadcast(Message::text(msg));
        });

    router.listen("127.0.0.1:8080").await?;
    Ok(())
}
```

## Message Parsing Errors

### JSON Validation

```
use wsforge::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct UserData {
    #[serde(default)]
    username: String,
    #[serde(default)]
    email: String,
}

async fn validate_handler(msg: Message) -> Result<String> {
    // Parse with detailed error handling
    let user: UserData = match msg.json() {
        Ok(u) => u,
        Err(Error::Json(e)) => {
            return Err(Error::custom(
                format!("Invalid JSON format: {}. Expected {{\"username\": \"...\", \"email\": \"...\"}}", e)
            ));
        }
        Err(e) => return Err(e),
    };

    // Validate fields
    if user.username.is_empty() {
        return Err(Error::custom("Username is required"));
    }

    if !user.email.contains('@') {
        return Err(Error::custom("Invalid email format"));
    }

    Ok("Validation successful".to_string())
}
```

### Binary Message Handling

```
async fn binary_handler(msg: Message) -> Result<Vec<u8>> {
    if !msg.is_binary() {
        return Err(Error::InvalidMessage);
    }

    let data = msg.as_bytes();

    if data.len() > 1_000_000 {
        return Err(Error::custom("Message too large (max 1MB)"));
    }

    // Process binary data
    Ok(data.to_vec())
}
```

## Handler Errors

### Timeout Handling

```
use tokio::time::{timeout, Duration};

async fn timeout_handler(msg: Message) -> Result<String> {
    // Set 5-second timeout for processing
    match timeout(Duration::from_secs(5), process_message(msg)).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(Error::handler("Processing timeout (>5s)")),
    }
}

async fn process_message(msg: Message) -> Result<String> {
    // Some potentially slow operation
    tokio::time::sleep(Duration::from_secs(1)).await;
    Ok("Processed".to_string())
}
```

### Resource Exhaustion

```
use std::sync::Arc;
use tokio::sync::Semaphore;

async fn rate_limited_handler(
    msg: Message,
    State(semaphore): State<Arc<Semaphore>>,
) -> Result<String> {
    // Limit concurrent operations
    let permit = semaphore.try_acquire()
        .map_err(|_| Error::handler("Server busy, try again later"))?;

    // Process with permit held
    let result = process_request(msg).await?;

    drop(permit);
    Ok(result)
}

async fn process_request(msg: Message) -> Result<String> {
    // Processing logic
    Ok("Done".to_string())
}
```

## Best Practices

### 1. Use Specific Error Types

```
// ❌ Bad: Generic error
return Err(Error::custom("Error"));

// ✅ Good: Specific error
return Err(Error::custom("Invalid username: must be 3-20 characters"));
```

### 2. Validate Early

```
async fn handler(msg: Message) -> Result<String> {
    // Validate message type first
    if !msg.is_text() {
        return Err(Error::InvalidMessage);
    }

    // Then parse
    let data: serde_json::Value = msg.json()?;

    // Then validate content
    if !data.is_object() {
        return Err(Error::custom("Expected JSON object"));
    }

    // Process...
    Ok("Success".to_string())
}
```

### 3. Log Errors Appropriately

```
async fn logging_handler(msg: Message) -> Result<String> {
    match process(msg).await {
        Ok(result) => Ok(result),
        Err(e) => {
            // Log error for debugging
            tracing::error!("Handler error: {}", e);

            // Return user-friendly message
            Err(Error::custom("Processing failed, please try again"))
        }
    }
}
```

### 4. Don't Expose Internal Details

```
// ❌ Bad: Exposes internal paths
Err(Error::custom(format!("Failed to read /etc/app/config.toml: {}", e)))

// ✅ Good: Generic but informative
Err(Error::custom("Configuration error, contact administrator"))
```

### 5. Handle All Error Cases

```
async fn complete_handler(msg: Message) -> Result<String> {
    // Handle all possible error cases
    let text = match msg.as_text() {
        Some(t) => t,
        None => return Err(Error::custom("Text message required")),
    };

    let data: serde_json::Value = match serde_json::from_str(text) {
        Ok(d) => d,
        Err(e) => return Err(Error::custom(format!("Invalid JSON: {}", e))),
    };

    let field = match data.get("field") {
        Some(f) => f,
        None => return Err(Error::custom("Missing 'field' in JSON")),
    };

    Ok(field.to_string())
}
```

## Error Recovery Strategies

### Retry Logic

```
async fn retry_handler(msg: Message) -> Result<String> {
    let max_retries = 3;
    let mut attempts = 0;

    loop {
        match process_message(&msg).await {
            Ok(result) => return Ok(result),
            Err(e) if attempts < max_retries => {
                attempts += 1;
                tracing::warn!("Attempt {} failed: {}", attempts, e);
                tokio::time::sleep(tokio::time::Duration::from_millis(100 * attempts)).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### Fallback Behavior

```
async fn fallback_handler(msg: Message) -> Result<String> {
    // Try primary method
    match primary_process(&msg).await {
        Ok(result) => Ok(result),
        Err(e) => {
            tracing::warn!("Primary method failed: {}, trying fallback", e);
            // Try fallback method
            fallback_process(&msg).await
        }
    }
}
```

### Circuit Breaker Pattern

```
use std::sync::atomic::{AtomicU32, Ordering};

static FAILURE_COUNT: AtomicU32 = AtomicU32::new(0);
const CIRCUIT_THRESHOLD: u32 = 5;

async fn circuit_breaker_handler(msg: Message) -> Result<String> {
    // Check if circuit is open
    if FAILURE_COUNT.load(Ordering::Relaxed) >= CIRCUIT_THRESHOLD {
        return Err(Error::handler("Service temporarily unavailable"));
    }

    match risky_operation(&msg).await {
        Ok(result) => {
            // Reset on success
            FAILURE_COUNT.store(0, Ordering::Relaxed);
            Ok(result)
        }
        Err(e) => {
            // Increment failure count
            FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);
            Err(e)
        }
    }
}
```

## Testing Error Cases

```
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_invalid_json_error() {
        let msg = Message::text("not json");
        let result = handler(msg).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_missing_field_error() {
        let msg = Message::text(r#"{"wrong": "field"}"#);
        let result = handler(msg).await;
        assert!(matches!(result, Err(Error::Custom(_))));
    }

    #[tokio::test]
    async fn test_connection_not_found() {
        let manager = Arc::new(ConnectionManager::new());
        let result = send_to_user(&manager, "nonexistent", "test").await;
        assert!(matches!(result, Err(Error::ConnectionNotFound(_))));
    }
}
```

## Summary

Effective error handling in WsForge:

1. ✅ Use `Result<T>` for all fallible operations
2. ✅ Provide specific, actionable error messages
3. ✅ Validate input early and thoroughly
4. ✅ Log errors for debugging
5. ✅ Handle all error cases explicitly
6. ✅ Implement recovery strategies when appropriate
7. ✅ Test error scenarios
8. ✅ Don't expose sensitive internal details

For more information:
- [Handlers Documentation](handlers.md)
- [Testing Guide](testing.md)
- [API Reference](api-reference.md)
