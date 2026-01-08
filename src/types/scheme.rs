// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Scheme type for light effects.
//!
//! This module provides a type-safe representation of Tasmota light schemes,
//! ensuring values are always within the valid range of 0-4.
//!
//! # Device Methods
//!
//! Use [`Scheme`] with these [`Device`](crate::Device) methods:
//! - [`set_scheme()`](crate::Device::set_scheme) - Set the light effect/scheme
//! - [`get_scheme()`](crate::Device::get_scheme) - Query current scheme

use std::fmt;

use crate::error::ValueError;

/// Light scheme/effect type (0-4 for standard lights).
///
/// Tasmota supports several built-in light effects:
///
/// | Value | Name | Description |
/// |-------|------|-------------|
/// | 0 | Single | Fixed color (default) |
/// | 1 | Wakeup | Gradual brightness increase (uses `WakeupDuration`) |
/// | 2 | Cycle Up | Color cycling with increasing brightness |
/// | 3 | Cycle Down | Color cycling with decreasing brightness |
/// | 4 | Random | Random color changes |
///
/// # Examples
///
/// ```
/// use tasmor_lib::types::Scheme;
///
/// // Use predefined constants
/// let wakeup = Scheme::WAKEUP;
/// assert_eq!(wakeup.value(), 1);
///
/// // Create from value
/// let scheme = Scheme::new(2).unwrap();
/// assert_eq!(scheme.value(), 2);
///
/// // Invalid values return error
/// assert!(Scheme::new(5).is_err());
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct Scheme(u8);

impl Scheme {
    /// Single color (default).
    pub const SINGLE: Self = Self(0);

    /// Wakeup effect - gradual brightness increase.
    ///
    /// This scheme gradually increases brightness from 0 to the current dimmer
    /// level over the duration set by `WakeupDuration`.
    pub const WAKEUP: Self = Self(1);

    /// Cycle up effect - color cycling with increasing brightness.
    pub const CYCLE_UP: Self = Self(2);

    /// Cycle down effect - color cycling with decreasing brightness.
    pub const CYCLE_DOWN: Self = Self(3);

    /// Random effect - random color changes.
    pub const RANDOM: Self = Self(4);

    /// Maximum valid scheme value.
    const MAX: u8 = 4;

    /// Creates a new scheme value.
    ///
    /// # Arguments
    ///
    /// * `value` - The scheme number (0-4)
    ///
    /// # Errors
    ///
    /// Returns `ValueError::OutOfRange` if value exceeds 4.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::types::Scheme;
    ///
    /// let scheme = Scheme::new(1).unwrap();
    /// assert_eq!(scheme.value(), 1);
    /// ```
    pub fn new(value: u8) -> Result<Self, ValueError> {
        if value > Self::MAX {
            return Err(ValueError::OutOfRange {
                min: 0,
                max: u16::from(Self::MAX),
                actual: u16::from(value),
            });
        }
        Ok(Self(value))
    }

    /// Returns the scheme value.
    #[must_use]
    pub const fn value(&self) -> u8 {
        self.0
    }

    /// Returns the scheme name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self.0 {
            0 => "Single",
            1 => "Wakeup",
            2 => "Cycle Up",
            3 => "Cycle Down",
            4 => "Random",
            _ => "Unknown",
        }
    }
}

impl Default for Scheme {
    fn default() -> Self {
        Self::SINGLE
    }
}

impl fmt::Display for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name(), self.0)
    }
}

impl TryFrom<u8> for Scheme {
    type Error = ValueError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheme_valid_values() {
        for v in 0..=4 {
            let scheme = Scheme::new(v).unwrap();
            assert_eq!(scheme.value(), v);
        }
    }

    #[test]
    fn scheme_invalid_value() {
        let result = Scheme::new(5);
        assert!(result.is_err());
    }

    #[test]
    fn scheme_constants() {
        assert_eq!(Scheme::SINGLE.value(), 0);
        assert_eq!(Scheme::WAKEUP.value(), 1);
        assert_eq!(Scheme::CYCLE_UP.value(), 2);
        assert_eq!(Scheme::CYCLE_DOWN.value(), 3);
        assert_eq!(Scheme::RANDOM.value(), 4);
    }

    #[test]
    fn scheme_names() {
        assert_eq!(Scheme::SINGLE.name(), "Single");
        assert_eq!(Scheme::WAKEUP.name(), "Wakeup");
        assert_eq!(Scheme::CYCLE_UP.name(), "Cycle Up");
        assert_eq!(Scheme::CYCLE_DOWN.name(), "Cycle Down");
        assert_eq!(Scheme::RANDOM.name(), "Random");
    }

    #[test]
    fn scheme_display() {
        assert_eq!(Scheme::SINGLE.to_string(), "Single (0)");
        assert_eq!(Scheme::WAKEUP.to_string(), "Wakeup (1)");
    }

    #[test]
    fn scheme_default() {
        assert_eq!(Scheme::default(), Scheme::SINGLE);
    }

    #[test]
    fn scheme_try_from() {
        let scheme: Scheme = 2u8.try_into().unwrap();
        assert_eq!(scheme.value(), 2);

        let result: Result<Scheme, _> = 5u8.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn scheme_ordering() {
        assert!(Scheme::SINGLE < Scheme::WAKEUP);
        assert!(Scheme::WAKEUP < Scheme::RANDOM);
    }
}
