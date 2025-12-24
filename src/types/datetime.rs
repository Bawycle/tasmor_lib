// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Datetime types for Tasmota telemetry.
//!
//! Tasmota devices can send timestamps in various formats depending on
//! their configuration. This module provides types and parsing functions
//! to handle all supported formats.
//!
//! # Supported Formats
//!
//! - ISO 8601 without timezone: `"2024-01-15T10:30:00"`
//! - ISO 8601 with timezone: `"2024-01-15T10:30:00+01:00"`
//! - Unix epoch seconds: `"1705318200"`
//! - Unix epoch milliseconds: `"1705318200000"`
//!
//! # Examples
//!
//! ```
//! use tasmor_lib::types::TasmotaDateTime;
//!
//! // Parse ISO 8601 without timezone
//! let dt: TasmotaDateTime = "2024-01-15T10:30:00".parse().unwrap();
//! assert!(dt.timezone_offset().is_none());
//!
//! // Parse ISO 8601 with timezone
//! let dt: TasmotaDateTime = "2024-01-15T10:30:00+01:00".parse().unwrap();
//! assert!(dt.timezone_offset().is_some());
//!
//! // Format using chrono's format method
//! println!("Date: {}", dt.naive().format("%Y-%m-%d"));
//! println!("French: {}", dt.naive().format("%d/%m/%Y"));
//! ```

use std::str::FromStr;

use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};

/// Error returned when parsing a datetime string fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateTimeParseError {
    input: String,
}

impl DateTimeParseError {
    /// Creates a new parse error for the given input.
    fn new(input: &str) -> Self {
        Self {
            input: input.to_string(),
        }
    }

    /// Returns the input string that failed to parse.
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }
}

impl std::fmt::Display for DateTimeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "failed to parse datetime: '{}' (expected ISO 8601 or Unix epoch)",
            self.input
        )
    }
}

impl std::error::Error for DateTimeParseError {}

/// A datetime value parsed from Tasmota telemetry.
///
/// This type provides access to both the naive datetime (without timezone)
/// and the timezone-aware datetime when the timezone is known.
///
/// # Timezone Availability
///
/// The timezone is available in these cases:
/// - The timestamp included a timezone offset (e.g., `+01:00`)
/// - The timestamp was in Unix epoch format (interpreted as UTC)
///
/// When the timestamp is a bare ISO 8601 datetime without timezone,
/// only the naive datetime is available.
///
/// # Formatting
///
/// Use chrono's `format()` method on `naive()` or `to_datetime()` for
/// custom formatting:
///
/// ```
/// use tasmor_lib::types::TasmotaDateTime;
///
/// let dt: TasmotaDateTime = "2024-01-15T10:30:00".parse().unwrap();
///
/// // ISO date
/// assert_eq!(dt.naive().format("%Y-%m-%d").to_string(), "2024-01-15");
///
/// // European format
/// assert_eq!(dt.naive().format("%d/%m/%Y").to_string(), "15/01/2024");
///
/// // Full datetime
/// assert_eq!(dt.naive().format("%Y-%m-%d %H:%M:%S").to_string(), "2024-01-15 10:30:00");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TasmotaDateTime {
    /// The naive datetime (without timezone).
    naive: NaiveDateTime,
    /// The timezone offset in seconds east of UTC, if known.
    /// Stored as `i32` instead of `FixedOffset` for serde compatibility.
    offset_secs: Option<i32>,
}

impl TasmotaDateTime {
    /// Parses a Tasmota datetime string.
    ///
    /// This is a convenience method that returns `Option<Self>`.
    /// For error details, use the `FromStr` implementation instead.
    ///
    /// # Supported Formats
    ///
    /// - `"2024-01-15T10:30:00"` - ISO 8601 without timezone
    /// - `"2024-01-15T10:30:00+01:00"` - ISO 8601 with timezone
    /// - `"2024-01-15T10:30:00Z"` - ISO 8601 with UTC timezone
    /// - `"1705318200"` - Unix epoch in seconds (UTC)
    /// - `"1705318200000"` - Unix epoch in milliseconds (UTC)
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::types::TasmotaDateTime;
    ///
    /// let dt = TasmotaDateTime::parse("2024-01-15T10:30:00").unwrap();
    /// println!("Naive: {}", dt.naive());
    /// ```
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    /// Parses a Unix epoch timestamp (seconds or milliseconds).
    fn parse_epoch(s: &str) -> Option<Self> {
        let timestamp: i64 = s.parse().ok()?;

        // Reject negative timestamps (before 1970)
        if timestamp < 0 {
            return None;
        }

        // Distinguish between seconds and milliseconds based on magnitude
        // Seconds: 10 digits (until year 2286)
        // Milliseconds: 13 digits
        let datetime = if timestamp > 9_999_999_999 {
            // Milliseconds
            let secs = timestamp / 1000;
            // Safe: (0..999) * 1_000_000 fits in u32
            let nsecs = u32::try_from((timestamp % 1000) * 1_000_000).ok()?;
            Utc.timestamp_opt(secs, nsecs).single()?
        } else {
            // Seconds
            Utc.timestamp_opt(timestamp, 0).single()?
        };

        Some(Self {
            naive: datetime.naive_utc(),
            offset_secs: Some(0), // UTC
        })
    }

    /// Parses an ISO 8601 datetime with timezone.
    fn parse_iso_with_tz(s: &str) -> Option<Self> {
        // Try parsing with timezone offset
        let datetime = DateTime::parse_from_rfc3339(s).ok()?;
        Some(Self {
            naive: datetime.naive_local(),
            offset_secs: Some(datetime.offset().local_minus_utc()),
        })
    }

    /// Parses an ISO 8601 datetime without timezone.
    fn parse_iso_naive(s: &str) -> Option<Self> {
        // Try common formats
        let formats = [
            "%Y-%m-%dT%H:%M:%S",
            "%Y-%m-%dT%H:%M:%S%.f",
            "%Y-%m-%d %H:%M:%S",
            "%Y-%m-%d %H:%M:%S%.f",
        ];

        for fmt in &formats {
            if let Ok(naive) = NaiveDateTime::parse_from_str(s, fmt) {
                return Some(Self {
                    naive,
                    offset_secs: None,
                });
            }
        }

        None
    }

    /// Returns the naive datetime (without timezone information).
    ///
    /// This is always available regardless of whether the original
    /// timestamp included timezone information. Use chrono's `format()`
    /// method for custom formatting.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::types::TasmotaDateTime;
    ///
    /// let dt: TasmotaDateTime = "2024-01-15T10:30:00".parse().unwrap();
    ///
    /// // Access chrono's NaiveDateTime methods
    /// use chrono::Datelike;
    /// assert_eq!(dt.naive().year(), 2024);
    ///
    /// // Format as needed
    /// let date_str = dt.naive().format("%Y-%m-%d").to_string();
    /// ```
    #[must_use]
    pub const fn naive(&self) -> NaiveDateTime {
        self.naive
    }

    /// Returns the timezone offset, if known.
    ///
    /// The offset is available when:
    /// - The original timestamp included a timezone offset
    /// - The timestamp was in Unix epoch format (UTC)
    #[must_use]
    pub fn timezone_offset(&self) -> Option<FixedOffset> {
        self.offset_secs.and_then(FixedOffset::east_opt)
    }

    /// Returns the timezone-aware datetime, if the timezone is known.
    ///
    /// # Returns
    ///
    /// Returns `Some(DateTime<FixedOffset>)` if the timezone was known,
    /// `None` if only the naive datetime is available.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::types::TasmotaDateTime;
    ///
    /// let dt: TasmotaDateTime = "2024-01-15T10:30:00+01:00".parse().unwrap();
    /// let datetime = dt.to_datetime().unwrap();
    ///
    /// // Format with timezone
    /// println!("{}", datetime.format("%Y-%m-%d %H:%M:%S %z"));
    /// ```
    #[must_use]
    pub fn to_datetime(&self) -> Option<DateTime<FixedOffset>> {
        self.timezone_offset()
            .and_then(|tz| self.naive.and_local_timezone(tz).single())
    }

    /// Returns true if the timezone is known.
    #[must_use]
    pub const fn has_timezone(&self) -> bool {
        self.offset_secs.is_some()
    }
}

impl FromStr for TasmotaDateTime {
    type Err = DateTimeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        // Try epoch formats first (all digits)
        if s.chars().all(|c| c.is_ascii_digit())
            && let Some(dt) = Self::parse_epoch(s)
        {
            return Ok(dt);
        }

        // Try ISO 8601 with timezone
        if let Some(dt) = Self::parse_iso_with_tz(s) {
            return Ok(dt);
        }

        // Try ISO 8601 without timezone
        Self::parse_iso_naive(s).ok_or_else(|| DateTimeParseError::new(s))
    }
}

impl From<NaiveDateTime> for TasmotaDateTime {
    fn from(naive: NaiveDateTime) -> Self {
        Self {
            naive,
            offset_secs: None,
        }
    }
}

impl From<DateTime<FixedOffset>> for TasmotaDateTime {
    fn from(datetime: DateTime<FixedOffset>) -> Self {
        Self {
            naive: datetime.naive_local(),
            offset_secs: Some(datetime.offset().local_minus_utc()),
        }
    }
}

impl std::fmt::Display for TasmotaDateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(dt) = self.to_datetime() {
            // Use chrono's RFC 3339 format for timezone-aware datetimes
            write!(f, "{}", dt.format("%Y-%m-%d %H:%M:%S %:z"))
        } else {
            // Use standard format for naive datetimes
            write!(f, "{}", self.naive.format("%Y-%m-%d %H:%M:%S"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn parse_iso_without_timezone() {
        let dt: TasmotaDateTime = "2024-01-15T10:30:00".parse().unwrap();
        assert_eq!(dt.naive().year(), 2024);
        assert_eq!(dt.naive().month(), 1);
        assert_eq!(dt.naive().day(), 15);
        assert_eq!(dt.naive().hour(), 10);
        assert_eq!(dt.naive().minute(), 30);
        assert!(dt.timezone_offset().is_none());
        assert!(dt.to_datetime().is_none());
    }

    #[test]
    fn parse_iso_with_positive_offset() {
        let dt: TasmotaDateTime = "2024-01-15T10:30:00+01:00".parse().unwrap();
        assert_eq!(dt.naive().hour(), 10);
        assert!(dt.has_timezone());
        let offset = dt.timezone_offset().unwrap();
        assert_eq!(offset.local_minus_utc(), 3600); // +1 hour
    }

    #[test]
    fn parse_iso_with_negative_offset() {
        let dt: TasmotaDateTime = "2024-01-15T10:30:00-05:00".parse().unwrap();
        assert!(dt.has_timezone());
        let offset = dt.timezone_offset().unwrap();
        assert_eq!(offset.local_minus_utc(), -5 * 3600); // -5 hours
    }

    #[test]
    fn parse_iso_with_utc() {
        let dt: TasmotaDateTime = "2024-01-15T10:30:00Z".parse().unwrap();
        assert!(dt.has_timezone());
        let offset = dt.timezone_offset().unwrap();
        assert_eq!(offset.local_minus_utc(), 0);
    }

    #[test]
    fn parse_epoch_seconds() {
        // 2024-01-15 10:30:00 UTC
        let dt: TasmotaDateTime = "1705314600".parse().unwrap();
        assert!(dt.has_timezone());
        assert_eq!(dt.naive().year(), 2024);
        assert_eq!(dt.naive().month(), 1);
        assert_eq!(dt.naive().day(), 15);
    }

    #[test]
    fn parse_epoch_milliseconds() {
        // 2024-01-15 10:30:00.123 UTC
        let dt: TasmotaDateTime = "1705314600123".parse().unwrap();
        assert!(dt.has_timezone());
        assert_eq!(dt.naive().year(), 2024);
    }

    #[test]
    fn parse_invalid_returns_error() {
        let err = "not a date".parse::<TasmotaDateTime>().unwrap_err();
        assert_eq!(err.input(), "not a date");
        assert!(err.to_string().contains("failed to parse datetime"));

        assert!("".parse::<TasmotaDateTime>().is_err());
        assert!("2024-13-45".parse::<TasmotaDateTime>().is_err());
    }

    #[test]
    fn parse_with_fractional_seconds() {
        let dt: TasmotaDateTime = "2024-01-15T10:30:00.123".parse().unwrap();
        assert_eq!(dt.naive().hour(), 10);
    }

    #[test]
    fn display_without_timezone() {
        let dt: TasmotaDateTime = "2024-01-15T10:30:00".parse().unwrap();
        assert_eq!(format!("{dt}"), "2024-01-15 10:30:00");
    }

    #[test]
    fn display_with_timezone() {
        let dt: TasmotaDateTime = "2024-01-15T10:30:00+01:00".parse().unwrap();
        assert_eq!(format!("{dt}"), "2024-01-15 10:30:00 +01:00");
    }

    #[test]
    fn custom_formatting_with_chrono() {
        let dt: TasmotaDateTime = "2024-01-15T10:30:00".parse().unwrap();

        // Users can format however they want using chrono
        assert_eq!(dt.naive().format("%Y-%m-%d").to_string(), "2024-01-15");
        assert_eq!(dt.naive().format("%d/%m/%Y").to_string(), "15/01/2024");
        assert_eq!(
            dt.naive().format("%B %d, %Y").to_string(),
            "January 15, 2024"
        );
    }

    #[test]
    fn from_naive_datetime() {
        let naive =
            NaiveDateTime::parse_from_str("2024-01-15 10:30:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let dt = TasmotaDateTime::from(naive);
        assert!(!dt.has_timezone());
        assert_eq!(dt.naive(), naive);
    }

    #[test]
    fn from_datetime_with_offset() {
        let datetime = DateTime::parse_from_rfc3339("2024-01-15T10:30:00+01:00").unwrap();
        let dt = TasmotaDateTime::from(datetime);
        assert!(dt.has_timezone());
        assert_eq!(dt.to_datetime(), Some(datetime));
    }

    #[test]
    fn into_conversion() {
        let naive =
            NaiveDateTime::parse_from_str("2024-01-15 10:30:00", "%Y-%m-%d %H:%M:%S").unwrap();

        // Test Into trait (automatically derived from From)
        let dt: TasmotaDateTime = naive.into();
        assert_eq!(dt.naive(), naive);
    }

    #[test]
    fn parse_convenience_method() {
        // parse() returns Option for convenience
        assert!(TasmotaDateTime::parse("2024-01-15T10:30:00").is_some());
        assert!(TasmotaDateTime::parse("invalid").is_none());
    }

    #[test]
    fn error_display() {
        let err = DateTimeParseError::new("bad input");
        assert!(err.to_string().contains("bad input"));
        assert!(err.to_string().contains("ISO 8601"));
    }
}
