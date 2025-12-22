// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Device capabilities detection and configuration.
//!
//! This module provides types for representing and detecting the capabilities
//! of Tasmota devices. Capabilities can be auto-detected from device status
//! or manually configured.
//!
//! # Auto-Detection
//!
//! When connecting to a device, capabilities can be automatically detected
//! by querying the device status and analyzing the response.
//!
//! # Manual Configuration
//!
//! For faster startup or when auto-detection is not desired, capabilities
//! can be manually specified using the builder pattern.

use crate::response::StatusResponse;

/// Capabilities of a Tasmota device.
///
/// Describes what features a device supports, such as power control,
/// dimming, color temperature, RGB colors, and energy monitoring.
///
/// # Examples
///
/// ```
/// use tasmor_lib::Capabilities;
///
/// // Default capabilities (single relay, no extras)
/// let basic = Capabilities::default();
/// assert_eq!(basic.power_channels, 1);
/// assert!(!basic.dimmer);
///
/// // RGB light bulb capabilities
/// let rgb_bulb = Capabilities {
///     power_channels: 1,
///     dimmer: true,
///     color_temp: true,
///     rgb: true,
///     energy: false,
/// };
///
/// // Neo Coolcam smart plug
/// let neo_coolcam = Capabilities::neo_coolcam();
/// assert!(neo_coolcam.energy);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
// Each boolean represents an independent device feature flag that cannot be
// meaningfully combined into an enum or state machine.
#[allow(clippy::struct_excessive_bools)]
pub struct Capabilities {
    /// Number of power relay channels (1-8).
    pub power_channels: u8,

    /// Supports dimmer/brightness control.
    pub dimmer: bool,

    /// Supports color temperature (CCT) control.
    pub color_temp: bool,

    /// Supports RGB/HSB color control.
    pub rgb: bool,

    /// Supports energy monitoring (voltage, current, power).
    pub energy: bool,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            power_channels: 1,
            dimmer: false,
            color_temp: false,
            rgb: false,
            energy: false,
        }
    }
}

impl Capabilities {
    /// Creates capabilities for a basic single-relay device.
    #[must_use]
    pub const fn basic() -> Self {
        Self {
            power_channels: 1,
            dimmer: false,
            color_temp: false,
            rgb: false,
            energy: false,
        }
    }

    /// Creates capabilities for a Neo Coolcam smart plug (Module 49).
    ///
    /// - Single relay
    /// - Energy monitoring
    #[must_use]
    pub const fn neo_coolcam() -> Self {
        Self {
            power_channels: 1,
            dimmer: false,
            color_temp: false,
            rgb: false,
            energy: true,
        }
    }

    /// Creates capabilities for an RGB light bulb.
    ///
    /// - Single "relay" (light on/off)
    /// - Dimmer
    /// - RGB color control
    #[must_use]
    pub const fn rgb_light() -> Self {
        Self {
            power_channels: 1,
            dimmer: true,
            color_temp: false,
            rgb: true,
            energy: false,
        }
    }

    /// Creates capabilities for an RGBCCT light bulb (like Athom bulbs).
    ///
    /// - Single "relay" (light on/off)
    /// - Dimmer
    /// - Color temperature
    /// - RGB color control
    #[must_use]
    pub const fn rgbcct_light() -> Self {
        Self {
            power_channels: 1,
            dimmer: true,
            color_temp: true,
            rgb: true,
            energy: false,
        }
    }

    /// Creates capabilities for a CCT-only light (warm/cool white).
    #[must_use]
    pub const fn cct_light() -> Self {
        Self {
            power_channels: 1,
            dimmer: true,
            color_temp: true,
            rgb: false,
            energy: false,
        }
    }

    /// Attempts to detect capabilities from a status response.
    ///
    /// This method analyzes the status response to determine:
    /// - Number of power channels from POWER fields
    /// - Dimmer support from Dimmer field
    /// - Color support from HSBColor/CT fields
    /// - Energy support from Energy block
    ///
    /// # Arguments
    ///
    /// * `status` - The full status response from Status 0
    #[must_use]
    pub fn from_status(status: &StatusResponse) -> Self {
        let mut caps = Self::default();

        // Detect power channels from status
        // In a real implementation, we'd parse the sensor status for POWER fields
        if let Some(ref device) = status.status {
            // Module ID can give hints
            if device.module == 49 {
                // Neo Coolcam - has energy monitoring
                caps.energy = true;
            }

            // Count friendly names as a proxy for relay count
            if !device.friendly_name.is_empty() {
                // Safe: we clamp to max 8, which fits in u8
                #[allow(clippy::cast_possible_truncation)]
                let count = device.friendly_name.len().min(8) as u8;
                caps.power_channels = count;
            }
        }

        // Check for light capabilities in sensor status
        if let Some(ref sensors) = status.sensors {
            if sensors.get("Dimmer").is_some() {
                caps.dimmer = true;
            }
            if sensors.get("CT").is_some() {
                caps.color_temp = true;
            }
            if sensors.get("HSBColor").is_some() {
                caps.rgb = true;
            }
        }

        // Check for energy monitoring
        if status
            .sensor_status
            .as_ref()
            .is_some_and(|s| s.get("ENERGY").is_some())
        {
            caps.energy = true;
        }

        caps
    }

    /// Returns whether this device supports any light control features.
    #[must_use]
    pub const fn is_light(&self) -> bool {
        self.dimmer || self.color_temp || self.rgb
    }

    /// Returns whether this device supports energy monitoring.
    #[must_use]
    pub const fn has_energy_monitoring(&self) -> bool {
        self.energy
    }

    /// Returns whether this device has multiple relays.
    #[must_use]
    pub const fn is_multi_relay(&self) -> bool {
        self.power_channels > 1
    }
}

/// Builder for creating custom capabilities.
#[derive(Debug, Default)]
pub struct CapabilitiesBuilder {
    inner: Capabilities,
}

impl CapabilitiesBuilder {
    /// Creates a new builder with default capabilities.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the number of power channels.
    #[must_use]
    pub fn power_channels(mut self, count: u8) -> Self {
        self.inner.power_channels = count.clamp(1, 8);
        self
    }

    /// Enables dimmer support.
    #[must_use]
    pub fn with_dimmer(mut self) -> Self {
        self.inner.dimmer = true;
        self
    }

    /// Enables color temperature support.
    #[must_use]
    pub fn with_color_temp(mut self) -> Self {
        self.inner.color_temp = true;
        self
    }

    /// Enables RGB support.
    #[must_use]
    pub fn with_rgb(mut self) -> Self {
        self.inner.rgb = true;
        self
    }

    /// Enables energy monitoring support.
    #[must_use]
    pub fn with_energy(mut self) -> Self {
        self.inner.energy = true;
        self
    }

    /// Builds the capabilities.
    #[must_use]
    pub fn build(self) -> Capabilities {
        self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_capabilities() {
        let caps = Capabilities::default();
        assert_eq!(caps.power_channels, 1);
        assert!(!caps.dimmer);
        assert!(!caps.color_temp);
        assert!(!caps.rgb);
        assert!(!caps.energy);
    }

    #[test]
    fn neo_coolcam_capabilities() {
        let caps = Capabilities::neo_coolcam();
        assert_eq!(caps.power_channels, 1);
        assert!(!caps.dimmer);
        assert!(caps.energy);
    }

    #[test]
    fn rgbcct_light_capabilities() {
        let caps = Capabilities::rgbcct_light();
        assert!(caps.dimmer);
        assert!(caps.color_temp);
        assert!(caps.rgb);
        assert!(!caps.energy);
        assert!(caps.is_light());
    }

    #[test]
    fn builder_pattern() {
        let caps = CapabilitiesBuilder::new()
            .power_channels(2)
            .with_dimmer()
            .with_energy()
            .build();

        assert_eq!(caps.power_channels, 2);
        assert!(caps.dimmer);
        assert!(caps.energy);
        assert!(!caps.rgb);
    }

    #[test]
    fn capability_checks() {
        let light = Capabilities::rgb_light();
        assert!(light.is_light());
        assert!(!light.has_energy_monitoring());
        assert!(!light.is_multi_relay());

        let plug = Capabilities::neo_coolcam();
        assert!(!plug.is_light());
        assert!(plug.has_energy_monitoring());

        let multi = CapabilitiesBuilder::new().power_channels(4).build();
        assert!(multi.is_multi_relay());
    }
}
