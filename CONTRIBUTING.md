# Contributing to TasmoR Lib

Thank you for your interest in contributing to TasmoR Lib! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Design Principles](#design-principles)
- [Code Style](#code-style)
- [Testing](#testing)
- [Documentation](#documentation)
- [Commit Messages](#commit-messages)
- [Pull Requests](#pull-requests)
- [License](#license)

## Code of Conduct

Please be respectful and constructive in all interactions. We welcome contributors of all experience levels.

## Getting Started

### Prerequisites

- Rust 1.92.0 or later (enforced by `rust-toolchain.toml`)
- A Tasmota device for integration testing (optional but recommended)

### Setup

1. Clone the repository:
   ```bash
   git clone https://codeberg.org/Bawycle/tasmor_lib.git
   cd tasmor_lib
   ```

2. The correct Rust version will be installed automatically via `rust-toolchain.toml`.

3. Build the project:
   ```bash
   cargo build
   ```

4. Run tests:
   ```bash
   cargo test
   ```

## Development Workflow

We follow Test-Driven Development (TDD):

1. **Write tests first** - Define expected behavior through tests
2. **Implement code** - Write the minimum code to make tests pass
3. **Verify** - Run the full verification pipeline before committing

### Verification Pipeline

Always run the complete verification before submitting changes:

```bash
cargo check && cargo build && cargo test && cargo fmt --check && cargo clippy -- -D warnings -W clippy::pedantic
```

Or individually:

```bash
cargo check                                       # Type checking
cargo build                                       # Compile
cargo test                                        # Run all tests
cargo test --doc                                  # Run documentation tests
cargo fmt --check                                 # Check formatting
cargo clippy -- -D warnings -W clippy::pedantic  # Lint with pedantic warnings
```

## Design Principles

This project follows specific architectural principles. Please adhere to these when contributing.

### Core Principles

| Principle | Description |
|-----------|-------------|
| **Single Source of Truth** | Each piece of data has one authoritative source |
| **DRY** | Don't Repeat Yourself - factor out common logic |
| **KISS** | Keep It Simple - prefer simplicity over complexity |
| **YAGNI** | You Aren't Gonna Need It - don't implement speculative features |

### Newtype Pattern

Use newtypes for domain values with constraints. This provides compile-time guarantees and self-documenting code.

```rust
/// Dimmer level (0-100).
pub struct Dimmer(u8);

impl Dimmer {
    pub fn new(value: u8) -> Result<Self, ValueError> {
        if value > 100 {
            return Err(ValueError::out_of_range("Dimmer", 0, 100, value));
        }
        Ok(Self(value))
    }

    pub const fn value(self) -> u8 {
        self.0
    }
}
```

**Benefits:**
- Invalid states are unrepresentable
- Type system prevents mixing different value types
- Validation happens once at construction

### Parse, Don't Validate

Convert data to valid types at system boundaries. Once parsed, the type guarantees validity.

```rust
// Good: Parse once, use safely everywhere
let dimmer = Dimmer::new(user_input)?;  // Validates here
device.set_dimmer(dimmer).await?;        // No validation needed

// Bad: Validate repeatedly
fn set_dimmer(value: u8) {
    assert!(value <= 100);  // Must validate every time
}
```

### Error Handling

- Use `Result<T, E>` for operations that can fail
- Use `Option<T>` for optional values
- Never use `.unwrap()` or `.expect()` on user-provided data
- Propagate errors with `?`, add context when helpful

```rust
// Good
pub fn parse_color(hex: &str) -> Result<RgbColor, ValueError> {
    let r = u8::from_str_radix(&hex[1..3], 16)
        .map_err(|_| ValueError::invalid_format("hex color", hex))?;
    // ...
}

// Bad
pub fn parse_color(hex: &str) -> RgbColor {
    let r = u8::from_str_radix(&hex[1..3], 16).unwrap();  // Panics on invalid input
    // ...
}
```

### Immutability

Prefer immutable data structures. Use mutation only when necessary for performance.

```rust
// Good: Methods return new values
impl Dimmer {
    #[must_use]
    pub fn with_value(self, value: u8) -> Result<Self, ValueError> {
        Self::new(value)
    }
}

// Acceptable: Mutation for state tracking
impl DeviceState {
    pub fn set_dimmer(&mut self, dimmer: Dimmer) {
        self.dimmer = Some(dimmer);
    }
}
```

### Avoid Over-Engineering

- Only make changes that are directly requested or clearly necessary
- Don't add features, refactor code, or make "improvements" beyond what was asked
- Don't add error handling for scenarios that can't happen
- Don't create abstractions for one-time operations

## Code Style

### Formatting

- Use `cargo fmt` to format code
- Run `cargo fmt --check` to verify formatting without modifying files

### Linting

We use Clippy with pedantic warnings as errors:

```bash
cargo clippy -- -D warnings -W clippy::pedantic
```

If you need to suppress a warning, use an `#[allow(...)]` attribute with a comment explaining why:

```rust
#[allow(clippy::cast_possible_truncation)]
// Truncation is acceptable here because value is validated to be < 256
fn to_u8(value: u32) -> u8 {
    value as u8
}
```

### General Guidelines

- **No unsafe code** - The crate forbids unsafe code
- **Error handling** - Use `thiserror` for error types, propagate with `?`
- **No `.unwrap()` on user data** - Always handle errors gracefully
- **Prefer `&str` over `String`** - When ownership isn't needed
- **Use newtypes** - For domain values with constraints (e.g., `Dimmer`, `PowerIndex`)

## Testing

### Test Coverage

We aim for high test coverage. Check coverage with:

```bash
cargo tarpaulin
```

### Recommended Tools

These tools are not installed by `rust-toolchain.toml` but are recommended for contributors:

```bash
# Code coverage
cargo install cargo-tarpaulin
cargo tarpaulin

# Security audit
cargo install cargo-audit
cargo audit
```

### Test Organization

- **Unit tests** - In the same file as the code, in a `tests` module
- **Integration tests** - In `tests/` directory
- **Documentation tests** - Examples in doc comments

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptive_test_name() {
        // Arrange
        let input = ...;

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected);
    }
}
```

## Documentation

### Requirements

All public items must have documentation:

- **Summary line** - First line, concise description
- **Detailed description** - If non-trivial
- **`# Examples`** - Runnable code examples
- **`# Errors`** - If the function returns `Result`
- **`# Panics`** - If the function can panic

### Example

```rust
/// Connects to a Tasmota device.
///
/// Establishes an HTTP connection to the device at the specified address
/// and retrieves its current state.
///
/// # Examples
///
/// ```
/// use tasmor_lib::Device;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let (device, state) = Device::http("192.168.1.100")
///     .build_with_probe()
///     .await?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns `DeviceError::Connection` if the device is unreachable.
pub async fn build_with_probe(self) -> Result<(Device, DeviceState), DeviceError> {
    // ...
}
```

### Building Documentation

```bash
cargo doc --no-deps --open                    # Generate and open docs
cargo doc --no-deps --document-private-items  # Include private items
```

## Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

### Types

- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation changes
- `refactor` - Code refactoring (no feature/fix)
- `test` - Adding or updating tests
- `chore` - Maintenance tasks

### Examples

```
feat(mqtt): add subscription support for power state changes

fix(http): handle timeout when device is unreachable

docs(readme): add MQTT connection examples

refactor(types): extract color validation to separate module
```

## Pull Requests

### Before Submitting

1. Ensure all tests pass: `cargo test`
2. Ensure no linting errors: `cargo clippy -- -D warnings -W clippy::pedantic`
3. Ensure code is formatted: `cargo fmt --check`
4. Update documentation if needed
5. Add tests for new functionality

### PR Process

1. Fork the repository
2. Create a feature branch from `dev`: `git checkout -b feat/my-feature dev`
3. Make your changes following the guidelines above
4. Push to your fork
5. Open a Pull Request against the `dev` branch
6. Respond to review feedback

### Branch Naming

- `feat/description` - New features
- `fix/description` - Bug fixes
- `docs/description` - Documentation
- `refactor/description` - Refactoring

## License

By contributing, you agree that your contributions will be licensed under the MPL-2.0 license.
