// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Scheme-related commands.
//!
//! This module provides commands for controlling light schemes/effects
//! and the wakeup duration setting.

use crate::command::Command;
use crate::types::{Scheme, WakeupDuration};

/// Command to control the light scheme/effect.
///
/// Tasmota supports several built-in light effects:
/// - 0: Single (fixed color)
/// - 1: Wakeup (gradual brightness increase)
/// - 2: Cycle Up (color cycling with increasing brightness)
/// - 3: Cycle Down (color cycling with decreasing brightness)
/// - 4: Random (random color changes)
///
/// # Examples
///
/// ```
/// use tasmor_lib::command::{Command, SchemeCommand};
/// use tasmor_lib::types::Scheme;
///
/// // Set wakeup scheme
/// let cmd = SchemeCommand::Set(Scheme::WAKEUP);
/// assert_eq!(cmd.name(), "Scheme");
/// assert_eq!(cmd.payload(), Some("1".to_string()));
///
/// // Query current scheme
/// let query = SchemeCommand::Get;
/// assert_eq!(query.payload(), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemeCommand {
    /// Query the current scheme.
    Get,
    /// Set the scheme to a specific value.
    Set(Scheme),
}

impl SchemeCommand {
    /// Creates a command to set a specific scheme.
    #[must_use]
    pub const fn set(scheme: Scheme) -> Self {
        Self::Set(scheme)
    }
}

impl Command for SchemeCommand {
    fn name(&self) -> String {
        "Scheme".to_string()
    }

    fn payload(&self) -> Option<String> {
        match self {
            Self::Get => None,
            Self::Set(scheme) => Some(scheme.value().to_string()),
        }
    }
}

/// Command to control the wakeup duration.
///
/// The wakeup duration controls how long Scheme 1 (Wakeup) takes to
/// gradually increase brightness from 0 to the current dimmer level.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use tasmor_lib::command::{Command, WakeupDurationCommand};
/// use tasmor_lib::types::WakeupDuration;
///
/// // Set wakeup duration to 5 minutes
/// let cmd = WakeupDurationCommand::Set(WakeupDuration::new(Duration::from_secs(300)).unwrap());
/// assert_eq!(cmd.name(), "WakeupDuration");
/// assert_eq!(cmd.payload(), Some("300".to_string()));
///
/// // Query current duration
/// let query = WakeupDurationCommand::Get;
/// assert_eq!(query.payload(), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeupDurationCommand {
    /// Query the current wakeup duration.
    Get,
    /// Set the wakeup duration.
    Set(WakeupDuration),
}

impl WakeupDurationCommand {
    /// Creates a command to set a specific wakeup duration.
    #[must_use]
    pub const fn set(duration: WakeupDuration) -> Self {
        Self::Set(duration)
    }
}

impl Command for WakeupDurationCommand {
    fn name(&self) -> String {
        "WakeupDuration".to_string()
    }

    fn payload(&self) -> Option<String> {
        match self {
            Self::Get => None,
            Self::Set(duration) => Some(duration.seconds().to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn scheme_command_get() {
        let cmd = SchemeCommand::Get;
        assert_eq!(cmd.name(), "Scheme");
        assert_eq!(cmd.payload(), None);
    }

    #[test]
    fn scheme_command_set() {
        let cmd = SchemeCommand::Set(Scheme::WAKEUP);
        assert_eq!(cmd.name(), "Scheme");
        assert_eq!(cmd.payload(), Some("1".to_string()));
    }

    #[test]
    fn scheme_command_all_values() {
        assert_eq!(
            SchemeCommand::Set(Scheme::SINGLE).payload(),
            Some("0".to_string())
        );
        assert_eq!(
            SchemeCommand::Set(Scheme::WAKEUP).payload(),
            Some("1".to_string())
        );
        assert_eq!(
            SchemeCommand::Set(Scheme::CYCLE_UP).payload(),
            Some("2".to_string())
        );
        assert_eq!(
            SchemeCommand::Set(Scheme::CYCLE_DOWN).payload(),
            Some("3".to_string())
        );
        assert_eq!(
            SchemeCommand::Set(Scheme::RANDOM).payload(),
            Some("4".to_string())
        );
    }

    #[test]
    fn scheme_command_http() {
        let cmd = SchemeCommand::Set(Scheme::RANDOM);
        assert_eq!(cmd.to_http_command(), "Scheme 4");
    }

    #[test]
    fn scheme_command_mqtt() {
        let cmd = SchemeCommand::Set(Scheme::RANDOM);
        assert_eq!(cmd.mqtt_topic_suffix(), "Scheme");
        assert_eq!(cmd.mqtt_payload(), "4");
    }

    #[test]
    fn wakeup_duration_command_get() {
        let cmd = WakeupDurationCommand::Get;
        assert_eq!(cmd.name(), "WakeupDuration");
        assert_eq!(cmd.payload(), None);
    }

    #[test]
    fn wakeup_duration_command_set() {
        let duration = WakeupDuration::new(Duration::from_secs(300)).unwrap();
        let cmd = WakeupDurationCommand::Set(duration);
        assert_eq!(cmd.name(), "WakeupDuration");
        assert_eq!(cmd.payload(), Some("300".to_string()));
    }

    #[test]
    fn wakeup_duration_command_http() {
        let duration = WakeupDuration::new(Duration::from_secs(60)).unwrap();
        let cmd = WakeupDurationCommand::Set(duration);
        assert_eq!(cmd.to_http_command(), "WakeupDuration 60");
    }

    #[test]
    fn wakeup_duration_command_mqtt() {
        let duration = WakeupDuration::new(Duration::from_secs(60)).unwrap();
        let cmd = WakeupDurationCommand::Set(duration);
        assert_eq!(cmd.mqtt_topic_suffix(), "WakeupDuration");
        assert_eq!(cmd.mqtt_payload(), "60");
    }
}
