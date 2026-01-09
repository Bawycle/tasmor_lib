// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Time-based types for Tasmota device control.
//!
//! This module provides type-safe representations of duration values used
//! in Tasmota lighting commands and uptime parsing.
//!
//! # Types
//!
//! - [`WakeupDuration`] - Duration for the wakeup scheme (1-3000 seconds)
//! - [`FadeDuration`] - Duration for fade transitions (0.5-20 seconds)
//!
//! # Functions
//!
//! - [`parse_uptime`] - Parse Tasmota uptime strings (e.g., `"1T23:46:58"`)
//!
//! # Device Methods
//!
//! Use [`WakeupDuration`] with:
//! - [`set_wakeup_duration()`](crate::Device::set_wakeup_duration) - Set wakeup timing
//! - [`get_wakeup_duration()`](crate::Device::get_wakeup_duration) - Query current duration
//!
//! Use [`FadeDuration`] with:
//! - [`set_fade_duration()`](crate::Device::set_fade_duration) - Set transition duration
//! - [`get_fade_duration()`](crate::Device::get_fade_duration) - Query current duration
//! - [`enable_fade()`](crate::Device::enable_fade) / [`disable_fade()`](crate::Device::disable_fade) - Toggle fade transitions

use std::fmt;
use std::time::Duration;

use crate::error::{ParseError, ValueError};

// =============================================================================
// WakeupDuration
// =============================================================================

/// Minimum wakeup duration (1 second).
const WAKEUP_MIN_SECS: u16 = 1;

/// Maximum wakeup duration (3000 seconds = 50 minutes).
const WAKEUP_MAX_SECS: u16 = 3000;

/// Wake-up duration for gradual brightness transitions.
///
/// This value controls how long the Wakeup scheme (Scheme 1) takes to gradually
/// increase brightness from 0 to the current dimmer level.
///
/// Valid range: 1 second to 3000 seconds (50 minutes).
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use tasmor_lib::types::WakeupDuration;
///
/// // Create from Duration
/// let duration = WakeupDuration::new(Duration::from_secs(300)).unwrap();  // 5 minutes
/// assert_eq!(duration.as_duration(), Duration::from_secs(300));
///
/// // Durations are rounded to the nearest second
/// let duration = WakeupDuration::new(Duration::from_millis(2700)).unwrap();
/// assert_eq!(duration.as_duration(), Duration::from_secs(3));  // Rounded
///
/// // Invalid values return error
/// assert!(WakeupDuration::new(Duration::ZERO).is_err());
/// assert!(WakeupDuration::new(Duration::from_secs(3001)).is_err());
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct WakeupDuration(u16);

impl WakeupDuration {
    /// Minimum wakeup duration.
    pub const MIN: Duration = Duration::from_secs(WAKEUP_MIN_SECS as u64);

    /// Maximum wakeup duration.
    pub const MAX: Duration = Duration::from_secs(WAKEUP_MAX_SECS as u64);

    /// Creates a new wakeup duration.
    ///
    /// The duration is rounded to the nearest second.
    ///
    /// # Arguments
    ///
    /// * `duration` - The duration (1 second to 3000 seconds)
    ///
    /// # Errors
    ///
    /// Returns `ValueError::OutOfRange` if the rounded duration is outside [1, 3000] seconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use tasmor_lib::types::WakeupDuration;
    ///
    /// let duration = WakeupDuration::new(Duration::from_secs(60)).unwrap();
    /// assert_eq!(duration.as_duration(), Duration::from_secs(60));
    ///
    /// // Sub-second precision is rounded
    /// let duration = WakeupDuration::new(Duration::from_millis(1600)).unwrap();
    /// assert_eq!(duration.as_duration(), Duration::from_secs(2));
    /// ```
    pub fn new(duration: Duration) -> Result<Self, ValueError> {
        // Truncation is safe: valid durations are 1s-3000s
        // Sign loss is safe: Duration is always non-negative
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let secs = duration.as_secs_f64().round() as u64;

        if secs < u64::from(WAKEUP_MIN_SECS) || secs > u64::from(WAKEUP_MAX_SECS) {
            return Err(ValueError::OutOfRange {
                min: WAKEUP_MIN_SECS,
                max: WAKEUP_MAX_SECS,
                #[allow(clippy::cast_possible_truncation)]
                actual: secs.min(u64::from(u16::MAX)) as u16,
            });
        }

        #[allow(clippy::cast_possible_truncation)]
        Ok(Self(secs as u16))
    }

    /// Creates a wakeup duration from a raw seconds value.
    ///
    /// This is used internally to parse responses from Tasmota devices.
    ///
    /// # Errors
    ///
    /// Returns `ValueError::OutOfRange` if value is outside [1, 3000].
    pub(crate) fn from_raw(seconds: u16) -> Result<Self, ValueError> {
        if !(WAKEUP_MIN_SECS..=WAKEUP_MAX_SECS).contains(&seconds) {
            return Err(ValueError::OutOfRange {
                min: WAKEUP_MIN_SECS,
                max: WAKEUP_MAX_SECS,
                actual: seconds,
            });
        }
        Ok(Self(seconds))
    }

    /// Returns the duration.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use tasmor_lib::types::WakeupDuration;
    ///
    /// let wakeup = WakeupDuration::new(Duration::from_secs(300)).unwrap();
    /// assert_eq!(wakeup.as_duration(), Duration::from_secs(300));
    /// ```
    #[must_use]
    pub const fn as_duration(&self) -> Duration {
        Duration::from_secs(self.0 as u64)
    }

    /// Returns the duration in seconds.
    ///
    /// This is the raw value sent to Tasmota.
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

// =============================================================================
// FadeDuration
// =============================================================================

/// Minimum fade value (fastest transition, 0.5 seconds).
const FADE_MIN_VALUE: u8 = 1;

/// Maximum fade value (slowest transition, 20 seconds).
const FADE_MAX_VALUE: u8 = 40;

/// Time per fade unit in milliseconds (500ms = 0.5 seconds).
const FADE_MS_PER_UNIT: u64 = 500;

/// Fade duration for light transitions.
///
/// This value controls how long it takes to fade from 0% to 100% brightness
/// (or the reverse). The duration is stored internally as units of 0.5 seconds.
///
/// Valid range: 0.5 seconds to 20 seconds.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use tasmor_lib::types::FadeDuration;
///
/// // Create from Duration
/// let fade = FadeDuration::new(Duration::from_secs(2)).unwrap();
/// assert_eq!(fade.as_duration(), Duration::from_secs(2));
///
/// // Durations are rounded to the nearest 0.5 second
/// let fade = FadeDuration::new(Duration::from_millis(1200)).unwrap();
/// assert_eq!(fade.as_duration(), Duration::from_millis(1000));  // Rounded to 1.0s
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct FadeDuration(u8);

impl FadeDuration {
    /// Minimum fade duration (fastest transition).
    pub const MIN: Duration = Duration::from_millis(FADE_MS_PER_UNIT);

    /// Maximum fade duration (slowest transition).
    pub const MAX: Duration = Duration::from_millis(FADE_MS_PER_UNIT * FADE_MAX_VALUE as u64);

    /// Creates a new fade duration.
    ///
    /// The duration is rounded to the nearest 0.5 second increment.
    ///
    /// # Arguments
    ///
    /// * `duration` - The fade duration (0.5 seconds to 20 seconds)
    ///
    /// # Errors
    ///
    /// Returns `ValueError::OutOfRange` if the rounded duration is outside
    /// [0.5, 20] seconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use tasmor_lib::types::FadeDuration;
    ///
    /// // 2 seconds = internal value 4
    /// let fade = FadeDuration::new(Duration::from_secs(2)).unwrap();
    /// assert_eq!(fade.value(), 4);
    /// assert_eq!(fade.as_duration(), Duration::from_secs(2));
    ///
    /// // Rounded to nearest 0.5s
    /// let fade = FadeDuration::new(Duration::from_millis(1700)).unwrap();
    /// assert_eq!(fade.as_duration(), Duration::from_millis(1500));  // 1.5s
    /// ```
    pub fn new(duration: Duration) -> Result<Self, ValueError> {
        // Convert to 0.5s units and round
        // Truncation is safe: valid durations are 0.5s-20s, max 40 units
        // Sign loss is safe: Duration is always non-negative
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let units = (duration.as_secs_f64() * 2.0).round() as u64;

        if units < u64::from(FADE_MIN_VALUE) || units > u64::from(FADE_MAX_VALUE) {
            return Err(ValueError::OutOfRange {
                min: u16::from(FADE_MIN_VALUE),
                max: u16::from(FADE_MAX_VALUE),
                #[allow(clippy::cast_possible_truncation)]
                actual: units.min(u64::from(u16::MAX)) as u16,
            });
        }

        #[allow(clippy::cast_possible_truncation)]
        Ok(Self(units as u8))
    }

    /// Creates a fade duration from a raw Tasmota speed value.
    ///
    /// This is used internally to parse responses from Tasmota devices.
    ///
    /// # Errors
    ///
    /// Returns `ValueError::OutOfRange` if value is outside [1, 40].
    pub(crate) fn from_raw(value: u8) -> Result<Self, ValueError> {
        if !(FADE_MIN_VALUE..=FADE_MAX_VALUE).contains(&value) {
            return Err(ValueError::OutOfRange {
                min: u16::from(FADE_MIN_VALUE),
                max: u16::from(FADE_MAX_VALUE),
                actual: u16::from(value),
            });
        }
        Ok(Self(value))
    }

    /// Returns the fade duration.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use tasmor_lib::types::FadeDuration;
    ///
    /// let fade = FadeDuration::new(Duration::from_secs(5)).unwrap();
    /// assert_eq!(fade.as_duration(), Duration::from_secs(5));
    ///
    /// // 0.5 seconds (minimum)
    /// let fast = FadeDuration::new(Duration::from_millis(500)).unwrap();
    /// assert_eq!(fast.as_duration(), Duration::from_millis(500));
    /// ```
    #[must_use]
    pub const fn as_duration(&self) -> Duration {
        Duration::from_millis(self.0 as u64 * FADE_MS_PER_UNIT)
    }

    /// Returns the raw value (1-40).
    ///
    /// This is the value sent to Tasmota. Each unit represents 0.5 seconds.
    #[must_use]
    pub const fn value(&self) -> u8 {
        self.0
    }

    /// Returns whether this is a fast transition (0.5s - 5s).
    #[must_use]
    pub const fn is_fast(&self) -> bool {
        self.0 <= 10
    }

    /// Returns whether this is a slow transition (15.5s - 20s).
    #[must_use]
    pub const fn is_slow(&self) -> bool {
        self.0 >= 31
    }
}

impl Default for FadeDuration {
    fn default() -> Self {
        // Default to 10 seconds (raw value 20)
        Self(20)
    }
}

impl fmt::Display for FadeDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let duration = self.as_duration();
        let secs = duration.as_secs_f64();
        // Check if duration is a whole number of seconds
        // Epsilon comparison is appropriate here since we're checking for exact .0 or .5 values
        #[allow(clippy::float_cmp)]
        let is_whole = secs == secs.trunc();
        if is_whole {
            // Truncation and sign loss are safe: secs is in range 0.5-20.0
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let whole_secs = secs as u64;
            write!(f, "{whole_secs}s")
        } else {
            write!(f, "{secs:.1}s")
        }
    }
}

// =============================================================================
// Uptime Parsing
// =============================================================================

/// Parses a Tasmota uptime string into a [`Duration`].
///
/// Tasmota reports device uptime in the format `"XdTHH:MM:SS"` where:
/// - `X` is the number of days (the 'd' suffix is sometimes omitted)
/// - `T` is the separator between days and time
/// - `HH:MM:SS` is hours, minutes, and seconds
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use tasmor_lib::types::parse_uptime;
///
/// // 1 day, 23 hours, 46 minutes, 58 seconds
/// let duration = parse_uptime("1T23:46:58").unwrap();
/// assert_eq!(duration, Duration::from_secs(172018));
///
/// // Zero days, 12 hours, 30 minutes, 45 seconds
/// let duration = parse_uptime("0T12:30:45").unwrap();
/// assert_eq!(duration, Duration::from_secs(45045));
///
/// // Multiple days
/// let duration = parse_uptime("17T04:02:54").unwrap();
/// assert_eq!(duration, Duration::from_secs(1_483_374));
/// ```
///
/// # Errors
///
/// Returns [`ParseError::InvalidValue`] if:
/// - The format doesn't contain 'T' separator
/// - Days, hours, minutes, or seconds are not valid numbers
/// - Hours > 23, minutes > 59, or seconds > 59
pub fn parse_uptime(s: &str) -> Result<Duration, ParseError> {
    let s = s.trim();

    // Find 'T' separator
    let t_idx = s.find('T').ok_or_else(|| ParseError::InvalidValue {
        field: "uptime".to_string(),
        message: format!("missing 'T' separator in uptime: {s}"),
    })?;

    // Parse days (before 'T')
    let days_str = &s[..t_idx];
    // Remove trailing 'd' if present (some firmware versions include it)
    let days_str = days_str.trim_end_matches('d');
    let days: u64 = days_str.parse().map_err(|_| ParseError::InvalidValue {
        field: "uptime".to_string(),
        message: format!("invalid days value: {days_str}"),
    })?;

    // Parse time (after 'T')
    let time_str = &s[t_idx + 1..];
    let parts: Vec<&str> = time_str.split(':').collect();

    if parts.len() != 3 {
        return Err(ParseError::InvalidValue {
            field: "uptime".to_string(),
            message: format!("expected HH:MM:SS format, got: {time_str}"),
        });
    }

    let hours: u64 = parts[0].parse().map_err(|_| ParseError::InvalidValue {
        field: "uptime".to_string(),
        message: format!("invalid hours: {}", parts[0]),
    })?;

    let minutes: u64 = parts[1].parse().map_err(|_| ParseError::InvalidValue {
        field: "uptime".to_string(),
        message: format!("invalid minutes: {}", parts[1]),
    })?;

    let seconds: u64 = parts[2].parse().map_err(|_| ParseError::InvalidValue {
        field: "uptime".to_string(),
        message: format!("invalid seconds: {}", parts[2]),
    })?;

    // Validate ranges
    if hours > 23 {
        return Err(ParseError::InvalidValue {
            field: "uptime".to_string(),
            message: format!("hours must be 0-23, got: {hours}"),
        });
    }
    if minutes > 59 {
        return Err(ParseError::InvalidValue {
            field: "uptime".to_string(),
            message: format!("minutes must be 0-59, got: {minutes}"),
        });
    }
    if seconds > 59 {
        return Err(ParseError::InvalidValue {
            field: "uptime".to_string(),
            message: format!("seconds must be 0-59, got: {seconds}"),
        });
    }

    let total_seconds = days * 86400 + hours * 3600 + minutes * 60 + seconds;
    Ok(Duration::from_secs(total_seconds))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // WakeupDuration Tests
    // -------------------------------------------------------------------------

    #[test]
    fn wakeup_duration_valid_values() {
        for v in [1, 60, 300, 1800, 3000] {
            let duration = WakeupDuration::new(Duration::from_secs(v)).unwrap();
            assert_eq!(duration.seconds(), v as u16);
        }
    }

    #[test]
    fn wakeup_duration_invalid_values() {
        assert!(WakeupDuration::new(Duration::ZERO).is_err());
        assert!(WakeupDuration::new(Duration::from_secs(3001)).is_err());
    }

    #[test]
    fn wakeup_duration_rounds_to_nearest_second() {
        // 1.4s rounds to 1s
        let duration = WakeupDuration::new(Duration::from_millis(1400)).unwrap();
        assert_eq!(duration.seconds(), 1);

        // 1.5s rounds to 2s
        let duration = WakeupDuration::new(Duration::from_millis(1500)).unwrap();
        assert_eq!(duration.seconds(), 2);

        // 1.6s rounds to 2s
        let duration = WakeupDuration::new(Duration::from_millis(1600)).unwrap();
        assert_eq!(duration.seconds(), 2);
    }

    #[test]
    fn wakeup_duration_rounding_at_boundaries() {
        // 0.4s rounds to 0s -> error (below minimum)
        assert!(WakeupDuration::new(Duration::from_millis(400)).is_err());

        // 0.5s rounds to 1s -> valid (at minimum)
        let duration = WakeupDuration::new(Duration::from_millis(500)).unwrap();
        assert_eq!(duration.seconds(), 1);

        // 3000.4s rounds to 3000s -> valid (at maximum)
        let duration = WakeupDuration::new(Duration::from_millis(3_000_400)).unwrap();
        assert_eq!(duration.seconds(), 3000);

        // 3000.5s rounds to 3001s -> error (above maximum)
        assert!(WakeupDuration::new(Duration::from_millis(3_000_500)).is_err());
    }

    #[test]
    fn wakeup_duration_from_raw() {
        let duration = WakeupDuration::from_raw(300).unwrap();
        assert_eq!(duration.seconds(), 300);

        assert!(WakeupDuration::from_raw(0).is_err());
        assert!(WakeupDuration::from_raw(3001).is_err());
    }

    #[test]
    fn wakeup_duration_as_duration() {
        let wakeup = WakeupDuration::new(Duration::from_secs(300)).unwrap();
        assert_eq!(wakeup.as_duration(), Duration::from_secs(300));
    }

    #[test]
    fn wakeup_duration_minutes() {
        assert_eq!(
            WakeupDuration::new(Duration::from_secs(30))
                .unwrap()
                .minutes(),
            0
        );
        assert_eq!(
            WakeupDuration::new(Duration::from_secs(60))
                .unwrap()
                .minutes(),
            1
        );
        assert_eq!(
            WakeupDuration::new(Duration::from_secs(90))
                .unwrap()
                .minutes(),
            1
        );
        assert_eq!(
            WakeupDuration::new(Duration::from_secs(120))
                .unwrap()
                .minutes(),
            2
        );
    }

    #[test]
    fn wakeup_duration_as_formatted() {
        assert_eq!(
            WakeupDuration::new(Duration::from_secs(30))
                .unwrap()
                .as_formatted(),
            "30s"
        );
        assert_eq!(
            WakeupDuration::new(Duration::from_secs(60))
                .unwrap()
                .as_formatted(),
            "1m"
        );
        assert_eq!(
            WakeupDuration::new(Duration::from_secs(90))
                .unwrap()
                .as_formatted(),
            "1m 30s"
        );
        assert_eq!(
            WakeupDuration::new(Duration::from_secs(300))
                .unwrap()
                .as_formatted(),
            "5m"
        );
    }

    #[test]
    fn wakeup_duration_display() {
        assert_eq!(
            WakeupDuration::new(Duration::from_secs(90))
                .unwrap()
                .to_string(),
            "1m 30s"
        );
    }

    #[test]
    fn wakeup_duration_default() {
        assert_eq!(WakeupDuration::default().seconds(), 60);
    }

    #[test]
    fn wakeup_duration_ordering() {
        assert!(
            WakeupDuration::new(Duration::from_secs(60)).unwrap()
                < WakeupDuration::new(Duration::from_secs(120)).unwrap()
        );
    }

    #[test]
    fn wakeup_duration_constants() {
        assert_eq!(WakeupDuration::MIN, Duration::from_secs(1));
        assert_eq!(WakeupDuration::MAX, Duration::from_secs(3000));
    }

    // -------------------------------------------------------------------------
    // FadeDuration Tests
    // -------------------------------------------------------------------------

    #[test]
    fn fade_duration_from_duration() {
        // 0.5s = value 1
        let fade = FadeDuration::new(Duration::from_millis(500)).unwrap();
        assert_eq!(fade.value(), 1);

        // 2s = value 4
        let fade = FadeDuration::new(Duration::from_secs(2)).unwrap();
        assert_eq!(fade.value(), 4);

        // 10s = value 20
        let fade = FadeDuration::new(Duration::from_secs(10)).unwrap();
        assert_eq!(fade.value(), 20);

        // 20s = value 40
        let fade = FadeDuration::new(Duration::from_secs(20)).unwrap();
        assert_eq!(fade.value(), 40);
    }

    #[test]
    fn fade_duration_invalid_values() {
        // Too short (< 0.5s)
        assert!(FadeDuration::new(Duration::from_millis(200)).is_err());

        // Too long (> 20s)
        assert!(FadeDuration::new(Duration::from_secs(21)).is_err());
    }

    #[test]
    fn fade_duration_rounds_to_nearest_half_second() {
        // 0.7s rounds to 0.5s (value 1)
        let fade = FadeDuration::new(Duration::from_millis(700)).unwrap();
        assert_eq!(fade.value(), 1);

        // 0.8s rounds to 1.0s (value 2)
        let fade = FadeDuration::new(Duration::from_millis(800)).unwrap();
        assert_eq!(fade.value(), 2);

        // 1.2s rounds to 1.0s (value 2)
        let fade = FadeDuration::new(Duration::from_millis(1200)).unwrap();
        assert_eq!(fade.value(), 2);

        // 1.3s rounds to 1.5s (value 3)
        let fade = FadeDuration::new(Duration::from_millis(1300)).unwrap();
        assert_eq!(fade.value(), 3);
    }

    #[test]
    fn fade_duration_rounding_at_boundaries() {
        // 0.2s rounds to 0 -> error (below minimum)
        assert!(FadeDuration::new(Duration::from_millis(200)).is_err());

        // 0.25s rounds to 0.5s -> valid (at minimum)
        let fade = FadeDuration::new(Duration::from_millis(250)).unwrap();
        assert_eq!(fade.value(), 1);

        // 20.2s rounds to 20s -> valid (at maximum)
        let fade = FadeDuration::new(Duration::from_millis(20_200)).unwrap();
        assert_eq!(fade.value(), 40);

        // 20.3s rounds to 20.5s -> error (above maximum)
        assert!(FadeDuration::new(Duration::from_millis(20_300)).is_err());
    }

    #[test]
    fn fade_duration_from_raw() {
        let fade = FadeDuration::from_raw(20).unwrap();
        assert_eq!(fade.value(), 20);

        assert!(FadeDuration::from_raw(0).is_err());
        assert!(FadeDuration::from_raw(41).is_err());
    }

    #[test]
    fn fade_duration_as_duration() {
        // Fast (0.5s, value 1)
        let fast = FadeDuration::from_raw(1).unwrap();
        assert_eq!(fast.as_duration(), Duration::from_millis(500));

        // Medium (10s, value 20)
        let medium = FadeDuration::from_raw(20).unwrap();
        assert_eq!(medium.as_duration(), Duration::from_secs(10));

        // Slow (20s, value 40)
        let slow = FadeDuration::from_raw(40).unwrap();
        assert_eq!(slow.as_duration(), Duration::from_secs(20));
    }

    #[test]
    fn fade_duration_classification() {
        let fast = FadeDuration::from_raw(1).unwrap();
        let slow = FadeDuration::from_raw(40).unwrap();
        let medium = FadeDuration::from_raw(20).unwrap();

        assert!(fast.is_fast());
        assert!(!fast.is_slow());
        assert!(slow.is_slow());
        assert!(!slow.is_fast());
        assert!(!medium.is_fast());
        assert!(!medium.is_slow());
    }

    #[test]
    fn fade_duration_ordering() {
        // Lower value = faster = shorter duration
        let fast = FadeDuration::from_raw(1).unwrap();
        let slow = FadeDuration::from_raw(40).unwrap();
        assert!(fast < slow);
    }

    #[test]
    fn fade_duration_display() {
        let fast = FadeDuration::from_raw(1).unwrap();
        let medium = FadeDuration::from_raw(20).unwrap();
        let slow = FadeDuration::from_raw(40).unwrap();

        assert_eq!(fast.to_string(), "0.5s");
        assert_eq!(medium.to_string(), "10s");
        assert_eq!(slow.to_string(), "20s");

        let fade = FadeDuration::new(Duration::from_millis(1500)).unwrap();
        assert_eq!(fade.to_string(), "1.5s");
    }

    #[test]
    fn fade_duration_constants() {
        assert_eq!(FadeDuration::MIN, Duration::from_millis(500));
        assert_eq!(FadeDuration::MAX, Duration::from_secs(20));
    }

    #[test]
    fn fade_duration_default() {
        assert_eq!(
            FadeDuration::default().as_duration(),
            Duration::from_secs(10)
        );
    }

    // -------------------------------------------------------------------------
    // parse_uptime Tests
    // -------------------------------------------------------------------------

    #[test]
    fn parse_uptime_with_one_day() {
        let dur = parse_uptime("1T23:46:58").unwrap();
        // 1 * 86400 + 23 * 3600 + 46 * 60 + 58 = 172018
        assert_eq!(dur, Duration::from_secs(172_018));
    }

    #[test]
    fn parse_uptime_zero_days() {
        let dur = parse_uptime("0T12:30:45").unwrap();
        // 12 * 3600 + 30 * 60 + 45 = 45045
        assert_eq!(dur, Duration::from_secs(45_045));
    }

    #[test]
    fn parse_uptime_multiple_days() {
        let dur = parse_uptime("17T04:02:54").unwrap();
        // 17 * 86400 + 4 * 3600 + 2 * 60 + 54 = 1483374
        assert_eq!(dur, Duration::from_secs(1_483_374));
    }

    #[test]
    fn parse_uptime_minimal() {
        let dur = parse_uptime("0T00:00:00").unwrap();
        assert_eq!(dur, Duration::from_secs(0));
    }

    #[test]
    fn parse_uptime_five_seconds() {
        let dur = parse_uptime("0T00:00:05").unwrap();
        assert_eq!(dur, Duration::from_secs(5));
    }

    #[test]
    fn parse_uptime_one_hour() {
        let dur = parse_uptime("0T01:00:00").unwrap();
        assert_eq!(dur, Duration::from_secs(3600));
    }

    #[test]
    fn parse_uptime_large_days() {
        let dur = parse_uptime("365T12:30:45").unwrap();
        // 365 * 86400 + 12 * 3600 + 30 * 60 + 45 = 31581045
        assert_eq!(dur, Duration::from_secs(31_581_045));
    }

    #[test]
    fn parse_uptime_with_whitespace() {
        let dur = parse_uptime("  1T23:46:58  ").unwrap();
        assert_eq!(dur, Duration::from_secs(172_018));
    }

    #[test]
    fn parse_uptime_invalid_no_separator() {
        let result = parse_uptime("12:30:45");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("missing 'T' separator"));
    }

    #[test]
    fn parse_uptime_invalid_days() {
        let result = parse_uptime("abcT12:30:45");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("invalid days"));
    }

    #[test]
    fn parse_uptime_invalid_hours() {
        let result = parse_uptime("1Tab:30:45");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("invalid hours"));
    }

    #[test]
    fn parse_uptime_hours_out_of_range() {
        let result = parse_uptime("1T25:30:45");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("hours must be 0-23"));
    }

    #[test]
    fn parse_uptime_minutes_out_of_range() {
        let result = parse_uptime("1T12:60:45");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("minutes must be 0-59"));
    }

    #[test]
    fn parse_uptime_seconds_out_of_range() {
        let result = parse_uptime("1T12:30:60");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("seconds must be 0-59"));
    }

    #[test]
    fn parse_uptime_wrong_time_format() {
        let result = parse_uptime("1T12:30");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("expected HH:MM:SS"));
    }

    #[test]
    fn parse_uptime_empty_string() {
        let result = parse_uptime("");
        assert!(result.is_err());
    }
}
