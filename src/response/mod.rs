// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Response parsing for Tasmota JSON responses.
//!
//! This module provides structures for deserializing JSON responses from
//! Tasmota devices. Each response type corresponds to a specific command
//! or status query.
//!
//! # Response Types
//!
//! | Response Type | Tasmota Commands | Description |
//! |--------------|------------------|-------------|
//! | [`PowerResponse`] | `Power`, `Power1`-`Power8` | Relay on/off state |
//! | [`DimmerResponse`] | `Dimmer` | Brightness level (0-100) |
//! | [`HsbColorResponse`] | `HSBColor` | Color in HSB format |
//! | [`ColorTemperatureResponse`] | `CT` | White color temperature |
//! | [`FadeResponse`] | `Fade` | Fade transition enable/disable |
//! | [`FadeSpeedResponse`] | `Speed` | Fade transition speed (1-40) |
//! | [`StartupFadeResponse`] | `SetOption91` | Fade at startup setting |
//! | [`EnergyResponse`] | `Status 10` | Power consumption data |
//! | [`StatusResponse`] | `Status 0` | Full device status |
//!
//! # Usage Pattern
//!
//! Response types are typically used with `serde_json` to parse the JSON
//! returned by [`Device::send_command`](crate::Device::send_command):
//!
//! ```
//! use tasmor_lib::response::PowerResponse;
//!
//! // Tasmota returns JSON like: {"POWER": "ON"} or {"POWER1": "ON", "POWER2": "OFF"}
//! let json = r#"{"POWER": "ON"}"#;
//! let response: PowerResponse = serde_json::from_str(json).unwrap();
//!
//! // Query the power state
//! let state = response.first_power_state().unwrap();
//! println!("Power is: {}", state);  // "ON"
//! ```

mod color;
mod dimmer;
mod energy;
mod fade;
mod power;
mod status;

pub use color::{ColorTemperatureResponse, HsbColorResponse};
pub use dimmer::DimmerResponse;
pub use energy::EnergyResponse;
pub use fade::{FadeResponse, FadeSpeedResponse, StartupFadeResponse};
pub use power::PowerResponse;
pub use status::{
    StatusDeviceParameters, StatusFirmware, StatusMemory, StatusMqtt, StatusNetwork, StatusResponse,
};
