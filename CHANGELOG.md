# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Routine builder** - Execute multiple commands atomically using `Routine::builder()` and `device.run(&routine)`. Supports power, lighting, fade, and scheme commands with configurable delays (max 30 steps)
- **MQTT device discovery** - `MqttBroker::discover_devices()` and standalone `discover_devices()` function to auto-discover Tasmota devices on a broker

### Fixed

- **MQTT capability detection** - `build()` now correctly detects device capabilities (dimmer, RGB, color temperature, energy monitoring) for MQTT connections
- **JSON parsing compatibility** - Improved parsing robustness for various Tasmota firmware responses

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

[Unreleased]: https://codeberg.org/Bawycle/tasmor_lib/compare/v0.1.0...HEAD
[0.1.0]: https://codeberg.org/Bawycle/tasmor_lib/releases/tag/v0.1.0
