// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Scheme and wakeup duration response parsing.

use serde::Deserialize;

use crate::error::ParseError;
use crate::types::{Scheme, WakeupDuration};

/// Response from a Scheme command.
///
/// Tasmota returns scheme state in JSON format like:
/// - `{"Scheme": 0}` for fixed color
/// - `{"Scheme": 1}` for wakeup effect
///
/// # Examples
///
/// ```
/// use tasmor_lib::response::SchemeResponse;
///
/// let json = r#"{"Scheme": 1}"#;
/// let response: SchemeResponse = serde_json::from_str(json).unwrap();
/// assert_eq!(response.scheme().unwrap().value(), 1);
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct SchemeResponse {
    /// The scheme value (0-4).
    #[serde(rename = "Scheme")]
    scheme: u8,
}

impl SchemeResponse {
    /// Returns the scheme.
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if the scheme value is invalid.
    pub fn scheme(&self) -> Result<Scheme, ParseError> {
        Scheme::new(self.scheme).map_err(|_| ParseError::InvalidValue {
            field: "Scheme".to_string(),
            message: format!("invalid scheme value: {}", self.scheme),
        })
    }

    /// Returns the raw scheme value.
    #[must_use]
    pub const fn scheme_raw(&self) -> u8 {
        self.scheme
    }
}

/// Response from a `WakeupDuration` command.
///
/// Tasmota returns wakeup duration in JSON format like:
/// - `{"WakeupDuration": 60}` for 60 seconds
///
/// # Examples
///
/// ```
/// use tasmor_lib::response::WakeupDurationResponse;
///
/// let json = r#"{"WakeupDuration": 300}"#;
/// let response: WakeupDurationResponse = serde_json::from_str(json).unwrap();
/// assert_eq!(response.duration().unwrap().seconds(), 300);
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct WakeupDurationResponse {
    /// The wakeup duration in seconds (1-3000).
    #[serde(rename = "WakeupDuration")]
    wakeup_duration: u16,
}

impl WakeupDurationResponse {
    /// Returns the wakeup duration.
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if the duration value is invalid.
    pub fn duration(&self) -> Result<WakeupDuration, ParseError> {
        WakeupDuration::new(self.wakeup_duration).map_err(|_| ParseError::InvalidValue {
            field: "WakeupDuration".to_string(),
            message: format!("invalid wakeup duration: {}", self.wakeup_duration),
        })
    }

    /// Returns the raw duration in seconds.
    #[must_use]
    pub const fn seconds(&self) -> u16 {
        self.wakeup_duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_scheme_response() {
        let json = r#"{"Scheme": 1}"#;
        let response: SchemeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.scheme().unwrap(), Scheme::WAKEUP);
        assert_eq!(response.scheme_raw(), 1);
    }

    #[test]
    fn parse_scheme_response_all_values() {
        for value in 0..=4 {
            let json = format!(r#"{{"Scheme": {value}}}"#);
            let response: SchemeResponse = serde_json::from_str(&json).unwrap();
            assert_eq!(response.scheme_raw(), value);
            assert!(response.scheme().is_ok());
        }
    }

    #[test]
    fn parse_scheme_response_invalid() {
        let json = r#"{"Scheme": 5}"#;
        let response: SchemeResponse = serde_json::from_str(json).unwrap();
        assert!(response.scheme().is_err());
    }

    #[test]
    fn parse_wakeup_duration_response() {
        let json = r#"{"WakeupDuration": 300}"#;
        let response: WakeupDurationResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.duration().unwrap().seconds(), 300);
        assert_eq!(response.seconds(), 300);
    }

    #[test]
    fn parse_wakeup_duration_response_edge_cases() {
        // Minimum
        let json = r#"{"WakeupDuration": 1}"#;
        let response: WakeupDurationResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.duration().unwrap().seconds(), 1);

        // Maximum
        let json = r#"{"WakeupDuration": 3000}"#;
        let response: WakeupDurationResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.duration().unwrap().seconds(), 3000);
    }

    #[test]
    fn parse_wakeup_duration_response_invalid() {
        // Below minimum
        let json = r#"{"WakeupDuration": 0}"#;
        let response: WakeupDurationResponse = serde_json::from_str(json).unwrap();
        assert!(response.duration().is_err());

        // Above maximum
        let json = r#"{"WakeupDuration": 3001}"#;
        let response: WakeupDurationResponse = serde_json::from_str(json).unwrap();
        assert!(response.duration().is_err());
    }
}
