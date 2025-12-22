// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! High-level device abstraction for Tasmota devices.
//!
//! This module provides a unified API for interacting with Tasmota devices
//! regardless of the underlying protocol (HTTP or MQTT).

use std::sync::Arc;

use crate::capabilities::Capabilities;
use crate::command::{
    ColorTempCommand, Command, DimmerCommand, EnergyCommand, FadeCommand, HsbColorCommand,
    PowerCommand, PowerOnFadeCommand, SpeedCommand, StatusCommand,
};
use crate::error::{DeviceError, Error};
use crate::protocol::{CommandResponse, HttpClient, MqttClient, Protocol};
use crate::response::{EnergyResponse, PowerResponse, StatusResponse};
use crate::types::{ColorTemp, Dimmer, FadeSpeed, HsbColor, PowerIndex, PowerState};

/// A Tasmota device that can be controlled via HTTP or MQTT.
///
/// The `Device` struct provides a high-level API for controlling Tasmota devices,
/// abstracting away the underlying protocol details.
///
/// # Creating a Device
///
/// Use [`Device::http`] or [`Device::mqtt`] to create a device builder:
///
/// ```ignore
/// use tasmor_lib::Device;
///
/// // HTTP device with auto-detection
/// let device = Device::http("192.168.1.100")
///     .build()
///     .await?;
///
/// // HTTP device with manual capabilities
/// let device = Device::http("192.168.1.100")
///     .with_capabilities(Capabilities::rgbcct_light())
///     .build_without_probe()?;
/// ```
#[derive(Debug)]
pub struct Device<P: Protocol> {
    protocol: Arc<P>,
    capabilities: Capabilities,
}

impl<P: Protocol> Device<P> {
    /// Creates a new device with the specified protocol and capabilities.
    pub(crate) fn new(protocol: P, capabilities: Capabilities) -> Self {
        Self {
            protocol: Arc::new(protocol),
            capabilities,
        }
    }

    /// Returns the device capabilities.
    #[must_use]
    pub fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    /// Sends a command to the device.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn send_command<C: Command + Sync>(
        &self,
        command: &C,
    ) -> Result<CommandResponse, Error> {
        self.protocol
            .send_command(command)
            .await
            .map_err(Error::Protocol)
    }

    // ========== Power Control ==========

    /// Turns on a specific relay.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn power_on(&self) -> Result<PowerResponse, Error> {
        self.power_on_index(PowerIndex::one()).await
    }

    /// Turns on a specific relay by index.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn power_on_index(&self, index: PowerIndex) -> Result<PowerResponse, Error> {
        self.set_power(index, PowerState::On).await
    }

    /// Turns off a specific relay.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn power_off(&self) -> Result<PowerResponse, Error> {
        self.power_off_index(PowerIndex::one()).await
    }

    /// Turns off a specific relay by index.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn power_off_index(&self, index: PowerIndex) -> Result<PowerResponse, Error> {
        self.set_power(index, PowerState::Off).await
    }

    /// Toggles a specific relay.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn power_toggle(&self) -> Result<PowerResponse, Error> {
        self.power_toggle_index(PowerIndex::one()).await
    }

    /// Toggles a specific relay by index.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn power_toggle_index(&self, index: PowerIndex) -> Result<PowerResponse, Error> {
        let cmd = PowerCommand::Toggle { index };
        let response = self.send_command(&cmd).await?;
        response.parse().map_err(Error::Parse)
    }

    /// Sets the power state of a specific relay.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn set_power(
        &self,
        index: PowerIndex,
        state: PowerState,
    ) -> Result<PowerResponse, Error> {
        let cmd = PowerCommand::Set { index, state };
        let response = self.send_command(&cmd).await?;
        response.parse().map_err(Error::Parse)
    }

    /// Gets the current power state.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn get_power(&self) -> Result<PowerResponse, Error> {
        self.get_power_index(PowerIndex::one()).await
    }

    /// Gets the power state of a specific relay.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn get_power_index(&self, index: PowerIndex) -> Result<PowerResponse, Error> {
        let cmd = PowerCommand::Get { index };
        let response = self.send_command(&cmd).await?;
        response.parse().map_err(Error::Parse)
    }

    // ========== Status ==========

    /// Gets the full device status.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn status(&self) -> Result<StatusResponse, Error> {
        let cmd = StatusCommand::all();
        let response = self.send_command(&cmd).await?;
        response.parse().map_err(Error::Parse)
    }

    /// Gets the abbreviated device status.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn status_abbreviated(&self) -> Result<StatusResponse, Error> {
        let cmd = StatusCommand::abbreviated();
        let response = self.send_command(&cmd).await?;
        response.parse().map_err(Error::Parse)
    }

    // ========== Dimmer ==========

    /// Sets the dimmer level.
    ///
    /// # Errors
    ///
    /// Returns error if the device doesn't support dimming or the command fails.
    pub async fn set_dimmer(&self, value: Dimmer) -> Result<CommandResponse, Error> {
        self.check_capability("dimmer", self.capabilities.dimmer)?;
        let cmd = DimmerCommand::Set(value);
        self.send_command(&cmd).await
    }

    /// Gets the current dimmer level.
    ///
    /// # Errors
    ///
    /// Returns error if the device doesn't support dimming or the command fails.
    pub async fn get_dimmer(&self) -> Result<CommandResponse, Error> {
        self.check_capability("dimmer", self.capabilities.dimmer)?;
        let cmd = DimmerCommand::Get;
        self.send_command(&cmd).await
    }

    // ========== Color Temperature ==========

    /// Sets the color temperature.
    ///
    /// # Errors
    ///
    /// Returns error if the device doesn't support color temperature or the command fails.
    pub async fn set_color_temp(&self, value: ColorTemp) -> Result<CommandResponse, Error> {
        self.check_capability("color temperature", self.capabilities.color_temp)?;
        let cmd = ColorTempCommand::Set(value);
        self.send_command(&cmd).await
    }

    /// Gets the current color temperature.
    ///
    /// # Errors
    ///
    /// Returns error if the device doesn't support color temperature or the command fails.
    pub async fn get_color_temp(&self) -> Result<CommandResponse, Error> {
        self.check_capability("color temperature", self.capabilities.color_temp)?;
        let cmd = ColorTempCommand::Get;
        self.send_command(&cmd).await
    }

    // ========== HSB Color ==========

    /// Sets the HSB color.
    ///
    /// # Errors
    ///
    /// Returns error if the device doesn't support RGB or the command fails.
    pub async fn set_hsb_color(&self, color: HsbColor) -> Result<CommandResponse, Error> {
        self.check_capability("RGB color", self.capabilities.rgb)?;
        let cmd = HsbColorCommand::Set(color);
        self.send_command(&cmd).await
    }

    /// Gets the current HSB color.
    ///
    /// # Errors
    ///
    /// Returns error if the device doesn't support RGB or the command fails.
    pub async fn get_hsb_color(&self) -> Result<CommandResponse, Error> {
        self.check_capability("RGB color", self.capabilities.rgb)?;
        let cmd = HsbColorCommand::Get;
        self.send_command(&cmd).await
    }

    // ========== Fade ==========

    /// Enables fade transitions.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn enable_fade(&self) -> Result<CommandResponse, Error> {
        let cmd = FadeCommand::Enable;
        self.send_command(&cmd).await
    }

    /// Disables fade transitions.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn disable_fade(&self) -> Result<CommandResponse, Error> {
        let cmd = FadeCommand::Disable;
        self.send_command(&cmd).await
    }

    /// Sets the fade speed.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn set_speed(&self, speed: FadeSpeed) -> Result<CommandResponse, Error> {
        let cmd = SpeedCommand::Set(speed);
        self.send_command(&cmd).await
    }

    /// Enables fade on power-on.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn enable_power_on_fade(&self) -> Result<CommandResponse, Error> {
        let cmd = PowerOnFadeCommand::Enable;
        self.send_command(&cmd).await
    }

    /// Disables fade on power-on.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn disable_power_on_fade(&self) -> Result<CommandResponse, Error> {
        let cmd = PowerOnFadeCommand::Disable;
        self.send_command(&cmd).await
    }

    // ========== Energy Monitoring ==========

    /// Gets energy monitoring data.
    ///
    /// # Errors
    ///
    /// Returns error if the device doesn't support energy monitoring or the command fails.
    pub async fn energy(&self) -> Result<EnergyResponse, Error> {
        self.check_capability("energy monitoring", self.capabilities.energy)?;
        let cmd = EnergyCommand::Get;
        let response = self.send_command(&cmd).await?;
        response.parse().map_err(Error::Parse)
    }

    // ========== Helpers ==========

    /// Checks if a capability is supported.
    // Uses &self for method call syntax consistency, even though it only needs the parameters.
    #[allow(clippy::unused_self)]
    fn check_capability(&self, name: &str, supported: bool) -> Result<(), Error> {
        if supported {
            Ok(())
        } else {
            Err(Error::Device(DeviceError::UnsupportedCapability {
                capability: name.to_string(),
            }))
        }
    }
}

// ========== HTTP Device Builder ==========

impl Device<HttpClient> {
    /// Creates a builder for an HTTP-based device.
    #[must_use]
    pub fn http(host: impl Into<String>) -> HttpDeviceBuilder {
        HttpDeviceBuilder::new(host)
    }
}

/// Builder for creating HTTP-based devices.
#[derive(Debug)]
pub struct HttpDeviceBuilder {
    host: String,
    username: Option<String>,
    password: Option<String>,
    capabilities: Option<Capabilities>,
}

impl HttpDeviceBuilder {
    /// Creates a new builder for the specified host.
    fn new(host: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            username: None,
            password: None,
            capabilities: None,
        }
    }

    /// Sets authentication credentials.
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
    /// Returns error if connection or capability detection fails.
    pub async fn build(self) -> Result<Device<HttpClient>, Error> {
        let client = self.create_client()?;

        // Auto-detect capabilities
        let capabilities = if let Some(caps) = self.capabilities {
            caps
        } else {
            let cmd = StatusCommand::all();
            let response = client.send_command(&cmd).await.map_err(Error::Protocol)?;
            let status: StatusResponse = response.parse().map_err(Error::Parse)?;
            Capabilities::from_status(&status)
        };

        Ok(Device::new(client, capabilities))
    }

    /// Builds the device without probing for capabilities.
    ///
    /// Use this when you've set capabilities manually or want faster startup.
    ///
    /// # Errors
    ///
    /// Returns error if the HTTP client cannot be created.
    pub fn build_without_probe(self) -> Result<Device<HttpClient>, Error> {
        let client = self.create_client()?;
        let capabilities = self.capabilities.unwrap_or_default();
        Ok(Device::new(client, capabilities))
    }

    /// Creates the HTTP client.
    fn create_client(&self) -> Result<HttpClient, Error> {
        let mut client = HttpClient::new(&self.host).map_err(Error::Protocol)?;

        if let (Some(username), Some(password)) = (&self.username, &self.password) {
            client = client.with_credentials(username, password);
        }

        Ok(client)
    }
}

// ========== MQTT Device Builder ==========

impl Device<MqttClient> {
    /// Creates a builder for an MQTT-based device.
    #[must_use]
    pub fn mqtt(broker: impl Into<String>, topic: impl Into<String>) -> MqttDeviceBuilder {
        MqttDeviceBuilder::new(broker, topic)
    }
}

/// Builder for creating MQTT-based devices.
#[derive(Debug)]
pub struct MqttDeviceBuilder {
    broker: String,
    topic: String,
    capabilities: Option<Capabilities>,
}

impl MqttDeviceBuilder {
    /// Creates a new builder for the specified broker and topic.
    fn new(broker: impl Into<String>, topic: impl Into<String>) -> Self {
        Self {
            broker: broker.into(),
            topic: topic.into(),
            capabilities: None,
        }
    }

    /// Sets the device capabilities manually (skips auto-detection).
    #[must_use]
    pub fn with_capabilities(mut self, capabilities: Capabilities) -> Self {
        self.capabilities = Some(capabilities);
        self
    }

    /// Builds the device with auto-detection of capabilities.
    ///
    /// # Errors
    ///
    /// Returns error if connection or capability detection fails.
    pub async fn build(self) -> Result<Device<MqttClient>, Error> {
        let client = MqttClient::connect(&self.broker, &self.topic)
            .await
            .map_err(Error::Protocol)?;

        // Auto-detect capabilities
        let capabilities = if let Some(caps) = self.capabilities {
            caps
        } else {
            let cmd = StatusCommand::all();
            let response = client.send_command(&cmd).await.map_err(Error::Protocol)?;
            let status: StatusResponse = response.parse().map_err(Error::Parse)?;
            Capabilities::from_status(&status)
        };

        Ok(Device::new(client, capabilities))
    }

    /// Builds the device without probing for capabilities.
    ///
    /// # Errors
    ///
    /// Returns error if the MQTT client cannot be created.
    pub async fn build_without_probe(self) -> Result<Device<MqttClient>, Error> {
        let client = MqttClient::connect(&self.broker, &self.topic)
            .await
            .map_err(Error::Protocol)?;
        let capabilities = self.capabilities.unwrap_or_default();
        Ok(Device::new(client, capabilities))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_device_builder() {
        let builder = Device::<HttpClient>::http("192.168.1.100")
            .with_credentials("admin", "pass")
            .with_capabilities(Capabilities::neo_coolcam());

        assert_eq!(builder.host, "192.168.1.100");
        assert!(builder.capabilities.is_some());
    }

    #[test]
    fn mqtt_device_builder() {
        let builder = Device::<MqttClient>::mqtt("mqtt://broker:1883", "tasmota_switch")
            .with_capabilities(Capabilities::basic());

        assert_eq!(builder.broker, "mqtt://broker:1883");
        assert_eq!(builder.topic, "tasmota_switch");
    }
}
