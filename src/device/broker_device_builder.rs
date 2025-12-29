// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Builder for creating devices via an `MqttBroker` connection.

use crate::capabilities::Capabilities;
use crate::command::StatusCommand;
use crate::device::Device;
use crate::error::Error;
use crate::protocol::{MqttBroker, Protocol, SharedMqttClient};
use crate::response::StatusResponse;
use crate::state::DeviceState;

/// Builder for creating devices that share a broker's MQTT connection.
///
/// This builder is created via [`MqttBroker::device()`] and creates devices
/// that share the broker's MQTT connection instead of creating their own.
///
/// # Examples
///
/// ```no_run
/// use tasmor_lib::MqttBroker;
///
/// # async fn example() -> tasmor_lib::Result<()> {
/// let broker = MqttBroker::builder()
///     .host("192.168.1.50")
///     .credentials("user", "pass")
///     .build()
///     .await?;
///
/// // All devices share the same connection
/// let (bulb, _) = broker.device("tasmota_bulb").build().await?;
/// let (plug, _) = broker.device("tasmota_plug").build().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct BrokerDeviceBuilder<'a> {
    broker: &'a MqttBroker,
    topic: String,
    capabilities: Option<Capabilities>,
}

impl<'a> BrokerDeviceBuilder<'a> {
    /// Creates a new builder for a device on the given broker.
    pub(crate) fn new(broker: &'a MqttBroker, topic: impl Into<String>) -> Self {
        Self {
            broker,
            topic: topic.into(),
            capabilities: None,
        }
    }

    /// Sets the device capabilities manually (skips auto-detection).
    ///
    /// Use this when you know the device capabilities and want to avoid
    /// the initial status query.
    #[must_use]
    pub fn with_capabilities(mut self, capabilities: Capabilities) -> Self {
        self.capabilities = Some(capabilities);
        self
    }

    /// Builds the device with auto-detection of capabilities.
    ///
    /// This will query the device status to detect capabilities, then query
    /// the device for its current state (power, energy, colors, etc.).
    ///
    /// Returns a tuple of `(Device, DeviceState)` where `DeviceState` contains
    /// the initial values for all supported capabilities.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Subscription to device topics fails
    /// - Capability detection fails
    /// - Initial state query fails
    pub async fn build(self) -> Result<(Device<SharedMqttClient>, DeviceState), Error> {
        let client = self.create_client().await?;

        // Use provided capabilities or auto-detect
        let capabilities = if let Some(caps) = self.capabilities {
            caps
        } else {
            // Query device parameters (Status 1) for FriendlyName count
            let cmd = StatusCommand::device_parameters();
            let response = client.send_command(&cmd).await.map_err(Error::Protocol)?;
            let mut status: StatusResponse = response.parse().map_err(Error::Parse)?;

            // Query runtime state (Status 11) for light/energy capabilities
            let cmd_state = StatusCommand::state();
            if let Ok(state_response) = client.send_command(&cmd_state).await
                && let Ok(state_status) = state_response.parse::<StatusResponse>()
            {
                // Merge sensor_status from Status 11 into our status
                status.sensor_status = state_status.sensor_status;
            }

            // Query sensor info (Status 10) for ENERGY data
            let cmd_sensors = StatusCommand::sensors();
            if let Ok(sensors_response) = client.send_command(&cmd_sensors).await
                && let Ok(sensors_status) = sensors_response.parse::<StatusResponse>()
            {
                // Merge sensors from Status 10 into our status
                status.sensors = sensors_status.sensors;
            }

            Capabilities::from_status(&status)
        };

        let device = Device::new(client, capabilities);

        // Register callbacks with the MQTT client for message routing
        device.register_shared_mqtt_callbacks();

        // Query initial state
        let initial_state = device.query_state().await?;

        Ok((device, initial_state))
    }

    /// Builds the device without probing for capabilities.
    ///
    /// Use this when you've set capabilities manually via [`with_capabilities`](Self::with_capabilities).
    /// Still queries the device for its current state.
    ///
    /// Returns a tuple of `(Device, DeviceState)` where `DeviceState` contains
    /// the initial values for all supported capabilities.
    ///
    /// # Errors
    ///
    /// Returns error if subscription fails or state query fails.
    pub async fn build_without_probe(
        self,
    ) -> Result<(Device<SharedMqttClient>, DeviceState), Error> {
        let client = self.create_client().await?;
        let capabilities = self.capabilities.unwrap_or_default();

        let device = Device::new(client, capabilities);

        // Register callbacks with the MQTT client for message routing
        device.register_shared_mqtt_callbacks();

        // Query initial state
        let initial_state = device.query_state().await?;

        Ok((device, initial_state))
    }

    /// Creates the shared MQTT client using the broker's connection.
    async fn create_client(&self) -> Result<SharedMqttClient, Error> {
        // Add subscription to broker and get response channel
        let (response_rx, router) = self
            .broker
            .add_device_subscription(self.topic.clone())
            .await
            .map_err(Error::Protocol)?;

        // Small delay to ensure subscriptions are acknowledged
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        Ok(SharedMqttClient::new(
            self.broker.client().clone(),
            self.topic.clone(),
            response_rx,
            router,
        ))
    }
}
