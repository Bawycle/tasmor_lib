// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Internal device wrapper for the device manager.

use tokio::sync::watch;

use crate::Capabilities;
use crate::event::DeviceId;
use crate::protocol::{PooledMqttClient, Protocol};
use crate::state::DeviceState;

use super::device_config::{ConnectionConfig, DeviceConfig};

/// Connection state for a managed device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    /// Device is not connected.
    Disconnected,
    /// Device is in the process of connecting.
    Connecting,
    /// Device is connected and operational.
    Connected,
    /// Connection failed with an error.
    Failed(String),
    /// Reconnecting after a connection loss.
    Reconnecting { attempt: u32 },
}

impl ConnectionState {
    /// Returns true if the device is connected.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected)
    }

    /// Returns true if the device is in a failed state.
    #[must_use]
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }
}

/// Internal representation of a device in the manager.
pub(crate) struct ManagedDevice {
    /// Unique device identifier.
    pub id: DeviceId,
    /// Device configuration.
    pub config: DeviceConfig,
    /// Current connection state.
    pub connection_state: ConnectionState,
    /// Device capabilities (detected or configured).
    pub capabilities: Capabilities,
    /// Current device state.
    pub state: DeviceState,
    /// Watch channel sender for state updates.
    pub state_tx: watch::Sender<DeviceState>,
    /// Protocol client (if connected).
    pub client: Option<DeviceClient>,
}

/// Protocol client for a managed device.
pub(crate) enum DeviceClient {
    /// Pooled MQTT client.
    Mqtt(PooledMqttClient),
    /// HTTP client (using the existing Device wrapper for now).
    Http(crate::Device<crate::protocol::HttpClient>),
}

impl DeviceClient {
    /// Sends a command using the appropriate protocol.
    pub async fn send_command<C: crate::command::Command + Sync>(
        &self,
        command: &C,
    ) -> Result<crate::protocol::CommandResponse, crate::error::Error> {
        match self {
            Self::Mqtt(client) => client
                .send_command(command)
                .await
                .map_err(crate::error::Error::Protocol),
            Self::Http(device) => device.send_command(command).await,
        }
    }
}

impl ManagedDevice {
    /// Creates a new managed device from configuration.
    pub fn new(config: DeviceConfig) -> Self {
        let id = DeviceId::new();
        let capabilities = config.capabilities.clone().unwrap_or_default();
        let state = DeviceState::new();
        let (state_tx, _) = watch::channel(state.clone());

        Self {
            id,
            config,
            connection_state: ConnectionState::Disconnected,
            capabilities,
            state,
            state_tx,
            client: None,
        }
    }

    /// Creates a new managed device with a specific ID.
    #[allow(dead_code)] // For future use when loading devices from persistence
    pub fn with_id(id: DeviceId, config: DeviceConfig) -> Self {
        let capabilities = config.capabilities.clone().unwrap_or_default();
        let state = DeviceState::new();
        let (state_tx, _) = watch::channel(state.clone());

        Self {
            id,
            config,
            connection_state: ConnectionState::Disconnected,
            capabilities,
            state,
            state_tx,
            client: None,
        }
    }

    /// Returns the device ID.
    pub fn id(&self) -> DeviceId {
        self.id
    }

    /// Returns the friendly name if set, otherwise the topic/host.
    pub fn display_name(&self) -> &str {
        if let Some(name) = &self.config.friendly_name {
            return name;
        }

        match &self.config.connection {
            ConnectionConfig::Mqtt { topic, .. } => topic,
            ConnectionConfig::Http { host, .. } => host,
        }
    }

    /// Creates a watch receiver for state updates.
    pub fn watch_state(&self) -> watch::Receiver<DeviceState> {
        self.state_tx.subscribe()
    }

    /// Updates the device state and notifies watchers.
    #[allow(dead_code)] // For future use in telemetry handling
    pub fn update_state(&mut self, new_state: DeviceState) {
        self.state = new_state.clone();
        // Ignore send errors (no receivers)
        let _ = self.state_tx.send(new_state);
    }

    /// Applies a state change and notifies watchers.
    ///
    /// Returns true if the state was actually changed.
    pub fn apply_state_change(&mut self, change: &crate::state::StateChange) -> bool {
        if self.state.apply(change) {
            let _ = self.state_tx.send(self.state.clone());
            true
        } else {
            false
        }
    }

    /// Sets the connection state.
    pub fn set_connection_state(&mut self, state: ConnectionState) {
        self.connection_state = state;
    }

    /// Returns true if the device is connected.
    pub fn is_connected(&self) -> bool {
        self.connection_state.is_connected()
    }

    /// Sets the client for this device.
    pub fn set_client(&mut self, client: DeviceClient) {
        self.client = Some(client);
        self.connection_state = ConnectionState::Connected;
    }

    /// Clears the client (disconnects).
    pub fn clear_client(&mut self) {
        self.client = None;
        self.connection_state = ConnectionState::Disconnected;
    }
}

impl std::fmt::Debug for ManagedDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagedDevice")
            .field("id", &self.id)
            .field("display_name", &self.display_name())
            .field("connection_state", &self.connection_state)
            .field("capabilities", &self.capabilities)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::StateChange;
    use crate::types::{Dimmer, PowerState};

    #[test]
    fn new_device_is_disconnected() {
        let config = DeviceConfig::mqtt("mqtt://localhost:1883", "test");
        let device = ManagedDevice::new(config);

        assert!(!device.is_connected());
        assert!(matches!(
            device.connection_state,
            ConnectionState::Disconnected
        ));
    }

    #[test]
    fn display_name_uses_friendly_name() {
        let config =
            DeviceConfig::mqtt("mqtt://localhost:1883", "topic").with_friendly_name("My Light");
        let device = ManagedDevice::new(config);

        assert_eq!(device.display_name(), "My Light");
    }

    #[test]
    fn display_name_falls_back_to_topic() {
        let config = DeviceConfig::mqtt("mqtt://localhost:1883", "tasmota_bulb");
        let device = ManagedDevice::new(config);

        assert_eq!(device.display_name(), "tasmota_bulb");
    }

    #[test]
    fn display_name_falls_back_to_host() {
        let config = DeviceConfig::http("192.168.1.100");
        let device = ManagedDevice::new(config);

        assert_eq!(device.display_name(), "192.168.1.100");
    }

    #[tokio::test]
    async fn watch_state_receives_updates() {
        let config = DeviceConfig::mqtt("mqtt://localhost:1883", "test");
        let mut device = ManagedDevice::new(config);
        let rx = device.watch_state();

        // Apply a state change (index is 1-based, like Tasmota POWER1)
        let change = StateChange::Power {
            index: 1,
            state: PowerState::On,
        };
        let changed = device.apply_state_change(&change);
        assert!(changed, "state should have changed");

        // Verify device state is updated
        assert_eq!(device.state.power(1), Some(PowerState::On));

        // The watch receiver should see the latest value when borrowed
        let state = rx.borrow();
        assert_eq!(state.power(1), Some(PowerState::On));
    }

    #[test]
    fn apply_state_change_returns_true_on_change() {
        let config = DeviceConfig::mqtt("mqtt://localhost:1883", "test");
        let mut device = ManagedDevice::new(config);

        let change = StateChange::Dimmer(Dimmer::new(50).unwrap());
        assert!(device.apply_state_change(&change));

        // Same value should return false
        assert!(!device.apply_state_change(&change));
    }

    #[test]
    fn connection_state_checks() {
        assert!(ConnectionState::Connected.is_connected());
        assert!(!ConnectionState::Disconnected.is_connected());
        assert!(!ConnectionState::Connecting.is_connected());

        assert!(ConnectionState::Failed("error".to_string()).is_failed());
        assert!(!ConnectionState::Connected.is_failed());
    }
}
