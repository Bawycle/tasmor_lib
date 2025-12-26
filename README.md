# TasmoR Lib

[![Crates.io](https://img.shields.io/crates/v/tasmor_lib.svg)](https://crates.io/crates/tasmor_lib)
[![Documentation](https://docs.rs/tasmor_lib/badge.svg)](https://docs.rs/tasmor_lib)
[![License: MPL 2.0](https://img.shields.io/badge/License-MPL_2.0-blue.svg)](https://opensource.org/licenses/MPL-2.0)
[![Rust](https://img.shields.io/badge/rust-1.92%2B-orange.svg)](https://www.rust-lang.org)

A modern, type-safe Rust library for controlling [Tasmota](https://tasmota.github.io) IoT devices via MQTT and HTTP protocols.

> ⚠️ **Early Development**: This project is in active development (v0.x.x). The API may change between versions. Not recommended for production use yet.

> 📌 **Tested with**: Tasmota firmware v15.2.0

## Features

- 🔒 **Type-safe API** - Compile-time guarantees for valid commands and values
- 🔌 **Dual protocol support** - Control devices via MQTT or HTTP
- ⚡ **Async/await** - Built on [Tokio](https://tokio.rs) for efficient async I/O
- 🎨 **Full device support** - Lights (RGB/CCT), switches, relays, energy monitors
- 📡 **Event-driven architecture** - Subscribe to device state changes in real-time
- 🏊 **Connection pooling** - Efficient broker connection sharing for multi-device setups
- 🧪 **Well-tested** - Comprehensive unit and integration tests (370+ tests)
- 📚 **Documented** - Comprehensive API documentation with examples

### Supported Capabilities

- **Power control** - On/Off/Toggle for single and multi-relay devices
- **Lighting** - Dimmer, color temperature (CCT), HSB color control
- **Energy monitoring** - Power, voltage, current, energy consumption tracking
- **Device status** - Query firmware, network, and sensor information
- **Transitions** - Fade effects and speed control

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
tasmor_lib = "0.1"
```

### Optional Features

- **`serde`** - Enable `Serialize`/`Deserialize` for all public types (device state, events, capabilities)

```toml
[dependencies]
tasmor_lib = { version = "0.1", features = ["serde"] }
```

## Quick Start

### HTTP Example

```rust
use tasmor_lib::{Device, Capabilities, ColorTemperature, Dimmer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Tasmota light bulb - returns device and initial state
    let (device, initial_state) = Device::http("192.168.1.100")
        .with_capabilities(Capabilities::rgbcct_light())
        .build_without_probe()
        .await?;

    // Initial state contains current values
    println!("Current power: {:?}", initial_state.power(1));

    // Turn on and set to warm white
    device.power_on().await?;
    device.set_color_temperature(ColorTemperature::WARM).await?;
    device.set_dimmer(Dimmer::new(75)?).await?;

    Ok(())
}
```

### MQTT Example

```rust
use tasmor_lib::{Device, Capabilities, HsbColor};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to device via MQTT broker (with authentication)
    let (device, initial_state) = Device::mqtt("mqtt://192.168.1.50:1883", "tasmota_bulb")
        .with_credentials("mqtt_user", "mqtt_password")
        .with_capabilities(Capabilities::rgbcct_light())
        .build_without_probe()
        .await?;

    // Initial state is available immediately
    println!("Current dimmer: {:?}", initial_state.dimmer());

    // Set RGB color
    device.set_hsb_color(HsbColor::blue()).await?;

    Ok(())
}
```

### Energy Monitoring

```rust
use tasmor_lib::{Device, Capabilities};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Energy data is available in initial state
    let (_device, state) = Device::http("192.168.1.101")
        .with_capabilities(Capabilities::neo_coolcam())
        .build_without_probe()
        .await?;

    // Access energy data from initial state
    println!("Power: {:?} W", state.power_consumption());
    println!("Voltage: {:?} V", state.voltage());
    println!("Today: {:?} kWh", state.energy_today());
    println!("Total: {:?} kWh", state.energy_total());

    Ok(())
}
```

### Multi-Device Management

For applications controlling multiple MQTT devices, create devices directly and use callbacks for state changes:

```rust
use tasmor_lib::{Device, Capabilities, Dimmer};
use tasmor_lib::subscription::Subscribable;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create multiple devices - each returns (device, initial_state)
    let (living_room, living_state) = Device::mqtt("mqtt://192.168.1.50:1883", "tasmota_living")
        .with_credentials("mqtt_user", "mqtt_pass")
        .with_capabilities(Capabilities::rgbcct_light())
        .build_without_probe()
        .await?;

    let (bedroom, bedroom_state) = Device::mqtt("mqtt://192.168.1.50:1883", "tasmota_bedroom")
        .with_credentials("mqtt_user", "mqtt_pass")
        .with_capabilities(Capabilities::rgbcct_light())
        .build_without_probe()
        .await?;

    // Initial states are immediately available
    println!("Living room power: {:?}", living_state.power(1));
    println!("Bedroom power: {:?}", bedroom_state.power(1));

    // Subscribe to power changes on each device
    living_room.on_power_changed(|relay, state| {
        println!("Living room relay {} is now {:?}", relay, state);
    });

    bedroom.on_power_changed(|relay, state| {
        println!("Bedroom relay {} is now {:?}", relay, state);
    });

    // Subscribe to dimmer changes
    living_room.on_dimmer_changed(|dimmer| {
        println!("Living room dimmer: {:?}", dimmer);
    });

    // Control devices directly
    living_room.power_on().await?;
    living_room.set_dimmer(Dimmer::new(75)?).await?;

    bedroom.power_on().await?;

    Ok(())
}
```

### Telemetry Parsing

Parse MQTT telemetry messages from Tasmota devices:

```rust
use tasmor_lib::telemetry::{parse_telemetry, TelemetryMessage};

fn handle_mqtt_message(topic: &str, payload: &str) {
    if let Ok(msg) = parse_telemetry(topic, payload) {
        match msg {
            TelemetryMessage::State { device_topic, state } => {
                println!("Device {} power: {:?}", device_topic, state.power());
                println!("Dimmer: {:?}", state.dimmer());
            }
            TelemetryMessage::Sensor { device_topic, data } => {
                if let Some(energy) = data.energy() {
                    println!("Device {} power: {:?} W", device_topic, energy.power);
                }
            }
            TelemetryMessage::LastWill { device_topic, online } => {
                println!("Device {} is {}", device_topic, if online { "online" } else { "offline" });
            }
            _ => {}
        }
    }
}
```

## Examples

The `examples/` directory contains runnable examples:

- **`bulb_test.rs`** - Simple example demonstrating basic device control
- **`energy_test.rs`** - Energy monitoring: query power, voltage, current, and consumption

Run an example with:

```bash
cargo run --example bulb_test -- mqtt://192.168.1.50:1883 tasmota_topic user pass
cargo run --example energy_test -- mqtt://192.168.1.50:1883 tasmota_plug user pass
```

## Documentation

- 📖 [API Documentation](https://docs.rs/tasmor_lib) - Full API reference
- 🔧 [Tasmota Commands Reference](https://tasmota.github.io/docs/Commands/) - Official Tasmota protocol

## Roadmap

- [ ] Auto-discovery via mDNS
- [ ] Sequence command builder
- [ ] Stabilize API for 1.0 release

## Development

```bash
# Run tests
cargo test

# Run tests with serde feature
cargo test --features serde

# Check code coverage
cargo tarpaulin --out Stdout

# Run all verification checks
cargo check && cargo build && cargo test && cargo fmt --check && cargo clippy -- -D warnings -W clippy::pedantic
```

## Contributing

Contributions are welcome! This project follows Test-Driven Development (TDD):

1. Write tests first
2. Implement code to pass tests
3. Run full verification pipeline before committing

Please ensure:
- All tests pass (`cargo test`)
- Code is formatted (`cargo fmt`)
- No clippy warnings (`cargo clippy -- -D warnings`)
- Add documentation for public APIs

## License

Licensed under the [Mozilla Public License 2.0](LICENSE) (MPL-2.0).

See [LICENSE](LICENSE) file for details.

## Credits

This library is built for controlling [Tasmota](https://tasmota.github.io/) open-source firmware.

**Key dependencies:**
- [Tokio](https://tokio.rs) - Async runtime
- [rumqttc](https://github.com/bytebeamio/rumqtt) - MQTT client
- [reqwest](https://github.com/seanmonstar/reqwest) - HTTP client

**Testing infrastructure:**
- [wiremock](https://github.com/LukeMathWalker/wiremock-rs) - HTTP mocking
- [mockforge-mqtt](https://github.com/SaaSy-Solutions/mockforge) - MQTT broker simulation

*...and all other amazing crates from the Rust ecosystem that made this project possible.*

---

Made with ❤️ for the Rust and home automation communities.
