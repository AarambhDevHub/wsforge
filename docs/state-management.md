# State Management

State management is crucial for sharing data across WebSocket connections. WsForge provides a powerful, type-safe state system that makes it easy to share databases, caches, configuration, and other resources.

## Table of Contents

- [Overview](#overview)
- [Basic Usage](#basic-usage)
- [The State Extractor](#the-state-extractor)
- [Multiple State Types](#multiple-state-types)
- [Mutable State](#mutable-state)
- [Connection State](#connection-state)
- [Common Patterns](#common-patterns)
- [Best Practices](#best-practices)
- [Advanced Usage](#advanced-usage)

## Overview

State in WsForge is:

- **Type-safe**: Each state type is stored and retrieved by its type
- **Thread-safe**: Uses `Arc` for safe sharing across async tasks
- **Zero-cost**: No runtime overhead when not used
- **Flexible**: Store any type that is `Send + Sync + 'static`

## Basic Usage

### Adding State to Router

```
use wsforge::prelude::*;
use std::sync::Arc;

struct Config {
    max_message_size: usize,
    rate_limit: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Arc::new(Config {
        max_message_size: 1024 * 64,
        rate_limit: 100,
    });

    let router = Router::new()
        .with_state(config)
        .default_handler(handler(my_handler));

    router.listen("127.0.0.1:8080").await
}
```

### Accessing State in Handlers

Use the `State` extractor to access state:

```
use wsforge::prelude::*;
use std::sync::Arc;

async fn my_handler(
    State(config): State<Arc<Config>>,
) -> Result<String> {
    Ok(format!("Max size: {}", config.max_message_size))
}
```

## The State Extractor

The `State<T>` extractor automatically retrieves state from the router:

```
// Extract single state
async fn handler1(State(config): State<Arc<Config>>) -> Result<String> {
    // Use config
}

// Extract with other extractors
async fn handler2(
    msg: Message,
    conn: Connection,
    State(config): State<Arc<Config>>,
) -> Result<()> {
    // Use all extractors
}
```

### How It Works

1. State is added to router with `.with_state(data)`
2. State is stored in `AppState` using `TypeId` as key
3. `State<T>` extractor looks up type `T` and returns it
4. If not found, returns error

## Multiple State Types

You can store multiple different types:

```
use wsforge::prelude::*;
use std::sync::Arc;

struct Database {
    pool: sqlx::PgPool,
}

struct Config {
    port: u16,
    host: String,
}

struct Cache {
    redis: redis::Client,
}

#[tokio::main]
async fn main() -> Result<()> {
    let router = Router::new()
        .with_state(Arc::new(Database { /* ... */ }))
        .with_state(Arc::new(Config { /* ... */ }))
        .with_state(Arc::new(Cache { /* ... */ }))
        .default_handler(handler(my_handler));

    router.listen("127.0.0.1:8080").await
}

async fn my_handler(
    State(db): State<Arc<Database>>,
    State(config): State<Arc<Config>>,
    State(cache): State<Arc<Cache>>,
) -> Result<String> {
    // Access all three state types
    Ok("Success".to_string())
}
```

## Mutable State

For state that needs to be modified, use synchronization primitives:

### Using RwLock

```
use wsforge::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

struct UserStore {
    users: RwLock<HashMap<u64, String>>,
}

impl UserStore {
    fn new() -> Self {
        Self {
            users: RwLock::new(HashMap::new()),
        }
    }

    async fn add_user(&self, id: u64, name: String) {
        self.users.write().await.insert(id, name);
    }

    async fn get_user(&self, id: u64) -> Option<String> {
        self.users.read().await.get(&id).cloned()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let store = Arc::new(UserStore::new());

    let router = Router::new()
        .with_state(store)
        .default_handler(handler(user_handler));

    router.listen("127.0.0.1:8080").await
}

async fn user_handler(
    State(store): State<Arc<UserStore>>,
) -> Result<String> {
    store.add_user(1, "Alice".to_string()).await;
    let user = store.get_user(1).await;
    Ok(format!("User: {:?}", user))
}
```

### Using DashMap

For concurrent hash maps, use `DashMap`:

```
use wsforge::prelude::*;
use std::sync::Arc;
use dashmap::DashMap;

struct SessionStore {
    sessions: DashMap<String, SessionData>,
}

struct SessionData {
    user_id: u64,
    created_at: u64,
}

impl SessionStore {
    fn new() -> Self {
        Self {
            sessions: DashMap::new(),
        }
    }

    fn create_session(&self, token: String, user_id: u64) {
        self.sessions.insert(token, SessionData {
            user_id,
            created_at: current_timestamp(),
        });
    }

    fn get_session(&self, token: &str) -> Option<u64> {
        self.sessions.get(token).map(|s| s.user_id)
    }
}
```

## Connection State

The `ConnectionManager` itself is state:

```
use wsforge::prelude::*;
use std::sync::Arc;

async fn broadcast_handler(
    msg: Message,
    State(manager): State<Arc<ConnectionManager>>,
) -> Result<()> {
    println!("Active connections: {}", manager.count());
    manager.broadcast(msg);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // ConnectionManager is automatically added to state
    let router = Router::new()
        .default_handler(handler(broadcast_handler));

    router.listen("127.0.0.1:8080").await
}
```

## Common Patterns

### Database Connection Pool

```
use wsforge::prelude::*;
use std::sync::Arc;
use sqlx::PgPool;

struct AppState {
    db: PgPool,
}

async fn query_handler(
    State(state): State<Arc<AppState>>,
) -> Result<JsonResponse<serde_json::Value>> {
    let result = sqlx::query!("SELECT * FROM users LIMIT 10")
        .fetch_all(&state.db)
        .await
        .map_err(|e| Error::custom(format!("Database error: {}", e)))?;

    Ok(JsonResponse(serde_json::json!({ "users": result })))
}

#[tokio::main]
async fn main() -> Result<()> {
    let db = PgPool::connect("postgres://localhost/mydb")
        .await
        .expect("Failed to connect to database");

    let state = Arc::new(AppState { db });

    let router = Router::new()
        .with_state(state)
        .default_handler(handler(query_handler));

    router.listen("127.0.0.1:8080").await
}
```

### Application Configuration

```
use wsforge::prelude::*;
use std::sync::Arc;
use serde::Deserialize;

#[derive(Deserialize, Clone)]
struct AppConfig {
    max_connections: usize,
    message_size_limit: usize,
    rate_limit_per_second: u32,
}

impl AppConfig {
    fn from_file(path: &str) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| Error::custom(format!("Failed to read config: {}", e)))?;

        toml::from_str(&contents)
            .map_err(|e| Error::custom(format!("Failed to parse config: {}", e)))
    }
}

async fn config_handler(
    State(config): State<Arc<AppConfig>>,
) -> Result<String> {
    Ok(format!("Max connections: {}", config.max_connections))
}
```

### Shared Cache

```
use wsforge::prelude::*;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;

struct Cache {
    data: RwLock<HashMap<String, String>>,
}

impl Cache {
    fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }

    async fn get(&self, key: &str) -> Option<String> {
        self.data.read().await.get(key).cloned()
    }

    async fn set(&self, key: String, value: String) {
        self.data.write().await.insert(key, value);
    }
}

async fn cache_handler(
    msg: Message,
    State(cache): State<Arc<Cache>>,
) -> Result<String> {
    if let Some(text) = msg.as_text() {
        cache.set("last_message".to_string(), text.to_string()).await;
    }

    let last = cache.get("last_message").await;
    Ok(format!("Last message: {:?}", last))
}
```

## Best Practices

### 1. Always Use Arc

State must be wrapped in `Arc` for sharing:

```
// ✅ Good
let config = Arc::new(Config { /* ... */ });
router.with_state(config)

// ❌ Bad - won't compile
let config = Config { /* ... */ };
router.with_state(config)
```

### 2. Choose the Right Lock

- **No mutation needed**: Just use `Arc<T>`
- **Infrequent writes**: Use `RwLock<T>`
- **Concurrent map**: Use `DashMap<K, V>`
- **Atomic counters**: Use `AtomicU64`, `AtomicUsize`

### 3. Keep State Small

Don't store large objects directly in state:

```
// ✅ Good - store connection pool
struct AppState {
    db: PgPool,  // Connection pool is small
}

// ❌ Bad - storing large data
struct AppState {
    all_users: Vec<User>,  // Large, growing data
}
```

### 4. Initialize Expensive Resources Once

```
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize once at startup
    let db = PgPool::connect("postgres://...").await?;
    let redis = redis::Client::open("redis://...")?;

    let state = Arc::new(AppState { db, redis });

    let router = Router::new()
        .with_state(state);

    router.listen("127.0.0.1:8080").await
}
```

### 5. Use Type Aliases for Complex Types

```
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

type SharedUsers = Arc<RwLock<HashMap<u64, User>>>;

async fn handler(State(users): State<SharedUsers>) -> Result<String> {
    // Much cleaner
}
```

## Advanced Usage

### State with Generics

```
use std::marker::PhantomData;

struct Repository<T> {
    data: Arc<RwLock<Vec<T>>>,
    _phantom: PhantomData<T>,
}

impl<T: Clone + Send + Sync + 'static> Repository<T> {
    fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(Vec::new())),
            _phantom: PhantomData,
        }
    }

    async fn add(&self, item: T) {
        self.data.write().await.push(item);
    }
}
```

### Lazy Initialization

```
use once_cell::sync::OnceCell;

struct AppState {
    expensive_resource: OnceCell<ExpensiveResource>,
}

impl AppState {
    fn get_resource(&self) -> &ExpensiveResource {
        self.expensive_resource.get_or_init(|| {
            ExpensiveResource::initialize()
        })
    }
}
```

### Per-Connection State

For data specific to each connection, use `Extensions`:

```
async fn middleware(
    msg: Message,
    conn: Connection,
    extensions: &Extensions,
) -> Result<()> {
    // Store connection-specific data
    extensions.insert("user_id", 123_u64);
    extensions.insert("session_token", "abc123".to_string());
    Ok(())
}

async fn handler(
    Extension(user_id): Extension<u64>,
) -> Result<String> {
    Ok(format!("User ID: {}", *user_id))
}
```

## Common Pitfalls

### ❌ Not Using Arc

```
// Won't compile
let config = Config { /* ... */ };
router.with_state(config)  // Error: Config is not Clone
```

### ❌ Holding Locks Too Long

```
// Bad - lock held across await
async fn bad_handler(State(data): State<Arc<RwLock<Data>>>) -> Result<()> {
    let guard = data.write().await;
    expensive_operation().await;  // Lock held here!
    drop(guard);
    Ok(())
}

// Good - release lock before await
async fn good_handler(State(data): State<Arc<RwLock<Data>>>) -> Result<()> {
    {
        let mut guard = data.write().await;
        guard.update();
    }  // Lock released
    expensive_operation().await;
    Ok(())
}
```

### ❌ Wrong State Type

```
// Added as Arc<Config>
router.with_state(Arc::new(Config { /* ... */ }))

// Trying to extract as Config (without Arc)
async fn handler(State(config): State<Config>) -> Result<()> {
    // Error: Config not found in state
}

// Correct - extract as Arc<Config>
async fn handler(State(config): State<Arc<Config>>) -> Result<()> {
    // Works!
}
```

## Next Steps

- [Extractors Guide](extractors.md) - Learn about all extractor types
- [Broadcasting](broadcasting.md) - Using ConnectionManager state
- [Error Handling](error-handling.md) - Handle state errors
- [Examples](examples.md) - Complete state management examples

---

**Questions?** Check the [FAQ](faq.md) or [Troubleshooting](troubleshooting.md) guide.
