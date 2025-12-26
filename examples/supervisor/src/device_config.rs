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
    /// Attempting to connect (reserved for async connection implementation)
    #[allow(dead_code)]
    Connecting,
    /// Connected and ready
    Connected,
    /// Connection error (reserved for error handling implementation)
    #[allow(dead_code)]
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

/// Managed device combining configuration and runtime state.
///
/// This struct separates persistent configuration from runtime state,
/// using the library's `DeviceState` for type-safe state tracking.
#[derive(Debug, Clone)]
pub struct ManagedDevice {
    /// Configuration (persistent)
    pub config: DeviceConfig,
    /// Connection status
    pub status: ConnectionStatus,
    /// Runtime state from library
    pub state: tasmor_lib::state::DeviceState,
    /// Last error message
    pub error: Option<String>,
}

impl ManagedDevice {
    /// Creates a new managed device from configuration.
    #[must_use]
    pub fn new(config: DeviceConfig) -> Self {
        Self {
            config,
            status: ConnectionStatus::Disconnected,
            state: tasmor_lib::state::DeviceState::new(),
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

    /// Returns a reference to the device state.
    /// Kept for future event-driven architecture integration.
    #[must_use]
    #[allow(dead_code)]
    pub const fn device_state(&self) -> &tasmor_lib::state::DeviceState {
        &self.state
    }

    /// Applies a state change from library events.
    ///
    /// Kept for direct state mutation when not using library events.
    #[allow(dead_code)]
    pub fn apply_state_change(&mut self, change: &tasmor_lib::state::StateChange) -> bool {
        self.state.apply(change)
    }

    /// Updates the full state from a library event.
    /// Kept for future event-driven architecture integration.
    #[allow(dead_code)]
    pub fn update_state(&mut self, new_state: tasmor_lib::state::DeviceState) {
        self.state = new_state;
    }

    /// Returns a mutable reference to the device state.
    /// Kept for direct state access when needed.
    #[allow(dead_code)]
    pub fn device_state_mut(&mut self) -> &mut tasmor_lib::state::DeviceState {
        &mut self.state
    }

    /// Clears all runtime state.
    ///
    /// Kept for manual state reset (e.g., on disconnect).
    #[allow(dead_code)]
    pub fn clear_state(&mut self) {
        self.state.clear();
    }

    // ========== Convenience getters for UI ==========

    /// Returns whether the device is powered on (checks POWER1).
    #[must_use]
    pub fn is_power_on(&self) -> Option<bool> {
        self.state.power(1).map(|p| p == tasmor_lib::PowerState::On)
    }

    /// Returns the dimmer value (0-100).
    #[must_use]
    pub fn dimmer_value(&self) -> Option<u8> {
        self.state.dimmer().map(|d| d.value())
    }

    /// Returns the HSB color as (hue, saturation, brightness).
    #[must_use]
    pub fn hsb_color_values(&self) -> Option<(u16, u8, u8)> {
        self.state
            .hsb_color()
            .map(|c| (c.hue(), c.saturation(), c.brightness()))
    }

    /// Returns the color temperature in mireds.
    #[must_use]
    pub fn color_temp_mireds(&self) -> Option<u16> {
        self.state.color_temperature().map(|c| c.value())
    }

    /// Returns the power consumption in watts.
    #[must_use]
    pub fn power_consumption_watts(&self) -> Option<f32> {
        self.state.power_consumption()
    }

    /// Returns the voltage in volts.
    #[must_use]
    pub fn voltage(&self) -> Option<f32> {
        self.state.voltage()
    }

    /// Returns the current in amperes.
    #[must_use]
    pub fn current(&self) -> Option<f32> {
        self.state.current()
    }

    /// Returns the apparent power in VA.
    #[must_use]
    pub fn apparent_power(&self) -> Option<f32> {
        self.state.apparent_power()
    }

    /// Returns the reactive power in `VAr`.
    #[must_use]
    pub fn reactive_power(&self) -> Option<f32> {
        self.state.reactive_power()
    }

    /// Returns the power factor (0-1).
    #[must_use]
    pub fn power_factor(&self) -> Option<f32> {
        self.state.power_factor()
    }

    /// Returns the energy consumed today in kWh.
    #[must_use]
    pub fn energy_today(&self) -> Option<f32> {
        self.state.energy_today()
    }

    /// Returns the energy consumed yesterday in kWh.
    #[must_use]
    pub fn energy_yesterday(&self) -> Option<f32> {
        self.state.energy_yesterday()
    }

    /// Returns the total energy consumed in kWh.
    #[must_use]
    pub fn energy_total(&self) -> Option<f32> {
        self.state.energy_total()
    }

    /// Returns the timestamp when total energy counting started.
    ///
    /// Returns a [`TasmotaDateTime`] which provides:
    /// - `naive()` for the datetime without timezone (always available)
    /// - `with_timezone()` for timezone-aware datetime (if timezone was known)
    #[must_use]
    pub fn total_start_time(&self) -> Option<&tasmor_lib::types::TasmotaDateTime> {
        self.state.total_start_time()
    }
}

// Keep backward compatibility alias during migration
pub type DeviceState = ManagedDevice;

// ============================================================================
// State Updates for Event-Driven Architecture
// ============================================================================

/// State update events sent from async callbacks to the UI thread.
///
/// This enables non-blocking, event-driven updates instead of polling.
#[derive(Debug, Clone)]
pub enum StateUpdate {
    /// A device's state changed (from MQTT callback or command response)
    StateChanged {
        device_id: uuid::Uuid,
        change: tasmor_lib::state::StateChange,
    },
    /// A device was added successfully (reserved for async device addition)
    #[allow(dead_code)]
    DeviceAdded(uuid::Uuid),
    /// A device was removed (reserved for async device removal)
    #[allow(dead_code)]
    DeviceRemoved(uuid::Uuid),
    /// A device's connection status changed (reserved for connection monitoring)
    #[allow(dead_code)]
    ConnectionChanged {
        device_id: uuid::Uuid,
        status: ConnectionStatus,
        error: Option<String>,
    },
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
        assert_eq!(ConnectionStatus::Connected.color(), egui::Color32::GREEN);
    }

    #[test]
    fn managed_device_creation() {
        let config = DeviceConfig::new_http(
            "Test Bulb".to_string(),
            DeviceModel::AthomBulb5W7W,
            "192.168.1.100".to_string(),
        );

        let device = ManagedDevice::new(config);

        assert_eq!(device.config.name, "Test Bulb");
        assert_eq!(device.model(), DeviceModel::AthomBulb5W7W);
        assert_eq!(device.status(), ConnectionStatus::Disconnected);
        // Library's DeviceState starts empty
        assert!(device.device_state().power(1).is_none());
        assert!(device.device_state().dimmer().is_none());
        assert!(device.device_state().power_consumption().is_none());
    }

    #[test]
    fn managed_device_state_changes() {
        use tasmor_lib::state::StateChange;
        use tasmor_lib::PowerState;

        let config = DeviceConfig::new_http(
            "Test Bulb".to_string(),
            DeviceModel::AthomBulb5W7W,
            "192.168.1.100".to_string(),
        );

        let mut device = ManagedDevice::new(config);

        // Apply a power change
        let change = StateChange::Power {
            index: 1,
            state: PowerState::On,
        };
        assert!(device.apply_state_change(&change));
        assert_eq!(device.device_state().power(1), Some(PowerState::On));

        // Applying same change returns false
        assert!(!device.apply_state_change(&change));
    }

    #[test]
    fn protocol_display() {
        assert_eq!(Protocol::Http.to_string(), "HTTP");
        assert_eq!(Protocol::Mqtt.to_string(), "MQTT");
    }
}
