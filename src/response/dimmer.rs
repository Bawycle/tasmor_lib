// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Dimmer response parsing.

use serde::Deserialize;

use crate::error::ParseError;
use crate::types::PowerState;

/// Response from a Dimmer command.
///
/// Tasmota returns dimmer state in JSON format like:
/// - `{"Dimmer": 75}` for dimmer-only response
/// - `{"Dimmer": 75, "POWER": "ON"}` when power state is included
///
/// # Examples
///
/// ```
/// use tasmor_lib::response::DimmerResponse;
///
/// let json = r#"{"Dimmer": 75, "POWER": "ON"}"#;
/// let response: DimmerResponse = serde_json::from_str(json).unwrap();
/// assert_eq!(response.dimmer(), 75);
/// assert_eq!(response.power_state().unwrap().unwrap(), tasmor_lib::PowerState::On);
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct DimmerResponse {
    /// The dimmer level (0-100).
    #[serde(rename = "Dimmer")]
    dimmer: u8,

    /// Optional power state included in the response.
    #[serde(rename = "POWER", default)]
    power: Option<String>,
}

impl DimmerResponse {
    /// Returns the dimmer level (0-100).
    #[must_use]
    pub fn dimmer(&self) -> u8 {
        self.dimmer
    }

    /// Returns the power state if included in the response.
    ///
    /// Tasmota often includes the power state in dimmer responses,
    /// especially when setting a dimmer value turns the light on.
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if the power state string is invalid.
    pub fn power_state(&self) -> Result<Option<PowerState>, ParseError> {
        match &self.power {
            Some(s) => s
                .parse::<PowerState>()
                .map(Some)
                .map_err(|_| ParseError::InvalidValue {
                    field: "POWER".to_string(),
                    message: format!("invalid power state: {s}"),
                }),
            None => Ok(None),
        }
    }

    /// Returns `true` if the device is on according to the response.
    ///
    /// Returns `None` if power state was not included in the response.
    #[must_use]
    pub fn is_on(&self) -> Option<bool> {
        self.power.as_ref().map(|s| s == "ON")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_dimmer_only() {
        let json = r#"{"Dimmer": 50}"#;
        let response: DimmerResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.dimmer(), 50);
        assert!(response.power_state().unwrap().is_none());
        assert!(response.is_on().is_none());
    }

    #[test]
    fn parse_dimmer_with_power_on() {
        let json = r#"{"Dimmer": 75, "POWER": "ON"}"#;
        let response: DimmerResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.dimmer(), 75);
        assert_eq!(response.power_state().unwrap().unwrap(), PowerState::On);
        assert_eq!(response.is_on(), Some(true));
    }

    #[test]
    fn parse_dimmer_with_power_off() {
        let json = r#"{"Dimmer": 0, "POWER": "OFF"}"#;
        let response: DimmerResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.dimmer(), 0);
        assert_eq!(response.power_state().unwrap().unwrap(), PowerState::Off);
        assert_eq!(response.is_on(), Some(false));
    }

    #[test]
    fn parse_dimmer_min_max() {
        let json_min = r#"{"Dimmer": 0}"#;
        let json_max = r#"{"Dimmer": 100}"#;

        let response_min: DimmerResponse = serde_json::from_str(json_min).unwrap();
        let response_max: DimmerResponse = serde_json::from_str(json_max).unwrap();

        assert_eq!(response_min.dimmer(), 0);
        assert_eq!(response_max.dimmer(), 100);
    }

    #[test]
    fn parse_dimmer_with_additional_fields() {
        // Tasmota often includes other fields in RESULT responses
        let json = r#"{
            "Dimmer": 75,
            "POWER": "ON",
            "Color": "FFFFFF",
            "HSBColor": "0,0,75",
            "White": 75,
            "CT": 327
        }"#;
        let response: DimmerResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.dimmer(), 75);
        assert_eq!(response.power_state().unwrap().unwrap(), PowerState::On);
    }
}
