# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Device is now Clone** - `Device<P>` implements `Clone`, enabling easy sharing across async tasks. Clones share the same connection and callbacks (via `Arc`), following the pattern of `reqwest::Client` and `rumqttc::AsyncClient`
- **System info in DeviceState** - New `SystemInfo` struct provides access to device diagnostics (uptime, Wi-Fi RSSI, heap memory). Available via `DeviceState::system_info()` and convenience method `DeviceState::uptime_seconds()`. System info is populated from `Status 0` during `query_state()` (heap, rssi) and from MQTT telemetry via `TelemetryState::to_system_info()` (uptime, rssi)
- **MQTT command timeout** - New `MqttBrokerBuilder::command_timeout()` configures the timeout for waiting on command responses (default: 5 seconds). Useful for slow-responding devices or routines with delays. Consistent with HTTP's `HttpConfig::with_timeout()`

### Changed

- **BREAKING: Renamed callback** - `on_energy_updated()` renamed to `on_energy_changed()` for API consistency with other callbacks (`on_power_changed`, `on_dimmer_changed`, etc.)
- **BREAKING: Removed `uptime_sec()`** - Use `uptime_seconds()` instead for consistency with `TelemetryState`

### Improved

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
- **Fade state tracking** - Initial device state now includes fade enabled/disabled status and fade speed for light devices

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

[Unreleased]: https://codeberg.org/Bawycle/tasmor_lib/compare/v0.3.0...HEAD
[0.3.0]: https://codeberg.org/Bawycle/tasmor_lib/compare/v0.2.1...v0.3.0
[0.2.1]: https://codeberg.org/Bawycle/tasmor_lib/compare/v0.2.0...v0.2.1
[0.2.0]: https://codeberg.org/Bawycle/tasmor_lib/compare/v0.1.0...v0.2.0
[0.1.0]: https://codeberg.org/Bawycle/tasmor_lib/releases/tag/v0.1.0
