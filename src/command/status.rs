// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Status query commands.
//!
//! This module provides commands for querying device status information.

use std::time::Duration;

use crate::command::Command;
use crate::protocol::ResponseSpec;

/// Type of status information to query.
///
/// Tasmota provides different status responses based on the status index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum StatusType {
    /// Abbreviated status information.
    #[default]
    Abbreviated = 255,
    /// All status information (combines 1-11).
    All = 0,
    /// Device parameters (Module, GPIO configuration).
    DeviceParameters = 1,
    /// Firmware information (Version, Build).
    Firmware = 2,
    /// Logging and telemetry settings.
    Logging = 3,
    /// Memory information (Heap, Flash).
    Memory = 4,
    /// Network information (IP, Gateway, DNS).
    Network = 5,
    /// MQTT configuration.
    Mqtt = 6,
    /// Time and sunrise/sunset information.
    Time = 7,
    /// Power thresholds (energy monitoring).
    PowerThresholds = 9,
    /// Connected sensors information.
    Sensors = 10,
    /// Runtime state (POWER, Dimmer, CT, `HSBColor`, ENERGY).
    State = 11,
    /// Shutter configuration.
    Shutter = 13,
}

impl StatusType {
    /// Returns all available status types for iteration.
    #[must_use]
    pub const fn all_types() -> &'static [Self] {
        &[
            Self::Abbreviated,
            Self::All,
            Self::DeviceParameters,
            Self::Firmware,
            Self::Logging,
            Self::Memory,
            Self::Network,
            Self::Mqtt,
            Self::Time,
            Self::PowerThresholds,
            Self::Sensors,
            Self::State,
            Self::Shutter,
        ]
    }

    /// Returns the numeric value for this status type.
    #[must_use]
    pub const fn value(&self) -> u8 {
        *self as u8
    }
}

/// Command to query device status.
///
/// # Examples
///
/// ```
/// use tasmor_lib::command::{Command, StatusCommand, StatusType};
///
/// // Query all status information
/// let cmd = StatusCommand::new(StatusType::All);
/// assert_eq!(cmd.name(), "Status");
/// assert_eq!(cmd.payload(), Some("0".to_string()));
///
/// // Query network information
/// let net = StatusCommand::network();
/// assert_eq!(net.payload(), Some("5".to_string()));
///
/// // Abbreviated status (no payload)
/// let abbrev = StatusCommand::abbreviated();
/// assert_eq!(abbrev.payload(), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusCommand {
    status_type: StatusType,
}

impl StatusCommand {
    /// Creates a new status command for the specified type.
    #[must_use]
    pub const fn new(status_type: StatusType) -> Self {
        Self { status_type }
    }

    /// Query abbreviated status (default).
    #[must_use]
    pub const fn abbreviated() -> Self {
        Self::new(StatusType::Abbreviated)
    }

    /// Query all status information.
    #[must_use]
    pub const fn all() -> Self {
        Self::new(StatusType::All)
    }

    /// Query device parameters.
    #[must_use]
    pub const fn device_parameters() -> Self {
        Self::new(StatusType::DeviceParameters)
    }

    /// Query firmware information.
    #[must_use]
    pub const fn firmware() -> Self {
        Self::new(StatusType::Firmware)
    }

    /// Query logging settings.
    #[must_use]
    pub const fn logging() -> Self {
        Self::new(StatusType::Logging)
    }

    /// Query memory information.
    #[must_use]
    pub const fn memory() -> Self {
        Self::new(StatusType::Memory)
    }

    /// Query network information.
    #[must_use]
    pub const fn network() -> Self {
        Self::new(StatusType::Network)
    }

    /// Query MQTT configuration.
    #[must_use]
    pub const fn mqtt() -> Self {
        Self::new(StatusType::Mqtt)
    }

    /// Query time information.
    #[must_use]
    pub const fn time() -> Self {
        Self::new(StatusType::Time)
    }

    /// Query sensor information.
    #[must_use]
    pub const fn sensors() -> Self {
        Self::new(StatusType::Sensors)
    }

    /// Query runtime state (POWER, Dimmer, CT, `HSBColor`, ENERGY).
    #[must_use]
    pub const fn state() -> Self {
        Self::new(StatusType::State)
    }

    /// Returns the status type being queried.
    #[must_use]
    pub const fn status_type(&self) -> StatusType {
        self.status_type
    }
}

impl Default for StatusCommand {
    fn default() -> Self {
        Self::abbreviated()
    }
}

/// Default timeout for collecting multi-message MQTT responses.
const MULTI_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);

impl Command for StatusCommand {
    fn name(&self) -> String {
        "Status".to_string()
    }

    fn payload(&self) -> Option<String> {
        match self.status_type {
            StatusType::Abbreviated => None,
            other => Some(other.value().to_string()),
        }
    }

    fn response_spec(&self) -> ResponseSpec {
        match self.status_type {
            // Status 0 returns multiple MQTT messages
            StatusType::All => ResponseSpec::status_all(MULTI_RESPONSE_TIMEOUT),
            // All other status types return a single message
            _ => ResponseSpec::Single,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_command_abbreviated() {
        let cmd = StatusCommand::abbreviated();
        assert_eq!(cmd.name(), "Status");
        assert_eq!(cmd.payload(), None);
    }

    #[test]
    fn status_command_all() {
        let cmd = StatusCommand::all();
        assert_eq!(cmd.payload(), Some("0".to_string()));
    }

    #[test]
    fn status_command_network() {
        let cmd = StatusCommand::network();
        assert_eq!(cmd.payload(), Some("5".to_string()));
    }

    #[test]
    fn status_command_firmware() {
        let cmd = StatusCommand::firmware();
        assert_eq!(cmd.payload(), Some("2".to_string()));
    }

    #[test]
    fn status_command_http_format() {
        let cmd = StatusCommand::all();
        assert_eq!(cmd.to_http_command(), "Status 0");

        let abbrev = StatusCommand::abbreviated();
        assert_eq!(abbrev.to_http_command(), "Status");
    }

    #[test]
    fn status_type_values() {
        assert_eq!(StatusType::All.value(), 0);
        assert_eq!(StatusType::DeviceParameters.value(), 1);
        assert_eq!(StatusType::Network.value(), 5);
        assert_eq!(StatusType::Sensors.value(), 10);
    }
}
