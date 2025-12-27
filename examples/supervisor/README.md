# Tasmota Supervisor

A cross-platform GUI application for monitoring and controlling Tasmota devices via HTTP and MQTT protocols.

## Features

- 📱 **Multi-device Management**: Add, configure, and monitor multiple Tasmota devices
- 🔌 **Protocol Support**: Connect via HTTP or MQTT
- 💡 **Smart Bulb Control**: RGB color, dimming, color temperature
- ⚡ **Energy Monitoring**: Real-time power consumption for compatible devices
- 🎨 **Modern UI**: Built with egui for a responsive cross-platform experience

## Supported Devices

### Smart Bulbs (RGBCCT)
- **Athom 5W/7W Bulb**: RGB + warm/cold white control
- **Athom 15W Bulb**: High-power RGB + warm/cold white control

### Smart Plugs
- **NOUS A1T**: Smart plug with energy monitoring

## Installation

### Prerequisites

- Rust 1.92.0 or later
- Linux x86_64 (other platforms may work but are untested)

### Build from Source

```bash
cd examples/supervisor
cargo build --release
```

The binary will be located at `target/release/tasmota-supervisor`.

## Usage

### Running the Application

```bash
cargo run --release
```

Or run the compiled binary:

```bash
./target/release/tasmota-supervisor
```

### Adding a Device

1. Click the **"➕ Add Device"** button
2. Fill in the device information:
   - **Name**: A friendly name for your device
   - **Model**: Select from supported device models
   - **Protocol**: Choose HTTP or MQTT
   - **Connection details**:
     - HTTP: Enter the device IP address (e.g., `192.168.1.100`)
     - MQTT: Enter broker URL and device topic
   - **Authentication** (optional): Enable and provide credentials

3. Click **"Add"** to save the device

### Controlling Devices

#### Connection
- Click **"Connect"** to establish connection
- Click **"Disconnect"** to close connection
- Click **"⟳"** to refresh device status

#### Power Control
- Click **"Turn On"** or **"Turn Off"** to control power
- Status shows current state (ON/OFF)

#### Dimming (for bulbs)
- Use the brightness slider to adjust light level (0-100%)
- Changes apply in real-time

#### Energy Monitoring (for plugs)
- Current power consumption displayed in watts
- Updates when status is refreshed

### Removing a Device

Click the **"🗑"** button on a device card to remove it.

## Configuration Examples

### HTTP Device

```
Name: Living Room Bulb
Model: Athom 15W Bulb
Protocol: HTTP
Host: 192.168.1.100
```

### MQTT Device

```
Name: Kitchen Plug
Model: NOUS A1T
Protocol: MQTT
Broker: mqtt://192.168.1.50:1883
Topic: tasmota_kitchen_plug
Username: mqtt_user
Password: mqtt_pass
```

## Architecture

The application follows a clean architecture with separation of concerns:

- **Device Models**: Predefined capabilities for supported devices
- **Device Manager**: Async communication with Tasmota devices via `tasmor_lib`
- **UI Components**: Reusable egui widgets for device cards and dialogs
- **Event-Driven**: Unidirectional data flow with command/event pattern

### Project Structure

```
src/
├── main.rs           # Application entry point and main UI
├── device_model.rs   # Device model definitions and capabilities
├── device_config.rs  # Device configuration and state management
├── device_manager.rs # Async device communication handler
└── ui.rs             # UI components and widgets
```

## Development

### Running Tests

```bash
cargo test
```

### Code Coverage

```bash
cargo tarpaulin
```

### Linting

```bash
cargo clippy -- -D warnings -W clippy::pedantic
```

### Formatting

```bash
cargo fmt
```

## Troubleshooting

### Connection Issues

**HTTP Connection Failed**
- Verify the device IP address is correct
- Ensure the device is powered on and connected to the network
- Check if HTTP authentication is required

**MQTT Connection Failed**
- Verify broker URL format: `mqtt://host:port`
- Check MQTT credentials if authentication is enabled
- Ensure the device topic matches the Tasmota configuration

### Device Not Responding

1. Click the **"⟳"** refresh button
2. Try disconnecting and reconnecting
3. Check device logs in Tasmota web interface

## License

This project is licensed under the Mozilla Public License 2.0 (MPL-2.0).

## Acknowledgments

- Built with [egui](https://github.com/emilk/egui) - Immediate mode GUI library
- Uses [tasmor_lib](../../) - Rust library for Tasmota device control
- Compatible with [Tasmota](https://tasmota.github.io/) firmware
