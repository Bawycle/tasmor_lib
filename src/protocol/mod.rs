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
//! - [`HttpClient`] (requires `http` feature): HTTP-based communication using REST API
//! - [`MqttClient`] (requires `mqtt` feature): MQTT-based communication for real-time updates
//! - [`PooledMqttClient`] (requires `mqtt` feature): MQTT with connection pooling
//!
//! # Feature Flags
//!
//! - `http` - Enables HTTP protocol support (enabled by default)
//! - `mqtt` - Enables MQTT protocol support (enabled by default)
//!
//! # Connection Pooling
//!
//! When managing multiple Tasmota devices on the same MQTT broker, use
//! [`PooledMqttClient`] or [`BrokerPool`] to share connections efficiently.

#[cfg(feature = "mqtt")]
mod broker_pool;
#[cfg(feature = "http")]
mod http;
#[cfg(feature = "mqtt")]
mod mqtt;
#[cfg(feature = "mqtt")]
mod mqtt_broker;
#[cfg(feature = "mqtt")]
mod mqtt_pooled;
#[cfg(feature = "mqtt")]
mod topic_router;

#[cfg(feature = "mqtt")]
pub use broker_pool::BrokerPool;
#[cfg(feature = "http")]
pub use http::{HttpClient, HttpClientBuilder, HttpConfig};
#[cfg(feature = "mqtt")]
pub use mqtt::{MqttClient, MqttClientBuilder};
#[cfg(feature = "mqtt")]
pub use mqtt_broker::{MqttBroker, MqttBrokerBuilder, MqttBrokerConfig};
#[cfg(feature = "mqtt")]
pub use mqtt_pooled::PooledMqttClient;
#[cfg(feature = "mqtt")]
pub use topic_router::TopicRouter;

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
///
/// All implementations must be `Send + Sync` to allow use in async contexts
/// and across thread boundaries.
#[allow(async_fn_in_trait)]
pub trait Protocol: Send + Sync {
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
