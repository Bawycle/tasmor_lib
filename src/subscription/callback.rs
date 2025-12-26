// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Callback management for device state subscriptions.
//!
//! This module provides the core types for managing subscription callbacks:
//!
//! - [`SubscriptionId`] - Unique identifier for unsubscribing
//! - [`CallbackRegistry`] - Internal registry for storing and dispatching callbacks

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::RwLock;

use crate::state::{DeviceState, StateChange};
use crate::types::{ColorTemperature, Dimmer, HsbColor, PowerState, Scheme};

/// Unique identifier for a subscription.
///
/// This ID is returned when creating a subscription and can be used to
/// unsubscribe later. IDs are unique within a device's lifetime.
///
/// # Examples
///
/// ```ignore
/// let sub_id = device.on_power_changed(|idx, state| { /* ... */ });
///
/// // Later, unsubscribe
/// device.unsubscribe(sub_id);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionId(u64);

impl SubscriptionId {
    /// Creates a new subscription ID with the given value.
    #[must_use]
    pub(crate) fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the raw ID value.
    #[must_use]
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for SubscriptionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sub({})", self.0)
    }
}

/// Type alias for power state callbacks.
type PowerCallback = Arc<dyn Fn(u8, PowerState) + Send + Sync>;

/// Type alias for dimmer callbacks.
type DimmerCallback = Arc<dyn Fn(Dimmer) + Send + Sync>;

/// Type alias for HSB color callbacks.
type HsbColorCallback = Arc<dyn Fn(HsbColor) + Send + Sync>;

/// Type alias for color temperature callbacks.
type ColorTempCallback = Arc<dyn Fn(ColorTemperature) + Send + Sync>;

/// Type alias for scheme callbacks.
type SchemeCallback = Arc<dyn Fn(Scheme) + Send + Sync>;

/// Type alias for energy callbacks.
type EnergyCallback = Arc<dyn Fn(EnergyData) + Send + Sync>;

/// Type alias for connected callbacks (receives initial state).
type ConnectedCallback = Arc<dyn Fn(&DeviceState) + Send + Sync>;

/// Type alias for disconnected callbacks.
type DisconnectedCallback = Arc<dyn Fn() + Send + Sync>;

/// Type alias for generic state change callbacks.
type StateChangedCallback = Arc<dyn Fn(&StateChange) + Send + Sync>;

/// Energy data passed to energy callbacks.
#[derive(Debug, Clone)]
pub struct EnergyData {
    /// Power consumption in Watts.
    pub power: Option<f32>,
    /// Voltage in Volts.
    pub voltage: Option<f32>,
    /// Current in Amperes.
    pub current: Option<f32>,
    /// Energy consumed today in kWh.
    pub energy_today: Option<f32>,
    /// Total energy consumed in kWh.
    pub energy_total: Option<f32>,
}

/// Registry for managing device subscription callbacks.
///
/// This is an internal type used by devices to store and dispatch callbacks.
/// It uses thread-safe interior mutability via `parking_lot::RwLock` for
/// high performance in async contexts.
///
/// # Thread Safety
///
/// The registry is fully thread-safe and can be accessed from multiple tasks
/// concurrently. Callbacks are wrapped in `Arc` so they can be cloned cheaply.
pub struct CallbackRegistry {
    /// Counter for generating unique subscription IDs.
    next_id: AtomicU64,
    /// Power state change callbacks.
    power_callbacks: RwLock<HashMap<SubscriptionId, PowerCallback>>,
    /// Dimmer change callbacks.
    dimmer_callbacks: RwLock<HashMap<SubscriptionId, DimmerCallback>>,
    /// HSB color change callbacks.
    hsb_color_callbacks: RwLock<HashMap<SubscriptionId, HsbColorCallback>>,
    /// Color temperature change callbacks.
    color_temp_callbacks: RwLock<HashMap<SubscriptionId, ColorTempCallback>>,
    /// Scheme change callbacks.
    scheme_callbacks: RwLock<HashMap<SubscriptionId, SchemeCallback>>,
    /// Energy update callbacks.
    energy_callbacks: RwLock<HashMap<SubscriptionId, EnergyCallback>>,
    /// Connected callbacks (called when device becomes available).
    connected_callbacks: RwLock<HashMap<SubscriptionId, ConnectedCallback>>,
    /// Disconnected callbacks (called when device becomes unavailable).
    disconnected_callbacks: RwLock<HashMap<SubscriptionId, DisconnectedCallback>>,
    /// Generic state change callbacks (receives all changes).
    state_changed_callbacks: RwLock<HashMap<SubscriptionId, StateChangedCallback>>,
}

impl CallbackRegistry {
    /// Creates a new empty callback registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            power_callbacks: RwLock::new(HashMap::new()),
            dimmer_callbacks: RwLock::new(HashMap::new()),
            hsb_color_callbacks: RwLock::new(HashMap::new()),
            color_temp_callbacks: RwLock::new(HashMap::new()),
            scheme_callbacks: RwLock::new(HashMap::new()),
            energy_callbacks: RwLock::new(HashMap::new()),
            connected_callbacks: RwLock::new(HashMap::new()),
            disconnected_callbacks: RwLock::new(HashMap::new()),
            state_changed_callbacks: RwLock::new(HashMap::new()),
        }
    }

    /// Generates a new unique subscription ID.
    fn next_id(&self) -> SubscriptionId {
        SubscriptionId::new(self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    // =========================================================================
    // Registration methods
    // =========================================================================

    /// Registers a callback for power state changes.
    ///
    /// The callback receives the relay index (1-8) and the new power state.
    pub fn on_power_changed<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(u8, PowerState) + Send + Sync + 'static,
    {
        let id = self.next_id();
        self.power_callbacks.write().insert(id, Arc::new(callback));
        id
    }

    /// Registers a callback for dimmer changes.
    pub fn on_dimmer_changed<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(Dimmer) + Send + Sync + 'static,
    {
        let id = self.next_id();
        self.dimmer_callbacks.write().insert(id, Arc::new(callback));
        id
    }

    /// Registers a callback for HSB color changes.
    pub fn on_hsb_color_changed<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(HsbColor) + Send + Sync + 'static,
    {
        let id = self.next_id();
        self.hsb_color_callbacks
            .write()
            .insert(id, Arc::new(callback));
        id
    }

    /// Registers a callback for color temperature changes.
    pub fn on_color_temp_changed<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(ColorTemperature) + Send + Sync + 'static,
    {
        let id = self.next_id();
        self.color_temp_callbacks
            .write()
            .insert(id, Arc::new(callback));
        id
    }

    /// Registers a callback for scheme changes.
    pub fn on_scheme_changed<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(Scheme) + Send + Sync + 'static,
    {
        let id = self.next_id();
        self.scheme_callbacks.write().insert(id, Arc::new(callback));
        id
    }

    /// Registers a callback for energy updates.
    pub fn on_energy_updated<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(EnergyData) + Send + Sync + 'static,
    {
        let id = self.next_id();
        self.energy_callbacks.write().insert(id, Arc::new(callback));
        id
    }

    /// Registers a callback for when the device becomes connected.
    ///
    /// The callback receives the initial device state.
    pub fn on_connected<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(&DeviceState) + Send + Sync + 'static,
    {
        let id = self.next_id();
        self.connected_callbacks
            .write()
            .insert(id, Arc::new(callback));
        id
    }

    /// Registers a callback for when the device becomes disconnected.
    pub fn on_disconnected<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn() + Send + Sync + 'static,
    {
        let id = self.next_id();
        self.disconnected_callbacks
            .write()
            .insert(id, Arc::new(callback));
        id
    }

    /// Registers a callback for all state changes.
    ///
    /// This is useful for logging or debugging, as it receives every change.
    pub fn on_state_changed<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(&StateChange) + Send + Sync + 'static,
    {
        let id = self.next_id();
        self.state_changed_callbacks
            .write()
            .insert(id, Arc::new(callback));
        id
    }

    // =========================================================================
    // Unsubscription
    // =========================================================================

    /// Unregisters a callback by its subscription ID.
    ///
    /// Returns `true` if a callback was found and removed.
    pub fn unsubscribe(&self, id: SubscriptionId) -> bool {
        // Try each callback map until we find and remove the ID
        if self.power_callbacks.write().remove(&id).is_some() {
            return true;
        }
        if self.dimmer_callbacks.write().remove(&id).is_some() {
            return true;
        }
        if self.hsb_color_callbacks.write().remove(&id).is_some() {
            return true;
        }
        if self.color_temp_callbacks.write().remove(&id).is_some() {
            return true;
        }
        if self.scheme_callbacks.write().remove(&id).is_some() {
            return true;
        }
        if self.energy_callbacks.write().remove(&id).is_some() {
            return true;
        }
        if self.connected_callbacks.write().remove(&id).is_some() {
            return true;
        }
        if self.disconnected_callbacks.write().remove(&id).is_some() {
            return true;
        }
        if self.state_changed_callbacks.write().remove(&id).is_some() {
            return true;
        }
        false
    }

    /// Clears all callbacks.
    pub fn clear(&self) {
        self.power_callbacks.write().clear();
        self.dimmer_callbacks.write().clear();
        self.hsb_color_callbacks.write().clear();
        self.color_temp_callbacks.write().clear();
        self.scheme_callbacks.write().clear();
        self.energy_callbacks.write().clear();
        self.connected_callbacks.write().clear();
        self.disconnected_callbacks.write().clear();
        self.state_changed_callbacks.write().clear();
    }

    // =========================================================================
    // Dispatch methods
    // =========================================================================

    /// Dispatches a state change to relevant callbacks.
    ///
    /// This method calls all registered callbacks that match the change type.
    /// Callbacks are called synchronously in an arbitrary order.
    pub fn dispatch(&self, change: &StateChange) {
        // Always dispatch to generic state_changed callbacks
        {
            let callbacks = self.state_changed_callbacks.read();
            for callback in callbacks.values() {
                callback(change);
            }
        }

        // Dispatch to specific callbacks based on change type
        match change {
            StateChange::Power { index, state } => {
                let callbacks = self.power_callbacks.read();
                for callback in callbacks.values() {
                    callback(*index, *state);
                }
            }
            StateChange::Dimmer(dimmer) => {
                let callbacks = self.dimmer_callbacks.read();
                for callback in callbacks.values() {
                    callback(*dimmer);
                }
            }
            StateChange::HsbColor(color) => {
                let callbacks = self.hsb_color_callbacks.read();
                for callback in callbacks.values() {
                    callback(*color);
                }
            }
            StateChange::ColorTemperature(ct) => {
                let callbacks = self.color_temp_callbacks.read();
                for callback in callbacks.values() {
                    callback(*ct);
                }
            }
            StateChange::Scheme(scheme) => {
                let callbacks = self.scheme_callbacks.read();
                for callback in callbacks.values() {
                    callback(*scheme);
                }
            }
            StateChange::WakeupDuration(_)
            | StateChange::FadeEnabled(_)
            | StateChange::FadeSpeed(_) => {
                // These have no specific callbacks; changes are captured
                // by generic state_changed callbacks
            }
            StateChange::Energy {
                power,
                voltage,
                current,
                energy_today,
                energy_total,
                ..
            } => {
                let data = EnergyData {
                    power: *power,
                    voltage: *voltage,
                    current: *current,
                    energy_today: *energy_today,
                    energy_total: *energy_total,
                };
                let callbacks = self.energy_callbacks.read();
                for callback in callbacks.values() {
                    callback(data.clone());
                }
            }
            StateChange::Batch(changes) => {
                // Recursively dispatch each change in the batch
                for nested_change in changes {
                    self.dispatch(nested_change);
                }
            }
        }
    }

    /// Dispatches the connected event with the initial device state.
    pub fn dispatch_connected(&self, state: &DeviceState) {
        let callbacks = self.connected_callbacks.read();
        for callback in callbacks.values() {
            callback(state);
        }
    }

    /// Dispatches the disconnected event.
    pub fn dispatch_disconnected(&self) {
        let callbacks = self.disconnected_callbacks.read();
        for callback in callbacks.values() {
            callback();
        }
    }

    // =========================================================================
    // Statistics
    // =========================================================================

    /// Returns the total number of registered callbacks.
    #[must_use]
    pub fn callback_count(&self) -> usize {
        self.power_callbacks.read().len()
            + self.dimmer_callbacks.read().len()
            + self.hsb_color_callbacks.read().len()
            + self.color_temp_callbacks.read().len()
            + self.scheme_callbacks.read().len()
            + self.energy_callbacks.read().len()
            + self.connected_callbacks.read().len()
            + self.disconnected_callbacks.read().len()
            + self.state_changed_callbacks.read().len()
    }

    /// Returns `true` if there are no registered callbacks.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.callback_count() == 0
    }
}

impl Default for CallbackRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for CallbackRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallbackRegistry")
            .field("callback_count", &self.callback_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    #[test]
    fn subscription_id_display() {
        let id = SubscriptionId::new(42);
        assert_eq!(id.to_string(), "Sub(42)");
    }

    #[test]
    fn subscription_id_equality() {
        let id1 = SubscriptionId::new(1);
        let id2 = SubscriptionId::new(1);
        let id3 = SubscriptionId::new(2);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn subscription_id_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(SubscriptionId::new(1));
        set.insert(SubscriptionId::new(2));
        set.insert(SubscriptionId::new(1)); // Duplicate

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn registry_new_is_empty() {
        let registry = CallbackRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.callback_count(), 0);
    }

    #[test]
    fn registry_power_callback() {
        let registry = CallbackRegistry::new();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let id = registry.on_power_changed(move |_idx, _state| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        assert!(!registry.is_empty());
        assert_eq!(registry.callback_count(), 1);

        // Dispatch a power change
        registry.dispatch(&StateChange::power(1, PowerState::On));
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Unsubscribe
        assert!(registry.unsubscribe(id));
        assert!(registry.is_empty());

        // Dispatch again - counter should not change
        registry.dispatch(&StateChange::power(1, PowerState::Off));
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn registry_dimmer_callback() {
        let registry = CallbackRegistry::new();
        let received = Arc::new(RwLock::new(None::<Dimmer>));
        let received_clone = received.clone();

        registry.on_dimmer_changed(move |dimmer| {
            *received_clone.write() = Some(dimmer);
        });

        let dimmer = Dimmer::new(75).unwrap();
        registry.dispatch(&StateChange::Dimmer(dimmer));

        assert_eq!(*received.read(), Some(dimmer));
    }

    #[test]
    fn registry_state_changed_callback() {
        let registry = CallbackRegistry::new();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        registry.on_state_changed(move |_change| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        // Different types of changes all trigger the generic callback
        registry.dispatch(&StateChange::power_on());
        registry.dispatch(&StateChange::Dimmer(Dimmer::MAX));
        registry.dispatch(&StateChange::HsbColor(HsbColor::red()));

        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn registry_batch_dispatch() {
        let registry = CallbackRegistry::new();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        registry.on_state_changed(move |_| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        let batch = StateChange::batch(vec![
            StateChange::power_on(),
            StateChange::Dimmer(Dimmer::new(50).unwrap()),
        ]);

        registry.dispatch(&batch);

        // Should be called for batch + each item = 3
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn registry_multiple_callbacks_same_type() {
        let registry = CallbackRegistry::new();
        let counter1 = Arc::new(AtomicU32::new(0));
        let counter2 = Arc::new(AtomicU32::new(0));
        let c1 = counter1.clone();
        let c2 = counter2.clone();

        registry.on_power_changed(move |_, _| {
            c1.fetch_add(1, Ordering::SeqCst);
        });
        registry.on_power_changed(move |_, _| {
            c2.fetch_add(1, Ordering::SeqCst);
        });

        registry.dispatch(&StateChange::power_on());

        assert_eq!(counter1.load(Ordering::SeqCst), 1);
        assert_eq!(counter2.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn registry_unsubscribe_nonexistent() {
        let registry = CallbackRegistry::new();
        let fake_id = SubscriptionId::new(999);

        assert!(!registry.unsubscribe(fake_id));
    }

    #[test]
    fn registry_clear() {
        let registry = CallbackRegistry::new();

        registry.on_power_changed(|_, _| {});
        registry.on_dimmer_changed(|_| {});
        registry.on_connected(|_| {});

        assert_eq!(registry.callback_count(), 3);

        registry.clear();
        assert!(registry.is_empty());
    }

    #[test]
    fn registry_connected_callback() {
        let registry = CallbackRegistry::new();
        let was_called = Arc::new(AtomicU32::new(0));
        let was_called_clone = was_called.clone();

        registry.on_connected(move |_state| {
            was_called_clone.fetch_add(1, Ordering::SeqCst);
        });

        registry.dispatch_connected(&DeviceState::new());
        assert_eq!(was_called.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn registry_disconnected_callback() {
        let registry = CallbackRegistry::new();
        let was_called = Arc::new(AtomicU32::new(0));
        let was_called_clone = was_called.clone();

        registry.on_disconnected(move || {
            was_called_clone.fetch_add(1, Ordering::SeqCst);
        });

        registry.dispatch_disconnected();
        assert_eq!(was_called.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn registry_unique_ids() {
        let registry = CallbackRegistry::new();

        let id1 = registry.on_power_changed(|_, _| {});
        let id2 = registry.on_dimmer_changed(|_| {});
        let id3 = registry.on_connected(|_| {});

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }

    #[test]
    fn registry_debug() {
        let registry = CallbackRegistry::new();
        registry.on_power_changed(|_, _| {});

        let debug = format!("{registry:?}");
        assert!(debug.contains("CallbackRegistry"));
        assert!(debug.contains("callback_count"));
    }

    #[test]
    fn energy_data_debug() {
        let data = EnergyData {
            power: Some(100.0),
            voltage: Some(230.0),
            current: Some(0.5),
            energy_today: Some(1.5),
            energy_total: Some(150.0),
        };

        let debug = format!("{data:?}");
        assert!(debug.contains("EnergyData"));
        assert!(debug.contains("100.0"));
    }
}
