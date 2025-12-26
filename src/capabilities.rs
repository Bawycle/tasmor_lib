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
/// assert_eq!(basic.power_channels(), 1);
/// assert!(!basic.supports_dimmer_control());
///
/// // RGB light bulb capabilities using builder
/// let rgb_bulb = tasmor_lib::CapabilitiesBuilder::new()
///     .with_dimmer_control()
///     .with_color_temperature_control()
///     .with_rgb_control()
///     .build();
///
/// // Neo Coolcam smart plug
/// let neo_coolcam = Capabilities::neo_coolcam();
/// assert!(neo_coolcam.supports_energy_monitoring());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
// Each boolean represents an independent device feature flag that cannot be
// meaningfully combined into an enum or state machine.
#[allow(clippy::struct_excessive_bools)]
pub struct Capabilities {
    /// Number of power relay channels (1-8).
    power_channels: u8,

    /// Supports dimmer/brightness control.
    dimmer_control: bool,

    /// Supports color temperature (CCT) control.
    color_temperature_control: bool,

    /// Supports RGB/HSB color control.
    rgb_control: bool,

    /// Supports energy monitoring (voltage, current, power).
    energy_monitoring: bool,
}

impl Capabilities {
    /// Returns the number of power relay channels (1-8).
    #[must_use]
    pub const fn power_channels(&self) -> u8 {
        self.power_channels
    }

    /// Returns whether the device supports dimmer/brightness control.
    #[must_use]
    pub const fn supports_dimmer_control(&self) -> bool {
        self.dimmer_control
    }

    /// Returns whether the device supports color temperature (CCT) control.
    #[must_use]
    pub const fn supports_color_temperature_control(&self) -> bool {
        self.color_temperature_control
    }

    /// Returns whether the device supports RGB/HSB color control.
    #[must_use]
    pub const fn supports_rgb_control(&self) -> bool {
        self.rgb_control
    }

    /// Returns whether the device supports energy monitoring.
    #[must_use]
    pub const fn supports_energy_monitoring(&self) -> bool {
        self.energy_monitoring
    }

    /// Returns an iterator over the names of enabled features.
    ///
    /// This is useful for introspection and debugging. The returned names
    /// are: `dimmer_control`, `color_temperature_control`, `rgb_control`, `energy_monitoring`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::Capabilities;
    ///
    /// let caps = Capabilities::rgbcct_light();
    /// let features: Vec<_> = caps.features().collect();
    /// assert!(features.contains(&"dimmer_control"));
    /// assert!(features.contains(&"color_temperature_control"));
    /// assert!(features.contains(&"rgb_control"));
    /// assert!(!features.contains(&"energy_monitoring"));
    /// ```
    pub fn features(&self) -> impl Iterator<Item = &'static str> {
        [
            self.dimmer_control.then_some("dimmer_control"),
            self.color_temperature_control
                .then_some("color_temperature_control"),
            self.rgb_control.then_some("rgb_control"),
            self.energy_monitoring.then_some("energy_monitoring"),
        ]
        .into_iter()
        .flatten()
    }
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            power_channels: 1,
            dimmer_control: false,
            color_temperature_control: false,
            rgb_control: false,
            energy_monitoring: false,
        }
    }
}

impl Capabilities {
    /// Creates capabilities for a basic single-relay device.
    #[must_use]
    pub const fn basic() -> Self {
        Self {
            power_channels: 1,
            dimmer_control: false,
            color_temperature_control: false,
            rgb_control: false,
            energy_monitoring: false,
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
            dimmer_control: false,
            color_temperature_control: false,
            rgb_control: false,
            energy_monitoring: true,
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
            dimmer_control: true,
            color_temperature_control: false,
            rgb_control: true,
            energy_monitoring: false,
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
            dimmer_control: true,
            color_temperature_control: true,
            rgb_control: true,
            energy_monitoring: false,
        }
    }

    /// Creates capabilities for a CCT-only light (warm/cool white).
    #[must_use]
    pub const fn cct_light() -> Self {
        Self {
            power_channels: 1,
            dimmer_control: true,
            color_temperature_control: true,
            rgb_control: false,
            energy_monitoring: false,
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
                caps.energy_monitoring = true;
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
                caps.dimmer_control = true;
            }
            if sensors.get("CT").is_some() {
                caps.color_temperature_control = true;
            }
            if sensors.get("HSBColor").is_some() {
                caps.rgb_control = true;
            }
        }

        // Check for energy monitoring
        if status
            .sensor_status
            .as_ref()
            .is_some_and(|s| s.get("ENERGY").is_some())
        {
            caps.energy_monitoring = true;
        }

        caps
    }

    /// Returns whether this device supports any light control features.
    #[must_use]
    pub const fn is_light(&self) -> bool {
        self.dimmer_control || self.color_temperature_control || self.rgb_control
    }

    /// Returns whether this device supports energy monitoring.
    #[must_use]
    pub const fn has_energy_monitoring(&self) -> bool {
        self.energy_monitoring
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

    /// Enables dimmer control support.
    #[must_use]
    pub fn with_dimmer_control(mut self) -> Self {
        self.inner.dimmer_control = true;
        self
    }

    /// Enables color temperature control support.
    #[must_use]
    pub fn with_color_temperature_control(mut self) -> Self {
        self.inner.color_temperature_control = true;
        self
    }

    /// Enables RGB control support.
    #[must_use]
    pub fn with_rgb_control(mut self) -> Self {
        self.inner.rgb_control = true;
        self
    }

    /// Enables energy monitoring support.
    #[must_use]
    pub fn with_energy_monitoring(mut self) -> Self {
        self.inner.energy_monitoring = true;
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
    use crate::response::StatusResponse;

    #[test]
    fn default_capabilities() {
        let caps = Capabilities::default();
        assert_eq!(caps.power_channels, 1);
        assert!(!caps.dimmer_control);
        assert!(!caps.color_temperature_control);
        assert!(!caps.rgb_control);
        assert!(!caps.energy_monitoring);
    }

    #[test]
    fn neo_coolcam_capabilities() {
        let caps = Capabilities::neo_coolcam();
        assert_eq!(caps.power_channels, 1);
        assert!(!caps.dimmer_control);
        assert!(caps.energy_monitoring);
    }

    #[test]
    fn rgbcct_light_capabilities() {
        let caps = Capabilities::rgbcct_light();
        assert!(caps.dimmer_control);
        assert!(caps.color_temperature_control);
        assert!(caps.rgb_control);
        assert!(!caps.energy_monitoring);
        assert!(caps.is_light());
    }

    #[test]
    fn builder_pattern() {
        let caps = CapabilitiesBuilder::new()
            .power_channels(2)
            .with_dimmer_control()
            .with_energy_monitoring()
            .build();

        assert_eq!(caps.power_channels, 2);
        assert!(caps.dimmer_control);
        assert!(caps.energy_monitoring);
        assert!(!caps.rgb_control);
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

    #[test]
    fn features_iterator() {
        // No features enabled
        let basic = Capabilities::basic();
        assert_eq!(basic.features().count(), 0);

        // Some features enabled
        let rgb = Capabilities::rgb_light();
        let features: Vec<_> = rgb.features().collect();
        assert_eq!(features.len(), 2);
        assert!(features.contains(&"dimmer_control"));
        assert!(features.contains(&"rgb_control"));

        // All features enabled
        let full = CapabilitiesBuilder::new()
            .with_dimmer_control()
            .with_color_temperature_control()
            .with_rgb_control()
            .with_energy_monitoring()
            .build();
        let all_features: Vec<_> = full.features().collect();
        assert_eq!(all_features.len(), 4);
        assert!(all_features.contains(&"dimmer_control"));
        assert!(all_features.contains(&"color_temperature_control"));
        assert!(all_features.contains(&"rgb_control"));
        assert!(all_features.contains(&"energy_monitoring"));
    }

    // ========================================================================
    // Tests for Capabilities::from_status() based on real Tasmota responses
    // Reference: https://tasmota.github.io/docs/JSON-Status-Responses/
    // ========================================================================

    #[test]
    fn from_status_detects_neo_coolcam_by_module_id() {
        // Neo Coolcam Power Plug has Module ID 49
        // Reference: https://tasmota.github.io/docs/Commands/#management
        let json = r#"{
            "Status": {
                "Module": 49,
                "DeviceName": "Neo Coolcam Plug",
                "FriendlyName": ["Plug"],
                "Topic": "tasmota_plug",
                "Power": 1
            }
        }"#;

        let status: StatusResponse = serde_json::from_str(json).unwrap();
        let caps = Capabilities::from_status(&status);

        assert!(
            caps.energy_monitoring,
            "Neo Coolcam (Module 49) should have energy monitoring"
        );
        assert_eq!(caps.power_channels, 1);
    }

    #[test]
    fn from_status_detects_multi_relay_from_friendly_names() {
        // Tasmota uses FriendlyName array to indicate multiple relays
        // Reference: https://tasmota.github.io/docs/JSON-Status-Responses/
        // Status response example: {"Status": {"FriendlyName": ["Relay1", "Relay2", "Relay3", "Relay4"]}}
        let json = r#"{
            "Status": {
                "Module": 18,
                "DeviceName": "4CH Pro",
                "FriendlyName": ["Relay 1", "Relay 2", "Relay 3", "Relay 4"],
                "Topic": "tasmota_4ch"
            }
        }"#;

        let status: StatusResponse = serde_json::from_str(json).unwrap();
        let caps = Capabilities::from_status(&status);

        assert_eq!(caps.power_channels, 4);
        assert!(caps.is_multi_relay());
    }

    #[test]
    fn from_status_detects_energy_from_status_sns() {
        // Energy data appears in StatusSNS (Status 10) with ENERGY object
        // Reference: https://tasmota.github.io/docs/JSON-Status-Responses/
        // Example: "ENERGY": {"Total": 3.185, "Yesterday": 3.058, "Today": 0.127, "Power": 45, "Voltage": 230}
        let json = r#"{
            "Status": {
                "Module": 18,
                "DeviceName": "Smart Plug",
                "FriendlyName": ["Plug"]
            },
            "StatusSTS": {
                "POWER": "ON",
                "ENERGY": {
                    "Total": 3.185,
                    "Yesterday": 3.058,
                    "Today": 0.127,
                    "Power": 45,
                    "Factor": 0.95,
                    "Voltage": 230,
                    "Current": 0.195
                }
            }
        }"#;

        let status: StatusResponse = serde_json::from_str(json).unwrap();
        let caps = Capabilities::from_status(&status);

        assert!(
            caps.energy_monitoring,
            "Device with ENERGY in StatusSTS should have energy monitoring"
        );
    }

    #[test]
    fn from_status_detects_dimmer_capability() {
        // Light devices report Dimmer in sensor status
        // Reference: https://tasmota.github.io/docs/Lights/
        // Dimmer range: 0-100%
        let json = r#"{
            "Status": {
                "Module": 18,
                "DeviceName": "Dimmable Light",
                "FriendlyName": ["Light"]
            },
            "StatusSNS": {
                "Time": "2024-01-15T12:00:00",
                "Dimmer": 75
            }
        }"#;

        let status: StatusResponse = serde_json::from_str(json).unwrap();
        let caps = Capabilities::from_status(&status);

        assert!(
            caps.dimmer_control,
            "Device with Dimmer in StatusSNS should have dimmer capability"
        );
        assert!(caps.is_light());
    }

    #[test]
    fn from_status_detects_color_temperature_capability() {
        // CCT lights report CT (color temperature) in mireds (153-500)
        // Reference: https://tasmota.github.io/docs/Lights/
        // CT 153 = 6500K (cold white), CT 500 = 2000K (warm white)
        let json = r#"{
            "Status": {
                "Module": 18,
                "DeviceName": "CCT Bulb",
                "FriendlyName": ["Bulb"]
            },
            "StatusSNS": {
                "Time": "2024-01-15T12:00:00",
                "CT": 250
            }
        }"#;

        let status: StatusResponse = serde_json::from_str(json).unwrap();
        let caps = Capabilities::from_status(&status);

        assert!(
            caps.color_temperature_control,
            "Device with CT in StatusSNS should have color temperature capability"
        );
        assert!(caps.is_light());
    }

    #[test]
    fn from_status_detects_rgb_capability() {
        // RGB lights report HSBColor as "Hue,Saturation,Brightness"
        // Reference: https://tasmota.github.io/docs/Lights/
        // Example: "HSBColor": "180,100,100" (Hue=180°, Sat=100%, Bright=100%)
        let json = r#"{
            "Status": {
                "Module": 18,
                "DeviceName": "RGB Bulb",
                "FriendlyName": ["Bulb"]
            },
            "StatusSNS": {
                "Time": "2024-01-15T12:00:00",
                "HSBColor": "180,100,100"
            }
        }"#;

        let status: StatusResponse = serde_json::from_str(json).unwrap();
        let caps = Capabilities::from_status(&status);

        assert!(
            caps.rgb_control,
            "Device with HSBColor in StatusSNS should have RGB capability"
        );
        assert!(caps.is_light());
    }

    #[test]
    fn from_status_detects_full_rgbcct_light() {
        // RGBCCT lights (5-channel) have Dimmer, CT, and HSBColor
        // Reference: https://tasmota.github.io/docs/Lights/
        // Response format from tele/STATE or Status 11
        let json = r#"{
            "Status": {
                "Module": 18,
                "DeviceName": "RGBCCT Bulb",
                "FriendlyName": ["Smart Bulb"]
            },
            "StatusSNS": {
                "Time": "2024-01-15T12:00:00",
                "Dimmer": 100,
                "Color": "255,128,64,200,100",
                "HSBColor": "20,75,100",
                "White": 78,
                "CT": 300,
                "Channel": [100, 50, 25, 78, 39]
            }
        }"#;

        let status: StatusResponse = serde_json::from_str(json).unwrap();
        let caps = Capabilities::from_status(&status);

        assert!(caps.dimmer_control, "RGBCCT light should have dimmer");
        assert!(
            caps.color_temperature_control,
            "RGBCCT light should have color temperature"
        );
        assert!(caps.rgb_control, "RGBCCT light should have RGB");
        assert!(caps.is_light());
    }

    #[test]
    fn from_status_basic_switch_no_special_capabilities() {
        // Basic switch/relay with no light or energy features
        let json = r#"{
            "Status": {
                "Module": 1,
                "DeviceName": "Basic Switch",
                "FriendlyName": ["Switch"],
                "Topic": "tasmota_switch",
                "Power": 0
            }
        }"#;

        let status: StatusResponse = serde_json::from_str(json).unwrap();
        let caps = Capabilities::from_status(&status);

        assert_eq!(caps.power_channels, 1);
        assert!(!caps.dimmer_control);
        assert!(!caps.color_temperature_control);
        assert!(!caps.rgb_control);
        assert!(!caps.energy_monitoring);
        assert!(!caps.is_light());
    }

    #[test]
    fn from_status_power_channels_clamped_to_8() {
        // Tasmota supports max 8 relays (POWER1-POWER8)
        // Reference: https://tasmota.github.io/docs/Commands/#power
        let json = r#"{
            "Status": {
                "Module": 18,
                "DeviceName": "Many Relays",
                "FriendlyName": ["R1", "R2", "R3", "R4", "R5", "R6", "R7", "R8", "R9", "R10"]
            }
        }"#;

        let status: StatusResponse = serde_json::from_str(json).unwrap();
        let caps = Capabilities::from_status(&status);

        assert_eq!(
            caps.power_channels, 8,
            "Power channels should be clamped to max 8"
        );
    }

    #[test]
    fn from_status_empty_response() {
        // Handle gracefully when status response has no data
        let json = r#"{}"#;

        let status: StatusResponse = serde_json::from_str(json).unwrap();
        let caps = Capabilities::from_status(&status);

        // Should return defaults
        assert_eq!(caps.power_channels, 1);
        assert!(!caps.dimmer_control);
        assert!(!caps.color_temperature_control);
        assert!(!caps.rgb_control);
        assert!(!caps.energy_monitoring);
    }

    #[test]
    fn builder_with_color_temperature_control() {
        let caps = CapabilitiesBuilder::new()
            .with_color_temperature_control()
            .build();

        assert!(caps.color_temperature_control);
        assert!(caps.is_light());
    }

    #[test]
    fn builder_with_rgb_control() {
        let caps = CapabilitiesBuilder::new().with_rgb_control().build();

        assert!(caps.rgb_control);
        assert!(caps.is_light());
    }
}
