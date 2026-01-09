// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `TasmoR` Lib - A Rust library to control Tasmota devices.
//!
//! This library provides async APIs to interact with Tasmota-powered devices
//! via both HTTP and MQTT protocols.
//!
//! # Supported Features
//!
//! - **Power control**: Turn devices on/off, toggle, blink
//! - **Light control**: Dimmer, color temperature, HSB colors, fade effects
//! - **Status queries**: Device status, network info, firmware version
//! - **Energy monitoring**: Power consumption, voltage, current readings
//!
//! # Supported Modules
//!
//! - Generic (Module 18): Flexible GPIO configuration
//! - Neo Coolcam (Module 49): Smart plugs with energy monitoring
//!
//! # Feature Flags
//!
//! This library supports optional features to reduce compile time and binary size:
//!
//! - `http` - Enables HTTP protocol support (enabled by default)
//! - `mqtt` - Enables MQTT protocol support (enabled by default)
//!
//! Both features are enabled by default. To use only one protocol:
//!
//! ```toml
//! # HTTP only
//! tasmor_lib = { version = "0.4", default-features = false, features = ["http"] }
//!
//! # MQTT only
//! tasmor_lib = { version = "0.4", default-features = false, features = ["mqtt"] }
//! ```
//!
//! # Quick Start
//!
//! ## HTTP Device with Auto-Detection
//!
//! ```no_run
//! use tasmor_lib::Device;
//!
//! #[tokio::main]
//! async fn main() -> tasmor_lib::Result<()> {
//!     // Create device with automatic capability detection
//!     // Returns (device, initial_state) tuple
//!     let (device, _initial_state) = Device::http("192.168.1.100")
//!         .build()
//!         .await?;
//!
//!     // Basic power control
//!     device.power_on().await?;
//!
//!     // Check capabilities before using features
//!     if device.capabilities().supports_dimmer_control() {
//!         device.set_dimmer(tasmor_lib::Dimmer::new(75)?).await?;
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## HTTP Device with Manual Capabilities
//!
//! ```no_run
//! use tasmor_lib::{Device, Capabilities};
//!
//! #[tokio::main]
//! async fn main() -> tasmor_lib::Result<()> {
//!     // Create device without probing (faster startup)
//!     // Returns (device, initial_state) tuple
//!     let (device, _initial_state) = Device::http("192.168.1.100")
//!         .with_capabilities(Capabilities::rgbcct_light())
//!         .build_without_probe()
//!         .await?;
//!
//!     device.power_on().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## MQTT Device
//!
//! ```no_run
//! use tasmor_lib::MqttBroker;
//!
//! #[tokio::main]
//! async fn main() -> tasmor_lib::Result<()> {
//!     // Connect to MQTT broker
//!     let broker = MqttBroker::builder()
//!         .host("192.168.1.50")
//!         .build()
//!         .await?;
//!
//!     // Create device - returns (device, initial_state) tuple
//!     let (device, _initial_state) = broker.device("tasmota_switch")
//!         .build()
//!         .await?;
//!
//!     device.power_toggle().await?;
//!
//!     // Clean disconnect when done
//!     device.disconnect().await;
//!     broker.disconnect().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## MQTT Device with Callbacks (Event Subscriptions)
//!
//! MQTT devices support real-time event subscriptions via callbacks:
//!
//! ```no_run
//! use tasmor_lib::{MqttBroker, subscription::Subscribable};
//!
//! #[tokio::main]
//! async fn main() -> tasmor_lib::Result<()> {
//!     let broker = MqttBroker::builder()
//!         .host("192.168.1.50")
//!         .build()
//!         .await?;
//!
//!     let (device, _initial_state) = broker.device("tasmota_switch")
//!         .build()
//!         .await?;
//!
//!     // Subscribe to power state changes
//!     device.on_power_changed(|relay_idx, state| {
//!         println!("Relay {} is now {:?}", relay_idx, state);
//!     });
//!
//!     // Subscribe to dimmer changes
//!     device.on_dimmer_changed(|value| {
//!         println!("Dimmer set to {:?}", value);
//!     });
//!
//!     device.power_toggle().await?;
//!
//!     // Clean disconnect when done
//!     device.disconnect().await;
//!     broker.disconnect().await?;
//!     Ok(())
//! }
//! ```
//!
//! # HTTP vs MQTT: Choosing a Protocol
//!
//! This library supports two protocols for communicating with Tasmota devices.
//! Each has distinct characteristics suited to different use cases.
//!
//! ## Feature Comparison
//!
//! | Feature | HTTP | MQTT |
//! |---------|------|------|
//! | Connection type | Stateless (request/response) | Persistent (pub/sub) |
//! | Real-time events | ❌ Not supported | ✅ Full support |
//! | Event subscriptions | ❌ Compile-time error | ✅ [`Subscribable`] trait |
//! | Connection overhead | New connection per command | Single persistent connection |
//! | Network requirements | Direct device access | MQTT broker required |
//! | Firewall friendly | ✅ Standard HTTP/HTTPS | May require port forwarding |
//! | Multi-device efficiency | One connection per device | Shared broker connection |
//! | Device topic | N/A | ✅ `device.topic()` |
//! | Explicit disconnect | N/A | ✅ `device.disconnect()` |
//!
//! ## When to Use HTTP
//!
//! - **Simple scripts**: One-off commands or automation scripts
//! - **Direct device access**: No MQTT broker available
//! - **Firewall constraints**: Only HTTP ports are open
//! - **Low-frequency control**: Occasional commands without state tracking
//!
//! ```no_run
//! use tasmor_lib::Device;
//!
//! # async fn example() -> tasmor_lib::Result<()> {
//! // HTTP: Simple, direct control
//! let (device, _) = Device::http("192.168.1.100").build().await?;
//! device.power_on().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## When to Use MQTT
//!
//! - **Real-time monitoring**: React to device state changes instantly
//! - **Home automation**: Integration with existing MQTT infrastructure
//! - **Multi-device setups**: Efficiently manage many devices via one broker
//! - **State synchronization**: Keep local state in sync with device state
//!
//! ```no_run
//! use tasmor_lib::{MqttBroker, subscription::Subscribable};
//!
//! # async fn example() -> tasmor_lib::Result<()> {
//! // MQTT: Real-time events and state tracking
//! let broker = MqttBroker::builder()
//!     .host("192.168.1.50")
//!     .build()
//!     .await?;
//!
//! let (device, _initial_state) = broker.device("tasmota_plug")
//!     .build()
//!     .await?;
//!
//! // React to external changes (physical button, other apps, etc.)
//! device.on_power_changed(|idx, state| {
//!     println!("Relay {idx} changed to {state:?}");
//! });
//!
//! // MQTT-specific methods
//! println!("Device topic: {}", device.topic());
//!
//! // Clean disconnect when done
//! device.disconnect().await;
//! assert!(device.is_disconnected());
//! # Ok(())
//! # }
//! ```
//!
//! ## Type Safety
//!
//! The protocol choice is encoded in the type system. Attempting to use
//! subscription methods on an HTTP device results in a **compile-time error**:
//!
//! ```compile_fail
//! use tasmor_lib::{Device, subscription::Subscribable};
//!
//! # async fn example() -> tasmor_lib::Result<()> {
//! let (device, _) = Device::http("192.168.1.100").build().await?;
//!
//! // This will NOT compile - HTTP devices don't implement Subscribable
//! device.on_power_changed(|idx, state| {
//!     println!("Power changed");
//! });
//! # Ok(())
//! # }
//! ```
//!
//! [`Subscribable`]: subscription::Subscribable

mod capabilities;
pub mod command;
mod device;
#[cfg(feature = "mqtt")]
pub mod discovery;
pub mod error;
pub mod protocol;
pub mod response;
pub mod state;
pub mod subscription;
pub mod telemetry;
pub mod types;

// Core types
pub use capabilities::{Capabilities, CapabilitiesBuilder};
pub use device::Device;
pub use error::{DeviceError, Error, ParseError, ProtocolError, Result, ValueError};
pub use state::{DeviceState, StateChange, SystemInfo};

// Protocol configuration
#[cfg(feature = "http")]
pub use protocol::HttpConfig;
#[cfg(feature = "mqtt")]
pub use protocol::{MqttBroker, MqttBrokerBuilder};

// Command building (Routine only - other commands via Device methods)
pub use command::{Routine, RoutineBuilder};

// Response types (returned by Device methods)
pub use response::{
    ColorTemperatureResponse, DimmerResponse, EnergyResponse, FadeDurationResponse, FadeResponse,
    HsbColorResponse, PowerResponse, RgbColorResponse, RoutineResponse, SchemeResponse,
    StartupFadeResponse, StatusResponse, WakeupDurationResponse,
};

// Subscriptions (MQTT only)
#[cfg(feature = "mqtt")]
pub use subscription::{Subscribable, SubscriptionId};

// Value types (parameters for commands and state)
pub use types::{
    ColorTemperature, DateTimeParseError, Dimmer, FadeDuration, HsbColor, PowerIndex, PowerState,
    RgbColor, Scheme, TasmotaDateTime, WakeupDuration,
};
