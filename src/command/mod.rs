// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Tasmota command definitions.
//!
//! This module provides typed representations of Tasmota commands that can be
//! sent via HTTP or MQTT protocols.
//!
//! # Available Commands
//!
//! | Command Type | Purpose | Example |
//! |-------------|---------|---------|
//! | [`PowerCommand`] | Control relay power state | On, Off, Toggle |
//! | [`DimmerCommand`] | Adjust brightness (0-100) | Set to 75% |
//! | [`ColorTemperatureCommand`] | Set white color temperature | Warm, Cool |
//! | [`HsbColorCommand`] | Set RGB color in HSB format | Red, Blue |
//! | [`SchemeCommand`] | Set light scheme/effect (0-4) | Wakeup, Random |
//! | [`WakeupDurationCommand`] | Set wakeup duration (1-3000s) | 5 minutes |
//! | [`FadeSpeedCommand`] | Set fade transition speed (1-40) | Fast, Slow |
//! | [`FadeCommand`] | Enable/disable fade transitions | On, Off |
//! | [`EnergyCommand`] | Query energy consumption | Get, Reset |
//! | [`StatusCommand`] | Query device status | Status 0-10 |
//!
//! # Command Structure
//!
//! Each Tasmota command consists of:
//! - A command name (e.g., "Power", "Dimmer", "Status")
//! - An optional index suffix (e.g., "Power1", "Power2")
//! - An optional payload (e.g., "ON", "50", "120,100,75")
//!
//! # Examples
//!
//! ## Basic power control
//!
//! ```
//! use tasmor_lib::command::{Command, PowerCommand};
//! use tasmor_lib::types::{PowerIndex, PowerState};
//!
//! let cmd = PowerCommand::Set {
//!     index: PowerIndex::one(),
//!     state: PowerState::On,
//! };
//!
//! assert_eq!(cmd.name(), "Power1");
//! assert_eq!(cmd.payload(), Some("ON".to_string()));
//! ```
//!
//! ## Light control
//!
//! ```
//! use tasmor_lib::command::{Command, DimmerCommand, ColorTemperatureCommand};
//! use tasmor_lib::types::{Dimmer, ColorTemperature};
//!
//! // Set brightness to 75%
//! let dim = DimmerCommand::Set(Dimmer::new(75).unwrap());
//! assert_eq!(dim.payload(), Some("75".to_string()));
//!
//! // Set warm white color temperature
//! let ct = ColorTemperatureCommand::Set(ColorTemperature::WARM);
//! assert_eq!(ct.name(), "CT");
//! ```

mod energy;
mod light;
mod power;
mod scheme;
mod status;

pub use energy::EnergyCommand;
pub use light::{
    ColorTemperatureCommand, DimmerCommand, FadeSpeedCommand, HsbColorCommand, StateCommand,
};
pub use power::{FadeCommand, PowerCommand, StartupFadeCommand};
pub use scheme::{SchemeCommand, WakeupDurationCommand};
pub use status::{StatusCommand, StatusType};

/// A command that can be sent to a Tasmota device.
///
/// Commands are serialized to the Tasmota command format for transmission
/// over HTTP or MQTT.
pub trait Command {
    /// Returns the command name with any index suffix.
    ///
    /// For example, `"Power"`, `"Power1"`, `"Status"`, `"Dimmer"`.
    fn name(&self) -> String;

    /// Returns the command payload, if any.
    ///
    /// The payload is the value sent with the command. For example:
    /// - `Power ON` has payload `Some("ON")`
    /// - `Status` (query) has payload `None`
    /// - `Dimmer 50` has payload `Some("50")`
    fn payload(&self) -> Option<String>;

    /// Returns the full command string for HTTP requests.
    ///
    /// Format: `<name> <payload>` or just `<name>` if no payload.
    fn to_http_command(&self) -> String {
        match self.payload() {
            Some(p) => format!("{} {}", self.name(), p),
            None => self.name(),
        }
    }

    /// Returns the MQTT topic suffix for this command.
    ///
    /// This is the part after `cmnd/<topic>/`.
    fn mqtt_topic_suffix(&self) -> String {
        self.name()
    }

    /// Returns the MQTT payload for this command.
    ///
    /// Returns empty string for query commands.
    fn mqtt_payload(&self) -> String {
        self.payload().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PowerIndex, PowerState};

    #[test]
    fn command_http_format() {
        let cmd = PowerCommand::Set {
            index: PowerIndex::one(),
            state: PowerState::On,
        };
        assert_eq!(cmd.to_http_command(), "Power1 ON");
    }

    #[test]
    fn command_http_format_no_payload() {
        let cmd = PowerCommand::Get {
            index: PowerIndex::one(),
        };
        assert_eq!(cmd.to_http_command(), "Power1");
    }

    #[test]
    fn command_mqtt_format() {
        let cmd = PowerCommand::Set {
            index: PowerIndex::one(),
            state: PowerState::On,
        };
        assert_eq!(cmd.mqtt_topic_suffix(), "Power1");
        assert_eq!(cmd.mqtt_payload(), "ON");
    }
}
