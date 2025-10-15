# Contributing to WsForge

Thank you for your interest in contributing to WsForge! 🎉 We welcome contributions from everyone, whether you're fixing a bug, adding a feature, improving documentation, or sharing ideas.

This guide will help you get started with contributing to the project.

## 📋 Table of Contents

- [Code of Conduct](#code-of-conduct)
- [How Can I Contribute?](#how-can-i-contribute)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Code Style Guidelines](#code-style-guidelines)
- [Testing](#testing)
- [Documentation](#documentation)
- [Pull Request Process](#pull-request-process)
- [Issue Guidelines](#issue-guidelines)
- [Community](#community)

## 🤝 Code of Conduct

This project adheres to a Code of Conduct that all contributors are expected to follow. Please be respectful and constructive in all interactions.

### Our Standards

- **Be welcoming**: Welcome newcomers and encourage diverse perspectives
- **Be respectful**: Respect differing viewpoints and experiences
- **Be constructive**: Provide actionable feedback and accept constructive criticism
- **Be patient**: Remember that everyone is learning

## 🎯 How Can I Contribute?

### Reporting Bugs

Found a bug? Please create an issue with:

- **Clear title**: Summarize the bug in the title
- **Description**: Detailed description of the problem
- **Steps to reproduce**: Minimal code example that reproduces the issue
- **Expected behavior**: What you expected to happen
- **Actual behavior**: What actually happened
- **Environment**: OS, Rust version, WsForge version

**Example Bug Report:**

```
**Title**: WebSocket connection fails when sending large messages

**Description**: When sending messages larger than 64KB, the connection closes unexpectedly.

**Steps to Reproduce**:
1. Create a WebSocket connection
2. Send a message larger than 64KB
3. Connection closes with error

**Expected**: Message should be sent successfully
**Actual**: Connection closes with "Message too large" error

**Environment**:
- OS: Ubuntu 22.04
- Rust: 1.75.0
- WsForge: 0.1.0
```

### Suggesting Features

Have an idea for a new feature? Create an issue with:

- **Use case**: Describe the problem you're trying to solve
- **Proposed solution**: Your suggested implementation
- **Alternatives**: Other approaches you've considered
- **Additional context**: Any relevant examples or references

### Improving Documentation

Documentation contributions are highly valued! You can:

- Fix typos or clarify existing documentation
- Add examples to existing documentation
- Write tutorials or guides
- Improve API documentation with better examples
- Add missing documentation for public APIs

## 🚀 Getting Started

### Prerequisites

- **Rust**: 1.70 or later (install via [rustup](https://rustup.rs/))
- **Git**: For version control
- **Editor**: VS Code with rust-analyzer recommended

### Setting Up Development Environment

1. **Fork the repository** on GitHub

2. **Clone your fork**:
   ```
   git clone https://github.com/YOUR_USERNAME/wsforge.git
   cd wsforge
   ```

3. **Add upstream remote**:
   ```
   git remote add upstream https://github.com/aarambhdevhub/wsforge.git
   ```

4. **Install development tools**:
   ```
   cargo install cargo-watch
   cargo install cargo-expand
   cargo install cargo-tarpaulin  # For code coverage
   ```

5. **Build the project**:
   ```
   cargo build --all
   ```

6. **Run tests**:
   ```
   cargo test --all
   ```

7. **Run examples**:
   ```
   cargo run --example echo
   cargo run --example chat
   cargo run --example chat-web
   ```

## 🔧 Development Workflow

### Creating a Branch

Always create a new branch for your work:

```
git checkout -b feature/your-feature-name
# or
git checkout -b fix/your-bug-fix
```

**Branch naming conventions**:
- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation improvements
- `refactor/` - Code refactoring
- `test/` - Test additions or improvements

### Making Changes

1. **Keep commits focused**: Each commit should be a logical unit of change
2. **Write clear commit messages**: Follow conventional commit format
3. **Test your changes**: Run tests and add new ones if needed
4. **Update documentation**: If you change APIs, update the docs

**Good commit messages**:
```
feat: add rate limiting middleware

Implements configurable rate limiting per connection with
token bucket algorithm. Includes tests and documentation.

Closes #123
```

```
fix: resolve connection leak on error

Ensures connections are properly cleaned up when handler
errors occur. Adds test case for error scenarios.

Fixes #456
```

### Keeping Your Fork Updated

Regularly sync with upstream:

```
git fetch upstream
git checkout main
git merge upstream/main
git push origin main
```

## 📝 Code Style Guidelines

### Rust Code Style

Follow the [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/) and use rustfmt:

```
cargo fmt --all
```

### Code Organization

- **Module organization**: Group related functionality in modules
- **File size**: Keep files under 500 lines when possible
- **Function size**: Prefer smaller, focused functions
- **Comments**: Use doc comments (`///`) for public APIs

### Documentation Comments

All public APIs must have documentation:

```
/// Sends a message to the connected client.
///
/// Messages are queued in an unbounded channel and sent asynchronously.
/// This method returns immediately without waiting for the message to be sent.
///
/// # Errors
///
/// Returns an error if the connection has been closed and the channel
/// receiver has been dropped.
///
/// # Examples
///
/// ```rust
/// use wsforge::prelude::*;
///
/// # async fn example(conn: Connection) -> Result<()> {
/// let msg = Message::text("Hello!");
/// conn.send(msg)?;
/// # Ok(())
/// # }
/// ```
pub fn send(&self, message: Message) -> Result<()> {
    // Implementation
}
```

### Error Handling

- Use `Result<T>` for fallible operations
- Create specific error types when needed
- Provide context in error messages

```
// Good
fn parse_config(path: &Path) -> Result<Config> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| Error::custom(format!("Failed to read config at {}: {}", path.display(), e)))?;
    // ...
}
```

## 🧪 Testing

### Running Tests

```
# Run all tests
cargo test --all

# Run tests for a specific package
cargo test -p wsforge-core

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run tests with coverage
cargo tarpaulin --all-features --workspace
```

### Writing Tests

#### Unit Tests

Place unit tests in the same file as the code:

```
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::text("hello");
        assert!(msg.is_text());
        assert_eq!(msg.as_text(), Some("hello"));
    }

    #[tokio::test]
    async fn test_async_handler() {
        let msg = Message::text("test");
        let result = my_handler(msg).await;
        assert!(result.is_ok());
    }
}
```

#### Integration Tests

Place integration tests in `tests/` directory:

```
// tests/websocket_integration.rs
use wsforge::prelude::*;

#[tokio::test]
async fn test_echo_server() {
    // Test implementation
}
```

### Test Guidelines

- **Test coverage**: Aim for >80% code coverage
- **Test all error paths**: Include error cases
- **Use descriptive names**: Test names should describe what they test
- **Keep tests focused**: One assertion per test when possible
- **Use test fixtures**: Share setup code with helper functions

## 📖 Documentation

### API Documentation

Use `cargo doc` to generate documentation:

```
# Generate and open documentation
cargo doc --all --no-deps --open

# Check for broken links
cargo doc --all --no-deps
```

### Documentation Requirements

- All public items must have documentation
- Include examples in documentation
- Explain parameters, return values, and errors
- Add usage examples for complex APIs

### Writing Good Examples

```
/// # Examples
///
/// Basic usage:
///
/// ```rust
/// use wsforge::prelude::*;
///
/// # fn example() {
/// let msg = Message::text("Hello");
/// # }
/// ```
///
/// With error handling:
///
/// ```rust
/// use wsforge::prelude::*;
///
/// # async fn example() -> Result<()> {
/// let msg = Message::text("Hello");
/// // Use the message
/// # Ok(())
/// # }
/// ```
```

## 🔄 Pull Request Process

### Before Submitting

1. **Update your branch**:
   ```
   git fetch upstream
   git rebase upstream/main
   ```

2. **Run the full test suite**:
   ```
   cargo test --all
   cargo clippy --all -- -D warnings
   cargo fmt --all -- --check
   ```

3. **Update documentation**:
   ```
   cargo doc --all --no-deps
   ```

4. **Update CHANGELOG.md** if applicable

### Submitting a Pull Request

1. **Push your branch**:
   ```
   git push origin feature/your-feature
   ```

2. **Create PR on GitHub** with:
   - Clear title describing the change
   - Description of what changed and why
   - Link to related issues (e.g., "Fixes #123")
   - Screenshots for UI changes
   - Breaking changes noted clearly

3. **PR Template**:

```
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Tests added/updated
- [ ] All tests passing
- [ ] Documentation updated

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Comments added for complex code
- [ ] Documentation updated
- [ ] No new warnings
- [ ] Tests pass locally

## Related Issues
Fixes #123
```

### Review Process

- Maintainers will review your PR
- Address feedback by pushing new commits
- Once approved, a maintainer will merge your PR
- PRs may be squashed when merged

## 📋 Issue Guidelines

### Issue Templates

Use the provided issue templates when creating issues:

- **Bug Report**: For reporting bugs
- **Feature Request**: For suggesting new features
- **Documentation**: For documentation improvements
- **Question**: For asking questions

### Issue Labels

Issues are labeled to help with organization:

- `good first issue` - Good for newcomers
- `help wanted` - Extra attention needed
- `bug` - Something isn't working
- `enhancement` - New feature or request
- `documentation` - Documentation improvements
- `performance` - Performance improvements
- `breaking change` - Breaking API changes

## 💬 Community

### Getting Help

- **GitHub Discussions**: Ask questions and discuss ideas
- **GitHub Issues**: Report bugs and request features
- **YouTube**: [@AarambhDevHub](https://youtube.com/@AarambhDevHub) for tutorials

### Communication Channels

- **GitHub**: Primary platform for code and issues
- **YouTube**: Video tutorials and updates
- **Email**: For private inquiries

## 🎓 Learning Resources

### Understanding the Codebase

1. **Start with examples**: Look at `examples/` directory
2. **Read the architecture docs**: Understand the design
3. **Explore tests**: Tests show how things work
4. **Use cargo-expand**: See generated macro code

### Rust Resources

- [The Rust Book](https://doc.rust-lang.org/book/)
- [Async Book](https://rust-lang.github.io/async-book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)

## 🏆 Recognition

Contributors are recognized in:
- `CONTRIBUTORS.md` file
- Release notes
- Project documentation

Thank you for contributing to WsForge! Your efforts help make real-time Rust development better for everyone. 🚀

---

**Questions?** Feel free to open an issue or reach out to [@AarambhDevHub](https://github.com/AarambhDevHub)
