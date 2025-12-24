// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Managed device with configuration and state tracking.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::device_model::DeviceModel;

/// Protocol type for device communication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Protocol {
    /// HTTP protocol
    Http,
    /// MQTT protocol
    Mqtt,
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http => write!(f, "HTTP"),
            Self::Mqtt => write!(f, "MQTT"),
        }
    }
}

/// Configuration for a managed device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    /// Unique identifier for the device
    pub id: Uuid,
    /// User-friendly name
    pub name: String,
    /// Device model
    pub model: DeviceModel,
    /// Communication protocol
    pub protocol: Protocol,
    /// HTTP host or MQTT broker URL
    pub host: String,
    /// MQTT topic (only for MQTT protocol)
    pub topic: Option<String>,
    /// Optional username for authentication
    pub username: Option<String>,
    /// Optional password for authentication
    pub password: Option<String>,
}

impl DeviceConfig {
    /// Creates a new device configuration with HTTP protocol.
    #[must_use]
    pub fn new_http(name: String, model: DeviceModel, host: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            model,
            protocol: Protocol::Http,
            host,
            topic: None,
            username: None,
            password: None,
        }
    }

    /// Creates a new device configuration with MQTT protocol.
    #[must_use]
    pub fn new_mqtt(name: String, model: DeviceModel, broker: String, topic: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            model,
            protocol: Protocol::Mqtt,
            host: broker,
            topic: Some(topic),
            username: None,
            password: None,
        }
    }

    /// Sets authentication credentials.
    #[must_use]
    pub fn with_credentials(mut self, username: String, password: String) -> Self {
        self.username = Some(username);
        self.password = Some(password);
        self
    }
}

/// Connection status of a device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// Not connected
    Disconnected,
    /// Attempting to connect
    Connecting,
    /// Connected and ready
    Connected,
    /// Connection error
    Error,
}

impl ConnectionStatus {
    /// Returns the display color for this status.
    #[must_use]
    pub const fn color(self) -> egui::Color32 {
        match self {
            Self::Disconnected => egui::Color32::GRAY,
            Self::Connecting => egui::Color32::YELLOW,
            Self::Connected => egui::Color32::GREEN,
            Self::Error => egui::Color32::RED,
        }
    }
}

/// Device state tracked by the application.
#[derive(Debug, Clone)]
pub struct DeviceState {
    /// Configuration
    pub config: DeviceConfig,
    /// Connection status
    pub status: ConnectionStatus,
    /// Whether the device is powered on
    pub power: Option<bool>,
    /// Dimmer level (0-100)
    pub dimmer: Option<u8>,
    /// HSB color: (hue 0-360, saturation 0-100, brightness 0-100)
    pub hsb_color: Option<(u16, u8, u8)>,
    /// Color temperature in mireds (153-500)
    pub color_temp: Option<u16>,
    /// Current power consumption in watts
    pub power_consumption: Option<u32>,
    /// Last error message
    pub error: Option<String>,
}

impl DeviceState {
    /// Creates a new device state from configuration.
    #[must_use]
    pub const fn new(config: DeviceConfig) -> Self {
        Self {
            config,
            status: ConnectionStatus::Disconnected,
            power: None,
            dimmer: None,
            hsb_color: None,
            color_temp: None,
            power_consumption: None,
            error: None,
        }
    }

    /// Returns the device model.
    #[must_use]
    pub const fn model(&self) -> DeviceModel {
        self.config.model
    }

    /// Returns the connection status.
    #[must_use]
    pub const fn status(&self) -> ConnectionStatus {
        self.status
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_http_device() {
        let config = DeviceConfig::new_http(
            "Test Bulb".to_string(),
            DeviceModel::AthomBulb5W7W,
            "192.168.1.100".to_string(),
        );

        assert_eq!(config.name, "Test Bulb");
        assert_eq!(config.model, DeviceModel::AthomBulb5W7W);
        assert_eq!(config.protocol, Protocol::Http);
        assert_eq!(config.host, "192.168.1.100");
        assert!(config.topic.is_none());
    }

    #[test]
    fn create_mqtt_device() {
        let config = DeviceConfig::new_mqtt(
            "Test Plug".to_string(),
            DeviceModel::NousA1T,
            "mqtt://192.168.1.50:1883".to_string(),
            "tasmota_plug".to_string(),
        );

        assert_eq!(config.name, "Test Plug");
        assert_eq!(config.model, DeviceModel::NousA1T);
        assert_eq!(config.protocol, Protocol::Mqtt);
        assert_eq!(config.host, "mqtt://192.168.1.50:1883");
        assert_eq!(config.topic, Some("tasmota_plug".to_string()));
    }

    #[test]
    fn device_with_credentials() {
        let config = DeviceConfig::new_http(
            "Test Device".to_string(),
            DeviceModel::AthomBulb5W7W,
            "192.168.1.100".to_string(),
        )
        .with_credentials("admin".to_string(), "password".to_string());

        assert_eq!(config.username, Some("admin".to_string()));
        assert_eq!(config.password, Some("password".to_string()));
    }

    #[test]
    fn connection_status_colors() {
        assert_eq!(ConnectionStatus::Disconnected.color(), egui::Color32::GRAY);
        assert_eq!(ConnectionStatus::Connecting.color(), egui::Color32::YELLOW);
        assert_eq!(ConnectionStatus::Connected.color(), egui::Color32::GREEN);
        assert_eq!(ConnectionStatus::Error.color(), egui::Color32::RED);
    }

    #[test]
    fn device_state_creation() {
        let config = DeviceConfig::new_http(
            "Test Bulb".to_string(),
            DeviceModel::AthomBulb5W7W,
            "192.168.1.100".to_string(),
        );

        let state = DeviceState::new(config);

        assert_eq!(state.config.name, "Test Bulb");
        assert_eq!(state.model(), DeviceModel::AthomBulb5W7W);
        assert_eq!(state.status(), ConnectionStatus::Disconnected);
        assert!(state.power.is_none());
        assert!(state.dimmer.is_none());
        assert!(state.power_consumption.is_none());
    }

    #[test]
    fn protocol_display() {
        assert_eq!(Protocol::Http.to_string(), "HTTP");
        assert_eq!(Protocol::Mqtt.to_string(), "MQTT");
    }
}
