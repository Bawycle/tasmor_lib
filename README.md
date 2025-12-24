# TasmoR Lib

[![Crates.io](https://img.shields.io/crates/v/tasmor_lib.svg)](https://crates.io/crates/tasmor_lib)
[![Documentation](https://docs.rs/tasmor_lib/badge.svg)](https://docs.rs/tasmor_lib)
[![License: MPL 2.0](https://img.shields.io/badge/License-MPL_2.0-blue.svg)](https://opensource.org/licenses/MPL-2.0)
[![Rust](https://img.shields.io/badge/rust-1.92%2B-orange.svg)](https://www.rust-lang.org)

A modern, type-safe Rust library for controlling [Tasmota](https://tasmota.github.io) IoT devices via MQTT and HTTP protocols.

> ⚠️ **Early Development**: This project is in active development (v0.x.x). The API may change between versions. Not recommended for production use yet.

## Features

- 🔒 **Type-safe API** - Compile-time guarantees for valid commands and values
- 🔌 **Dual protocol support** - Control devices via MQTT or HTTP
- ⚡ **Async/await** - Built on [Tokio](https://tokio.rs) for efficient async I/O
- 🎨 **Full device support** - Lights (RGB/CCT), switches, relays, energy monitors
- 📡 **Event-driven architecture** - Subscribe to device state changes in real-time
- 🏊 **Connection pooling** - Efficient broker connection sharing for multi-device setups
- 🧪 **Well-tested** - Comprehensive unit and integration tests (340+ tests)
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
use tasmor_lib::{Device, Capabilities, ColorTemp, Dimmer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Tasmota light bulb
    let device = Device::http("192.168.1.100")
        .with_capabilities(Capabilities::rgbcct_light())
        .build_without_probe()?;

    // Turn on and set to warm white
    device.power_on().await?;
    device.set_color_temp(ColorTemp::WARM).await?;
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
    let device = Device::mqtt("mqtt://192.168.1.50:1883", "tasmota_bulb")
        .with_credentials("mqtt_user", "mqtt_password")
        .with_capabilities(Capabilities::rgbcct_light())
        .build_without_probe()
        .await?;

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
    let device = Device::http("192.168.1.101")
        .with_capabilities(Capabilities::neo_coolcam())
        .build_without_probe()?;

    // Query energy consumption
    let energy = device.energy().await?;
    if let Some(data) = energy.energy() {
        println!("Power: {} W", data.power);
        println!("Voltage: {} V", data.voltage);
        println!("Today: {} kWh", data.today);
    }

    Ok(())
}
```

### Multi-Device Management

For applications controlling multiple devices, use the `DeviceManager`:

```rust
use tasmor_lib::manager::{DeviceManager, DeviceConfig};
use tasmor_lib::event::DeviceEvent;
use tasmor_lib::{Capabilities, Dimmer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = DeviceManager::new();

    // Subscribe to device events
    let mut events = manager.subscribe();
    tokio::spawn(async move {
        while let Ok(event) = events.recv().await {
            match event {
                DeviceEvent::StateChanged { device_id, change, .. } => {
                    println!("Device {:?} changed: {:?}", device_id, change);
                }
                DeviceEvent::ConnectionChanged { device_id, connected: true, .. } => {
                    println!("Device {:?} connected", device_id);
                }
                _ => {}
            }
        }
    });

    // Add devices
    let config = DeviceConfig::mqtt("mqtt://192.168.1.50:1883", "living_room")
        .with_capabilities(Capabilities::rgbcct_light())
        .with_friendly_name("Living Room Light");

    let device_id = manager.add_device(config).await;

    // Control devices by ID
    manager.set_dimmer(device_id, Dimmer::new(75)?).await?;

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
