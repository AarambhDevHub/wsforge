# Installation Guide

This guide covers everything you need to install and set up WsForge for your WebSocket projects.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Installing Rust](#installing-rust)
- [Adding WsForge to Your Project](#adding-wsforge-to-your-project)
- [Features](#features)
- [Platform-Specific Instructions](#platform-specific-instructions)
- [Development Tools](#development-tools)
- [Verification](#verification)
- [IDE Setup](#ide-setup)
- [Troubleshooting](#troubleshooting)

## Prerequisites

Before installing WsForge, ensure you have:

- **Rust**: Version 1.70 or later (recommended: latest stable)
- **Operating System**: Windows, Linux, or macOS
- **Internet Connection**: For downloading dependencies

## Installing Rust

If you don't have Rust installed, use rustup (the official Rust installer):

### Unix/Linux/macOS

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Follow the on-screen instructions. After installation, reload your shell:

```
source $HOME/.cargo/env
```

### Windows

Download and run [rustup-init.exe](https://rustup.rs/) from the official website.

### Verify Rust Installation

```
rustc --version
cargo --version
```

You should see version information for both commands.

### Update Rust

Keep Rust up to date:

```
rustup update
```

## Adding WsForge to Your Project

### Creating a New Project

```
cargo new my-websocket-app
cd my-websocket-app
```

### Basic Dependencies

Add WsForge to your `Cargo.toml`:

```
[dependencies]
wsforge = "0.1.0"
tokio = { version = "1.40", features = ["full"] }
```

### With JSON Support

For JSON message handling:

```
[dependencies]
wsforge = "0.1.0"
tokio = { version = "1.40", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Complete Setup

Recommended full setup with logging:

```
[dependencies]
wsforge = "0.1.0"
tokio = { version = "1.40", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
```

## Features

WsForge supports optional features for flexibility:

### Available Features

| Feature | Description | Default |
|---------|-------------|---------|
| `macros` | Procedural macros for convenience | ✅ Yes |
| `full` | All features enabled | ❌ No |

### Enabling Features

#### Default (with macros)

```
[dependencies]
wsforge = "0.1.0"
```

#### All Features

```
[dependencies]
wsforge = { version = "0.1.0", features = ["full"] }
```

#### No Macros

```
[dependencies]
wsforge = { version = "0.1.0", default-features = false }
```

#### Custom Features

```
[dependencies]
wsforge = { version = "0.1.0", features = ["macros"] }
```

## Platform-Specific Instructions

### Linux

#### Debian/Ubuntu

Install OpenSSL development libraries:

```
sudo apt-get update
sudo apt-get install -y libssl-dev pkg-config build-essential
```

#### Fedora/RHEL/CentOS

```
sudo dnf install -y openssl-devel gcc
```

#### Arch Linux

```
sudo pacman -S openssl pkg-config
```

### Windows

No additional system dependencies required. However, you may need:

1. **Visual Studio Build Tools**: Download from [Microsoft](https://visualstudio.microsoft.com/downloads/)
   - Select "Desktop development with C++"

2. **Alternative**: Install [MSYS2](https://www.msys2.org/) for MinGW toolchain

### macOS

No additional dependencies required. Xcode Command Line Tools should be sufficient:

```
xcode-select --install
```

## Development Tools

### Essential Tools

Install helpful development tools:

```
# Code formatting
rustup component add rustfmt

# Linting
rustup component add clippy

# Watch for file changes and auto-rebuild
cargo install cargo-watch

# View expanded macros
cargo install cargo-expand

# Code coverage
cargo install cargo-tarpaulin
```

### Optional Tools

```
# Better error messages
cargo install cargo-nextest

# Dependency graph visualization
cargo install cargo-deps

# Audit dependencies for security issues
cargo install cargo-audit
```

## Verification

### Test Installation

Create a minimal test file `src/main.rs`:

```
use wsforge::prelude::*;

async fn echo(msg: Message) -> Result<Message> {
    Ok(msg)
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("✅ WsForge installed successfully!");

    let router = Router::new()
        .default_handler(handler(echo));

    println!("🚀 Server ready on ws://127.0.0.1:8080");
    router.listen("127.0.0.1:8080").await?;
    Ok(())
}
```

### Build the Project

```
cargo build
```

If successful, you should see:

```
Compiling wsforge v0.1.0
Compiling my-websocket-app v0.1.0
Finished dev [unoptimized + debuginfo] target(s) in X.XXs
```

### Run the Project

```
cargo run
```

You should see:

```
✅ WsForge installed successfully!
🚀 Server ready on ws://127.0.0.1:8080
```

## IDE Setup

### Visual Studio Code

Recommended extensions:

1. **rust-analyzer** (rust-lang.rust-analyzer)
   - Install: `code --install-extension rust-lang.rust-analyzer`
   - Provides code completion, go-to-definition, and more

2. **CodeLLDB** (vadimcn.vscode-lldb)
   - Install: `code --install-extension vadimcn.vscode-lldb`
   - Debugging support

3. **crates** (serayuzgur.crates)
   - Install: `code --install-extension serayuzgur.crates`
   - Manage dependencies

4. **Even Better TOML** (tamasfe.even-better-toml)
   - Install: `code --install-extension tamasfe.even-better-toml`
   - TOML syntax support

#### VS Code Settings

Create `.vscode/settings.json`:

```
{
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.checkOnSave.command": "clippy",
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

### IntelliJ IDEA / CLion

Install the **Rust plugin**:
1. Go to Settings → Plugins
2. Search for "Rust"
3. Install and restart

### Vim/Neovim

Use **rust.vim** and **coc.nvim** with coc-rust-analyzer:

```
Plug 'rust-lang/rust.vim'
Plug 'neoclide/coc.nvim', {'branch': 'release'}
```

### Emacs

Use **rust-mode** and **lsp-mode**:

```
(use-package rust-mode
  :ensure t
  :hook (rust-mode . lsp))
```

## Troubleshooting

### Common Issues

#### Issue: "error: linker `cc` not found"

**Solution**: Install a C compiler:

- **Linux**: `sudo apt-get install build-essential`
- **macOS**: `xcode-select --install`
- **Windows**: Install Visual Studio Build Tools

#### Issue: "could not find `Cargo.toml`"

**Solution**: Make sure you're in the project directory:

```
cd my-websocket-app
cargo build
```

#### Issue: OpenSSL errors on Linux

**Solution**: Install OpenSSL development libraries:

```
sudo apt-get install libssl-dev pkg-config
```

#### Issue: Slow compilation times

**Solution**: Use a faster linker:

1. Install `lld` or `mold`:
   ```
   sudo apt-get install lld  # or mold
   ```

2. Create `.cargo/config.toml`:
   ```
   [target.x86_64-unknown-linux-gnu]
   linker = "clang"
   rustflags = ["-C", "link-arg=-fuse-ld=lld"]
   ```

#### Issue: "feature `macros` not found"

**Solution**: Ensure your `Cargo.toml` includes:

```
[dependencies]
wsforge = { version = "0.1.0", features = ["macros"] }
```

### Getting Help

If you encounter issues:

1. **Check documentation**: [docs/](.)
2. **Search issues**: [GitHub Issues](https://github.com/aarambhdevhub/wsforge/issues)
3. **Ask for help**: Open a new issue with:
   - Your OS and Rust version (`rustc --version`)
   - Error messages (full output)
   - Steps to reproduce

## Next Steps

Now that WsForge is installed:

1. ✅ [Quick Start Guide](quick-start.md) - Build your first server
2. ✅ [Handlers Documentation](handlers.md) - Learn about handler functions
3. ✅ [Examples](examples.md) - Explore working examples

## Version Information

- **Current Version**: 0.1.0
- **Minimum Rust**: 1.70
- **Recommended Rust**: Latest stable
- **License**: MIT

---

**Need help?** Open an issue on [GitHub](https://github.com/aarambhdevhub/wsforge/issues) or check the [FAQ](faq.md).
