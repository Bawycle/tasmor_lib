// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Fade and speed response types.
//!
//! This module provides response types for fade transition and speed settings.

use serde::Deserialize;

use crate::error::ParseError;
use crate::types::FadeSpeed;

/// Response from fade enable/disable commands.
///
/// Tasmota returns either `{"Fade":"ON"}`, `{"Fade":"OFF"}`, or numeric values.
///
/// # Examples
///
/// ```
/// use tasmor_lib::response::FadeResponse;
///
/// let json = r#"{"Fade":"ON"}"#;
/// let response: FadeResponse = serde_json::from_str(json).unwrap();
/// assert!(response.is_enabled().unwrap());
///
/// let json = r#"{"Fade":"OFF"}"#;
/// let response: FadeResponse = serde_json::from_str(json).unwrap();
/// assert!(!response.is_enabled().unwrap());
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FadeResponse {
    fade: FadeValue,
}

/// Helper enum to handle both string and numeric fade values.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum FadeValue {
    Text(String),
    Number(u8),
}

impl FadeResponse {
    /// Returns whether fade transitions are enabled.
    ///
    /// # Errors
    ///
    /// Returns error if the value cannot be interpreted as a boolean.
    pub fn is_enabled(&self) -> Result<bool, ParseError> {
        match &self.fade {
            FadeValue::Text(s) => match s.to_uppercase().as_str() {
                "ON" | "1" => Ok(true),
                "OFF" | "0" => Ok(false),
                _ => Err(ParseError::InvalidValue {
                    field: "Fade".to_string(),
                    message: format!("expected ON, OFF, 0, or 1, got '{s}'"),
                }),
            },
            FadeValue::Number(n) => Ok(*n != 0),
        }
    }
}

/// Response from fade speed (Speed) commands.
///
/// Tasmota returns `{"Speed":X}` where X is 1-40.
///
/// # Examples
///
/// ```
/// use tasmor_lib::response::FadeSpeedResponse;
///
/// let json = r#"{"Speed":20}"#;
/// let response: FadeSpeedResponse = serde_json::from_str(json).unwrap();
/// assert_eq!(response.speed_value(), 20);
/// assert!(response.speed().is_ok());
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FadeSpeedResponse {
    speed: u8,
}

impl FadeSpeedResponse {
    /// Returns the raw speed value (1-40).
    #[must_use]
    pub fn speed_value(&self) -> u8 {
        self.speed
    }

    /// Returns the speed as a validated `FadeSpeed` type.
    ///
    /// # Errors
    ///
    /// Returns error if the value is outside the valid range (1-40).
    pub fn speed(&self) -> Result<FadeSpeed, ParseError> {
        FadeSpeed::new(self.speed).map_err(|_| ParseError::InvalidValue {
            field: "Speed".to_string(),
            message: format!("expected 1-40, got {}", self.speed),
        })
    }
}

/// Response from startup fade (`SetOption91`) commands.
///
/// Tasmota returns `{"SetOption91":"ON"}` or `{"SetOption91":"OFF"}`.
///
/// # Examples
///
/// ```
/// use tasmor_lib::response::StartupFadeResponse;
///
/// let json = r#"{"SetOption91":"ON"}"#;
/// let response: StartupFadeResponse = serde_json::from_str(json).unwrap();
/// assert!(response.is_enabled().unwrap());
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct StartupFadeResponse {
    #[serde(rename = "SetOption91")]
    set_option_91: SetOptionValue,
}

/// Helper enum to handle both string and numeric `SetOption` values.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum SetOptionValue {
    Text(String),
    Number(u8),
}

impl StartupFadeResponse {
    /// Returns whether fade at startup is enabled.
    ///
    /// # Errors
    ///
    /// Returns error if the value cannot be interpreted as a boolean.
    pub fn is_enabled(&self) -> Result<bool, ParseError> {
        match &self.set_option_91 {
            SetOptionValue::Text(s) => match s.to_uppercase().as_str() {
                "ON" | "1" => Ok(true),
                "OFF" | "0" => Ok(false),
                _ => Err(ParseError::InvalidValue {
                    field: "SetOption91".to_string(),
                    message: format!("expected ON, OFF, 0, or 1, got '{s}'"),
                }),
            },
            SetOptionValue::Number(n) => Ok(*n != 0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fade_response_on() {
        let json = r#"{"Fade":"ON"}"#;
        let response: FadeResponse = serde_json::from_str(json).unwrap();
        assert!(response.is_enabled().unwrap());
    }

    #[test]
    fn fade_response_off() {
        let json = r#"{"Fade":"OFF"}"#;
        let response: FadeResponse = serde_json::from_str(json).unwrap();
        assert!(!response.is_enabled().unwrap());
    }

    #[test]
    fn fade_response_numeric() {
        let json = r#"{"Fade":1}"#;
        let response: FadeResponse = serde_json::from_str(json).unwrap();
        assert!(response.is_enabled().unwrap());

        let json = r#"{"Fade":0}"#;
        let response: FadeResponse = serde_json::from_str(json).unwrap();
        assert!(!response.is_enabled().unwrap());
    }

    #[test]
    fn fade_speed_response() {
        let json = r#"{"Speed":20}"#;
        let response: FadeSpeedResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.speed_value(), 20);
        assert!(response.speed().is_ok());
    }

    #[test]
    fn fade_speed_invalid() {
        // Value outside range should fail validation
        let json = r#"{"Speed":50}"#;
        let response: FadeSpeedResponse = serde_json::from_str(json).unwrap();
        assert!(response.speed().is_err());
    }

    #[test]
    fn startup_fade_response_on() {
        let json = r#"{"SetOption91":"ON"}"#;
        let response: StartupFadeResponse = serde_json::from_str(json).unwrap();
        assert!(response.is_enabled().unwrap());
    }

    #[test]
    fn startup_fade_response_off() {
        let json = r#"{"SetOption91":"OFF"}"#;
        let response: StartupFadeResponse = serde_json::from_str(json).unwrap();
        assert!(!response.is_enabled().unwrap());
    }

    #[test]
    fn startup_fade_response_numeric() {
        let json = r#"{"SetOption91":1}"#;
        let response: StartupFadeResponse = serde_json::from_str(json).unwrap();
        assert!(response.is_enabled().unwrap());
    }
}
