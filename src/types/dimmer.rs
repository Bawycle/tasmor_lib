// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Dimmer type for brightness control.
//!
//! This module provides a type-safe representation of dimmer values,
//! ensuring values are always within the valid range of 0-100%.

use std::fmt;

use crate::error::ValueError;

/// Brightness level as a percentage (0-100).
///
/// Tasmota uses 0-100 for dimmer values, where 0 is off and 100 is full
/// brightness.
///
/// # Examples
///
/// ```
/// use tasmor_lib::types::Dimmer;
///
/// // Create a dimmer at 75%
/// let dim = Dimmer::new(75).unwrap();
/// assert_eq!(dim.value(), 75);
///
/// // Use predefined values
/// let off = Dimmer::MIN;
/// let full = Dimmer::MAX;
/// assert_eq!(off.value(), 0);
/// assert_eq!(full.value(), 100);
///
/// // Invalid values return error
/// assert!(Dimmer::new(101).is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Dimmer(u8);

impl Dimmer {
    /// Minimum dimmer value (0%).
    pub const MIN: Self = Self(0);

    /// Maximum dimmer value (100%).
    pub const MAX: Self = Self(100);

    /// Creates a new dimmer value.
    ///
    /// # Arguments
    ///
    /// * `value` - The brightness percentage (0-100)
    ///
    /// # Errors
    ///
    /// Returns `ValueError::OutOfRange` if value exceeds 100.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::types::Dimmer;
    ///
    /// let dim = Dimmer::new(50).unwrap();
    /// assert_eq!(dim.value(), 50);
    /// ```
    pub fn new(value: u8) -> Result<Self, ValueError> {
        if value > 100 {
            return Err(ValueError::OutOfRange {
                min: 0,
                max: 100,
                actual: u16::from(value),
            });
        }
        Ok(Self(value))
    }

    /// Creates a dimmer value, clamping to the valid range.
    ///
    /// Values above 100 are clamped to 100.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::types::Dimmer;
    ///
    /// let dim = Dimmer::clamped(150);
    /// assert_eq!(dim.value(), 100);
    /// ```
    #[must_use]
    pub const fn clamped(value: u8) -> Self {
        if value > 100 { Self(100) } else { Self(value) }
    }

    /// Returns the brightness percentage value.
    #[must_use]
    pub const fn value(&self) -> u8 {
        self.0
    }

    /// Returns the value as a float between 0.0 and 1.0.
    #[must_use]
    pub fn as_fraction(&self) -> f32 {
        f32::from(self.0) / 100.0
    }

    /// Creates a dimmer from a fraction between 0.0 and 1.0.
    ///
    /// # Errors
    ///
    /// Returns `ValueError::OutOfRange` if the fraction is outside [0.0, 1.0].
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::types::Dimmer;
    ///
    /// let dim = Dimmer::from_fraction(0.5).unwrap();
    /// assert_eq!(dim.value(), 50);
    /// ```
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn from_fraction(fraction: f32) -> Result<Self, ValueError> {
        if !(0.0..=1.0).contains(&fraction) {
            return Err(ValueError::OutOfRange {
                min: 0,
                max: 100,
                // Safe: fraction out of range, so actual value is informational only
                actual: (fraction * 100.0) as u16,
            });
        }
        Ok(Self((fraction * 100.0).round() as u8))
    }
}

impl fmt::Display for Dimmer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}%", self.0)
    }
}

impl TryFrom<u8> for Dimmer {
    type Error = ValueError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimmer_valid_values() {
        for v in 0..=100 {
            let dim = Dimmer::new(v).unwrap();
            assert_eq!(dim.value(), v);
        }
    }

    #[test]
    fn dimmer_invalid_value() {
        let result = Dimmer::new(101);
        assert!(result.is_err());
    }

    #[test]
    fn dimmer_clamped() {
        assert_eq!(Dimmer::clamped(50).value(), 50);
        assert_eq!(Dimmer::clamped(150).value(), 100);
        assert_eq!(Dimmer::clamped(255).value(), 100);
    }

    #[test]
    fn dimmer_as_fraction() {
        assert!((Dimmer::MIN.as_fraction() - 0.0).abs() < f32::EPSILON);
        assert!((Dimmer::MAX.as_fraction() - 1.0).abs() < f32::EPSILON);
        assert!((Dimmer::new(50).unwrap().as_fraction() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn dimmer_from_fraction() {
        assert_eq!(Dimmer::from_fraction(0.0).unwrap().value(), 0);
        assert_eq!(Dimmer::from_fraction(0.5).unwrap().value(), 50);
        assert_eq!(Dimmer::from_fraction(1.0).unwrap().value(), 100);
    }

    #[test]
    fn dimmer_from_fraction_invalid() {
        assert!(Dimmer::from_fraction(-0.1).is_err());
        assert!(Dimmer::from_fraction(1.1).is_err());
    }

    #[test]
    fn dimmer_display() {
        assert_eq!(Dimmer::new(75).unwrap().to_string(), "75%");
    }

    #[test]
    fn dimmer_ordering() {
        assert!(Dimmer::MIN < Dimmer::MAX);
        assert!(Dimmer::new(50).unwrap() < Dimmer::new(75).unwrap());
    }
}
