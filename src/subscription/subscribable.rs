// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Subscribable trait for devices that support event subscriptions.
//!
//! This trait is implemented only for MQTT devices. HTTP devices do not
//! support subscriptions because HTTP is a stateless protocol without
//! persistent connections.

use crate::state::{DeviceState, StateChange};
use crate::subscription::{EnergyData, SubscriptionId};
use crate::types::{ColorTemperature, Dimmer, HsbColor, PowerState, Scheme};

/// Trait for types that support event subscriptions.
///
/// This trait provides methods to subscribe to various device events.
/// It is implemented for MQTT devices but not for HTTP devices, providing
/// compile-time safety.
///
/// # Type Safety
///
/// ```ignore
/// // MQTT devices support subscriptions
/// let mqtt_device: Device<MqttClient> = ...;
/// mqtt_device.on_power_changed(|idx, state| { /* ... */ }); // OK
///
/// // HTTP devices do NOT support subscriptions
/// let http_device: Device<HttpClient> = ...;
/// http_device.on_power_changed(|idx, state| { /* ... */ }); // Compile error!
/// ```
///
/// # Examples
///
/// ```no_run
/// use tasmor_lib::Device;
/// use tasmor_lib::subscription::Subscribable;
///
/// # async fn example() -> tasmor_lib::Result<()> {
/// let (device, _) = Device::mqtt("mqtt://192.168.1.50:1883", "tasmota_device")
///     .build()
///     .await?;
///
/// // Subscribe to power state changes
/// let sub_id = device.on_power_changed(|index, state| {
///     println!("Relay {index} is now {:?}", state);
/// });
///
/// // Subscribe to dimmer changes
/// device.on_dimmer_changed(|dimmer| {
///     println!("Brightness: {}%", dimmer.value());
/// });
///
/// // Unsubscribe when no longer needed
/// device.unsubscribe(sub_id);
/// # Ok(())
/// # }
/// ```
pub trait Subscribable {
    /// Subscribes to power state changes.
    ///
    /// The callback is called whenever a relay's power state changes.
    /// It receives the relay index (1-8) and the new power state.
    fn on_power_changed<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(u8, PowerState) + Send + Sync + 'static;

    /// Subscribes to dimmer value changes.
    ///
    /// The callback is called whenever the dimmer level changes.
    fn on_dimmer_changed<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(Dimmer) + Send + Sync + 'static;

    /// Subscribes to HSB color changes.
    ///
    /// The callback is called whenever the device's color changes.
    fn on_color_changed<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(HsbColor) + Send + Sync + 'static;

    /// Subscribes to color temperature changes.
    ///
    /// The callback is called whenever the white color temperature changes.
    fn on_color_temp_changed<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(ColorTemperature) + Send + Sync + 'static;

    /// Subscribes to scheme changes.
    ///
    /// The callback is called whenever the light scheme/effect changes.
    fn on_scheme_changed<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(Scheme) + Send + Sync + 'static;

    /// Subscribes to energy monitoring updates.
    ///
    /// The callback is called whenever energy data is received.
    fn on_energy_updated<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(EnergyData) + Send + Sync + 'static;

    /// Subscribes to connection events.
    ///
    /// The callback is called when the device becomes available.
    /// It receives the initial device state.
    fn on_connected<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(&DeviceState) + Send + Sync + 'static;

    /// Subscribes to disconnection events.
    ///
    /// The callback is called when the device becomes unavailable.
    fn on_disconnected<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn() + Send + Sync + 'static;

    /// Subscribes to all state changes.
    ///
    /// This is useful for logging or when you need to react to any change.
    /// The callback receives every state change.
    fn on_state_changed<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(&StateChange) + Send + Sync + 'static;

    /// Unsubscribes a callback by its subscription ID.
    ///
    /// Returns `true` if the subscription was found and removed.
    fn unsubscribe(&self, id: SubscriptionId) -> bool;
}
