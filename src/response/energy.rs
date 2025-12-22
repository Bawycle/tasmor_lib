// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Energy monitoring response parsing.

use serde::Deserialize;

/// Energy monitoring response from Status 8 or Energy command.
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
/// assert_eq!(energy.power, 45);
/// assert_eq!(energy.voltage, 230);
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
    pub fn power(&self) -> Option<u32> {
        self.energy().map(|e| e.power)
    }

    /// Returns the current voltage in Volts.
    #[must_use]
    pub fn voltage(&self) -> Option<u16> {
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
    pub total_start_time: String,

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
    pub power: u32,

    /// Apparent power in `VA`.
    #[serde(default)]
    pub apparent_power: u32,

    /// Reactive power in `VAr`.
    #[serde(default)]
    pub reactive_power: u32,

    /// Power factor (0-1).
    #[serde(default)]
    pub factor: f32,

    /// Voltage in Volts.
    #[serde(default)]
    pub voltage: u16,

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
        self.power > 0
    }

    /// Calculates the estimated daily cost based on current power and price per kWh.
    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn estimated_daily_cost(&self, price_per_kwh: f32) -> f32 {
        let kwh_per_day = (self.power as f32) * 24.0 / 1000.0;
        kwh_per_day * price_per_kwh
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_energy_response() {
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

        assert_eq!(energy.power, 45);
        assert_eq!(energy.voltage, 230);
        assert!((energy.current - 0.196).abs() < f32::EPSILON);
        assert!((energy.total - 123.456).abs() < 0.001);
        assert!((energy.factor - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn energy_helper_methods() {
        let energy = EnergyData {
            total_start_time: String::new(),
            total: 100.0,
            yesterday: 2.0,
            today: 1.0,
            power: 100,
            apparent_power: 110,
            reactive_power: 20,
            factor: 0.9,
            voltage: 230,
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
    fn parse_direct_energy() {
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
        assert_eq!(response.power(), Some(25));
        assert_eq!(response.voltage(), Some(120));
    }

    #[test]
    fn response_helper_methods() {
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

        assert_eq!(response.power(), Some(50));
        assert_eq!(response.voltage(), Some(230));
        assert!((response.current().unwrap() - 0.217).abs() < 0.001);
        assert!((response.total_energy().unwrap() - 100.0).abs() < f32::EPSILON);
        assert!((response.today_energy().unwrap() - 1.5).abs() < f32::EPSILON);
        assert!((response.yesterday_energy().unwrap() - 2.0).abs() < f32::EPSILON);
    }
}
