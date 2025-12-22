// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Energy monitoring commands.
//!
//! This module provides commands for querying energy consumption data
//! from devices with power monitoring capabilities.

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
/// # Examples
///
/// ```
/// use tasmor_lib::command::{Command, EnergyCommand};
///
/// // Query current energy readings
/// let cmd = EnergyCommand::Get;
/// assert_eq!(cmd.name(), "Status");
/// assert_eq!(cmd.payload(), Some("8".to_string()));
///
/// // Reset energy counters
/// let reset = EnergyCommand::ResetTotal;
/// assert_eq!(reset.name(), "EnergyReset3");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyCommand {
    /// Query current energy readings (Status 8).
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
            Self::ResetToday => "EnergyReset1".to_string(),
            Self::ResetYesterday => "EnergyReset2".to_string(),
            Self::ResetTotal => "EnergyReset3".to_string(),
            Self::SetToday(_) => "EnergyToday".to_string(),
            Self::SetTotal(_) => "EnergyTotal".to_string(),
        }
    }

    fn payload(&self) -> Option<String> {
        match self {
            Self::Get => Some("8".to_string()),
            Self::ResetToday | Self::ResetYesterday | Self::ResetTotal => Some("0".to_string()),
            Self::SetToday(wh) | Self::SetTotal(wh) => Some(wh.to_string()),
        }
    }
}

/// Command to query voltage.
///
/// # Examples
///
/// ```
/// use tasmor_lib::command::{Command, VoltageCommand};
///
/// let cmd = VoltageCommand::Get;
/// assert_eq!(cmd.name(), "Voltage");
/// assert_eq!(cmd.payload(), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoltageCommand {
    /// Query current voltage.
    Get,
}

impl Command for VoltageCommand {
    fn name(&self) -> String {
        "Voltage".to_string()
    }

    fn payload(&self) -> Option<String> {
        None
    }
}

/// Command to query current.
///
/// # Examples
///
/// ```
/// use tasmor_lib::command::{Command, CurrentCommand};
///
/// let cmd = CurrentCommand::Get;
/// assert_eq!(cmd.name(), "Current");
/// assert_eq!(cmd.payload(), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentCommand {
    /// Query current amperage.
    Get,
}

impl Command for CurrentCommand {
    fn name(&self) -> String {
        "Current".to_string()
    }

    fn payload(&self) -> Option<String> {
        None
    }
}

/// Command to query power consumption.
///
/// # Examples
///
/// ```
/// use tasmor_lib::command::{Command, PowerMonitorCommand};
///
/// let cmd = PowerMonitorCommand::Get;
/// assert_eq!(cmd.name(), "Power");
/// assert_eq!(cmd.payload(), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerMonitorCommand {
    /// Query current power consumption in Watts.
    Get,
}

impl Command for PowerMonitorCommand {
    fn name(&self) -> String {
        "Power".to_string()
    }

    fn payload(&self) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn energy_command_get() {
        let cmd = EnergyCommand::Get;
        assert_eq!(cmd.name(), "Status");
        assert_eq!(cmd.payload(), Some("8".to_string()));
    }

    #[test]
    fn energy_command_reset() {
        assert_eq!(EnergyCommand::ResetToday.name(), "EnergyReset1");
        assert_eq!(EnergyCommand::ResetYesterday.name(), "EnergyReset2");
        assert_eq!(EnergyCommand::ResetTotal.name(), "EnergyReset3");
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

    #[test]
    fn voltage_command() {
        let cmd = VoltageCommand::Get;
        assert_eq!(cmd.name(), "Voltage");
        assert_eq!(cmd.payload(), None);
    }

    #[test]
    fn current_command() {
        let cmd = CurrentCommand::Get;
        assert_eq!(cmd.name(), "Current");
        assert_eq!(cmd.payload(), None);
    }

    #[test]
    fn power_monitor_command() {
        let cmd = PowerMonitorCommand::Get;
        assert_eq!(cmd.name(), "Power");
        assert_eq!(cmd.payload(), None);
    }
}
