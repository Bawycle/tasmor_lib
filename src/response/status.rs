// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Status response parsing.

use serde::{Deserialize, Deserializer};

/// Deserializes a value that can be either a number or a string representation of a number.
/// This is needed because Tasmota sometimes returns numeric values as strings.
fn deserialize_string_or_number<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct StringOrNumber;

    impl Visitor<'_> for StringOrNumber {
        type Value = u8;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a number or a string containing a number")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            u8::try_from(value).map_err(|_| E::custom(format!("u8 out of range: {value}")))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            u8::try_from(value).map_err(|_| E::custom(format!("u8 out of range: {value}")))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse::<u8>().map_err(de::Error::custom)
        }
    }

    deserializer.deserialize_any(StringOrNumber)
}

/// Optional version that handles missing or null values.
fn deserialize_string_or_number_opt<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_string_or_number(deserializer).or(Ok(0))
}

/// Deserializes an i8 value that can be either a number or a string representation.
fn deserialize_string_or_number_i8<'de, D>(deserializer: D) -> Result<i8, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct StringOrNumberI8;

    impl Visitor<'_> for StringOrNumberI8 {
        type Value = i8;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a number or a string containing a number")
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            i8::try_from(value).map_err(|_| E::custom(format!("i8 out of range: {value}")))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            i8::try_from(value).map_err(|_| E::custom(format!("i8 out of range: {value}")))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse::<i8>().map_err(de::Error::custom)
        }
    }

    deserializer.deserialize_any(StringOrNumberI8)
}

/// Optional version for i8 that handles missing or null values.
fn deserialize_string_or_number_i8_opt<'de, D>(deserializer: D) -> Result<i8, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_string_or_number_i8(deserializer).or(Ok(0))
}

/// Deserializes a u16 value that can be either a number or a string representation.
fn deserialize_string_or_number_u16<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct StringOrNumberU16;

    impl Visitor<'_> for StringOrNumberU16 {
        type Value = u16;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a number or a string containing a number")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            u16::try_from(value).map_err(|_| E::custom(format!("u16 out of range: {value}")))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            u16::try_from(value).map_err(|_| E::custom(format!("u16 out of range: {value}")))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse::<u16>().map_err(de::Error::custom)
        }
    }

    deserializer.deserialize_any(StringOrNumberU16)
}

/// Optional version for u16.
fn deserialize_string_or_number_u16_opt<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_string_or_number_u16(deserializer).or(Ok(0))
}

/// Deserializes a u32 value that can be either a number or a string representation.
fn deserialize_string_or_number_u32<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct StringOrNumberU32;

    impl Visitor<'_> for StringOrNumberU32 {
        type Value = u32;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a number or a string containing a number")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            u32::try_from(value).map_err(|_| E::custom(format!("u32 out of range: {value}")))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            u32::try_from(value).map_err(|_| E::custom(format!("u32 out of range: {value}")))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse::<u32>().map_err(de::Error::custom)
        }
    }

    deserializer.deserialize_any(StringOrNumberU32)
}

/// Optional version for u32.
fn deserialize_string_or_number_u32_opt<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_string_or_number_u32(deserializer).or(Ok(0))
}

/// Complete status response from `Status 0`.
///
/// Contains all device status information in a single response.
///
/// # Examples
///
/// ```
/// use tasmor_lib::response::StatusResponse;
///
/// let json = r#"{
///     "Status": {"Module": 18, "DeviceName": "Tasmota", "FriendlyName": ["Light"]},
///     "StatusFWR": {"Version": "13.1.0", "BuildDateTime": "2024-01-01T00:00:00"},
///     "StatusNET": {"Hostname": "tasmota", "IPAddress": "192.168.1.100"}
/// }"#;
/// let response: StatusResponse = serde_json::from_str(json).unwrap();
/// assert_eq!(response.status.as_ref().unwrap().module, 18);
/// ```
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct StatusResponse {
    /// Device parameters (Status 1).
    #[serde(rename = "Status")]
    pub status: Option<StatusDeviceParameters>,

    /// Parameter settings (Status PRM).
    #[serde(rename = "StatusPRM")]
    pub status_prm: Option<StatusParameters>,

    /// Firmware information (Status 2).
    #[serde(rename = "StatusFWR")]
    pub firmware: Option<StatusFirmware>,

    /// Logging settings (Status 3).
    #[serde(rename = "StatusLOG")]
    pub logging: Option<StatusLogging>,

    /// Memory information (Status 4).
    #[serde(rename = "StatusMEM")]
    pub memory: Option<StatusMemory>,

    /// Network information (Status 5).
    #[serde(rename = "StatusNET")]
    pub network: Option<StatusNetwork>,

    /// MQTT configuration (Status 6).
    #[serde(rename = "StatusMQT")]
    pub mqtt: Option<StatusMqtt>,

    /// Time information (Status 7).
    #[serde(rename = "StatusTIM")]
    pub time: Option<StatusTime>,

    /// Sensor data (Status 10).
    #[serde(rename = "StatusSNS")]
    pub sensors: Option<serde_json::Value>,

    /// Power thresholds (Status 9).
    #[serde(rename = "StatusPTH")]
    pub power_thresholds: Option<serde_json::Value>,

    /// State information (Status 11 / runtime state).
    #[serde(rename = "StatusSTS")]
    pub sensor_status: Option<serde_json::Value>,
}

impl StatusResponse {
    /// Returns the device module ID.
    #[must_use]
    pub fn module_id(&self) -> Option<u8> {
        self.status.as_ref().map(|s| s.module)
    }

    /// Returns the device name.
    #[must_use]
    pub fn device_name(&self) -> Option<&str> {
        self.status.as_ref().map(|s| s.device_name.as_str())
    }

    /// Returns the firmware version.
    #[must_use]
    pub fn firmware_version(&self) -> Option<&str> {
        self.firmware.as_ref().map(|f| f.version.as_str())
    }

    /// Returns the IP address.
    #[must_use]
    pub fn ip_address(&self) -> Option<&str> {
        self.network.as_ref().map(|n| n.ip_address.as_str())
    }

    /// Returns the hostname.
    #[must_use]
    pub fn hostname(&self) -> Option<&str> {
        self.network.as_ref().map(|n| n.hostname.as_str())
    }
}

/// Device parameters from Status 1.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StatusDeviceParameters {
    /// Module ID (e.g., 18 for Generic, 49 for Neo Coolcam).
    #[serde(default, deserialize_with = "deserialize_string_or_number_opt")]
    pub module: u8,

    /// Device name.
    #[serde(default)]
    pub device_name: String,

    /// Friendly names for each relay.
    #[serde(default)]
    pub friendly_name: Vec<String>,

    /// Topic for MQTT.
    #[serde(default)]
    pub topic: String,

    /// Button topic.
    #[serde(default)]
    pub button_topic: String,

    /// Power state on startup (0=Off, 1=On, 2=Toggle, 3=Last).
    #[serde(default, deserialize_with = "deserialize_string_or_number_opt")]
    pub power: u8,

    /// Power retention flag.
    #[serde(default, deserialize_with = "deserialize_string_or_number_opt")]
    pub power_retain: u8,

    /// LED state.
    #[serde(
        default,
        rename = "LedState",
        deserialize_with = "deserialize_string_or_number_opt"
    )]
    pub led_state: u8,
}

/// Parameter settings from Status PRM.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StatusParameters {
    /// Baudrate for serial communication.
    #[serde(default, deserialize_with = "deserialize_string_or_number_u32_opt")]
    pub baudrate: u32,

    /// Serial configuration.
    #[serde(default)]
    pub serial_config: String,

    /// Group topic.
    #[serde(default)]
    pub group_topic: String,

    /// OTA URL.
    #[serde(default, rename = "OtaUrl")]
    pub ota_url: String,

    /// Restart reason.
    #[serde(default)]
    pub restart_reason: String,

    /// Uptime.
    #[serde(default)]
    pub uptime: String,

    /// Boot count.
    #[serde(default, deserialize_with = "deserialize_string_or_number_u32_opt")]
    pub boot_count: u32,
}

/// Firmware information from Status 2.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StatusFirmware {
    /// Firmware version string.
    #[serde(default)]
    pub version: String,

    /// Build date and time.
    #[serde(default)]
    pub build_date_time: String,

    /// Boot version.
    #[serde(default, deserialize_with = "deserialize_string_or_number_opt")]
    pub boot: u8,

    /// Core version.
    #[serde(default)]
    pub core: String,

    /// SDK version.
    #[serde(default, rename = "SDK")]
    pub sdk: String,

    /// CPU frequency in MHz.
    #[serde(
        default,
        rename = "CpuFrequency",
        deserialize_with = "deserialize_string_or_number_u16_opt"
    )]
    pub cpu_frequency: u16,

    /// Hardware identifier.
    #[serde(default)]
    pub hardware: String,
}

/// Logging settings from Status 3.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StatusLogging {
    /// Serial log level.
    #[serde(default, deserialize_with = "deserialize_string_or_number_opt")]
    pub serial_log: u8,

    /// Web log level.
    #[serde(default, deserialize_with = "deserialize_string_or_number_opt")]
    pub web_log: u8,

    /// MQTT log level.
    #[serde(default, deserialize_with = "deserialize_string_or_number_opt")]
    pub mqtt_log: u8,

    /// Syslog level.
    #[serde(default, deserialize_with = "deserialize_string_or_number_opt")]
    pub sys_log: u8,

    /// Syslog host.
    #[serde(default)]
    pub log_host: String,

    /// Syslog port.
    #[serde(default, deserialize_with = "deserialize_string_or_number_u16_opt")]
    pub log_port: u16,

    /// Telemetry period in seconds.
    #[serde(
        default,
        rename = "TelePeriod",
        deserialize_with = "deserialize_string_or_number_u16_opt"
    )]
    pub tele_period: u16,
}

/// Memory information from Status 4.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StatusMemory {
    /// Program size in KB.
    #[serde(default, deserialize_with = "deserialize_string_or_number_u32_opt")]
    pub program_size: u32,

    /// Free program space in KB.
    #[serde(default, deserialize_with = "deserialize_string_or_number_u32_opt")]
    pub free: u32,

    /// Heap size in bytes.
    #[serde(default, deserialize_with = "deserialize_string_or_number_u32_opt")]
    pub heap: u32,

    /// Program flash size in KB.
    #[serde(
        default,
        rename = "ProgramFlashSize",
        deserialize_with = "deserialize_string_or_number_u32_opt"
    )]
    pub program_flash_size: u32,

    /// Flash size in KB.
    #[serde(default, deserialize_with = "deserialize_string_or_number_u32_opt")]
    pub flash_size: u32,

    /// Flash chip ID.
    #[serde(default, rename = "FlashChipId")]
    pub flash_chip_id: String,

    /// Flash mode.
    #[serde(default, deserialize_with = "deserialize_string_or_number_opt")]
    pub flash_mode: u8,

    /// Features list.
    #[serde(default)]
    pub features: Vec<String>,
}

/// Network information from Status 5.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StatusNetwork {
    /// Hostname.
    #[serde(default)]
    pub hostname: String,

    /// IP address.
    #[serde(default, rename = "IPAddress")]
    pub ip_address: String,

    /// Gateway address.
    #[serde(default)]
    pub gateway: String,

    /// Subnet mask.
    #[serde(default, rename = "Subnetmask")]
    pub subnet_mask: String,

    /// DNS server.
    #[serde(default, rename = "DNSServer1")]
    pub dns_server: String,

    /// MAC address.
    #[serde(default)]
    pub mac: String,

    /// Wi-Fi SSID.
    #[serde(default, rename = "SSId")]
    pub ssid: String,

    /// Wi-Fi BSSID.
    #[serde(default, rename = "BSSId")]
    pub bssid: String,

    /// Wi-Fi channel.
    #[serde(default, deserialize_with = "deserialize_string_or_number_opt")]
    pub channel: u8,

    /// Wi-Fi mode.
    #[serde(default)]
    pub mode: String,

    /// Wi-Fi RSSI.
    #[serde(
        default,
        rename = "RSSI",
        deserialize_with = "deserialize_string_or_number_i8_opt"
    )]
    pub rssi: i8,

    /// Wi-Fi signal strength.
    #[serde(default, deserialize_with = "deserialize_string_or_number_i8_opt")]
    pub signal: i8,

    /// Link count.
    #[serde(default, deserialize_with = "deserialize_string_or_number_u32_opt")]
    pub link_count: u32,

    /// Downtime.
    #[serde(default)]
    pub downtime: String,
}

/// MQTT configuration from Status 6.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StatusMqtt {
    /// MQTT host.
    #[serde(default, rename = "MqttHost")]
    pub host: String,

    /// MQTT port.
    #[serde(
        default,
        rename = "MqttPort",
        deserialize_with = "deserialize_string_or_number_u16_opt"
    )]
    pub port: u16,

    /// MQTT client ID mask.
    #[serde(default, rename = "MqttClientMask")]
    pub client_mask: String,

    /// MQTT client ID.
    #[serde(default, rename = "MqttClient")]
    pub client: String,

    /// MQTT user.
    #[serde(default, rename = "MqttUser")]
    pub user: String,

    /// MQTT count.
    #[serde(
        default,
        rename = "MqttCount",
        deserialize_with = "deserialize_string_or_number_u32_opt"
    )]
    pub count: u32,

    /// `MAX_PACKET_SIZE` configuration.
    #[serde(
        default,
        rename = "MAX_PACKET_SIZE",
        deserialize_with = "deserialize_string_or_number_u32_opt"
    )]
    pub max_packet_size: u32,

    /// KEEPALIVE.
    #[serde(
        default,
        rename = "KEEPALIVE",
        deserialize_with = "deserialize_string_or_number_u16_opt"
    )]
    pub keepalive: u16,

    /// Socket timeout.
    #[serde(
        default,
        rename = "SOCKET_TIMEOUT",
        deserialize_with = "deserialize_string_or_number_opt"
    )]
    pub socket_timeout: u8,
}

/// Time information from Status 7.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StatusTime {
    /// UTC time.
    #[serde(default, rename = "UTC")]
    pub utc: String,

    /// Local time.
    #[serde(default)]
    pub local: String,

    /// Start daylight saving time.
    #[serde(default, rename = "StartDST")]
    pub start_dst: String,

    /// End daylight saving time.
    #[serde(default, rename = "EndDST")]
    pub end_dst: String,

    /// Timezone.
    #[serde(default)]
    pub timezone: String,

    /// Sunrise time.
    #[serde(default)]
    pub sunrise: String,

    /// Sunset time.
    #[serde(default)]
    pub sunset: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_status_response() {
        let json = r#"{
            "Status": {
                "Module": 18,
                "DeviceName": "Tasmota",
                "FriendlyName": ["Light"],
                "Topic": "tasmota",
                "ButtonTopic": "0",
                "Power": 1,
                "PowerRetain": 0,
                "LedState": 0
            }
        }"#;

        let response: StatusResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.module_id(), Some(18));
        assert_eq!(response.device_name(), Some("Tasmota"));
    }

    #[test]
    fn parse_firmware_info() {
        let json = r#"{
            "StatusFWR": {
                "Version": "13.1.0",
                "BuildDateTime": "2024-01-01T00:00:00",
                "Boot": 7,
                "Core": "3.0.2",
                "SDK": "2.2.2",
                "CpuFrequency": 80,
                "Hardware": "ESP8266"
            }
        }"#;

        let response: StatusResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.firmware_version(), Some("13.1.0"));
    }

    #[test]
    fn parse_network_info() {
        let json = r#"{
            "StatusNET": {
                "Hostname": "tasmota-device",
                "IPAddress": "192.168.1.100",
                "Gateway": "192.168.1.1",
                "Subnetmask": "255.255.255.0",
                "DNSServer1": "192.168.1.1",
                "Mac": "AA:BB:CC:DD:EE:FF",
                "SSId": "MyNetwork",
                "Channel": 6,
                "RSSI": -50,
                "Signal": 100
            }
        }"#;

        let response: StatusResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.ip_address(), Some("192.168.1.100"));
        assert_eq!(response.hostname(), Some("tasmota-device"));
    }

    #[test]
    fn parse_mqtt_info() {
        let json = r#"{
            "StatusMQT": {
                "MqttHost": "192.168.1.50",
                "MqttPort": 1883,
                "MqttClient": "tasmota_123456",
                "MqttUser": "mqtt_user",
                "MqttCount": 1,
                "MAX_PACKET_SIZE": 1200,
                "KEEPALIVE": 30
            }
        }"#;

        let response: StatusResponse = serde_json::from_str(json).unwrap();
        let mqtt = response.mqtt.unwrap();
        assert_eq!(mqtt.host, "192.168.1.50");
        assert_eq!(mqtt.port, 1883);
    }
}
