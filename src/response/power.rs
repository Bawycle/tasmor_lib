// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Power response parsing.

use serde::Deserialize;

use crate::error::{ParseError, ValueError};
use crate::types::PowerState;

/// Response from a Power command.
///
/// Tasmota returns power state in JSON format like:
/// - `{"POWER": "ON"}` for single-relay devices
/// - `{"POWER1": "ON", "POWER2": "OFF"}` for multi-relay devices
///
/// # Examples
///
/// ```
/// use tasmor_lib::response::PowerResponse;
///
/// let json = r#"{"POWER": "ON"}"#;
/// let response: PowerResponse = serde_json::from_str(json).unwrap();
/// assert_eq!(response.power_state(1).unwrap().unwrap().as_str(), "ON");
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct PowerResponse {
    #[serde(rename = "POWER", default)]
    power: Option<String>,
    #[serde(rename = "POWER1", default)]
    power1: Option<String>,
    #[serde(rename = "POWER2", default)]
    power2: Option<String>,
    #[serde(rename = "POWER3", default)]
    power3: Option<String>,
    #[serde(rename = "POWER4", default)]
    power4: Option<String>,
    #[serde(rename = "POWER5", default)]
    power5: Option<String>,
    #[serde(rename = "POWER6", default)]
    power6: Option<String>,
    #[serde(rename = "POWER7", default)]
    power7: Option<String>,
    #[serde(rename = "POWER8", default)]
    power8: Option<String>,
}

impl PowerResponse {
    /// Gets the power state for a specific relay index.
    ///
    /// # Arguments
    ///
    /// * `index` - The relay index (1-8)
    ///
    /// # Returns
    ///
    /// - `Ok(Some(state))` if the relay exists and has a valid state
    /// - `Ok(None)` if the relay doesn't exist in the response
    /// - `Err` if the state string is invalid
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if the power state string cannot be parsed.
    pub fn power_state(&self, index: u8) -> Result<Option<PowerState>, ParseError> {
        let state_str = match index {
            1 => self.power1.as_ref().or(self.power.as_ref()),
            2 => self.power2.as_ref(),
            3 => self.power3.as_ref(),
            4 => self.power4.as_ref(),
            5 => self.power5.as_ref(),
            6 => self.power6.as_ref(),
            7 => self.power7.as_ref(),
            8 => self.power8.as_ref(),
            _ => return Ok(None),
        };

        match state_str {
            Some(s) => s
                .parse::<PowerState>()
                .map(Some)
                .map_err(|e| ParseError::InvalidValue {
                    field: format!("POWER{index}"),
                    message: match e {
                        ValueError::InvalidPowerState(s) => s,
                        _ => "unknown error".to_string(),
                    },
                }),
            None => Ok(None),
        }
    }

    /// Gets the first available power state.
    ///
    /// Useful for single-relay devices.
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if no power state is found or it cannot be parsed.
    pub fn first_power_state(&self) -> Result<PowerState, ParseError> {
        for i in 1..=8 {
            if let Some(state) = self.power_state(i)? {
                return Ok(state);
            }
        }
        Err(ParseError::MissingField("POWER".to_string()))
    }

    /// Returns all power states as a vector of (index, state) tuples.
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if any power state cannot be parsed.
    pub fn all_power_states(&self) -> Result<Vec<(u8, PowerState)>, ParseError> {
        let mut states = Vec::new();
        for i in 1..=8 {
            if let Some(state) = self.power_state(i)? {
                states.push((i, state));
            }
        }
        Ok(states)
    }

    /// Returns the number of relays present in the response.
    #[must_use]
    pub fn relay_count(&self) -> u8 {
        let mut count = 0;
        if self.power.is_some() || self.power1.is_some() {
            count += 1;
        }
        if self.power2.is_some() {
            count += 1;
        }
        if self.power3.is_some() {
            count += 1;
        }
        if self.power4.is_some() {
            count += 1;
        }
        if self.power5.is_some() {
            count += 1;
        }
        if self.power6.is_some() {
            count += 1;
        }
        if self.power7.is_some() {
            count += 1;
        }
        if self.power8.is_some() {
            count += 1;
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_power() {
        let json = r#"{"POWER": "ON"}"#;
        let response: PowerResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.first_power_state().unwrap(), PowerState::On);
        assert_eq!(response.relay_count(), 1);
    }

    #[test]
    fn parse_power1() {
        let json = r#"{"POWER1": "OFF"}"#;
        let response: PowerResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.power_state(1).unwrap().unwrap(), PowerState::Off);
    }

    #[test]
    fn parse_multi_relay() {
        let json = r#"{"POWER1": "ON", "POWER2": "OFF", "POWER3": "ON"}"#;
        let response: PowerResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.power_state(1).unwrap().unwrap(), PowerState::On);
        assert_eq!(response.power_state(2).unwrap().unwrap(), PowerState::Off);
        assert_eq!(response.power_state(3).unwrap().unwrap(), PowerState::On);
        assert!(response.power_state(4).unwrap().is_none());
        assert_eq!(response.relay_count(), 3);
    }

    #[test]
    fn all_power_states() {
        let json = r#"{"POWER1": "ON", "POWER2": "OFF"}"#;
        let response: PowerResponse = serde_json::from_str(json).unwrap();

        let states = response.all_power_states().unwrap();
        assert_eq!(states.len(), 2);
        assert_eq!(states[0], (1, PowerState::On));
        assert_eq!(states[1], (2, PowerState::Off));
    }
}
