// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! RGB color type with hex parsing and HSB conversion.
//!
//! This module provides an RGB color representation that can be converted
//! to/from HSB format for use with Tasmota devices.
//!
//! # Device Methods
//!
//! Use [`RgbColor`] with this [`Device`](crate::Device) method:
//! - [`set_rgb_color()`](crate::Device::set_rgb_color) - Set color using RGB values

use std::fmt;
use std::str::FromStr;

use crate::error::ValueError;

use super::HsbColor;

/// RGB color with 8-bit channels (0-255).
///
/// This type provides a convenient way to work with colors in the familiar
/// RGB format. Colors are converted to HSB internally when sent to Tasmota
/// devices.
///
/// # Examples
///
/// ```
/// use tasmor_lib::types::RgbColor;
///
/// // Create from RGB values
/// let color = RgbColor::new(255, 128, 0);  // Orange
/// assert_eq!(color.red(), 255);
/// assert_eq!(color.green(), 128);
/// assert_eq!(color.blue(), 0);
///
/// // Parse from hex string
/// let red = RgbColor::from_hex("#FF0000").unwrap();
/// assert_eq!(red.red(), 255);
/// assert_eq!(red.green(), 0);
/// assert_eq!(red.blue(), 0);
///
/// // Convert to hex
/// assert_eq!(red.to_hex(), "FF0000");
/// assert_eq!(red.to_hex_with_hash(), "#FF0000");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RgbColor {
    red: u8,
    green: u8,
    blue: u8,
}

impl RgbColor {
    /// Creates a new RGB color.
    ///
    /// # Arguments
    ///
    /// * `red` - Red component (0-255)
    /// * `green` - Green component (0-255)
    /// * `blue` - Blue component (0-255)
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::types::RgbColor;
    ///
    /// let orange = RgbColor::new(255, 165, 0);
    /// ```
    #[must_use]
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }

    /// Parses an RGB color from a hex string.
    ///
    /// Accepts formats: `#RRGGBB`, `RRGGBB`, `#RGB`, `RGB`
    ///
    /// # Arguments
    ///
    /// * `hex` - The hex color string
    ///
    /// # Errors
    ///
    /// Returns `ValueError` if the hex string is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::types::RgbColor;
    ///
    /// let color = RgbColor::from_hex("#FF5733").unwrap();
    /// assert_eq!(color.red(), 255);
    /// assert_eq!(color.green(), 87);
    /// assert_eq!(color.blue(), 51);
    ///
    /// // Without hash
    /// let color = RgbColor::from_hex("00FF00").unwrap();
    /// assert_eq!(color.green(), 255);
    ///
    /// // Short format
    /// let color = RgbColor::from_hex("#F00").unwrap();
    /// assert_eq!(color.red(), 255);
    /// ```
    pub fn from_hex(hex: &str) -> Result<Self, ValueError> {
        let hex = hex.trim_start_matches('#');

        match hex.len() {
            3 => {
                // Short format: RGB -> RRGGBB
                let chars: Vec<char> = hex.chars().collect();
                let r = parse_hex_char(chars[0])?;
                let g = parse_hex_char(chars[1])?;
                let b = parse_hex_char(chars[2])?;
                Ok(Self::new(r * 17, g * 17, b * 17)) // Expand 0-F to 0-255
            }
            6 => {
                // Full format: RRGGBB
                let r = parse_hex_pair(&hex[0..2])?;
                let g = parse_hex_pair(&hex[2..4])?;
                let b = parse_hex_pair(&hex[4..6])?;
                Ok(Self::new(r, g, b))
            }
            _ => Err(ValueError::InvalidHexColor(hex.to_string())),
        }
    }

    /// Returns the red component.
    #[must_use]
    pub const fn red(&self) -> u8 {
        self.red
    }

    /// Returns the green component.
    #[must_use]
    pub const fn green(&self) -> u8 {
        self.green
    }

    /// Returns the blue component.
    #[must_use]
    pub const fn blue(&self) -> u8 {
        self.blue
    }

    /// Returns the color as a hex string without the hash prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::types::RgbColor;
    ///
    /// let color = RgbColor::new(255, 128, 0);
    /// assert_eq!(color.to_hex(), "FF8000");
    /// ```
    #[must_use]
    pub fn to_hex(&self) -> String {
        format!("{:02X}{:02X}{:02X}", self.red, self.green, self.blue)
    }

    /// Returns the color as a hex string with the hash prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::types::RgbColor;
    ///
    /// let color = RgbColor::new(255, 128, 0);
    /// assert_eq!(color.to_hex_with_hash(), "#FF8000");
    /// ```
    #[must_use]
    pub fn to_hex_with_hash(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.red, self.green, self.blue)
    }

    /// Converts this RGB color to HSB format.
    ///
    /// Note: Due to rounding in the conversion, converting RGB to HSB and back
    /// may not produce the exact same RGB values.
    ///
    /// # Panics
    ///
    /// This method should never panic as the internal conversion always produces
    /// valid HSB values. If it does panic, it indicates a bug in the conversion
    /// algorithm.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::types::RgbColor;
    ///
    /// let red = RgbColor::new(255, 0, 0);
    /// let hsb = red.to_hsb();
    /// assert_eq!(hsb.hue(), 0);
    /// assert_eq!(hsb.saturation(), 100);
    /// assert_eq!(hsb.brightness(), 100);
    /// ```
    #[must_use]
    pub fn to_hsb(&self) -> HsbColor {
        let (h, s, b) = rgb_to_hsb(self.red, self.green, self.blue);
        // Safe: rgb_to_hsb always returns valid HSB values
        HsbColor::new(h, s, b).expect("rgb_to_hsb should return valid HSB values")
    }

    /// Creates an RGB color from an HSB color.
    ///
    /// Note: Due to rounding in the conversion, converting HSB to RGB and back
    /// may not produce the exact same HSB values.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::types::{RgbColor, HsbColor};
    ///
    /// let hsb = HsbColor::red();
    /// let rgb = RgbColor::from_hsb(&hsb);
    /// assert_eq!(rgb.red(), 255);
    /// assert_eq!(rgb.green(), 0);
    /// assert_eq!(rgb.blue(), 0);
    /// ```
    #[must_use]
    pub fn from_hsb(hsb: &HsbColor) -> Self {
        let (r, g, b) = hsb_to_rgb(hsb.hue(), hsb.saturation(), hsb.brightness());
        Self::new(r, g, b)
    }

    /// Creates a pure red color.
    #[must_use]
    pub const fn red_color() -> Self {
        Self::new(255, 0, 0)
    }

    /// Creates a pure green color.
    #[must_use]
    pub const fn green_color() -> Self {
        Self::new(0, 255, 0)
    }

    /// Creates a pure blue color.
    #[must_use]
    pub const fn blue_color() -> Self {
        Self::new(0, 0, 255)
    }

    /// Creates a white color.
    #[must_use]
    pub const fn white() -> Self {
        Self::new(255, 255, 255)
    }

    /// Creates a black color.
    #[must_use]
    pub const fn black() -> Self {
        Self::new(0, 0, 0)
    }
}

impl Default for RgbColor {
    fn default() -> Self {
        Self::white()
    }
}

impl fmt::Display for RgbColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex_with_hash())
    }
}

impl FromStr for RgbColor {
    type Err = ValueError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_hex(s)
    }
}

impl TryFrom<&str> for RgbColor {
    type Error = ValueError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_hex(value)
    }
}

impl From<(u8, u8, u8)> for RgbColor {
    fn from((red, green, blue): (u8, u8, u8)) -> Self {
        Self::new(red, green, blue)
    }
}

// Helper function to parse a single hex character
fn parse_hex_char(c: char) -> Result<u8, ValueError> {
    c.to_digit(16)
        .and_then(|d| u8::try_from(d).ok())
        .ok_or_else(|| ValueError::InvalidHexColor(c.to_string()))
}

// Helper function to parse a two-character hex pair
fn parse_hex_pair(s: &str) -> Result<u8, ValueError> {
    u8::from_str_radix(s, 16).map_err(|_| ValueError::InvalidHexColor(s.to_string()))
}

/// Converts RGB values to HSB.
///
/// Returns (hue: 0-360, saturation: 0-100, brightness: 0-100)
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]
fn rgb_to_hsb(r: u8, g: u8, b: u8) -> (u16, u8, u8) {
    let r = f32::from(r) / 255.0;
    let g = f32::from(g) / 255.0;
    let b = f32::from(b) / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    // Brightness (0-100)
    let brightness = (max * 100.0).round() as u8;

    // Saturation (0-100)
    let saturation = if max == 0.0 {
        0
    } else {
        ((delta / max) * 100.0).round() as u8
    };

    // Hue (0-360)
    let hue = if delta < f32::EPSILON {
        0
    } else if (max - r).abs() < f32::EPSILON {
        let h = 60.0 * (((g - b) / delta) % 6.0);
        if h < 0.0 {
            (h + 360.0).round() as u16
        } else {
            h.round() as u16
        }
    } else if (max - g).abs() < f32::EPSILON {
        (60.0 * (((b - r) / delta) + 2.0)).round() as u16
    } else {
        (60.0 * (((r - g) / delta) + 4.0)).round() as u16
    };

    (hue, saturation, brightness)
}

/// Converts HSB values to RGB.
///
/// Takes (hue: 0-360, saturation: 0-100, brightness: 0-100)
/// Returns (red: 0-255, green: 0-255, blue: 0-255)
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]
fn hsb_to_rgb(h: u16, s: u8, v: u8) -> (u8, u8, u8) {
    let s = f32::from(s) / 100.0;
    let v = f32::from(v) / 100.0;
    let h = f32::from(h);

    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (
        ((r + m) * 255.0).round() as u8,
        ((g + m) * 255.0).round() as u8,
        ((b + m) * 255.0).round() as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_new() {
        let color = RgbColor::new(255, 128, 0);
        assert_eq!(color.red(), 255);
        assert_eq!(color.green(), 128);
        assert_eq!(color.blue(), 0);
    }

    #[test]
    fn rgb_from_hex_full() {
        let color = RgbColor::from_hex("#FF5733").unwrap();
        assert_eq!(color.red(), 255);
        assert_eq!(color.green(), 87);
        assert_eq!(color.blue(), 51);

        // Without hash
        let color = RgbColor::from_hex("00FF00").unwrap();
        assert_eq!(color.red(), 0);
        assert_eq!(color.green(), 255);
        assert_eq!(color.blue(), 0);
    }

    #[test]
    fn rgb_from_hex_short() {
        let color = RgbColor::from_hex("#F00").unwrap();
        assert_eq!(color.red(), 255);
        assert_eq!(color.green(), 0);
        assert_eq!(color.blue(), 0);

        let color = RgbColor::from_hex("0F0").unwrap();
        assert_eq!(color.red(), 0);
        assert_eq!(color.green(), 255);
        assert_eq!(color.blue(), 0);
    }

    #[test]
    fn rgb_from_hex_invalid() {
        assert!(RgbColor::from_hex("#GG0000").is_err());
        assert!(RgbColor::from_hex("#FF00").is_err());
        assert!(RgbColor::from_hex("").is_err());
    }

    #[test]
    fn rgb_to_hex() {
        let color = RgbColor::new(255, 128, 0);
        assert_eq!(color.to_hex(), "FF8000");
        assert_eq!(color.to_hex_with_hash(), "#FF8000");
    }

    #[test]
    fn rgb_to_hex_leading_zeros() {
        let color = RgbColor::new(0, 15, 255);
        assert_eq!(color.to_hex(), "000FFF");
    }

    #[test]
    fn rgb_to_hsb_red() {
        let rgb = RgbColor::new(255, 0, 0);
        let hsb = rgb.to_hsb();
        assert_eq!(hsb.hue(), 0);
        assert_eq!(hsb.saturation(), 100);
        assert_eq!(hsb.brightness(), 100);
    }

    #[test]
    fn rgb_to_hsb_green() {
        let rgb = RgbColor::new(0, 255, 0);
        let hsb = rgb.to_hsb();
        assert_eq!(hsb.hue(), 120);
        assert_eq!(hsb.saturation(), 100);
        assert_eq!(hsb.brightness(), 100);
    }

    #[test]
    fn rgb_to_hsb_blue() {
        let rgb = RgbColor::new(0, 0, 255);
        let hsb = rgb.to_hsb();
        assert_eq!(hsb.hue(), 240);
        assert_eq!(hsb.saturation(), 100);
        assert_eq!(hsb.brightness(), 100);
    }

    #[test]
    fn rgb_to_hsb_white() {
        let rgb = RgbColor::white();
        let hsb = rgb.to_hsb();
        assert_eq!(hsb.saturation(), 0);
        assert_eq!(hsb.brightness(), 100);
    }

    #[test]
    fn rgb_to_hsb_black() {
        let rgb = RgbColor::black();
        let hsb = rgb.to_hsb();
        assert_eq!(hsb.brightness(), 0);
    }

    #[test]
    fn hsb_to_rgb_red() {
        let hsb = HsbColor::red();
        let rgb = RgbColor::from_hsb(&hsb);
        assert_eq!(rgb.red(), 255);
        assert_eq!(rgb.green(), 0);
        assert_eq!(rgb.blue(), 0);
    }

    #[test]
    fn hsb_to_rgb_green() {
        let hsb = HsbColor::green();
        let rgb = RgbColor::from_hsb(&hsb);
        assert_eq!(rgb.red(), 0);
        assert_eq!(rgb.green(), 255);
        assert_eq!(rgb.blue(), 0);
    }

    #[test]
    fn hsb_to_rgb_blue() {
        let hsb = HsbColor::blue();
        let rgb = RgbColor::from_hsb(&hsb);
        assert_eq!(rgb.red(), 0);
        assert_eq!(rgb.green(), 0);
        assert_eq!(rgb.blue(), 255);
    }

    #[test]
    fn rgb_presets() {
        assert_eq!(RgbColor::red_color().red(), 255);
        assert_eq!(RgbColor::red_color().green(), 0);
        assert_eq!(RgbColor::red_color().blue(), 0);

        assert_eq!(RgbColor::green_color().green(), 255);
        assert_eq!(RgbColor::blue_color().blue(), 255);
        assert_eq!(RgbColor::white(), RgbColor::new(255, 255, 255));
        assert_eq!(RgbColor::black(), RgbColor::new(0, 0, 0));
    }

    #[test]
    fn rgb_display() {
        let color = RgbColor::new(255, 128, 0);
        assert_eq!(color.to_string(), "#FF8000");
    }

    #[test]
    fn rgb_from_str() {
        let color: RgbColor = "#FF0000".parse().unwrap();
        assert_eq!(color, RgbColor::red_color());
    }

    #[test]
    fn rgb_try_from() {
        let color: RgbColor = "#00FF00".try_into().unwrap();
        assert_eq!(color, RgbColor::green_color());
    }

    #[test]
    fn rgb_from_tuple() {
        let color: RgbColor = (255u8, 0u8, 0u8).into();
        assert_eq!(color, RgbColor::red_color());
    }

    #[test]
    fn rgb_default() {
        assert_eq!(RgbColor::default(), RgbColor::white());
    }

    #[test]
    fn roundtrip_rgb_hsb_rgb() {
        // Test that primary colors roundtrip correctly
        let colors = [
            RgbColor::red_color(),
            RgbColor::green_color(),
            RgbColor::blue_color(),
            RgbColor::white(),
            RgbColor::black(),
        ];

        for original in colors {
            let hsb = original.to_hsb();
            let roundtrip = RgbColor::from_hsb(&hsb);
            assert_eq!(
                original, roundtrip,
                "Color {original:?} did not roundtrip correctly"
            );
        }
    }
}
