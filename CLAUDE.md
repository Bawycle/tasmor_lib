# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

TasmoR Lib is a Rust library for controlling Tasmota devices via MQTT and HTTP protocols. It provides a type-safe async API built on Tokio. Current version: 0.6.0 (pre-1.0, API may change).

## Common Commands

```bash
# Full verification pipeline (run before committing)
cargo check && cargo build && cargo test && cargo fmt --check && cargo clippy -- -D warnings -W clippy::pedantic

# Run a single test
cargo test test_name

# Run tests for a specific module
cargo test module_name::

# Test with only one protocol feature
cargo test --no-default-features --features http
cargo test --no-default-features --features mqtt

# Documentation tests only
cargo test --doc

# Generate docs
cargo doc --no-deps --open
```

## Architecture

### Protocol Layer (`src/protocol/`)

Two protocol backends behind a unified `Device` API:
- **HTTP** (`http.rs`): Stateless request/response via reqwest. No event subscriptions.
- **MQTT** (`mqtt_broker.rs`, `shared_mqtt_client.rs`, `topic_router.rs`, `response_collector.rs`): Persistent pub/sub via rumqttc. Supports real-time event subscriptions. Multiple devices share a single broker connection.

Protocol choice is encoded in the type system — calling subscription methods on HTTP devices is a compile-time error.

### Device Layer (`src/device/`)

`Device<P>` is generic over protocol. Builders:
- `Device::http(addr)` → `HttpBuilder` → `Device<HttpClient>`
- `broker.device(topic)` → `BrokerDeviceBuilder` → `Device<SharedMqttClient>`

Both builders support `build()` (auto-detect capabilities by querying the device) or `build_without_probe()` (user provides capabilities). Both return `(Device, DeviceState)`.

### Command Layer (`src/command/`)

Command modules (power, light, energy, scheme, status, routine) define the Tasmota commands. `Routine` uses `Backlog0` to execute multiple commands atomically.

### Type System (`src/types/`)

Newtypes with validation at construction (Parse Don't Validate): `Dimmer`, `ColorTemperature`, `HsbColor`, `PowerIndex`, `RgbColor`, etc. Invalid states are unrepresentable.

### Response Layer (`src/response/`)

Typed response structs returned by device command methods — parsed from Tasmota JSON responses.

### State & Subscriptions

- `src/state/`: `DeviceState` tracks current device state; `StateChange` represents diffs.
- `src/subscription/`: `Subscribable` trait (MQTT-only) for real-time callbacks on state changes.
- `src/telemetry/`: Parsing of raw Tasmota MQTT telemetry messages.

### Capabilities (`src/capabilities.rs`)

Describes what a device supports (power channels, dimmer, CCT, RGB, energy monitoring). Predefined profiles: `basic()`, `neo_coolcam()`, `rgbcct_light()`, etc. Custom via `CapabilitiesBuilder`.

## Key Design Decisions

- **Feature flags**: `http` and `mqtt` are both default-enabled. Code is conditionally compiled with `#[cfg(feature = "...")]`.
- **unsafe_code = "forbid"** at crate level.
- **clippy::pedantic** is enabled as a warning in `Cargo.toml` lints section.
- **Rust edition 2024**, MSRV 1.92.0 (pinned via `rust-toolchain.toml`).
- Tests use `wiremock` for HTTP mocking and `approx` for float comparisons.

## Code Quality Policy

- Any technical debt spotted during a read or edit — even if unrelated to the task — must be flagged explicitly with its nature and potential impact.
- No deliberate shortcuts or "we'll fix it later" decisions. If a cleaner approach exists, it is the only acceptable approach.

## Git Workflow

- **Branches**: `master` (release), `dev` (development). Feature branches from `dev`.
- **Commits**: Conventional Commits format (`feat(scope):`, `fix(scope):`, etc.)
- **Branch naming**: `feat/`, `fix/`, `docs/`, `refactor/`
