// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Telemetry message parsing for Tasmota MQTT messages.
//!
//! This module provides parsers for Tasmota telemetry messages sent via MQTT.
//! Tasmota devices periodically send telemetry data on topics like:
//!
//! - `tele/<topic>/STATE` - Device state (power, dimmer, color, wifi info)
//! - `tele/<topic>/SENSOR` - Sensor readings (energy, temperature, humidity)
//! - `tele/<topic>/LWT` - Last Will Testament (Online/Offline status)
//!
//! # Examples
//!
//! ```
//! use tasmor_lib::telemetry::{TelemetryMessage, parse_telemetry};
//!
//! // Parse a STATE message
//! let topic = "tele/tasmota_bulb/STATE";
//! let payload = r#"{"POWER":"ON","Dimmer":75}"#;
//!
//! if let Ok(msg) = parse_telemetry(topic, payload) {
//!     match msg {
//!         TelemetryMessage::State { device_topic, state } => {
//!             println!("Device {} state: {:?}", device_topic, state);
//!         }
//!         _ => {}
//!     }
//! }
//! ```

mod sensor_parser;
mod state_parser;

pub use sensor_parser::{EnergyReading, SensorData, StatusSnsResponse};
pub use state_parser::TelemetryState;

use crate::error::ParseError;
use crate::state::StateChange;

/// A parsed telemetry message from a Tasmota device.
#[derive(Debug, Clone)]
pub enum TelemetryMessage {
    /// Device state from `tele/<topic>/STATE`.
    State {
        /// The device topic extracted from the MQTT topic.
        device_topic: String,
        /// The parsed state data.
        state: TelemetryState,
    },

    /// Sensor data from `tele/<topic>/SENSOR`.
    Sensor {
        /// The device topic extracted from the MQTT topic.
        device_topic: String,
        /// The parsed sensor data.
        data: SensorData,
    },

    /// Last Will Testament from `tele/<topic>/LWT`.
    LastWill {
        /// The device topic extracted from the MQTT topic.
        device_topic: String,
        /// Whether the device is online.
        online: bool,
    },

    /// Result message from `stat/<topic>/RESULT`.
    Result {
        /// The device topic extracted from the MQTT topic.
        device_topic: String,
        /// The raw JSON payload.
        payload: String,
    },
}

impl TelemetryMessage {
    /// Returns the device topic for this message.
    #[must_use]
    pub fn device_topic(&self) -> &str {
        match self {
            Self::State { device_topic, .. }
            | Self::Sensor { device_topic, .. }
            | Self::LastWill { device_topic, .. }
            | Self::Result { device_topic, .. } => device_topic,
        }
    }

    /// Converts the telemetry message into state changes.
    ///
    /// Returns a list of state changes that can be applied to a `DeviceState`.
    #[must_use]
    pub fn to_state_changes(&self) -> Vec<StateChange> {
        match self {
            Self::State { state, .. } => state.to_state_changes(),
            Self::Sensor { data, .. } => data.to_state_changes(),
            Self::LastWill { .. } | Self::Result { .. } => Vec::new(),
        }
    }

    /// Extracts system information from the telemetry message.
    ///
    /// Only STATE messages contain system information (uptime, Wi-Fi signal).
    /// Returns `None` for other message types.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use tasmor_lib::telemetry::parse_telemetry;
    ///
    /// let msg = parse_telemetry(
    ///     "tele/device/STATE",
    ///     r#"{"UptimeSec":172800,"Wifi":{"Signal":-55}}"#
    /// ).unwrap();
    ///
    /// if let Some(info) = msg.to_system_info() {
    ///     let uptime = info.uptime().unwrap_or(Duration::ZERO);
    ///     println!("Uptime: {} seconds", uptime.as_secs());
    /// }
    /// ```
    #[must_use]
    pub fn to_system_info(&self) -> Option<crate::state::SystemInfo> {
        match self {
            Self::State { state, .. } => {
                let info = state.to_system_info();
                if info.is_empty() { None } else { Some(info) }
            }
            _ => None,
        }
    }

    /// Returns true if this is an online LWT message.
    #[must_use]
    pub fn is_online(&self) -> bool {
        matches!(self, Self::LastWill { online: true, .. })
    }

    /// Returns true if this is an offline LWT message.
    #[must_use]
    pub fn is_offline(&self) -> bool {
        matches!(self, Self::LastWill { online: false, .. })
    }
}

/// Parses an MQTT topic and payload into a telemetry message.
///
/// # Arguments
///
/// * `topic` - The MQTT topic (e.g., `tele/tasmota_bulb/STATE`)
/// * `payload` - The JSON payload
///
/// # Returns
///
/// Returns the parsed telemetry message, or an error if parsing fails.
///
/// # Errors
///
/// Returns `ParseError` if:
/// - The topic format is not recognized
/// - The JSON payload is malformed
///
/// # Examples
///
/// ```
/// use tasmor_lib::telemetry::parse_telemetry;
///
/// // Parse a STATE message
/// let msg = parse_telemetry(
///     "tele/living_room/STATE",
///     r#"{"POWER":"ON","Dimmer":50}"#
/// ).unwrap();
///
/// // Parse a SENSOR message with energy data
/// let msg = parse_telemetry(
///     "tele/smart_plug/SENSOR",
///     r#"{"Time":"2024-01-01T12:00:00","ENERGY":{"Power":150,"Voltage":230,"Current":0.65}}"#
/// ).unwrap();
///
/// // Parse LWT message
/// let msg = parse_telemetry("tele/device/LWT", "Online").unwrap();
/// ```
pub fn parse_telemetry(topic: &str, payload: &str) -> Result<TelemetryMessage, ParseError> {
    let parts: Vec<&str> = topic.split('/').collect();

    if parts.len() < 3 {
        return Err(ParseError::UnexpectedFormat(format!(
            "Invalid topic format: {topic}"
        )));
    }

    let prefix = parts[0];
    let device_topic = parts[1].to_string();
    let suffix = parts[2];

    match (prefix, suffix) {
        ("tele", "STATE") => {
            let state = state_parser::parse_state(payload)?;
            Ok(TelemetryMessage::State {
                device_topic,
                state,
            })
        }
        ("tele", "SENSOR") => {
            let data = sensor_parser::parse_sensor(payload)?;
            Ok(TelemetryMessage::Sensor { device_topic, data })
        }
        ("tele", "LWT") => {
            let online = payload.eq_ignore_ascii_case("online");
            Ok(TelemetryMessage::LastWill {
                device_topic,
                online,
            })
        }
        ("stat", "RESULT") => Ok(TelemetryMessage::Result {
            device_topic,
            payload: payload.to_string(),
        }),
        _ => Err(ParseError::UnexpectedFormat(format!(
            "Unknown topic type: {prefix}/{suffix}"
        ))),
    }
}

/// Extracts the device topic from an MQTT topic string.
///
/// # Examples
///
/// ```
/// use tasmor_lib::telemetry::extract_device_topic;
///
/// assert_eq!(extract_device_topic("tele/tasmota_bulb/STATE"), Some("tasmota_bulb"));
/// assert_eq!(extract_device_topic("stat/my_device/RESULT"), Some("my_device"));
/// assert_eq!(extract_device_topic("invalid"), None);
/// ```
#[must_use]
pub fn extract_device_topic(mqtt_topic: &str) -> Option<&str> {
    let parts: Vec<&str> = mqtt_topic.split('/').collect();
    if parts.len() >= 3 {
        Some(parts[1])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_state_message() {
        let msg = parse_telemetry("tele/tasmota_bulb/STATE", r#"{"POWER":"ON"}"#).unwrap();

        assert_eq!(msg.device_topic(), "tasmota_bulb");
        assert!(matches!(msg, TelemetryMessage::State { .. }));
    }

    #[test]
    fn parse_sensor_message() {
        let msg = parse_telemetry(
            "tele/smart_plug/SENSOR",
            r#"{"Time":"2024-01-01T12:00:00","ENERGY":{"Power":100}}"#,
        )
        .unwrap();

        assert_eq!(msg.device_topic(), "smart_plug");
        assert!(matches!(msg, TelemetryMessage::Sensor { .. }));
    }

    #[test]
    fn parse_lwt_online() {
        let msg = parse_telemetry("tele/device/LWT", "Online").unwrap();

        assert_eq!(msg.device_topic(), "device");
        assert!(msg.is_online());
        assert!(!msg.is_offline());
    }

    #[test]
    fn parse_lwt_offline() {
        let msg = parse_telemetry("tele/device/LWT", "Offline").unwrap();

        assert!(msg.is_offline());
        assert!(!msg.is_online());
    }

    #[test]
    fn parse_result_message() {
        let msg = parse_telemetry("stat/device/RESULT", r#"{"POWER":"ON"}"#).unwrap();

        assert!(matches!(msg, TelemetryMessage::Result { .. }));
    }

    #[test]
    fn extract_device_topic_valid() {
        assert_eq!(
            extract_device_topic("tele/tasmota_bulb/STATE"),
            Some("tasmota_bulb")
        );
        assert_eq!(
            extract_device_topic("stat/my_device/RESULT"),
            Some("my_device")
        );
        assert_eq!(extract_device_topic("cmnd/switch/POWER"), Some("switch"));
    }

    #[test]
    fn extract_device_topic_invalid() {
        assert_eq!(extract_device_topic("invalid"), None);
        assert_eq!(extract_device_topic("only/two"), None);
    }

    #[test]
    fn invalid_topic_format() {
        let result = parse_telemetry("invalid", "{}");
        assert!(result.is_err());
    }

    #[test]
    fn unknown_topic_type() {
        let result = parse_telemetry("foo/device/BAR", "{}");
        assert!(result.is_err());
    }

    // ========== to_system_info() Tests ==========

    #[test]
    fn telemetry_message_to_system_info_state() {
        use std::time::Duration;

        let msg = parse_telemetry(
            "tele/device/STATE",
            r#"{"UptimeSec":172800,"Wifi":{"Signal":-55}}"#,
        )
        .unwrap();

        let info = msg.to_system_info();
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.uptime(), Some(Duration::from_secs(172800)));
        assert_eq!(info.wifi_rssi(), Some(-55));
    }

    #[test]
    fn telemetry_message_to_system_info_none_for_sensor() {
        let msg = parse_telemetry("tele/device/SENSOR", r#"{"ENERGY":{"Power":100}}"#).unwrap();

        assert!(msg.to_system_info().is_none());
    }

    #[test]
    fn telemetry_message_to_system_info_none_for_lwt() {
        let msg = parse_telemetry("tele/device/LWT", "Online").unwrap();
        assert!(msg.to_system_info().is_none());
    }

    #[test]
    fn telemetry_message_to_system_info_none_when_empty() {
        let msg = parse_telemetry("tele/device/STATE", r#"{"POWER":"ON"}"#).unwrap();

        // No system info fields, should return None
        assert!(msg.to_system_info().is_none());
    }
}
