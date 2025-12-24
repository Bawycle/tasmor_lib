// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Energy monitoring commands.
//!
//! This module provides commands for querying energy consumption data
//! from devices with power monitoring capabilities.
//!
//! Energy data (voltage, current, power) is obtained via `Status 10` command,
//! which returns sensor information including the ENERGY object.
//!
//! Reference: <https://tasmota.github.io/docs/Commands/#management>

use crate::command::Command;

/// Command to query energy monitoring data.
///
/// Energy monitoring provides information about:
/// - Current power consumption (Watts)
/// - Voltage (Volts)
/// - Current (Amperes)
/// - Power factor
/// - Total energy consumed (kWh)
/// - Today's energy consumption (kWh)
/// - Yesterday's energy consumption (kWh)
///
/// Note: To query individual values like voltage or current, use `EnergyCommand::Get`
/// which returns all energy data via `Status 10`. There are no separate Tasmota commands
/// for querying voltage or current individually.
///
/// # Examples
///
/// ```
/// use tasmor_lib::command::{Command, EnergyCommand};
///
/// // Query current energy readings (uses Status 10)
/// let cmd = EnergyCommand::Get;
/// assert_eq!(cmd.name(), "Status");
/// assert_eq!(cmd.payload(), Some("10".to_string()));
///
/// // Reset energy counters (uses modern EnergyTotal command)
/// let reset = EnergyCommand::ResetTotal;
/// assert_eq!(reset.name(), "EnergyTotal");
/// assert_eq!(reset.payload(), Some("0".to_string()));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyCommand {
    /// Query current energy readings (Status 10).
    ///
    /// Returns sensor information including ENERGY object with:
    /// - Total, Yesterday, Today (kWh)
    /// - Power (W), Voltage (V), Current (A)
    /// - Factor, Frequency, etc.
    Get,
    /// Reset today's energy counter.
    ResetToday,
    /// Reset yesterday's energy counter.
    ResetYesterday,
    /// Reset total energy counter.
    ResetTotal,
    /// Set today's energy value in Wh.
    SetToday(u32),
    /// Set total energy value in Wh.
    SetTotal(u32),
}

impl EnergyCommand {
    /// Creates a command to query energy readings.
    #[must_use]
    pub const fn query() -> Self {
        Self::Get
    }

    /// Creates a command to reset today's counter.
    #[must_use]
    pub const fn reset_today() -> Self {
        Self::ResetToday
    }

    /// Creates a command to reset total counter.
    #[must_use]
    pub const fn reset_total() -> Self {
        Self::ResetTotal
    }
}

impl Command for EnergyCommand {
    fn name(&self) -> String {
        match self {
            Self::Get => "Status".to_string(),
            // Use modern commands (EnergyReset1/2/3 deprecated in Tasmota v10+)
            Self::ResetToday | Self::SetToday(_) => "EnergyToday".to_string(),
            Self::ResetYesterday => "EnergyYesterday".to_string(),
            Self::ResetTotal | Self::SetTotal(_) => "EnergyTotal".to_string(),
        }
    }

    fn payload(&self) -> Option<String> {
        match self {
            // Status 10 returns sensor information (replaces deprecated Status 8)
            // Reference: https://tasmota.github.io/docs/Commands/#management
            Self::Get => Some("10".to_string()),
            Self::ResetToday | Self::ResetYesterday | Self::ResetTotal => Some("0".to_string()),
            Self::SetToday(wh) | Self::SetTotal(wh) => Some(wh.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn energy_command_get() {
        // Status 10 is the current command for sensor data
        // (Status 8 is deprecated but retained for backwards compatibility)
        let cmd = EnergyCommand::Get;
        assert_eq!(cmd.name(), "Status");
        assert_eq!(cmd.payload(), Some("10".to_string()));
    }

    #[test]
    fn energy_command_reset() {
        // Modern commands (EnergyReset1/2/3 deprecated in Tasmota v10+)
        assert_eq!(EnergyCommand::ResetToday.name(), "EnergyToday");
        assert_eq!(EnergyCommand::ResetToday.payload(), Some("0".to_string()));

        assert_eq!(EnergyCommand::ResetYesterday.name(), "EnergyYesterday");
        assert_eq!(
            EnergyCommand::ResetYesterday.payload(),
            Some("0".to_string())
        );

        assert_eq!(EnergyCommand::ResetTotal.name(), "EnergyTotal");
        assert_eq!(EnergyCommand::ResetTotal.payload(), Some("0".to_string()));
    }

    #[test]
    fn energy_command_set() {
        let cmd = EnergyCommand::SetToday(1500);
        assert_eq!(cmd.name(), "EnergyToday");
        assert_eq!(cmd.payload(), Some("1500".to_string()));

        let total = EnergyCommand::SetTotal(50000);
        assert_eq!(total.name(), "EnergyTotal");
        assert_eq!(total.payload(), Some("50000".to_string()));
    }
}
