// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Device manager using the library's device-centric API.
//!
//! This module provides device management using individual `Device` instances.
//! MQTT devices share broker connections and receive real-time state updates via callbacks.

use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::Arc;

use eframe::egui;
use tasmor_lib::protocol::{HttpClient, SharedMqttClient};
use tasmor_lib::subscription::Subscribable;
use tasmor_lib::{Device, MqttBroker};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::device_config::{ConnectionStatus, DeviceConfig, ManagedDevice, Protocol, StateUpdate};

/// Key for identifying unique broker connections.
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct BrokerKey {
    host: String,
    port: u16,
    credentials: Option<(String, String)>,
}

impl BrokerKey {
    fn from_config(config: &DeviceConfig) -> Self {
        Self {
            host: config.host.clone(),
            port: 1883, // Default MQTT port
            credentials: config
                .username
                .as_ref()
                .zip(config.password.as_ref())
                .map(|(u, p)| (u.clone(), p.clone())),
        }
    }
}

/// Wrapper for different device types.
enum DeviceHandle {
    Http(Device<HttpClient>),
    /// MQTT device with reference to its broker key
    Mqtt {
        device: Device<SharedMqttClient>,
        broker_key: BrokerKey,
    },
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
/// MQTT devices share broker connections for efficiency.
/// State updates are sent via channel for event-driven UI updates.
pub struct DeviceManager {
    /// Mapping from config ID to device entry
    devices: Arc<RwLock<HashMap<Uuid, DeviceEntry>>>,
    /// Shared MQTT brokers keyed by connection info
    brokers: Arc<RwLock<HashMap<BrokerKey, MqttBroker>>>,
    /// Channel sender for state updates to the UI
    update_tx: mpsc::Sender<StateUpdate>,
    /// Egui context for triggering repaints
    egui_ctx: egui::Context,
}

impl DeviceManager {
    /// Creates a new device manager with a channel for state updates.
    ///
    /// # Arguments
    ///
    /// * `update_tx` - Channel sender for state updates to the UI thread
    /// * `egui_ctx` - Egui context for triggering repaints when state changes
    #[must_use]
    pub fn new(update_tx: mpsc::Sender<StateUpdate>, egui_ctx: egui::Context) -> Self {
        Self {
            devices: Arc::new(RwLock::new(HashMap::new())),
            brokers: Arc::new(RwLock::new(HashMap::new())),
            update_tx,
            egui_ctx,
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

    /// Returns the total subscription count across all MQTT brokers.
    pub async fn total_subscription_count(&self) -> usize {
        let brokers = self.brokers.read().await;
        let mut total = 0;
        for broker in brokers.values() {
            total += broker.subscription_count().await;
        }
        total
    }

    /// Returns the number of active MQTT brokers.
    pub async fn broker_count(&self) -> usize {
        self.brokers.read().await.len()
    }

    // =========================================================================
    // Device Management
    // =========================================================================

    /// Adds a device to the manager.
    ///
    /// The device is created and connected automatically based on its protocol:
    /// - **HTTP**: Ready immediately (stateless)
    /// - **MQTT**: Uses shared broker connection
    ///
    /// # Errors
    ///
    /// Returns an error if the device creation fails.
    pub async fn add_device(&self, config: DeviceConfig) -> Result<(), String> {
        let config_id = config.id;
        let capabilities = config.model.capabilities();

        let (handle, initial_state) = match config.protocol {
            Protocol::Http => {
                let mut builder = Device::http(&config.host).with_capabilities(capabilities);

                if let (Some(user), Some(pass)) = (&config.username, &config.password) {
                    builder = builder.with_credentials(user, pass);
                }

                let (device, state) = builder
                    .build_without_probe()
                    .await
                    .map_err(|e| e.to_string())?;
                (DeviceHandle::Http(device), state)
            }
            Protocol::Mqtt => {
                let topic = config.topic.as_deref().unwrap_or("tasmota");
                let broker_key = BrokerKey::from_config(&config);

                // Get or create the broker for this connection
                let broker = self.get_or_create_broker(&broker_key).await?;

                // Create device from shared broker
                let (device, state) = broker
                    .device(topic)
                    .with_capabilities(capabilities)
                    .build_without_probe()
                    .await
                    .map_err(|e| e.to_string())?;

                // Set up callback to receive state changes from MQTT
                let devices_clone = Arc::clone(&self.devices);
                let update_tx = self.update_tx.clone();
                let egui_ctx = self.egui_ctx.clone();
                device.on_state_changed(move |change| {
                    // Clone change for the async task
                    let change = change.clone();
                    let devices = Arc::clone(&devices_clone);
                    let update_tx = update_tx.clone();
                    let egui_ctx = egui_ctx.clone();

                    // Spawn async task to update state without blocking the callback
                    tokio::spawn(async move {
                        let mut devices = devices.write().await;
                        if let Some(entry) = devices.get_mut(&config_id) {
                            if entry.managed.apply_state_change(&change) {
                                tracing::debug!(
                                    device_id = %config_id,
                                    ?change,
                                    "Applied state change from MQTT callback"
                                );

                                // Send update to UI thread via channel
                                let _ = update_tx.send(StateUpdate::StateChanged {
                                    device_id: config_id,
                                    change: change.clone(),
                                });

                                // Request UI repaint
                                egui_ctx.request_repaint();
                            }
                        }
                    });
                });

                (DeviceHandle::Mqtt { device, broker_key }, state)
            }
        };

        // Create managed device entry with initial state from the builder
        let mut managed = ManagedDevice::new(config);
        managed.status = ConnectionStatus::Connected;
        managed.state = initial_state;

        let entry = DeviceEntry { handle, managed };
        self.devices.write().await.insert(config_id, entry);

        tracing::info!(
            device_id = %config_id,
            subscriptions = self.total_subscription_count().await,
            brokers = self.broker_count().await,
            "Device added"
        );

        Ok(())
    }

    /// Removes a device from the manager.
    ///
    /// For MQTT devices, this disconnects the device and cleans up the broker
    /// if no other devices are using it.
    pub async fn remove_device(&self, config_id: Uuid) -> bool {
        let entry = self.devices.write().await.remove(&config_id);

        if let Some(entry) = entry {
            // For MQTT devices, disconnect and maybe cleanup broker
            if let DeviceHandle::Mqtt { device, broker_key } = entry.handle {
                // Disconnect the device (unsubscribe from topics)
                device.disconnect().await;

                tracing::info!(
                    device_id = %config_id,
                    subscriptions = self.total_subscription_count().await,
                    "Device disconnected"
                );

                // Check if any other device uses this broker
                let devices = self.devices.read().await;
                let broker_still_needed = devices.values().any(|e| {
                    matches!(&e.handle, DeviceHandle::Mqtt { broker_key: key, .. } if key == &broker_key)
                });
                drop(devices);

                // If no other devices use this broker, disconnect and remove it
                if !broker_still_needed {
                    let mut brokers = self.brokers.write().await;
                    if let Some(broker) = brokers.remove(&broker_key) {
                        let _ = broker.disconnect().await;
                        tracing::info!(
                            host = %broker_key.host,
                            "Broker disconnected (no more devices)"
                        );
                    }
                }
            }

            tracing::info!(
                device_id = %config_id,
                subscriptions = self.total_subscription_count().await,
                brokers = self.broker_count().await,
                "Device removed"
            );

            true
        } else {
            false
        }
    }

    /// Gets or creates an MQTT broker for the given connection key.
    async fn get_or_create_broker(&self, key: &BrokerKey) -> Result<MqttBroker, String> {
        // Check if broker already exists
        {
            let brokers = self.brokers.read().await;
            if let Some(broker) = brokers.get(key) {
                tracing::debug!(host = %key.host, "Reusing existing broker");
                return Ok(broker.clone());
            }
        }

        // Create new broker
        let mut builder = MqttBroker::builder().host(&key.host).port(key.port);
        if let Some((user, pass)) = &key.credentials {
            builder = builder.credentials(user, pass);
        }

        let broker = builder.build().await.map_err(|e| e.to_string())?;

        tracing::info!(host = %key.host, "Created new MQTT broker");

        // Store and return
        self.brokers.write().await.insert(key.clone(), broker.clone());
        Ok(broker)
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
        let response = {
            let devices = self.devices.read().await;
            let entry = devices.get(&config_id).ok_or("Device not found")?;

            match &entry.handle {
                DeviceHandle::Http(device) => {
                    device.power_toggle().await.map_err(|e| e.to_string())
                }
                DeviceHandle::Mqtt { device, .. } => {
                    device.power_toggle().await.map_err(|e| e.to_string())
                }
            }
        }?;

        // Update local state from the response
        let mut devices = self.devices.write().await;
        if let Some(entry) = devices.get_mut(&config_id) {
            if let Ok(Some(power_state)) = response.power_state(1) {
                entry.managed.state.set_power(1, power_state);
            }
        }

        Ok(())
    }

    /// Turns the power on.
    pub async fn power_on(&self, config_id: Uuid) -> Result<(), String> {
        let response = {
            let devices = self.devices.read().await;
            let entry = devices.get(&config_id).ok_or("Device not found")?;

            match &entry.handle {
                DeviceHandle::Http(device) => device.power_on().await.map_err(|e| e.to_string()),
                DeviceHandle::Mqtt { device, .. } => device.power_on().await.map_err(|e| e.to_string()),
            }
        }?;

        // Update local state from the response
        let mut devices = self.devices.write().await;
        if let Some(entry) = devices.get_mut(&config_id) {
            if let Ok(Some(power_state)) = response.power_state(1) {
                entry.managed.state.set_power(1, power_state);
            }
        }

        Ok(())
    }

    /// Turns the power off.
    pub async fn power_off(&self, config_id: Uuid) -> Result<(), String> {
        let response = {
            let devices = self.devices.read().await;
            let entry = devices.get(&config_id).ok_or("Device not found")?;

            match &entry.handle {
                DeviceHandle::Http(device) => device.power_off().await.map_err(|e| e.to_string()),
                DeviceHandle::Mqtt { device, .. } => device.power_off().await.map_err(|e| e.to_string()),
            }
        }?;

        // Update local state from the response
        let mut devices = self.devices.write().await;
        if let Some(entry) = devices.get_mut(&config_id) {
            if let Ok(Some(power_state)) = response.power_state(1) {
                entry.managed.state.set_power(1, power_state);
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
            DeviceHandle::Mqtt { device, .. } => {
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
            DeviceHandle::Mqtt { device, .. } => {
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
            DeviceHandle::Mqtt { device, .. } => {
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

    /// Sets the RGB color (convenience method that converts to HSB internally).
    pub async fn set_rgb_color(&self, config_id: Uuid, hex: &str) -> Result<(), String> {
        let color = tasmor_lib::RgbColor::from_hex(hex).map_err(|e| e.to_string())?;

        let devices = self.devices.read().await;
        let entry = devices.get(&config_id).ok_or("Device not found")?;

        let response = match &entry.handle {
            DeviceHandle::Http(device) => device
                .set_rgb_color(color)
                .await
                .map_err(|e| e.to_string())?,
            DeviceHandle::Mqtt { device, .. } => device
                .set_rgb_color(color)
                .await
                .map_err(|e| e.to_string())?,
        };

        // Update local state for HTTP devices
        drop(devices);
        let mut devices = self.devices.write().await;
        if let Some(entry) = devices.get_mut(&config_id) {
            if matches!(entry.handle, DeviceHandle::Http(_)) {
                entry.managed.state.set_hsb_color(response.hsb_color());
            }
        }

        Ok(())
    }

    /// Sets the light scheme/effect (0-4).
    pub async fn set_scheme(&self, config_id: Uuid, value: u8) -> Result<(), String> {
        let scheme = tasmor_lib::Scheme::new(value).map_err(|e| e.to_string())?;

        let devices = self.devices.read().await;
        let entry = devices.get(&config_id).ok_or("Device not found")?;

        let response = match &entry.handle {
            DeviceHandle::Http(device) => {
                device.set_scheme(scheme).await.map_err(|e| e.to_string())?
            }
            DeviceHandle::Mqtt { device, .. } => {
                device.set_scheme(scheme).await.map_err(|e| e.to_string())?
            }
        };

        // Update local state for HTTP devices
        drop(devices);
        let mut devices = self.devices.write().await;
        if let Some(entry) = devices.get_mut(&config_id) {
            if matches!(entry.handle, DeviceHandle::Http(_)) {
                if let Ok(s) = response.scheme() {
                    entry.managed.state.set_scheme(s);
                }
            }
        }

        Ok(())
    }

    /// Sets the wakeup duration (1-3000 seconds).
    pub async fn set_wakeup_duration(&self, config_id: Uuid, seconds: u16) -> Result<(), String> {
        let duration = tasmor_lib::WakeupDuration::new(seconds).map_err(|e| e.to_string())?;

        let devices = self.devices.read().await;
        let entry = devices.get(&config_id).ok_or("Device not found")?;

        let response = match &entry.handle {
            DeviceHandle::Http(device) => device
                .set_wakeup_duration(duration)
                .await
                .map_err(|e| e.to_string())?,
            DeviceHandle::Mqtt { device, .. } => device
                .set_wakeup_duration(duration)
                .await
                .map_err(|e| e.to_string())?,
        };

        // Update local state for HTTP devices
        drop(devices);
        let mut devices = self.devices.write().await;
        if let Some(entry) = devices.get_mut(&config_id) {
            if matches!(entry.handle, DeviceHandle::Http(_)) {
                if let Ok(d) = response.duration() {
                    entry.managed.state.set_wakeup_duration(d);
                }
            }
        }

        Ok(())
    }

    /// Enables fade transitions.
    pub async fn enable_fade(&self, config_id: Uuid) -> Result<(), String> {
        let devices = self.devices.read().await;
        let entry = devices.get(&config_id).ok_or("Device not found")?;

        match &entry.handle {
            DeviceHandle::Http(device) => {
                device.enable_fade().await.map_err(|e| e.to_string())?;
            }
            DeviceHandle::Mqtt { device, .. } => {
                device.enable_fade().await.map_err(|e| e.to_string())?;
            }
        }

        Ok(())
    }

    /// Disables fade transitions.
    pub async fn disable_fade(&self, config_id: Uuid) -> Result<(), String> {
        let devices = self.devices.read().await;
        let entry = devices.get(&config_id).ok_or("Device not found")?;

        match &entry.handle {
            DeviceHandle::Http(device) => {
                device.disable_fade().await.map_err(|e| e.to_string())?;
            }
            DeviceHandle::Mqtt { device, .. } => {
                device.disable_fade().await.map_err(|e| e.to_string())?;
            }
        }

        Ok(())
    }

    /// Sets the fade transition speed (1-40).
    pub async fn set_fade_speed(&self, config_id: Uuid, speed: u8) -> Result<(), String> {
        let fade_speed = tasmor_lib::FadeSpeed::new(speed).map_err(|e| e.to_string())?;

        let devices = self.devices.read().await;
        let entry = devices.get(&config_id).ok_or("Device not found")?;

        match &entry.handle {
            DeviceHandle::Http(device) => {
                device
                    .set_fade_speed(fade_speed)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            DeviceHandle::Mqtt { device, .. } => {
                device
                    .set_fade_speed(fade_speed)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }

        Ok(())
    }

    /// Resets the total energy counter and returns the updated energy data.
    ///
    /// Returns the updated `DeviceState` with the new energy values including `TotalStartTime`.
    pub async fn reset_energy_total(
        &self,
        config_id: Uuid,
    ) -> Result<tasmor_lib::state::DeviceState, String> {
        // Call the library's reset_energy_total which resets and returns updated data
        let energy_response = {
            let devices = self.devices.read().await;
            let entry = devices.get(&config_id).ok_or("Device not found")?;

            match &entry.handle {
                DeviceHandle::Http(device) => device
                    .reset_energy_total()
                    .await
                    .map_err(|e| e.to_string())?,
                DeviceHandle::Mqtt { device, .. } => device
                    .reset_energy_total()
                    .await
                    .map_err(|e| e.to_string())?,
            }
        };

        // Update local state with the new energy values
        let mut devices = self.devices.write().await;
        if let Some(entry) = devices.get_mut(&config_id) {
            if let Some(energy) = energy_response.energy() {
                entry.managed.state.set_energy_total(energy.total);
                entry.managed.state.set_energy_today(energy.today);
                entry.managed.state.set_energy_yesterday(energy.yesterday);
                if let Some(start_time) = &energy.total_start_time {
                    entry.managed.state.set_total_start_time(start_time.clone());
                }
            }
            Ok(entry.managed.state.clone())
        } else {
            Err("Device not found".to_string())
        }
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
            DeviceHandle::Mqtt { device, .. } => device.get_power().await,
        };
        if let Ok(power_response) = power_result {
            if let Ok(power_state) = power_response.first_power_state() {
                state.set_power(1, power_state);
            }
        }

        // Query dimmer if supported
        let dimmer_result = match &entry.handle {
            DeviceHandle::Http(device) => device.get_dimmer().await,
            DeviceHandle::Mqtt { device, .. } => device.get_dimmer().await,
        };
        if let Ok(dimmer_response) = dimmer_result {
            if let Ok(dimmer) = tasmor_lib::Dimmer::new(dimmer_response.dimmer()) {
                state.set_dimmer(dimmer);
            }
        }

        // Query color temperature if supported
        let ct_result = match &entry.handle {
            DeviceHandle::Http(device) => device.get_color_temperature().await,
            DeviceHandle::Mqtt { device, .. } => device.get_color_temperature().await,
        };
        if let Ok(ct_response) = ct_result {
            if let Ok(ct) = tasmor_lib::ColorTemperature::new(ct_response.color_temperature()) {
                state.set_color_temperature(ct);
            }
        }

        // Query HSB color if supported
        let hsb_result = match &entry.handle {
            DeviceHandle::Http(device) => device.get_hsb_color().await,
            DeviceHandle::Mqtt { device, .. } => device.get_hsb_color().await,
        };
        if let Ok(hsb_response) = hsb_result {
            if let Ok(hsb) = hsb_response.hsb_color() {
                state.set_hsb_color(hsb);
            }
        }

        // Query energy data if supported
        let energy_result = match &entry.handle {
            DeviceHandle::Http(device) => device.energy().await,
            DeviceHandle::Mqtt { device, .. } => device.energy().await,
        };
        #[allow(clippy::cast_precision_loss)]
        if let Ok(energy_response) = energy_result {
            if let Some(energy) = energy_response.energy() {
                state.set_power_consumption(energy.power as f32);
                state.set_voltage(f32::from(energy.voltage));
                state.set_current(energy.current);
                state.set_energy_today(energy.today);
                state.set_energy_yesterday(energy.yesterday);
                state.set_energy_total(energy.total);
                state.set_apparent_power(energy.apparent_power as f32);
                state.set_reactive_power(energy.reactive_power as f32);
                state.set_power_factor(energy.factor);
                if let Some(start_time) = &energy.total_start_time {
                    state.set_total_start_time(start_time.clone());
                }
            }
        }

        // Query fade state if supported
        let fade_result = match &entry.handle {
            DeviceHandle::Http(device) => device.get_fade().await,
            DeviceHandle::Mqtt { device, .. } => device.get_fade().await,
        };
        if let Ok(fade_response) = fade_result {
            if let Ok(enabled) = fade_response.is_enabled() {
                state.set_fade_enabled(enabled);
            }
        }

        // Query fade speed if supported
        let fade_speed_result = match &entry.handle {
            DeviceHandle::Http(device) => device.get_fade_speed().await,
            DeviceHandle::Mqtt { device, .. } => device.get_fade_speed().await,
        };
        if let Ok(speed_response) = fade_speed_result {
            if let Ok(speed) = speed_response.speed() {
                state.set_fade_speed(speed);
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
    ///
    /// Disconnects all devices and brokers cleanly.
    pub async fn shutdown(&self) {
        tracing::info!("Shutting down device manager");

        // Disconnect all MQTT devices first
        let devices = self.devices.write().await;
        for entry in devices.values() {
            if let DeviceHandle::Mqtt { device, .. } = &entry.handle {
                device.disconnect().await;
            }
        }
        drop(devices);

        // Then disconnect all brokers
        let mut brokers = self.brokers.write().await;
        for (key, broker) in brokers.drain() {
            let _ = broker.disconnect().await;
            tracing::debug!(host = %key.host, "Broker disconnected during shutdown");
        }

        self.devices.write().await.clear();
        tracing::info!("Device manager shutdown complete");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_model::DeviceModel;

    /// Creates a test device manager with mock channel and context.
    fn create_test_manager() -> (DeviceManager, mpsc::Receiver<StateUpdate>) {
        let (tx, rx) = mpsc::channel();
        let ctx = egui::Context::default();
        (DeviceManager::new(tx, ctx), rx)
    }

    #[tokio::test]
    async fn create_manager() {
        let (manager, _rx) = create_test_manager();
        let devices = manager.devices().await;
        assert!(devices.is_empty());
        assert_eq!(manager.broker_count().await, 0);
        assert_eq!(manager.total_subscription_count().await, 0);
    }

    #[tokio::test]
    async fn add_http_device() {
        let (manager, _rx) = create_test_manager();

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
        // HTTP devices don't create brokers
        assert_eq!(manager.broker_count().await, 0);
    }

    #[tokio::test]
    async fn remove_device() {
        let (manager, _rx) = create_test_manager();

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
