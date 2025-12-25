// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Device manager wrapping the library's `DeviceManager`.
//!
//! This module provides a thin wrapper around `tasmor_lib::manager::DeviceManager`
//! that maintains the mapping between supervisor's `DeviceConfig` (with model info)
//! and the library's device management system.

use std::collections::HashMap;
use std::sync::Arc;

use tasmor_lib::event::DeviceId;
use tasmor_lib::manager::DeviceManager as LibraryDeviceManager;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use crate::device_config::{ConnectionStatus, DeviceConfig, ManagedDevice};

/// Device entry tracking supervisor config and library device ID.
struct DeviceEntry {
    /// Library's device ID (used for all library operations)
    device_id: DeviceId,
    /// Supervisor's managed device (with config and UI state)
    managed: ManagedDevice,
}

/// Manager for Tasmota devices wrapping the library's `DeviceManager`.
///
/// This wrapper maintains the mapping between supervisor's configuration
/// (with model info for UI) and the library's device management.
pub struct DeviceManager {
    /// The library's device manager (handles connections, pooling, events)
    library_manager: LibraryDeviceManager,
    /// Mapping from supervisor config ID to device entry
    devices: Arc<RwLock<HashMap<Uuid, DeviceEntry>>>,
}

impl DeviceManager {
    /// Creates a new device manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            library_manager: LibraryDeviceManager::new(),
            devices: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Subscribes to device events from the library.
    ///
    /// Returns a receiver for the library's rich `DeviceEvent` type which includes
    /// `StateChanged`, `ConnectionChanged`, `DeviceAdded`, and `DeviceRemoved` events.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<DeviceEvent> {
        self.library_manager.subscribe()
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

    /// Looks up the library `DeviceId` for a config ID.
    async fn get_device_id(&self, config_id: Uuid) -> Option<DeviceId> {
        self.devices
            .read()
            .await
            .get(&config_id)
            .map(|e| e.device_id)
    }

    /// Updates connection status for a device.
    pub async fn update_connection_status(&self, device_id: DeviceId, status: ConnectionStatus) {
        let mut devices = self.devices.write().await;
        for entry in devices.values_mut() {
            if entry.device_id == device_id {
                entry.managed.status = status;
                if status == ConnectionStatus::Connected {
                    entry.managed.error = None;
                }
                break;
            }
        }
    }

    /// Updates the full state for a device from a library event.
    pub async fn update_device_state(
        &self,
        device_id: DeviceId,
        new_state: tasmor_lib::state::DeviceState,
    ) {
        let mut devices = self.devices.write().await;
        for entry in devices.values_mut() {
            if entry.device_id == device_id {
                entry.managed.state = new_state;
                break;
            }
        }
    }

    /// Sets an error message for a device.
    pub async fn set_device_error(&self, device_id: DeviceId, error: Option<String>) {
        let mut devices = self.devices.write().await;
        for entry in devices.values_mut() {
            if entry.device_id == device_id {
                entry.managed.error.clone_from(&error);
                if error.is_some() {
                    entry.managed.status = ConnectionStatus::Error;
                }
                break;
            }
        }
    }

    // =========================================================================
    // Device Management
    // =========================================================================

    /// Adds a device to the manager.
    pub async fn add_device(&self, config: DeviceConfig) {
        let config_id = config.id;
        let lib_config = config.to_library_config();

        // Add to library manager (this generates the DeviceId)
        let device_id = self.library_manager.add_device(lib_config).await;

        // Create our managed device entry
        let mut managed = ManagedDevice::new(config);
        managed.id = device_id;

        let entry = DeviceEntry { device_id, managed };

        self.devices.write().await.insert(config_id, entry);
    }

    /// Removes a device from the manager.
    pub async fn remove_device(&self, config_id: Uuid) -> bool {
        let device_id = {
            let devices = self.devices.read().await;
            devices.get(&config_id).map(|e| e.device_id)
        };

        if let Some(device_id) = device_id {
            self.library_manager.remove_device(device_id).await;
            self.devices.write().await.remove(&config_id);
            true
        } else {
            false
        }
    }

    // =========================================================================
    // Connection Management
    // =========================================================================

    /// Connects to a device.
    pub async fn connect(&self, config_id: Uuid) -> Result<(), String> {
        // Set connecting status
        {
            let mut devices = self.devices.write().await;
            if let Some(entry) = devices.get_mut(&config_id) {
                entry.managed.status = ConnectionStatus::Connecting;
            }
        }

        let device_id = self
            .get_device_id(config_id)
            .await
            .ok_or("Device not found")?;

        match self.library_manager.connect(device_id).await {
            Ok(()) => {
                let mut devices = self.devices.write().await;
                if let Some(entry) = devices.get_mut(&config_id) {
                    entry.managed.status = ConnectionStatus::Connected;
                    entry.managed.error = None;
                }
                Ok(())
            }
            Err(e) => {
                let error_msg = e.to_string();
                let mut devices = self.devices.write().await;
                if let Some(entry) = devices.get_mut(&config_id) {
                    entry.managed.status = ConnectionStatus::Error;
                    entry.managed.error = Some(error_msg.clone());
                }
                Err(error_msg)
            }
        }
    }

    /// Disconnects from a device.
    pub async fn disconnect(&self, config_id: Uuid) -> Result<(), String> {
        let device_id = self
            .get_device_id(config_id)
            .await
            .ok_or("Device not found")?;

        self.library_manager
            .disconnect(device_id)
            .await
            .map_err(|e| e.to_string())?;

        let mut devices = self.devices.write().await;
        if let Some(entry) = devices.get_mut(&config_id) {
            entry.managed.status = ConnectionStatus::Disconnected;
            entry.managed.state.clear();
            entry.managed.error = None;
        }

        Ok(())
    }

    // =========================================================================
    // Device Commands
    // =========================================================================

    /// Toggles the power state.
    pub async fn toggle_power(&self, config_id: Uuid) -> Result<(), String> {
        let device_id = self
            .get_device_id(config_id)
            .await
            .ok_or("Device not found")?;

        self.library_manager
            .power_toggle(device_id)
            .await
            .map_err(|e| e.to_string())
    }

    /// Sets the dimmer level.
    pub async fn set_dimmer(&self, config_id: Uuid, level: u8) -> Result<(), String> {
        let device_id = self
            .get_device_id(config_id)
            .await
            .ok_or("Device not found")?;

        let dimmer = tasmor_lib::Dimmer::new(level).map_err(|e| e.to_string())?;

        self.library_manager
            .set_dimmer(device_id, dimmer)
            .await
            .map_err(|e| e.to_string())
    }

    /// Sets the HSB color.
    pub async fn set_hsb_color(
        &self,
        config_id: Uuid,
        hue: u16,
        sat: u8,
        bri: u8,
    ) -> Result<(), String> {
        let device_id = self
            .get_device_id(config_id)
            .await
            .ok_or("Device not found")?;

        let color = tasmor_lib::HsbColor::new(hue, sat, bri).map_err(|e| e.to_string())?;

        self.library_manager
            .set_hsb_color(device_id, color)
            .await
            .map_err(|e| e.to_string())
    }

    /// Sets the color temperature.
    pub async fn set_color_temp(&self, config_id: Uuid, ct: u16) -> Result<(), String> {
        let device_id = self
            .get_device_id(config_id)
            .await
            .ok_or("Device not found")?;

        let color_temp = tasmor_lib::ColorTemperature::new(ct).map_err(|e| e.to_string())?;

        self.library_manager
            .set_color_temperature(device_id, color_temp)
            .await
            .map_err(|e| e.to_string())
    }

    /// Resets the total energy counter.
    pub async fn reset_energy_total(&self, config_id: Uuid) -> Result<(), String> {
        let device_id = self
            .get_device_id(config_id)
            .await
            .ok_or("Device not found")?;

        self.library_manager
            .reset_energy_total(device_id)
            .await
            .map_err(|e| e.to_string())
    }

    /// Refreshes the device status by querying the library's current state.
    pub async fn refresh_status(&self, config_id: Uuid) -> Result<(), String> {
        let device_id = self
            .get_device_id(config_id)
            .await
            .ok_or("Device not found")?;

        // Get the current state from the library manager
        if let Some(state) = self.library_manager.get_state(device_id).await {
            let mut devices = self.devices.write().await;
            if let Some(entry) = devices.get_mut(&config_id) {
                entry.managed.state = state;
            }
        }

        Ok(())
    }

    /// Finds the config ID for a library device ID.
    ///
    /// Kept for potential future use (e.g., reverse lookup from library events).
    #[allow(dead_code)]
    pub async fn config_id_for_device(&self, device_id: DeviceId) -> Option<Uuid> {
        self.devices
            .read()
            .await
            .iter()
            .find(|(_, entry)| entry.device_id == device_id)
            .map(|(config_id, _)| *config_id)
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceManager {
    /// Shuts down the device manager, disconnecting all devices.
    ///
    /// This should be called when the application is closing to ensure
    /// proper cleanup of MQTT connections.
    pub async fn shutdown(&self) {
        tracing::info!("Shutting down device manager");

        // Get all config IDs
        let config_ids: Vec<Uuid> = self.devices.read().await.keys().copied().collect();

        // Disconnect all devices
        for config_id in config_ids {
            if let Err(e) = self.disconnect(config_id).await {
                tracing::warn!(config_id = %config_id, error = %e, "Failed to disconnect device during shutdown");
            }
        }

        // Clear all devices
        self.devices.write().await.clear();

        tracing::info!("Device manager shutdown complete");
    }
}

// Re-export library event types for convenience
pub use tasmor_lib::event::DeviceEvent;

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

        manager.add_device(config).await;

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

        manager.add_device(config).await;
        assert_eq!(manager.devices().await.len(), 1);

        let removed = manager.remove_device(id).await;
        assert!(removed);
        assert!(manager.devices().await.is_empty());
    }

    #[tokio::test]
    async fn subscribe_to_events() {
        let manager = DeviceManager::new();
        let mut event_rx = manager.subscribe();

        let config = DeviceConfig::new_http(
            "Test Bulb".to_string(),
            DeviceModel::AthomBulb5W7W,
            "192.168.1.100".to_string(),
        );

        manager.add_device(config).await;

        // Should receive DeviceAdded event from library
        let event = event_rx.try_recv();
        assert!(event.is_ok());
        assert!(matches!(event.unwrap(), DeviceEvent::DeviceAdded { .. }));
    }

    #[tokio::test]
    async fn config_id_mapping() {
        let manager = DeviceManager::new();

        let config = DeviceConfig::new_http(
            "Test Bulb".to_string(),
            DeviceModel::AthomBulb5W7W,
            "192.168.1.100".to_string(),
        );
        let config_id = config.id;

        manager.add_device(config).await;

        // Get the device ID from our devices
        let device_id = manager.get_device_id(config_id).await;
        assert!(device_id.is_some());

        // Verify reverse lookup works
        let found_config_id = manager.config_id_for_device(device_id.unwrap()).await;
        assert_eq!(found_config_id, Some(config_id));
    }
}
