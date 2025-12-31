# TasmoR Lib

[![Crates.io](https://img.shields.io/crates/v/tasmor_lib.svg)](https://crates.io/crates/tasmor_lib)
[![Documentation](https://docs.rs/tasmor_lib/badge.svg)](https://docs.rs/tasmor_lib)
[![License: MPL 2.0](https://img.shields.io/badge/License-MPL_2.0-blue.svg)](https://opensource.org/licenses/MPL-2.0)
[![Rust](https://img.shields.io/badge/rust-1.92%2B-orange.svg)](https://www.rust-lang.org)

> **Primary repository**: [Codeberg](https://codeberg.org/Bawycle/tasmor_lib) — Please submit issues and pull requests there.

A modern, type-safe Rust library for controlling [Tasmota](https://tasmota.github.io) IoT devices via MQTT and HTTP protocols.

> **Early Development**: This project is in active development (v0.x.x). The API may change between versions. Not recommended for production use yet.

> **Tested with**: Tasmota firmware v15.2.0

## Features

- **Type-safe API** - Compile-time guarantees for valid commands and values
- **Dual protocol support** - Control devices via MQTT or HTTP
- **Async/await** - Built on [Tokio](https://tokio.rs) for efficient async I/O
- **Full device support** - Lights (RGB/CCT), switches, relays, energy monitors
- **Event-driven architecture** - Subscribe to device state changes in real-time (MQTT)
- **Well-tested** - Comprehensive unit and integration tests (580+ tests)

### Supported Capabilities

| Capability | Description |
|------------|-------------|
| Power control | On/Off/Toggle for single and multi-relay devices |
| Lighting | Dimmer, color temperature (CCT), HSB color control |
| Energy monitoring | Power, voltage, current, energy consumption tracking |
| Device status | Query firmware, network, and sensor information |
| Transitions | Fade effects and speed control |
| Light schemes | Effects (wakeup, color cycling, random) |
| RGB colors | Hex color input (#RRGGBB) with HSB conversion |
| Routines | Execute multiple commands atomically via Backlog0 |
| Device discovery | Auto-discover Tasmota devices on MQTT broker |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
tasmor_lib = "0.2"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

### Feature Flags

Both HTTP and MQTT protocols are enabled by default. To reduce compile time and binary size, you can enable only the protocol you need:

```toml
# HTTP only (no MQTT dependencies)
tasmor_lib = { version = "0.2", default-features = false, features = ["http"] }

# MQTT only (no HTTP dependencies)
tasmor_lib = { version = "0.2", default-features = false, features = ["mqtt"] }
```

## Quick Start

### Basic Switch Control

The simplest use case - controlling a smart switch or relay:

```rust
use tasmor_lib::{Device, Capabilities};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Tasmota switch via HTTP
    let (device, initial_state) = Device::http("192.168.1.100")
        .with_capabilities(Capabilities::basic())
        .build_without_probe()
        .await?;

    // Check current state
    println!("Power is {:?}", initial_state.power(1));

    // Toggle power and get the response
    let response = device.power_toggle().await?;
    println!("Power is now {:?}", response.power_state(1));

    Ok(())
}
```

### MQTT Connection

For persistent connections with real-time updates:

```rust
use tasmor_lib::{MqttBroker, Capabilities};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to MQTT broker
    let broker = MqttBroker::builder()
        .host("192.168.1.50")
        .credentials("mqtt_user", "mqtt_password")
        .build()
        .await?;

    // Create device from broker
    let (device, initial_state) = broker.device("tasmota_switch")
        .with_capabilities(Capabilities::basic())
        .build_without_probe()
        .await?;

    println!("Power is {:?}", initial_state.power(1));
    device.power_on().await?;

    // Clean disconnect when done
    device.disconnect().await;
    broker.disconnect().await?;

    Ok(())
}
```

## Building Devices

### `build()` vs `build_without_probe()`

Both methods return `(Device, DeviceState)` - the device handle and its initial state.

| Method | When to use |
|--------|-------------|
| `build()` | Auto-detects device capabilities by querying device status. Use when you don't know the device type. |
| `build_without_probe()` | Uses capabilities you provide. Faster startup, recommended when you know the device type. |

```rust
// Auto-detection: queries the device to discover capabilities
let (device, state) = Device::http("192.168.1.100")
    .build()
    .await?;

// Manual: you specify the capabilities (no capability query)
let (device, state) = Device::http("192.168.1.100")
    .with_capabilities(Capabilities::rgbcct_light())
    .build_without_probe()
    .await?;
```

Both methods query the device for its current state (power, dimmer, energy, etc.) and return it as `DeviceState`.

### Predefined Capabilities

```rust
use tasmor_lib::Capabilities;

Capabilities::basic()           // Simple switch (1 relay)
Capabilities::neo_coolcam()     // Smart plug with energy monitoring
Capabilities::rgbcct_light()    // Full RGB + CCT light bulb
Capabilities::rgb_light()       // RGB only light
Capabilities::cct_light()       // Color temperature only light
```

### Custom Capabilities

```rust
use tasmor_lib::CapabilitiesBuilder;

let caps = CapabilitiesBuilder::new()
    .with_power_channels(2)           // Dual relay
    .with_dimmer_control()
    .with_energy_monitoring()
    .build();
```

## Examples by Use Case

### Light Control (RGB/CCT)

```rust
use tasmor_lib::{Device, Capabilities, ColorTemperature, Dimmer, HsbColor};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (device, _) = Device::http("192.168.1.100")
        .with_capabilities(Capabilities::rgbcct_light())
        .build_without_probe()
        .await?;

    // Power and brightness
    device.power_on().await?;
    device.set_dimmer(Dimmer::new(75)?).await?;

    // Color temperature (warm to cold)
    device.set_color_temperature(ColorTemperature::WARM).await?;

    // RGB color
    device.set_hsb_color(HsbColor::blue()).await?;

    // Custom HSB color (hue: 0-360, saturation: 0-100, brightness: 0-100)
    device.set_hsb_color(HsbColor::new(120, 80, 100)?).await?;

    Ok(())
}
```

### Multi-Relay Control

For devices with multiple relays (e.g., dual switch):

```rust
use tasmor_lib::{Device, CapabilitiesBuilder, PowerIndex, PowerState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let caps = CapabilitiesBuilder::new()
        .with_power_channels(2)
        .build();

    let (device, state) = Device::http("192.168.1.100")
        .with_capabilities(caps)
        .build_without_probe()
        .await?;

    // Check each relay
    println!("Relay 1: {:?}", state.power(1));
    println!("Relay 2: {:?}", state.power(2));

    // Control individual relays
    device.set_power(PowerIndex::new(1)?, PowerState::On).await?;
    device.set_power(PowerIndex::new(2)?, PowerState::Off).await?;

    // Toggle a specific relay
    device.toggle_power(PowerIndex::new(2)?).await?;

    Ok(())
}
```

### Energy Monitoring

```rust
use tasmor_lib::{Device, Capabilities};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (device, state) = Device::http("192.168.1.101")
        .with_capabilities(Capabilities::neo_coolcam())
        .build_without_probe()
        .await?;

    // Energy data is available in initial state
    println!("Power:     {:?} W", state.power_consumption());
    println!("Voltage:   {:?} V", state.voltage());
    println!("Current:   {:?} A", state.current());
    println!("Today:     {:?} kWh", state.energy_today());
    println!("Yesterday: {:?} kWh", state.energy_yesterday());
    println!("Total:     {:?} kWh", state.energy_total());

    // Reset total energy counter
    let updated = device.reset_energy_total().await?;
    if let Some(energy) = updated.energy() {
        println!("Reset! New total: {} kWh", energy.total);
    }

    Ok(())
}
```

### Typed Responses

All commands return typed responses for reliable parsing:

```rust
use tasmor_lib::{Device, Capabilities, Dimmer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (device, _) = Device::http("192.168.1.100")
        .with_capabilities(Capabilities::rgbcct_light())
        .build_without_probe()
        .await?;

    // PowerResponse
    let power_resp = device.power_on().await?;
    println!("Relay 1 state: {:?}", power_resp.power_state(1));
    println!("All relays: {:?}", power_resp.all_power_states());

    // DimmerResponse
    let dimmer_resp = device.set_dimmer(Dimmer::new(50)?).await?;
    println!("Dimmer level: {}", dimmer_resp.dimmer());

    // ColorTemperatureResponse
    let ct_resp = device.set_color_temperature(153u16.try_into()?).await?;
    println!("Color temp: {} mireds", ct_resp.color_temperature());

    // HsbColorResponse
    let hsb_resp = device.set_hsb_color((180u16, 100u8, 100u8).try_into()?).await?;
    println!("HSB: {:?}", hsb_resp.hsb_color());

    Ok(())
}
```

### Real-Time Updates (MQTT Callbacks)

Subscribe to device state changes pushed via MQTT:

```rust
use tasmor_lib::{MqttBroker, Capabilities};
use tasmor_lib::subscription::Subscribable;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let broker = MqttBroker::builder()
        .host("192.168.1.50")
        .credentials("mqtt_user", "mqtt_pass")
        .build()
        .await?;

    let (device, _) = broker.device("tasmota_bulb")
        .with_capabilities(Capabilities::rgbcct_light())
        .build_without_probe()
        .await?;

    // Subscribe to power changes (triggered by external events)
    device.on_power_changed(|relay_index, power_state| {
        println!("Relay {} changed to {:?}", relay_index, power_state);
    });

    // Subscribe to dimmer changes
    device.on_dimmer_changed(|dimmer| {
        println!("Dimmer changed to {}", dimmer.value());
    });

    // Subscribe to all state changes
    device.on_state_changed(|change| {
        println!("State change: {:?}", change);
    });

    // Keep the application running to receive callbacks
    tokio::signal::ctrl_c().await?;

    device.disconnect().await;
    broker.disconnect().await?;
    Ok(())
}
```

### Multi-Device Management

Multiple devices can share a single broker connection for efficiency:

```rust
use tasmor_lib::{MqttBroker, Capabilities, Dimmer};
use tasmor_lib::subscription::Subscribable;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to broker once
    let broker = MqttBroker::builder()
        .host("192.168.1.50")
        .credentials("mqtt_user", "mqtt_pass")
        .build()
        .await?;

    // Create multiple devices sharing the broker connection
    let (living_room, living_state) = broker.device("tasmota_living")
        .with_capabilities(Capabilities::rgbcct_light())
        .build_without_probe()
        .await?;

    let (bedroom, bedroom_state) = broker.device("tasmota_bedroom")
        .with_capabilities(Capabilities::rgbcct_light())
        .build_without_probe()
        .await?;

    // Initial states are immediately available
    println!("Living room: {:?}", living_state.power(1));
    println!("Bedroom: {:?}", bedroom_state.power(1));

    // Subscribe to changes on each device
    living_room.on_power_changed(|relay, state| {
        println!("Living room relay {} -> {:?}", relay, state);
    });

    bedroom.on_power_changed(|relay, state| {
        println!("Bedroom relay {} -> {:?}", relay, state);
    });

    // Control devices
    living_room.power_on().await?;
    living_room.set_dimmer(Dimmer::new(75)?).await?;
    bedroom.power_on().await?;

    // Clean disconnect
    living_room.disconnect().await;
    bedroom.disconnect().await;
    broker.disconnect().await?;

    Ok(())
}
```

### MQTT Device Discovery

Automatically discover all Tasmota devices on an MQTT broker:

```rust
use tasmor_lib::MqttBroker;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to broker
    let broker = MqttBroker::builder()
        .host("192.168.1.50")
        .port(1883)
        .credentials("mqtt_user", "mqtt_pass")
        .build()
        .await?;

    // Discover devices (10 second timeout)
    let devices = broker.discover_devices(Duration::from_secs(10)).await?;

    println!("Found {} devices:", devices.len());
    for (device, state) in &devices {
        println!("  - Power: {:?}, Dimmer: {:?}", state.power(1), state.dimmer());
    }

    // Use discovered devices...
    for (device, _) in devices {
        device.power_toggle().await?;
    }

    broker.disconnect().await?;
    Ok(())
}
```

### Command Routines

Execute multiple commands atomically using the Routine builder:

```rust
use std::time::Duration;
use tasmor_lib::{Device, Capabilities, Routine, Dimmer, HsbColor};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (device, _) = Device::http("192.168.1.100")
        .with_capabilities(Capabilities::rgbcct_light())
        .build_without_probe()
        .await?;

    // Build a routine with multiple commands
    let wakeup_routine = Routine::builder()
        .set_dimmer(Dimmer::new(10)?)
        .power_on()
        .delay(Duration::from_secs(5))
        .set_dimmer(Dimmer::new(50)?)
        .delay(Duration::from_secs(5))
        .set_dimmer(Dimmer::new(100)?)
        .build()?;

    // Execute all commands atomically
    device.run(&wakeup_routine).await?;

    // RGB color transition routine
    let color_routine = Routine::builder()
        .set_hsb_color(HsbColor::red())
        .delay(Duration::from_secs(2))
        .set_hsb_color(HsbColor::green())
        .delay(Duration::from_secs(2))
        .set_hsb_color(HsbColor::blue())
        .build()?;

    device.run(&color_routine).await?;

    Ok(())
}
```

### Parsing External Telemetry

If you're using your own MQTT client and want to parse Tasmota messages:

```rust
use tasmor_lib::telemetry::{parse_telemetry, TelemetryMessage};

fn handle_mqtt_message(topic: &str, payload: &str) {
    if let Ok(msg) = parse_telemetry(topic, payload) {
        match msg {
            TelemetryMessage::State { device_topic, state } => {
                println!("[{}] Power: {:?}, Dimmer: {:?}",
                    device_topic, state.power(), state.dimmer());
            }
            TelemetryMessage::Sensor { device_topic, data } => {
                if let Some(energy) = data.energy() {
                    println!("[{}] Power: {} W", device_topic, energy.power);
                }
            }
            TelemetryMessage::LastWill { device_topic, online } => {
                println!("[{}] {}", device_topic, if online { "online" } else { "offline" });
            }
            _ => {}
        }
    }
}
```

## Runnable Examples

The `examples/` directory contains complete runnable examples:

| Example | Description |
|---------|-------------|
| `bulb_test.rs` | Basic light bulb control |
| `energy_test.rs` | Energy monitoring with formatted output |
| `routine_test.rs` | Wakeup routine with gradual brightness increase |
| `discovery_test.rs` | MQTT device discovery |

```bash
cargo run --example bulb_test -- 192.168.1.50 tasmota_topic user pass
cargo run --example energy_test -- 192.168.1.50 tasmota_plug user pass
cargo run --example routine_test -- 192.168.1.50 tasmota_bulb user pass
cargo run --example discovery_test -- 192.168.1.50 user pass
```

## Documentation

- [API Documentation](https://docs.rs/tasmor_lib) - Full API reference
- [Tasmota Commands Reference](https://tasmota.github.io/docs/Commands/) - Official Tasmota protocol

## Roadmap

- [ ] Stabilize API for 1.0 release

## Development

```bash
# Run tests
cargo test

# Run tests with serde feature
cargo test --features serde

# Check code coverage
cargo tarpaulin --out Stdout

# Full verification pipeline
cargo check && cargo build && cargo test && cargo fmt --check && cargo clippy -- -D warnings -W clippy::pedantic
```

## Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) for details on:

- Development setup
- Code style and linting
- Testing requirements
- Commit message conventions
- Pull request process

## License

Licensed under the [Mozilla Public License 2.0](LICENSE) (MPL-2.0).

## Credits

Built for controlling [Tasmota](https://tasmota.github.io/) open-source firmware.

**Key dependencies:**
- [Tokio](https://tokio.rs) - Async runtime
- [rumqttc](https://github.com/bytebeamio/rumqtt) - MQTT client
- [reqwest](https://github.com/seanmonstar/reqwest) - HTTP client

**Testing:**
- [wiremock](https://github.com/LukeMathWalker/wiremock-rs) - HTTP mocking
