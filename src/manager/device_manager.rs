// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Device manager for coordinating multiple Tasmota devices.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{RwLock, broadcast, watch};

use crate::Capabilities;
use crate::command::{DimmerCommand, HsbColorCommand, PowerCommand};
use crate::error::Error;
use crate::event::{DeviceEvent, DeviceId, EventBus};
use crate::protocol::PooledMqttClient;
use crate::state::{DeviceState, StateChange};
use crate::types::{ColorTemp, Dimmer, HsbColor, PowerIndex};

use super::device_config::{ConnectionConfig, DeviceConfig, ReconnectionPolicy};
use super::managed_device::{ConnectionState, DeviceClient, ManagedDevice};

/// Manager for coordinating multiple Tasmota devices.
///
/// The `DeviceManager` provides a centralized way to manage multiple Tasmota
/// devices, handling connection pooling, state management, and event distribution.
///
/// # Features
///
/// - **Connection Pooling**: MQTT connections are shared between devices on the same broker
/// - **State Management**: Device state is tracked and updated automatically
/// - **Event System**: Subscribe to device events via broadcast channels
/// - **Auto-Reconnection**: Configurable automatic reconnection on connection loss
///
/// # Examples
///
/// ```ignore
/// use tasmor_lib::manager::{DeviceManager, DeviceConfig};
///
/// #[tokio::main]
/// async fn main() -> tasmor_lib::Result<()> {
///     let manager = DeviceManager::new();
///
///     // Subscribe to events
///     let mut events = manager.subscribe();
///     tokio::spawn(async move {
///         while let Ok(event) = events.recv().await {
///             println!("Event: {:?}", event);
///         }
///     });
///
///     // Add and connect a device
///     let config = DeviceConfig::mqtt("mqtt://192.168.1.50:1883", "tasmota_bulb");
///     let device_id = manager.add_device(config).await?;
///     manager.connect(device_id).await?;
///
///     // Control the device
///     manager.power_on(device_id).await?;
///     manager.set_dimmer(device_id, tasmor_lib::Dimmer::new(75)?).await?;
///
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct DeviceManager {
    /// Managed devices, keyed by device ID.
    devices: Arc<RwLock<HashMap<DeviceId, ManagedDevice>>>,
    /// Event bus for broadcasting device events.
    event_bus: EventBus,
    /// Default reconnection policy for new devices.
    default_reconnection: ReconnectionPolicy,
}

impl DeviceManager {
    /// Creates a new device manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            devices: Arc::new(RwLock::new(HashMap::new())),
            event_bus: EventBus::new(),
            default_reconnection: ReconnectionPolicy::default(),
        }
    }

    /// Creates a new device manager with custom event bus capacity.
    #[must_use]
    pub fn with_capacity(event_capacity: usize) -> Self {
        Self {
            devices: Arc::new(RwLock::new(HashMap::new())),
            event_bus: EventBus::with_capacity(event_capacity),
            default_reconnection: ReconnectionPolicy::default(),
        }
    }

    /// Sets the default reconnection policy for new devices.
    #[must_use]
    pub fn with_reconnection_policy(mut self, policy: ReconnectionPolicy) -> Self {
        self.default_reconnection = policy;
        self
    }

    // =========================================================================
    // Subscription
    // =========================================================================

    /// Subscribes to device events.
    ///
    /// Returns a receiver that will receive all events for all managed devices.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::manager::DeviceManager;
    /// use tasmor_lib::event::DeviceEvent;
    ///
    /// let manager = DeviceManager::new();
    /// let mut events = manager.subscribe();
    ///
    /// // In a task:
    /// // while let Ok(event) = events.recv().await {
    /// //     match event {
    /// //         DeviceEvent::StateChanged { device_id, change, .. } => { ... }
    /// //         _ => {}
    /// //     }
    /// // }
    /// ```
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<DeviceEvent> {
        self.event_bus.subscribe()
    }

    /// Returns the number of active event subscribers.
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        self.event_bus.subscriber_count()
    }

    // =========================================================================
    // Device Management
    // =========================================================================

    /// Adds a device to the manager.
    ///
    /// The device is not connected automatically. Call [`connect`](Self::connect)
    /// to establish the connection.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use tasmor_lib::manager::{DeviceManager, DeviceConfig};
    ///
    /// let manager = DeviceManager::new();
    /// let config = DeviceConfig::mqtt("mqtt://localhost:1883", "tasmota_bulb");
    /// let device_id = manager.add_device(config).await;
    /// ```
    pub async fn add_device(&self, config: DeviceConfig) -> DeviceId {
        let device = ManagedDevice::new(config);
        let device_id = device.id();

        self.devices.write().await.insert(device_id, device);

        self.event_bus.publish(DeviceEvent::device_added(device_id));

        device_id
    }

    /// Removes a device from the manager.
    ///
    /// This will disconnect the device if it is connected.
    ///
    /// # Returns
    ///
    /// Returns `true` if the device was found and removed, `false` otherwise.
    pub async fn remove_device(&self, device_id: DeviceId) -> bool {
        let removed = self.devices.write().await.remove(&device_id).is_some();

        if removed {
            self.event_bus
                .publish(DeviceEvent::device_removed(device_id));
        }

        removed
    }

    /// Returns a list of all device IDs.
    pub async fn device_ids(&self) -> Vec<DeviceId> {
        self.devices.read().await.keys().copied().collect()
    }

    /// Returns the number of managed devices.
    pub async fn device_count(&self) -> usize {
        self.devices.read().await.len()
    }

    /// Returns the capabilities for a device.
    pub async fn capabilities(&self, device_id: DeviceId) -> Option<Capabilities> {
        self.devices
            .read()
            .await
            .get(&device_id)
            .map(|d| d.capabilities.clone())
    }

    /// Returns the friendly name for a device.
    pub async fn friendly_name(&self, device_id: DeviceId) -> Option<String> {
        self.devices
            .read()
            .await
            .get(&device_id)
            .map(|d| d.display_name().to_string())
    }

    // =========================================================================
    // Connection Management
    // =========================================================================

    /// Connects to a device.
    ///
    /// # Errors
    ///
    /// Returns an error if the device is not found or connection fails.
    pub async fn connect(&self, device_id: DeviceId) -> Result<(), Error> {
        let config = {
            let mut devices = self.devices.write().await;
            let device = devices.get_mut(&device_id).ok_or(Error::DeviceNotFound)?;

            if device.is_connected() {
                return Ok(()); // Already connected
            }

            device.set_connection_state(ConnectionState::Connecting);
            device.config.clone()
        };

        // Attempt connection
        let result = self.create_client(&config).await;

        let mut devices = self.devices.write().await;
        let device = devices.get_mut(&device_id).ok_or(Error::DeviceNotFound)?;

        match result {
            Ok(client) => {
                device.set_client(client);
                drop(devices);
                self.event_bus.publish(DeviceEvent::connected(device_id));
                Ok(())
            }
            Err(e) => {
                let error_msg = e.to_string();
                device.set_connection_state(ConnectionState::Failed(error_msg.clone()));
                drop(devices);
                self.event_bus
                    .publish(DeviceEvent::disconnected_with_error(device_id, error_msg));
                Err(e)
            }
        }
    }

    /// Disconnects from a device.
    ///
    /// # Errors
    ///
    /// Returns an error if the device is not found.
    pub async fn disconnect(&self, device_id: DeviceId) -> Result<(), Error> {
        let mut devices = self.devices.write().await;
        let device = devices.get_mut(&device_id).ok_or(Error::DeviceNotFound)?;

        device.clear_client();
        drop(devices);

        self.event_bus.publish(DeviceEvent::disconnected(device_id));

        Ok(())
    }

    /// Returns true if the device is connected.
    pub async fn is_connected(&self, device_id: DeviceId) -> bool {
        self.devices
            .read()
            .await
            .get(&device_id)
            .is_some_and(ManagedDevice::is_connected)
    }

    /// Creates a client for the given configuration.
    async fn create_client(&self, config: &DeviceConfig) -> Result<DeviceClient, Error> {
        match &config.connection {
            ConnectionConfig::Mqtt {
                broker_url,
                topic,
                credentials,
            } => {
                let creds = credentials.as_ref().map(|(u, p)| (u.as_str(), p.as_str()));
                let client = PooledMqttClient::connect(broker_url, topic, creds)
                    .await
                    .map_err(Error::Protocol)?;
                Ok(DeviceClient::Mqtt(client))
            }
            ConnectionConfig::Http {
                host,
                port,
                credentials,
                use_https,
            } => {
                // Construct URL with scheme and port
                let scheme = if *use_https { "https" } else { "http" };
                let url = if *port == 80 && !*use_https || *port == 443 && *use_https {
                    format!("{scheme}://{host}")
                } else {
                    format!("{scheme}://{host}:{port}")
                };

                let mut builder = crate::Device::http(&url);
                if let Some((user, pass)) = credentials {
                    builder = builder.with_credentials(user, pass);
                }
                // Build without probe for now (we may probe on demand)
                let device = builder.build_without_probe()?;
                Ok(DeviceClient::Http(device))
            }
        }
    }

    // =========================================================================
    // State Management
    // =========================================================================

    /// Returns the current state of a device.
    pub async fn get_state(&self, device_id: DeviceId) -> Option<DeviceState> {
        self.devices
            .read()
            .await
            .get(&device_id)
            .map(|d| d.state.clone())
    }

    /// Creates a watch receiver for a device's state.
    ///
    /// The receiver will be notified whenever the device state changes.
    pub async fn watch_device(&self, device_id: DeviceId) -> Option<watch::Receiver<DeviceState>> {
        self.devices
            .read()
            .await
            .get(&device_id)
            .map(ManagedDevice::watch_state)
    }

    // =========================================================================
    // Power Commands
    // =========================================================================

    /// Turns on the device power.
    ///
    /// # Errors
    ///
    /// Returns an error if the device is not found or not connected.
    pub async fn power_on(&self, device_id: DeviceId) -> Result<(), Error> {
        self.power_on_index(device_id, PowerIndex::default()).await
    }

    /// Turns on a specific power output.
    ///
    /// # Errors
    ///
    /// Returns an error if the device is not found or not connected.
    pub async fn power_on_index(
        &self,
        device_id: DeviceId,
        index: PowerIndex,
    ) -> Result<(), Error> {
        self.send_power_command(device_id, index, PowerCommand::on(index))
            .await
    }

    /// Turns off the device power.
    ///
    /// # Errors
    ///
    /// Returns an error if the device is not found or not connected.
    pub async fn power_off(&self, device_id: DeviceId) -> Result<(), Error> {
        self.power_off_index(device_id, PowerIndex::default()).await
    }

    /// Turns off a specific power output.
    ///
    /// # Errors
    ///
    /// Returns an error if the device is not found or not connected.
    pub async fn power_off_index(
        &self,
        device_id: DeviceId,
        index: PowerIndex,
    ) -> Result<(), Error> {
        self.send_power_command(device_id, index, PowerCommand::off(index))
            .await
    }

    /// Toggles the device power.
    ///
    /// # Errors
    ///
    /// Returns an error if the device is not found or not connected.
    pub async fn power_toggle(&self, device_id: DeviceId) -> Result<(), Error> {
        self.power_toggle_index(device_id, PowerIndex::default())
            .await
    }

    /// Toggles a specific power output.
    ///
    /// # Errors
    ///
    /// Returns an error if the device is not found or not connected.
    pub async fn power_toggle_index(
        &self,
        device_id: DeviceId,
        index: PowerIndex,
    ) -> Result<(), Error> {
        self.send_power_command(device_id, index, PowerCommand::toggle(index))
            .await
    }

    /// Sends a power command and updates state.
    async fn send_power_command(
        &self,
        device_id: DeviceId,
        index: PowerIndex,
        command: PowerCommand,
    ) -> Result<(), Error> {
        let response = self.send_command(device_id, &command).await?;

        // Parse response and update state
        let power_response: crate::response::PowerResponse = response.parse()?;
        if let Some(state) = power_response.power_state(index.value())? {
            let change = StateChange::Power {
                index: index.value(),
                state,
            };
            self.apply_state_change(device_id, change).await;
        }

        Ok(())
    }

    // =========================================================================
    // Light Commands
    // =========================================================================

    /// Sets the dimmer value.
    ///
    /// # Errors
    ///
    /// Returns an error if the device is not found, not connected, or doesn't
    /// have dimmer capability.
    pub async fn set_dimmer(&self, device_id: DeviceId, value: Dimmer) -> Result<(), Error> {
        self.check_capability(device_id, |c| c.dimmer).await?;

        let command = DimmerCommand::set(value);
        let response = self.send_command(device_id, &command).await?;

        // Parse response and update state
        let dimmer_response: crate::response::DimmerResponse = response.parse()?;
        let change = StateChange::Dimmer(Dimmer::clamped(dimmer_response.dimmer()));
        self.apply_state_change(device_id, change).await;

        // Also update power state if returned
        if let Some(power_state) = dimmer_response.power_state()? {
            let power_change = StateChange::Power {
                index: 0,
                state: power_state,
            };
            self.apply_state_change(device_id, power_change).await;
        }

        Ok(())
    }

    /// Sets the HSB color.
    ///
    /// # Errors
    ///
    /// Returns an error if the device is not found, not connected, or doesn't
    /// have RGB capability.
    pub async fn set_hsb_color(&self, device_id: DeviceId, color: HsbColor) -> Result<(), Error> {
        self.check_capability(device_id, |c| c.rgb).await?;

        let command = HsbColorCommand::set(color);
        let response = self.send_command(device_id, &command).await?;

        // Parse response and update state
        let color_response: crate::response::HsbColorResponse = response.parse()?;
        if let Ok(parsed_color) = color_response.hsb_color() {
            let change = StateChange::HsbColor(parsed_color);
            self.apply_state_change(device_id, change).await;
        }

        // Also update dimmer if returned
        if let Some(dimmer) = color_response.dimmer() {
            let dimmer_change = StateChange::Dimmer(Dimmer::clamped(dimmer));
            self.apply_state_change(device_id, dimmer_change).await;
        }

        Ok(())
    }

    /// Sets the color temperature.
    ///
    /// # Errors
    ///
    /// Returns an error if the device is not found, not connected, or doesn't
    /// have color temperature capability.
    pub async fn set_color_temp(&self, device_id: DeviceId, ct: ColorTemp) -> Result<(), Error> {
        self.check_capability(device_id, |c| c.color_temp).await?;

        let command = crate::command::ColorTempCommand::set(ct);
        let response = self.send_command(device_id, &command).await?;

        // Parse response and update state
        let ct_response: crate::response::ColorTempResponse = response.parse()?;
        if let Ok(parsed_ct) = ct_response.color_temp() {
            let change = StateChange::ColorTemp(parsed_ct);
            self.apply_state_change(device_id, change).await;
        }

        Ok(())
    }

    // =========================================================================
    // Internal Helpers
    // =========================================================================

    /// Sends a command to a device.
    async fn send_command<C: crate::command::Command + Sync>(
        &self,
        device_id: DeviceId,
        command: &C,
    ) -> Result<crate::protocol::CommandResponse, Error> {
        let devices = self.devices.read().await;
        let device = devices.get(&device_id).ok_or(Error::DeviceNotFound)?;

        let client = device.client.as_ref().ok_or(Error::NotConnected)?;
        let response = client.send_command(command).await?;

        Ok(response)
    }

    /// Checks if a device has a specific capability.
    async fn check_capability<F>(&self, device_id: DeviceId, check: F) -> Result<(), Error>
    where
        F: FnOnce(&Capabilities) -> bool,
    {
        let devices = self.devices.read().await;
        let device = devices.get(&device_id).ok_or(Error::DeviceNotFound)?;

        if !check(&device.capabilities) {
            return Err(Error::CapabilityNotSupported);
        }

        Ok(())
    }

    /// Applies a state change and publishes an event.
    async fn apply_state_change(&self, device_id: DeviceId, change: StateChange) {
        let mut devices = self.devices.write().await;

        if let Some(device) = devices.get_mut(&device_id)
            && device.apply_state_change(&change)
        {
            let event = DeviceEvent::state_changed(device_id, change, device.state.clone());
            drop(devices);
            self.event_bus.publish(event);
        }
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for DeviceManager {
    fn clone(&self) -> Self {
        Self {
            devices: Arc::clone(&self.devices),
            event_bus: self.event_bus.clone(),
            default_reconnection: self.default_reconnection.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn new_manager_is_empty() {
        let manager = DeviceManager::new();

        assert_eq!(manager.device_count().await, 0);
        assert!(manager.device_ids().await.is_empty());
    }

    #[tokio::test]
    async fn add_device_returns_id() {
        let manager = DeviceManager::new();
        let config = DeviceConfig::mqtt("mqtt://localhost:1883", "test");

        let id = manager.add_device(config).await;

        assert_eq!(manager.device_count().await, 1);
        assert!(manager.device_ids().await.contains(&id));
    }

    #[tokio::test]
    async fn add_device_publishes_event() {
        let manager = DeviceManager::new();
        let mut events = manager.subscribe();

        let config = DeviceConfig::mqtt("mqtt://localhost:1883", "test");
        let id = manager.add_device(config).await;

        let event = events.recv().await.unwrap();
        assert!(matches!(event, DeviceEvent::DeviceAdded { device_id } if device_id == id));
    }

    #[tokio::test]
    async fn remove_device_publishes_event() {
        let manager = DeviceManager::new();
        let config = DeviceConfig::mqtt("mqtt://localhost:1883", "test");
        let id = manager.add_device(config).await;

        let mut events = manager.subscribe();

        let removed = manager.remove_device(id).await;
        assert!(removed);
        assert_eq!(manager.device_count().await, 0);

        let event = events.recv().await.unwrap();
        assert!(matches!(event, DeviceEvent::DeviceRemoved { device_id } if device_id == id));
    }

    #[tokio::test]
    async fn remove_nonexistent_device_returns_false() {
        let manager = DeviceManager::new();
        let fake_id = DeviceId::new();

        assert!(!manager.remove_device(fake_id).await);
    }

    #[tokio::test]
    async fn get_state_returns_none_for_unknown_device() {
        let manager = DeviceManager::new();
        let fake_id = DeviceId::new();

        assert!(manager.get_state(fake_id).await.is_none());
    }

    #[tokio::test]
    async fn get_state_returns_initial_state() {
        let manager = DeviceManager::new();
        let config = DeviceConfig::mqtt("mqtt://localhost:1883", "test");
        let id = manager.add_device(config).await;

        let state = manager.get_state(id).await.unwrap();
        assert!(state.power(0).is_none()); // Initial state has no power set
    }

    #[tokio::test]
    async fn capabilities_returns_configured_caps() {
        let manager = DeviceManager::new();
        let config =
            DeviceConfig::mqtt("mqtt://localhost:1883", "test").with_capabilities(Capabilities {
                dimmer: true,
                rgb: true,
                ..Default::default()
            });
        let id = manager.add_device(config).await;

        let caps = manager.capabilities(id).await.unwrap();
        assert!(caps.dimmer);
        assert!(caps.rgb);
    }

    #[tokio::test]
    async fn friendly_name_returns_configured_name() {
        let manager = DeviceManager::new();
        let config =
            DeviceConfig::mqtt("mqtt://localhost:1883", "topic").with_friendly_name("My Light");
        let id = manager.add_device(config).await;

        assert_eq!(
            manager.friendly_name(id).await,
            Some("My Light".to_string())
        );
    }

    #[tokio::test]
    async fn is_connected_false_initially() {
        let manager = DeviceManager::new();
        let config = DeviceConfig::mqtt("mqtt://localhost:1883", "test");
        let id = manager.add_device(config).await;

        assert!(!manager.is_connected(id).await);
    }
}
