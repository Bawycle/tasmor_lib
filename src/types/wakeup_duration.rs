// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Wakeup duration type for gradual brightness transitions.
//!
//! This module provides a type-safe representation of the wakeup duration
//! used with the Wakeup scheme (Scheme 1).
//!
//! # Device Methods
//!
//! Use [`WakeupDuration`] with these [`Device`](crate::Device) methods:
//! - [`set_wakeup_duration()`](crate::Device::set_wakeup_duration) - Set wakeup timing
//! - [`get_wakeup_duration()`](crate::Device::get_wakeup_duration) - Query current duration

use std::fmt;

use crate::error::ValueError;

/// Wake-up duration in seconds (1-3000).
///
/// This value controls how long the Wakeup scheme (Scheme 1) takes to gradually
/// increase brightness from 0 to the current dimmer level.
///
/// # Examples
///
/// ```
/// use tasmor_lib::types::WakeupDuration;
///
/// // Create from seconds
/// let duration = WakeupDuration::new(300).unwrap();  // 5 minutes
/// assert_eq!(duration.seconds(), 300);
///
/// // Create from minutes
/// let duration = WakeupDuration::from_minutes(5).unwrap();
/// assert_eq!(duration.seconds(), 300);
///
/// // Invalid values return error
/// assert!(WakeupDuration::new(0).is_err());
/// assert!(WakeupDuration::new(3001).is_err());
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct WakeupDuration(u16);

impl WakeupDuration {
    /// Minimum wakeup duration (1 second).
    pub const MIN: u16 = 1;

    /// Maximum wakeup duration (3000 seconds = 50 minutes).
    pub const MAX: u16 = 3000;

    /// Creates a new wakeup duration.
    ///
    /// # Arguments
    ///
    /// * `seconds` - The duration in seconds (1-3000)
    ///
    /// # Errors
    ///
    /// Returns `ValueError::OutOfRange` if value is outside [1, 3000].
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::types::WakeupDuration;
    ///
    /// let duration = WakeupDuration::new(60).unwrap();
    /// assert_eq!(duration.seconds(), 60);
    /// ```
    pub fn new(seconds: u16) -> Result<Self, ValueError> {
        if !(Self::MIN..=Self::MAX).contains(&seconds) {
            return Err(ValueError::OutOfRange {
                min: Self::MIN,
                max: Self::MAX,
                actual: seconds,
            });
        }
        Ok(Self(seconds))
    }

    /// Creates a wakeup duration from minutes.
    ///
    /// # Arguments
    ///
    /// * `minutes` - The duration in minutes (1-50)
    ///
    /// # Errors
    ///
    /// Returns `ValueError::OutOfRange` if the resulting seconds value is
    /// outside [1, 3000].
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::types::WakeupDuration;
    ///
    /// let duration = WakeupDuration::from_minutes(5).unwrap();
    /// assert_eq!(duration.seconds(), 300);
    /// assert_eq!(duration.minutes(), 5);
    /// ```
    pub fn from_minutes(minutes: u16) -> Result<Self, ValueError> {
        let seconds = minutes.saturating_mul(60);
        Self::new(seconds)
    }

    /// Creates a wakeup duration, clamping to the valid range.
    ///
    /// Values below 1 are clamped to 1, values above 3000 are clamped to 3000.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::types::WakeupDuration;
    ///
    /// let duration = WakeupDuration::clamped(0);
    /// assert_eq!(duration.seconds(), 1);
    ///
    /// let duration = WakeupDuration::clamped(5000);
    /// assert_eq!(duration.seconds(), 3000);
    /// ```
    #[must_use]
    pub const fn clamped(seconds: u16) -> Self {
        if seconds < Self::MIN {
            Self(Self::MIN)
        } else if seconds > Self::MAX {
            Self(Self::MAX)
        } else {
            Self(seconds)
        }
    }

    /// Returns the duration in seconds.
    #[must_use]
    pub const fn seconds(&self) -> u16 {
        self.0
    }

    /// Returns the duration in whole minutes (truncated).
    #[must_use]
    pub const fn minutes(&self) -> u16 {
        self.0 / 60
    }

    /// Returns the duration as a formatted string (e.g., "5m 30s").
    #[must_use]
    pub fn as_formatted(&self) -> String {
        let mins = self.0 / 60;
        let secs = self.0 % 60;
        if mins > 0 && secs > 0 {
            format!("{mins}m {secs}s")
        } else if mins > 0 {
            format!("{mins}m")
        } else {
            format!("{secs}s")
        }
    }
}

impl Default for WakeupDuration {
    fn default() -> Self {
        // Default to 60 seconds (1 minute)
        Self(60)
    }
}

impl fmt::Display for WakeupDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_formatted())
    }
}

impl TryFrom<u16> for WakeupDuration {
    type Error = ValueError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wakeup_duration_valid_values() {
        for v in [1, 60, 300, 1800, 3000] {
            let duration = WakeupDuration::new(v).unwrap();
            assert_eq!(duration.seconds(), v);
        }
    }

    #[test]
    fn wakeup_duration_invalid_values() {
        assert!(WakeupDuration::new(0).is_err());
        assert!(WakeupDuration::new(3001).is_err());
    }

    #[test]
    fn wakeup_duration_from_minutes() {
        let duration = WakeupDuration::from_minutes(5).unwrap();
        assert_eq!(duration.seconds(), 300);
        assert_eq!(duration.minutes(), 5);
    }

    #[test]
    fn wakeup_duration_from_minutes_invalid() {
        // 51 minutes = 3060 seconds > 3000
        assert!(WakeupDuration::from_minutes(51).is_err());
    }

    #[test]
    fn wakeup_duration_clamped() {
        assert_eq!(WakeupDuration::clamped(0).seconds(), 1);
        assert_eq!(WakeupDuration::clamped(60).seconds(), 60);
        assert_eq!(WakeupDuration::clamped(5000).seconds(), 3000);
    }

    #[test]
    fn wakeup_duration_minutes() {
        assert_eq!(WakeupDuration::new(30).unwrap().minutes(), 0);
        assert_eq!(WakeupDuration::new(60).unwrap().minutes(), 1);
        assert_eq!(WakeupDuration::new(90).unwrap().minutes(), 1);
        assert_eq!(WakeupDuration::new(120).unwrap().minutes(), 2);
    }

    #[test]
    fn wakeup_duration_as_formatted() {
        assert_eq!(WakeupDuration::new(30).unwrap().as_formatted(), "30s");
        assert_eq!(WakeupDuration::new(60).unwrap().as_formatted(), "1m");
        assert_eq!(WakeupDuration::new(90).unwrap().as_formatted(), "1m 30s");
        assert_eq!(WakeupDuration::new(300).unwrap().as_formatted(), "5m");
    }

    #[test]
    fn wakeup_duration_display() {
        assert_eq!(WakeupDuration::new(90).unwrap().to_string(), "1m 30s");
    }

    #[test]
    fn wakeup_duration_default() {
        assert_eq!(WakeupDuration::default().seconds(), 60);
    }

    #[test]
    fn wakeup_duration_try_from() {
        let duration: WakeupDuration = 120u16.try_into().unwrap();
        assert_eq!(duration.seconds(), 120);

        let result: Result<WakeupDuration, _> = 0u16.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn wakeup_duration_ordering() {
        assert!(WakeupDuration::new(60).unwrap() < WakeupDuration::new(120).unwrap());
    }
}
