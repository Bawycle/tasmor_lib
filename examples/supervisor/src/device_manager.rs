// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Device manager handling communication with Tasmota devices.
//!
//! This module manages persistent connections to Tasmota devices and handles
//! all communication through a command/event pattern.

use std::collections::HashMap;
use std::sync::Arc;

use tasmor_lib::protocol::{HttpClient, MqttClient};
use tasmor_lib::Device;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use crate::device_config::{ConnectionStatus, DeviceConfig, DeviceState, Protocol};

/// Commands sent to the device manager.
#[derive(Debug)]
pub enum DeviceCommand {
    /// Add a new device
    AddDevice(DeviceConfig),
    /// Remove a device by ID
    RemoveDevice(Uuid),
    /// Connect to a device
    Connect(Uuid),
    /// Disconnect from a device
    Disconnect(Uuid),
    /// Toggle device power
    TogglePower(Uuid),
    /// Set dimmer level (0-100)
    SetDimmer(Uuid, u8),
    /// Set HSB color (hue 0-360, saturation 0-100, brightness 0-100)
    SetHsbColor(Uuid, u16, u8, u8),
    /// Set color temperature in mireds (153-500)
    SetColorTemp(Uuid, u16),
    /// Refresh device status
    RefreshStatus(Uuid),
}

/// Events emitted by the device manager.
#[derive(Debug, Clone)]
pub enum DeviceEvent {
    /// Device added
    DeviceAdded,
    /// Device removed
    DeviceRemoved,
    /// Device state updated
    StateUpdated,
    /// Error occurred
    Error(String),
}

/// Connected device handle - either HTTP or MQTT.
enum ConnectedDevice {
    Http(Device<HttpClient>),
    Mqtt(Device<MqttClient>),
}

/// Internal managed device with state and optional connection.
struct ManagedDevice {
    state: DeviceState,
    connection: Option<ConnectedDevice>,
}

impl ManagedDevice {
    fn new(config: DeviceConfig) -> Self {
        Self {
            state: DeviceState::new(config),
            connection: None,
        }
    }
}

/// Manager for Tasmota devices with async communication.
pub struct DeviceManager {
    devices: Arc<RwLock<HashMap<Uuid, ManagedDevice>>>,
    command_tx: mpsc::UnboundedSender<DeviceCommand>,
    event_rx: Arc<RwLock<mpsc::UnboundedReceiver<DeviceEvent>>>,
}

impl DeviceManager {
    /// Creates a new device manager.
    #[must_use]
    pub fn new() -> Self {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let devices = Arc::new(RwLock::new(HashMap::new()));

        let manager = Self {
            devices: Arc::clone(&devices),
            command_tx,
            event_rx: Arc::new(RwLock::new(event_rx)),
        };

        // Spawn background task to process commands
        tokio::spawn(Self::process_commands(
            command_rx,
            event_tx,
            Arc::clone(&devices),
        ));

        manager
    }

    /// Sends a command to the device manager.
    ///
    /// # Errors
    ///
    /// Returns error if the command cannot be sent.
    pub fn send_command(&self, command: DeviceCommand) -> Result<(), String> {
        self.command_tx
            .send(command)
            .map_err(|e| format!("Failed to send command: {e}"))
    }

    /// Gets a snapshot of all devices.
    pub async fn devices(&self) -> Vec<DeviceState> {
        self.devices
            .read()
            .await
            .values()
            .map(|m| m.state.clone())
            .collect()
    }

    /// Polls for the next event (non-blocking).
    pub async fn poll_event(&self) -> Option<DeviceEvent> {
        self.event_rx.write().await.try_recv().ok()
    }

    /// Background task processing commands.
    async fn process_commands(
        mut command_rx: mpsc::UnboundedReceiver<DeviceCommand>,
        event_tx: mpsc::UnboundedSender<DeviceEvent>,
        devices: Arc<RwLock<HashMap<Uuid, ManagedDevice>>>,
    ) {
        while let Some(command) = command_rx.recv().await {
            match command {
                DeviceCommand::AddDevice(config) => {
                    let id = config.id;
                    let managed = ManagedDevice::new(config);
                    devices.write().await.insert(id, managed);
                    let _ = event_tx.send(DeviceEvent::DeviceAdded);
                    let _ = event_tx.send(DeviceEvent::StateUpdated);
                }

                DeviceCommand::RemoveDevice(id) => {
                    // Connection will be dropped automatically
                    if devices.write().await.remove(&id).is_some() {
                        let _ = event_tx.send(DeviceEvent::DeviceRemoved);
                    }
                }

                DeviceCommand::Connect(id) => {
                    Self::handle_connect(id, &devices, &event_tx).await;
                }

                DeviceCommand::Disconnect(id) => {
                    Self::handle_disconnect(id, &devices, &event_tx).await;
                }

                DeviceCommand::TogglePower(id) => {
                    Self::handle_toggle_power(id, &devices, &event_tx).await;
                }

                DeviceCommand::SetDimmer(id, level) => {
                    Self::handle_set_dimmer(id, level, &devices, &event_tx).await;
                }

                DeviceCommand::SetHsbColor(id, hue, sat, bri) => {
                    Self::handle_set_hsb_color(id, hue, sat, bri, &devices, &event_tx).await;
                }

                DeviceCommand::SetColorTemp(id, ct) => {
                    Self::handle_set_color_temp(id, ct, &devices, &event_tx).await;
                }

                DeviceCommand::RefreshStatus(id) => {
                    Self::handle_refresh_status(id, &devices, &event_tx).await;
                }
            }
        }
    }

    /// Handles connect command - establishes persistent connection.
    async fn handle_connect(
        id: Uuid,
        devices: &Arc<RwLock<HashMap<Uuid, ManagedDevice>>>,
        event_tx: &mpsc::UnboundedSender<DeviceEvent>,
    ) {
        // Get config and set connecting status
        let config = {
            let mut guard = devices.write().await;
            if let Some(managed) = guard.get_mut(&id) {
                managed.state.status = ConnectionStatus::Connecting;
                let _ = event_tx.send(DeviceEvent::StateUpdated);
                Some(managed.state.config.clone())
            } else {
                None
            }
        };

        let Some(config) = config else {
            return;
        };

        // Try to establish connection
        let result = match config.protocol {
            Protocol::Http => Self::create_http_device(&config).map(ConnectedDevice::Http),
            Protocol::Mqtt => Self::create_mqtt_device(&config)
                .await
                .map(ConnectedDevice::Mqtt),
        };

        // Update state based on result
        let mut guard = devices.write().await;
        if let Some(managed) = guard.get_mut(&id) {
            match result {
                Ok(connection) => {
                    managed.connection = Some(connection);
                    managed.state.status = ConnectionStatus::Connected;
                    managed.state.error = None;
                    let _ = event_tx.send(DeviceEvent::StateUpdated);

                    // Drop the lock before refreshing status
                    drop(guard);

                    // Refresh status after successful connection
                    Self::handle_refresh_status(id, devices, event_tx).await;
                }
                Err(e) => {
                    managed.connection = None;
                    managed.state.status = ConnectionStatus::Error;
                    managed.state.error = Some(e.clone());
                    let _ = event_tx.send(DeviceEvent::StateUpdated);
                    let _ = event_tx.send(DeviceEvent::Error(e));
                }
            }
        }
    }

    /// Handles disconnect command.
    async fn handle_disconnect(
        id: Uuid,
        devices: &Arc<RwLock<HashMap<Uuid, ManagedDevice>>>,
        event_tx: &mpsc::UnboundedSender<DeviceEvent>,
    ) {
        let mut guard = devices.write().await;
        if let Some(managed) = guard.get_mut(&id) {
            // Drop the connection
            managed.connection = None;
            managed.state.status = ConnectionStatus::Disconnected;
            managed.state.power = None;
            managed.state.dimmer = None;
            managed.state.hsb_color = None;
            managed.state.color_temp = None;
            managed.state.power_consumption = None;
            managed.state.error = None;
            let _ = event_tx.send(DeviceEvent::StateUpdated);
        }
    }

    /// Handles toggle power using existing connection.
    async fn handle_toggle_power(
        id: Uuid,
        devices: &Arc<RwLock<HashMap<Uuid, ManagedDevice>>>,
        event_tx: &mpsc::UnboundedSender<DeviceEvent>,
    ) {
        let result = {
            let guard = devices.read().await;
            let Some(managed) = guard.get(&id) else {
                return;
            };
            let Some(conn) = &managed.connection else {
                let _ = event_tx.send(DeviceEvent::Error("Device not connected".to_string()));
                return;
            };

            match conn {
                ConnectedDevice::Http(device) => device.power_toggle().await,
                ConnectedDevice::Mqtt(device) => device.power_toggle().await,
            }
        };

        match result {
            Ok(response) => {
                if let Ok(power_state) = response.first_power_state() {
                    let power = power_state == tasmor_lib::PowerState::On;
                    let mut guard = devices.write().await;
                    if let Some(managed) = guard.get_mut(&id) {
                        managed.state.power = Some(power);
                        let _ = event_tx.send(DeviceEvent::StateUpdated);
                    }
                }
            }
            Err(e) => {
                Self::handle_connection_error(id, devices, event_tx, &e.to_string()).await;
            }
        }
    }

    /// Handles set dimmer using existing connection.
    async fn handle_set_dimmer(
        id: Uuid,
        level: u8,
        devices: &Arc<RwLock<HashMap<Uuid, ManagedDevice>>>,
        event_tx: &mpsc::UnboundedSender<DeviceEvent>,
    ) {
        let result = {
            let guard = devices.read().await;
            let Some(managed) = guard.get(&id) else {
                return;
            };
            let Some(conn) = &managed.connection else {
                let _ = event_tx.send(DeviceEvent::Error("Device not connected".to_string()));
                return;
            };

            let Ok(dimmer) = tasmor_lib::Dimmer::new(level) else {
                let _ = event_tx.send(DeviceEvent::Error("Invalid dimmer value".to_string()));
                return;
            };

            match conn {
                ConnectedDevice::Http(device) => device.set_dimmer(dimmer).await,
                ConnectedDevice::Mqtt(device) => device.set_dimmer(dimmer).await,
            }
        };

        match result {
            Ok(response) => {
                let mut guard = devices.write().await;
                if let Some(managed) = guard.get_mut(&id) {
                    managed.state.dimmer = Some(level);
                    // Tasmota includes power state in dimmer response - update it too
                    if let Some(power) = parse_power_from_response(&response.body) {
                        managed.state.power = Some(power);
                    }
                    let _ = event_tx.send(DeviceEvent::StateUpdated);
                }
            }
            Err(e) => {
                Self::handle_connection_error(id, devices, event_tx, &e.to_string()).await;
            }
        }
    }

    /// Handles set HSB color using existing connection.
    async fn handle_set_hsb_color(
        id: Uuid,
        hue: u16,
        sat: u8,
        bri: u8,
        devices: &Arc<RwLock<HashMap<Uuid, ManagedDevice>>>,
        event_tx: &mpsc::UnboundedSender<DeviceEvent>,
    ) {
        let result = {
            let guard = devices.read().await;
            let Some(managed) = guard.get(&id) else {
                return;
            };
            let Some(conn) = &managed.connection else {
                let _ = event_tx.send(DeviceEvent::Error("Device not connected".to_string()));
                return;
            };

            let Ok(color) = tasmor_lib::HsbColor::new(hue, sat, bri) else {
                let _ = event_tx.send(DeviceEvent::Error("Invalid color value".to_string()));
                return;
            };

            match conn {
                ConnectedDevice::Http(device) => device.set_hsb_color(color).await,
                ConnectedDevice::Mqtt(device) => device.set_hsb_color(color).await,
            }
        };

        match result {
            Ok(response) => {
                let mut guard = devices.write().await;
                if let Some(managed) = guard.get_mut(&id) {
                    managed.state.hsb_color = Some((hue, sat, bri));
                    // Tasmota includes power state in color response - update it too
                    if let Some(power) = parse_power_from_response(&response.body) {
                        managed.state.power = Some(power);
                    }
                    let _ = event_tx.send(DeviceEvent::StateUpdated);
                }
            }
            Err(e) => {
                Self::handle_connection_error(id, devices, event_tx, &e.to_string()).await;
            }
        }
    }

    /// Handles set color temperature using existing connection.
    async fn handle_set_color_temp(
        id: Uuid,
        ct: u16,
        devices: &Arc<RwLock<HashMap<Uuid, ManagedDevice>>>,
        event_tx: &mpsc::UnboundedSender<DeviceEvent>,
    ) {
        let result = {
            let guard = devices.read().await;
            let Some(managed) = guard.get(&id) else {
                return;
            };
            let Some(conn) = &managed.connection else {
                let _ = event_tx.send(DeviceEvent::Error("Device not connected".to_string()));
                return;
            };

            let Ok(color_temp) = tasmor_lib::ColorTemp::new(ct) else {
                let _ = event_tx.send(DeviceEvent::Error("Invalid color temperature".to_string()));
                return;
            };

            match conn {
                ConnectedDevice::Http(device) => device.set_color_temp(color_temp).await,
                ConnectedDevice::Mqtt(device) => device.set_color_temp(color_temp).await,
            }
        };

        match result {
            Ok(response) => {
                let mut guard = devices.write().await;
                if let Some(managed) = guard.get_mut(&id) {
                    managed.state.color_temp = Some(ct);
                    // Tasmota may include power state in color temp response - update it too
                    if let Some(power) = parse_power_from_response(&response.body) {
                        managed.state.power = Some(power);
                    }
                    let _ = event_tx.send(DeviceEvent::StateUpdated);
                }
            }
            Err(e) => {
                Self::handle_connection_error(id, devices, event_tx, &e.to_string()).await;
            }
        }
    }

    /// Handles refresh status using existing connection.
    async fn handle_refresh_status(
        id: Uuid,
        devices: &Arc<RwLock<HashMap<Uuid, ManagedDevice>>>,
        event_tx: &mpsc::UnboundedSender<DeviceEvent>,
    ) {
        // Get connection info
        let (supports_dimming, supports_color, supports_color_temp, supports_energy) = {
            let guard = devices.read().await;
            let Some(managed) = guard.get(&id) else {
                return;
            };
            if managed.connection.is_none() {
                let _ = event_tx.send(DeviceEvent::Error("Device not connected".to_string()));
                return;
            }
            let model = managed.state.config.model;
            (
                model.supports_dimming(),
                model.supports_color(),
                model.capabilities().color_temp,
                model.supports_energy_monitoring(),
            )
        };

        // Get power state
        let power_result = {
            let guard = devices.read().await;
            let Some(managed) = guard.get(&id) else {
                return;
            };
            let Some(conn) = &managed.connection else {
                return;
            };
            match conn {
                ConnectedDevice::Http(device) => device.get_power().await,
                ConnectedDevice::Mqtt(device) => device.get_power().await,
            }
        };

        let power = match power_result {
            Ok(response) => response
                .first_power_state()
                .ok()
                .map(|s| s == tasmor_lib::PowerState::On),
            Err(e) => {
                Self::handle_connection_error(id, devices, event_tx, &e.to_string()).await;
                return;
            }
        };

        // Get dimmer if supported
        let dimmer = if supports_dimming {
            let guard = devices.read().await;
            let Some(managed) = guard.get(&id) else {
                return;
            };
            let Some(conn) = &managed.connection else {
                return;
            };
            let result = match conn {
                ConnectedDevice::Http(device) => device.get_dimmer().await,
                ConnectedDevice::Mqtt(device) => device.get_dimmer().await,
            };
            result.ok().and_then(|r| parse_dimmer_response(&r.body))
        } else {
            None
        };

        // Get HSB color if supported
        let hsb_color = if supports_color {
            let guard = devices.read().await;
            let Some(managed) = guard.get(&id) else {
                return;
            };
            let Some(conn) = &managed.connection else {
                return;
            };
            let result = match conn {
                ConnectedDevice::Http(device) => device.get_hsb_color().await,
                ConnectedDevice::Mqtt(device) => device.get_hsb_color().await,
            };
            result.ok().and_then(|r| parse_hsb_color_response(&r.body))
        } else {
            None
        };

        // Get color temperature if supported
        let color_temp = if supports_color_temp {
            let guard = devices.read().await;
            let Some(managed) = guard.get(&id) else {
                return;
            };
            let Some(conn) = &managed.connection else {
                return;
            };
            let result = match conn {
                ConnectedDevice::Http(device) => device.get_color_temp().await,
                ConnectedDevice::Mqtt(device) => device.get_color_temp().await,
            };
            result.ok().and_then(|r| parse_color_temp_response(&r.body))
        } else {
            None
        };

        // Get energy if supported
        let power_consumption = if supports_energy {
            let guard = devices.read().await;
            let Some(managed) = guard.get(&id) else {
                return;
            };
            let Some(conn) = &managed.connection else {
                return;
            };
            let result = match conn {
                ConnectedDevice::Http(device) => device.energy().await,
                ConnectedDevice::Mqtt(device) => device.energy().await,
            };
            result.ok().and_then(|e| e.power())
        } else {
            None
        };

        // Update state
        let mut guard = devices.write().await;
        if let Some(managed) = guard.get_mut(&id) {
            managed.state.power = power;
            managed.state.dimmer = dimmer;
            managed.state.hsb_color = hsb_color;
            managed.state.color_temp = color_temp;
            managed.state.power_consumption = power_consumption;
            let _ = event_tx.send(DeviceEvent::StateUpdated);
        }
    }

    /// Handles connection errors by updating device state.
    async fn handle_connection_error(
        id: Uuid,
        devices: &Arc<RwLock<HashMap<Uuid, ManagedDevice>>>,
        event_tx: &mpsc::UnboundedSender<DeviceEvent>,
        error: &str,
    ) {
        let mut guard = devices.write().await;
        if let Some(managed) = guard.get_mut(&id) {
            // Mark as error but keep connection for retry
            managed.state.status = ConnectionStatus::Error;
            managed.state.error = Some(error.to_string());
            let _ = event_tx.send(DeviceEvent::StateUpdated);
            let _ = event_tx.send(DeviceEvent::Error(error.to_string()));
        }
    }

    /// Creates an HTTP device connection.
    fn create_http_device(config: &DeviceConfig) -> Result<Device<HttpClient>, String> {
        let mut builder =
            Device::http(&config.host).with_capabilities(config.model.capabilities());

        if let (Some(username), Some(password)) = (&config.username, &config.password) {
            builder = builder.with_credentials(username, password);
        }

        builder
            .build_without_probe()
            .map_err(|e| format!("HTTP connection failed: {e}"))
    }

    /// Creates an MQTT device connection.
    async fn create_mqtt_device(config: &DeviceConfig) -> Result<Device<MqttClient>, String> {
        let topic = config.topic.as_ref().ok_or("MQTT topic not configured")?;

        let mut builder =
            Device::mqtt(&config.host, topic).with_capabilities(config.model.capabilities());

        if let (Some(username), Some(password)) = (&config.username, &config.password) {
            builder = builder.with_credentials(username, password);
        }

        let device = builder
            .build_without_probe()
            .await
            .map_err(|e| format!("MQTT connection failed: {e}"))?;

        // Test the connection by querying power state
        device
            .get_power()
            .await
            .map_err(|e| format!("MQTT connection test failed: {e}"))?;

        Ok(device)
    }
}

/// Parses the power state from a JSON response.
/// Tasmota often includes POWER in responses to other commands (like Dimmer).
fn parse_power_from_response(body: &str) -> Option<bool> {
    #[derive(serde::Deserialize)]
    struct PowerResponse {
        #[serde(rename = "POWER")]
        power: String,
    }

    serde_json::from_str::<PowerResponse>(body)
        .ok()
        .map(|r| r.power == "ON")
}

/// Parses the dimmer response JSON to extract the dimmer value.
fn parse_dimmer_response(body: &str) -> Option<u8> {
    #[derive(serde::Deserialize)]
    struct DimmerResponse {
        #[serde(rename = "Dimmer")]
        dimmer: u8,
    }

    serde_json::from_str::<DimmerResponse>(body)
        .ok()
        .map(|r| r.dimmer)
}

/// Parses the HSB color response JSON to extract the color values.
fn parse_hsb_color_response(body: &str) -> Option<(u16, u8, u8)> {
    #[derive(serde::Deserialize)]
    struct HsbColorResponse {
        #[serde(rename = "HSBColor")]
        hsb_color: String,
    }

    serde_json::from_str::<HsbColorResponse>(body)
        .ok()
        .and_then(|r| {
            let parts: Vec<&str> = r.hsb_color.split(',').collect();
            if parts.len() == 3 {
                let hue = parts[0].parse().ok()?;
                let sat = parts[1].parse().ok()?;
                let bri = parts[2].parse().ok()?;
                Some((hue, sat, bri))
            } else {
                None
            }
        })
}

/// Parses the color temperature response JSON to extract the CT value.
fn parse_color_temp_response(body: &str) -> Option<u16> {
    #[derive(serde::Deserialize)]
    struct ColorTempResponse {
        #[serde(rename = "CT")]
        ct: u16,
    }

    serde_json::from_str::<ColorTempResponse>(body)
        .ok()
        .map(|r| r.ct)
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_model::DeviceModel;

    #[tokio::test]
    async fn create_manager() {
        let manager = DeviceManager::new();
        let devices = manager.devices().await;
        assert!(devices.is_empty());
    }

    #[tokio::test]
    async fn add_device() {
        let manager = DeviceManager::new();

        let config = DeviceConfig::new_http(
            "Test Bulb".to_string(),
            DeviceModel::AthomBulb5W7W,
            "192.168.1.100".to_string(),
        );

        manager
            .send_command(DeviceCommand::AddDevice(config))
            .unwrap();

        // Wait for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let devices = manager.devices().await;
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].config.name, "Test Bulb");
    }

    #[tokio::test]
    async fn remove_device() {
        let manager = DeviceManager::new();

        let config = DeviceConfig::new_http(
            "Test Bulb".to_string(),
            DeviceModel::AthomBulb5W7W,
            "192.168.1.100".to_string(),
        );
        let id = config.id;

        manager
            .send_command(DeviceCommand::AddDevice(config))
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        manager
            .send_command(DeviceCommand::RemoveDevice(id))
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let devices = manager.devices().await;
        assert!(devices.is_empty());
    }

    #[tokio::test]
    async fn poll_events() {
        let manager = DeviceManager::new();

        let config = DeviceConfig::new_http(
            "Test Bulb".to_string(),
            DeviceModel::AthomBulb5W7W,
            "192.168.1.100".to_string(),
        );

        manager
            .send_command(DeviceCommand::AddDevice(config))
            .unwrap();

        // Wait and check for events
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let event = manager.poll_event().await;
        assert!(event.is_some());

        if let Some(DeviceEvent::DeviceAdded) = event {
            // Event received successfully
        } else {
            panic!("Expected DeviceAdded event");
        }
    }
}
