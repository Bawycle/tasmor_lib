// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Fade speed type for light transitions.
//!
//! This module provides a type-safe representation of fade speed values
//! for smooth light transitions in Tasmota devices.
//!
//! # Device Methods
//!
//! Use [`FadeSpeed`] with these [`Device`](crate::Device) methods:
//! - [`set_fade_speed()`](crate::Device::set_fade_speed) - Set transition speed
//! - [`get_fade_speed()`](crate::Device::get_fade_speed) - Query current speed
//! - [`enable_fade()`](crate::Device::enable_fade) / [`disable_fade()`](crate::Device::disable_fade) - Toggle fade transitions

use std::fmt;

use crate::error::ValueError;

/// Fade speed for light transitions (1-40).
///
/// Lower values mean faster transitions, higher values mean slower transitions.
/// - 1 = Fastest (nearly instant)
/// - 40 = Slowest
///
/// # Examples
///
/// ```
/// use tasmor_lib::types::FadeSpeed;
///
/// // Create a medium speed
/// let speed = FadeSpeed::new(20).unwrap();
/// assert_eq!(speed.value(), 20);
///
/// // Use predefined values
/// let fast = FadeSpeed::FAST;
/// let slow = FadeSpeed::SLOW;
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct FadeSpeed(u8);

impl FadeSpeed {
    /// Minimum speed value (fastest transition).
    pub const MIN: u8 = 1;

    /// Maximum speed value (slowest transition).
    pub const MAX: u8 = 40;

    /// Fast transition speed.
    pub const FAST: Self = Self(1);

    /// Medium transition speed.
    pub const MEDIUM: Self = Self(20);

    /// Slow transition speed.
    pub const SLOW: Self = Self(40);

    /// Creates a new fade speed value.
    ///
    /// # Arguments
    ///
    /// * `value` - The speed value (1-40)
    ///
    /// # Errors
    ///
    /// Returns `ValueError::OutOfRange` if value is outside [1, 40].
    pub fn new(value: u8) -> Result<Self, ValueError> {
        if !(Self::MIN..=Self::MAX).contains(&value) {
            return Err(ValueError::OutOfRange {
                min: u16::from(Self::MIN),
                max: u16::from(Self::MAX),
                actual: u16::from(value),
            });
        }
        Ok(Self(value))
    }

    /// Creates a fade speed, clamping to the valid range.
    #[must_use]
    pub const fn clamped(value: u8) -> Self {
        if value < Self::MIN {
            Self(Self::MIN)
        } else if value > Self::MAX {
            Self(Self::MAX)
        } else {
            Self(value)
        }
    }

    /// Returns the speed value.
    #[must_use]
    pub const fn value(&self) -> u8 {
        self.0
    }

    /// Returns whether this is a fast speed (1-10).
    #[must_use]
    pub const fn is_fast(&self) -> bool {
        self.0 <= 10
    }

    /// Returns whether this is a slow speed (31-40).
    #[must_use]
    pub const fn is_slow(&self) -> bool {
        self.0 >= 31
    }
}

impl Default for FadeSpeed {
    fn default() -> Self {
        Self::MEDIUM
    }
}

impl fmt::Display for FadeSpeed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<u8> for FadeSpeed {
    type Error = ValueError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fade_speed_valid() {
        for v in 1..=40 {
            let speed = FadeSpeed::new(v).unwrap();
            assert_eq!(speed.value(), v);
        }
    }

    #[test]
    fn fade_speed_invalid() {
        assert!(FadeSpeed::new(0).is_err());
        assert!(FadeSpeed::new(41).is_err());
    }

    #[test]
    fn fade_speed_clamped() {
        assert_eq!(FadeSpeed::clamped(0).value(), 1);
        assert_eq!(FadeSpeed::clamped(50).value(), 40);
        assert_eq!(FadeSpeed::clamped(25).value(), 25);
    }

    #[test]
    fn fade_speed_presets() {
        assert_eq!(FadeSpeed::FAST.value(), 1);
        assert_eq!(FadeSpeed::MEDIUM.value(), 20);
        assert_eq!(FadeSpeed::SLOW.value(), 40);
    }

    #[test]
    fn fade_speed_classification() {
        assert!(FadeSpeed::FAST.is_fast());
        assert!(!FadeSpeed::FAST.is_slow());
        assert!(FadeSpeed::SLOW.is_slow());
        assert!(!FadeSpeed::SLOW.is_fast());
        assert!(!FadeSpeed::MEDIUM.is_fast());
        assert!(!FadeSpeed::MEDIUM.is_slow());
    }

    #[test]
    fn fade_speed_ordering() {
        // Note: Lower value = faster, but in Ord terms, lower < higher
        assert!(FadeSpeed::FAST < FadeSpeed::SLOW);
    }
}
