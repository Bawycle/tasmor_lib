// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Color types for light control.
//!
//! This module provides types for color temperature (CCT) and HSB color
//! control on Tasmota light devices.
//!
//! # Device Methods
//!
//! Use [`ColorTemperature`] with these [`Device`](crate::Device) methods:
//! - [`set_color_temperature()`](crate::Device::set_color_temperature) - Set white color temperature
//! - [`get_color_temperature()`](crate::Device::get_color_temperature) - Query current color temperature
//!
//! Use [`HsbColor`] with these [`Device`](crate::Device) methods:
//! - [`set_hsb_color()`](crate::Device::set_hsb_color) - Set color using HSB values
//! - [`get_hsb_color()`](crate::Device::get_hsb_color) - Query current HSB color

use std::fmt;

use crate::error::ValueError;

/// Color temperature in mireds (153-500).
///
/// Tasmota uses mireds for color temperature, where lower values are cooler
/// (bluer) and higher values are warmer (more orange/yellow).
///
/// - 153 (6500K) - Cool daylight
/// - 250 (4000K) - Neutral white
/// - 500 (2000K) - Warm candlelight
///
/// # Examples
///
/// ```
/// use tasmor_lib::types::ColorTemperature;
///
/// // Create a neutral white color temperature
/// let ct = ColorTemperature::new(250).unwrap();
/// assert_eq!(ct.value(), 250);
///
/// // Use predefined values
/// let cool = ColorTemperature::COOL;
/// let warm = ColorTemperature::WARM;
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct ColorTemperature(u16);

impl ColorTemperature {
    /// Minimum color temperature (coolest, ~6500K).
    pub const MIN: u16 = 153;

    /// Maximum color temperature (warmest, ~2000K).
    pub const MAX: u16 = 500;

    /// Cool daylight (~6500K).
    pub const COOL: Self = Self(153);

    /// Neutral white (~4000K).
    pub const NEUTRAL: Self = Self(250);

    /// Warm white (~2700K).
    pub const WARM: Self = Self(370);

    /// Candlelight (~2000K).
    pub const CANDLE: Self = Self(500);

    /// Creates a new color temperature value.
    ///
    /// # Arguments
    ///
    /// * `value` - The color temperature in mireds (153-500)
    ///
    /// # Errors
    ///
    /// Returns `ValueError::OutOfRange` if value is outside [153, 500].
    pub fn new(value: u16) -> Result<Self, ValueError> {
        if !(Self::MIN..=Self::MAX).contains(&value) {
            return Err(ValueError::OutOfRange {
                min: Self::MIN,
                max: Self::MAX,
                actual: value,
            });
        }
        Ok(Self(value))
    }

    /// Creates a color temperature, clamping to the valid range.
    #[must_use]
    pub const fn clamped(value: u16) -> Self {
        if value < Self::MIN {
            Self(Self::MIN)
        } else if value > Self::MAX {
            Self(Self::MAX)
        } else {
            Self(value)
        }
    }

    /// Returns the color temperature value in mireds.
    #[must_use]
    pub const fn value(&self) -> u16 {
        self.0
    }

    /// Returns the approximate color temperature in Kelvin.
    #[must_use]
    pub fn to_kelvin(&self) -> u16 {
        #[allow(clippy::cast_possible_truncation)]
        let kelvin = (1_000_000 / u32::from(self.0)) as u16;
        kelvin
    }

    /// Creates a color temperature from Kelvin value.
    ///
    /// # Arguments
    ///
    /// * `kelvin` - The color temperature in Kelvin (2000-6500)
    ///
    /// # Errors
    ///
    /// Returns error if the resulting mired value is outside the valid range.
    pub fn from_kelvin(kelvin: u16) -> Result<Self, ValueError> {
        if kelvin == 0 {
            return Err(ValueError::OutOfRange {
                min: Self::MIN,
                max: Self::MAX,
                actual: 0,
            });
        }
        #[allow(clippy::cast_possible_truncation)]
        let mireds = (1_000_000 / u32::from(kelvin)) as u16;
        Self::new(mireds)
    }
}

impl Default for ColorTemperature {
    fn default() -> Self {
        Self::NEUTRAL
    }
}

impl fmt::Display for ColorTemperature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}K", self.to_kelvin())
    }
}

impl TryFrom<u16> for ColorTemperature {
    type Error = ValueError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// HSB color representation (Hue, Saturation, Brightness).
///
/// # Examples
///
/// ```
/// use tasmor_lib::types::HsbColor;
///
/// // Create a pure red color at full brightness
/// let red = HsbColor::new(0, 100, 100).unwrap();
/// assert_eq!(red.hue(), 0);
/// assert_eq!(red.saturation(), 100);
/// assert_eq!(red.brightness(), 100);
///
/// // Create a green color
/// let green = HsbColor::new(120, 100, 100).unwrap();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct HsbColor {
    hue: u16,
    saturation: u8,
    brightness: u8,
}

impl HsbColor {
    /// Maximum hue value (exclusive, wraps at 360).
    pub const MAX_HUE: u16 = 360;

    /// Maximum saturation value.
    pub const MAX_SATURATION: u8 = 100;

    /// Maximum brightness value.
    pub const MAX_BRIGHTNESS: u8 = 100;

    /// Creates a new HSB color.
    ///
    /// # Arguments
    ///
    /// * `hue` - Color hue (0-360 degrees, where 0/360 is red)
    /// * `saturation` - Color saturation (0-100%)
    /// * `brightness` - Color brightness (0-100%)
    ///
    /// # Errors
    ///
    /// Returns error if any value is outside its valid range.
    pub fn new(hue: u16, saturation: u8, brightness: u8) -> Result<Self, ValueError> {
        if hue > Self::MAX_HUE {
            return Err(ValueError::InvalidHue(hue));
        }
        if saturation > Self::MAX_SATURATION {
            return Err(ValueError::InvalidSaturation(saturation));
        }
        if brightness > Self::MAX_BRIGHTNESS {
            return Err(ValueError::InvalidBrightness(brightness));
        }
        Ok(Self {
            hue,
            saturation,
            brightness,
        })
    }

    /// Creates a pure red color at full brightness.
    #[must_use]
    pub const fn red() -> Self {
        Self {
            hue: 0,
            saturation: 100,
            brightness: 100,
        }
    }

    /// Creates a pure green color at full brightness.
    #[must_use]
    pub const fn green() -> Self {
        Self {
            hue: 120,
            saturation: 100,
            brightness: 100,
        }
    }

    /// Creates a pure blue color at full brightness.
    #[must_use]
    pub const fn blue() -> Self {
        Self {
            hue: 240,
            saturation: 100,
            brightness: 100,
        }
    }

    /// Creates a white color (no saturation).
    #[must_use]
    pub const fn white() -> Self {
        Self {
            hue: 0,
            saturation: 0,
            brightness: 100,
        }
    }

    /// Returns the hue value (0-360).
    #[must_use]
    pub const fn hue(&self) -> u16 {
        self.hue
    }

    /// Returns the saturation value (0-100).
    #[must_use]
    pub const fn saturation(&self) -> u8 {
        self.saturation
    }

    /// Returns the brightness value (0-100).
    #[must_use]
    pub const fn brightness(&self) -> u8 {
        self.brightness
    }

    /// Returns the color as a Tasmota command string.
    #[must_use]
    pub fn to_command_string(&self) -> String {
        format!("{},{},{}", self.hue, self.saturation, self.brightness)
    }

    /// Creates a new color with a different hue.
    ///
    /// # Errors
    ///
    /// Returns error if hue is greater than 360.
    pub fn with_hue(&self, hue: u16) -> Result<Self, ValueError> {
        Self::new(hue, self.saturation, self.brightness)
    }

    /// Creates a new color with a different saturation.
    ///
    /// # Errors
    ///
    /// Returns error if saturation is greater than 100.
    pub fn with_saturation(&self, saturation: u8) -> Result<Self, ValueError> {
        Self::new(self.hue, saturation, self.brightness)
    }

    /// Creates a new color with a different brightness.
    ///
    /// # Errors
    ///
    /// Returns error if brightness is greater than 100.
    pub fn with_brightness(&self, brightness: u8) -> Result<Self, ValueError> {
        Self::new(self.hue, self.saturation, brightness)
    }

    /// Converts this HSB color to RGB format.
    ///
    /// Note: Due to rounding in the conversion, converting HSB to RGB and back
    /// may not produce the exact same HSB values.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::types::HsbColor;
    ///
    /// let red = HsbColor::red();
    /// let rgb = red.to_rgb();
    /// assert_eq!(rgb.red(), 255);
    /// assert_eq!(rgb.green(), 0);
    /// assert_eq!(rgb.blue(), 0);
    /// ```
    #[must_use]
    pub fn to_rgb(&self) -> super::RgbColor {
        super::RgbColor::from_hsb(self)
    }

    /// Creates an HSB color from an RGB color.
    ///
    /// Note: Due to rounding in the conversion, converting RGB to HSB and back
    /// may not produce the exact same RGB values.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::types::{HsbColor, RgbColor};
    ///
    /// let rgb = RgbColor::new(255, 0, 0);
    /// let hsb = HsbColor::from_rgb(&rgb);
    /// assert_eq!(hsb.hue(), 0);
    /// assert_eq!(hsb.saturation(), 100);
    /// assert_eq!(hsb.brightness(), 100);
    /// ```
    #[must_use]
    pub fn from_rgb(rgb: &super::RgbColor) -> Self {
        rgb.to_hsb()
    }
}

impl Default for HsbColor {
    fn default() -> Self {
        Self::white()
    }
}

impl fmt::Display for HsbColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HSB({}, {}%, {}%)",
            self.hue, self.saturation, self.brightness
        )
    }
}

impl TryFrom<(u16, u8, u8)> for HsbColor {
    type Error = ValueError;

    fn try_from((hue, saturation, brightness): (u16, u8, u8)) -> Result<Self, Self::Error> {
        Self::new(hue, saturation, brightness)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_temp_valid() {
        for v in 153..=500 {
            let ct = ColorTemperature::new(v).unwrap();
            assert_eq!(ct.value(), v);
        }
    }

    #[test]
    fn color_temp_invalid() {
        assert!(ColorTemperature::new(152).is_err());
        assert!(ColorTemperature::new(501).is_err());
    }

    #[test]
    fn color_temp_clamped() {
        assert_eq!(ColorTemperature::clamped(100).value(), 153);
        assert_eq!(ColorTemperature::clamped(600).value(), 500);
        assert_eq!(ColorTemperature::clamped(300).value(), 300);
    }

    #[test]
    fn color_temp_kelvin_conversion() {
        // 153 mireds ≈ 6535K (cool)
        let cool = ColorTemperature::COOL;
        assert!(cool.to_kelvin() > 6000);

        // 500 mireds ≈ 2000K (warm)
        let warm = ColorTemperature::CANDLE;
        assert_eq!(warm.to_kelvin(), 2000);
    }

    #[test]
    fn color_temp_from_kelvin() {
        let ct = ColorTemperature::from_kelvin(4000).unwrap();
        assert_eq!(ct.value(), 250);
    }

    #[test]
    fn hsb_color_valid() {
        let color = HsbColor::new(180, 50, 75).unwrap();
        assert_eq!(color.hue(), 180);
        assert_eq!(color.saturation(), 50);
        assert_eq!(color.brightness(), 75);
    }

    #[test]
    fn hsb_color_invalid_hue() {
        let result = HsbColor::new(361, 50, 50);
        assert!(matches!(result, Err(ValueError::InvalidHue(361))));
    }

    #[test]
    fn hsb_color_invalid_saturation() {
        let result = HsbColor::new(180, 101, 50);
        assert!(matches!(result, Err(ValueError::InvalidSaturation(101))));
    }

    #[test]
    fn hsb_color_invalid_brightness() {
        let result = HsbColor::new(180, 50, 101);
        assert!(matches!(result, Err(ValueError::InvalidBrightness(101))));
    }

    #[test]
    fn hsb_color_presets() {
        assert_eq!(HsbColor::red().hue(), 0);
        assert_eq!(HsbColor::green().hue(), 120);
        assert_eq!(HsbColor::blue().hue(), 240);
        assert_eq!(HsbColor::white().saturation(), 0);
    }

    #[test]
    fn hsb_color_command_string() {
        let color = HsbColor::new(120, 100, 75).unwrap();
        assert_eq!(color.to_command_string(), "120,100,75");
    }

    #[test]
    fn hsb_color_with_methods() {
        let color = HsbColor::red();
        let green = color.with_hue(120).unwrap();
        assert_eq!(green.hue(), 120);
        assert_eq!(green.saturation(), 100);
    }

    #[test]
    fn color_temperature_try_from() {
        let ct: ColorTemperature = 250u16.try_into().unwrap();
        assert_eq!(ct.value(), 250);

        let result: Result<ColorTemperature, _> = 100u16.try_into();
        assert!(result.is_err());

        let result: Result<ColorTemperature, _> = 600u16.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn hsb_color_try_from() {
        let color: HsbColor = (180u16, 50u8, 75u8).try_into().unwrap();
        assert_eq!(color.hue(), 180);
        assert_eq!(color.saturation(), 50);
        assert_eq!(color.brightness(), 75);

        let result: Result<HsbColor, _> = (361u16, 50u8, 50u8).try_into();
        assert!(result.is_err());

        let result: Result<HsbColor, _> = (180u16, 101u8, 50u8).try_into();
        assert!(result.is_err());
    }
}
