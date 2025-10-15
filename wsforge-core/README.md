# WsForge Core

The foundational library for the WsForge WebSocket framework.

## Overview

`wsforge-core` contains the core implementation of WsForge, including:

- Connection management and lifecycle
- Message handling and routing
- Type-safe extractors
- Handler traits and implementations
- State management
- Error types
- Static file serving

## Usage

**Note**: Most users should use the main [`wsforge`](../wsforge) crate instead, which re-exports everything from `wsforge-core` and includes additional conveniences.

### Direct Usage (Advanced)

If you need only the core functionality without macros:

```
[dependencies]
wsforge-core = "0.1.0"
tokio = { version = "1.40", features = ["full"] }
```

```rust
use wsforge_core::prelude::*;

async fn echo(msg: Message) -> Result<Message> {
    Ok(msg)
}

#[tokio::main]
async fn main() -> Result<()> {
    Router::new()
        .default_handler(handler(echo))
        .listen("127.0.0.1:8080")
        .await
}
```

## Documentation

- [Main WsForge Documentation](../README.md)
- [API Reference](https://docs.rs/wsforge-core)
- [Examples](../examples)

## Components

- **connection**: WebSocket connection management
- **message**: Message types and parsing
- **handler**: Handler traits and response types
- **extractor**: Type-safe data extraction
- **router**: Routing and server management
- **state**: Shared application state
- **error**: Error types and handling
- **static_files**: Static file serving

## License

MIT - See [LICENSE](../LICENSE) for details.

## Related Crates

- [`wsforge`](../wsforge) - Main crate (recommended for most users)
- [`wsforge-macros`](../wsforge-macros) - Procedural macros
