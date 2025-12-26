// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Error types for the `TasmoR` library.
//!
//! This module provides a comprehensive error hierarchy for handling failures
//! across the library: value validation, protocol communication, JSON parsing,
//! and device operations.

use thiserror::Error;

/// The main error type for this library.
///
/// This enum encompasses all possible errors that can occur when interacting
/// with Tasmota devices.
#[derive(Debug, Error)]
pub enum Error {
    /// Error occurred during value validation.
    #[error("value error: {0}")]
    Value(#[from] ValueError),

    /// Error occurred during protocol communication.
    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    /// Error occurred while parsing a response.
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),

    /// Error occurred during device operations.
    #[error("device error: {0}")]
    Device(#[from] DeviceError),

    /// Device was not found in the manager.
    #[error("device not found")]
    DeviceNotFound,

    /// Device is not connected.
    #[error("device is not connected")]
    NotConnected,

    /// Device does not support the requested capability.
    #[error("device does not support this capability")]
    CapabilityNotSupported,
}

/// Errors related to value validation and constraints.
///
/// These errors occur when attempting to create constrained types
/// with invalid values.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ValueError {
    /// A numeric value is outside the allowed range.
    #[error("value {actual} is out of range [{min}, {max}]")]
    OutOfRange {
        /// Minimum allowed value.
        min: u16,
        /// Maximum allowed value.
        max: u16,
        /// The actual value that was provided.
        actual: u16,
    },

    /// An invalid power state string was provided.
    #[error("invalid power state: {0}")]
    InvalidPowerState(String),

    /// A hue value is outside the valid range (0-360).
    #[error("hue value {0} is out of range [0, 360]")]
    InvalidHue(u16),

    /// A saturation value is outside the valid range (0-100).
    #[error("saturation value {0} is out of range [0, 100]")]
    InvalidSaturation(u8),

    /// A brightness value is outside the valid range (0-100).
    #[error("brightness value {0} is out of range [0, 100]")]
    InvalidBrightness(u8),
}

/// Errors related to protocol communication (HTTP/MQTT).
#[derive(Debug, Error)]
pub enum ProtocolError {
    /// HTTP request failed.
    #[cfg(feature = "http")]
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// MQTT connection or communication failed.
    #[cfg(feature = "mqtt")]
    #[error("MQTT error: {0}")]
    Mqtt(#[from] rumqttc::ClientError),

    /// Connection to the device failed.
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    /// Request timed out.
    #[error("request timed out after {0} ms")]
    Timeout(u64),

    /// Invalid URL or address.
    #[error("invalid address: {0}")]
    InvalidAddress(String),

    /// Authentication failed.
    #[error("authentication failed")]
    AuthenticationFailed,

    /// Internal channel was closed.
    #[error("channel closed: {0}")]
    ChannelClosed(String),
}

/// Errors related to parsing Tasmota responses.
#[derive(Debug, Error)]
pub enum ParseError {
    /// JSON parsing failed.
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    /// Expected field is missing from the response.
    #[error("missing field in response: {0}")]
    MissingField(String),

    /// Unexpected response format.
    #[error("unexpected response format: {0}")]
    UnexpectedFormat(String),

    /// Failed to parse a specific value.
    #[error("failed to parse {field}: {message}")]
    InvalidValue {
        /// The field that failed to parse.
        field: String,
        /// Description of the parsing failure.
        message: String,
    },
}

/// Errors related to device operations.
#[derive(Debug, Error)]
pub enum DeviceError {
    /// Device does not support the requested capability.
    #[error("device does not support {capability}")]
    UnsupportedCapability {
        /// The capability that is not supported.
        capability: String,
    },

    /// Device is not connected.
    #[error("device is not connected")]
    NotConnected,

    /// Command was rejected by the device.
    #[error("command rejected: {0}")]
    CommandRejected(String),

    /// Device configuration is invalid.
    #[error("invalid device configuration: {0}")]
    InvalidConfiguration(String),
}

/// A specialized Result type for this library.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_error_display() {
        let err = ValueError::OutOfRange {
            min: 0,
            max: 100,
            actual: 150,
        };
        assert_eq!(err.to_string(), "value 150 is out of range [0, 100]");
    }

    #[test]
    fn error_from_value_error() {
        let value_err = ValueError::InvalidHue(400);
        let err: Error = value_err.into();
        assert!(matches!(err, Error::Value(ValueError::InvalidHue(400))));
    }

    #[test]
    fn parse_error_display() {
        let err = ParseError::MissingField("POWER".to_string());
        assert_eq!(err.to_string(), "missing field in response: POWER");
    }

    #[test]
    fn device_error_display() {
        let err = DeviceError::UnsupportedCapability {
            capability: "energy monitoring".to_string(),
        };
        assert_eq!(err.to_string(), "device does not support energy monitoring");
    }
}
