# WsForge

High-performance WebSocket framework for Rust.

## Overview

This is the main WsForge crate that re-exports functionality from `wsforge-core` and `wsforge-macros`. Use this crate in your applications.

## Installation

```
[dependencies]
wsforge = "0.1.0"
tokio = { version = "1.40", features = ["full"] }
```

## Quick Example

```
use wsforge::prelude::*;

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

## Features

- `macros` (default) - Procedural macros for convenience
- `full` - All features enabled

## Documentation

- [Main Documentation](https://docs.rs/wsforge)
- [GitHub Repository](https://github.com/aarambhdevhub/wsforge)
- [Examples](../examples)

## License

MIT License - See [LICENSE](../LICENSE) for details.
