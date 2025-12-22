// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Protocol implementations for communicating with Tasmota devices.
//!
//! This module provides HTTP and MQTT protocol implementations for sending
//! commands and receiving responses from Tasmota devices.
//!
//! # Protocols
//!
//! - [`HttpClient`]: HTTP-based communication using REST API
//! - [`MqttClient`]: MQTT-based communication for real-time updates

mod http;
mod mqtt;

pub use http::{HttpClient, HttpClientBuilder};
pub use mqtt::{MqttClient, MqttClientBuilder};

use crate::command::Command;
use crate::error::ProtocolError;

/// Response from a Tasmota command.
#[derive(Debug, Clone)]
pub struct CommandResponse {
    /// The raw JSON response body.
    pub body: String,
}

impl CommandResponse {
    /// Parses the response as a specific type.
    ///
    /// # Errors
    ///
    /// Returns error if the JSON cannot be parsed into the target type.
    pub fn parse<T: serde::de::DeserializeOwned>(&self) -> Result<T, crate::error::ParseError> {
        serde_json::from_str(&self.body).map_err(Into::into)
    }
}

/// Trait for protocol implementations that can send commands to Tasmota devices.
#[allow(async_fn_in_trait)]
pub trait Protocol {
    /// Sends a command to the device and returns the response.
    ///
    /// # Arguments
    ///
    /// * `command` - The command to send
    ///
    /// # Errors
    ///
    /// Returns `ProtocolError` if the command fails to send or receive.
    async fn send_command<C: Command + Sync>(
        &self,
        command: &C,
    ) -> Result<CommandResponse, ProtocolError>;

    /// Sends a raw command string to the device.
    ///
    /// # Arguments
    ///
    /// * `command` - The raw command string
    ///
    /// # Errors
    ///
    /// Returns `ProtocolError` if the command fails.
    async fn send_raw(&self, command: &str) -> Result<CommandResponse, ProtocolError>;
}
