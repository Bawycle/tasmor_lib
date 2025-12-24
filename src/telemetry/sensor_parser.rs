// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Parser for Tasmota SENSOR telemetry messages.

use serde::Deserialize;

use crate::error::ParseError;
use crate::state::StateChange;

/// Parsed sensor data from a `tele/<topic>/SENSOR` message.
///
/// This struct represents sensor readings as reported in periodic
/// telemetry messages. Energy data is the most common, but temperature
/// and humidity sensors are also supported.
///
/// # Examples
///
/// ```
/// use tasmor_lib::telemetry::SensorData;
///
/// let json = r#"{"Time":"2024-01-01T12:00:00","ENERGY":{"Power":150,"Voltage":230,"Current":0.65}}"#;
/// let data: SensorData = serde_json::from_str(json).unwrap();
///
/// if let Some(energy) = data.energy() {
///     assert_eq!(energy.power, Some(150));
///     assert_eq!(energy.voltage, Some(230));
/// }
/// ```
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SensorData {
    /// Timestamp of the reading.
    #[serde(rename = "Time", default)]
    time: Option<String>,

    /// Energy readings (power, voltage, current, etc.).
    #[serde(rename = "ENERGY", default)]
    energy: Option<EnergyReading>,

    /// Temperature reading (from various sensor types).
    #[serde(rename = "Temperature", default)]
    temperature: Option<f32>,

    /// Humidity reading.
    #[serde(rename = "Humidity", default)]
    humidity: Option<f32>,

    /// Pressure reading (in hPa).
    #[serde(rename = "Pressure", default)]
    pressure: Option<f32>,

    /// DS18B20 temperature sensor.
    #[serde(rename = "DS18B20", default)]
    ds18b20: Option<TemperatureSensor>,

    /// DHT11/DHT22 sensor.
    #[serde(rename = "DHT11", default)]
    dht11: Option<DhtSensor>,

    /// AM2301 sensor (same as DHT21).
    #[serde(rename = "AM2301", default)]
    am2301: Option<DhtSensor>,

    /// BME280 sensor.
    #[serde(rename = "BME280", default)]
    bme280: Option<Bme280Sensor>,
}

/// Energy readings from a power monitoring device.
///
/// Fields correspond to Tasmota's ENERGY telemetry output.
/// All fields are optional as not all devices report all values.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct EnergyReading {
    /// Timestamp when total energy counting started.
    ///
    /// Format: ISO 8601 datetime string (e.g., "2024-01-15T10:30:00").
    #[serde(rename = "TotalStartTime", default)]
    pub total_start_time: Option<String>,

    /// Total energy consumed today (in kWh).
    #[serde(rename = "Today", default)]
    pub today: Option<f32>,

    /// Total energy consumed yesterday (in kWh).
    #[serde(rename = "Yesterday", default)]
    pub yesterday: Option<f32>,

    /// Total energy consumed (in kWh).
    #[serde(rename = "Total", default)]
    pub total: Option<f32>,

    /// Current power consumption (in Watts).
    #[serde(rename = "Power", default)]
    pub power: Option<u32>,

    /// Apparent power (in VA).
    #[serde(rename = "ApparentPower", default)]
    pub apparent_power: Option<u32>,

    /// Reactive power (in `VAr`).
    #[serde(rename = "ReactivePower", default)]
    pub reactive_power: Option<u32>,

    /// Power factor (0-1).
    #[serde(rename = "Factor", default)]
    pub factor: Option<f32>,

    /// Voltage (in Volts).
    #[serde(rename = "Voltage", default)]
    pub voltage: Option<u16>,

    /// Current (in Amps).
    #[serde(rename = "Current", default)]
    pub current: Option<f32>,

    /// Frequency (in Hz).
    #[serde(rename = "Frequency", default)]
    pub frequency: Option<f32>,
}

/// Temperature sensor reading.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TemperatureSensor {
    /// Temperature in configured units (C or F).
    #[serde(rename = "Temperature", default)]
    pub temperature: Option<f32>,

    /// Sensor ID (for multi-sensor setups).
    #[serde(rename = "Id", default)]
    id: Option<String>,
}

impl TemperatureSensor {
    /// Returns the temperature reading.
    #[must_use]
    pub fn temperature(&self) -> Option<f32> {
        self.temperature
    }

    /// Returns the sensor ID (for multi-sensor setups).
    #[must_use]
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }
}

/// DHT temperature/humidity sensor reading.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DhtSensor {
    /// Temperature in configured units (C or F).
    #[serde(rename = "Temperature", default)]
    temperature: Option<f32>,

    /// Relative humidity (0-100%).
    #[serde(rename = "Humidity", default)]
    humidity: Option<f32>,

    /// Dew point temperature.
    #[serde(rename = "DewPoint", default)]
    dew_point: Option<f32>,
}

impl DhtSensor {
    /// Returns the temperature reading.
    #[must_use]
    pub fn temperature(&self) -> Option<f32> {
        self.temperature
    }

    /// Returns the humidity reading (0-100%).
    #[must_use]
    pub fn humidity(&self) -> Option<f32> {
        self.humidity
    }

    /// Returns the dew point temperature.
    #[must_use]
    pub fn dew_point(&self) -> Option<f32> {
        self.dew_point
    }
}

/// BME280 environmental sensor reading.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Bme280Sensor {
    /// Temperature in configured units (C or F).
    #[serde(rename = "Temperature", default)]
    temperature: Option<f32>,

    /// Relative humidity (0-100%).
    #[serde(rename = "Humidity", default)]
    humidity: Option<f32>,

    /// Dew point temperature.
    #[serde(rename = "DewPoint", default)]
    dew_point: Option<f32>,

    /// Atmospheric pressure (in hPa).
    #[serde(rename = "Pressure", default)]
    pressure: Option<f32>,
}

impl Bme280Sensor {
    /// Returns the temperature reading.
    #[must_use]
    pub fn temperature(&self) -> Option<f32> {
        self.temperature
    }

    /// Returns the humidity reading (0-100%).
    #[must_use]
    pub fn humidity(&self) -> Option<f32> {
        self.humidity
    }

    /// Returns the dew point temperature.
    #[must_use]
    pub fn dew_point(&self) -> Option<f32> {
        self.dew_point
    }

    /// Returns the atmospheric pressure (in hPa).
    #[must_use]
    pub fn pressure(&self) -> Option<f32> {
        self.pressure
    }
}

impl SensorData {
    /// Returns the timestamp of the sensor reading.
    #[must_use]
    pub fn time(&self) -> Option<&str> {
        self.time.as_deref()
    }

    /// Returns the energy reading if present.
    #[must_use]
    pub fn energy(&self) -> Option<&EnergyReading> {
        self.energy.as_ref()
    }

    /// Returns the temperature from any available sensor.
    ///
    /// Checks in order: direct temperature field, DS18B20, DHT11, AM2301, BME280.
    #[must_use]
    pub fn temperature(&self) -> Option<f32> {
        self.temperature
            .or_else(|| {
                self.ds18b20
                    .as_ref()
                    .and_then(TemperatureSensor::temperature)
            })
            .or_else(|| self.dht11.as_ref().and_then(DhtSensor::temperature))
            .or_else(|| self.am2301.as_ref().and_then(DhtSensor::temperature))
            .or_else(|| self.bme280.as_ref().and_then(Bme280Sensor::temperature))
    }

    /// Returns the humidity from any available sensor.
    ///
    /// Checks in order: direct humidity field, DHT11, AM2301, BME280.
    #[must_use]
    pub fn humidity(&self) -> Option<f32> {
        self.humidity
            .or_else(|| self.dht11.as_ref().and_then(DhtSensor::humidity))
            .or_else(|| self.am2301.as_ref().and_then(DhtSensor::humidity))
            .or_else(|| self.bme280.as_ref().and_then(Bme280Sensor::humidity))
    }

    /// Returns the pressure from any available sensor.
    ///
    /// Checks in order: direct pressure field, BME280.
    #[must_use]
    pub fn pressure(&self) -> Option<f32> {
        self.pressure
            .or_else(|| self.bme280.as_ref().and_then(Bme280Sensor::pressure))
    }

    /// Returns the DS18B20 temperature sensor reading if present.
    #[must_use]
    pub fn ds18b20(&self) -> Option<&TemperatureSensor> {
        self.ds18b20.as_ref()
    }

    /// Returns the DHT11 sensor reading if present.
    #[must_use]
    pub fn dht11(&self) -> Option<&DhtSensor> {
        self.dht11.as_ref()
    }

    /// Returns the AM2301 sensor reading if present.
    #[must_use]
    pub fn am2301(&self) -> Option<&DhtSensor> {
        self.am2301.as_ref()
    }

    /// Returns the BME280 sensor reading if present.
    #[must_use]
    pub fn bme280(&self) -> Option<&Bme280Sensor> {
        self.bme280.as_ref()
    }

    /// Converts the sensor data into a list of state changes.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn to_state_changes(&self) -> Vec<StateChange> {
        let mut changes = Vec::new();

        if let Some(energy) = &self.energy {
            // Only emit energy change if at least one field is present
            if energy.has_power_data() || energy.has_consumption_data() {
                // Safe: power and voltage values from Tasmota are well within f32 precision range
                changes.push(StateChange::Energy {
                    power: energy.power.map(|p| p as f32),
                    voltage: energy.voltage.map(f32::from),
                    current: energy.current,
                    apparent_power: energy.apparent_power.map(|p| p as f32),
                    reactive_power: energy.reactive_power.map(|p| p as f32),
                    power_factor: energy.factor,
                    energy_today: energy.today,
                    energy_yesterday: energy.yesterday,
                    energy_total: energy.total,
                    total_start_time: energy.total_start_time.clone(),
                });
            }
        }

        changes
    }
}

impl EnergyReading {
    /// Returns true if any power-related field is present.
    #[must_use]
    pub fn has_power_data(&self) -> bool {
        self.power.is_some() || self.voltage.is_some() || self.current.is_some()
    }

    /// Returns true if any energy consumption data is present.
    #[must_use]
    pub fn has_consumption_data(&self) -> bool {
        self.today.is_some() || self.yesterday.is_some() || self.total.is_some()
    }
}

/// Parses a SENSOR telemetry JSON payload.
pub(crate) fn parse_sensor(payload: &str) -> Result<SensorData, ParseError> {
    serde_json::from_str(payload).map_err(ParseError::Json)
}

/// Response wrapper for `Status 10` command.
///
/// The `Status 10` command returns sensor data wrapped in a `StatusSNS` object:
/// ```json
/// {"StatusSNS":{"Time":"...","ENERGY":{"Power":150,...}}}
/// ```
#[derive(Debug, Clone, Default, Deserialize)]
pub struct StatusSnsResponse {
    /// The wrapped sensor data.
    #[serde(rename = "StatusSNS")]
    pub status_sns: Option<SensorData>,
}

impl StatusSnsResponse {
    /// Returns the sensor data if present.
    #[must_use]
    pub fn sensor_data(&self) -> Option<&SensorData> {
        self.status_sns.as_ref()
    }

    /// Converts to state changes.
    #[must_use]
    pub fn to_state_changes(&self) -> Vec<StateChange> {
        self.status_sns
            .as_ref()
            .map_or_else(Vec::new, SensorData::to_state_changes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_energy_basic() {
        let json = r#"{"Time":"2024-01-01T12:00:00","ENERGY":{"Power":150}}"#;
        let data: SensorData = serde_json::from_str(json).unwrap();

        let energy = data.energy().unwrap();
        assert_eq!(energy.power, Some(150));
    }

    #[test]
    fn parse_energy_full() {
        let json = r#"{
            "Time": "2024-01-01T12:00:00",
            "ENERGY": {
                "Today": 1.5,
                "Yesterday": 2.3,
                "Total": 1234.5,
                "Power": 150,
                "ApparentPower": 160,
                "ReactivePower": 20,
                "Factor": 0.95,
                "Voltage": 230,
                "Current": 0.65,
                "Frequency": 50.0
            }
        }"#;
        let data: SensorData = serde_json::from_str(json).unwrap();

        let energy = data.energy().unwrap();
        assert_eq!(energy.today, Some(1.5));
        assert_eq!(energy.yesterday, Some(2.3));
        assert_eq!(energy.total, Some(1234.5));
        assert_eq!(energy.power, Some(150));
        assert_eq!(energy.apparent_power, Some(160));
        assert_eq!(energy.reactive_power, Some(20));
        assert_eq!(energy.factor, Some(0.95));
        assert_eq!(energy.voltage, Some(230));
        assert_eq!(energy.current, Some(0.65));
        assert_eq!(energy.frequency, Some(50.0));
    }

    #[test]
    fn parse_temperature_direct() {
        let json = r#"{"Time":"2024-01-01T12:00:00","Temperature":23.5}"#;
        let data: SensorData = serde_json::from_str(json).unwrap();

        assert_eq!(data.temperature(), Some(23.5));
    }

    #[test]
    fn parse_ds18b20() {
        let json = r#"{"Time":"2024-01-01T12:00:00","DS18B20":{"Temperature":22.5,"Id":"28-0123456789ab"}}"#;
        let data: SensorData = serde_json::from_str(json).unwrap();

        assert_eq!(data.temperature(), Some(22.5));
        assert_eq!(
            data.ds18b20().and_then(TemperatureSensor::id),
            Some("28-0123456789ab")
        );
    }

    #[test]
    fn parse_dht11() {
        let json = r#"{"Time":"2024-01-01T12:00:00","DHT11":{"Temperature":24.0,"Humidity":55.0}}"#;
        let data: SensorData = serde_json::from_str(json).unwrap();

        assert_eq!(data.temperature(), Some(24.0));
        assert_eq!(data.humidity(), Some(55.0));
    }

    #[test]
    fn parse_bme280() {
        let json = r#"{
            "Time": "2024-01-01T12:00:00",
            "BME280": {
                "Temperature": 21.5,
                "Humidity": 60.0,
                "DewPoint": 13.2,
                "Pressure": 1013.25
            }
        }"#;
        let data: SensorData = serde_json::from_str(json).unwrap();

        assert_eq!(data.temperature(), Some(21.5));
        assert_eq!(data.humidity(), Some(60.0));
        assert_eq!(data.pressure(), Some(1013.25));

        let bme = data.bme280().unwrap();
        assert_eq!(bme.dew_point(), Some(13.2));
    }

    #[test]
    fn energy_has_power_data() {
        let energy = EnergyReading {
            power: Some(100),
            ..Default::default()
        };
        assert!(energy.has_power_data());

        let empty = EnergyReading::default();
        assert!(!empty.has_power_data());
    }

    #[test]
    fn energy_has_consumption_data() {
        let energy = EnergyReading {
            total: Some(1234.5),
            ..Default::default()
        };
        assert!(energy.has_consumption_data());

        let empty = EnergyReading::default();
        assert!(!empty.has_consumption_data());
    }

    #[test]
    fn to_state_changes_with_energy() {
        let json = r#"{"ENERGY":{"Power":150,"Voltage":230,"Current":0.65}}"#;
        let data: SensorData = serde_json::from_str(json).unwrap();

        let changes = data.to_state_changes();
        assert_eq!(changes.len(), 1);
        if let StateChange::Energy {
            power,
            voltage,
            current,
            ..
        } = &changes[0]
        {
            assert!((power.unwrap() - 150.0).abs() < f32::EPSILON);
            assert!((voltage.unwrap() - 230.0).abs() < f32::EPSILON);
            assert!((current.unwrap() - 0.65).abs() < f32::EPSILON);
        } else {
            panic!("Expected StateChange::Energy");
        }
    }

    #[test]
    fn to_state_changes_empty() {
        let json = r#"{"Time":"2024-01-01T12:00:00"}"#;
        let data: SensorData = serde_json::from_str(json).unwrap();

        let changes = data.to_state_changes();
        assert!(changes.is_empty());
    }

    #[test]
    fn parse_sensor_function() {
        let json = r#"{"Time":"2024-01-01T12:00:00","ENERGY":{"Power":100}}"#;
        let result = parse_sensor(json);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.energy().unwrap().power, Some(100));
    }

    #[test]
    fn parse_sensor_invalid_json() {
        let result = parse_sensor("not json");
        assert!(result.is_err());
    }

    #[test]
    fn parse_status_sns_response() {
        // This is the format returned by Status 10 command
        let json = r#"{
            "StatusSNS": {
                "Time": "2024-01-01T12:00:00",
                "ENERGY": {
                    "Power": 182,
                    "Voltage": 224,
                    "Current": 0.706,
                    "Total": 1104.315
                }
            }
        }"#;

        let response: StatusSnsResponse = serde_json::from_str(json).unwrap();
        let sensor = response.sensor_data().unwrap();
        let energy = sensor.energy().unwrap();

        assert_eq!(energy.power, Some(182));
        assert_eq!(energy.voltage, Some(224));
        assert!((energy.current.unwrap() - 0.706).abs() < 0.001);
        assert!((energy.total.unwrap() - 1104.315).abs() < 0.001);
    }

    #[test]
    fn status_sns_to_state_changes() {
        let json = r#"{"StatusSNS":{"ENERGY":{"Power":150,"Voltage":230,"Current":0.65}}}"#;

        let response: StatusSnsResponse = serde_json::from_str(json).unwrap();
        let changes = response.to_state_changes();

        assert_eq!(changes.len(), 1);
        if let StateChange::Energy {
            power,
            voltage,
            current,
            ..
        } = &changes[0]
        {
            assert!((power.unwrap() - 150.0).abs() < f32::EPSILON);
            assert!((voltage.unwrap() - 230.0).abs() < f32::EPSILON);
            assert!((current.unwrap() - 0.65).abs() < f32::EPSILON);
        } else {
            panic!("Expected StateChange::Energy");
        }
    }

    #[test]
    fn to_state_changes_with_full_energy() {
        let json = r#"{
            "ENERGY": {
                "Power": 182,
                "Voltage": 224,
                "Current": 0.706,
                "ApparentPower": 195,
                "ReactivePower": 50,
                "Factor": 0.93,
                "Today": 1.5,
                "Yesterday": 2.3,
                "Total": 1104.315
            }
        }"#;
        let data: SensorData = serde_json::from_str(json).unwrap();

        let changes = data.to_state_changes();
        assert_eq!(changes.len(), 1);
        if let StateChange::Energy {
            power,
            voltage,
            current,
            apparent_power,
            reactive_power,
            power_factor,
            energy_today,
            energy_yesterday,
            energy_total,
            total_start_time,
        } = &changes[0]
        {
            assert!((power.unwrap() - 182.0).abs() < f32::EPSILON);
            assert!((voltage.unwrap() - 224.0).abs() < f32::EPSILON);
            assert!((current.unwrap() - 0.706).abs() < 0.001);
            assert!((apparent_power.unwrap() - 195.0).abs() < f32::EPSILON);
            assert!((reactive_power.unwrap() - 50.0).abs() < f32::EPSILON);
            assert!((power_factor.unwrap() - 0.93).abs() < 0.01);
            assert!((energy_today.unwrap() - 1.5).abs() < 0.01);
            assert!((energy_yesterday.unwrap() - 2.3).abs() < 0.01);
            assert!((energy_total.unwrap() - 1104.315).abs() < 0.01);
            assert!(total_start_time.is_none()); // Not in test JSON
        } else {
            panic!("Expected StateChange::Energy");
        }
    }

    #[test]
    fn to_state_changes_with_total_start_time() {
        let json = r#"{
            "ENERGY": {
                "TotalStartTime": "2024-01-15T10:30:00",
                "Power": 100,
                "Voltage": 230,
                "Current": 0.5,
                "Total": 500.0
            }
        }"#;
        let data: SensorData = serde_json::from_str(json).unwrap();

        let changes = data.to_state_changes();
        assert_eq!(changes.len(), 1);
        if let StateChange::Energy {
            total_start_time, ..
        } = &changes[0]
        {
            assert_eq!(total_start_time.as_deref(), Some("2024-01-15T10:30:00"));
        } else {
            panic!("Expected StateChange::Energy");
        }
    }
}
