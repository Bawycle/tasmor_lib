// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Parser for Tasmota STATE telemetry messages.

use serde::Deserialize;

use crate::error::ParseError;
use crate::state::StateChange;
use crate::types::{ColorTemp, Dimmer, HsbColor, PowerState};

/// Parsed state from a `tele/<topic>/STATE` message.
///
/// This struct represents the device state as reported in periodic
/// telemetry messages. Not all fields are present in every message.
///
/// # Examples
///
/// ```
/// use tasmor_lib::telemetry::TelemetryState;
///
/// let json = r#"{"POWER":"ON","Dimmer":75,"CT":326}"#;
/// let state: TelemetryState = serde_json::from_str(json).unwrap();
///
/// assert_eq!(state.power(), Some(tasmor_lib::PowerState::On));
/// assert_eq!(state.dimmer(), Some(75));
/// assert_eq!(state.color_temp(), Some(326));
/// ```
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TelemetryState {
    /// Power state for relay 1 (or single relay).
    #[serde(rename = "POWER", default)]
    power: Option<String>,

    /// Power state for relay 1 (alternative format).
    #[serde(rename = "POWER1", default)]
    power1: Option<String>,

    /// Power state for relay 2.
    #[serde(rename = "POWER2", default)]
    power2: Option<String>,

    /// Power state for relay 3.
    #[serde(rename = "POWER3", default)]
    power3: Option<String>,

    /// Power state for relay 4.
    #[serde(rename = "POWER4", default)]
    power4: Option<String>,

    /// Power state for relay 5.
    #[serde(rename = "POWER5", default)]
    power5: Option<String>,

    /// Power state for relay 6.
    #[serde(rename = "POWER6", default)]
    power6: Option<String>,

    /// Power state for relay 7.
    #[serde(rename = "POWER7", default)]
    power7: Option<String>,

    /// Power state for relay 8.
    #[serde(rename = "POWER8", default)]
    power8: Option<String>,

    /// Dimmer level (0-100).
    #[serde(rename = "Dimmer", default)]
    dimmer: Option<u8>,

    /// Color temperature in mireds (153-500).
    #[serde(rename = "CT", default)]
    ct: Option<u16>,

    /// HSB color as comma-separated string (e.g., "180,100,75").
    #[serde(rename = "HSBColor", default)]
    hsb_color: Option<String>,

    /// RGB color as hex string (e.g., "FF0000").
    #[serde(rename = "Color", default)]
    color: Option<String>,

    /// White channel value (0-100).
    #[serde(rename = "White", default)]
    white: Option<u8>,

    /// Fade setting (0 = off, 1 = on).
    #[serde(rename = "Fade", default)]
    fade: Option<u8>,

    /// Transition speed (1-40).
    #[serde(rename = "Speed", default)]
    speed: Option<u8>,

    /// Color scheme (0 = single color, 1-4 = patterns).
    #[serde(rename = "Scheme", default)]
    scheme: Option<u8>,

    /// Device uptime as string (e.g., "17T04:02:54").
    #[serde(rename = "Uptime", default)]
    uptime: Option<String>,

    /// Device uptime in seconds.
    #[serde(rename = "UptimeSec", default)]
    uptime_sec: Option<u64>,

    /// Wi-Fi information.
    #[serde(rename = "Wifi", default)]
    wifi: Option<WifiInfo>,
}

/// Wi-Fi connection information from telemetry.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct WifiInfo {
    /// SSID of the connected network.
    #[serde(rename = "SSId", default)]
    pub ssid: Option<String>,

    /// RSSI (signal strength percentage, 0-100).
    #[serde(rename = "RSSI", default)]
    pub rssi: Option<i32>,

    /// Signal strength in dBm.
    #[serde(rename = "Signal", default)]
    pub signal: Option<i32>,

    /// Wi-Fi channel.
    #[serde(rename = "Channel", default)]
    pub channel: Option<u8>,

    /// Number of reconnections.
    #[serde(rename = "LinkCount", default)]
    pub link_count: Option<u32>,
}

impl TelemetryState {
    /// Returns the power state for the primary relay.
    #[must_use]
    pub fn power(&self) -> Option<PowerState> {
        self.power
            .as_ref()
            .or(self.power1.as_ref())
            .and_then(|s| s.parse().ok())
    }

    /// Returns the power state for a specific relay (1-8).
    #[must_use]
    pub fn power_index(&self, index: u8) -> Option<PowerState> {
        let power_str = match index {
            1 => self.power.as_ref().or(self.power1.as_ref()),
            2 => self.power2.as_ref(),
            3 => self.power3.as_ref(),
            4 => self.power4.as_ref(),
            5 => self.power5.as_ref(),
            6 => self.power6.as_ref(),
            7 => self.power7.as_ref(),
            8 => self.power8.as_ref(),
            _ => None,
        };
        power_str.and_then(|s| s.parse().ok())
    }

    /// Returns all power states as (index, state) pairs.
    #[must_use]
    pub fn all_power_states(&self) -> Vec<(u8, PowerState)> {
        (1..=8)
            .filter_map(|i| self.power_index(i).map(|s| (i, s)))
            .collect()
    }

    /// Returns the dimmer level (0-100).
    #[must_use]
    pub fn dimmer(&self) -> Option<u8> {
        self.dimmer
    }

    /// Returns the color temperature in mireds.
    #[must_use]
    pub fn color_temp(&self) -> Option<u16> {
        self.ct
    }

    /// Returns the HSB color if present.
    #[must_use]
    pub fn hsb_color(&self) -> Option<HsbColor> {
        let hsb_str = self.hsb_color.as_ref()?;
        let parts: Vec<&str> = hsb_str.split(',').collect();
        if parts.len() != 3 {
            return None;
        }

        let hue: u16 = parts[0].parse().ok()?;
        let saturation: u8 = parts[1].parse().ok()?;
        let brightness: u8 = parts[2].parse().ok()?;

        HsbColor::new(hue, saturation, brightness).ok()
    }

    /// Returns the RGB color hex string.
    #[must_use]
    pub fn rgb_color(&self) -> Option<&str> {
        self.color.as_deref()
    }

    /// Returns the white channel value (0-100).
    #[must_use]
    pub fn white(&self) -> Option<u8> {
        self.white
    }

    /// Returns whether fade is enabled.
    #[must_use]
    pub fn fade_enabled(&self) -> Option<bool> {
        self.fade.map(|v| v != 0)
    }

    /// Returns the transition speed (1-40).
    #[must_use]
    pub fn speed(&self) -> Option<u8> {
        self.speed
    }

    /// Returns the color scheme (0 = single color, 1-4 = patterns).
    #[must_use]
    pub fn scheme(&self) -> Option<u8> {
        self.scheme
    }

    /// Returns the device uptime as a string (e.g., "17T04:02:54").
    #[must_use]
    pub fn uptime(&self) -> Option<&str> {
        self.uptime.as_deref()
    }

    /// Returns the device uptime in seconds.
    #[must_use]
    pub fn uptime_seconds(&self) -> Option<u64> {
        self.uptime_sec
    }

    /// Returns the Wi-Fi information.
    #[must_use]
    pub fn wifi(&self) -> Option<&WifiInfo> {
        self.wifi.as_ref()
    }

    /// Converts the telemetry state into a list of state changes.
    #[must_use]
    pub fn to_state_changes(&self) -> Vec<StateChange> {
        let mut changes = Vec::new();

        // Power states
        for (index, state) in self.all_power_states() {
            changes.push(StateChange::Power { index, state });
        }

        // Dimmer
        if let Some(dimmer) = self.dimmer {
            changes.push(StateChange::Dimmer(Dimmer::clamped(dimmer)));
        }

        // Color temperature
        if let Some(ct) = self.ct
            && let Ok(color_temp) = ColorTemp::new(ct)
        {
            changes.push(StateChange::ColorTemp(color_temp));
        }

        // HSB Color
        if let Some(hsb) = self.hsb_color() {
            changes.push(StateChange::HsbColor(hsb));
        }

        // If we have multiple changes, wrap in a batch
        if changes.len() > 1 {
            vec![StateChange::Batch(changes)]
        } else {
            changes
        }
    }
}

/// Parses a STATE telemetry JSON payload.
pub(crate) fn parse_state(payload: &str) -> Result<TelemetryState, ParseError> {
    serde_json::from_str(payload).map_err(ParseError::Json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_power_state() {
        let json = r#"{"POWER":"ON"}"#;
        let state: TelemetryState = serde_json::from_str(json).unwrap();

        assert_eq!(state.power(), Some(PowerState::On));
    }

    #[test]
    fn parse_power_off() {
        let json = r#"{"POWER":"OFF"}"#;
        let state: TelemetryState = serde_json::from_str(json).unwrap();

        assert_eq!(state.power(), Some(PowerState::Off));
    }

    #[test]
    fn parse_power1_format() {
        let json = r#"{"POWER1":"ON"}"#;
        let state: TelemetryState = serde_json::from_str(json).unwrap();

        assert_eq!(state.power(), Some(PowerState::On));
        assert_eq!(state.power_index(1), Some(PowerState::On));
    }

    #[test]
    fn parse_multiple_relays() {
        let json = r#"{"POWER1":"ON","POWER2":"OFF","POWER3":"ON"}"#;
        let state: TelemetryState = serde_json::from_str(json).unwrap();

        assert_eq!(state.power_index(1), Some(PowerState::On));
        assert_eq!(state.power_index(2), Some(PowerState::Off));
        assert_eq!(state.power_index(3), Some(PowerState::On));
        assert_eq!(state.power_index(4), None);

        let all = state.all_power_states();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn parse_dimmer() {
        let json = r#"{"POWER":"ON","Dimmer":75}"#;
        let state: TelemetryState = serde_json::from_str(json).unwrap();

        assert_eq!(state.dimmer(), Some(75));
    }

    #[test]
    fn parse_color_temp() {
        let json = r#"{"POWER":"ON","CT":326}"#;
        let state: TelemetryState = serde_json::from_str(json).unwrap();

        assert_eq!(state.color_temp(), Some(326));
    }

    #[test]
    fn parse_hsb_color() {
        let json = r#"{"HSBColor":"180,100,75"}"#;
        let state: TelemetryState = serde_json::from_str(json).unwrap();

        let hsb = state.hsb_color().unwrap();
        assert_eq!(hsb.hue(), 180);
        assert_eq!(hsb.saturation(), 100);
        assert_eq!(hsb.brightness(), 75);
    }

    #[test]
    fn parse_full_light_state() {
        let json = r#"{
            "POWER": "ON",
            "Dimmer": 50,
            "CT": 400,
            "HSBColor": "120,80,50",
            "Color": "80FF80",
            "White": 0,
            "Fade": 1,
            "Speed": 10,
            "Scheme": 0
        }"#;
        let state: TelemetryState = serde_json::from_str(json).unwrap();

        assert_eq!(state.power(), Some(PowerState::On));
        assert_eq!(state.dimmer(), Some(50));
        assert_eq!(state.color_temp(), Some(400));
        assert!(state.hsb_color().is_some());
        assert_eq!(state.rgb_color(), Some("80FF80"));
        assert_eq!(state.white(), Some(0));
        assert_eq!(state.fade_enabled(), Some(true));
        assert_eq!(state.speed(), Some(10));
    }

    #[test]
    fn parse_with_wifi_info() {
        let json = r#"{
            "POWER": "ON",
            "Wifi": {
                "SSId": "MyNetwork",
                "RSSI": 80,
                "Signal": -60,
                "Channel": 6,
                "LinkCount": 5
            }
        }"#;
        let state: TelemetryState = serde_json::from_str(json).unwrap();

        let wifi = state.wifi().unwrap();
        assert_eq!(wifi.ssid, Some("MyNetwork".to_string()));
        assert_eq!(wifi.rssi, Some(80));
        assert_eq!(wifi.signal, Some(-60));
        assert_eq!(wifi.channel, Some(6));
        assert_eq!(wifi.link_count, Some(5));
    }

    #[test]
    fn to_state_changes_single_power() {
        let json = r#"{"POWER":"ON"}"#;
        let state: TelemetryState = serde_json::from_str(json).unwrap();

        let changes = state.to_state_changes();
        assert_eq!(changes.len(), 1);
        assert!(matches!(
            &changes[0],
            StateChange::Power {
                index: 1,
                state: PowerState::On
            }
        ));
    }

    #[test]
    fn to_state_changes_multiple() {
        let json = r#"{"POWER":"ON","Dimmer":75,"CT":326}"#;
        let state: TelemetryState = serde_json::from_str(json).unwrap();

        let changes = state.to_state_changes();
        // Should be wrapped in a batch since multiple changes
        assert_eq!(changes.len(), 1);
        if let StateChange::Batch(batch) = &changes[0] {
            assert_eq!(batch.len(), 3); // power, dimmer, ct
        } else {
            panic!("Expected batch");
        }
    }

    #[test]
    fn parse_uptime() {
        let json = r#"{"Uptime":"17T04:02:54","UptimeSec":1483374}"#;
        let state: TelemetryState = serde_json::from_str(json).unwrap();

        assert_eq!(state.uptime_seconds(), Some(1_483_374));
    }
}
