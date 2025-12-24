// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Device model definitions and presets for supported Tasmota devices.

use serde::{Deserialize, Serialize};
use tasmor_lib::Capabilities;

/// Supported device models with their capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DeviceModel {
    /// Athom 5W/7W RGBCCT Smart Bulb
    #[default]
    AthomBulb5W7W,
    /// Athom 15W RGBCCT Smart Bulb
    AthomBulb15W,
    /// NOUS A1T Smart Plug with Energy Monitoring
    NousA1T,
}

impl DeviceModel {
    /// Returns all supported device models.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::AthomBulb5W7W, Self::AthomBulb15W, Self::NousA1T]
    }

    /// Returns the human-readable name of the device.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::AthomBulb5W7W => "Athom 5W/7W Bulb",
            Self::AthomBulb15W => "Athom 15W Bulb",
            Self::NousA1T => "NOUS A1T Smart Plug",
        }
    }

    /// Returns the capabilities for this device model.
    #[must_use]
    pub fn capabilities(self) -> Capabilities {
        match self {
            Self::AthomBulb5W7W | Self::AthomBulb15W => Capabilities::rgbcct_light(),
            Self::NousA1T => Capabilities::neo_coolcam(),
        }
    }

    /// Returns whether this device supports RGB color control.
    #[must_use]
    pub const fn supports_color(self) -> bool {
        matches!(self, Self::AthomBulb5W7W | Self::AthomBulb15W)
    }

    /// Returns whether this device supports dimming.
    #[must_use]
    pub const fn supports_dimming(self) -> bool {
        matches!(self, Self::AthomBulb5W7W | Self::AthomBulb15W)
    }

    /// Returns whether this device supports energy monitoring.
    #[must_use]
    pub const fn supports_energy_monitoring(self) -> bool {
        matches!(self, Self::NousA1T)
    }
}

impl std::fmt::Display for DeviceModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn athom_5w_7w_capabilities() {
        let model = DeviceModel::AthomBulb5W7W;
        let caps = model.capabilities();

        assert_eq!(caps.power_channels, 1);
        assert!(caps.rgb);
        assert!(caps.color_temp);
        assert!(caps.dimmer);
        assert!(!caps.energy);
    }

    #[test]
    fn athom_15w_capabilities() {
        let model = DeviceModel::AthomBulb15W;
        let caps = model.capabilities();

        assert_eq!(caps.power_channels, 1);
        assert!(caps.rgb);
        assert!(caps.color_temp);
        assert!(caps.dimmer);
        assert!(!caps.energy);
    }

    #[test]
    fn nous_a1t_capabilities() {
        let model = DeviceModel::NousA1T;
        let caps = model.capabilities();

        assert_eq!(caps.power_channels, 1);
        assert!(!caps.rgb);
        assert!(!caps.color_temp);
        assert!(!caps.dimmer);
        assert!(caps.energy);
    }

    #[test]
    fn device_model_display() {
        assert_eq!(DeviceModel::AthomBulb5W7W.to_string(), "Athom 5W/7W Bulb");
        assert_eq!(DeviceModel::AthomBulb15W.to_string(), "Athom 15W Bulb");
        assert_eq!(DeviceModel::NousA1T.to_string(), "NOUS A1T Smart Plug");
    }

    #[test]
    fn supports_features() {
        let bulb = DeviceModel::AthomBulb5W7W;
        assert!(bulb.supports_color());
        assert!(bulb.supports_dimming());
        assert!(!bulb.supports_energy_monitoring());

        let plug = DeviceModel::NousA1T;
        assert!(!plug.supports_color());
        assert!(!plug.supports_dimming());
        assert!(plug.supports_energy_monitoring());
    }

    #[test]
    fn all_models() {
        let models = DeviceModel::all();
        assert_eq!(models.len(), 3);
        assert!(models.contains(&DeviceModel::AthomBulb5W7W));
        assert!(models.contains(&DeviceModel::AthomBulb15W));
        assert!(models.contains(&DeviceModel::NousA1T));
    }
}
