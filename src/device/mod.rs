// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! High-level device abstraction for Tasmota devices.
//!
//! This module provides a unified API for interacting with Tasmota devices
//! regardless of the underlying protocol (HTTP or MQTT).
//!
//! # Protocol Differences
//!
//! ## HTTP Devices
//!
//! HTTP devices are stateless - each command is an independent HTTP request.
//! They do not support real-time event subscriptions.
//!
//! ```no_run
//! use tasmor_lib::Device;
//!
//! # async fn example() -> tasmor_lib::Result<()> {
//! let device = Device::http("192.168.1.100")
//!     .with_credentials("admin", "password")
//!     .build()
//!     .await?;
//!
//! device.power_on().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## MQTT Devices
//!
//! MQTT devices maintain a persistent connection through a broker and support
//! real-time event subscriptions via the [`Subscribable`](crate::subscription::Subscribable) trait.
//!
//! ```ignore
//! use tasmor_lib::protocol::MqttBroker;
//! use tasmor_lib::Device;
//! use tasmor_lib::subscription::Subscribable;
//!
//! # async fn example() -> tasmor_lib::Result<()> {
//! let broker = MqttBroker::builder()
//!     .host("192.168.1.50")
//!     .port(1883)
//!     .build()
//!     .await?;
//!
//! let device = Device::mqtt(&broker, "tasmota_bedroom")
//!     .build()
//!     .await?;
//!
//! // MQTT devices support subscriptions
//! device.on_power_changed(|index, state| {
//!     println!("Power {index} is now {:?}", state);
//! });
//!
//! device.power_on().await?;
//! # Ok(())
//! # }
//! ```

mod http_builder;
mod mqtt_builder;

pub use http_builder::HttpDeviceBuilder;
pub use mqtt_builder::MqttDeviceBuilder;

use std::sync::Arc;

use parking_lot::RwLock;

use crate::capabilities::Capabilities;
use crate::command::{
    ColorTemperatureCommand, Command, DimmerCommand, EnergyCommand, FadeCommand, FadeSpeedCommand,
    HsbColorCommand, PowerCommand, StartupFadeCommand, StatusCommand,
};
use crate::error::{DeviceError, Error};
use crate::protocol::{CommandResponse, HttpClient, Protocol};
use crate::response::{
    ColorTemperatureResponse, DimmerResponse, EnergyResponse, HsbColorResponse, PowerResponse,
    StatusResponse,
};
use crate::state::DeviceState;
use crate::subscription::CallbackRegistry;
use crate::types::{ColorTemperature, Dimmer, FadeSpeed, HsbColor, PowerIndex, PowerState};

/// A Tasmota device that can be controlled via HTTP or MQTT.
///
/// The `Device` struct provides a high-level API for controlling Tasmota devices,
/// abstracting away the underlying protocol details.
///
/// # Type Parameter
///
/// The type parameter `P` determines the underlying protocol:
/// - `HttpClient` for HTTP devices (no subscriptions)
/// - `MqttDeviceHandle` for MQTT devices (supports subscriptions)
///
/// # Creating a Device
///
/// Use [`Device::http`] or [`Device::mqtt`] to create a device builder:
///
/// ```no_run
/// use tasmor_lib::{Device, Capabilities};
///
/// # async fn example() -> tasmor_lib::Result<()> {
/// // HTTP device with auto-detection
/// let device = Device::http("192.168.1.100")
///     .build()
///     .await?;
///
/// // HTTP device with manual capabilities
/// let device = Device::http("192.168.1.100")
///     .with_capabilities(Capabilities::rgbcct_light())
///     .build_without_probe()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct Device<P: Protocol> {
    protocol: Arc<P>,
    capabilities: Capabilities,
    state: Arc<RwLock<DeviceState>>,
    callbacks: Arc<CallbackRegistry>,
}

impl<P: Protocol> Device<P> {
    /// Creates a new device with the specified protocol and capabilities.
    pub(crate) fn new(protocol: P, capabilities: Capabilities) -> Self {
        Self {
            protocol: Arc::new(protocol),
            capabilities,
            state: Arc::new(RwLock::new(DeviceState::new())),
            callbacks: Arc::new(CallbackRegistry::new()),
        }
    }

    /// Returns the device capabilities.
    #[must_use]
    pub fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    /// Returns a snapshot of the current device state.
    ///
    /// For MQTT devices, this reflects the latest known state from telemetry.
    /// For HTTP devices, this is updated after each command response.
    #[must_use]
    pub fn state(&self) -> DeviceState {
        self.state.read().clone()
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
    /// Returns a typed response including the new dimmer level and power state.
    ///
    /// # Errors
    ///
    /// Returns error if the device doesn't support dimming or the command fails.
    pub async fn set_dimmer(&self, value: Dimmer) -> Result<DimmerResponse, Error> {
        self.check_capability("dimmer", self.capabilities.supports_dimmer_control())?;
        let cmd = DimmerCommand::Set(value);
        let response = self.send_command(&cmd).await?;
        response.parse().map_err(Error::Parse)
    }

    /// Gets the current dimmer level.
    ///
    /// Returns a typed response including the current dimmer level and power state.
    ///
    /// # Errors
    ///
    /// Returns error if the device doesn't support dimming or the command fails.
    pub async fn get_dimmer(&self) -> Result<DimmerResponse, Error> {
        self.check_capability("dimmer", self.capabilities.supports_dimmer_control())?;
        let cmd = DimmerCommand::Get;
        let response = self.send_command(&cmd).await?;
        response.parse().map_err(Error::Parse)
    }

    // ========== Color Temperature ==========

    /// Sets the color temperature.
    ///
    /// Returns a typed response including the new color temperature and power state.
    ///
    /// # Errors
    ///
    /// Returns error if the device doesn't support color temperature or the command fails.
    pub async fn set_color_temperature(
        &self,
        value: ColorTemperature,
    ) -> Result<ColorTemperatureResponse, Error> {
        self.check_capability(
            "color temperature",
            self.capabilities.supports_color_temperature_control(),
        )?;
        let cmd = ColorTemperatureCommand::Set(value);
        let response = self.send_command(&cmd).await?;
        response.parse().map_err(Error::Parse)
    }

    /// Gets the current color temperature.
    ///
    /// Returns a typed response including the current color temperature and power state.
    ///
    /// # Errors
    ///
    /// Returns error if the device doesn't support color temperature or the command fails.
    pub async fn get_color_temperature(&self) -> Result<ColorTemperatureResponse, Error> {
        self.check_capability(
            "color temperature",
            self.capabilities.supports_color_temperature_control(),
        )?;
        let cmd = ColorTemperatureCommand::Get;
        let response = self.send_command(&cmd).await?;
        response.parse().map_err(Error::Parse)
    }

    // ========== HSB Color ==========

    /// Sets the HSB color.
    ///
    /// Returns a typed response including the new HSB color, dimmer level, and power state.
    ///
    /// # Errors
    ///
    /// Returns error if the device doesn't support RGB or the command fails.
    pub async fn set_hsb_color(&self, color: HsbColor) -> Result<HsbColorResponse, Error> {
        self.check_capability("RGB color", self.capabilities.supports_rgb_control())?;
        let cmd = HsbColorCommand::Set(color);
        let response = self.send_command(&cmd).await?;
        response.parse().map_err(Error::Parse)
    }

    /// Gets the current HSB color.
    ///
    /// Returns a typed response including the current HSB color, dimmer level, and power state.
    ///
    /// # Errors
    ///
    /// Returns error if the device doesn't support RGB or the command fails.
    pub async fn get_hsb_color(&self) -> Result<HsbColorResponse, Error> {
        self.check_capability("RGB color", self.capabilities.supports_rgb_control())?;
        let cmd = HsbColorCommand::Get;
        let response = self.send_command(&cmd).await?;
        response.parse().map_err(Error::Parse)
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

    /// Sets the fade transition speed.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn set_fade_speed(&self, speed: FadeSpeed) -> Result<CommandResponse, Error> {
        let cmd = FadeSpeedCommand::Set(speed);
        self.send_command(&cmd).await
    }

    /// Enables fade at startup.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn enable_fade_at_startup(&self) -> Result<CommandResponse, Error> {
        let cmd = StartupFadeCommand::Enable;
        self.send_command(&cmd).await
    }

    /// Disables fade at startup.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn disable_fade_at_startup(&self) -> Result<CommandResponse, Error> {
        let cmd = StartupFadeCommand::Disable;
        self.send_command(&cmd).await
    }

    // ========== Energy Monitoring ==========

    /// Gets energy monitoring data.
    ///
    /// # Errors
    ///
    /// Returns error if the device doesn't support energy monitoring or the command fails.
    pub async fn energy(&self) -> Result<EnergyResponse, Error> {
        self.check_capability(
            "energy monitoring",
            self.capabilities.supports_energy_monitoring(),
        )?;
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

// ========== HTTP Device Entry Point ==========

impl Device<HttpClient> {
    /// Creates a builder for an HTTP-based device from a host string.
    ///
    /// This is a convenience method equivalent to `Device::http_config(HttpConfig::new(host))`.
    ///
    /// # Arguments
    ///
    /// * `host` - The hostname or IP address of the Tasmota device
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tasmor_lib::Device;
    ///
    /// # async fn example() -> tasmor_lib::Result<()> {
    /// let device = Device::http("192.168.1.100")
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn http(host: impl Into<String>) -> HttpDeviceBuilder {
        HttpDeviceBuilder::new(crate::protocol::HttpConfig::new(host))
    }

    /// Creates a builder for an HTTP-based device from an `HttpConfig`.
    ///
    /// Use this when you need to configure advanced options like port, HTTPS,
    /// or credentials at the configuration level.
    ///
    /// # Arguments
    ///
    /// * `config` - HTTP configuration including host, port, and credentials
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tasmor_lib::Device;
    /// use tasmor_lib::protocol::HttpConfig;
    ///
    /// # async fn example() -> tasmor_lib::Result<()> {
    /// let config = HttpConfig::new("192.168.1.100")
    ///     .with_port(8080)
    ///     .with_credentials("admin", "password");
    ///
    /// let device = Device::http_config(config)
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn http_config(config: crate::protocol::HttpConfig) -> HttpDeviceBuilder {
        HttpDeviceBuilder::new(config)
    }
}

// ========== MQTT Device Subscriptions ==========

use crate::protocol::MqttClient;
use crate::state::StateChange;
use crate::subscription::{EnergyData, Subscribable, SubscriptionId};

impl Device<MqttClient> {
    /// Registers the device's callbacks with the MQTT client for message routing.
    ///
    /// This is called automatically by the builder after device creation.
    pub(crate) fn register_mqtt_callbacks(&self) {
        self.protocol.register_callbacks(&self.callbacks);
    }
}

impl Subscribable for Device<MqttClient> {
    fn on_power_changed<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(u8, PowerState) + Send + Sync + 'static,
    {
        self.callbacks.on_power_changed(callback)
    }

    fn on_dimmer_changed<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(Dimmer) + Send + Sync + 'static,
    {
        self.callbacks.on_dimmer_changed(callback)
    }

    fn on_color_changed<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(HsbColor) + Send + Sync + 'static,
    {
        self.callbacks.on_hsb_color_changed(callback)
    }

    fn on_color_temp_changed<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(ColorTemperature) + Send + Sync + 'static,
    {
        self.callbacks.on_color_temp_changed(callback)
    }

    fn on_energy_updated<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(EnergyData) + Send + Sync + 'static,
    {
        self.callbacks.on_energy_updated(callback)
    }

    fn on_connected<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(&DeviceState) + Send + Sync + 'static,
    {
        self.callbacks.on_connected(callback)
    }

    fn on_disconnected<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.callbacks.on_disconnected(callback)
    }

    fn on_state_changed<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(&StateChange) + Send + Sync + 'static,
    {
        self.callbacks.on_state_changed(callback)
    }

    fn unsubscribe(&self, id: SubscriptionId) -> bool {
        self.callbacks.unsubscribe(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::HttpConfig;

    #[test]
    fn http_device_builder_from_config() {
        let config = HttpConfig::new("192.168.1.100").with_credentials("admin", "pass");

        let builder = Device::<HttpClient>::http_config(config)
            .with_capabilities(Capabilities::neo_coolcam());

        assert!(builder.capabilities().is_some());
    }

    #[test]
    fn http_device_builder_from_host() {
        let builder = Device::<HttpClient>::http("192.168.1.100")
            .with_credentials("admin", "pass")
            .with_capabilities(Capabilities::neo_coolcam());

        assert!(builder.capabilities().is_some());
    }

    #[test]
    fn device_state_default() {
        // This test verifies the Device struct can hold state
        let state = DeviceState::new();
        assert!(state.power(1).is_none());
    }
}
