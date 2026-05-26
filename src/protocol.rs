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
//! - [`SharedMqttClient`] (requires `mqtt` feature): MQTT-based communication for real-time updates
//!
//! # Feature Flags
//!
//! - `http` - Enables HTTP protocol support (enabled by default)
//! - `mqtt` - Enables MQTT protocol support (enabled by default)
//!
//! # Creating MQTT Devices
//!
//! Use [`MqttBroker`] to manage connections and create devices:
//!
//! ```no_run
//! use tasmor_lib::MqttBroker;
//!
//! # async fn example() -> tasmor_lib::Result<()> {
//! let broker = MqttBroker::builder()
//!     .host("192.168.1.50")
//!     .credentials("user", "pass")
//!     .build()
//!     .await?;
//!
//! // Create devices - credentials are inherited from broker
//! let (bulb, _) = broker.device("tasmota_bulb").build().await?;
//! let (plug, _) = broker.device("tasmota_plug").build().await?;
//! # Ok(())
//! # }
//! ```

#[cfg(feature = "http")]
mod http;
#[cfg(feature = "mqtt")]
mod mqtt_broker;
#[cfg(feature = "mqtt")]
mod response_collector;
#[cfg(feature = "mqtt")]
mod shared_mqtt_client;
#[cfg(feature = "mqtt")]
mod topic_router;

// Public configuration types (user-facing)
#[cfg(feature = "http")]
pub use http::HttpConfig;
#[cfg(feature = "mqtt")]
pub use mqtt_broker::{MqttBroker, MqttBrokerBuilder};

// Protocol clients - public because they're type parameters in Device<P>
// Users typically don't import these directly; they use Device::http() or MqttBroker::device()
#[cfg(feature = "http")]
pub use http::{HttpClient, HttpClientBuilder};
#[cfg(feature = "mqtt")]
pub use shared_mqtt_client::SharedMqttClient;

// Internal types - exposed for advanced usage but not re-exported at crate root
#[cfg(feature = "mqtt")]
pub use mqtt_broker::MqttBrokerConfig;
#[cfg(feature = "mqtt")]
pub use response_collector::ResponseSpec;
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
