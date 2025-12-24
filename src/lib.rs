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
//!     let device = Device::http("192.168.1.100")
//!         .build()
//!         .await?;
//!
//!     // Basic power control
//!     device.power_on().await?;
//!
//!     // Check capabilities before using features
//!     if device.capabilities().dimmer() {
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
//!     let device = Device::http("192.168.1.100")
//!         .with_capabilities(Capabilities::rgbcct_light())
//!         .build_without_probe()?;
//!
//!     device.power_on().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## MQTT Device
//!
//! ```no_run
//! use tasmor_lib::Device;
//!
//! #[tokio::main]
//! async fn main() -> tasmor_lib::Result<()> {
//!     let device = Device::mqtt("mqtt://192.168.1.50:1883", "tasmota_switch")
//!         .build()
//!         .await?;
//!
//!     device.power_toggle().await?;
//!     Ok(())
//! }
//! ```

mod capabilities;
pub mod command;
mod device;
pub mod error;
pub mod event;
pub mod manager;
pub mod protocol;
pub mod response;
pub mod state;
pub mod telemetry;
pub mod types;

pub use capabilities::{Capabilities, CapabilitiesBuilder};
pub use command::{
    ColorTempCommand, Command, DimmerCommand, EnergyCommand, FadeCommand, HsbColorCommand,
    PowerCommand, PowerOnFadeCommand, SpeedCommand, StatusCommand,
};
pub use device::{Device, HttpDeviceBuilder, MqttDeviceBuilder};
pub use error::{DeviceError, Error, ParseError, ProtocolError, Result, ValueError};
pub use response::{EnergyResponse, PowerResponse, StatusResponse};
pub use types::{ColorTemp, Dimmer, FadeSpeed, HsbColor, PowerIndex, PowerState};
