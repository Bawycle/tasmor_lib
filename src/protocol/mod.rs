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
//! - [`PooledMqttClient`]: MQTT with connection pooling for multi-device scenarios
//!
//! # Connection Pooling
//!
//! When managing multiple Tasmota devices on the same MQTT broker, use
//! [`PooledMqttClient`] or [`BrokerPool`] to share connections efficiently.

mod broker_pool;
mod http;
mod mqtt;
mod mqtt_pooled;

pub use broker_pool::BrokerPool;
pub use http::{HttpClient, HttpClientBuilder};
pub use mqtt::{MqttClient, MqttClientBuilder};
pub use mqtt_pooled::PooledMqttClient;

use crate::command::Command;
use crate::error::ProtocolError;

/// Response from a Tasmota command.
#[derive(Debug, Clone)]
pub struct CommandResponse {
    /// The raw JSON response body.
    body: String,
}

impl CommandResponse {
    /// Creates a new command response with the given body.
    #[must_use]
    pub fn new(body: String) -> Self {
        Self { body }
    }

    /// Returns the raw JSON response body.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }

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
