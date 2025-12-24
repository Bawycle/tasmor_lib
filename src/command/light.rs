// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Light control commands.
//!
//! This module provides commands for controlling light brightness, color
//! temperature, HSB color, and transition speed.

use crate::command::Command;
use crate::types::{ColorTemp, Dimmer, FadeSpeed, HsbColor};

/// Command to control dimmer/brightness level.
///
/// # Examples
///
/// ```
/// use tasmor_lib::command::{Command, DimmerCommand};
/// use tasmor_lib::types::Dimmer;
///
/// // Set brightness to 75%
/// let cmd = DimmerCommand::Set(Dimmer::new(75).unwrap());
/// assert_eq!(cmd.name(), "Dimmer");
/// assert_eq!(cmd.payload(), Some("75".to_string()));
///
/// // Query current brightness
/// let query = DimmerCommand::Get;
/// assert_eq!(query.payload(), None);
///
/// // Increase brightness by step
/// let inc = DimmerCommand::Increase;
/// assert_eq!(inc.payload(), Some("+".to_string()));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimmerCommand {
    /// Query the current dimmer value.
    Get,
    /// Set the dimmer to a specific value.
    Set(Dimmer),
    /// Increase brightness by `DimmerStep`.
    Increase,
    /// Decrease brightness by `DimmerStep`.
    Decrease,
    /// Decrease to minimum (1).
    Minimum,
    /// Increase to maximum (100).
    Maximum,
    /// Stop a fade in progress.
    Stop,
}

impl DimmerCommand {
    /// Creates a command to set a specific brightness.
    #[must_use]
    pub const fn set(value: Dimmer) -> Self {
        Self::Set(value)
    }
}

impl Command for DimmerCommand {
    fn name(&self) -> String {
        "Dimmer".to_string()
    }

    fn payload(&self) -> Option<String> {
        match self {
            Self::Get => None,
            Self::Set(dim) => Some(dim.value().to_string()),
            Self::Increase => Some("+".to_string()),
            Self::Decrease => Some("-".to_string()),
            Self::Minimum => Some("<".to_string()),
            Self::Maximum => Some(">".to_string()),
            Self::Stop => Some("!".to_string()),
        }
    }
}

/// Command to control color temperature.
///
/// # Examples
///
/// ```
/// use tasmor_lib::command::{Command, ColorTempCommand};
/// use tasmor_lib::types::ColorTemp;
///
/// // Set to neutral white
/// let cmd = ColorTempCommand::Set(ColorTemp::NEUTRAL);
/// assert_eq!(cmd.name(), "CT");
/// assert_eq!(cmd.payload(), Some("250".to_string()));
///
/// // Increase color temperature (warmer)
/// let warmer = ColorTempCommand::Increase;
/// assert_eq!(warmer.payload(), Some("+".to_string()));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorTempCommand {
    /// Query the current color temperature.
    Get,
    /// Set color temperature to a specific value.
    Set(ColorTemp),
    /// Increase color temperature by 34 (warmer).
    Increase,
    /// Decrease color temperature by 34 (cooler).
    Decrease,
}

impl ColorTempCommand {
    /// Creates a command to set a specific color temperature.
    #[must_use]
    pub const fn set(value: ColorTemp) -> Self {
        Self::Set(value)
    }
}

impl Command for ColorTempCommand {
    fn name(&self) -> String {
        "CT".to_string()
    }

    fn payload(&self) -> Option<String> {
        match self {
            Self::Get => None,
            Self::Set(ct) => Some(ct.value().to_string()),
            Self::Increase => Some("+".to_string()),
            Self::Decrease => Some("-".to_string()),
        }
    }
}

/// Command to control HSB (Hue, Saturation, Brightness) color.
///
/// # Examples
///
/// ```
/// use tasmor_lib::command::{Command, HsbColorCommand};
/// use tasmor_lib::types::HsbColor;
///
/// // Set to pure green
/// let cmd = HsbColorCommand::Set(HsbColor::green());
/// assert_eq!(cmd.name(), "HSBColor");
/// assert_eq!(cmd.payload(), Some("120,100,100".to_string()));
///
/// // Set only hue
/// let hue = HsbColorCommand::SetHue(180);
/// assert_eq!(hue.name(), "HSBColor1");
/// assert_eq!(hue.payload(), Some("180".to_string()));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HsbColorCommand {
    /// Query the current HSB color.
    Get,
    /// Set the complete HSB color.
    Set(HsbColor),
    /// Set only the hue (0-360).
    SetHue(u16),
    /// Set only the saturation (0-100).
    SetSaturation(u8),
    /// Set only the brightness (0-100).
    SetBrightness(u8),
}

impl HsbColorCommand {
    /// Creates a command to set a complete HSB color.
    #[must_use]
    pub const fn set(color: HsbColor) -> Self {
        Self::Set(color)
    }

    /// Creates a command to set only the hue.
    #[must_use]
    pub const fn hue(value: u16) -> Self {
        Self::SetHue(value)
    }

    /// Creates a command to set only the saturation.
    #[must_use]
    pub const fn saturation(value: u8) -> Self {
        Self::SetSaturation(value)
    }

    /// Creates a command to set only the brightness.
    #[must_use]
    pub const fn brightness(value: u8) -> Self {
        Self::SetBrightness(value)
    }
}

impl Command for HsbColorCommand {
    fn name(&self) -> String {
        match self {
            Self::Get | Self::Set(_) => "HSBColor".to_string(),
            Self::SetHue(_) => "HSBColor1".to_string(),
            Self::SetSaturation(_) => "HSBColor2".to_string(),
            Self::SetBrightness(_) => "HSBColor3".to_string(),
        }
    }

    fn payload(&self) -> Option<String> {
        match self {
            Self::Get => None,
            Self::Set(color) => Some(color.to_command_string()),
            Self::SetHue(h) => Some(h.to_string()),
            Self::SetSaturation(s) => Some(s.to_string()),
            Self::SetBrightness(b) => Some(b.to_string()),
        }
    }
}

/// Command to control fade/transition speed.
///
/// # Examples
///
/// ```
/// use tasmor_lib::command::{Command, SpeedCommand};
/// use tasmor_lib::types::FadeSpeed;
///
/// // Set medium speed
/// let cmd = SpeedCommand::Set(FadeSpeed::MEDIUM);
/// assert_eq!(cmd.name(), "Speed");
/// assert_eq!(cmd.payload(), Some("20".to_string()));
///
/// // Increase speed (slower transitions)
/// let slower = SpeedCommand::Increase;
/// assert_eq!(slower.payload(), Some("+".to_string()));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeedCommand {
    /// Query the current speed setting.
    Get,
    /// Set the speed to a specific value.
    Set(FadeSpeed),
    /// Increase speed value (slower transitions).
    Increase,
    /// Decrease speed value (faster transitions).
    Decrease,
}

impl SpeedCommand {
    /// Creates a command to set a specific speed.
    #[must_use]
    pub const fn set(value: FadeSpeed) -> Self {
        Self::Set(value)
    }
}

impl Command for SpeedCommand {
    fn name(&self) -> String {
        "Speed".to_string()
    }

    fn payload(&self) -> Option<String> {
        match self {
            Self::Get => None,
            Self::Set(speed) => Some(speed.value().to_string()),
            Self::Increase => Some("+".to_string()),
            Self::Decrease => Some("-".to_string()),
        }
    }
}

/// Command to query the current device state.
///
/// The `State` command returns all current light settings including:
/// - Power state
/// - Dimmer level
/// - Color temperature (CT)
/// - HSB color
/// - Fade/Speed settings
///
/// This is useful for synchronizing local state with the device,
/// especially after establishing a connection.
///
/// # Examples
///
/// ```
/// use tasmor_lib::command::{Command, StateCommand};
///
/// let cmd = StateCommand;
/// assert_eq!(cmd.name(), "State");
/// assert_eq!(cmd.payload(), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StateCommand;

impl Command for StateCommand {
    fn name(&self) -> String {
        "State".to_string()
    }

    fn payload(&self) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimmer_command_set() {
        let cmd = DimmerCommand::Set(Dimmer::new(50).unwrap());
        assert_eq!(cmd.name(), "Dimmer");
        assert_eq!(cmd.payload(), Some("50".to_string()));
    }

    #[test]
    fn dimmer_command_adjustments() {
        assert_eq!(DimmerCommand::Increase.payload(), Some("+".to_string()));
        assert_eq!(DimmerCommand::Decrease.payload(), Some("-".to_string()));
        assert_eq!(DimmerCommand::Minimum.payload(), Some("<".to_string()));
        assert_eq!(DimmerCommand::Maximum.payload(), Some(">".to_string()));
        assert_eq!(DimmerCommand::Stop.payload(), Some("!".to_string()));
    }

    #[test]
    fn color_temp_command_set() {
        let cmd = ColorTempCommand::Set(ColorTemp::COOL);
        assert_eq!(cmd.name(), "CT");
        assert_eq!(cmd.payload(), Some("153".to_string()));
    }

    #[test]
    fn color_temp_command_adjustments() {
        assert_eq!(ColorTempCommand::Increase.payload(), Some("+".to_string()));
        assert_eq!(ColorTempCommand::Decrease.payload(), Some("-".to_string()));
    }

    #[test]
    fn hsb_color_command_set() {
        let cmd = HsbColorCommand::Set(HsbColor::red());
        assert_eq!(cmd.name(), "HSBColor");
        assert_eq!(cmd.payload(), Some("0,100,100".to_string()));
    }

    #[test]
    fn hsb_color_command_individual() {
        assert_eq!(HsbColorCommand::SetHue(120).name(), "HSBColor1");
        assert_eq!(
            HsbColorCommand::SetHue(120).payload(),
            Some("120".to_string())
        );

        assert_eq!(HsbColorCommand::SetSaturation(50).name(), "HSBColor2");
        assert_eq!(
            HsbColorCommand::SetSaturation(50).payload(),
            Some("50".to_string())
        );

        assert_eq!(HsbColorCommand::SetBrightness(75).name(), "HSBColor3");
        assert_eq!(
            HsbColorCommand::SetBrightness(75).payload(),
            Some("75".to_string())
        );
    }

    #[test]
    fn speed_command_set() {
        let cmd = SpeedCommand::Set(FadeSpeed::SLOW);
        assert_eq!(cmd.name(), "Speed");
        assert_eq!(cmd.payload(), Some("40".to_string()));
    }

    #[test]
    fn speed_command_adjustments() {
        assert_eq!(SpeedCommand::Increase.payload(), Some("+".to_string()));
        assert_eq!(SpeedCommand::Decrease.payload(), Some("-".to_string()));
    }

    #[test]
    fn state_command() {
        let cmd = StateCommand;
        assert_eq!(cmd.name(), "State");
        assert_eq!(cmd.payload(), None);
        assert_eq!(cmd.to_http_command(), "State");
        assert_eq!(cmd.mqtt_topic_suffix(), "State");
        assert_eq!(cmd.mqtt_payload(), "");
    }
}
