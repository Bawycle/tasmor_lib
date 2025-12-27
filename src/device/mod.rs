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
//! let (device, _initial_state) = Device::http("192.168.1.100")
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
//! ```no_run
//! use tasmor_lib::Device;
//! use tasmor_lib::subscription::Subscribable;
//!
//! # async fn example() -> tasmor_lib::Result<()> {
//! let (device, _initial_state) = Device::mqtt("mqtt://192.168.1.50:1883", "tasmota_bedroom")
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

#[cfg(feature = "http")]
mod http_builder;
#[cfg(feature = "mqtt")]
mod mqtt_builder;

#[cfg(feature = "http")]
pub use http_builder::HttpDeviceBuilder;
#[cfg(feature = "mqtt")]
pub use mqtt_builder::MqttDeviceBuilder;

use std::sync::Arc;

use crate::capabilities::Capabilities;
use crate::command::{
    ColorTemperatureCommand, Command, DimmerCommand, EnergyCommand, FadeCommand, FadeSpeedCommand,
    HsbColorCommand, PowerCommand, SchemeCommand, StartupFadeCommand, StatusCommand,
    WakeupDurationCommand,
};
use crate::error::{DeviceError, Error};
#[cfg(feature = "http")]
use crate::protocol::HttpClient;
use crate::protocol::{CommandResponse, Protocol};
use crate::response::{
    ColorTemperatureResponse, DimmerResponse, EnergyResponse, FadeResponse, FadeSpeedResponse,
    HsbColorResponse, PowerResponse, RgbColorResponse, SchemeResponse, StartupFadeResponse,
    StatusResponse, WakeupDurationResponse,
};
use crate::state::DeviceState;
use crate::subscription::CallbackRegistry;
use crate::types::{
    ColorTemperature, Dimmer, FadeSpeed, HsbColor, PowerIndex, PowerState, RgbColor, Scheme,
    WakeupDuration,
};

/// A Tasmota device that can be controlled via HTTP or MQTT.
///
/// The `Device` struct provides a high-level API for controlling Tasmota devices,
/// abstracting away the underlying protocol details.
///
/// # Type Parameter
///
/// The type parameter `P` determines the underlying protocol:
/// - `HttpClient` for HTTP devices (no subscriptions)
/// - `MqttClient` for MQTT devices (supports subscriptions)
///
/// # Thread Safety
///
/// `Device<P>` is `Send + Sync` when the protocol `P` is `Send + Sync`.
/// Both `HttpClient` and `MqttClient` are `Send + Sync`, so devices can be
/// safely shared across threads and used in async contexts with Tokio.
///
/// ```
/// use tasmor_lib::Device;
/// use tasmor_lib::protocol::HttpClient;
///
/// fn assert_send_sync<T: Send + Sync>() {}
/// assert_send_sync::<Device<HttpClient>>();
/// ```
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
/// let (device, _initial_state) = Device::http("192.168.1.100")
///     .build()
///     .await?;
///
/// // HTTP device with manual capabilities
/// let (device, _initial_state) = Device::http("192.168.1.100")
///     .with_capabilities(Capabilities::rgbcct_light())
///     .build_without_probe()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct Device<P: Protocol> {
    protocol: Arc<P>,
    capabilities: Capabilities,
    callbacks: Arc<CallbackRegistry>,
}

impl<P: Protocol> Device<P> {
    /// Creates a new device with the specified protocol and capabilities.
    pub(crate) fn new(protocol: P, capabilities: Capabilities) -> Self {
        Self {
            protocol: Arc::new(protocol),
            capabilities,
            callbacks: Arc::new(CallbackRegistry::new()),
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
        let parsed: PowerResponse = response.parse().map_err(Error::Parse)?;

        // Dispatch callbacks for state changes
        self.apply_power_response(&parsed);

        Ok(parsed)
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
        let parsed: PowerResponse = response.parse().map_err(Error::Parse)?;

        // Dispatch callbacks for state changes
        self.apply_power_response(&parsed);

        Ok(parsed)
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
        let parsed: PowerResponse = response.parse().map_err(Error::Parse)?;

        // Dispatch callbacks for state changes
        self.apply_power_response(&parsed);

        Ok(parsed)
    }

    /// Dispatches power state changes from a response to callbacks.
    fn apply_power_response(&self, response: &PowerResponse) {
        for idx in 1..=8 {
            if let Ok(Some(power_state)) = response.power_state(idx) {
                let change = crate::state::StateChange::power(idx, power_state);
                self.callbacks.dispatch(&change);
            }
        }
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
        let parsed: DimmerResponse = response.parse().map_err(Error::Parse)?;

        // Dispatch callbacks for state changes
        self.apply_dimmer_response(&parsed);

        Ok(parsed)
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
        let parsed: DimmerResponse = response.parse().map_err(Error::Parse)?;

        // Dispatch callbacks for state changes
        self.apply_dimmer_response(&parsed);

        Ok(parsed)
    }

    /// Dispatches dimmer state changes from a response to callbacks.
    fn apply_dimmer_response(&self, response: &DimmerResponse) {
        if let Ok(dimmer) = Dimmer::new(response.dimmer()) {
            let change = crate::state::StateChange::dimmer(dimmer);
            self.callbacks.dispatch(&change);
        }

        if let Ok(Some(power)) = response.power_state() {
            let change = crate::state::StateChange::power(1, power);
            self.callbacks.dispatch(&change);
        }
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
        let parsed: ColorTemperatureResponse = response.parse().map_err(Error::Parse)?;

        // Dispatch callbacks for state changes
        self.apply_color_temperature_response(&parsed);

        Ok(parsed)
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
        let parsed: ColorTemperatureResponse = response.parse().map_err(Error::Parse)?;

        // Dispatch callbacks for state changes
        self.apply_color_temperature_response(&parsed);

        Ok(parsed)
    }

    /// Dispatches color temperature state changes from a response to callbacks.
    fn apply_color_temperature_response(&self, response: &ColorTemperatureResponse) {
        if let Ok(ct) = ColorTemperature::new(response.color_temperature()) {
            let change = crate::state::StateChange::color_temperature(ct);
            self.callbacks.dispatch(&change);
        }

        if let Ok(Some(power)) = response.power_state() {
            let change = crate::state::StateChange::power(1, power);
            self.callbacks.dispatch(&change);
        }
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
        let parsed: HsbColorResponse = response.parse().map_err(Error::Parse)?;

        // Dispatch callbacks for state changes
        self.apply_hsb_color_response(&parsed);

        Ok(parsed)
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
        let parsed: HsbColorResponse = response.parse().map_err(Error::Parse)?;

        // Dispatch callbacks for state changes
        self.apply_hsb_color_response(&parsed);

        Ok(parsed)
    }

    /// Dispatches HSB color state changes from a response to callbacks.
    fn apply_hsb_color_response(&self, response: &HsbColorResponse) {
        if let Ok(color) = response.hsb_color() {
            let change = crate::state::StateChange::hsb_color(color);
            self.callbacks.dispatch(&change);
        }

        if let Some(dimmer_value) = response.dimmer()
            && let Ok(dimmer) = Dimmer::new(dimmer_value)
        {
            let change = crate::state::StateChange::dimmer(dimmer);
            self.callbacks.dispatch(&change);
        }

        if let Ok(Some(power)) = response.power_state() {
            let change = crate::state::StateChange::power(1, power);
            self.callbacks.dispatch(&change);
        }
    }

    // ========== RGB Color ==========

    /// Sets the RGB color.
    ///
    /// This is a convenience method that converts the RGB color to HSB internally
    /// and sends an `HSBColor` command to the device. The response contains both
    /// the RGB and HSB representations.
    ///
    /// # Errors
    ///
    /// Returns error if the device doesn't support RGB or the command fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tasmor_lib::RgbColor;
    ///
    /// # async fn example(device: &tasmor_lib::Device<impl tasmor_lib::protocol::Protocol>) -> tasmor_lib::Result<()> {
    /// // Set color using hex string
    /// let color = RgbColor::from_hex("#FF5733")?;
    /// let response = device.set_rgb_color(color).await?;
    /// println!("Color set to: {}", response.to_hex_with_hash());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_rgb_color(&self, color: RgbColor) -> Result<RgbColorResponse, Error> {
        self.check_capability("RGB color", self.capabilities.supports_rgb_control())?;

        // Convert RGB to HSB and send the command
        let hsb = color.to_hsb();
        let cmd = HsbColorCommand::Set(hsb);
        let response = self.send_command(&cmd).await?;
        let hsb_response: HsbColorResponse = response.parse().map_err(Error::Parse)?;

        // Dispatch callbacks for state changes
        self.apply_hsb_color_response(&hsb_response);

        // Create RGB response preserving the original RGB value
        let returned_hsb = hsb_response.hsb_color().unwrap_or(hsb);
        Ok(RgbColorResponse::new(color, returned_hsb))
    }

    // ========== Scheme ==========

    /// Sets the light scheme/effect.
    ///
    /// Tasmota supports several built-in light schemes:
    /// - 0: Single (fixed color, default)
    /// - 1: Wakeup (gradual brightness increase, uses [`WakeupDuration`])
    /// - 2: Cycle Up (color cycling with increasing brightness)
    /// - 3: Cycle Down (color cycling with decreasing brightness)
    /// - 4: Random (random color changes)
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tasmor_lib::Scheme;
    ///
    /// # async fn example(device: &tasmor_lib::Device<impl tasmor_lib::protocol::Protocol>) -> tasmor_lib::Result<()> {
    /// // Set wakeup scheme
    /// device.set_scheme(Scheme::WAKEUP).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_scheme(&self, scheme: Scheme) -> Result<SchemeResponse, Error> {
        let cmd = SchemeCommand::Set(scheme);
        let response = self.send_command(&cmd).await?;
        let parsed: SchemeResponse = response.parse().map_err(Error::Parse)?;

        // Dispatch callbacks for state changes
        if let Ok(s) = parsed.scheme() {
            let change = crate::state::StateChange::scheme(s);
            self.callbacks.dispatch(&change);
        }

        Ok(parsed)
    }

    /// Gets the current light scheme.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn get_scheme(&self) -> Result<SchemeResponse, Error> {
        let cmd = SchemeCommand::Get;
        let response = self.send_command(&cmd).await?;
        let parsed: SchemeResponse = response.parse().map_err(Error::Parse)?;

        // Dispatch callbacks for state changes
        if let Ok(s) = parsed.scheme() {
            let change = crate::state::StateChange::scheme(s);
            self.callbacks.dispatch(&change);
        }

        Ok(parsed)
    }

    // ========== Wakeup Duration ==========

    /// Sets the wakeup duration.
    ///
    /// The wakeup duration controls how long Scheme 1 (Wakeup) takes to
    /// gradually increase brightness from 0 to the current dimmer level.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tasmor_lib::WakeupDuration;
    ///
    /// # async fn example(device: &tasmor_lib::Device<impl tasmor_lib::protocol::Protocol>) -> tasmor_lib::Result<()> {
    /// // Set wakeup duration to 5 minutes
    /// let duration = WakeupDuration::from_minutes(5)?;
    /// device.set_wakeup_duration(duration).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_wakeup_duration(
        &self,
        duration: WakeupDuration,
    ) -> Result<WakeupDurationResponse, Error> {
        let cmd = WakeupDurationCommand::Set(duration);
        let response = self.send_command(&cmd).await?;
        let parsed: WakeupDurationResponse = response.parse().map_err(Error::Parse)?;

        // Dispatch callbacks for state changes
        if let Ok(d) = parsed.duration() {
            let change = crate::state::StateChange::wakeup_duration(d);
            self.callbacks.dispatch(&change);
        }

        Ok(parsed)
    }

    /// Gets the current wakeup duration.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn get_wakeup_duration(&self) -> Result<WakeupDurationResponse, Error> {
        let cmd = WakeupDurationCommand::Get;
        let response = self.send_command(&cmd).await?;
        let parsed: WakeupDurationResponse = response.parse().map_err(Error::Parse)?;

        // Dispatch callbacks for state changes
        if let Ok(d) = parsed.duration() {
            let change = crate::state::StateChange::wakeup_duration(d);
            self.callbacks.dispatch(&change);
        }

        Ok(parsed)
    }

    // ========== Fade ==========

    /// Enables fade transitions.
    ///
    /// Returns a typed response indicating whether fade is now enabled.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn enable_fade(&self) -> Result<FadeResponse, Error> {
        let cmd = FadeCommand::Enable;
        let response = self.send_command(&cmd).await?;
        response.parse().map_err(Error::Parse)
    }

    /// Disables fade transitions.
    ///
    /// Returns a typed response indicating whether fade is now disabled.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn disable_fade(&self) -> Result<FadeResponse, Error> {
        let cmd = FadeCommand::Disable;
        let response = self.send_command(&cmd).await?;
        response.parse().map_err(Error::Parse)
    }

    /// Gets the current fade setting.
    ///
    /// Returns a typed response indicating whether fade is enabled.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn get_fade(&self) -> Result<FadeResponse, Error> {
        let cmd = FadeCommand::Get;
        let response = self.send_command(&cmd).await?;
        response.parse().map_err(Error::Parse)
    }

    /// Sets the fade transition speed.
    ///
    /// Returns a typed response with the new speed value.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn set_fade_speed(&self, speed: FadeSpeed) -> Result<FadeSpeedResponse, Error> {
        let cmd = FadeSpeedCommand::Set(speed);
        let response = self.send_command(&cmd).await?;
        response.parse().map_err(Error::Parse)
    }

    /// Gets the current fade speed setting.
    ///
    /// Returns a typed response with the current speed value.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn get_fade_speed(&self) -> Result<FadeSpeedResponse, Error> {
        let cmd = FadeSpeedCommand::Get;
        let response = self.send_command(&cmd).await?;
        response.parse().map_err(Error::Parse)
    }

    /// Enables fade at startup.
    ///
    /// Returns a typed response indicating whether startup fade is now enabled.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn enable_fade_at_startup(&self) -> Result<StartupFadeResponse, Error> {
        let cmd = StartupFadeCommand::Enable;
        let response = self.send_command(&cmd).await?;
        response.parse().map_err(Error::Parse)
    }

    /// Disables fade at startup.
    ///
    /// Returns a typed response indicating whether startup fade is now disabled.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn disable_fade_at_startup(&self) -> Result<StartupFadeResponse, Error> {
        let cmd = StartupFadeCommand::Disable;
        let response = self.send_command(&cmd).await?;
        response.parse().map_err(Error::Parse)
    }

    /// Gets the current fade at startup setting.
    ///
    /// Returns a typed response indicating whether startup fade is enabled.
    ///
    /// # Errors
    ///
    /// Returns error if the command fails.
    pub async fn get_fade_at_startup(&self) -> Result<StartupFadeResponse, Error> {
        let cmd = StartupFadeCommand::Get;
        let response = self.send_command(&cmd).await?;
        response.parse().map_err(Error::Parse)
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

    /// Resets the total energy counter to zero and returns the updated energy data.
    ///
    /// This resets both the total energy value and the `TotalStartTime` to the current time,
    /// then queries the device to return the updated energy data.
    ///
    /// # Errors
    ///
    /// Returns error if the device doesn't support energy monitoring or the command fails.
    pub async fn reset_energy_total(&self) -> Result<EnergyResponse, Error> {
        self.check_capability(
            "energy monitoring",
            self.capabilities.supports_energy_monitoring(),
        )?;

        // Send the reset command
        let cmd = EnergyCommand::ResetTotal;
        self.send_command(&cmd).await?;

        // Query and return the updated energy data
        let query_cmd = EnergyCommand::Get;
        let response = self.send_command(&query_cmd).await?;
        response.parse().map_err(Error::Parse)
    }

    // ========== Initial State Query ==========

    /// Queries the device for its current state.
    ///
    /// This method queries all supported capabilities and returns a complete
    /// `DeviceState` with the current values. It's called automatically by
    /// the device builders to provide initial state.
    ///
    /// # Errors
    ///
    /// Returns error if any of the queries fail.
    #[allow(clippy::cast_precision_loss)]
    pub async fn query_state(&self) -> Result<DeviceState, Error> {
        tracing::debug!(
            energy_monitoring = self.capabilities.supports_energy_monitoring(),
            dimmer = self.capabilities.supports_dimmer_control(),
            rgb = self.capabilities.supports_rgb_control(),
            cct = self.capabilities.supports_color_temperature_control(),
            "Querying device state"
        );

        let mut state = DeviceState::new();

        // Query power state
        match self.get_power().await {
            Ok(power_response) => {
                if let Ok(power_state) = power_response.first_power_state() {
                    tracing::debug!(?power_state, "Got power state");
                    state.set_power(1, power_state);
                }
            }
            Err(e) => tracing::debug!(error = %e, "Failed to get power state"),
        }

        // Query dimmer if supported
        if self.capabilities.supports_dimmer_control() {
            match self.get_dimmer().await {
                Ok(dimmer_response) => {
                    if let Ok(dimmer) = Dimmer::new(dimmer_response.dimmer()) {
                        tracing::debug!(dimmer = dimmer.value(), "Got dimmer");
                        state.set_dimmer(dimmer);
                    }
                }
                Err(e) => tracing::debug!(error = %e, "Failed to get dimmer"),
            }
        }

        // Query color temperature if supported
        if self.capabilities.supports_color_temperature_control() {
            match self.get_color_temperature().await {
                Ok(ct_response) => {
                    if let Ok(ct) = ColorTemperature::new(ct_response.color_temperature()) {
                        tracing::debug!(ct = ct.value(), "Got color temperature");
                        state.set_color_temperature(ct);
                    }
                }
                Err(e) => tracing::debug!(error = %e, "Failed to get color temperature"),
            }
        }

        // Query HSB color if supported
        if self.capabilities.supports_rgb_control() {
            match self.get_hsb_color().await {
                Ok(hsb_response) => {
                    if let Ok(hsb) = hsb_response.hsb_color() {
                        tracing::debug!(hue = hsb.hue(), sat = hsb.saturation(), "Got HSB color");
                        state.set_hsb_color(hsb);
                    }
                }
                Err(e) => tracing::debug!(error = %e, "Failed to get HSB color"),
            }
        }

        // Query energy data if supported
        if self.capabilities.supports_energy_monitoring() {
            tracing::debug!("Querying energy data");
            match self.energy().await {
                Ok(energy_response) => {
                    tracing::debug!(?energy_response, "Got energy response");
                    if let Some(energy) = energy_response.energy() {
                        tracing::debug!(
                            power = energy.power,
                            voltage = energy.voltage,
                            current = energy.current,
                            "Setting energy data"
                        );
                        state.set_power_consumption(energy.power as f32);
                        state.set_voltage(f32::from(energy.voltage));
                        state.set_current(energy.current);
                        state.set_energy_today(energy.today);
                        state.set_energy_yesterday(energy.yesterday);
                        state.set_energy_total(energy.total);
                        state.set_apparent_power(energy.apparent_power as f32);
                        state.set_reactive_power(energy.reactive_power as f32);
                        state.set_power_factor(energy.factor);
                        if let Some(start_time) = &energy.total_start_time {
                            state.set_total_start_time(start_time.clone());
                        }
                    } else {
                        tracing::debug!("Energy response has no energy data");
                    }
                }
                Err(e) => tracing::debug!(error = %e, "Failed to get energy data"),
            }
        }

        Ok(state)
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

#[cfg(feature = "http")]
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

#[cfg(feature = "mqtt")]
use crate::protocol::MqttClient;
#[cfg(feature = "mqtt")]
use crate::state::StateChange;
#[cfg(feature = "mqtt")]
use crate::subscription::{EnergyData, Subscribable, SubscriptionId};

#[cfg(feature = "mqtt")]
impl Device<MqttClient> {
    /// Registers the device's callbacks with the MQTT client for message routing.
    ///
    /// This is called automatically by the builder after device creation.
    pub(crate) fn register_mqtt_callbacks(&self) {
        self.protocol.register_callbacks(&self.callbacks);
    }
}

#[cfg(feature = "mqtt")]
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

    fn on_scheme_changed<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(Scheme) + Send + Sync + 'static,
    {
        self.callbacks.on_scheme_changed(callback)
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
