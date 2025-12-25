// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! MQTT device builder.

use crate::capabilities::Capabilities;
use crate::command::StatusCommand;
use crate::device::Device;
use crate::error::Error;
use crate::protocol::{MqttClient, MqttClientBuilder, Protocol};
use crate::response::StatusResponse;

/// Builder for creating MQTT-based devices.
///
/// MQTT devices maintain a persistent connection through a broker and support
/// real-time event subscriptions via the [`Subscribable`](crate::subscription::Subscribable) trait.
///
/// # Examples
///
/// ```no_run
/// use tasmor_lib::Device;
///
/// # async fn example() -> tasmor_lib::Result<()> {
/// // Create an MQTT device
/// let device = Device::mqtt("mqtt://192.168.1.50:1883", "tasmota_switch")
///     .with_credentials("mqtt_user", "mqtt_password")
///     .build()
///     .await?;
///
/// device.power_toggle().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct MqttDeviceBuilder {
    broker: String,
    topic: String,
    username: Option<String>,
    password: Option<String>,
    capabilities: Option<Capabilities>,
}

impl MqttDeviceBuilder {
    /// Creates a new builder for the specified broker and topic.
    pub(crate) fn new(broker: impl Into<String>, topic: impl Into<String>) -> Self {
        Self {
            broker: broker.into(),
            topic: topic.into(),
            username: None,
            password: None,
            capabilities: None,
        }
    }

    /// Sets authentication credentials for the MQTT broker.
    ///
    /// Most MQTT brokers require authentication. Use this method to provide
    /// the username and password configured on your broker.
    ///
    /// # Arguments
    ///
    /// * `username` - MQTT broker username
    /// * `password` - MQTT broker password
    #[must_use]
    pub fn with_credentials(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
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
    /// This will query the device status to detect capabilities.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Connection to broker fails
    /// - Capability detection fails
    pub async fn build(self) -> Result<Device<MqttClient>, Error> {
        let client = self.create_client().await?;

        // Auto-detect capabilities if not set
        let capabilities = if let Some(caps) = self.capabilities {
            caps
        } else {
            let cmd = StatusCommand::all();
            let response = client.send_command(&cmd).await.map_err(Error::Protocol)?;
            let status: StatusResponse = response.parse().map_err(Error::Parse)?;
            Capabilities::from_status(&status)
        };

        let device = Device::new(client, capabilities);

        // Register callbacks with the MQTT client for message routing
        device.register_mqtt_callbacks();

        Ok(device)
    }

    /// Builds the device without probing for capabilities.
    ///
    /// Use this when you've set capabilities manually via [`with_capabilities`](Self::with_capabilities)
    /// or want faster startup.
    ///
    /// # Errors
    ///
    /// Returns error if the MQTT client cannot be created.
    pub async fn build_without_probe(self) -> Result<Device<MqttClient>, Error> {
        let client = self.create_client().await?;
        let capabilities = self.capabilities.unwrap_or_default();

        let device = Device::new(client, capabilities);

        // Register callbacks with the MQTT client for message routing
        device.register_mqtt_callbacks();

        Ok(device)
    }

    /// Creates the MQTT client with the configured options.
    async fn create_client(&self) -> Result<MqttClient, Error> {
        let mut builder = MqttClientBuilder::new()
            .broker(&self.broker)
            .device_topic(&self.topic);

        if let (Some(username), Some(password)) = (&self.username, &self.password) {
            builder = builder.credentials(username, password);
        }

        builder.build().await.map_err(Error::Protocol)
    }
}

// Entry point for MQTT devices
impl Device<MqttClient> {
    /// Creates a builder for an MQTT-based device.
    ///
    /// # Arguments
    ///
    /// * `broker` - The MQTT broker URL (e.g., `mqtt://192.168.1.50:1883`)
    /// * `topic` - The device's Tasmota topic (configured in Tasmota settings)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tasmor_lib::Device;
    ///
    /// # async fn example() -> tasmor_lib::Result<()> {
    /// let device = Device::mqtt("mqtt://192.168.1.50:1883", "tasmota_switch")
    ///     .with_credentials("mqtt_user", "mqtt_password")
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn mqtt(broker: impl Into<String>, topic: impl Into<String>) -> MqttDeviceBuilder {
        MqttDeviceBuilder::new(broker, topic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_new() {
        let builder = MqttDeviceBuilder::new("mqtt://localhost:1883", "device");
        assert_eq!(builder.broker, "mqtt://localhost:1883");
        assert_eq!(builder.topic, "device");
        assert!(builder.capabilities.is_none());
    }

    #[test]
    fn builder_with_credentials() {
        let builder = MqttDeviceBuilder::new("mqtt://localhost:1883", "device")
            .with_credentials("user", "pass");
        assert_eq!(builder.username, Some("user".to_string()));
        assert_eq!(builder.password, Some("pass".to_string()));
    }

    #[test]
    fn builder_with_capabilities() {
        let builder = MqttDeviceBuilder::new("mqtt://localhost:1883", "device")
            .with_capabilities(Capabilities::rgbcct_light());
        assert!(builder.capabilities.is_some());
    }

    #[test]
    fn builder_chain() {
        let builder = MqttDeviceBuilder::new("mqtt://192.168.1.50:1883", "tasmota_bulb")
            .with_credentials("mqtt_user", "mqtt_pass")
            .with_capabilities(Capabilities::basic());

        assert_eq!(builder.broker, "mqtt://192.168.1.50:1883");
        assert_eq!(builder.topic, "tasmota_bulb");
        assert!(builder.username.is_some());
        assert!(builder.password.is_some());
        assert!(builder.capabilities.is_some());
    }
}
