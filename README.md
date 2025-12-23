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
- 🧪 **Well-tested** - Comprehensive unit and integration tests
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

## Documentation

- 📖 [API Documentation](https://docs.rs/tasmor_lib) - Full API reference
- 🔧 [Tasmota Commands Reference](https://tasmota.github.io/docs/Commands/) - Official Tasmota protocol

## Roadmap

- [ ] Auto-discovery
- [ ] WebSocket support for real-time updates
- [ ] Sequence command builder
- [ ] Additional device types (shutters, fans, sensors)
- [ ] Stabilize API for 1.0 release

## Development

```bash
# Run tests
cargo test

# Check code coverage
cargo tarpaulin --out Stdout

# Run all verification checks
cargo check && cargo build && cargo test && cargo fmt --check && cargo clippy -- -D warnings
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
