// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Power-related types for Tasmota devices.
//!
//! This module provides types for controlling power state and addressing
//! specific power channels on multi-relay devices.

use std::fmt;
use std::str::FromStr;

use crate::error::ValueError;

/// Represents the power state of a device or relay.
///
/// # Examples
///
/// ```
/// use tasmor_lib::types::PowerState;
///
/// let on = PowerState::On;
/// let off = PowerState::Off;
/// let toggle = PowerState::Toggle;
///
/// assert_eq!(on.as_str(), "ON");
/// assert_eq!(off.as_str(), "OFF");
/// assert_eq!(toggle.as_str(), "TOGGLE");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PowerState {
    /// Power is off.
    Off,
    /// Power is on.
    On,
    /// Toggle the current power state.
    Toggle,
    /// Blink the relay for a specified number of times.
    Blink,
    /// Stop a blink sequence in progress.
    BlinkOff,
}

impl PowerState {
    /// Returns the Tasmota command string representation.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Off => "OFF",
            Self::On => "ON",
            Self::Toggle => "TOGGLE",
            Self::Blink => "BLINK",
            Self::BlinkOff => "BLINKOFF",
        }
    }

    /// Returns the numeric value used by Tasmota.
    #[must_use]
    pub const fn as_num(&self) -> u8 {
        match self {
            Self::Off => 0,
            Self::On => 1,
            Self::Toggle => 2,
            Self::Blink => 3,
            Self::BlinkOff => 4,
        }
    }
}

impl fmt::Display for PowerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for PowerState {
    type Err = ValueError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "OFF" | "0" | "FALSE" => Ok(Self::Off),
            "ON" | "1" | "TRUE" => Ok(Self::On),
            "TOGGLE" | "2" => Ok(Self::Toggle),
            "BLINK" | "3" => Ok(Self::Blink),
            "BLINKOFF" | "4" => Ok(Self::BlinkOff),
            _ => Err(ValueError::InvalidPowerState(s.to_string())),
        }
    }
}

impl From<bool> for PowerState {
    fn from(value: bool) -> Self {
        if value { Self::On } else { Self::Off }
    }
}

/// Index of a power channel on a multi-relay device.
///
/// Tasmota devices can have up to 8 relays, indexed from 1 to 8.
/// Index 0 represents all relays simultaneously.
///
/// # Examples
///
/// ```
/// use tasmor_lib::types::PowerIndex;
///
/// // Create index for relay 1
/// let idx = PowerIndex::new(1).unwrap();
/// assert_eq!(idx.value(), 1);
///
/// // Index for all relays
/// let all = PowerIndex::all();
/// assert_eq!(all.value(), 0);
///
/// // Invalid index returns error
/// assert!(PowerIndex::new(9).is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PowerIndex(u8);

impl PowerIndex {
    /// Maximum valid power index (8 relays).
    pub const MAX: u8 = 8;

    /// Creates a new power index.
    ///
    /// # Arguments
    ///
    /// * `index` - The relay index (0-8, where 0 means all relays)
    ///
    /// # Errors
    ///
    /// Returns `ValueError::OutOfRange` if index is greater than 8.
    pub fn new(index: u8) -> Result<Self, ValueError> {
        if index > Self::MAX {
            return Err(ValueError::OutOfRange {
                min: 0,
                max: u16::from(Self::MAX),
                actual: u16::from(index),
            });
        }
        Ok(Self(index))
    }

    /// Creates a power index that targets all relays.
    #[must_use]
    pub const fn all() -> Self {
        Self(0)
    }

    /// Creates a power index for relay 1.
    #[must_use]
    pub const fn one() -> Self {
        Self(1)
    }

    /// Returns the numeric value of the index.
    #[must_use]
    pub const fn value(&self) -> u8 {
        self.0
    }

    /// Returns the suffix for Tasmota commands.
    ///
    /// Returns empty string for index 0, otherwise returns the index as string.
    #[must_use]
    pub fn command_suffix(&self) -> String {
        if self.0 == 0 {
            String::new()
        } else {
            self.0.to_string()
        }
    }
}

impl fmt::Display for PowerIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 == 0 {
            write!(f, "all")
        } else {
            write!(f, "{}", self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_state_as_str() {
        assert_eq!(PowerState::Off.as_str(), "OFF");
        assert_eq!(PowerState::On.as_str(), "ON");
        assert_eq!(PowerState::Toggle.as_str(), "TOGGLE");
        assert_eq!(PowerState::Blink.as_str(), "BLINK");
        assert_eq!(PowerState::BlinkOff.as_str(), "BLINKOFF");
    }

    #[test]
    fn power_state_from_str() {
        assert_eq!("ON".parse::<PowerState>().unwrap(), PowerState::On);
        assert_eq!("off".parse::<PowerState>().unwrap(), PowerState::Off);
        assert_eq!("1".parse::<PowerState>().unwrap(), PowerState::On);
        assert_eq!("0".parse::<PowerState>().unwrap(), PowerState::Off);
        assert_eq!("true".parse::<PowerState>().unwrap(), PowerState::On);
        assert_eq!("false".parse::<PowerState>().unwrap(), PowerState::Off);
        assert_eq!("toggle".parse::<PowerState>().unwrap(), PowerState::Toggle);
    }

    #[test]
    fn power_state_from_str_invalid() {
        let result = "invalid".parse::<PowerState>();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValueError::InvalidPowerState(_)
        ));
    }

    #[test]
    fn power_state_from_bool() {
        assert_eq!(PowerState::from(true), PowerState::On);
        assert_eq!(PowerState::from(false), PowerState::Off);
    }

    #[test]
    fn power_index_valid() {
        for i in 0..=8 {
            let idx = PowerIndex::new(i).unwrap();
            assert_eq!(idx.value(), i);
        }
    }

    #[test]
    fn power_index_invalid() {
        let result = PowerIndex::new(9);
        assert!(result.is_err());
    }

    #[test]
    fn power_index_command_suffix() {
        assert_eq!(PowerIndex::all().command_suffix(), "");
        assert_eq!(PowerIndex::one().command_suffix(), "1");
        assert_eq!(PowerIndex::new(3).unwrap().command_suffix(), "3");
    }

    #[test]
    fn power_index_display() {
        assert_eq!(PowerIndex::all().to_string(), "all");
        assert_eq!(PowerIndex::one().to_string(), "1");
    }
}
