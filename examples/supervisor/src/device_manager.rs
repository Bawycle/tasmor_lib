// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Device manager using the library's device-centric API.
//!
//! This module provides device management using individual `Device` instances.
//! State updates are handled through explicit status queries.

use std::collections::HashMap;
use std::sync::Arc;

use tasmor_lib::protocol::{HttpClient, MqttClient};
use tasmor_lib::{Device, PowerState};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::device_config::{ConnectionStatus, DeviceConfig, ManagedDevice, Protocol};

/// Wrapper for different device types.
enum DeviceHandle {
    Http(Device<HttpClient>),
    Mqtt(Device<MqttClient>),
}

/// Device entry tracking the device handle and managed device state.
struct DeviceEntry {
    /// The actual device (HTTP or MQTT)
    handle: DeviceHandle,
    /// Supervisor's managed device (with config and UI state)
    managed: ManagedDevice,
}

/// Manager for Tasmota devices using the library's device-centric API.
///
/// This manager creates and stores individual devices directly.
/// State updates are refreshed through explicit status queries.
pub struct DeviceManager {
    /// Mapping from config ID to device entry
    devices: Arc<RwLock<HashMap<Uuid, DeviceEntry>>>,
}

impl DeviceManager {
    /// Creates a new device manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            devices: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Gets a snapshot of all managed devices for UI display.
    pub async fn devices(&self) -> Vec<ManagedDevice> {
        self.devices
            .read()
            .await
            .values()
            .map(|entry| entry.managed.clone())
            .collect()
    }

    // =========================================================================
    // Device Management
    // =========================================================================

    /// Adds a device to the manager.
    ///
    /// The device is created and connected automatically based on its protocol:
    /// - **HTTP**: Ready immediately (stateless)
    /// - **MQTT**: Connection established to broker
    ///
    /// # Errors
    ///
    /// Returns an error if the device creation fails.
    pub async fn add_device(&self, config: DeviceConfig) -> Result<(), String> {
        let config_id = config.id;
        let capabilities = config.model.capabilities();

        let handle = match config.protocol {
            Protocol::Http => {
                let mut builder = Device::http(&config.host).with_capabilities(capabilities);

                if let (Some(user), Some(pass)) = (&config.username, &config.password) {
                    builder = builder.with_credentials(user, pass);
                }

                let device = builder.build_without_probe().map_err(|e| e.to_string())?;
                DeviceHandle::Http(device)
            }
            Protocol::Mqtt => {
                let topic = config.topic.as_deref().unwrap_or("tasmota");
                let mut builder = Device::mqtt(&config.host, topic).with_capabilities(capabilities);

                if let (Some(user), Some(pass)) = (&config.username, &config.password) {
                    builder = builder.with_credentials(user, pass);
                }

                let device = builder
                    .build_without_probe()
                    .await
                    .map_err(|e| e.to_string())?;
                DeviceHandle::Mqtt(device)
            }
        };

        // Create managed device entry
        let mut managed = ManagedDevice::new(config);
        managed.status = ConnectionStatus::Connected;

        let entry = DeviceEntry { handle, managed };
        self.devices.write().await.insert(config_id, entry);

        Ok(())
    }

    /// Removes a device from the manager.
    pub async fn remove_device(&self, config_id: Uuid) -> bool {
        self.devices.write().await.remove(&config_id).is_some()
    }

    // =========================================================================
    // Connection Management
    // =========================================================================

    /// Connects to a device.
    ///
    /// For HTTP devices, this is a no-op (stateless).
    /// For MQTT devices, the connection is already established on add.
    pub async fn connect(&self, config_id: Uuid) -> Result<(), String> {
        let mut devices = self.devices.write().await;
        if let Some(entry) = devices.get_mut(&config_id) {
            entry.managed.status = ConnectionStatus::Connected;
            entry.managed.error = None;
            Ok(())
        } else {
            Err("Device not found".to_string())
        }
    }

    /// Disconnects from a device.
    pub async fn disconnect(&self, config_id: Uuid) -> Result<(), String> {
        let mut devices = self.devices.write().await;
        if let Some(entry) = devices.get_mut(&config_id) {
            entry.managed.status = ConnectionStatus::Disconnected;
            entry.managed.state.clear();
            entry.managed.error = None;
            Ok(())
        } else {
            Err("Device not found".to_string())
        }
    }

    // =========================================================================
    // Device Commands
    // =========================================================================

    /// Toggles the power state.
    pub async fn toggle_power(&self, config_id: Uuid) -> Result<(), String> {
        let devices = self.devices.read().await;
        let entry = devices.get(&config_id).ok_or("Device not found")?;

        match &entry.handle {
            DeviceHandle::Http(device) => {
                device.power_toggle().await.map_err(|e| e.to_string())?;
            }
            DeviceHandle::Mqtt(device) => {
                device.power_toggle().await.map_err(|e| e.to_string())?;
            }
        }

        Ok(())
    }

    /// Turns the power on.
    pub async fn power_on(&self, config_id: Uuid) -> Result<(), String> {
        let devices = self.devices.read().await;
        let entry = devices.get(&config_id).ok_or("Device not found")?;

        match &entry.handle {
            DeviceHandle::Http(device) => {
                device.power_on().await.map_err(|e| e.to_string())?;
            }
            DeviceHandle::Mqtt(device) => {
                device.power_on().await.map_err(|e| e.to_string())?;
            }
        }

        // Update local state for HTTP devices
        drop(devices);
        let mut devices = self.devices.write().await;
        if let Some(entry) = devices.get_mut(&config_id) {
            if matches!(entry.handle, DeviceHandle::Http(_)) {
                entry.managed.state.set_power(1, PowerState::On);
            }
        }

        Ok(())
    }

    /// Turns the power off.
    pub async fn power_off(&self, config_id: Uuid) -> Result<(), String> {
        let devices = self.devices.read().await;
        let entry = devices.get(&config_id).ok_or("Device not found")?;

        match &entry.handle {
            DeviceHandle::Http(device) => {
                device.power_off().await.map_err(|e| e.to_string())?;
            }
            DeviceHandle::Mqtt(device) => {
                device.power_off().await.map_err(|e| e.to_string())?;
            }
        }

        // Update local state for HTTP devices
        drop(devices);
        let mut devices = self.devices.write().await;
        if let Some(entry) = devices.get_mut(&config_id) {
            if matches!(entry.handle, DeviceHandle::Http(_)) {
                entry.managed.state.set_power(1, PowerState::Off);
            }
        }

        Ok(())
    }

    /// Sets the dimmer level.
    pub async fn set_dimmer(&self, config_id: Uuid, level: u8) -> Result<(), String> {
        let dimmer = tasmor_lib::Dimmer::new(level).map_err(|e| e.to_string())?;

        let devices = self.devices.read().await;
        let entry = devices.get(&config_id).ok_or("Device not found")?;

        match &entry.handle {
            DeviceHandle::Http(device) => {
                device.set_dimmer(dimmer).await.map_err(|e| e.to_string())?;
            }
            DeviceHandle::Mqtt(device) => {
                device.set_dimmer(dimmer).await.map_err(|e| e.to_string())?;
            }
        }

        // Update local state for HTTP devices
        drop(devices);
        let mut devices = self.devices.write().await;
        if let Some(entry) = devices.get_mut(&config_id) {
            if matches!(entry.handle, DeviceHandle::Http(_)) {
                entry.managed.state.set_dimmer(dimmer);
            }
        }

        Ok(())
    }

    /// Sets the HSB color.
    pub async fn set_hsb_color(
        &self,
        config_id: Uuid,
        hue: u16,
        sat: u8,
        bri: u8,
    ) -> Result<(), String> {
        let color = tasmor_lib::HsbColor::new(hue, sat, bri).map_err(|e| e.to_string())?;

        let devices = self.devices.read().await;
        let entry = devices.get(&config_id).ok_or("Device not found")?;

        match &entry.handle {
            DeviceHandle::Http(device) => {
                device
                    .set_hsb_color(color)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            DeviceHandle::Mqtt(device) => {
                device
                    .set_hsb_color(color)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }

        // Update local state for HTTP devices
        drop(devices);
        let mut devices = self.devices.write().await;
        if let Some(entry) = devices.get_mut(&config_id) {
            if matches!(entry.handle, DeviceHandle::Http(_)) {
                entry.managed.state.set_hsb_color(color);
            }
        }

        Ok(())
    }

    /// Sets the color temperature.
    pub async fn set_color_temperature(&self, config_id: Uuid, ct: u16) -> Result<(), String> {
        let color_temp = tasmor_lib::ColorTemperature::new(ct).map_err(|e| e.to_string())?;

        let devices = self.devices.read().await;
        let entry = devices.get(&config_id).ok_or("Device not found")?;

        match &entry.handle {
            DeviceHandle::Http(device) => {
                device
                    .set_color_temperature(color_temp)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            DeviceHandle::Mqtt(device) => {
                device
                    .set_color_temperature(color_temp)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }

        // Update local state for HTTP devices
        drop(devices);
        let mut devices = self.devices.write().await;
        if let Some(entry) = devices.get_mut(&config_id) {
            if matches!(entry.handle, DeviceHandle::Http(_)) {
                entry.managed.state.set_color_temperature(color_temp);
            }
        }

        Ok(())
    }

    /// Resets the total energy counter by querying energy data.
    ///
    /// Note: The library doesn't have a direct reset method, so this
    /// just refreshes the energy data.
    pub async fn reset_energy_total(&self, config_id: Uuid) -> Result<(), String> {
        // Query energy to refresh data (actual reset not implemented in library)
        let devices = self.devices.read().await;
        let entry = devices.get(&config_id).ok_or("Device not found")?;

        match &entry.handle {
            DeviceHandle::Http(device) => {
                device.energy().await.map_err(|e| e.to_string())?;
            }
            DeviceHandle::Mqtt(device) => {
                device.energy().await.map_err(|e| e.to_string())?;
            }
        }

        Ok(())
    }

    /// Queries the device status by fetching individual state values.
    pub async fn query_status(
        &self,
        config_id: Uuid,
    ) -> Result<tasmor_lib::state::DeviceState, String> {
        let devices = self.devices.read().await;
        let entry = devices.get(&config_id).ok_or("Device not found")?;

        let mut state = tasmor_lib::state::DeviceState::new();

        // Query power state
        let power_result = match &entry.handle {
            DeviceHandle::Http(device) => device.get_power().await,
            DeviceHandle::Mqtt(device) => device.get_power().await,
        };
        if let Ok(power_response) = power_result {
            if let Ok(power_state) = power_response.first_power_state() {
                state.set_power(1, power_state);
            }
        }

        // Query dimmer if supported
        let dimmer_result = match &entry.handle {
            DeviceHandle::Http(device) => device.get_dimmer().await,
            DeviceHandle::Mqtt(device) => device.get_dimmer().await,
        };
        if let Ok(dimmer_response) = dimmer_result {
            if let Ok(dimmer) = tasmor_lib::Dimmer::new(dimmer_response.dimmer()) {
                state.set_dimmer(dimmer);
            }
        }

        // Query color temperature if supported
        let ct_result = match &entry.handle {
            DeviceHandle::Http(device) => device.get_color_temperature().await,
            DeviceHandle::Mqtt(device) => device.get_color_temperature().await,
        };
        if let Ok(ct_response) = ct_result {
            if let Ok(ct) = tasmor_lib::ColorTemperature::new(ct_response.color_temperature()) {
                state.set_color_temperature(ct);
            }
        }

        // Query HSB color if supported
        let hsb_result = match &entry.handle {
            DeviceHandle::Http(device) => device.get_hsb_color().await,
            DeviceHandle::Mqtt(device) => device.get_hsb_color().await,
        };
        if let Ok(hsb_response) = hsb_result {
            if let Ok(hsb) = hsb_response.hsb_color() {
                state.set_hsb_color(hsb);
            }
        }

        // Update our local copy
        drop(devices);
        let mut devices = self.devices.write().await;
        if let Some(entry) = devices.get_mut(&config_id) {
            entry.managed.state = state.clone();
        }

        Ok(state)
    }

    /// Shuts down the device manager.
    pub async fn shutdown(&self) {
        tracing::info!("Shutting down device manager");
        self.devices.write().await.clear();
        tracing::info!("Device manager shutdown complete");
    }
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
    async fn add_http_device() {
        let manager = DeviceManager::new();

        let config = DeviceConfig::new_http(
            "Test Bulb".to_string(),
            DeviceModel::AthomBulb5W7W,
            "192.168.1.100".to_string(),
        );

        manager.add_device(config).await.unwrap();

        let devices = manager.devices().await;
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].config.name, "Test Bulb");
        assert_eq!(devices[0].status, ConnectionStatus::Connected);
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

        manager.add_device(config).await.unwrap();
        assert_eq!(manager.devices().await.len(), 1);

        let removed = manager.remove_device(id).await;
        assert!(removed);
        assert!(manager.devices().await.is_empty());
    }
}
