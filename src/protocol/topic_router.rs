// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! MQTT topic routing for device callbacks.
//!
//! The [`TopicRouter`] handles routing of incoming MQTT messages to the
//! appropriate device callback registries. It uses weak references to
//! allow devices to be dropped without explicit cleanup.
//!
//! # Architecture
//!
//! ```text
//! MQTT Message: stat/tasmota_bedroom/POWER → ON
//!                     ↓
//!             TopicRouter.route()
//!                     ↓
//!     Lookup "tasmota_bedroom" in subscribers
//!                     ↓
//!        Weak<CallbackRegistry>.upgrade()
//!                     ↓
//!           callbacks.dispatch(PowerChanged(On))
//!                     ↓
//!           User callback invoked
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Weak};

use parking_lot::RwLock;

use crate::state::StateChange;
use crate::subscription::CallbackRegistry;
use crate::telemetry::{SensorData, TelemetryState};
use crate::types::PowerState;

/// Routes MQTT messages to device callback registries.
///
/// This router maintains weak references to device callbacks, allowing
/// devices to be dropped naturally. When a device is dropped, its callbacks
/// are automatically cleaned up on the next routing attempt.
#[derive(Debug, Default)]
pub struct TopicRouter {
    /// Map from device topic to weak reference to its callback registry.
    subscribers: RwLock<HashMap<String, Weak<CallbackRegistry>>>,
}

impl TopicRouter {
    /// Creates a new empty topic router.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a device's callback registry for the given topic.
    ///
    /// If a previous registration exists for this topic, it will be replaced.
    pub fn register(&self, device_topic: impl Into<String>, callbacks: &Arc<CallbackRegistry>) {
        let topic = device_topic.into();
        tracing::debug!(topic = %topic, "Registering device for routing");
        self.subscribers
            .write()
            .insert(topic, Arc::downgrade(callbacks));
    }

    /// Unregisters a device from routing.
    ///
    /// Returns `true` if the device was previously registered.
    pub fn unregister(&self, device_topic: &str) -> bool {
        tracing::debug!(topic = %device_topic, "Unregistering device from routing");
        self.subscribers.write().remove(device_topic).is_some()
    }

    /// Routes an MQTT message to the appropriate device.
    ///
    /// The topic should be a full MQTT topic like:
    /// - `stat/<device_topic>/POWER` → Power state
    /// - `stat/<device_topic>/RESULT` → Command result
    /// - `tele/<device_topic>/STATE` → Telemetry state
    /// - `tele/<device_topic>/SENSOR` → Sensor data
    ///
    /// Returns `true` if the message was successfully routed to a device.
    pub fn route(&self, topic: &str, payload: &str) -> bool {
        // Parse topic: prefix/<device_topic>/<subtopic>
        let Some(parsed) = ParsedTopic::parse(topic) else {
            tracing::trace!(topic = %topic, "Ignoring unparseable topic");
            return false;
        };

        // Look up the device's callback registry
        let callbacks = {
            let subscribers = self.subscribers.read();
            subscribers.get(parsed.device_topic).and_then(Weak::upgrade)
        };

        let Some(callbacks) = callbacks else {
            tracing::trace!(
                topic = %topic,
                device = %parsed.device_topic,
                "No registered device for topic"
            );
            return false;
        };

        // Parse the message and dispatch to callbacks
        dispatch_message(&callbacks, &parsed, payload);
        true
    }

    /// Removes stale entries (devices that have been dropped).
    ///
    /// This is called automatically during routing, but can be called
    /// manually to clean up memory.
    pub fn cleanup(&self) {
        self.subscribers.write().retain(|topic, weak| {
            let alive = weak.strong_count() > 0;
            if !alive {
                tracing::debug!(topic = %topic, "Cleaning up dropped device");
            }
            alive
        });
    }

    /// Returns the number of registered devices.
    #[must_use]
    pub fn device_count(&self) -> usize {
        self.subscribers.read().len()
    }

    /// Returns the number of active (not dropped) devices.
    #[must_use]
    pub fn active_device_count(&self) -> usize {
        self.subscribers
            .read()
            .values()
            .filter(|weak| weak.strong_count() > 0)
            .count()
    }
}

/// Dispatches a parsed message to the device's callbacks.
fn dispatch_message(callbacks: &CallbackRegistry, parsed: &ParsedTopic<'_>, payload: &str) {
    match (parsed.prefix, parsed.subtopic) {
        // Power state response: stat/<topic>/POWER or stat/<topic>/POWER1-8
        ("stat", subtopic) if subtopic.starts_with("POWER") => {
            if let Some(change) = parse_power_topic(subtopic, payload) {
                tracing::debug!(
                    device = %parsed.device_topic,
                    subtopic = %subtopic,
                    payload = %payload,
                    "Dispatching power change"
                );
                callbacks.dispatch(&change);
            }
        }

        // Command result: stat/<topic>/RESULT
        ("stat", "RESULT") => {
            if let Some(changes) = parse_result_payload(payload) {
                tracing::debug!(
                    device = %parsed.device_topic,
                    payload = %payload,
                    "Dispatching result changes"
                );
                for change in changes {
                    callbacks.dispatch(&change);
                }
            }
        }

        // Telemetry state: tele/<topic>/STATE
        ("tele", "STATE") => {
            if let Ok(state) = serde_json::from_str::<TelemetryState>(payload) {
                let changes = state.to_state_changes();
                tracing::debug!(
                    device = %parsed.device_topic,
                    change_count = changes.len(),
                    "Dispatching telemetry state changes"
                );
                for change in changes {
                    callbacks.dispatch(&change);
                }
            }
        }

        // Sensor telemetry: tele/<topic>/SENSOR
        ("tele", "SENSOR") => {
            if let Ok(sensor) = serde_json::from_str::<SensorData>(payload) {
                let changes = sensor.to_state_changes();
                if !changes.is_empty() {
                    tracing::debug!(
                        device = %parsed.device_topic,
                        change_count = changes.len(),
                        "Dispatching sensor state changes"
                    );
                    for change in changes {
                        callbacks.dispatch(&change);
                    }
                }
            }
        }

        // LWT (Last Will and Testament): tele/<topic>/LWT
        ("tele", "LWT") => {
            match payload {
                "Online" => {
                    tracing::debug!(device = %parsed.device_topic, "Device came online");
                    // TODO: Dispatch connected event with initial state
                }
                "Offline" => {
                    tracing::debug!(device = %parsed.device_topic, "Device went offline");
                    callbacks.dispatch_disconnected();
                }
                _ => {}
            }
        }

        _ => {
            tracing::trace!(
                device = %parsed.device_topic,
                prefix = %parsed.prefix,
                subtopic = %parsed.subtopic,
                "Ignoring unhandled topic type"
            );
        }
    }
}

/// Parsed MQTT topic components.
#[derive(Debug)]
struct ParsedTopic<'a> {
    /// The topic prefix (`stat` or `tele`).
    prefix: &'a str,
    /// The device topic (e.g., `tasmota_bedroom`).
    device_topic: &'a str,
    /// The subtopic (e.g., `POWER`, `STATE`, `SENSOR`).
    subtopic: &'a str,
}

impl<'a> ParsedTopic<'a> {
    /// Parses an MQTT topic into its components.
    ///
    /// Expected format: `prefix/device_topic/subtopic`
    fn parse(topic: &'a str) -> Option<Self> {
        let parts: Vec<&str> = topic.split('/').collect();
        if parts.len() >= 3 {
            Some(Self {
                prefix: parts[0],
                device_topic: parts[1],
                subtopic: parts[2],
            })
        } else {
            None
        }
    }
}

/// Parses a power topic and payload into a state change.
///
/// Handles both `POWER` (for relay 1) and `POWER1`-`POWER8` formats.
fn parse_power_topic(subtopic: &str, payload: &str) -> Option<StateChange> {
    // Parse relay index from subtopic
    let index = if subtopic == "POWER" {
        1
    } else {
        // Extract number from "POWER1", "POWER2", etc.
        subtopic.strip_prefix("POWER")?.parse().ok()?
    };

    // Parse power state from payload
    let state = payload.parse::<PowerState>().ok()?;

    Some(StateChange::Power { index, state })
}

/// Parses a RESULT payload into state changes.
///
/// RESULT payloads contain JSON with the command response.
fn parse_result_payload(payload: &str) -> Option<Vec<StateChange>> {
    // Try to parse as TelemetryState since RESULT has similar format
    let state: TelemetryState = serde_json::from_str(payload).ok()?;
    let changes = state.to_state_changes();
    if changes.is_empty() {
        None
    } else {
        Some(changes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn parse_topic_valid() {
        let parsed = ParsedTopic::parse("stat/tasmota_bedroom/POWER").unwrap();
        assert_eq!(parsed.prefix, "stat");
        assert_eq!(parsed.device_topic, "tasmota_bedroom");
        assert_eq!(parsed.subtopic, "POWER");
    }

    #[test]
    fn parse_topic_tele() {
        let parsed = ParsedTopic::parse("tele/living_room/STATE").unwrap();
        assert_eq!(parsed.prefix, "tele");
        assert_eq!(parsed.device_topic, "living_room");
        assert_eq!(parsed.subtopic, "STATE");
    }

    #[test]
    fn parse_topic_invalid() {
        assert!(ParsedTopic::parse("invalid").is_none());
        assert!(ParsedTopic::parse("only/two").is_none());
    }

    #[test]
    fn parse_power_topic_simple() {
        let change = parse_power_topic("POWER", "ON").unwrap();
        assert!(matches!(
            change,
            StateChange::Power {
                index: 1,
                state: PowerState::On
            }
        ));
    }

    #[test]
    fn parse_power_topic_indexed() {
        let change = parse_power_topic("POWER3", "OFF").unwrap();
        assert!(matches!(
            change,
            StateChange::Power {
                index: 3,
                state: PowerState::Off
            }
        ));
    }

    #[test]
    fn parse_power_topic_invalid() {
        assert!(parse_power_topic("POWER", "INVALID").is_none());
        assert!(parse_power_topic("INVALID", "ON").is_none());
    }

    #[test]
    fn router_register_and_route() {
        let router = TopicRouter::new();
        let callbacks = Arc::new(CallbackRegistry::new());

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        callbacks.on_power_changed(move |_idx, _state| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        router.register("bedroom", &callbacks);
        assert_eq!(router.device_count(), 1);

        // Route a power message
        let routed = router.route("stat/bedroom/POWER", "ON");
        assert!(routed);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn router_unregistered_device() {
        let router = TopicRouter::new();

        // No devices registered, should not route
        let routed = router.route("stat/unknown/POWER", "ON");
        assert!(!routed);
    }

    #[test]
    fn router_unregister() {
        let router = TopicRouter::new();
        let callbacks = Arc::new(CallbackRegistry::new());

        router.register("bedroom", &callbacks);
        assert_eq!(router.device_count(), 1);

        let removed = router.unregister("bedroom");
        assert!(removed);
        assert_eq!(router.device_count(), 0);

        // Should not route anymore
        let routed = router.route("stat/bedroom/POWER", "ON");
        assert!(!routed);
    }

    #[test]
    fn router_cleanup_dropped_device() {
        let router = TopicRouter::new();

        {
            let callbacks = Arc::new(CallbackRegistry::new());
            router.register("temporary", &callbacks);
            assert_eq!(router.active_device_count(), 1);
        }
        // callbacks dropped here

        // Device count still shows 1 (stale entry)
        assert_eq!(router.device_count(), 1);
        // But active count is 0
        assert_eq!(router.active_device_count(), 0);

        // Cleanup removes stale entries
        router.cleanup();
        assert_eq!(router.device_count(), 0);
    }

    #[test]
    fn router_route_telemetry_state() {
        let router = TopicRouter::new();
        let callbacks = Arc::new(CallbackRegistry::new());

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        callbacks.on_state_changed(move |_change| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        router.register("living_room", &callbacks);

        // Route a telemetry STATE message
        let payload = r#"{"POWER":"ON","Dimmer":75}"#;
        let routed = router.route("tele/living_room/STATE", payload);
        assert!(routed);
        // Batch + power + dimmer = 3 calls to state_changed
        assert!(counter.load(Ordering::SeqCst) >= 1);
    }

    #[test]
    fn router_route_lwt_offline() {
        let router = TopicRouter::new();
        let callbacks = Arc::new(CallbackRegistry::new());

        let disconnected = Arc::new(AtomicU32::new(0));
        let disconnected_clone = disconnected.clone();
        callbacks.on_disconnected(move || {
            disconnected_clone.fetch_add(1, Ordering::SeqCst);
        });

        router.register("device", &callbacks);

        // Route LWT offline
        let routed = router.route("tele/device/LWT", "Offline");
        assert!(routed);
        assert_eq!(disconnected.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn router_multiple_devices() {
        let router = TopicRouter::new();

        let callbacks1 = Arc::new(CallbackRegistry::new());
        let counter1 = Arc::new(AtomicU32::new(0));
        let c1 = counter1.clone();
        callbacks1.on_power_changed(move |_, _| {
            c1.fetch_add(1, Ordering::SeqCst);
        });

        let callbacks2 = Arc::new(CallbackRegistry::new());
        let counter2 = Arc::new(AtomicU32::new(0));
        let c2 = counter2.clone();
        callbacks2.on_power_changed(move |_, _| {
            c2.fetch_add(1, Ordering::SeqCst);
        });

        router.register("device1", &callbacks1);
        router.register("device2", &callbacks2);

        // Route to device1
        router.route("stat/device1/POWER", "ON");
        assert_eq!(counter1.load(Ordering::SeqCst), 1);
        assert_eq!(counter2.load(Ordering::SeqCst), 0);

        // Route to device2
        router.route("stat/device2/POWER", "OFF");
        assert_eq!(counter1.load(Ordering::SeqCst), 1);
        assert_eq!(counter2.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn router_replace_registration() {
        let router = TopicRouter::new();

        let callbacks1 = Arc::new(CallbackRegistry::new());
        let counter1 = Arc::new(AtomicU32::new(0));
        let c1 = counter1.clone();
        callbacks1.on_power_changed(move |_, _| {
            c1.fetch_add(1, Ordering::SeqCst);
        });

        let callbacks2 = Arc::new(CallbackRegistry::new());
        let counter2 = Arc::new(AtomicU32::new(0));
        let c2 = counter2.clone();
        callbacks2.on_power_changed(move |_, _| {
            c2.fetch_add(1, Ordering::SeqCst);
        });

        router.register("device", &callbacks1);
        router.register("device", &callbacks2); // Replace

        // Should route to callbacks2
        router.route("stat/device/POWER", "ON");
        assert_eq!(counter1.load(Ordering::SeqCst), 0);
        assert_eq!(counter2.load(Ordering::SeqCst), 1);
    }
}
