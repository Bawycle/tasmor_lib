// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! RGB color response.
//!
//! This module provides a response type that wraps the HSB response from
//! Tasmota and converts it back to RGB for user convenience.

use crate::types::{HsbColor, RgbColor};

/// Response from an RGB color command.
///
/// When setting an RGB color, the library internally converts it to HSB
/// and sends an `HSBColor` command to Tasmota. This response wraps the
/// HSB response and provides both the RGB and HSB representations.
///
/// # Examples
///
/// ```
/// use tasmor_lib::response::RgbColorResponse;
/// use tasmor_lib::types::{HsbColor, RgbColor};
///
/// // Create a response from HSB values
/// let hsb = HsbColor::new(0, 100, 100).unwrap();
/// let response = RgbColorResponse::from_hsb(hsb);
///
/// // Access both representations
/// let rgb = response.rgb_color();
/// let hsb = response.hsb_color();
///
/// // Red in HSB (0°, 100%, 100%) should be close to RGB (255, 0, 0)
/// assert_eq!(rgb.red(), 255);
/// assert_eq!(hsb.hue(), 0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbColorResponse {
    rgb_color: RgbColor,
    hsb_color: HsbColor,
}

impl RgbColorResponse {
    /// Creates a new RGB color response from an HSB color.
    ///
    /// This is typically used internally when converting the Tasmota
    /// HSB response back to RGB for the user.
    #[must_use]
    pub fn from_hsb(hsb: HsbColor) -> Self {
        Self {
            rgb_color: RgbColor::from_hsb(&hsb),
            hsb_color: hsb,
        }
    }

    /// Creates a new RGB color response from both RGB and HSB colors.
    ///
    /// This allows preserving the original RGB value that was sent,
    /// avoiding potential rounding differences from double conversion.
    #[must_use]
    pub const fn new(rgb_color: RgbColor, hsb_color: HsbColor) -> Self {
        Self {
            rgb_color,
            hsb_color,
        }
    }

    /// Returns the RGB color.
    #[must_use]
    pub const fn rgb_color(&self) -> RgbColor {
        self.rgb_color
    }

    /// Returns the HSB color as reported by Tasmota.
    #[must_use]
    pub const fn hsb_color(&self) -> HsbColor {
        self.hsb_color
    }

    /// Returns the red component (0-255).
    #[must_use]
    pub const fn red(&self) -> u8 {
        self.rgb_color.red()
    }

    /// Returns the green component (0-255).
    #[must_use]
    pub const fn green(&self) -> u8 {
        self.rgb_color.green()
    }

    /// Returns the blue component (0-255).
    #[must_use]
    pub const fn blue(&self) -> u8 {
        self.rgb_color.blue()
    }

    /// Returns the color as a hex string without the hash prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::response::RgbColorResponse;
    /// use tasmor_lib::types::HsbColor;
    ///
    /// let hsb = HsbColor::new(0, 100, 100).unwrap();  // Red
    /// let response = RgbColorResponse::from_hsb(hsb);
    /// assert_eq!(response.to_hex(), "FF0000");
    /// ```
    #[must_use]
    pub fn to_hex(&self) -> String {
        self.rgb_color.to_hex()
    }

    /// Returns the color as a hex string with the hash prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::response::RgbColorResponse;
    /// use tasmor_lib::types::HsbColor;
    ///
    /// let hsb = HsbColor::new(0, 100, 100).unwrap();  // Red
    /// let response = RgbColorResponse::from_hsb(hsb);
    /// assert_eq!(response.to_hex_with_hash(), "#FF0000");
    /// ```
    #[must_use]
    pub fn to_hex_with_hash(&self) -> String {
        self.rgb_color.to_hex_with_hash()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_color_response_from_hsb() {
        // Red: HSB(0, 100, 100) -> RGB(255, 0, 0)
        let hsb = HsbColor::new(0, 100, 100).unwrap();
        let response = RgbColorResponse::from_hsb(hsb);

        assert_eq!(response.red(), 255);
        assert_eq!(response.green(), 0);
        assert_eq!(response.blue(), 0);
        assert_eq!(response.hsb_color(), hsb);
    }

    #[test]
    fn rgb_color_response_new() {
        let rgb = RgbColor::new(255, 128, 64);
        let hsb = HsbColor::new(20, 75, 100).unwrap();
        let response = RgbColorResponse::new(rgb, hsb);

        assert_eq!(response.rgb_color(), rgb);
        assert_eq!(response.hsb_color(), hsb);
    }

    #[test]
    fn rgb_color_response_hex() {
        let hsb = HsbColor::new(0, 100, 100).unwrap();
        let response = RgbColorResponse::from_hsb(hsb);

        assert_eq!(response.to_hex(), "FF0000");
        assert_eq!(response.to_hex_with_hash(), "#FF0000");
    }

    #[test]
    fn rgb_color_response_green() {
        // Green: HSB(120, 100, 100) -> RGB(0, 255, 0)
        let hsb = HsbColor::new(120, 100, 100).unwrap();
        let response = RgbColorResponse::from_hsb(hsb);

        assert_eq!(response.red(), 0);
        assert_eq!(response.green(), 255);
        assert_eq!(response.blue(), 0);
    }

    #[test]
    fn rgb_color_response_blue() {
        // Blue: HSB(240, 100, 100) -> RGB(0, 0, 255)
        let hsb = HsbColor::new(240, 100, 100).unwrap();
        let response = RgbColorResponse::from_hsb(hsb);

        assert_eq!(response.red(), 0);
        assert_eq!(response.green(), 0);
        assert_eq!(response.blue(), 255);
    }
}
