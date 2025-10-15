//! Shared application state management.
//!
//! This module provides a type-safe, thread-safe container for storing and retrieving
//! application state that needs to be shared across all WebSocket connections. State
//! is commonly used for database connections, configuration, caches, and other shared
//! resources.
//!
//! # Overview
//!
//! The [`AppState`] type uses a type-map pattern, allowing you to store multiple
//! different types of state in a single container. Each type is stored separately
//! and can be retrieved by its type, ensuring type safety at compile time.
//!
//! # Design
//!
//! - **Type-safe**: Each state type is stored and retrieved by its exact type
//! - **Thread-safe**: Uses `Arc` and `DashMap` for lock-free concurrent access
//! - **Zero-cost abstraction**: No runtime overhead when state is not used
//! - **Flexible**: Any type that is `Send + Sync + 'static` can be stored
//!
//! # Common Use Cases
//!
//! | Use Case | Example Type | Description |
//! |----------|--------------|-------------|
//! | Database | `Arc<DatabasePool>` | Shared database connection pool |
//! | Configuration | `Arc<Config>` | Application settings and configuration |
//! | Cache | `Arc<Cache>` | In-memory cache for frequently accessed data |
//! | Metrics | `Arc<Metrics>` | Performance metrics and monitoring |
//! | Connection Manager | `Arc<ConnectionManager>` | Manage all active WebSocket connections |
//!
//! # Examples
//!
//! ## Single State Type
//!
//! ```
//! use wsforge::prelude::*;
//! use std::sync::Arc;
//!
//! struct Database {
//!     connection_string: String,
//! }
//!
//! async fn query_handler(State(db): State<Arc<Database>>) -> Result<String> {
//!     Ok(format!("Connected to: {}", db.connection_string))
//! }
//!
//! # fn example() {
//! let db = Arc::new(Database {
//!     connection_string: "postgres://localhost/mydb".to_string(),
//! });
//!
//! let router = Router::new()
//!     .with_state(db)
//!     .default_handler(handler(query_handler));
//! # }
//! ```
//!
//! ## Multiple State Types
//!
//! ```
//! use wsforge::prelude::*;
//! use std::sync::Arc;
//!
//! struct Database {
//!     url: String,
//! }
//!
//! struct Config {
//!     max_connections: usize,
//!     timeout_seconds: u64,
//! }
//!
//! struct Cache {
//!     data: std::collections::HashMap<String, String>,
//! }
//!
//! async fn handler(
//!     State(db): State<Arc<Database>>,
//!     State(config): State<Arc<Config>>,
//!     State(cache): State<Arc<Cache>>,
//! ) -> Result<String> {
//!     Ok(format!(
//!         "DB: {}, Max: {}, Cache size: {}",
//!         db.url,
//!         config.max_connections,
//!         cache.data.len()
//!     ))
//! }
//!
//! # fn example() {
//! let router = Router::new()
//!     .with_state(Arc::new(Database { url: "...".to_string() }))
//!     .with_state(Arc::new(Config { max_connections: 100, timeout_seconds: 30 }))
//!     .with_state(Arc::new(Cache { data: Default::default() }))
//!     .default_handler(handler(handler));
//! # }
//! ```
//!
//! ## Mutable State with RwLock
//!
//! ```
//! use wsforge::prelude::*;
//! use std::sync::{Arc, RwLock};
//!
//! struct Counter {
//!     value: RwLock<u64>,
//! }
//!
//! impl Counter {
//!     fn increment(&self) {
//!         let mut value = self.value.write().unwrap();
//!         *value += 1;
//!     }
//!
//!     fn get(&self) -> u64 {
//!         *self.value.read().unwrap()
//!     }
//! }
//!
//! async fn count_handler(State(counter): State<Arc<Counter>>) -> Result<String> {
//!     counter.increment();
//!     Ok(format!("Count: {}", counter.get()))
//! }
//!
//! # fn example() {
//! let counter = Arc::new(Counter {
//!     value: RwLock::new(0),
//! });
//!
//! let router = Router::new()
//!     .with_state(counter)
//!     .default_handler(handler(count_handler));
//! # }
//! ```

use dashmap::DashMap;
use std::any::{Any, TypeId};
use std::sync::Arc;

/// A type-safe container for shared application state.
///
/// `AppState` allows you to store multiple different types of state in a single
/// container. Each type is identified by its `TypeId`, ensuring type safety when
/// retrieving state.
///
/// # Thread Safety
///
/// `AppState` is fully thread-safe and can be cloned cheaply (uses `Arc` internally).
/// Multiple handlers can access the same state concurrently without additional
/// synchronization.
///
/// # Memory Management
///
/// State is stored using `Arc`, so cloning `AppState` or extracting state with
/// the `State` extractor only increments a reference count. The actual state
/// data is shared across all references.
///
/// # Type Requirements
///
/// Types stored in `AppState` must be:
/// - `Send`: Can be sent between threads
/// - `Sync`: Can be referenced from multiple threads
/// - `'static`: Has a static lifetime
///
/// # Examples
///
/// ## Creating and Using State
///
/// ```
/// use wsforge::prelude::*;
/// use std::sync::Arc;
///
/// # fn example() {
/// // Create empty state
/// let state = AppState::new();
///
/// // Add some data
/// state.insert(Arc::new("Hello".to_string()));
/// state.insert(Arc::new(42_u32));
///
/// // Retrieve data
/// let text: Option<Arc<String>> = state.get();
/// assert_eq!(*text.unwrap(), "Hello");
///
/// let number: Option<Arc<u32>> = state.get();
/// assert_eq!(*number.unwrap(), 42);
/// # }
/// ```
///
/// ## With Router
///
/// ```
/// use wsforge::prelude::*;
/// use std::sync::Arc;
///
/// struct AppConfig {
///     name: String,
///     version: String,
/// }
///
/// async fn info_handler(State(config): State<Arc<AppConfig>>) -> Result<String> {
///     Ok(format!("{} v{}", config.name, config.version))
/// }
///
/// # fn example() {
/// let config = Arc::new(AppConfig {
///     name: "MyApp".to_string(),
///     version: "1.0.0".to_string(),
/// });
///
/// let router = Router::new()
///     .with_state(config)
///     .default_handler(handler(info_handler));
/// # }
/// ```
///
/// ## Complex State Management
///
/// ```
/// use wsforge::prelude::*;
/// use std::sync::Arc;
/// use std::collections::HashMap;
///
/// struct UserStore {
///     users: tokio::sync::RwLock<HashMap<u64, String>>,
/// }
///
/// impl UserStore {
///     fn new() -> Self {
///         Self {
///             users: tokio::sync::RwLock::new(HashMap::new()),
///         }
///     }
///
///     async fn add_user(&self, id: u64, name: String) {
///         self.users.write().await.insert(id, name);
///     }
///
///     async fn get_user(&self, id: u64) -> Option<String> {
///         self.users.read().await.get(&id).cloned()
///     }
/// }
///
/// async fn user_handler(
///     State(store): State<Arc<UserStore>>,
/// ) -> Result<String> {
///     store.add_user(1, "Alice".to_string()).await;
///     let user = store.get_user(1).await;
///     Ok(format!("User: {:?}", user))
/// }
/// # }
/// ```
#[derive(Clone)]
pub struct AppState {
    /// Internal storage mapping TypeId to Arc-wrapped values
    data: Arc<DashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
}

impl AppState {
    /// Creates a new empty `AppState`.
    ///
    /// The state starts with no data. Use [`insert`](Self::insert) to add state.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    ///
    /// let state = AppState::new();
    /// ```
    pub fn new() -> Self {
        Self {
            data: Arc::new(DashMap::new()),
        }
    }

    /// Inserts a value into the state.
    ///
    /// If a value of the same type already exists, it will be replaced.
    /// The value is automatically wrapped in an `Arc`.
    ///
    /// # Type Requirements
    ///
    /// The type `T` must implement:
    /// - `Send`: Can be transferred across thread boundaries
    /// - `Sync`: Can be safely shared between threads
    /// - `'static`: Has a static lifetime (no borrowed data)
    ///
    /// # Arguments
    ///
    /// * `value` - The value to store in state
    ///
    /// # Examples
    ///
    /// ## Basic Usage
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use std::sync::Arc;
    ///
    /// # fn example() {
    /// let state = AppState::new();
    ///
    /// // Insert different types
    /// state.insert(Arc::new(String::from("Hello")));
    /// state.insert(Arc::new(42_u32));
    /// state.insert(Arc::new(true));
    /// # }
    /// ```
    ///
    /// ## Replacing Values
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use std::sync::Arc;
    ///
    /// # fn example() {
    /// let state = AppState::new();
    ///
    /// state.insert(Arc::new(10_u32));
    /// assert_eq!(*state.get::<u32>().unwrap(), 10);
    ///
    /// // Replace with new value
    /// state.insert(Arc::new(20_u32));
    /// assert_eq!(*state.get::<u32>().unwrap(), 20);
    /// # }
    /// ```
    ///
    /// ## Custom Types
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use std::sync::Arc;
    ///
    /// struct Database {
    ///     url: String,
    /// }
    ///
    /// # fn example() {
    /// let state = AppState::new();
    ///
    /// let db = Arc::new(Database {
    ///     url: "postgres://localhost/mydb".to_string(),
    /// });
    ///
    /// state.insert(db);
    /// # }
    /// ```
    pub fn insert<T: Send + Sync + 'static>(&self, value: Arc<T>) {
        self.data.insert(TypeId::of::<T>(), value);
    }

    /// Retrieves a value from the state by its type.
    ///
    /// Returns `None` if no value of type `T` has been stored.
    /// Returns `Some(Arc<T>)` if a value exists.
    ///
    /// # Type Safety
    ///
    /// The returned value is guaranteed to be of type `T` because values
    /// are stored and retrieved using `TypeId`.
    ///
    /// # Performance
    ///
    /// This operation is O(1) and lock-free, making it very efficient even
    /// with concurrent access from multiple threads.
    ///
    /// # Examples
    ///
    /// ## Basic Retrieval
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use std::sync::Arc;
    ///
    /// # fn example() {
    /// let state = AppState::new();
    /// state.insert(Arc::new(String::from("Hello")));
    ///
    /// let text: Option<Arc<String>> = state.get();
    /// assert_eq!(*text.unwrap(), "Hello");
    ///
    /// // Trying to get a type that doesn't exist
    /// let number: Option<Arc<u32>> = state.get();
    /// assert!(number.is_none());
    /// # }
    /// ```
    ///
    /// ## Pattern Matching
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use std::sync::Arc;
    ///
    /// # fn example() {
    /// let state = AppState::new();
    /// state.insert(Arc::new(42_u32));
    ///
    /// match state.get::<u32>() {
    ///     Some(value) => println!("Found: {}", value),
    ///     None => println!("Not found"),
    /// }
    /// # }
    /// ```
    ///
    /// ## Multiple Types
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use std::sync::Arc;
    ///
    /// struct Config { port: u16 }
    /// struct Database { url: String }
    ///
    /// # fn example() {
    /// let state = AppState::new();
    /// state.insert(Arc::new(Config { port: 8080 }));
    /// state.insert(Arc::new(Database { url: "...".to_string() }));
    ///
    /// // Each type is stored separately
    /// let config: Arc<Config> = state.get().unwrap();
    /// let db: Arc<Database> = state.get().unwrap();
    ///
    /// println!("Port: {}, DB: {}", config.port, db.url);
    /// # }
    /// ```
    ///
    /// ## With Error Handling
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use std::sync::Arc;
    ///
    /// struct Database;
    ///
    /// # fn example() -> Result<()> {
    /// let state = AppState::new();
    ///
    /// let db = state
    ///     .get::<Database>()
    ///     .ok_or_else(|| Error::custom("Database not configured"))?;
    ///
    /// // Use db...
    /// # Ok(())
    /// # }
    /// ```
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.data
            .get(&TypeId::of::<T>())
            .and_then(|arc| arc.value().clone().downcast::<T>().ok())
    }

    /// Checks if a value of type `T` exists in the state.
    ///
    /// This is equivalent to `state.get::<T>().is_some()` but more explicit.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use std::sync::Arc;
    ///
    /// # fn example() {
    /// let state = AppState::new();
    /// state.insert(Arc::new(42_u32));
    ///
    /// assert!(state.contains::<u32>());
    /// assert!(!state.contains::<String>());
    /// # }
    /// ```
    pub fn contains<T: Send + Sync + 'static>(&self) -> bool {
        self.data.contains_key(&TypeId::of::<T>())
    }

    /// Removes a value of type `T` from the state.
    ///
    /// Returns the removed value if it existed, or `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use std::sync::Arc;
    ///
    /// # fn example() {
    /// let state = AppState::new();
    /// state.insert(Arc::new(42_u32));
    ///
    /// assert!(state.contains::<u32>());
    ///
    /// let value = state.remove::<u32>();
    /// assert_eq!(*value.unwrap(), 42);
    ///
    /// assert!(!state.contains::<u32>());
    /// # }
    /// ```
    pub fn remove<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.data
            .remove(&TypeId::of::<T>())
            .and_then(|(_, arc)| arc.downcast::<T>().ok())
    }

    /// Returns the number of different types stored in the state.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use std::sync::Arc;
    ///
    /// # fn example() {
    /// let state = AppState::new();
    /// assert_eq!(state.len(), 0);
    ///
    /// state.insert(Arc::new(String::from("Hello")));
    /// assert_eq!(state.len(), 1);
    ///
    /// state.insert(Arc::new(42_u32));
    /// assert_eq!(state.len(), 2);
    ///
    /// // Replacing same type doesn't increase count
    /// state.insert(Arc::new(100_u32));
    /// assert_eq!(state.len(), 2);
    /// # }
    /// ```
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Checks if the state is empty (contains no data).
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use std::sync::Arc;
    ///
    /// # fn example() {
    /// let state = AppState::new();
    /// assert!(state.is_empty());
    ///
    /// state.insert(Arc::new(42_u32));
    /// assert!(!state.is_empty());
    /// # }
    /// ```
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Clears all state data.
    ///
    /// Removes all stored values, leaving the state empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use wsforge::prelude::*;
    /// use std::sync::Arc;
    ///
    /// # fn example() {
    /// let state = AppState::new();
    /// state.insert(Arc::new(String::from("Hello")));
    /// state.insert(Arc::new(42_u32));
    ///
    /// assert_eq!(state.len(), 2);
    ///
    /// state.clear();
    ///
    /// assert_eq!(state.len(), 0);
    /// assert!(state.is_empty());
    /// # }
    /// ```
    pub fn clear(&self) {
        self.data.clear();
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let state = AppState::new();
        state.insert(Arc::new(String::from("test")));

        let value: Option<Arc<String>> = state.get();
        assert_eq!(*value.unwrap(), "test");
    }

    #[test]
    fn test_multiple_types() {
        let state = AppState::new();
        state.insert(Arc::new(42_u32));
        state.insert(Arc::new(String::from("hello")));
        state.insert(Arc::new(true));

        assert_eq!(*state.get::<u32>().unwrap(), 42);
        assert_eq!(*state.get::<String>().unwrap(), "hello");
        assert_eq!(*state.get::<bool>().unwrap(), true);
    }

    #[test]
    fn test_get_nonexistent() {
        let state = AppState::new();
        let value: Option<Arc<String>> = state.get();
        assert!(value.is_none());
    }

    #[test]
    fn test_contains() {
        let state = AppState::new();
        assert!(!state.contains::<u32>());

        state.insert(Arc::new(42_u32));
        assert!(state.contains::<u32>());
    }

    #[test]
    fn test_remove() {
        let state = AppState::new();
        state.insert(Arc::new(42_u32));

        assert!(state.contains::<u32>());

        let removed = state.remove::<u32>();
        assert_eq!(*removed.unwrap(), 42);
        assert!(!state.contains::<u32>());
    }

    #[test]
    fn test_len_and_empty() {
        let state = AppState::new();
        assert!(state.is_empty());
        assert_eq!(state.len(), 0);

        state.insert(Arc::new(42_u32));
        assert!(!state.is_empty());
        assert_eq!(state.len(), 1);

        state.insert(Arc::new(String::from("test")));
        assert_eq!(state.len(), 2);
    }

    #[test]
    fn test_clear() {
        let state = AppState::new();
        state.insert(Arc::new(42_u32));
        state.insert(Arc::new(String::from("test")));

        assert_eq!(state.len(), 2);

        state.clear();

        assert_eq!(state.len(), 0);
        assert!(state.is_empty());
    }

    #[test]
    fn test_replace_value() {
        let state = AppState::new();
        state.insert(Arc::new(10_u32));
        assert_eq!(*state.get::<u32>().unwrap(), 10);

        state.insert(Arc::new(20_u32));
        assert_eq!(*state.get::<u32>().unwrap(), 20);
    }

    #[test]
    fn test_clone() {
        let state1 = AppState::new();
        state1.insert(Arc::new(42_u32));

        let state2 = state1.clone();
        assert_eq!(*state2.get::<u32>().unwrap(), 42);

        // Both share the same data
        state2.insert(Arc::new(100_u32));
        assert_eq!(*state1.get::<u32>().unwrap(), 100);
    }
}
