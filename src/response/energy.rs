// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Energy monitoring response parsing.

use crate::types::TasmotaDateTime;
use serde::Deserialize;

/// Energy monitoring response from Status 10 command.
///
/// Note: Status 10 replaces the deprecated Status 8 for sensor data.
/// Reference: <https://tasmota.github.io/docs/Commands/#management>
///
/// Contains power consumption data including:
/// - Current power (Watts)
/// - Voltage (Volts)
/// - Current (Amperes)
/// - Total energy consumed (kWh)
/// - Today's energy consumption (kWh)
/// - Yesterday's energy consumption (kWh)
///
/// # Examples
///
/// ```
/// use tasmor_lib::response::EnergyResponse;
///
/// let json = r#"{
///     "StatusSNS": {
///         "Time": "2024-01-01T12:00:00",
///         "ENERGY": {
///             "TotalStartTime": "2023-01-01T00:00:00",
///             "Total": 123.456,
///             "Yesterday": 1.234,
///             "Today": 0.567,
///             "Power": 45,
///             "Voltage": 230,
///             "Current": 0.196
///         }
///     }
/// }"#;
/// let response: EnergyResponse = serde_json::from_str(json).unwrap();
/// let energy = response.energy().unwrap();
/// assert_eq!(energy.power, 45.0);
/// assert_eq!(energy.voltage, 230.0);
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct EnergyResponse {
    /// Sensor status containing energy data.
    #[serde(rename = "StatusSNS")]
    pub status_sns: Option<SensorStatus>,

    /// Direct energy data (when using Energy command directly).
    #[serde(rename = "ENERGY")]
    pub direct_energy: Option<EnergyData>,
}

impl EnergyResponse {
    /// Returns the energy data from the response.
    #[must_use]
    pub fn energy(&self) -> Option<&EnergyData> {
        self.status_sns
            .as_ref()
            .and_then(|s| s.energy.as_ref())
            .or(self.direct_energy.as_ref())
    }

    /// Returns the current power consumption in Watts.
    #[must_use]
    pub fn power(&self) -> Option<f32> {
        self.energy().map(|e| e.power)
    }

    /// Returns the current voltage in Volts.
    #[must_use]
    pub fn voltage(&self) -> Option<f32> {
        self.energy().map(|e| e.voltage)
    }

    /// Returns the current in Amperes.
    #[must_use]
    pub fn current(&self) -> Option<f32> {
        self.energy().map(|e| e.current)
    }

    /// Returns the total energy consumed in kWh.
    #[must_use]
    pub fn total_energy(&self) -> Option<f32> {
        self.energy().map(|e| e.total)
    }

    /// Returns today's energy consumption in kWh.
    #[must_use]
    pub fn today_energy(&self) -> Option<f32> {
        self.energy().map(|e| e.today)
    }

    /// Returns yesterday's energy consumption in kWh.
    #[must_use]
    pub fn yesterday_energy(&self) -> Option<f32> {
        self.energy().map(|e| e.yesterday)
    }
}

/// Sensor status wrapper containing energy and other sensor data.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SensorStatus {
    /// Timestamp of the reading.
    #[serde(default)]
    pub time: String,

    /// Energy monitoring data.
    #[serde(rename = "ENERGY")]
    pub energy: Option<EnergyData>,
}

/// Energy monitoring data.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EnergyData {
    /// Start time for total energy counting.
    #[serde(default)]
    pub total_start_time: Option<TasmotaDateTime>,

    /// Total energy consumed in kWh.
    #[serde(default)]
    pub total: f32,

    /// Yesterday's energy consumption in kWh.
    #[serde(default)]
    pub yesterday: f32,

    /// Today's energy consumption in kWh.
    #[serde(default)]
    pub today: f32,

    /// Current power consumption in Watts.
    #[serde(default)]
    pub power: f32,

    /// Apparent power in `VA`.
    #[serde(default)]
    pub apparent_power: f32,

    /// Reactive power in `VAr`.
    #[serde(default)]
    pub reactive_power: f32,

    /// Power factor (0-1).
    #[serde(default)]
    pub factor: f32,

    /// Voltage in Volts.
    #[serde(default)]
    pub voltage: f32,

    /// Current in Amperes.
    #[serde(default)]
    pub current: f32,
}

impl EnergyData {
    /// Returns the power factor as a percentage (0-100).
    #[must_use]
    pub fn power_factor_percent(&self) -> f32 {
        self.factor * 100.0
    }

    /// Returns whether the device is currently consuming power.
    #[must_use]
    pub fn is_consuming(&self) -> bool {
        self.power > 0.0
    }

    /// Calculates the estimated daily cost based on current power and price per kWh.
    #[must_use]
    pub fn estimated_daily_cost(&self, price_per_kwh: f32) -> f32 {
        let kwh_per_day = self.power * 24.0 / 1000.0;
        kwh_per_day * price_per_kwh
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

    use super::*;

    #[test]
    fn parse_energy_response_0() {
        let json = r#"{
            "StatusSNS": {
                "Time": "2024-01-01T12:00:00",
                "ENERGY": {
                    "TotalStartTime": "2023-01-01T00:00:00",
                    "Total": 123.45678,
                    "Yesterday": 1.23456,
                    "Today": 0.56789,
                    "Power": 45.001,
                    "ApparentPower": 50.000,
                    "ReactivePower": 10.000,
                    "Factor": 0.9,
                    "Voltage": 229.987,
                    "Current": 0.196
                }
            }
        }"#;

        let response: EnergyResponse = serde_json::from_str(json).unwrap();
        let energy = response.energy().unwrap();

        assert_eq!(energy.power, 45.001);
        assert_eq!(energy.voltage, 229.987);
        assert!((energy.current - 0.196).abs() < f32::EPSILON);
        assert!((energy.total - 123.456).abs() < 0.001);
        assert!((energy.factor - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn parse_energy_response_1() {
        let json = r#"{
            "StatusSNS": {
                "Time": "2024-01-01T12:00:00",
                "ENERGY": {
                    "TotalStartTime": "2023-01-01T00:00:00",
                    "Total": 123.456,
                    "Yesterday": 1.234,
                    "Today": 0.567,
                    "Power": 45,
                    "ApparentPower": 50,
                    "ReactivePower": 10,
                    "Factor": 0.9,
                    "Voltage": 230,
                    "Current": 0.196
                }
            }
        }"#;

        let response: EnergyResponse = serde_json::from_str(json).unwrap();
        let energy = response.energy().unwrap();

        assert_abs_diff_eq!(energy.power, 45.0, epsilon = f32::EPSILON);
        assert_abs_diff_eq!(energy.voltage, 230.0, epsilon = f32::EPSILON);
        assert_abs_diff_eq!(energy.current, 0.196, epsilon = 0.001);
        assert_abs_diff_eq!(energy.total, 123.456, epsilon = 0.01);
        assert_abs_diff_eq!(energy.factor, 0.9, epsilon = 0.01);
    }

    #[test]
    fn energy_helper_methods_0() {
        let energy = EnergyData {
            total_start_time: None,
            total: 100.0,
            yesterday: 2.0,
            today: 1.0,
            power: 100.0,
            apparent_power: 110.0,
            reactive_power: 20.0,
            factor: 0.9,
            voltage: 230.0,
            current: 0.435,
        };

        assert!(energy.is_consuming());
        assert_abs_diff_eq!(energy.power_factor_percent(), 90.0, epsilon = 0.01);

        // 100W * 24h / 1000 = 2.4 kWh/day
        // 2.4 kWh * 0.15€/kWh = 0.36€/day
        let cost = energy.estimated_daily_cost(0.15);
        assert_abs_diff_eq!(cost, 0.36, epsilon = 0.01);
    }

    #[test]
    fn energy_helper_methods_1() {
        let energy = EnergyData {
            total_start_time: None,
            total: 100.0,
            yesterday: 2.0,
            today: 1.0,
            power: 100.001,
            apparent_power: 110.002,
            reactive_power: 20.003,
            factor: 0.9,
            voltage: 230.004,
            current: 0.435,
        };

        assert!(energy.is_consuming());
        assert!((energy.power_factor_percent() - 90.0).abs() < f32::EPSILON);

        // 100W * 24h / 1000 = 2.4 kWh/day
        // 2.4 kWh * 0.15€/kWh = 0.36€/day
        let cost = energy.estimated_daily_cost(0.15);
        assert!((cost - 0.36).abs() < 0.01);
    }

    #[test]
    fn parse_direct_energy_0() {
        let json = r#"{
            "ENERGY": {
                "Total": 50.0,
                "Yesterday": 1.0,
                "Today": 0.5,
                "Power": 25,
                "Voltage": 120,
                "Current": 0.208
            }
        }"#;

        let response: EnergyResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.power(), Some(25.0));
        assert_eq!(response.voltage(), Some(120.0));
    }

    #[test]
    fn parse_direct_energy_1() {
        let json = r#"{
            "ENERGY": {
                "Total": 50.012,
                "Yesterday": 1.123,
                "Today": 0.567,
                "Power": 25.987,
                "Voltage": 120.555,
                "Current": 0.20868
            }
        }"#;

        let response: EnergyResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.power(), Some(25.987));
        assert_eq!(response.voltage(), Some(120.555));
    }

    #[test]
    fn response_helper_methods_0() {
        let json = r#"{
            "StatusSNS": {
                "ENERGY": {
                    "Total": 100.0,
                    "Yesterday": 2.0,
                    "Today": 1.5,
                    "Power": 50,
                    "Voltage": 230,
                    "Current": 0.217
                }
            }
        }"#;

        let response: EnergyResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.power(), Some(50.0));
        assert_eq!(response.voltage(), Some(230.0));
        assert_abs_diff_eq!(response.current().unwrap(), 0.217, epsilon = 0.001);
        assert_abs_diff_eq!(response.total_energy().unwrap(), 100.0, epsilon = 0.01);
        assert_abs_diff_eq!(response.today_energy().unwrap(), 1.5, epsilon = 0.01);
        assert_abs_diff_eq!(response.yesterday_energy().unwrap(), 2.0, epsilon = 0.01);
    }

    #[test]
    fn response_helper_methods_1() {
        let json = r#"{
            "StatusSNS": {
                "ENERGY": {
                    "Total": 100.012,
                    "Yesterday": 2.123,
                    "Today": 1.234,
                    "Power": 50.345,
                    "Voltage": 230.456,
                    "Current": 0.21789
                }
            }
        }"#;

        let response: EnergyResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.power(), Some(50.345));
        assert_eq!(response.voltage(), Some(230.456));
        assert!((response.current().unwrap() - 0.21789).abs() < 0.001);
        assert!((response.total_energy().unwrap() - 100.012).abs() < f32::EPSILON);
        assert!((response.today_energy().unwrap() - 1.234).abs() < f32::EPSILON);
        assert!((response.yesterday_energy().unwrap() - 2.123).abs() < f32::EPSILON);
    }
}
