// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Color response parsing for HSB color and color temperature.

use serde::Deserialize;

use crate::error::ParseError;
use crate::types::{ColorTemp, HsbColor, PowerState};

/// Response from an `HSBColor` command.
///
/// Tasmota returns HSB color in JSON format like:
/// - `{"HSBColor": "180,100,75"}` for color-only response
/// - `{"HSBColor": "180,100,75", "Dimmer": 75, "POWER": "ON"}` with state
///
/// The HSB values are:
/// - Hue: 0-360 degrees on the color wheel
/// - Saturation: 0-100 percentage
/// - Brightness: 0-100 percentage
///
/// # Examples
///
/// ```
/// use tasmor_lib::response::HsbColorResponse;
///
/// let json = r#"{"HSBColor": "180,100,75", "POWER": "ON"}"#;
/// let response: HsbColorResponse = serde_json::from_str(json).unwrap();
/// assert_eq!(response.hue().unwrap(), 180);
/// assert_eq!(response.saturation().unwrap(), 100);
/// assert_eq!(response.brightness().unwrap(), 75);
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct HsbColorResponse {
    /// The HSB color as a comma-separated string (e.g., "180,100,75").
    #[serde(rename = "HSBColor")]
    hsb_color: String,

    /// Optional dimmer level included in the response.
    #[serde(rename = "Dimmer", default)]
    dimmer: Option<u8>,

    /// Optional power state included in the response.
    #[serde(rename = "POWER", default)]
    power: Option<String>,
}

impl HsbColorResponse {
    /// Parses the HSB color string into components.
    fn parse_hsb(&self) -> Result<(u16, u8, u8), ParseError> {
        let parts: Vec<&str> = self.hsb_color.split(',').collect();

        if parts.len() != 3 {
            return Err(ParseError::InvalidValue {
                field: "HSBColor".to_string(),
                message: format!("expected 3 comma-separated values, got: {}", self.hsb_color),
            });
        }

        let hue = parts[0]
            .parse::<u16>()
            .map_err(|_| ParseError::InvalidValue {
                field: "HSBColor.hue".to_string(),
                message: format!("invalid hue value: {}", parts[0]),
            })?;

        let saturation = parts[1]
            .parse::<u8>()
            .map_err(|_| ParseError::InvalidValue {
                field: "HSBColor.saturation".to_string(),
                message: format!("invalid saturation value: {}", parts[1]),
            })?;

        let brightness = parts[2]
            .parse::<u8>()
            .map_err(|_| ParseError::InvalidValue {
                field: "HSBColor.brightness".to_string(),
                message: format!("invalid brightness value: {}", parts[2]),
            })?;

        Ok((hue, saturation, brightness))
    }

    /// Returns the hue component (0-360).
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if the HSB color string is malformed.
    pub fn hue(&self) -> Result<u16, ParseError> {
        self.parse_hsb().map(|(h, _, _)| h)
    }

    /// Returns the saturation component (0-100).
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if the HSB color string is malformed.
    pub fn saturation(&self) -> Result<u8, ParseError> {
        self.parse_hsb().map(|(_, s, _)| s)
    }

    /// Returns the brightness component (0-100).
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if the HSB color string is malformed.
    pub fn brightness(&self) -> Result<u8, ParseError> {
        self.parse_hsb().map(|(_, _, b)| b)
    }

    /// Returns all HSB components as a tuple (hue, saturation, brightness).
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if the HSB color string is malformed.
    pub fn as_tuple(&self) -> Result<(u16, u8, u8), ParseError> {
        self.parse_hsb()
    }

    /// Returns the HSB color as an [`HsbColor`](crate::types::HsbColor) type.
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if the HSB color string is malformed or values are out of range.
    pub fn hsb_color(&self) -> Result<HsbColor, ParseError> {
        let (hue, saturation, brightness) = self.parse_hsb()?;
        HsbColor::new(hue, saturation, brightness).map_err(|e| ParseError::InvalidValue {
            field: "HSBColor".to_string(),
            message: e.to_string(),
        })
    }

    /// Returns the raw HSB color string.
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.hsb_color
    }

    /// Returns the dimmer level if included in the response.
    #[must_use]
    pub fn dimmer(&self) -> Option<u8> {
        self.dimmer
    }

    /// Returns the power state if included in the response.
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

/// Response from a `CT` (Color Temperature) command.
///
/// Tasmota returns color temperature in JSON format like:
/// - `{"CT": 250}` for CT-only response
/// - `{"CT": 250, "POWER": "ON"}` with power state
///
/// Color temperature is measured in mireds (micro reciprocal degrees):
/// - 153 = coldest (6500K, daylight)
/// - 500 = warmest (2000K, candlelight)
///
/// # Examples
///
/// ```
/// use tasmor_lib::response::ColorTempResponse;
///
/// let json = r#"{"CT": 326, "POWER": "ON"}"#;
/// let response: ColorTempResponse = serde_json::from_str(json).unwrap();
/// assert_eq!(response.ct(), 326);
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct ColorTempResponse {
    /// The color temperature in mireds (153-500).
    #[serde(rename = "CT")]
    ct: u16,

    /// Optional power state included in the response.
    #[serde(rename = "POWER", default)]
    power: Option<String>,
}

impl ColorTempResponse {
    /// Returns the color temperature in mireds (153-500).
    #[must_use]
    pub fn ct(&self) -> u16 {
        self.ct
    }

    /// Returns the approximate Kelvin temperature.
    ///
    /// Calculated as 1,000,000 / mireds.
    #[must_use]
    pub fn kelvin(&self) -> u32 {
        if self.ct == 0 {
            0
        } else {
            1_000_000 / u32::from(self.ct)
        }
    }

    /// Returns the power state if included in the response.
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

    /// Returns the color temperature as a [`ColorTemp`](crate::types::ColorTemp) type.
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if the color temperature value is out of range.
    pub fn color_temp(&self) -> Result<ColorTemp, ParseError> {
        ColorTemp::new(self.ct).map_err(|e| ParseError::InvalidValue {
            field: "CT".to_string(),
            message: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // HsbColorResponse tests
    // ========================================================================

    #[test]
    fn parse_hsb_color_only() {
        let json = r#"{"HSBColor": "180,100,75"}"#;
        let response: HsbColorResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.hue().unwrap(), 180);
        assert_eq!(response.saturation().unwrap(), 100);
        assert_eq!(response.brightness().unwrap(), 75);
        assert_eq!(response.as_tuple().unwrap(), (180, 100, 75));
        assert!(response.power_state().unwrap().is_none());
    }

    #[test]
    fn parse_hsb_color_with_power() {
        let json = r#"{"HSBColor": "0,100,100", "POWER": "ON"}"#;
        let response: HsbColorResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.hue().unwrap(), 0);
        assert_eq!(response.power_state().unwrap().unwrap(), PowerState::On);
        assert_eq!(response.is_on(), Some(true));
    }

    #[test]
    fn parse_hsb_color_with_dimmer() {
        let json = r#"{"HSBColor": "120,80,50", "Dimmer": 50}"#;
        let response: HsbColorResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.hue().unwrap(), 120);
        assert_eq!(response.dimmer(), Some(50));
    }

    #[test]
    fn parse_hsb_color_full_response() {
        let json = r#"{
            "HSBColor": "240,50,75",
            "Dimmer": 75,
            "POWER": "ON",
            "Color": "5959BF",
            "White": 0,
            "CT": 327
        }"#;
        let response: HsbColorResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.as_tuple().unwrap(), (240, 50, 75));
        assert_eq!(response.dimmer(), Some(75));
        assert_eq!(response.power_state().unwrap().unwrap(), PowerState::On);
    }

    #[test]
    fn parse_hsb_color_red() {
        let json = r#"{"HSBColor": "0,100,100"}"#;
        let response: HsbColorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.as_tuple().unwrap(), (0, 100, 100));
    }

    #[test]
    fn parse_hsb_color_green() {
        let json = r#"{"HSBColor": "120,100,100"}"#;
        let response: HsbColorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.as_tuple().unwrap(), (120, 100, 100));
    }

    #[test]
    fn parse_hsb_color_blue() {
        let json = r#"{"HSBColor": "240,100,100"}"#;
        let response: HsbColorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.as_tuple().unwrap(), (240, 100, 100));
    }

    #[test]
    fn parse_hsb_color_max_hue() {
        let json = r#"{"HSBColor": "360,100,100"}"#;
        let response: HsbColorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.hue().unwrap(), 360);
    }

    #[test]
    fn parse_hsb_invalid_format() {
        let json = r#"{"HSBColor": "180,100"}"#; // Missing brightness
        let response: HsbColorResponse = serde_json::from_str(json).unwrap();
        assert!(response.as_tuple().is_err());
    }

    // ========================================================================
    // ColorTempResponse tests
    // ========================================================================

    #[test]
    fn parse_ct_only() {
        let json = r#"{"CT": 326}"#;
        let response: ColorTempResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.ct(), 326);
        assert!(response.power_state().unwrap().is_none());
    }

    #[test]
    fn parse_ct_with_power() {
        let json = r#"{"CT": 250, "POWER": "ON"}"#;
        let response: ColorTempResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.ct(), 250);
        assert_eq!(response.power_state().unwrap().unwrap(), PowerState::On);
    }

    #[test]
    fn parse_ct_coldest() {
        let json = r#"{"CT": 153}"#;
        let response: ColorTempResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.ct(), 153);
        assert_eq!(response.kelvin(), 6535); // ~6500K
    }

    #[test]
    fn parse_ct_warmest() {
        let json = r#"{"CT": 500}"#;
        let response: ColorTempResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.ct(), 500);
        assert_eq!(response.kelvin(), 2000); // 2000K
    }

    #[test]
    fn parse_ct_neutral() {
        let json = r#"{"CT": 326}"#;
        let response: ColorTempResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.ct(), 326);
        assert_eq!(response.kelvin(), 3067); // ~3000K neutral
    }

    #[test]
    fn parse_ct_with_additional_fields() {
        let json = r#"{
            "CT": 327,
            "POWER": "ON",
            "Dimmer": 100,
            "Color": "FFFFFF"
        }"#;
        let response: ColorTempResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.ct(), 327);
        assert_eq!(response.power_state().unwrap().unwrap(), PowerState::On);
    }
}
