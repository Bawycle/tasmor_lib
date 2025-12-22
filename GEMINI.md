# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

TasmoR Lib is a Rust library to control [Tasmota](https://tasmota.github.io/docs/) devices over MQTT and HTTP.

## Development Workflow (TDD)

Follow this strict order for all changes:

1. **Write tests first** - Define expected behavior through unit tests
2. **Implement code** - Write the minimum code to make tests pass
3. **Verify** - Run the full verification pipeline before committing

## Build & Verification Commands

```bash
# Full verification pipeline (run all in order before committing)
cargo check && cargo build && cargo test && cargo fmt --check && cargo clippy -- -D warnings -W clippy::pedantic

# Individual commands
cargo check                                      # Type checking without building
cargo build                                      # Compile the library
cargo test                                       # Run all tests
cargo test <test_name>                           # Run a specific test
cargo test --lib                                 # Run only library tests
cargo test --doc                                 # Run documentation tests
cargo fmt                                        # Format code
cargo fmt --check                                # Check formatting without modifying
cargo clippy -- -D warnings -W clippy::pedantic # Lint with pedantic warnings as errors

# Documentation
cargo doc --no-deps --open                       # Generate and open documentation
cargo doc --no-deps --document-private-items     # Include private items in docs
```

## Clippy Configuration

Use pedantic warnings. A warning can only be suppressed with:
1. A `#[allow(...)]` attribute on the specific item
2. A comment explaining **why** the warning is acceptable

```rust
// Example: allowing a clippy warning with justification
#[allow(clippy::cast_possible_truncation)]
// Truncation is acceptable here because value is validated to be < 256
fn to_u8(value: u32) -> u8 {
    value as u8
}
```

## Code Documentation Standards

### Module-level documentation
Every module must have a `//!` doc comment explaining:
- Purpose of the module
- Main types and their relationships
- Usage examples with `# Examples` section

### Public API documentation
All public items (`pub`) require:
- A summary line (first line, concise)
- Detailed description if non-trivial
- `# Examples` section with runnable code
- `# Errors` section if the function returns `Result`
- `# Panics` section if the function can panic

### Documentation tests
All examples in documentation must be valid, runnable code tested by `cargo test --doc`.

```rust
/// Connects to a Tasmota device.
///
/// # Examples
///
/// ```
/// use tasmor_lib::Device;
///
/// let device = Device::new("192.168.1.100");
/// ```
///
/// # Errors
///
/// Returns `ConnectionError` if the device is unreachable.
pub fn connect(&self) -> Result<(), ConnectionError> {
    // ...
}
```

## Error Handling

- Use `thiserror` for library error types
- Each error variant must have a descriptive message
- Never use `.unwrap()` or `.expect()` on user-provided data
- Propagate errors with `?` operator, add context when helpful

## Architecture Guidelines

### Library structure
```
src/
├── lib.rs          # Public API exports and crate-level docs
├── error.rs        # Error types (using thiserror)
├── mqtt/           # MQTT protocol implementation
│   ├── mod.rs
│   └── ...
├── http/           # HTTP protocol implementation
│   ├── mod.rs
│   └── ...
└── device/         # Device abstractions
    ├── mod.rs
    └── ...
```

### Type design
- Use newtypes for domain values with constraints (IP addresses, ports, device IDs)
- Prefer `&str` over `String` in function parameters when ownership isn't needed
- Use builder pattern for types with many optional parameters

### Async considerations
- Use `async`/`await` for I/O operations (network calls)
- Provide both sync and async APIs if the library targets diverse use cases
- Document runtime requirements (tokio, async-std) clearly
