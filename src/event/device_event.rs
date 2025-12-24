// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Device event types.

use crate::state::{DeviceState, StateChange};

use super::DeviceId;

/// Events emitted by the device manager.
///
/// These events notify subscribers about device lifecycle changes,
/// connection status, and state updates. All events include the
/// relevant device ID for targeted handling.
///
/// # Examples
///
/// ```
/// use tasmor_lib::event::{DeviceId, DeviceEvent};
/// use tasmor_lib::state::StateChange;
/// use tasmor_lib::types::PowerState;
///
/// let device_id = DeviceId::new();
///
/// // Device lifecycle events
/// let added = DeviceEvent::DeviceAdded { device_id };
/// let removed = DeviceEvent::DeviceRemoved { device_id };
///
/// // Connection events
/// let connected = DeviceEvent::ConnectionChanged {
///     device_id,
///     connected: true,
///     error: None,
/// };
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DeviceEvent {
    /// A device was added to the manager.
    DeviceAdded {
        /// The ID of the added device.
        device_id: DeviceId,
    },

    /// A device was removed from the manager.
    DeviceRemoved {
        /// The ID of the removed device.
        device_id: DeviceId,
    },

    /// Device connection state changed.
    ConnectionChanged {
        /// The ID of the device.
        device_id: DeviceId,
        /// Whether the device is now connected.
        connected: bool,
        /// Error message if disconnection was due to an error.
        error: Option<String>,
    },

    /// Device state changed.
    ///
    /// This event is emitted whenever the device reports a state change,
    /// either in response to a command or from telemetry updates.
    StateChanged {
        /// The ID of the device.
        device_id: DeviceId,
        /// The specific change that occurred.
        change: StateChange,
        /// The complete new state of the device.
        new_state: DeviceState,
    },
}

impl DeviceEvent {
    /// Returns the device ID associated with this event.
    #[must_use]
    pub fn device_id(&self) -> DeviceId {
        match self {
            Self::DeviceAdded { device_id }
            | Self::DeviceRemoved { device_id }
            | Self::ConnectionChanged { device_id, .. }
            | Self::StateChanged { device_id, .. } => *device_id,
        }
    }

    /// Returns `true` if this is a device lifecycle event (added/removed).
    #[must_use]
    pub fn is_lifecycle(&self) -> bool {
        matches!(self, Self::DeviceAdded { .. } | Self::DeviceRemoved { .. })
    }

    /// Returns `true` if this is a connection event.
    #[must_use]
    pub fn is_connection(&self) -> bool {
        matches!(self, Self::ConnectionChanged { .. })
    }

    /// Returns `true` if this is a state change event.
    #[must_use]
    pub fn is_state_change(&self) -> bool {
        matches!(self, Self::StateChanged { .. })
    }

    /// Creates a device added event.
    #[must_use]
    pub fn device_added(device_id: DeviceId) -> Self {
        Self::DeviceAdded { device_id }
    }

    /// Creates a device removed event.
    #[must_use]
    pub fn device_removed(device_id: DeviceId) -> Self {
        Self::DeviceRemoved { device_id }
    }

    /// Creates a connected event.
    #[must_use]
    pub fn connected(device_id: DeviceId) -> Self {
        Self::ConnectionChanged {
            device_id,
            connected: true,
            error: None,
        }
    }

    /// Creates a disconnected event.
    #[must_use]
    pub fn disconnected(device_id: DeviceId) -> Self {
        Self::ConnectionChanged {
            device_id,
            connected: false,
            error: None,
        }
    }

    /// Creates a disconnected event with an error.
    #[must_use]
    pub fn disconnected_with_error(device_id: DeviceId, error: impl Into<String>) -> Self {
        Self::ConnectionChanged {
            device_id,
            connected: false,
            error: Some(error.into()),
        }
    }

    /// Creates a state changed event.
    #[must_use]
    pub fn state_changed(device_id: DeviceId, change: StateChange, new_state: DeviceState) -> Self {
        Self::StateChanged {
            device_id,
            change,
            new_state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PowerState;

    #[test]
    fn device_id_extraction() {
        let id = DeviceId::new();

        let added = DeviceEvent::device_added(id);
        assert_eq!(added.device_id(), id);

        let removed = DeviceEvent::device_removed(id);
        assert_eq!(removed.device_id(), id);

        let connected = DeviceEvent::connected(id);
        assert_eq!(connected.device_id(), id);
    }

    #[test]
    fn lifecycle_events() {
        let id = DeviceId::new();

        assert!(DeviceEvent::device_added(id).is_lifecycle());
        assert!(DeviceEvent::device_removed(id).is_lifecycle());
        assert!(!DeviceEvent::connected(id).is_lifecycle());
    }

    #[test]
    fn connection_events() {
        let id = DeviceId::new();

        assert!(DeviceEvent::connected(id).is_connection());
        assert!(DeviceEvent::disconnected(id).is_connection());
        assert!(!DeviceEvent::device_added(id).is_connection());
    }

    #[test]
    fn state_change_events() {
        let id = DeviceId::new();
        let change = StateChange::Power {
            index: 1,
            state: PowerState::On,
        };
        let state = DeviceState::new();

        let event = DeviceEvent::state_changed(id, change, state);
        assert!(event.is_state_change());
        assert!(!event.is_lifecycle());
        assert!(!event.is_connection());
    }

    #[test]
    fn disconnected_with_error() {
        let id = DeviceId::new();
        let event = DeviceEvent::disconnected_with_error(id, "Connection lost");

        if let DeviceEvent::ConnectionChanged {
            connected, error, ..
        } = event
        {
            assert!(!connected);
            assert_eq!(error, Some("Connection lost".to_string()));
        } else {
            panic!("Expected ConnectionChanged event");
        }
    }
}
