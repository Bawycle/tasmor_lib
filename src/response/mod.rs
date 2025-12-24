// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Response parsing for Tasmota JSON responses.
//!
//! This module provides structures for deserializing JSON responses from
//! Tasmota devices. Each response type corresponds to a specific command
//! or status query.

mod color;
mod dimmer;
mod energy;
mod power;
mod status;

pub use color::{ColorTempResponse, HsbColorResponse};
pub use dimmer::DimmerResponse;
pub use energy::EnergyResponse;
pub use power::PowerResponse;
pub use status::{
    StatusDeviceParameters, StatusFirmware, StatusMemory, StatusMqtt, StatusNetwork, StatusResponse,
};
