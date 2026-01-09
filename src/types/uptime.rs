// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Tasmota uptime format parsing.
//!
//! Tasmota reports device uptime in the format `"XdTHH:MM:SS"` where:
//! - `Xd` is the number of days (the 'd' suffix is sometimes omitted)
//! - `T` is the separator between days and time
//! - `HH:MM:SS` is hours, minutes, and seconds
//!
//! # Examples
//!
//! ```
//! use std::time::Duration;
//!
//! // Parse various uptime formats
//! let duration = tasmor_lib::types::parse_uptime("1T23:46:58").unwrap();
//! assert_eq!(duration, Duration::from_secs(172018));
//!
//! let duration = tasmor_lib::types::parse_uptime("0T00:00:05").unwrap();
//! assert_eq!(duration, Duration::from_secs(5));
//! ```

use std::time::Duration;

use crate::error::ParseError;

/// Parses a Tasmota uptime string into a [`Duration`].
///
/// # Format
///
/// Tasmota uses the format `"XdTHH:MM:SS"` or `"XTMM:SS"` where:
/// - `X` is the number of days
/// - `T` is the separator
/// - `HH:MM:SS` is hours (0-23), minutes (0-59), seconds (0-59)
///
/// The 'd' suffix after days may or may not be present depending on
/// Tasmota firmware version.
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

#[cfg(test)]
mod tests {
    use super::*;

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
