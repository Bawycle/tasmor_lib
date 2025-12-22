// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Power control commands.
//!
//! This module provides commands for controlling device power state,
//! fade transitions, and power-on behavior.

use crate::command::Command;
use crate::types::{PowerIndex, PowerState};

/// Command to control device power state.
///
/// # Examples
///
/// ```
/// use tasmor_lib::command::{Command, PowerCommand};
/// use tasmor_lib::types::{PowerIndex, PowerState};
///
/// // Turn on relay 1
/// let cmd = PowerCommand::Set {
///     index: PowerIndex::one(),
///     state: PowerState::On,
/// };
/// assert_eq!(cmd.name(), "Power1");
/// assert_eq!(cmd.payload(), Some("ON".to_string()));
///
/// // Query power state of all relays
/// let query = PowerCommand::Get { index: PowerIndex::all() };
/// assert_eq!(query.name(), "Power");
/// assert_eq!(query.payload(), None);
///
/// // Toggle relay 2
/// let toggle = PowerCommand::Toggle { index: PowerIndex::new(2).unwrap() };
/// assert_eq!(toggle.payload(), Some("TOGGLE".to_string()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PowerCommand {
    /// Query the current power state.
    Get {
        /// The relay index to query.
        index: PowerIndex,
    },
    /// Set the power state.
    Set {
        /// The relay index to control.
        index: PowerIndex,
        /// The desired power state.
        state: PowerState,
    },
    /// Toggle the power state.
    Toggle {
        /// The relay index to toggle.
        index: PowerIndex,
    },
}

impl PowerCommand {
    /// Creates a command to turn on a relay.
    #[must_use]
    pub const fn on(index: PowerIndex) -> Self {
        Self::Set {
            index,
            state: PowerState::On,
        }
    }

    /// Creates a command to turn off a relay.
    #[must_use]
    pub const fn off(index: PowerIndex) -> Self {
        Self::Set {
            index,
            state: PowerState::Off,
        }
    }

    /// Creates a command to toggle a relay.
    #[must_use]
    pub const fn toggle(index: PowerIndex) -> Self {
        Self::Toggle { index }
    }

    /// Creates a command to query relay state.
    #[must_use]
    pub const fn query(index: PowerIndex) -> Self {
        Self::Get { index }
    }
}

impl Command for PowerCommand {
    fn name(&self) -> String {
        let index = match self {
            Self::Get { index } | Self::Set { index, .. } | Self::Toggle { index } => index,
        };
        format!("Power{}", index.command_suffix())
    }

    fn payload(&self) -> Option<String> {
        match self {
            Self::Get { .. } => None,
            Self::Set { state, .. } => Some(state.as_str().to_string()),
            Self::Toggle { .. } => Some(PowerState::Toggle.as_str().to_string()),
        }
    }
}

/// Command to enable or disable fade transitions.
///
/// When enabled, brightness and color changes will transition smoothly
/// instead of changing instantly.
///
/// # Examples
///
/// ```
/// use tasmor_lib::command::{Command, FadeCommand};
///
/// let enable = FadeCommand::Enable;
/// assert_eq!(enable.name(), "Fade");
/// assert_eq!(enable.payload(), Some("1".to_string()));
///
/// let disable = FadeCommand::Disable;
/// assert_eq!(disable.payload(), Some("0".to_string()));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadeCommand {
    /// Query the current fade setting.
    Get,
    /// Enable fade transitions.
    Enable,
    /// Disable fade transitions.
    Disable,
}

impl Command for FadeCommand {
    fn name(&self) -> String {
        "Fade".to_string()
    }

    fn payload(&self) -> Option<String> {
        match self {
            Self::Get => None,
            Self::Enable => Some("1".to_string()),
            Self::Disable => Some("0".to_string()),
        }
    }
}

/// Command to enable or disable fade on power-on.
///
/// This corresponds to Tasmota's `SetOption91`.
///
/// # Examples
///
/// ```
/// use tasmor_lib::command::{Command, PowerOnFadeCommand};
///
/// let enable = PowerOnFadeCommand::Enable;
/// assert_eq!(enable.name(), "SetOption91");
/// assert_eq!(enable.payload(), Some("1".to_string()));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerOnFadeCommand {
    /// Query the current setting.
    Get,
    /// Enable fade on power-on.
    Enable,
    /// Disable fade on power-on.
    Disable,
}

impl Command for PowerOnFadeCommand {
    fn name(&self) -> String {
        "SetOption91".to_string()
    }

    fn payload(&self) -> Option<String> {
        match self {
            Self::Get => None,
            Self::Enable => Some("1".to_string()),
            Self::Disable => Some("0".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_command_on() {
        let cmd = PowerCommand::on(PowerIndex::one());
        assert_eq!(cmd.name(), "Power1");
        assert_eq!(cmd.payload(), Some("ON".to_string()));
    }

    #[test]
    fn power_command_off() {
        let cmd = PowerCommand::off(PowerIndex::new(2).unwrap());
        assert_eq!(cmd.name(), "Power2");
        assert_eq!(cmd.payload(), Some("OFF".to_string()));
    }

    #[test]
    fn power_command_toggle() {
        let cmd = PowerCommand::toggle(PowerIndex::all());
        assert_eq!(cmd.name(), "Power");
        assert_eq!(cmd.payload(), Some("TOGGLE".to_string()));
    }

    #[test]
    fn power_command_query() {
        let cmd = PowerCommand::query(PowerIndex::one());
        assert_eq!(cmd.name(), "Power1");
        assert_eq!(cmd.payload(), None);
    }

    #[test]
    fn fade_command() {
        assert_eq!(FadeCommand::Get.payload(), None);
        assert_eq!(FadeCommand::Enable.payload(), Some("1".to_string()));
        assert_eq!(FadeCommand::Disable.payload(), Some("0".to_string()));
    }

    #[test]
    fn power_on_fade_command() {
        assert_eq!(PowerOnFadeCommand::Get.name(), "SetOption91");
        assert_eq!(PowerOnFadeCommand::Enable.payload(), Some("1".to_string()));
    }
}
