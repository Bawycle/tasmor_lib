# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.0] - 2026-06-05

### Added

- `MqttBrokerBuilder::tls_ca_cert(path)` — enables TLS for the broker connection with
  mandatory CA certificate verification (PEM format). Server certificate verification is
  always enforced; no insecure mode is provided by design. The file is read
  by the paho-mqtt C library at connection time on its own thread — no file I/O occurs on
  the calling async runtime.

### Changed

- **BREAKING**: `ProtocolError::Http` and `ProtocolError::Mqtt` now wrap `String` instead
  of `reqwest::Error` / `paho_mqtt::Error`, removing backend crate types from the public
  API surface. `From<reqwest::Error>` and `From<paho_mqtt::Error>` impls are still provided
  for seamless `?` propagation. Code that pattern-matched on the inner error type (e.g.
  `ProtocolError::Mqtt(paho_err)`) must be updated.

### Fixed

- IPv6 addresses are now correctly wrapped in brackets in MQTT broker URIs
  (e.g. `tcp://[::1]:1883` instead of the invalid `tcp://::1:1883`).

## [0.8.0] - 2026-06-04

### Security

- **Credential zeroization (CWE-316)** — `MqttBrokerConfig`, `HttpConfig`, `DiscoveryOptions`, `HttpClientBuilder`, and `HttpClient` now store credentials in zeroized allocations that overwrite heap memory on `Drop`. `Debug` output of these structs now prints `[REDACTED]` for credentials instead of the plaintext value. Residual: `paho-mqtt` and `reqwest` hold their own copies for the connection lifetime, outside Rust's control — use TLS to protect credentials in transit.
- **HTTP debug log no longer includes credentials** — The per-command debug trace previously logged the full request URL, which includes `user=` and `password=` query parameters. It now logs only the command name.

## [0.7.0] - 2026-05-27

### Changed

- **BREAKING: `ProtocolError::Mqtt` inner type changed** — Now wraps `paho_mqtt::Error` instead of `rumqttc::ClientError`. Affects only code that pattern-matches on this variant and uses the inner error type directly.
- **MQTT backend replaced: rumqttc → paho-mqtt 0.14** — New system requirements: `cmake` at build time, `libssl`/`libcrypto` (OpenSSL) at runtime. See README for details.

## [0.6.0] - 2026-04-20

### Added

- **`frequency` field on energy types** — `EnergyData` (HTTP response), `StateChange::Energy`, `DeviceState`, and the subscription `EnergyData` callback struct now all carry `frequency: Option<f32>` (Hz). The field is `None` for DC monitors and devices that do not report it. Fully propagated through the MQTT telemetry pipeline (`EnergyReading` → `StateChange::Energy` → subscriber callbacks). New convenience accessors: `EnergyResponse::frequency()` and `DeviceState::frequency()` / `set_frequency()`.

### Fixed

- **BREAKING: Energy power and voltage fields are now `f32`** — `EnergyData` and `EnergyReading` fields `power`, `apparent_power`, `reactive_power` (previously `u32`) and `voltage` (previously `u16`) are now `f32`. Tasmota devices configured with `WattRes`/`VoltRes` > 0 report these fields as floats; the previous integer types caused serde deserialization failures. Update any code that assigned these fields to integer variables or cast them explicitly.

## [0.5.0] - 2026-01-09

### Changed

- **BREAKING: Uptime now returns `Duration`** - `SystemInfo::uptime()`, `DeviceState::uptime()`, and `TelemetryState::uptime()` now return `Option<Duration>` instead of `Option<u64>`. Use `.as_secs()` if you need the raw seconds value
- **BREAKING: `Command` trait extended** - Added `response_spec()` method. Existing implementations remain compatible thanks to the default implementation
- **BREAKING: `WakeupDuration::new()` now takes `Duration`** - Use `WakeupDuration::new(Duration::from_secs(300))` instead of `WakeupDuration::new(300)`. Range: 1-3000 seconds
- **BREAKING: `FadeSpeed` renamed to `FadeDuration`** - Also renamed: `FadeSpeedCommand` → `FadeDurationCommand`, `FadeSpeedResponse` → `FadeDurationResponse`, `Device::set_fade_speed()` → `set_fade_duration()`, `Device::get_fade_speed()` → `get_fade_duration()`. Now takes `Duration` (range: 0.5-20 seconds)

### Removed

- **BREAKING: `uptime_seconds()` and `uptime_string()`** - Use `uptime()` instead, which returns `Option<Duration>`
- **BREAKING: `WakeupDuration::from_minutes()`** - Use `WakeupDuration::new(Duration::from_secs(minutes * 60))` instead
- **BREAKING: `FadeSpeed::FAST`, `MEDIUM`, `SLOW` constants** - Use `FadeDuration::new(Duration::...)` instead

### Fixed

- **MQTT status queries now return complete data** - `query_state()` and `build()` via MQTT now return the same complete device information as HTTP, including uptime. Previously some fields were missing when using MQTT

## [0.4.1] - 2026-01-08

### Fixed

- **Device<SharedMqttClient> now implements Clone and Debug** - Fixed `#[derive(Clone, Debug)]` adding unnecessary `P: Clone + Debug` bounds. Manual implementations now correctly allow `Device<P>` to be Clone/Debug for any `P: Protocol`, matching the documented behavior

## [0.4.0] - 2026-01-08

### Added

- **Device is now Clone** - `Device<P>` implements `Clone`, enabling easy sharing across async tasks. Clones share the same connection and callbacks (via `Arc`), following the pattern of `reqwest::Client` and `rumqttc::AsyncClient`
- **System info in DeviceState** - New `SystemInfo` struct provides access to device diagnostics (uptime, Wi-Fi RSSI, heap memory). Available via `DeviceState::system_info()` and convenience method `DeviceState::uptime_seconds()`. System info is populated from `Status 0` during `query_state()` (heap, rssi) and from MQTT telemetry via `TelemetryState::to_system_info()` (uptime, rssi)
- **MQTT command timeout** - New `MqttBrokerBuilder::command_timeout()` configures the timeout for waiting on command responses (default: 5 seconds). Useful for slow-responding devices or routines with delays. Consistent with HTTP's `HttpConfig::with_timeout()`

### Changed

- **BREAKING: Renamed callback** - `on_energy_updated()` renamed to `on_energy_changed()` for API consistency with other callbacks (`on_power_changed`, `on_dimmer_changed`, etc.)
- **BREAKING: Removed `uptime_sec()`** - Use `uptime_seconds()` instead for consistency with `TelemetryState`

### Changed

- **Enhanced documentation** - Added `# Examples` sections to main Device methods (`power_on`, `power_off`, `power_toggle`, `set_dimmer`, `energy`)
- **Better error documentation** - Enriched `# Errors` sections with specific error conditions and types
- **Type cross-references** - Type modules now link to relevant Device methods (e.g., `Dimmer` → `set_dimmer()`)
- **API pattern documentation** - Documented `query_state()` vs `get_*` methods usage pattern

## [0.3.0] - 2025-12-31

### Added

- **MQTT reconnection handling** - Automatic topic resubscription when broker connection is restored. New `on_reconnected()` callback notifies applications when reconnection occurs

### Fixed

- **MQTT connection resilience** - Event loop no longer terminates on connection errors, allowing rumqttc to automatically reconnect

## [0.2.1] - 2025-12-29

### Fixed

- **Documentation** - Fixed outdated version references in README examples

## [0.2.0] - 2025-12-29

### Added

- **Command routines** - Execute multiple commands as a single atomic operation with optional delays between steps (max 30 steps). Supports power, lighting, fade, and scheme commands
- **MQTT device discovery** - Automatically discover all Tasmota devices connected to an MQTT broker
- **Device disconnect** - Properly close device connections to release resources
- **Fade state tracking** - Initial device state now includes fade enabled/disabled status and fade duration for light devices

### Changed

- **BREAKING: Simplified MQTT API** - Use `MqttBroker` to connect to a broker, then create devices with `broker.device()`. The previous `Device::mqtt()` method has been removed:
  ```rust
  // Before (removed):
  // let (device, _) = Device::mqtt("mqtt://broker:1883", "topic").build().await?;

  // After:
  let broker = MqttBroker::builder().host("192.168.1.50").build().await?;
  let (device, _) = broker.device("topic").build().await?;

  // Clean disconnect when done
  device.disconnect().await;
  broker.disconnect().await?;
  ```
- **Streamlined exports** - Reduced public API surface; internal types moved to submodules (e.g., `command::PowerCommand` instead of root export)

### Fixed

- **MQTT command responses** - Commands now reliably receive their correct response, even after executing routines with delays
- **MQTT capability detection** - Device capabilities (dimmer, color, energy monitoring) are now correctly detected for MQTT devices
- **Status parsing** - Fixed parsing of timezone and wakeup duration fields for compatibility with various Tasmota firmware versions

## [0.1.0] - 2025-12-27

### Added

- **Core types**
  - `PowerState`, `PowerIndex` for relay control
  - `Dimmer` (0-100) for brightness control
  - `HsbColor` for HSB color control (hue 0-360, saturation 0-100, brightness 0-100)
  - `RgbColor` for RGB color with hex parsing (#RRGGBB)
  - `ColorTemperature` for CCT control (153-500 mireds)
  - `Scheme` for light effects (0-4: Single, Wakeup, Cycle Up, Cycle Down, Random)
  - `WakeupDuration` for wakeup effect timing (1-3000 seconds)
  - `FadeSpeed` for transition speed control (1-40)
  - `TasmotaDateTime` for timestamp parsing with timezone support

- **Device control**
  - HTTP protocol support with async/await
  - MQTT protocol support with shared broker connections
  - Power control: `power_on()`, `power_off()`, `power_toggle()`, `set_power()`
  - Light control: `set_dimmer()`, `set_hsb_color()`, `set_rgb_color()`, `set_color_temperature()`
  - Scheme control: `set_scheme()`, `get_scheme()`, `set_wakeup_duration()`, `get_wakeup_duration()`
  - Fade control: `enable_fade()`, `disable_fade()`, `set_fade_speed()`
  - Energy monitoring: `get_energy()`, `reset_energy_total()`
  - Status queries: `get_status()`, `get_firmware_info()`, `get_network_info()`

- **State management**
  - `DeviceState` for tracking device state
  - `StateChange` enum for state updates
  - State is automatically updated from command responses

- **MQTT subscriptions**
  - `on_power_changed()` - Power state callbacks
  - `on_dimmer_changed()` - Dimmer level callbacks
  - `on_hsb_color_changed()` - Color change callbacks
  - `on_color_temperature_changed()` - CT change callbacks
  - `on_scheme_changed()` - Scheme change callbacks
  - `on_connected()` / `on_disconnected()` - Connection status
  - `on_state_changed()` - Generic state change callbacks

- **Telemetry parsing**
  - Parse `tele/<topic>/STATE` messages
  - Parse `tele/<topic>/SENSOR` messages (energy data)
  - Parse `tele/<topic>/LWT` messages (online/offline)

- **Capabilities system**
  - `Capabilities` for describing device features
  - `CapabilitiesBuilder` for custom capability sets
  - Predefined profiles: `basic()`, `neo_coolcam()`, `rgbcct_light()`, `rgb_light()`, `cct_light()`
  - Auto-detection from device status response

- **Feature flags**
  - `http` - Enable HTTP protocol (default)
  - `mqtt` - Enable MQTT protocol (default)

- **Documentation**
  - Full API documentation with examples
  - README with usage examples
  - CONTRIBUTING.md with development guidelines

[Unreleased]: https://codeberg.org/Bawycle/tasmor_lib/compare/v0.9.0...HEAD
[0.9.0]: https://codeberg.org/Bawycle/tasmor_lib/compare/v0.8.0...v0.9.0
[0.8.0]: https://codeberg.org/Bawycle/tasmor_lib/compare/v0.7.0...v0.8.0
[0.7.0]: https://codeberg.org/Bawycle/tasmor_lib/compare/v0.6.0...v0.7.0
[0.6.0]: https://codeberg.org/Bawycle/tasmor_lib/compare/v0.5.0...v0.6.0
[0.5.0]: https://codeberg.org/Bawycle/tasmor_lib/compare/v0.4.1...v0.5.0
[0.4.1]: https://codeberg.org/Bawycle/tasmor_lib/compare/v0.4.0...v0.4.1
[0.4.0]: https://codeberg.org/Bawycle/tasmor_lib/compare/v0.3.0...v0.4.0
[0.3.0]: https://codeberg.org/Bawycle/tasmor_lib/compare/v0.2.1...v0.3.0
[0.2.1]: https://codeberg.org/Bawycle/tasmor_lib/compare/v0.2.0...v0.2.1
[0.2.0]: https://codeberg.org/Bawycle/tasmor_lib/compare/v0.1.0...v0.2.0
[0.1.0]: https://codeberg.org/Bawycle/tasmor_lib/releases/tag/v0.1.0
